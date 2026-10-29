# Track M2: water and buckets

## The owner's request

The owner's words for round 10 (`docs/PROGRESS.md`, round 10 brief):

> D. Finish track D: the rest of d5 (trial chambers and trial spawners, lodestones), d3 (deep dark, warden, sculk), and d4 (the remaining mobs), with d4 split into two or three tracks by mob group. Land the shared changes (mob list, spawn tables, render registry) first so the later merges stay small.

Track D0 (`docs/round10/track-d0.md`) is that shared part: the mob registry, the spawn rows, the client and render registry and the round 10 mob list. Tracks M1-M3 are d4 split by mob group. **M2 is the water group** (registry group `water`, hit-key bytes 64-79): water movement and mob buckets (D0-6's prerequisite P2), squids and glow squids, the four fish, dolphins, axolotls, frogs, tadpoles and frogspawn, guardians and elder guardians, nautiluses and zombie nautiluses, and the sulfur cube (it is `Bucketable`, so it moved here, D0-8). These are rows 25-38 and 49 of `docs/round10/mobs.md`.

Every family plugs into D0's registry: its own files plus one registration line per registry, all in the `water` group files. Paths and line numbers are `minecraft-survival` at `d1a3d028` unless marked otherwise; tracks D, W, E1, E2, F, M1 and M3 run at the same time, so re-check every line before editing. Where D0's merged code differs from `track-d0.md`, follow the code (it already has, for example, `MobCore.ambient_interval`, `Despawn::of_category`, `Hurt::new(..).by(..)` and `family_goal_step(id, level, players, speed, ai)`). Vanilla facts were checked with javap on the 26.3 `client.jar` and the 26.3 data in that jar this round, unless marked "javap during the work". No Mojang code or data is copied: numbers and behaviour only, data read at runtime.

**Multiplayer rule for every item.** The host owns every level, mob and rule. New state is per mob (saved in the vanilla entity format through the registry) or per player (keyed by UUID). Nothing keys on player 0. Friends run no AI: they mirror a registered mob from its look tag and its per-tick `MobShow.family` (`FamilyShow`), and friends' clicks, buckets and rides run on the host.

---

## M20. Shared points for the track

### Start, segments and waits

- **Branch** `track-m2`, worktree `C:\Dev\mc-wt\track-m2`, target dir `D:\iw4l-wt\track-m2`. The branch starts from `minecraft-survival` once D0's segment **d0a** (registry-core ... silverfish-drawn) is merged. Each segment's base is tagged `r10-m2<letter>-base` before it starts, and its run spec is `docs/round10/runs/m2<letter>.json`.
- **d0b is not needed by any M2 group.** d0b's flying navigation builds on `path_search.rs` (D0-6.2); M2's evaluator split is in `walk_path.rs` (M2-1). If d0b chose `walk_path.rs` and merged before `water-paths` starts, `water-paths` plugs into that split and skips its first commit.
- **Waits.** Before a segment that needs another track's work, the orchestrator merges that work into `minecraft-survival` and `minecraft-survival` into `track-m2`, as it does for D0. If one group's wait is not met when its segment starts, the orchestrator moves only that group to the next segment and the others run. No group builds a copy of another track's piece (the owners are below).

| Segment | Groups | Waits for (in `minecraft-survival`) |
|---|---|---|
| m2a | `water-hooks`, `water-paths`, `water-buckets`, `squids-live`, `squids-drawn` | D0 d0a |
| m2b | `water-swimming`, `fish-live`, `fish-spawning`, `fish-drawn`, `fish-pufferfish` | M1's "Registered Minecraft mobs can run their own AI step" (`family_goal_step_with`, rabbits' brief). If D0's merged `family_goal_step` already takes the step function, nothing waits. |
| m2c | `fish-buckets`, `dolphins-live`, `dolphins-drawn`, `dolphins-treasure` | m2b |
| m2d | `guardians-live`, `guardians-drawn`, `guardians-effects` | m2b |
| m2e | `sulfur-cubes-live`, `sulfur-cubes-drawn`, `sulfur-cubes-blocks`, `sulfur-cubes-kicks`, `sulfur-cubes-items` | m2b (the cube hop runs through `family_goal_step_with`); E2's `ti1` softly (`sulfur-cubes-kicks` then goes through `hurt_for_player`) |
| m2f | `axolotls-live`, `axolotls-drawn`, `axolotls-hunt`, `axolotls-buckets` | M1's "Minecraft animals share vanilla's brain behaviours for panic, temptation, love and following adults" (`world/families/animal_brain.rs`, goats' `goats-live` 1) |
| m2g | `frogs-live`, `frogs-drawn`, `tadpoles-live`, `tadpoles-drawn`, `frogspawn` | M1's `animal_brain.rs`; M1's `World::biome_in_tag` (rabbits' `rabbits-live`) |
| m2h | `frogs-leap-eat`, `frogs-froglights`, `nautiluses-live`, `nautiluses-drawn`, `nautiluses-taming` | M1's "Minecraft mobs can long-jump like vanilla goats and frogs" (`goats-leap` 1, M1 segment m1d) for `frogs-leap-eat`; M1's "Minecraft loot tables know what killed a mob" (`turtles-live` 1, M1 segment m1g) for `frogs-froglights`; E1's segment e1b for `nautiluses-taming` (`horse-dye` changes `server.rs:4134`, which its commit 3 then edits). m1g comes late in M1's order, so `frogs-froglights` is the group most likely to move to m2i |
| m2i | `dolphins-grace`, `dolphins-escort`, `guardians-curse`, `nautiluses-riding` | W's segment w1 (W7's `Output.player_effects` and `EffectGrant`); M1's "Minecraft mobs can tell when a player sprints" (`PlayerControls.sprinting`, armadillos' `armadillos-live` 1) for `dolphins-escort` |

- **W's game events (P7).** W's segment w1 adds `Level::game_event`, `EntityWorld::game_event` and `Entities::game_event` (W0 rule 1). M2 sites that post one: the mob bucket (`entity_place`, `fluid_place`, M2-1), the squid's pulse (`swim`, M2-2), frogspawn (`block_place`, `block_destroy`, M2-8), the sulfur cube (`prime_fuse`, `shear`, `entity_interact`, `bounce`, M2-6) and the nautilus dash (`entity_action`, M2-9). When w1 is in the branch at the group's start, the group adds its one line per site. Otherwise it lists the site under "Game events still to add" in the log, and the first group of the next M2 segment that starts after w1's merge adds those lines (W0 rule 3: whichever merges second adds the line).
- **E1's entity-event particles.** Default rows (7 and 6 taming, 18 love) cover every M2 family. The dolphin's event 38 row is added by `dolphins-treasure` when E1's `mob_particles::puff_for` is in the branch, else it goes to the log's "Left for later".
- **E2.** Thrown items hit every registered mob through E2's `hit_boxes` with no M2 work. `axolotls-buckets` puts its one `splash.rs` line outside E2's loop in `wet`.
- **D0 pieces still on their way in d0a.** When this brief was checked, `track-d0`'s working tree (group `silverfish-live`, not yet committed) was adding: `Family::emits_sounds(kind)` (default true; false gives `MovementSounds::without_sounds()`, vanilla's `EVENTS` movement emission, with no step or swim sound while the splash still plays), `EntityWorld::family_alert::<S>(id, attacker, ai)` (alerts same-kind mobs without a target in the follow-range box), `world/families/goals.rs` (`RandomStrollGoal { speed, interval }`, `reduced_tick_delay`, `ClimbOnTopOfPowderSnowGoal`) and `WalkTarget::MonsterOver(..)` after `Hoglin`. Check the merged d0a. Where they are there, use them and add no copy: squids and guardians answer `emits_sounds` false, dolphins alert their pod with `family_alert`, and guardians build their stroll on `goals::RandomStrollGoal`. `water-hooks` then adds only `swim_sounds` and `quiet_steps` (fish, tadpoles and nautiluses keep a swim sound but have no step).

### The D0 plug-in checklist is the law

Read `docs/round10/track-d0.md` D0-2 and D0-3 before the first commit. The rules that bite most:
1. A family is its own files (entities `world/families/water/<family>.rs`, terrain `families/water/<family>.rs`, rows in world `natural_spawner/families/water.rs`) plus its registration lines in the three `water.rs` group files. Nothing else registers it.
2. **Never edit** the per-family lists D0-3 names (`world.rs` walks, `EntityKey`, `MobHit`, `MonsterKind`, `MonsterGoalContext`, `Prey`, `SIMULATED_KINDS`, `spawn_saved`, `mob_tags_where`, `VEHICLES`, `MobShow`, `VOLATILE`, `tracking_range`, `client_mobs.rs`, `spawner_display.rs`, the `mesh.rs` texture list, `entity_view.rs`, survival `entities.rs` keys, `death.rs`, `HATCHING_EGGS`, `PROTOCOL`). If one seems to need an edit, the registry lacks a hook: add the hook to the root file in its own commit named for it (the owners are below) and log it.
3. Behaviour in a shared file (blocks, items, player state, effects) lives in a new file of the family's and is called from one line placed next to a related arm, never at the end of a list.
4. Every drawn state goes through `TerrainFamily::show`/`show_on`; animation clocks travel as ticks; changing saved fields go in `VOLATILE`; attacks with their own damage type or effect use `PlayerHitKind::Typed`; keyframes come from `keyframes::shared()`.
5. Tests sit in the family's files. D0's generic tests already cover the eggs survey, the hit keys and the spawner cage for every registered kind.

**Hit keys** (the `water` group, 64-79): 64 `squids`, 65 `fish`, 66 `dolphins`, 67 `axolotls`, 68 `frogs`, 69 `tadpoles`, 70 `guardians`, 71 `nautiluses`, 72 `sulfur_cubes`, 79 the `#[cfg(test)]` probe `minecraft:swim_probe` (M2-1). 73-78 stay free.

### Owners of shared pieces

M1 and M3 run beside M2 and plug into the same registry. Each shared piece below has one owner; the others only use it once it is merged. "After X" means a new line after that line, never an edit of X's line.

**Root registry hooks** (D0's root `families.rs` files; each hook is its own commit, named for it):

| Hook | Owner (group) | Users |
|---|---|---|
| `Family::breath` (`Breath::Lungs { max, refill }`, `Gills`, `Own`) with `living::breathe_up_to` | M2 `water-hooks` | every M2 family (the dolphin's 4800 air is `Lungs { max: 4800, refill: 4800 }`) |
| `Family::swim_sounds`, `Family::quiet_steps` and the zero-volume step guard at `emissions.rs:64` | M2 `water-hooks` | fish, dolphins, axolotls, tadpoles, nautiluses, sulfur cubes; squids and guardians only if d0a lacks D0's `Family::emits_sounds` (see the waits above) |
| `family_goal_step_with(.., run)` | M1 rabbits | M2-1's `swim_ai_step`, guardians, sulfur cubes |
| `Family::ambient` / `ambient_sound` | nobody this round (M1's brief drops it) | M2 does not need it: its families set `core.ambient_interval` in `spawn` and rewrite or drop the `Voice::Ambient` that `family_base_tick` pushes, after the base tick, as M3's allays do |
| `Family::OWN_DYING_TRAVEL` | M2 `squids-live` | dolphins, axolotls |
| `TerrainFamily::tracking_range(kind)` | M2 `squids-drawn` | guardians |
| `TerrainFamily::shadow_radius(mob)` | M2 `fish-drawn` (m2b), under the name and subject M1's brief uses, "Registered Minecraft mobs cast their own shadow size" | guardians; M1's armadillos and turtles. M1's `armadillos-drawn` plans the same commit; its brief drops its copy when the other is merged first |
| `FamilyDraw.camera` | M2 `guardians-drawn` | |
| `FamilyDraw.blocks`, the family knockback hook, family blasts | M2 sulfur cube groups | |
| `ClientFx.item_crumbs` | M1 pandas (`pandas-eating`) | the sulfur cube's optional goo |
| `Family::seen_as_enemy` | M2 `axolotls-hunt` | guardians' targets |
| `PlayerEffect` queue (`family_player_effect`, `take_player_effects`) | M2 `dolphins-grace` | `guardians-curse` (adds `cue`), `nautiluses-riding`; W5 may reuse it |
| `TerrainFamily::walk_target` | M3 creakings | `frogs-drawn` |
| `world/families/animal_brain.rs`, `world/families/long_jump.rs` | M1 goats | axolotls, frogs, nautiluses |
| `LookControl::tick_from` (`look.rs`, beside `tick_limited` `:144`) | M1 foxes | frogs, optionally |

**Shared files outside the registry:**

| Site | Owner and placement | Notes |
|---|---|---|
| `walk_path.rs` evaluator split and amphibious mode, `navigation.rs` `WaterNav`/`tick_water` (after `wall_climber` `:278`, after `tick_amphibious` `:532`), `stroll.rs` `water_bound_random_position` (after `amphibious_random_position` `:199-221`), `movement.rs` `travel_swimming` (after `travel_drowned_swimming` `:275-278`, before the private `travel_water_mode`) | M2 `water-paths` and `water-swimming` | M1's turtles use M2's amphibious mode and `travel_swimming`: no `travel_turtle` and no amphibious copy in M1. Only if M2's P2 is still unmerged when m1g is M1's last segment does M1's optional `turtles-water` build them under M2's names (`track-m1.md` M1-8, decision 1); then the second of the two to merge drops its copy. M3's undead horses add `travel_water_slowed` after `travel_water` (`:270`). |
| `WalkTarget` (`walk_path.rs:46-60`) and its values (`stroll.rs:14-46`) | M1 turtles `Turtle` after `Animal`, M1 bees `Air` after `Monster`, M2 guardians `Guardian` after `Strider { .. }`; the value arms in the same order | |
| `look.rs` | M2 `tick_smooth_swimming` after `set_look_at_with_limits` (`:133`); M1 foxes `tick_from` at `:144` | |
| `emissions.rs` | M2 the guard at `:64`; W `:55`, `:78`, `:89`; M3 camels `:21` | |
| `movement.rs` `travel_air_flying` (`:504-609`), `restitute` (`:641-671`) | M2: squids split `Body::move_self` out first, then the sulfur cube's `BodyPhysics` | |
| vendored `World` (`player/src/lib.rs`) and `MobWorld` (`server_mobs.rs`) | M1 foxes after `destroy_block` (`:98`, `:258`); M1 rabbits `biome_in_tag` after `biome` (`:140`, `:236`); M1 `raining`/`thundering` after `rain_at` (`:189`, `:357`); M1 turtles `sea_level` after `max_y` (`:203`, `:385`); M1 bees after `block_in_tag` (`:238`, `:262`); M2 dolphins `nearest_structure` after `can_see_sky` (`:185`, `:353`); M2 sulfur cubes `best_neighbor_signal` and `tnt_explodes` after `snow_golem_melts` (`:192`, `:361`); M3's copper golems and E2's `chest_open_count` take anchors of their own | tadpoles use M1's `biome_in_tag` |
| `ambient_particles.rs` `Kind` and its matches | M2's kinds go after `Flame` (`:53`) and get their own arms after each `Kind::Flame` arm (`:469`, `:493`, `:1074`, `:1106`, `:1168`); elsewhere on new lines of their own, never inside another kind's `\|` line and never inside `coast`'s match (`:372-384`, next to M3's `Trail`). M3 phantoms use `HappyVillager`, M3 creakings `EndRod`, M3 happy ghasts `Smoke`, M1 pandas `Poof`, E1 the end of `Kind`. | |
| `world/src/level/mod.rs` | M2: `pub mod mob_bucket;` after `pub mod dispenser;` (`:13`), `pub mod frogspawn;` after `pub mod fluid;` (`:27`), the frogspawn `on_place` arm after the `FrostedIceBlock` arm (`:705-706`), its `tick_block` arm after the coral arm (`:856-857`, before the farmland arm, so it stays two lines away from M3's creaking heart arm before `ComposterBlock` `:860`). M1's sniffer eggs take the `wet_sponge` arm in `on_place` and `FireBlock` in `tick_block`, M3's dried ghasts `FrostedIceBlock` in `tick_block`, M3's creaking hearts the line before `ComposterBlock`, M3's `creaking_heart` module the line after `container` (`:12`). | M1's brief expected frogspawn after `FrostedIceBlock` in `tick_block` too; M3 took that line, so M2 uses the coral arm there |
| `dispenser.rs` | M2 an arm after `"water_bucket" \| "lava_bucket"` (`:93`), with `OTHER_BEHAVIOURS` (`:17-26`) left as it is (the earlier arm wins; its lines also hold M1's shears and glass bottles); M1 bees after `"honeycomb"` (`:95`) | |
| `natural_spawner.rs` variant registries | M2 frogs after the chicken lines (`:404`, `:542`), nautiluses after the wolf lines (`:408`, `:546`) | |
| `check_spawn_obstruction` (`natural_spawner.rs:821-843`) | M2 `squids-live` (one `InWater` line at `:822`) | serves every M2 water row |
| `spawning.rs:1583` (the zombie nautilus jockey stub in `finalize_drowned`) | M2 nautiluses | M3 camels own `finalize_husk` (`:1517-1569`) |
| `player/src/survival.rs` `EffectKind` and `ALL` (`:36-82`), the name rows in `player_nbt.rs`, `commands.rs`, `potions.rs` | M2: `DolphinsGrace` after `ConduitPower` (`:51`, `:75`), then `BreathOfTheNautilus` after `SlowFalling` (`:53`, `:77`); `ALL`'s length line (`:62`) is M2's this round | M1 leaves turtle-shell water breathing for later |
| `loot.rs` `EntityLootContext` (`:17-34`), `damage_source_properties` (`:636-646`), `health::Death` | M1 turtles' "Minecraft loot tables know what killed a mob" rewrites them first; M2 `frogs-froglights` then adds `attacker_variant` and the components branch | |
| survival `PlayerView`, `PlayerControls` | M1 armadillos add `sprinting` (M3 camels reuse it) | M2 adds no `PlayerView` field: `dolphins-escort` reads `sprinting` and the water at the player |
| `mount_inventory.rs`, `server.rs:4134` (`Command::MountInventory`) | M3 owns the horse lines; E1's `horse-dye` (e1b) changes `:4134` and the stack builder (`:34-45`) | M2 adds new lines only (a field after `llama` `:22`, the first line of `of` (`:31`) and `still_valid` (`:198`), `apply` after `still_valid`) and changes `:4134` after e1b |
| menus and screens beside each `llama` | M2 nautiluses; E1's trading keys are near (`menus.rs:1907`, `:1972`) | |
| `potions.rs` (`:380`, `:425-434`), `server.rs:4278`, `splash.rs` `wet` (`:53-54`) | M2 axolotls, one line each | outside E2's loop in `wet` |
| `monster_ai.rs` | M2 sulfur cubes `cube_goals()` before `slime_goals` (`:1150`); M1 rabbits `AvoidEntityGoal::matching` (`:1737`), M1 turtles `Prey::BabiesOnLand` (`:415`) and the zombie targets (`:1948-1965`) | M2 adds no `Prey` variant (guardians target through their own goal); drowned hunting axolotls waits (Left for later) |
| `heard.rs`, `ToClient::GameEvent`, the totem sites (`minecraft_world.rs:934`, `minecraft_remote.rs:791`) | M2 `guardians-curse` | |
| D0's knockback, blast and arrow arms (`combat.rs`, `world.rs`) | D0's; M2 `sulfur-cubes-kicks` puts its hook in front of them, after E2's `ti1` | |

### Protocol

`minecraft_net::PROTOCOL` is never bumped by a track (`docs/round10/decisions.md`); the release step bumps it once when messages changed. A commit that changes a `ToClient`/`ToServer` message or a serialized struct adds a line under "## Protocol" in the track log. In M2 that is `ToClient::GameEvent` (`guardians-curse` 2) and `MenuExtras::Mount.nautilus` (`nautiluses-taming` 3). Everything else rides existing fields: `FamilyShow`, look tags, `ServerMob.x_rot`, `EffectView` names and the `mc_float` bits. `Change`, `Command` and `ItemOnBlock` are in-process and not serialized.

### Log, mob list and docs

- **Track log** `docs/round10/track-m2-log.md`: each commit, the javap numbers it used, decisions (the open decisions of each section with the choice made), the vendored files patched and added (counted for the CLAUDE.md total), "## Protocol", "State doc changes" (the wording for `CLAUDE.md`, `docs/MINECRAFT-SURVIVAL.md` and `docs/MULTIPLAYER.md`, which a track run never edits, nor `docs/PROGRESS.md` or `docs/round10/decisions.md`), "Left for later", and each group's owner's steps and kit line (at most 27 stacks, in its own code block).
- **`docs/round10/mobs.md`:** only the "M2: water and buckets" section, with each row's status and commits updated in the same commit as the code. The swim probe's key 79 goes in a note under that table, not in the shared hit-key table.

### Combined kit

One kit for a test of the whole track (27 stacks). Each section keeps its own kit for its detailed steps.

```
IW4L_MINECRAFT_KIT=minecraft:squid_spawn_egg*8,minecraft:glow_squid_spawn_egg*4,minecraft:cod_spawn_egg*16,minecraft:salmon_spawn_egg*8,minecraft:tropical_fish_spawn_egg*8,minecraft:pufferfish_spawn_egg*8,minecraft:dolphin_spawn_egg*8,minecraft:axolotl_spawn_egg*8,minecraft:frog_spawn_egg*8,minecraft:tadpole_spawn_egg*8,minecraft:guardian_spawn_egg*8,minecraft:elder_guardian_spawn_egg*2,minecraft:nautilus_spawn_egg*8,minecraft:zombie_nautilus_spawn_egg*4,minecraft:sulfur_cube_spawn_egg*8,minecraft:water_bucket,minecraft:bucket*16,minecraft:glass*64,minecraft:cod*32,minecraft:pufferfish*16,minecraft:slime_ball*32,minecraft:saddle,minecraft:iron_nautilus_armor,minecraft:tropical_fish_bucket,minecraft:dirt*16,minecraft:lead*4,minecraft:iron_sword
```

Every id was checked against the 26.3 item models (`assets/minecraft/items`). The class loadout's M4A1 covers the gun steps. A friend uses the same line.

---

## M2-1. Water movement and buckets (groups `water-hooks`, `water-paths`, `water-buckets`, `water-swimming`)

This is D0-6's prerequisite P2. It adds no mob of its own: it gives every M2 family the registry hooks a swimmer needs, vanilla's water and amphibious path search, the swimming controls and goals, gill breathing, flopping and mob buckets.

Lines are `minecraft-survival` at `d1a3d028`; check each one again before editing. The only key used is **79** (the end of `water` 64-79), for a `#[cfg(test)]` probe `minecraft:swim_probe`. Note it in M2's section of `docs/round10/mobs.md`.

### What exists today

- **Only the drowned swims, on a measured stand-in.**
  - Paths: `world.rs:1466-1480` and `:1543` use `navigate_amphibious_to_with_accuracy` (`navigation.rs:206`) over `MeasuredWaterFloorTerrain` (`path_search.rs:48-75`, water plus four block ids; any other block counts as unsupported and solid).
  - Corner checks: `tick_amphibious` (`navigation.rs:532`) cuts corners with `clear_full_cubes` (`:689`), which knows the same four ids.
  - Travel: `world.rs:1511-1541` calls `Body::travel_drowned_swimming` (`movement.rs:275`, which is `travel_water_mode(0.01, 0.9, no gravity)`; the call is at `:1526`), and `world.rs:1886` calls `MoveControl::tick_drowned_swimming` (`control.rs:70`).
- **The walk search cannot swim.** In `walk_path.rs` the A* loop (`find_walk_path_to_any` `:807`) is tied to its private `Evaluator` (`:264`), `Node` and `Heap`. It has no amphibious mode: `find_accepted_node` (`:476`) sinks through water via `first_non_water_below` (`:565`), and `floor_level` (`:468`) never answers `y + 0.5`. `GroundNavigation::recompute` (`navigation.rs:312`) re-plans only with that search, and `follow_path` (`:594`) has a fixed vertical reach of 1.0.
- **Random spots.** `stroll.rs` gives ground-stable spots (`:65`, a solid block below) or amphibious ones (`:199-221`), but no water-bound one. `StrollState` hard-codes the 120-tick interval.
- **Goal mobs walk.** `monster_ai_step` (`world.rs:1022-1097`) ticks the ground navigation (`:1055`) and the ground `MoveControl`, adds `Monster`'s light idle time (`:1031`), and travels with `(sideways, 0, forward)`, so `yya` is lost (`:1084`).
- **Pitch.** `LookControl.pitch` is the mob's `xRot`; `tick_limited` resets it each tick (`look.rs:145`). Friends get it as `ServerMob.x_rot` (`client_mobs.rs:1203`), so it needs no new plumbing.
- **Air.** `living::breathe` (`world/living.rs:15`) models lungs only. `Air` is a global `VOLATILE` key (`mob_tracking.rs:23`). Saving writes `Rotation [yaw, 0]` and no `Air` (`server_mobs.rs:1512`).
- **Sounds.** In `MovementSounds` (`world/emissions.rs:12`) an empty `step` means "the block's step", so a mob with no step sound cannot be expressed (own-step arm `:64`). Swim and splash are fixed by category. D0's d0a is adding `Family::emits_sounds` and `MovementSounds::without_sounds()` (M20, "D0 pieces still on their way"), which silences step and swim together (vanilla `EVENTS`); a mob that keeps its swim sound but has no step still needs `quiet_steps`. After d0a the own-step arm sits about five lines lower.
- **Buckets.**
  - Using one does nothing today: `Fluid::of_bucket` (`bucket.rs:99`) knows water and lava only, so `bucket_use` (`:290`) answers `None` for `cod_bucket`.
  - The path a bucket takes: `pour` (`:349`) → `Change::Server(.., ItemOnBlock::EmptyBucket)` (`bucket.rs:146`, `server.rs:31`) → `actions.rs:1074` and `:1167` → `Entities::item_on_block` (`entities.rs:994`) → `Command::ItemOnBlock` (`server.rs:3408`, `:3790`, `:4073`) → `item_on_block_by` (`:2485`) → `Level::empty_bucket_into` (`dispenser.rs:197`).
  - The seven mob buckets are listed as unported in `OTHER_BEHAVIOURS` (`dispenser.rs:17-26`).
  - Helpers to reuse:
    - `bottle::filled_result` (`bottle.rs:54`) and the creative branch of cow milking (`cow.rs:150-181`);
    - `drop_leash` (`world/leashes.rs:551`);
    - `item_nbt::json_tag` (`item_nbt.rs:30`), which turns floats into doubles;
    - `Level::summon_mob` (`level/spawning.rs:206`), which finalizes only when given no NBT;
    - `Level.spawned` (`level/mod.rs:153`), taken by `take_spawned` (`server.rs:1016`); an unsimulated kind waits dormant and is saved unchanged.
- **Spawning.** `Placement::InWater` exists (`natural_spawner.rs:192`, `:697`). The categories are set (`spawning.rs:103-108`), and `water_ambient` despawns beyond 64 blocks (`:87`). `SpawnLevel::sea_level` exists (`natural_spawner.rs:47`). There is no surface water rule.

### Vanilla 26.3 (javap this round)

- **`WaterBoundPathNavigation`** (fish, tadpole, dolphin, guardian, nautiluses):

  | | 26.3 |
  |---|---|
  | Evaluator | `SwimNodeEvaluator(allowBreaching = mob is DOLPHIN)`, no doors |
  | `canUpdatePath` | `allowBreaching \|\| isInLiquid` |
  | `getTempMobPos` | `(x, y + 0.5 × height, z)` |
  | `getGroundY` | the waypoint's `y` |
  | `canMoveDirectly` | `!sight::blocked(from, to + 0.5 × height)` (collider clip, no fluids) |
  | `isStableDestination(pos)` | `!solidRender(pos)` |
  | `canNavigateGround` | false |
  | `getMaxVerticalDistanceToWaypoint` | 0.5 (base 1.0) |

  `PathNavigation.tick`, when it cannot update: it advances only if `mobPos.y > node.y`, the mob is not on the ground and it stands in the node's column.
- **`SwimNodeEvaluator`**:
  - Start at `(floor(minX), floor(minY + 0.5), floor(minZ))`. Target at the floored point. Box `floor(size + 1)`. Path types are cached per search.
  - Neighbours: the six `Direction.values()` (down, up, N, S, W, E) through `findAcceptedNode`, kept if not closed. Then, for N, E, S and W with their clockwise neighbour, the same-height diagonal when both sides exist with a malus of at least 0.
  - `findAcceptedNode`: a node only for `WATER`, or `BREACH` if breaching. Its cost is `max(cost, mob malus)`, plus 8 where the block holds no fluid.
  - `getPathTypeOfMob` over the box cells (x, y, z order):
    - the first cell with no fluid, a block below that is `isPathfindable(WATER)` and air in the cell gives `BREACH`;
    - any cell whose fluid is not water gives `BLOCKED`;
    - otherwise the last cell gives `WATER` if `isPathfindable(WATER)`, else `BLOCKED`.
- **`AmphibiousPathNavigation`** (axolotl; frog and turtle subclass it):

  | | 26.3 |
  |---|---|
  | Evaluator | `AmphibiousNodeEvaluator(false)` |
  | `canUpdatePath` | always |
  | Temp position, ground `y` | as water-bound |
  | `canMoveDirectly` | the same clip, only in a liquid |
  | Stable destination | the block below is not air |

- **`AmphibiousNodeEvaluator(prefersShallow)`** extends `WalkNodeEvaluator`:
  - Mob maluses: `WATER` 0 for good; `WALKABLE` 6 and `WATER_BORDER` 4 for the search only.
  - In water it starts at the swim start. The target is at `y + 0.5`.
  - Neighbours: the walk ones, plus up (with `jump − 1`) and down, kept only if `WATER`. No down neighbour from a `TRAPDOOR`. `jump` is `floor(max(1, maxUpStep))`, or 0 when the type above has a negative malus or the node is honey.
  - Shallow preference: `WATER` nodes below `seaLevel − 10` cost +1.
  - Water with a `BLOCKED` face neighbour is `WATER_BORDER`.
  - `isAmphibious` changes the walk evaluator in two places: water's floor is `y + 0.5`, and a `WATER` node returns at once (no search below the water).
- **`SmoothSwimmingMoveControl(maxTurnX, maxTurnY, inWater, outside, gravity)`**. Dolphin, tadpole and frog use `(85, 10, 0.02, 0.1, true)`; nautiluses `(85, 10, 0.011, 0, true)`; the axolotl subclasses it.
  1. With gravity, in water: `vy += 0.005`.
  2. With no `MOVE_TO`, or the path done: speed, `xxa`, `yya` and `zza` are all 0.
  3. Closer than `2.500000277905201E-7` squared: `zza = 0`.
  4. Otherwise yaw is `rotlerp(.., maxTurnY)`, and the body and head yaw follow it. `s = speedModifier × MOVEMENT_SPEED`.
  5. In water:
     - `setSpeed(s × inWater)`;
     - the pitch target is `-atan2(dy, horizontal)`, clamped to `±maxTurnX`; the pitch moves at most 5° a tick;
     - `zza = cos(pitch) × s` and `yya = −sin(pitch) × s`.
  6. Out of water: `setSpeed(s × outside × (1 − clamp((|Δyaw| − 10) / 50, 0, 1)))`.
- **`SmoothSwimmingLookControl(maxFromCentre)`** (10 for dolphin, tadpole and nautiluses). It never resets the pitch.
  - Looking at something: head yaw towards the target + 20 at `yMaxRotSpeed`; pitch towards the target + 10 at `xMaxRotAngle`.
  - Otherwise: pitch towards 0 at 5° a tick once the path is done; head yaw towards the body.
  - Then the body follows 4° a tick while the head is more than `maxFromCentre` away.
- **`AbstractFish`** (the four fish; tadpole):
  - `FishMoveControl`:
    - eyes in water: `vy += 0.005`;
    - no move or the path done: speed 0;
    - else speed is `lerp(0.125, speed, mod × MOVEMENT_SPEED)`, `vy += speed × dy / distance × 0.1`, and yaw is `rotlerp(.., 90)` with the body following.
  - `travelInWater`: `moveRelative(0.01)`, then × 0.9, then `vy −= 0.005` when it has no target.
  - Flop, in `aiStep`, when out of water, on the ground and after a vertical collision:
    - `v += ((r × 2 − 1) × 0.05, 0.4, (r × 2 − 1) × 0.05)`, with x drawn before z;
    - not on the ground; `needsSync`;
    - the flop sound at the voice pitch: `entity.{cod,salmon,tropical_fish,puffer_fish,tadpole}.flop`.
  - Sounds: no step sound; swim `entity.fish.swim`.
  - Goals: `PanicGoal(1.25)`, `AvoidEntityGoal(Player, 8, 1.6, 1.4)`, and `RandomSwimmingGoal(1.0, 40)` while it `canRandomSwim`.
  - Persistence: removed when far away unless it came from a bucket or has a custom name; `FromBucket` is saved.
  - Health 3; speed is the attribute default 0.7.
- **`WaterAnimal` and `AgeableWaterCreature`**:
  - Basics: malus `WATER` 0, ambient interval 120, experience `1 + nextInt(3)` at any age, not leashable, not pushed by fluid.
  - **Air** (`handleAirSupply`, from the air before `super.baseTick`):
    - alive and not `isInWater` (the body, not the eyes): air goes down by 1;
    - at `≤ −20`: air 0 and 2 `drown` damage;
    - in water: air 300.

    So a stranded 3-health fish is hurt at tick 320 and dies at tick 340.
  - The dolphin's `handleAirSupply` is empty: it breathes with lungs, with 4800 air that refills to the maximum at once. The axolotl has its own air.
  - The surface spawn rule: `seaLevel − 13 ≤ y ≤ seaLevel`, water below, and `minecraft:water` above.
- **Wandering and finding water:**

  | | 26.3 |
  |---|---|
  | `RandomStrollGoal(speed, interval)` | `MOVE`; refuses with a controlling passenger; `noActionTime ≥ 100` refuses; `nextInt(reducedTickDelay(interval)) != 0` refuses; continues while the path runs |
  | `RandomSwimmingGoal` | `getRandomSwimmablePos(10, 7)`: `DefaultRandomPos.getPos(10, 7)` with the swimmer's stable rule, retried up to 10 times until the block is `isPathfindable(WATER)` |
  | `RandomStroll.swim(speed)` (axolotl, frog, tadpole, nautilus brains) | tiers (1,1), (3,3), (5,5), (6,5), (7,7), (10,7): a random swimmable spot, then each tier goes further along the direction to the last candidate; the first one in fluid and not restricted wins, else the last candidate |
  | `TryFindLiquidGoal(#water)` (dolphin) | no flags; on the ground and not in water; the first water block of `x ± 2, y − 2..blockY, z ± 2` (x fastest), moved to as the block corner at speed 1 |
  | `TryFindLiquid.create(range, speed, tag)` (axolotl brain, dolphin) | with no attack or walk target, not in the fluid. Before its clock: the clock becomes time + 22. Otherwise a Manhattan scan for liquid blocks that are not in its own column: the first with air above, else the first not within 1.5 blocks. It sets the walk target, and the clock becomes time + 40 |

- **Buckets:**
  - **Items.** Cod, salmon, tropical fish and pufferfish buckets are `MobBucketItem(type, WATER, item.bucket.empty_fish)`; the axolotl and tadpole buckets have their own `empty_*` sounds. `sulfur_cube_bucket` has an `EMPTY` fluid. All stack to 1 and start with `bucket_entity_data` `{}`. The fish buckets also carry `minecraft:food` with no `consumable` component (that is for the nautiluses group).
  - **`BucketItem.use`**: ray-cast with `getFluidContext()` (`NONE` for mob buckets). `posToPlace` is the clicked block when that block is a `LiquidBlockContainer` and the content is water, else the block against the clicked face. Then `emptyContents`, `checkExtraContent` and `createFilledResult(.., getEmptySuccessItem)`. The result is a bucket, or the same stack in creative.
  - **`MobBucketItem`**:
    - an `EMPTY` fluid only plays the empty sound;
    - `playEmptySound` plays only the item's own sound; an evaporating pour hisses instead;
    - `checkExtraContent` spawns the mob and posts `entity_place`;
    - `spawn`: `EntityType.create(.., posToPlace, BUCKET)` with the item's components (name and variants), then `loadFromBucketTag`, `setFromBucket(true)`, and its ambient sound.
  - **`Bucketable.bucketMobPickup`**:
    - only a `water_bucket`, on a living mob;
    - the fill sound (`item.bucket.fill_fish` or the mob's own `fill_*`);
    - `saveToBucketTag` (`CUSTOM_NAME` onto the item; `NoAI`, `Silent`, `NoGravity`, `Glowing`, `Invulnerable` and `PersistenceRequired` when true; `Health` as a float);
    - `createFilledResult(.., false)`: creative keeps the water bucket and adds the fish bucket once;
    - `dropLeash`, then `discard`.
  - **Family extras:**
    - salmon: the `SALMON_SIZE` component;
    - tropical fish: three pattern components;
    - axolotl: `AXOLOTL_VARIANT`, `Age`, `AgeLocked` and `HuntingCooldown`;
    - tadpole: `Age` and `AgeLocked`.

### Plan

**New files** (each must pass `rustfmt --check`; no comments):

| File | Holds |
|---|---|
| `entities/src/world/families/water/swim.rs` (`pub(crate) mod swim;` in the water group file; not a crate-root module, because D0-3 keeps entities `lib.rs` closed to the mob tracks, and `crate::world::families::water::swim` still reaches every caller in the crate) | the water-navigation glue; `SmoothSwimmingMoveControl`; `FishMoveControl`; `SwimFrame { yaw, pitch, body_yaw, head_yaw, speed, forward, up }`; `random_swimmable_position`; `swim_stroll_target`; `TryFindLiquid` (brain clock); `flop`; `water_walk(&mut WalkProfile)` (`WATER` 0, `WalkTarget::Zero`, no float) |
| `entities/src/world/families/water/swimming.rs` (under `world/` for `play_movement`) | `swim_ai_step`; `RandomSwimmingGoal`; `TryFindLiquidGoal`; `SwimGoals` in `GoalExtra` (the wanted spot `TryFindLiquidGoal` hands to the move control); `#[cfg(test)] swim_probe.rs` |
| `crates/minecraft_terrain/src/families/water/buckets.rs` | the `Bucketable` trait, `pickup::<S>`, `bucket_use`, and the seven `BUCKETS` rows (item, kind, water, fill and empty sounds, extra keys, component converters that start empty and are filled by the family groups) |
| `world/src/level/mob_bucket.rs` (`pub mod mob_bucket;` after `pub mod dispenser;`, `level/mod.rs:13`) | `Level::empty_mob_bucket` |

**Registration lines (M2's own files):**
- entities `families/water.rs`: `pub(crate) mod swim;` and `pub(crate) mod swimming;` (`swim` in `water-swimming`), and `#[cfg(test)] swim_probe: swimming::swim_probe::SwimProbe,` (key 79);
- terrain `families/water.rs`: `pub(crate) mod buckets;` and `pub(crate) const IN_WATER: u32 = 1;`;
- world `natural_spawner/families/water.rs`: nothing yet. `squids-live` (M2-2) adds the first rows there with the shared `surface_water_animal` rule and the `InWater` obstruction line.

If D0's terrain `families.rs` declares `mod water;`, make it `pub(crate)` in its own logged commit: `crate::bucket` and `crate::server` call into it.

**New root hooks in `entities/src/world/families.rs`** (D0-12: each in its own commit; M20 lists the owners):
- `Family::breath(mob) -> Breath { Lungs { max: i32, refill: i32 }, Gills, Own }`, default `Lungs { max: living::MAX_AIR, refill: 4 }`:
  - `Lungs` is today's `living::breathe`, moved into a new `living::breathe_up_to(.., max, refill)` beside `breathe` (`world/living.rs:15`), which `breathe` now calls. The dolphin answers `Lungs { max: 4800, refill: 4800 }` (vanilla's `getMaxAirSupply` and `increaseAirSupply`), so it still drowns;
  - `Gills` is the `WaterAnimal` rule above (`Hurt` 2 of `minecraft:drown`);
  - `Own` leaves the air alone (axolotls' and nautiluses' drying, in their own ticks).
- `Family::swim_sounds(kind) -> Option<(&'static str, &'static str)>` (swim, splash; `None` keeps the category's, an empty swim sound means none, as the guardian's `Some(("", "entity.hostile.splash"))`) and `Family::quiet_steps(kind) -> bool`. D0's `movement_sounds::<S>` uses them, and quiet steps become `step: Some("")` with `with_step_volume(0.0)`. **One changed line, `emissions.rs:64`:** the own-step arm pushes only when `step_volume > 0.0`. The step emission and W's step game event (at `:55`) still run.
- **Not added here** (M20 owners): M1 rabbits' `family_goal_step_with(.., run)` (`run` has `monster_ai_step`'s signature), which `water-swimming` waits for. No track adds an ambient hook this round (M1's brief drops `Family::ambient`), and M2 does not need one (below).

**Hooks the families will set:** `breath`, `swim_sounds`, `quiet_steps` and `monster` (a `MonsterSpec` with `walk: swim::water_walk`). Each water family sets `core.ambient_interval` (120 for water animals) in `spawn`, and rewrites or drops the `Voice::Ambient` that `family_base_tick` pushes, right after the base tick. `experience` is `1 + next_int(3)` at any age: every water family overrides it, because D0's default gives water categories 0. Fish set `despawn` to `removable: !from_bucket`.

**Shared files, one change each, beside a related arm:**

| File | Change |
|---|---|
| `walk_path.rs` | `WalkProfile.amphibious: Option<bool>` (shallow; `None` in `animal`); the `isAmphibious` branches in `start` (`:324`), `neighbors` (`:382`), `floor_level` (`:468`) and `find_accepted_node` (`:476`); `WATER_BORDER` in `path_type_of_mob` (`:631`) |
| `navigation.rs` | `pub water: WaterNav { Ground, Bound { breaching }, Amphibious { shallow } }` after `wall_climber` (`:278`); `tick_water` after `tick_amphibious` (`:532`); one branch in `recompute` (`:323`) |
| `stroll.rs` | `water_bound_random_position(.., home: Option<(BlockPos, i32)>)` after `amphibious_random_position` (`:199-221`); the home restriction refuses spots outside it (`GoalUtils.isRestricted`), as the elder guardian and the tame nautilus need |
| `look.rs` | `LookControl::tick_smooth_swimming` after `set_look_at_with_limits` (`:133`), because `x_max_rot_angle` is private. M1's `tick_from` goes beside `:144` |
| `movement.rs` | `Body::travel_swimming(world, input, yaw, acceleration)` = `travel_water_mode(acc, 0.9, false)` after `travel_drowned_swimming` (`:275-278`), before the private `travel_water_mode`. Fish use 0.01, guardians and turtles 0.1, dolphins, axolotls, frogs and nautiluses their speed. M1's turtles use it (no `travel_turtle`) |
| `bucket.rs` | `Change::MobBucket { pos, water, kind, data: Tag }` beside `Server` (`:146`); first line of `bucket_use` (`:297`) calls `families::water::buckets::bucket_use`; `pour` becomes `pub(crate)`; the test arm at `:548` |
| `actions.rs` | an arm beside `Change::Server` (`:1167`) calling `entities.empty_mob_bucket(..)` |
| survival `entities.rs` | `empty_mob_bucket` beside `item_on_block` (`:994`): `y_rot` drawn as `hatch_egg` draws it (`:1196`), `item_use_waiting = Some(sent)` |
| terrain `server.rs` | `Command::EmptyMobBucket` beside `ItemOnBlock` (`:3408`), its sender beside `:3790`, its dispatch beside `:4073` (into `out.item_uses`), `ServerSim::empty_mob_bucket` beside `:2485` |
| `dispenser.rs` | an arm after `"water_bucket" \| "lava_bucket"` (`:93`) for the six water mob buckets (cod, salmon, tropical fish, pufferfish, axolotl, tadpole). `OTHER_BEHAVIOURS` (`:17-26`) stays as it is: the earlier arm wins, and its lines also hold M1's shears and glass bottles. The sulfur cube's own behaviour stays unported |

**The search split.** Vanilla's `PathFinder` takes any `NodeEvaluator`, but here the loop lives inside the walk evaluator. M2 owns this split. d0b's flying search builds on `path_search.rs` (D0-6.2); only if d0b split `walk_path.rs` itself and merged first, plug in there and skip the refactor. Otherwise `water-paths` starts with a pure refactor: the loop becomes generic over a private `NodeSource`, and the walk tests and `bin/check_*` fixtures prove nothing changed. `SwimEvaluator` then sits beside the walk evaluator, so `Node` and `Heap` stay private. `isPathfindable(WATER)` is "holds water, not lava" (as `bucket.rs:203`); javap the overrides for doors, fence gates and trapdoors during the work.

**`swim_ai_step`** runs through `family_goal_step_with` in `monster_ai_step`'s order. Its parameter is a `SwimControls` constant: movement `Smooth(..) | Fish`, look `Smooth(n) | Plain`, acceleration `Fixed(f) | Speed`, `sink_without_target`. M1's `run` has `monster_ai_step`'s signature, so each family passes a two-line wrapper (`fn fish_step(..) { swim_ai_step(&FISH_SWIM, ..) }`) rather than a closure.

**Which navigation a goal family gets.** `MonsterSpec.walk` sees only the `WalkProfile`, and `MonsterKind`/`of_kind` stay untouched, so the family sets `ai.state.navigation.water` (`MonsterGoalContext.navigation`, `monster_ai.rs:507`) on its own `MonsterAi` in `spawn`, right after building it (as `MonsterAi::new` sets `wall_climber` at `:936`). A loaded mob goes through `spawn` too, so nothing is saved for it.
- No light idle time.
- Navigation through `tick_water`.
- The move control writes the pitch into `look.pitch`; the smooth look keeps it.
- Travel gets `(xxa, yya, zza)`: `travel_swimming` in water, the existing travel elsewhere.
- Then `play_movement` and the body rotation.
- Keep the input fade (`:1062`); javap `LivingEntity.applyInput` to see whether `yya` fades too.

**Mob buckets:**
- **Emptying.** It aims like water, the sulfur cube against the clicked face. The sound is the row's `empty_*`, or the hiss in the Nether. The result is an empty bucket. The data is `json_tag` of the stack's `bucket_entity_data` plus the row's converted components.
- **On the server, `Level::empty_mob_bucket`:**
  1. `empty_bucket_into` for water;
  2. `summon_mob(kind, centre, None, y_rot)`, which runs the creature finalize;
  3. merge only the default keys, the row's keys and `FromBucket: 1b`;
  4. push to `Level.spawned`.

  An unsimulated kind waits dormant, like a `/summon`.
- **Pickup**, called first in a bucketable family's `TerrainFamily::interact`:
  - only the family's `Bucketable::PICKUP_ITEM` (default `minecraft:water_bucket`; the sulfur cube's is `minecraft:bucket`, M2-6), on a living mob;
  - the data: `NoAI`/`PersistenceRequired` when true, plus `Health` and the row's keys from `S::fields`;
  - the fill sound;
  - `filled_result`, or the creative branch;
  - `drop_leash`, with the lead drops added to `outcome.drops`;
  - `core.removed` (no death, loot or experience).

**Multiplayer:**
- Water families set `FamilyShow.flags` bit 0 = `IN_WATER` from the host's `core.body.touching_water`. A friend's mirror never updates its fluid state, so without this flag a stranded fish would not lie on its side for the friend.
- The pitch travels as `x_rot`.
- No new `VOLATILE` keys (`Air` is global; `FromBucket` is set once).
- `fields` writes `Air` and `Rotation [yaw, pitch]`; `read` restores them.
- Friends act through the host, as with milking.

**Keyframes:** none. **Protocol:** no change, because `ItemOnBlock`, `Command` and `Change` are in-process and not serialized. Note that under "## Protocol" in the log.

### Commit groups

| Key (segment) | Commits |
|---|---|
| `water-hooks` (m2a) | 1. "Registered Minecraft mobs breathe like vanilla water animals" (`Breath` with `Lungs { max, refill }`, `breathe_up_to`, the probe). 2. "Registered Minecraft mobs make their own swim and step sounds" |
| `water-paths` (m2a) | 1. "Minecraft path search takes any node evaluator" (skipped only if d0b already split `walk_path.rs`). 2. "Minecraft mobs find paths through water like vanilla swimmers and amphibians" |
| `water-buckets` (m2a) | 1. "Minecraft mob buckets pour their water and let out the mob they hold" (the `BUCKETS` rows, `bucket_use`, `Change::MobBucket`, the `actions.rs` arm, `Entities::empty_mob_bucket`, `Command::EmptyMobBucket`, `Level::empty_mob_bucket`). 2. "Minecraft fish buckets carry the mob they caught like vanilla" (`Bucketable` with `PICKUP_ITEM`, `pickup::<S>`). 3. "Dispensers empty fish buckets like vanilla" |
| `water-swimming` (m2b, after M1's `family_goal_step_with`) | 1. "Minecraft mobs swim with vanilla smooth swimming and fish controls". 2. "Minecraft swimmers wander the water, head for it on land and flop when stranded" |

`water-buckets` runs before `water-swimming`: it needs no AI step, and the buckets work for dormant kinds.

### Tests

- **`water-hooks`:**
  - `families::tests::a_gill_breather_keeps_full_air_in_water_and_dies_on_land_after_340_ticks`
  - `a_lung_breather_still_drowns_under_water`
  - `an_own_breather_keeps_its_air`
  - `a_quiet_stepper_flops_without_a_step_sound_but_swims_with_its_own`
  - `a_lung_breather_with_4800_air_refills_at_once_and_still_drowns` (`Lungs { max: 4800, refill: 4800 }`)
  - `an_empty_swim_sound_is_silent_but_the_splash_plays`
- **`water-paths`:**
  - `walk_path::tests::the_walk_search_is_unchanged_behind_the_evaluator_split` (if split)
  - `a_swim_path_goes_around_a_glass_wall_through_water_only`
  - `a_swim_path_breaches_only_for_a_dolphin` (an air node costs malus + 8)
  - `a_two_block_swimmer_does_not_fit_a_one_block_tunnel`
  - `the_swim_start_is_half_a_block_above_the_feet`
  - `an_amphibious_path_swims_across_a_pond_and_walks_out` (6, 4, water-only vertical neighbours, shallow +1, water floor `y + 0.5`)
  - `navigation::tests::a_water_bound_mob_reaches_a_node_within_half_a_block_up_or_down`
  - `a_stranded_swimmer_makes_no_path_unless_it_breaches`
  - `it_cuts_a_corner_only_with_a_clear_line`
  - `stroll::tests::a_water_bound_spot_needs_no_floor_but_no_solid_block`
- **`water-swimming`:**
  - `world::families::water::swim::tests::smooth_swimming_turns_ten_degrees_and_pitches_five_up_to_eighty_five` (the `zza`/`yya` split, × 0.02 in water, the turning factor on land, +0.005)
  - `the_smooth_look_tilts_twenty_and_ten_and_drags_the_body_after_ten_degrees`
  - `the_fish_control_lerps_its_speed_by_an_eighth_and_rises_towards_its_target`
  - `try_find_liquid_waits_on_its_clock_and_prefers_water_with_air_above`
  - `the_swim_stroll_target_widens_through_six_tiers`
  - `movement::tests::swimming_travel_keeps_nine_tenths_without_gravity`
  - `world::families::water::tests::a_swim_probe_swims_to_a_spot_in_a_pool_and_never_leaves_it`
  - `a_swimmer_wanders_only_to_water_within_ten_and_seven` (interval 20, 11 tries)
  - `a_stranded_swimmer_heads_for_water_within_two_blocks`
  - `a_stranded_fish_flops_four_tenths_up_with_its_flop_sound`
  - `stroll::tests::a_water_bound_spot_stays_inside_its_home`
- **`water-buckets` 1 (pouring):**
  - `level::mob_bucket::tests::a_cod_bucket_pours_water_and_summons_a_cod_from_the_bucket` (`FromBucket`, `Health`, `NoAI` and `PersistenceRequired` kept; `Pos` and unknown keys ignored)
  - `a_sulfur_cube_bucket_pours_nothing`
  - `a_fish_bucket_in_the_nether_hisses_and_still_leaves_its_fish`
  - `bucket::tests::a_fish_bucket_aims_like_water_with_its_own_splash`
  - `server::tests::an_emptied_mob_bucket_spawns_its_tag_with_the_bucket_data` (no fish is simulated yet in m2a, so the cod reaches `Level.spawned` and stays dormant; `fish-buckets` adds the simulated case)
  - `entities::tests::a_friend_empties_a_fish_bucket_through_the_host`
- **`water-buckets` 2 (pickup).** No water family is registered in terrain yet when this commit runs, so the tests give D0's `Silverfish` a `#[cfg(test)]` `Bucketable` impl and a test-only row:
  - `families::water::buckets::tests::a_picked_up_mob_keeps_its_health_and_vanishes_without_dying`
  - `only_the_family_s_pickup_item_picks_a_mob_up` (a water bucket by default, an empty bucket for a family that says so)
  - `creative_keeps_the_water_bucket_and_adds_one_fish_bucket`
- **`water-buckets` 3 (dispensers):**
  - `dispenser::tests::a_dispenser_empties_a_cod_bucket_and_keeps_the_empty_bucket`
  - `a_sulfur_cube_bucket_is_still_not_dispensed`

### Owner's steps

Setup: survival, Normal, by day, flat ground in a normal world (not superflat), the kit below. Steps 1-7 need segment m2a. Steps 8-13 need the fish (segment m2b), and steps 10-11 and the scooping in 13 also need `fish-buckets` (segment m2c).

Standing on the flat ground, make two pools in the ground and a tunnel between them (the host can run these in survival):
- `/fill ~2 ~-4 ~2 ~11 ~-1 ~11 minecraft:water`: pool A, 10 × 10 and 4 deep;
- `/fill ~18 ~-4 ~16 ~23 ~-1 ~21 minecraft:water`: pool B, 6 × 6, 6 blocks past pool A;
- `/fill ~12 ~-2 ~7 ~20 ~-2 ~7 minecraft:water` and `/fill ~20 ~-2 ~8 ~20 ~-2 ~15 minecraft:water`: a water tunnel 1 high under the ground, with one turn, from pool A to pool B.

1. Pour and pick up a water bucket, pour lava, and fill a cauldron: everything works as before.
2. Hatch a drowned in pool A (4 deep): it still swims after you. Push a cow into pool A: it floats as before.
3. Use the cod bucket on the ground 3 blocks away. A water source appears, you hear the fish-bucket splash (not the plain pour), and you hold an empty bucket. **wm1_cod_bucket**
4. Do the same with the axolotl and tadpole buckets: each has its own splash. The sulfur cube bucket pours no water and has its own sound. Until those families are in the build, their mobs are kept in the save but not shown.
5. `/gamemode creative`, then use the salmon bucket: water appears and you keep the bucket. `/gamemode survival`.
6. Put the pufferfish bucket in a dispenser facing open ground and flip a lever: water flows out, and the dispenser now holds an empty bucket. **wm2_dispenser**
7. Friend window: the friend empties the tropical fish bucket. Both windows show the water and hear the splash; the friend holds an empty bucket.
8. Hatch 8 cod in pool A. They swim in every direction, turning smoothly as they rise and sink, and none stays in a corner. Within two minutes some reach pool B through the tunnel. **wm3_tunnel**
9. Place stone on the ground, empty the second cod bucket on it and take the water back with an empty bucket. The cod lies on its side and flops with a flop sound. After about 16 s it flashes red, and one second later it dies. `/summon minecraft:cod ~2 ~ ~` on the stone: it flops too; pour a water bucket on it within 10 s: it swims off unhurt. **wm4_flop**
10. Use a water bucket on a swimming cod: it vanishes into the bucket with the fill sound. Empty it into pool B: it is back.
11. Punch a cod once, bucket it and empty it: two more punches kill it (a fresh cod takes three).
12. Empty the third cod bucket into pool B and `/summon minecraft:cod` into pool B too. Walk 70 blocks away and come back: the bucket cod is still there, and the summoned one is gone.
13. Friend window: the friend sees the cod swim, tilting nose up and down. A stranded cod lies on its side and flops. When the friend buckets a cod, it vanishes for the host too. **wm5_friend**

### Kit

```
IW4L_MINECRAFT_KIT=minecraft:cod_bucket,minecraft:salmon_bucket,minecraft:tropical_fish_bucket,minecraft:pufferfish_bucket,minecraft:axolotl_bucket,minecraft:tadpole_bucket,minecraft:sulfur_cube_bucket,minecraft:water_bucket,minecraft:water_bucket,minecraft:lava_bucket,minecraft:powder_snow_bucket,minecraft:bucket*16,minecraft:cauldron,minecraft:cod_bucket,minecraft:cod_bucket,minecraft:stone*64,minecraft:dispenser*2,minecraft:lever*2,minecraft:cod_spawn_egg*16,minecraft:salmon_spawn_egg*8,minecraft:drowned_spawn_egg*2,minecraft:cow_spawn_egg*2,minecraft:iron_sword,minecraft:cooked_beef*32,minecraft:torch*64
```

25 stacks, each id checked against the 26.3 item models. The friend uses the same kit.

### Depends on

- **d0a**: all four groups.
- **d0b**: none (M20: d0b's flying search is in `path_search.rs`).
- **M1**:
  - `water-swimming` waits for rabbits' `family_goal_step_with` (segment m2b);
  - M1's turtles use M2's amphibious mode and `travel_swimming` once m2a and m2b are merged, and add no copy of either.
- **W**:
  - `entity_place` and `fluid_place` at `Level::empty_mob_bucket` (M20, W's game events);
  - W's lines at `emissions.rs:55`, `:78` and `:89` stay clear of this section's `:64`.
- **E1, E2**: none.
- **Within M2**:
  - `squids` needs `water-hooks`;
  - `fish`, `frogs-tadpoles`, `axolotls` and `sulfur-cubes` need all four groups;
  - `dolphins`, `guardians` and `nautiluses` need `water-hooks`, `water-paths` and `water-swimming`.

### Open decisions (recommended choice)

1. **Search split**: one evaluator-generic loop in `walk_path.rs`, owned by M2 (`water-paths`); d0b's flying search is in `path_search.rs`.
2. **Hooks shared with M1**: M1 owns `family_goal_step_with`, and M2 waits for it. No ambient hook exists this round; M2 uses `core.ambient_interval` plus a voice rewrite after the base tick.
3. **Unsimulated kinds from a bucket** wait dormant, like a `/summon`, rather than failing like eggs do. Vanilla always spawns them.
4. **Quiet steps** use the zero-volume guard at `emissions.rs:64`: one changed line. A new `MovementSounds` field would also work (`equines.rs:76` builds its literal with `..MovementSounds::creature(None)`, so it would not change, and D0's `emits_sounds` is added that way), but it changes more lines in a file W and D0 also edit.
5. **Bucket data** travels in `Change::MobBucket` to a new `Command`, because `ItemOnBlock` is `Copy`.

---

## M2-2. Squids and glow squids (groups `squids-live`, `squids-drawn`)

Family `squids`, group `water`, hit-key byte **64** (rows 25 and 26 of `docs/round10/mobs.md`). The family has two kinds, `minecraft:squid` and `minecraft:glow_squid`. `GlowSquid extends Squid`, so one store holds both, and the state has a `glow` flag. Lines are `minecraft-survival` at `d1a3d028`; re-check them before editing. Names follow `track-d0.md`, but where D0's merged code differs, follow the code. Squids have no keyframes, no brain and no `MonsterSpec`: they run two simple goals of their own, kept in their state.

### What exists today

- **Dormant.**
  - Neither kind is in `SIMULATED_KINDS` (`crates/minecraft_terrain/src/server_mobs.rs:433`), so `spawn_saved` (`:580`) returns `None` (`:1321`).
  - The eggs already aim into water (`spawn_egg.rs:470-476`), but nothing hatches.
  - `tracking_range` (`mob_tracking.rs:285`) gives the default 5.
- **Spawning.**
  - Categories: squids are `WaterCreature` and glow squids `UndergroundWaterCreature` (`world/src/natural_spawner/spawning.rs:106-107`). The cap is 5 (`:71`) and the despawn distance 128 (`:86-88`).
  - The glow squid has a row (`natural_spawner.rs:317`, `:347`, `:361`, `:365`). Its rule `Rules::GlowSquid` (`spawning.rs:754-758`) is already vanilla's.
  - The squid has no row. A natural spawn logs `natural spawning of minecraft:squid` (`spawning.rs:576`), and `/summon` returns the bare tag (`:931`).
- **Two bugs block water spawns.**
  - `Rules::GlowSquid` is not `creature()` (`natural_spawner.rs:254-256`), so a glow squid falls into `finalize_monster`'s catch-all (`spawning.rs:1336-1338`) and logs a `note_unsupported`.
  - `check_spawn_obstruction` (`natural_spawner.rs:821-850`) refuses any box holding fluid unless the rule is `Strider` or `Drowned`.
  - javap: `WaterAnimal`, `AgeableWaterCreature`, `Axolotl`, `Guardian` and `Drowned` all override `checkSpawnObstruction` with `isUnobstructed` alone, so no `InWater` type tests for liquid.
- **Reuse:**
  - `age::Age` (`entities/src/age.rs:6`, `set` `:34`, `tick` `:78`);
  - `animal::interact` (`animal.rs:12`; the golden dandelion, with `|_| false` as food), and the lock sound `mob_actions.rs:124`;
  - `voice_pitch` (`world.rs:711`), `living::breathe` (`world/living.rs:15`), `fall_damage` (`:104`);
  - `Entity.move`, inlined in `Body::travel_air_flying` (`movement.rs:555-598`);
  - `minecraftoss_player::mth::{sin, atan2}` (`player/src/mth.rs:14`, `:29`);
  - the elastic lead pull (`world/leashes.rs:808-831`; it moves a squid through its velocity);
  - the default lead offset (`leash_render.rs:72-80`), which is vanilla's for squids;
  - the look tag's `Baby` (`mob_tracking.rs:319-326`);
  - render helpers `vex_render.rs:88` (`deformed`), `:97` (`model_root`) and `:137` (the draw shape), plus the light from `strider_render.rs:240-241`;
  - `ambient_particles.rs`: `Kind::EndRod` (`:55`, `:249`, `:373`, `:1076`, `:1114`), `base` (`:82`), the Flame light `glow()` (`:1168`) and `textures()` (`:678`). `generic_0..7` are already loaded.
- **Missing everywhere:** impaling, sign text, and baby-from-egg-on-mob.
- **Tags** that name both kinds (26.3): `#can_breathe_under_water`, `#aquatic`, `#axolotl_hunt_targets` and `#not_scary_for_pufferfish`. Neither is undead nor fire immune.

### Vanilla 26.3 (javap this round)

- **`EntityTypes`:**
  - `SQUID`: `WATER_CREATURE`, `sized(0.8, 0.8)`, `eyeHeight(0.4)`, `clientTrackingRange(8)`.
  - `GLOW_SQUID`: `UNDERGROUND_WATER_CREATURE`, the same size and eye height, `clientTrackingRange(10)`.
  - Both are allowed on peaceful and not fire immune.
  - The baby's `getDefaultDimensions` is `scalable(0.5, 0.5).withEyeHeight(0.37)`, which replaces the age scale.
- **Class and stats.** `Squid extends AgeableWaterCreature extends AgeableMob`, not an `Animal`: no food, no love.
  - Attributes: health 10 (`Mob` defaults otherwise, follow range 16).
  - Experience `1 + nextInt(3)`.
  - `shouldDropExperience` and `shouldDropLoot` are `!isBaby()`.
  - Saved keys: `Age`, `ForcedAge`, `AgeLocked`, plus `DarkTicksRemaining` for the glow squid.
- **`AgeableWaterCreature`:**
  - `checkSpawnObstruction` is `isUnobstructed`.
  - The ambient interval is 120.
  - It is not pushed by fluid. That is moot, because the fork pushes no mob with currents.
  - `baseTick` saves the air, runs `super`, then `handleAirSupply(saved)`.
    - Alive and not `isInWater`: air = saved − 1; at −20 or less, air = 0 and 2 `drown` damage.
    - Otherwise air = 300. Squids never drown under water.
- **`Squid`, vanilla behaviour:**
  - **Leads:** `canBeLeashed` true.
  - **Sounds:** `entity.squid.{ambient,hurt,death,squirt}`, or `entity.glow_squid.*` for the glow squid, at volume 0.4 on `NEUTRAL`.
  - **Movement:** the movement emission is `EVENTS` (no step or swim sound). Gravity 0.08. `travel` is only `move(SELF, getDeltaMovement())`: no input, no drag.
  - **`LivingEntity.aiStep`:** trims velocity below 0.003, runs `serverAiStep` unless dead, then `travel` whenever `isEffectiveAi`, even while dead. NoAI means no travel.
  - **`aiStep` tail** (after `super`, which also ages it):
    1. The `O` fields take the current values. `tentacleMovement += tentacleSpeed` (the constructor draws `1/(nextFloat+1)·0.2`).
    2. If it passes `6.283185307179586`: subtract `6.2831855f`, `nextInt(10)==0` draws a new speed, and entity event 19 resets the client's own phase.
    3. **In water:**
       - if `tm < π`: `f = tm/π`, `tentacleAngle = Mth.sin(f²π)·π·0.25`; if `f > 0.75`, velocity = `movementVector` and `rotateSpeed = 1`, else `rotateSpeed *= 0.8`;
       - otherwise the angle is 0, the velocity × 0.9 and `rotateSpeed *= 0.99`;
       - then `yBodyRot += (−atan2(v.x, v.z)·57.295776 − yBodyRot)·0.1` and `setYRot(yBodyRot)`;
       - `zBodyRot += π·rotateSpeed·1.5` (degrees);
       - `xBodyRot += (−atan2(horiz, v.y)·57.295776 − xBodyRot)·0.1`.
    4. **Out of water:**
       - angle `|Mth.sin(tm)|·π·0.25`;
       - velocity `(0, vy′·0.98, 0)`, where `vy′` is `0.05·(amplifier+1)` under Levitation and `vy − 0.08` otherwise;
       - `xBodyRot += (−90 − xBodyRot)·0.02`.
  - **`hurtServer`:** when `super` returns true and `getLastHurtByMob() != null` (any living attacker, players too, kept for 100 ticks; the killing hit counts), it calls `spawnInk`:
    - the squirt plays at 0.4 with the voice pitch;
    - origin `= rotateVector((0,−1,0)) + pos`, where `rotateVector` is `xRot(xBodyRotO°)` then `yRot(−yBodyRotO°)`;
    - 30 times: `d = rotateVector((r·0.6−0.3, −1, r·0.6−0.3))·(0.3 (baby 0.1) + r·2)`, then `sendParticles(ink, origin + (0,0.5,0), count 0, d, speed 0.1)`, so each particle moves at `0.1·d`.
  - **Goals:** 0 `SquidRandomMovementGoal`, 1 `SquidFleeGoal`. Neither sets flags, so both run, and in one tick the flee vector is written last.
    - **Random:** always usable. Its tick runs on full selector ticks only: `(tickCount + id) % 2 == 0 || tickCount <= 1`, after `noActionTime++` (javap `Mob.serverAiStep`).
      - If `noActionTime > 100`, the vector is `ZERO`.
      - Otherwise, if `nextInt(25) == 0`, or it is not touching water, or the vector's `lengthSqr` is at most `1e−5`, it picks `a = nextFloat·2π` and sets the vector to `(Mth.cos(a)·0.2, −0.1 + nextFloat·0.2, Mth.sin(a)·0.2)`.
    - **Flee:**
      - usable while in water with `lastHurtByMob` set and `distanceToSqr < 100`; it ticks every tick;
      - `d = squid − attacker`, applied only if the fluid at `containing(squid + d)` is water or that block is air;
      - `d.normalize()` is called and **its result discarded**, so `d` keeps its length;
      - `scale = 3 − max(0, (len−5)/5)`, and `d *= scale` when that is above 0;
      - if that block is air, the vertical part is dropped;
      - `movementVector = d/20`, up to about 1 block a tick at 10 blocks;
      - its bubbles are a server-side `addParticle`, which shows nothing, so do not port them.
  - **`finalizeSpawn`:** `AgeableMobGroupData(0.05)`.
  - **Interaction:** no `mobInteract`, so `AgeableMob`'s golden dandelion lock works on babies.
- **`GlowSquid`:**
  - Synced and saved `DarkTicksRemaining`. Any successful hurt sets it to 100, with or without an attacker.
  - `aiStep` counts it down, then adds one `GLOW` particle at `getRandomX(0.6)`, `getRandomY()`, `getRandomZ(0.6)` with zero velocity every tick. That shows on clients only.
- **Spawn rules** (`SpawnPlacements`, from the bootstrap methods). Both are `IN_WATER` with `MOTION_BLOCKING_NO_LEAVES`, have no light rule, walk value 0, are removed when far away, and spawn in clusters of 4.
  - Squid: `checkSurfaceAgeableWaterCreatureSpawnRules`: `sea−13 ≤ y ≤ sea`, `#water` fluid below, a `Blocks.WATER` block above. Dolphins share it.
  - Glow squid: `checkGlowSquidSpawnRules`: `y ≤ sea−33`, `getRawBrightness(pos,0) == 0`, and the block is water.
- **Biome spawners:**
  - Squid (`water_creature`), by weight and group size:
    - lukewarm 10 (1-2) and warm 10 (4);
    - deep lukewarm 8 (1-4);
    - cold and deep cold 3 (1-4);
    - river and frozen river 2 (1-4);
    - ocean, deep, frozen and deep frozen 1 (1-4).
  - Glow squid (`underground_water_creature`): 10 (4-6) in every Overworld biome.
- **Loot:** `entities/squid` gives `ink_sac` and `entities/glow_squid` gives `glow_ink_sac`, 1-3 each, plus 0-1 per Looting level.
- **Render:**
  - `AgeableMobRenderer(SquidModel, BabySquidModel, shadow 0.7)`, textures `entity/squid/{squid,squid_baby,glow_squid,glow_squid_baby}`. Both kinds bake the same two layers. The render type is `entityCutout` (culled).
  - The render state lerps `tentacleAngle`, `xBodyRot` and `zBodyRot`.
  - **`setupRotations` replaces the living one, so a dying squid never tips over.** Its steps:
    1. `translate(0, 0.5 (baby 0.25), 0)`;
    2. `YP(180 − bodyRot)`;
    3. `XP(xBodyRot)`;
    4. `YP(zBodyRot)`;
    5. `translate(0, −1.2 (baby −0.6), 0)`;
    6. the living renderer's `scale(−1,−1,1)` and `translate(0,−1.501,0)`.
  - Glow squid light: `i = (int) clampedLerp(1 − dark/10, 0, 15)`. If `i == 15` the light is 15, else `max(i, normal)`: full bright, dimming over the last 10 dark ticks.
- **Models:**
  - `SquidModel` (64 x 32):
    - `body` `texOffs(0,0)` box `(−6,−8,−6) 12×16×12`, deformation 0.02, offset `(0,8,0)`;
    - eight root children `tentacle<i>`, each `texOffs(48,0)` box `(−1,0,−1) 2×18×2` at `(cos a·5, 15, sin a·5)` with `yRot = (float)(−i·2π/8 + π/2)`, where `a = i·2π/8`;
    - `setupAnim` sets every tentacle's `xRot = tentacleAngle`.
  - `BabySquidModel` (32 x 32):
    - `body` box `(−4,−5,−4) 8×10×8` at `(0,13,0)`;
    - tentacles `texOffs(0,18)` box `(−1,−0.5,−1) 2×6×2` at `(cos a·3, 18.5, sin a·3)`.
- **Particles:**
  - `SquidInkParticle`:
    - no jitter, friction 0.92, size 0.5, no physics;
    - colour black for squid ink, `(0.2, 0.8, 0.6)` for glow ink;
    - lifetime `(int)(6/(r·0.8+0.2))`, sprites `generic_7..0` by age;
    - after half its life `alpha = 1 − (age − life/2)/life`; inside an air block `yd −= 0.0074`.
  - `GlowParticle$GlowSquidProvider`:
    - sprite `glow`, opaque;
    - `Particle`'s jitter around `(0.5−r, 0, 0.5−r)`, then `yd ×0.2`, `xd`/`zd ×0.1`;
    - colour `(0.6,1,0.8)` or `(0.08,0.4,0.4)`;
    - lifetime `(int)(8/(r·0.8+0.2))`, friction 0.96, speeds up when its y is blocked, size ×0.75;
    - the light rises with age like the flame's.
- **Entity events:** 19, client only, and the generic 60. No `puff_for` row is needed.

### Plan

**Files (D0-3):**
- `third_party/minecraftoss/entities/src/world/families/water/squids.rs`:
  - `pub struct Squid { glow, age: Age, x_body_rot(_o), z_body_rot(_o), y_body_rot_o, tentacle_movement(_old), tentacle_angle(_old), tentacle_speed, rotate_speed, movement_vector, fleeing: Option<i32>, ink_spurts: u32, dark_ticks }`;
  - `impl Family`, the tick, both goals and the tests. Put the goals in `water/squids/goals.rs` if the file passes about 800 lines.
- `crates/minecraft_terrain/src/families/water/squids.rs` (`TerrainFamily`) and `families/water/squids/render.rs` (both models).
- Two rows in `world/src/natural_spawner/families/water.rs`.

**Registration lines:**
- `pub mod squids;` and `squids: squids::Squid,` in `family_group!(water 64..=79 { .. })`.
- `mod squids;` and `Squid` in `terrain_group!(water { .. })`.
- The two `ROWS`.

**Hooks (D0-2.2):**
- `KEY` 64; `KINDS` = both kinds.
- `spawn`: `MobCore::new(kind, pos, 0.8, 0.8, 0.4, 10.0)`, `ambient_interval` 120, the tentacle speed from `core.random`.
- `category`: `"water_creature"` or `"underground_water_creature"`. The default `Despawn` (128, removable, allowed on peaceful) is vanilla's.
- `experience`: an adult gives `1 + next_int(3)`, a baby 0.
- `breath`: `Gills` (M2-1, the `AgeableWaterCreature` rule). `breathes_water` and `leashable`: true. `leash_walk` stays a no-op, because vanilla's navigation never moves a squid.
- `emits_sounds`: false (D0's hook for vanilla's movement emission `EVENTS`, M20). Only if the merged d0a lacks it: `quiet_steps` true and `swim_sounds` `Some(("", <the category's splash>))` (M2-1).
- `voice`: `(family, 0.4, "neutral_volume")`.
- `hurt`: `hurt_plain`. On an applied hit with `hurt.attacker` or `core.hurt_by` set, `ink_spurts += 1` and push `Voice::Event("entity.<kind>.squirt", 0.4, voice_pitch)` after the hurt voice. A glow squid also sets `dark_ticks = 100` on any applied hit.
- Everything else uses the defaults.

**Tick**, in vanilla order:
1. `family_base_tick`, which applies the `Gills` air rule from the air before it (M2-1).
2. NoAI: run the tail only.
3. Trim the velocity.
4. Alive only: `no_action_time += 1`, the random goal on full ticks, then the flee goal. The attacker's position comes from `players` or `body_of`.
5. `Body::move_self`. A landing passes its fall to `fall_damage`, and the result goes to `S::hurt` as `"minecraft:fall"`. Clear `body.emission`, because `EVENTS` is silent.
6. `family_after_move`.
7. `age.tick`; at the baby/adult boundary, resize the body (0.5/0.37 or 0.8/0.4) and set `core.baby`.
8. The tail (`y_body_rot_o` takes the old yaw, then `core.yaw = y_body_rot`).
9. The glow squid's dark-tick countdown.

A dead squid runs steps 5-9 too, so it drifts on its last pulse.

**Shared pieces this section owns** (M20; the air rule itself is M2-1's `Breath::Gills`):
- `Body::move_self(world) -> Option<f64>` (vanilla `Entity.move`), split out of `travel_air_flying` (`movement.rs:555-598`) and placed beside it. M2-6's `BodyPhysics` later edits the same function, after this split.
- `Family::OWN_DYING_TRAVEL` (const, default false) in the root `world/families.rs`. It lets `family_base_tick` skip the shared `dying_travel` (drag 0.8, sinking, swim sounds). Commit "Registered Minecraft mobs can move by their own rules while dying" (decision 2). Dolphins and axolotls use it too.
- `natural_spawner.rs:822`: `|| info.placement == Placement::InWater` beside `Rules::Strider | Rules::Drowned` (decision 6). It serves every M2 water row (fish, dolphins, axolotls, guardians, nautiluses).
- `surface_water_animal`, the shared `FamilyRule` in `natural_spawner/families/water.rs` (`seaLevel − 13 ≤ y ≤ seaLevel`, `#water` fluid below, a `minecraft:water` block above; vanilla's `checkSurfaceWaterAnimalSpawnRules` and `checkSurfaceAgeableWaterCreatureSpawnRules` are the same test). Squids, cod, salmon, pufferfish and dolphins use it.

**Spawn rows:**
- `minecraft:squid`:
  - size 0.8 × 0.8, `InWater`, `MotionBlockingNoLeaves`, not fire immune;
  - `Rules::Family { creature: true }`, `Walk::Neutral`, `remove_far`, cluster 4;
  - `WaterCreature`, follow range 16, `baby_chance: Some(0.05)`, no finalize;
  - rule `surface_water_animal`.
- `minecraft:glow_squid`: the same, but `UndergroundWaterCreature` with the rule `dark_deep_water` (the tests of `spawning.rs:754-758`). Rows are checked first, so it overrides the built-in row and fixes the finalize path.

**One-line hooks in shared files**, each placed next to a related arm:
- `crates/minecraft_terrain/src/ambient_particles.rs` (M20: M2's kinds go after `Flame`, because M3's creakings take `EndRod` and E1 the end of `Kind`):
  - `SquidInk`, `GlowSquidInk` and `Glow` after `Flame` (`:53`);
  - their constructors after `flame` (`:132`);
  - their own arms after the `Flame` arms in `add` (`:1074`), the tick (`:1106`), the sprite (`:469`), the size (`:493`) and the light (`:1168`, the glow through `glow()`);
  - the ink's tick arm after `Kind::Flame => particle.rise(),` (`:1106`) is a block like the `Totem` arm (`:1116-1120`): `coast()` (`:362`), then `fade_after_half_life()` (`:338`), then the `yd −= 0.0074` sink when the cell is air (`scene` is in scope there). Do not add an arm inside `coast`'s own match (`:372-384`): M3's `Trail` takes the lines beside `Kind::EndRod` there (`track-m3.md`, `creakings-heart`);
  - the translucent kinds on a new line of `translucent` (`:445-448`), and `"glow"` in `textures()` (`:678`). `generic_0..7` are already loaded.
- Terrain root `families.rs`: `TerrainFamily::tracking_range(kind) -> i32 { Self::TRACKING_RANGE }`, read by `families::tracking_range`. It is its own commit, "Registered Minecraft mobs can have a tracking range per kind" (decision 1), and guardians need it too.

**Multiplayer:**
- **Constants:** `TRACKING_RANGE` 8, with `tracking_range` 10 for the glow squid. `SHADOW_RADIUS` 0.7. The 4 `TEXTURES`.
- **Saving:** `VOLATILE` `["AgeLocked", "DarkTicksRemaining"]` (`Age` is in the base list). `read` and `fields` handle `Age` (the look tag's `Baby` gives `BABY_START`), `ForcedAge`, `AgeLocked`, and the glow squid's `DarkTicksRemaining`.
- **`show`:** `FamilyShow { flags: 0, values: [tentacle_angle, x_body_rot, z_body_rot, y_body_rot as f32 bits; ink_spurts; dark_ticks; 0; 0] }`.
- **`show_on`:** shifts each value into its `_o` field, then stores the new ones, so the host and the friend lerp the same pairs.
- **`interact`:** a golden dandelion on a baby goes through `animal::interact` and returns true. Leads use D0's generic arm.
- **`draw`:**
  - `MobPose` gives the feet, light and overlay, but the yaw comes from the state pair (decision 5);
  - root `T(feet)·T(0,0.5|0.25,0)·Ry(180−yaw)·Rx(xBody)·Ry(zBody)·T(0,−1.2|−0.6,0)`, then `model_root`'s flip, with no death flip;
  - the model goes into `out.culled`, and the glow squid gets its block light rule.
- **`client_effects`** (once per client tick):
  - ink: `fx.seen[id]` holds the last `ink_spurts`. On first sight only record it. Each new spurt draws 30 ink specks from the origin and velocities above with `fx.random` (decision 4);
  - glow: one `Glow` per glow squid per tick.
- **Sounds:** host voices, which reach friends as for every mob.
- **Protocol:** none.

**Commit groups**

| Group | Commits | Contents |
|---|---|---|
| `squids-live` (m2a) | 1. "Registered Minecraft mobs can move by their own rules while dying" 2. "Minecraft squids swim in pulses, squirt ink and flee in the entity world" 3. "Minecraft squids spawn in oceans and rivers, and glow squids in dark water underground" | The hook, then the entity family and `move_self`, then the rows, `surface_water_animal` and the obstruction line. No terrain registration yet, so in-game squids stay dormant. |
| `squids-drawn` (m2a) | 1. "Registered Minecraft mobs can have a tracking range per kind" 2. "Minecraft squids and glow squids are saved, hatch from eggs and are drawn like vanilla" 3. "Minecraft squids leave ink clouds and glow squids sparkle, for friends too" | `TerrainFamily`, the models, glow light, `FamilyShow` and `interact` make the rows `drawn`. The particles and `client_effects` make them `done`, with the steps and kit in the log. |

### Tests

- **`squids-live` 1** (the hook, before any squid exists): `families::tests::a_family_that_moves_its_own_way_while_dying_skips_the_shared_dying_travel` (M2-1's swim probe answers `OWN_DYING_TRAVEL` true; D0's probe keeps the shared drag and sinking).
- **`squids-live` 2** (the family):
  - `a_squid_is_0_8_with_ten_health_and_its_kind`: both kinds, census categories, `mob_candidates`, the loot table for adults only, experience 1-3 or 0.
  - `a_baby_squid_is_half_a_block_and_grows_up_to_full_size`.
  - `a_squid_pulses_only_in_the_last_quarter_of_its_tentacle_stroke`.
  - `a_squid_picks_a_new_heading_on_full_ticks_and_stops_after_a_hundred_idle_ticks`.
  - `a_squid_turns_tilts_and_rolls_with_its_motion`.
  - `a_hit_squid_squirts_and_flees_up_to_a_block_a_tick`: the vector at 1, 5 and 10 blocks; no flee beyond 10 blocks or out of water; no rise toward air.
  - `a_squid_hurt_again_within_five_seconds_squirts_even_without_an_attacker`.
  - `a_squid_on_land_falls_curls_and_dries_out`: gravity, drag, Levitation, the tilt, the air and the drown timing.
  - `a_glow_squid_goes_dark_for_a_hundred_ticks_after_any_hit`.
  - `a_dying_squid_drifts_on_its_last_pulse_and_does_not_sink`.
  - `squids_are_silent_when_they_swim_and_call_every_120_ticks_or_so`.
  - `a_leashed_squid_is_pulled_after_its_player`.
- **`squids-live` 3:**
  - `natural_spawner::families::water::tests::surface_water_animals_spawn_from_thirteen_below_sea_level_up_to_it`
  - `squids_spawn_in_the_top_thirteen_blocks_of_open_water`
  - `glow_squids_spawn_only_in_unlit_water_thirty_three_below_sea_level`
  - `a_squid_group_starts_with_an_adult_and_has_babies_one_time_in_twenty`
  - `water_spawns_ignore_the_water_in_their_box` (the drowned and axolotl rows too)
  - `summoning_a_squid_notes_nothing_unsupported`
- **`squids-drawn` 1** (the hook): `families::tests::a_family_s_tracking_range_by_kind_defaults_to_its_constant` (D0's silverfish and endermites, the only terrain families before commit 2).
- **`squids-drawn` 2:**
  - `a_squid_saves_and_loads_with_its_age_and_a_glow_squid_with_its_dark_ticks`
  - `a_friend_sees_a_squid_from_its_look_tag`
  - `a_squid_tracks_at_eight_chunks_and_a_glow_squid_at_ten`
  - `render::tests::an_adult_squid_has_a_body_and_eight_tentacles_on_64_by_32_and_a_baby_on_32_by_32`
  - `every_squid_picks_its_texture_by_kind_and_age`
  - `a_dying_squid_flashes_red_but_does_not_tip_over`
  - `a_glow_squid_is_full_bright_and_dims_in_its_last_ten_dark_ticks`
  - `a_golden_dandelion_locks_a_baby_squid_s_age`
- **`squids-drawn` 3:**
  - `ambient_particles::tests::squid_ink_is_black_glow_ink_teal_and_sinks_slowly_in_air`
  - `a_glow_sparkle_drifts_brightens_and_lives_eight_to_forty_ticks`
  - `a_squirt_draws_thirty_specks_behind_the_squid_once_and_none_on_first_sight`
  - `mob_tracking::tests::a_friend_sees_a_squid_squirt_and_a_glow_squid_go_dark_at_the_same_tick`

### Owner's steps (survival, Normal, by day; `/locate biome minecraft:lukewarm_ocean`, `/tp` there, and stand on a 5 x 5 glass platform on the sea surface)

1. Use the squid egg on the water 6 blocks away three times. Dark blue squids swim in pulses: the eight tentacles snap shut, the squid shoots forward, then it glides, slows and slowly rolls, tilted toward where it goes. **squid1_swim**
2. Hit one with the sword. A black ink cloud bursts from its rear with a squirt sound, and it darts away fast. Shoot one with the bow and one with the M4A1 from 8 blocks: each inks and flees. **squid2_ink**
3. Kill one: it flashes red without tipping over, vanishes in a poof, and leaves 1-3 ink sacs and an orb.
4. Use an egg on the glass. The squid lies there with its tentacles beating and slowly tips flat. At about 16 s it flashes red, then once a second, and it dies at about 20 s (five hits of 2). An egg used 3 blocks above the glass falls. **squid4_land**
5. `/summon minecraft:squid ~ ~-2 ~4 {Age:-24000}` gives a small, round baby with short tentacles. Use a golden dandelion on it and listen for the lock sound. Kill a second baby: no drops and no orb. **squid5_baby**
6. Put a lead on a squid and walk along the platform: it is pulled after you. At 13 blocks the lead snaps.
7. `/time set midnight`. Use the glow squid egg on the water: a teal squid glows at full brightness, with tiny sparkles drifting off it. **squid7_glow**
8. Hit the glow squid: teal ink, then it is lit only like the water around it for 5 s, and it brightens again within half a second. **squid8_glow_dark** Kill it: 1-3 glow ink sacs. `/time set day`.
9. Stand on land 30 blocks from the ocean for a minute: squids appear near the surface in ones and twos, sometimes with a baby. Optionally, in a flooded cave below y 30 with no light, glow squids appear in groups of 4-6.
10. Leave two squids, the locked baby and a glow squid alive. Save, quit and load: all four are there and swimming, and the baby is still small.
11. `/gamemode creative`, place the spawner and use a squid egg on it: a small squid hangs in the cage. `/gamemode survival`.
12. Two windows (`scripts\mp-local.ps1`). The host hatches two squids and a glow squid.
    - The friend sees the same pulses, tilts, rolls and tentacle beats at the same moments, and the glow sparkles.
    - The host hits a squid: the friend sees the ink cloud in the same place and hears the squirt.
    - The friend hits the glow squid: it inks, goes dark in both windows and flees from the friend.
    - The friend kills a squid with the M4A1: the ink sacs drop in both windows.
    - **squid12-host**, **squid12-friend**

### Kit

```
IW4L_MINECRAFT_KIT=minecraft:squid_spawn_egg*16,minecraft:glow_squid_spawn_egg*16,minecraft:iron_sword,minecraft:bow,minecraft:arrow*64,minecraft:lead*4,minecraft:golden_dandelion*4,minecraft:glass*64,minecraft:water_bucket,minecraft:water_bucket,minecraft:water_bucket,minecraft:water_bucket,minecraft:cobblestone*64,minecraft:torch*64,minecraft:spawner,minecraft:cooked_beef*32,minecraft:diamond_pickaxe,minecraft:snowball*16,minecraft:bucket
```

19 stacks, every id checked against the 26.3 `assets/minecraft/items`. The friend needs the same line.

### Depends on

- **D0 d0a:** `registry-core`, `registry-spawning`, `registry-client` (`FamilyShow`) and `registry-draw` (`draw`, `interact`, `client_effects`, `ClientFx.seen`). Not d0b, and no keyframes.
- **M2 `water-hooks` (M2-1):** `Breath::Gills` and `quiet_steps`. Squids need no water navigation, swim controls or buckets, so they run in m2a before `water-swimming`.
- **E1:** both tracks edit `ambient_particles.rs`, E1 at the end of `Kind`, M2 after `Flame`. Squids need no `puff_for` row.
- **E2:** thrown things hit squids through `hit_boxes`.
- **W (optional):** squid moves post `swim` game events (`EVENTS`). Whichever of W and `squids-live` merges second adds that one line where `body.emission` is cleared.
- **Other M2 groups:**
  - `guardians` target squids with their own target goal and use the per-kind range;
  - `axolotls` hunt squids through their tag;
  - fish and dolphins use `surface_water_animal` and the obstruction line;
  - dolphins and axolotls use `OWN_DYING_TRAVEL`.

### Open decisions (recommended choice)

1. **Tracking range per kind:** add the `TerrainFamily::tracking_range(kind)` hook (8 and 10), not 10 for both.
2. **Dying travel:** add `Family::OWN_DYING_TRAVEL`. A dead squid then drifts like vanilla instead of sinking with swim sounds; dolphins and axolotls need it too.
3. **Animation:** the host computes the tentacles, tilt and roll and sends them in `FamilyShow`, so both windows show one pose. Vanilla's client clock draws its own random speed and cannot match between windows.
4. **Ink:** send a spurt counter and let each view draw its own 30 specks (E1's rule for puffs), rather than shipping velocities.
5. **Drawn yaw:** take it from the state's `y_body_rot` pair, not `MobPose.body_rot`, which eases a still mob toward its head yaw (`client_mobs.rs:703-718`); squids have no such ease.
6. **Spawn obstruction:** one generic `Placement::InWater` arm, which javap supports for every `InWater` type, rather than a flag per row.

---

## M2-3. Fish: cod, salmon, tropical fish and pufferfish (groups `fish-live`, `fish-spawning`, `fish-drawn`, `fish-pufferfish`, `fish-buckets`)

Family `fish`, group `water`, hit-key byte **65** (rows 27-30 of `docs/round10/mobs.md`). It is one store with four kinds (`minecraft:cod`, `minecraft:salmon`, `minecraft:tropical_fish`, `minecraft:pufferfish`), because all four are vanilla `AbstractFish`. The tadpole is one too (javap: `Tadpole extends AbstractFish`), so the shared part goes in `world/families/water/abstract_fish.rs` for M2's `frogs-tadpoles` to reuse. Lines are `minecraft-survival` at `d1a3d028`; check them again before editing. Where D0's code differs from `track-d0.md`, follow the code.

### What exists today

- **Dormant.**
  - The fish are not in `SIMULATED_KINDS` (`crates/minecraft_terrain/src/server_mobs.rs:433`).
  - `tracking_range` (`mob_tracking.rs:285`) gives them `DEFAULT_RANGE` 5 (`:13`); vanilla uses 4.
- **Spawning picks fish and then gives up.**
  - `MobCategory::of_type` already files all four as `WaterAmbient` (`world/src/natural_spawner/spawning.rs:108`; cap 20 `:70`; despawn distance 64 `:86-88`), and `#reduced_water_ambient_spawns` is honoured (`:661-666`).
  - `type_info` has no fish (`natural_spawner.rs:273-324`), so each pick logs `note_unsupported` (`spawning.rs:575-577`).
  - `Placement::InWater` exists (`natural_spawner.rs:190-195`, `:697-700`). But `check_spawn_obstruction` refuses any fluid in the box except for striders and drowned (`:821-843`); vanilla `WaterAnimal.checkSpawnObstruction` is `isUnobstructed` only.
- **Reuse:**
  - `Body::travel_drowned_swimming` (`entities/src/movement.rs:275`) is exactly `AbstractFish.travelInWater`'s 0.01, 0.9, no gravity;
  - `living::breathe` (`world/living.rs:15`) and `MovementSounds` (`world/emissions.rs:13-33`);
  - `AvoidEntityGoal` (`monster_ai.rs:1735`, a `MonsterGoalContext` goal that already avoids players); `PanicState::can_use` (`panic.rs:15`); `stroll.rs:131`, `:224`;
  - the slime's `playerTouch` pass (`world/slimes.rs:338-357`, into `player_hits` `world.rs:2552`); `mob_hits_mob` (`world.rs:4179`); `mob_effects_mut` (`world/witches.rs:478`); `EntityWorld.level_random` (`world.rs:2581`).
- **Items:**
  - eggs used on water spawn in the water (`spawn_egg.rs:97-109`);
  - the cow's `createFilledResult` (`entities/src/cow.rs:148-181`, from `mob_actions.rs:191-201`); `MobOutcome` (`mob_actions.rs:28`); item components are JSON (`player/src/inventory.rs:21`);
  - fish buckets are listed as "unsupported" dispenser behaviours (`world/src/level/dispenser.rs:17-26`, `:104-107`).
- **Loot:** the four tables need nothing new. `furnace_smelt` and `is_on_fire` work (`entities/src/loot.rs:524`, `:662`); attacker enchantments are never modelled (`:710-712`).
- **Render:** `DYE_DIFFUSE` (`wolf_render.rs:22`, public), tinted cubes `cow_render.rs:433`, `:474`, `:512` (`posed_cube::Skin` has no tint), small ports `bat_render.rs:134`, `vex_render.rs:137`.

### Vanilla 26.3 (javap this round)

- **`EntityTypes`** (all `WATER_AMBIENT`, `clientTrackingRange(4)`, peaceful allowed, not fire immune):

  | Kind | Size | Eye | Scaled by |
  |---|---|---|---|
  | cod | 0.5 × 0.3 | 0.195 | |
  | salmon | 0.7 × 0.4 | 0.26 | its size: small 0.5, medium 1, large 1.5 |
  | tropical fish | 0.5 × 0.4 | 0.26 | |
  | pufferfish | 0.7 × 0.7 | 0.455 | its puff: 0 → 0.5, 1 → 0.7, 2 → 1.0 |

  The scale applies to eye heights too.
- **Attributes:** all four use `AbstractFish.createAttributes`: health 3, the attribute default speed 0.7, follow range 16.
- **`WaterAnimal`:**
  - malus `WATER` 0; ambient interval 120; experience `1 + nextInt(3)`; not pushed by fluid, not leashable.
  - `baseTick` keeps the air from before `super.baseTick()`, then `handleAirSupply`: alive and not `isInWater` → that air − 1; at ≤ −20 (`shouldTakeDrowningDamage`) air 0 and `drown` 2. In water, air 300. All four are in `#can_breathe_under_water`. A fish on land is hurt at tick 320 and dies at 340.
  - `checkSurfaceWaterAnimalSpawnRules`: `seaLevel − 13 ≤ y ≤ seaLevel`, the fluid below is water and the block above is `minecraft:water`.
- **`AbstractFish`** (`Bucketable`):
  - saved `FromBucket`; `requiresCustomPersistence` = from-bucket; `removeWhenFarAway` = `!fromBucket && !hasCustomName`; cluster 8; navigation `WaterBoundPathNavigation`.
  - Goals: 0 `PanicGoal(1.25)`, 2 `AvoidEntityGoal(Player, 8, 1.6, 1.4)` (`forCombat`: creative players are ignored), 4 `FishSwimGoal` = `RandomSwimmingGoal(1.0, 40)` while `canRandomSwim`. No targets.
  - `travelInWater`: `moveRelative(0.01)`, move, ×0.9, `dy −= 0.005` (it never has a target).
  - `aiStep` before `super`: out of water, `onGround` and `verticalCollision` → velocity += `((r·2−1)·0.05, 0.4, (r·2−1)·0.05)` from its own random, off the ground, `needsSync`, flop sound.
  - `mobInteract`: `bucketMobPickup` first. Swim sound `entity.fish.swim`; `playStepSound` does nothing.
- **`FishMoveControl.tick`:**
  1. Eye in water → `dy += 0.005`.
  2. Not `MOVE_TO`, or navigation done → speed 0.
  3. Otherwise `speed = lerp(0.125, speed, modifier × 0.7)`, `vy += speed·(dy/dist)·0.1`, `yRot = rotlerp(yRot, atan2(dz, dx)·57.2957763671875 − 90, 90)`, `yBodyRot = yRot`.
- **`AbstractSchoolingFish`** (cod, salmon, tropical):
  - `leader` and `schoolSize` (1) are not saved; `canRandomSwim` = `!isFollower`; max school (= cluster) 8, salmon 5.
  - `canBeFollowed` = followers and `schoolSize < max`. `inRangeOfLeader`: distance² ≤ 121. `pathToLeader`: `moveTo(leader, 1.0)`.
  - `tick`: with followers, `level.random.nextInt(200) == 1` and no other same-class fish in its box grown by 8 → `schoolSize = 1`.
  - `finalizeSpawn`: no group data → `SchoolSpawnGroupData(this)`; otherwise it follows `data.leader`.
  - **`FollowFlockLeaderGoal`** (priority 5, no flags):
    - `nextStartTick = reducedTickDelay(200 + nextInt(200) % 20)`.
    - `canUse`: false with followers; true as a follower; otherwise count down to 0, reset, then take the same-class fish in the box grown by 8 that `canBeFollowed || !isFollower`. The leader is any one that `canBeFollowed`, else itself; it recruits up to `max − schoolSize` non-followers other than itself. Return `isFollower`.
    - continue while a follower in range; `stop` → `stopFollowing`; `tick` → `pathToLeader` every `adjustedTickDelay(10)`.
- **`Salmon`:** variant small (id 0, ×0.5), medium (1, ×1, default), large (2, ×1.5); `BY_ID` clamps. Saved as the string `type`. `finalizeSpawn` uses its own random: small 30, medium 50, large 15. The bucket copies `minecraft:salmon/size`.
- **`TropicalFish`:**
  - `Variant` int = `pattern.packedId | baseColor.id << 16 | patternColor.id << 24` (default 0 = kob white/white). `packedId = base.id | index << 8`:
    - small (base 0): kob, sunstreak, snooper, dasher, brinely, spotty;
    - large (base 1): flopper, stripey, glitter, blockfish, betty, clayfish.
  - `finalizeSpawn` (after `super`, **level** random):
    - `TropicalFishGroupData` → its variant;
    - else `nextFloat() < 0.9` → one of the 22 `COMMON_VARIANTS` (javap the static init) and new group data;
    - else `isSchool = false` and a random pattern, base colour and pattern colour.
  - `isMaxGroupSizeReached` = `!isSchool`. The bucket copies `tropical_fish/pattern`, `/base_color` and `/pattern_color`.
  - Rule: water below, `minecraft:water` above, and the biome is in `#allows_tropical_fish_spawns_at_any_height` (= `lush_caves`) or the surface rule holds.
- **`Pufferfish`** (not schooling):
  - `PuffState` saved (read `min(v, 2)`); dimensions follow it.
  - Goal 1 `PufferfishPuffGoal` (no flags): runs while a `LivingEntity` in its box grown by 2 is alive, not a spectator, not a creative player, and not in `#not_scary_for_pufferfish` (turtle, guardians, the fish, dolphin, both squids, tadpole, both nautiluses, sulfur cube). `start`: inflate 1, deflate 0. `stop`: inflate 0.
  - `tick`, before `super`:
    - inflate > 0: puff 0 → `blow_up`, puff 1; inflate > 40 and puff 1 → `blow_up`, puff 2; inflate++.
    - else, puff ≠ 0: deflate > 60 and puff 2 → `blow_out`, puff 1; > 100 and puff 1 → `blow_out`, 0; deflate++.
  - `aiStep`, puff > 0, mobs in its box grown by 0.3 under the same conditions: `hurtServer(mobAttack, 1 + puff)`; if hurt, poison for 60·puff ticks (amplifier 0) and `entity.puffer_fish.sting` (1, 1).
  - `playerTouch` (the player's box grown 1 and 0.5): the same hurt and poison for a player. The sting sound plays for that player only.
  - No ambient sound (no `entity.puffer_fish.ambient` exists).
- **`Bucketable.bucketMobPickup`:**
  - A `water_bucket` on a living fish → `item.bucket.fill_fish` (1, 1) and the fish's bucket with `custom_name` and `bucket_entity_data` (`NoAI`, `Silent`, `NoGravity`, `Glowing`, `Invulnerable`, `PersistenceRequired` when set, always `Health`), plus the kind's components.
  - `createFilledResult(.., false)`: in survival the hand becomes the fish bucket; in creative the water bucket stays and **every** pickup adds a fish bucket.
  - Then `filled_bucket`, the leash dropped, and `discard`.
- **`MobBucketItem.spawn`:** `create(.., BUCKET)`, so `finalizeSpawn` runs first; then the stack's components (size, pattern), `loadFromBucketTag` (flags, `Health`), `setFromBucket(true)`, the ambient sound, `entity_place`, `item.bucket.empty_fish`.
- **Sounds:** `entity.{cod,salmon,tropical_fish}.{ambient,hurt,death,flop}`; `entity.puffer_fish.{hurt,death,flop,blow_up,blow_out,sting}`. **Entity events:** none of their own.
- **Loot:** cod and salmon drop 1 fish (smelted when on fire or by a `#smelts_loot` weapon); the tropical fish and pufferfish drop their item. Each has bone meal at 5% and no killed-by-player condition.
- **Spawning:** `IN_WATER`, `MOTION_BLOCKING_NO_LEAVES`. Cod, salmon and pufferfish use `checkSurfaceWaterAnimalSpawnRules`; the tropical fish uses its own (bootstrap methods 2 and 6). `water_ambient` (weight, count):

  | Biomes | Spawns |
  |---|---|
  | ocean, deep ocean | cod 10, 3-6 |
  | lukewarm ocean | cod 15, 3-6; pufferfish 5, 1-3; tropical 25, 8 |
  | deep lukewarm ocean | cod 8, 3-6; pufferfish 5, 1-3; tropical 25, 8 |
  | warm ocean | pufferfish 15, 1-3; tropical 25, 8 |
  | cold oceans | cod 15, 3-6; salmon 15, 1-5 |
  | frozen oceans | salmon 15, 1-5 |
  | rivers | salmon 5, 1-5 |
  | lush caves, mangrove swamp | tropical 25, 8 |
- **Render** (`entityCutoutNoCull`, 32 × 32, `entity/fish/`). Copy the boxes with javap during the work; the part counts were checked this round:

  | Model | Parts | `setupAnim` | Renderer |
  |---|---|---|---|
  | `CodModel` | 7 | `tail_fin.yRot = −f·0.45·sin(0.6·age)`, f = 1 in water, 1.5 out | shadow 0.3; y by `4.3·sin(0.6·age)`°; out of water `translate(0.1, 0.1, −0.1)` and z 90° |
  | `SalmonModel` | 8 (fins on `body_front`/`body_back`) | `body_back.yRot = −f·0.25·sin(g·0.6·age)`, (f, g) = (1, 1) in, (1.3, 1.7) out | shadow 0.4; y by `f·4.3·sin(g·0.6·age)`°; out of water `translate(0.2, 0.1, 0)` and z 90° |
  | `TropicalFishSmallModel` (A) | 5 | `tail.yRot` as the cod | shadow 0.15; as the salmon with f = g = 1 |
  | `TropicalFishLargeModel` (B) | 6 | as A | as A |
  | `Pufferfish{Small,Mid,Big}Model` | 6 / 11 / 13 | fins `zRot = ∓0.2 ± 0.4·sin(0.2·age)` | model by puff; shadow `0.1 + 0.1·puff`; bob `translate(0, cos(0.05·age)·0.08, 0)` |

  - The salmon's small and large meshes use `MeshTransformer.scaling(s)`: the root scaled by s and moved by `24.016·(1 − s)`, so the fish stays on the ground.
  - The tropical body is `tropical_a`/`_b` tinted by `baseColor.getTextureDiffuseColor`. `TropicalFishPatternLayer` draws `tropical_{a,b}_pattern_{1..6}` by pattern index, tinted by the pattern colour, on a copy with `FISH_PATTERN_DEFORMATION` 0.008.
  - Several fins have zero thickness and negative UV offsets (cod top_fin (20, −6), salmon right_fin (−4, 0)).
  - 17 textures. No keyframe class.

### Plan

**Files (D0-3)**
- `entities/src/world/families/water/fish.rs`:
  - `Fish { variant: FishVariant (Cod | Salmon(size) | Tropical(packed) | Pufferfish), from_bucket, in_water, school, puff, ai: MonsterAi }`;
  - `impl Family` (`KEY` 65, the four `KINDS`, `spawn`, `tick`); the size refresh (body and eye × scale); tests.
- `water/fish/ai.rs`: four `MonsterSpec`s built on `abstract_fish.rs`; the flock and puff goals.
- `water/fish/school.rs`: school bookkeeping.
- `water/abstract_fish.rs`: what every `AbstractFish` shares beyond M2-1: the three fish goals (`FishPanicGoal`, the player avoidance, `FishSwimGoal`), the fish `SwimControls` (`movement: Fish, look: Plain, acceleration: Fixed(0.01), sink_without_target: true`, which gives vanilla's `travelInWater` with its `vy −= 0.005`), and the bucket persistence. `FishMoveControl`, `flop` and `swim_ai_step` are M2-1's (`swim.rs`, `swimming.rs`); this file only uses them.
- Terrain `families/water/fish.rs` and `fish/render.rs`.
- Four `ROWS` in `natural_spawner/families/water.rs`.

**Registration lines:** `pub mod fish;`, `pub mod abstract_fish;` and `fish: fish::Fish,` in `family_group!(water 64..=79 { .. })`; terrain `mod fish;` and `Fish` in `terrain_group!(water { .. })`; the rows; `mobs.md` rows 27-30 (M2 section).

**Hooks:**
- `category` `"water_ambient"`; `experience` `1 + nextInt(3)`.
- `despawn`: `Despawn::of_category("water_ambient")` with `removable = !from_bucket` and `kept = from_bucket`.
- `breath` `Gills`; `breathes_water` true; `quiet_steps` true; `swim_sounds` `Some(("entity.fish.swim", <the category's splash>))`; `voice` (`cod`/`salmon`/`tropical_fish`/`puffer_fish`, 1, `"neutral_volume"`).
- `loot`: `on_fire` from `fire_ticks`, unless D0 sets it.
- `monster(kind)`: the spec (eye as above, follow 16; `walk`: `swim::water_walk` with M2-1's water-bound navigation).
- `spawn`: `core.ambient_interval = 120`.

**The tick:**
1. `family_base_tick`, which applies `Breath::Gills` from the air before it (the 2-point drowning with `minecraft:drown`). The pufferfish drops the `Voice::Ambient` the base tick pushed.
2. The puff step.
3. The school board.
4. M2-1's `swim_ai_step` (through `family_goal_step_with`) with the fish `SwimControls` from `abstract_fish.rs`.
5. The flop (M2-1's `flop`), the touches, then `family_after_move`.

`in_water` comes from `FluidFrame::sample`. The flop, `blow_up`, `blow_out` and sting sounds are `Voice::Event`s.

**Flocks (decision 1).** Goals cannot see or change another fish's school. Before the step, the tick fills `GoalExtra` with a `SchoolBoard`: same-kind fish in the box grown by 8 with `is_follower`/`can_be_followed`, plus its own leader position and followers. `FollowFlockLeaderGoal` makes vanilla's choice on the board, returns `isFollower` in the same tick and writes `recruits`; `stop` writes `stop_following`. The tick applies both to the store after the step.

**Pufferfish touches.**
- Players: the slime's pass (`slimes.rs:345-357`) pushes `PlayerHit { damage: 1 + puff, kind: PlayerHitKind::Typed { damage_type: "minecraft:mob_attack", message: "mob", from: fish, push: ZERO, bypasses_armor: false, bypasses_shield: false, disable_shield_seconds: 0.0, effect: Some((Poison, 60·puff, 0)) }, source: Some(id) }`. D0's arm scales by difficulty (nothing on Peaceful). The sting voice is placed at the player.
- Mobs: `mob_candidates` in the box grown by 0.3, not in the tag → `mob_hits_mob(id, victim, 1 + puff, pos, 0.0)`, then poison through `mob_effects_mut` (its immunity flag). Never `family_strike`: a fish does not swing.

**Shared files:** none of the fish's own. The swim sound and silent steps are M2-1's `swim_sounds` and `quiet_steps`, and the in-water spawn obstruction and `surface_water_animal` are `squids-live`'s. One root hook is this section's (M20 owner), in its own commit at the start of `fish-drawn`:
- **"Registered Minecraft mobs cast their own shadow size"** (the name and subject of M1's brief, which plans the same commit in `armadillos-drawn` and drops it when this one is merged first): `TerrainFamily::shadow_radius(mob) -> f32` (default `SHADOW_RADIUS`, halved for babies as D0 does), read by D0's `shadow_casters` arm. Fish answer by kind and puff; guardians and M1's armadillos and turtles use it later.

**Rows (`fish-spawning`):**
- All: `InWater`, `MotionBlockingNoLeaves`, `Rules::Family { creature: true }`, `Walk::Neutral`, `remove_far` true, `WaterAmbient`, follow 16, `baby_chance: None`; the `EntityTypes` sizes; clusters cod 8, salmon 5, tropical 8, pufferfish 8.
- Rules: `surface_water_animal` (M2-2; `level.sea_level()`; spawners the same) and `tropical_fish_rule` (biome tag through `registries`).
- Finalize:
  - salmon: 30/50/15 → `type`, from the level random (the fork's convention);
  - tropical: group `Some(packed)` gives the variant, otherwise vanilla's draw order from the level random → `Variant`, and `*group = Some(packed)` for a common pick;
  - cod and pufferfish: none.

**Multiplayer:**
- `TRACKING_RANGE` 4; `VOLATILE` `["PuffState"]`; `TEXTURES` 17; `SHADOW_RADIUS` 0.3, with `shadow_radius` by kind and puff.
- `read`/`fields`: `FromBucket`, salmon `type` (default `medium`), tropical `Variant` (default 0), `PuffState` (min 2). `read` refreshes the size.
- `show` = `FamilyShow { flags: bit 0 in water, values[0] puff }`; `show_on` sets them and refreshes the pufferfish's size. Size and pattern come only in the look tag.
- `interact`: the bucket pickup. `client_effects`: none. **Keyframes:** none.

| Group | Commits | Contents |
|---|---|---|
| `fish-live` (m2b) | 1. "Minecraft cod, salmon, tropical fish and pufferfish swim in the entity world" 2. "Minecraft cod, salmon and tropical fish swim in schools" | family, `abstract_fish.rs`, AI, air, flop, sounds, despawn, sizes; the school board |
| `fish-spawning` (m2b) | 1. "Minecraft fish spawn in oceans, rivers and lush caves like vanilla" | rows, rules, finalizes |
| `fish-drawn` (m2b) | 1. "Registered Minecraft mobs cast their own shadow size" 2. "Minecraft fish are saved, hatch from spawn eggs and are seen by friends" 3. "Minecraft cod, salmon, tropical fish and pufferfish are drawn like vanilla" | the shadow hook; `TerrainFamily`, `FamilyShow` (rows → `live`); models, pattern layer, flop pose, textures (rows → `drawn`) |
| `fish-pufferfish` (m2b) | 1. "Minecraft pufferfish puff up and sting what touches them" | puff goal, states and sizes, touches, poison, sounds |
| `fish-buckets` (m2c) | 1. "Minecraft fish can be caught in water buckets and poured back out" | `interact` with M2-1's bucket hooks; components; `FromBucket` on pouring (rows → `done`, steps and kit in the log) |

### Tests

- **live 1:**
  - `a_cod_is_half_a_block_long_with_three_health_and_its_kind`;
  - `salmon_sizes_and_pufferfish_puffs_scale_their_bodies`;
  - `a_fish_swims_with_the_fish_move_control_and_settles_when_idle`;
  - `a_fish_out_of_water_flops_and_drowns_in_seventeen_seconds`;
  - `a_hurt_fish_panics_and_fish_flee_survival_players_within_eight_blocks`;
  - `fish_despawn_beyond_64_blocks_unless_from_a_bucket`;
  - `fish_sound_like_vanilla` (interval 120, a silent pufferfish, fish swim, no steps);
  - `a_cod_drops_cod_and_sometimes_bone_meal_and_a_burning_one_cooked_cod`.
- **live 2:** `cod_gather_into_schools_of_up_to_eight_behind_one_leader` (salmon 5, pufferfish never); `a_follower_lets_go_of_a_leader_more_than_eleven_blocks_away`; `a_lone_leader_forgets_its_school`; `a_follower_does_not_swim_at_random`.
- **spawning** (`natural_spawner::families::water::tests`):
  - `cod_salmon_and_pufferfish_spawn_only_near_the_sea_surface`;
  - `tropical_fish_spawn_at_any_height_in_lush_caves`;
  - `a_salmon_spawns_small_medium_or_large_thirty_fifty_fifteen`;
  - `tropical_fish_groups_share_a_common_variant`;
  - `fish_rows_carry_their_sizes_categories_and_school_clusters`;
  - `a_fish_spawn_is_not_obstructed_by_water`.
- **drawn 1:** `families::tests::a_family_casts_its_shadow_by_kind_and_size` (the default, halved for babies; a puffed pufferfish's grows).
- **drawn 2:** `server_mobs::tests::fish_save_and_load_with_from_bucket_type_variant_and_puff_state` (`PuffState` 7 → 2); `mob_tracking::tests::a_friend_sees_a_fish_from_its_look_tag` (water flag, puff, volatile, range 4).
- **drawn 3:** `render::tests::a_cod_has_seven_parts_and_a_salmon_eight_on_32_by_32`; `salmon_sizes_scale_from_the_feet`; `tropical_fish_draw_a_tinted_body_and_a_tinted_pattern_of_their_shape`; `pufferfish_draw_six_eleven_or_thirteen_parts_by_puff`; `a_fish_out_of_water_lies_on_its_side`.
- **pufferfish:**
  - `a_pufferfish_puffs_up_near_a_survival_player_and_deflates_after_three_and_five_seconds` (nothing for creative players, spectators or fish);
  - `a_puffed_pufferfish_stings_a_touching_player_with_poison` (none on Peaceful);
  - `a_puffed_pufferfish_stings_touching_mobs_but_not_sea_life` (a cow poisoned, a zombie hurt but not poisoned, a cod untouched).
- **buckets:**
  - `a_water_bucket_scoops_a_fish_with_its_size_pattern_and_health` (removed without a death);
  - `creative_scooping_keeps_the_water_bucket_and_adds_a_fish_bucket_each_time`;
  - `a_poured_fish_is_the_same_fish_and_never_despawns`;
  - `a_dead_fish_cannot_be_scooped`.

### Owner's steps (survival, Normal, by day; `/locate biome minecraft:warm_ocean`, `/tp @s <x> 63 <z>`, then a 3 × 3 glass platform at the surface)

Steps 8 and 9 and the scooping in step 13 need `fish-buckets` (segment m2c); the rest need segment m2b.

1. Use each fish egg on the water 4 blocks away: a cod, a salmon, a coloured tropical fish and a small pufferfish swim slowly with waving tails. Use the salmon egg five more times: small, medium and large salmon. **fish1_swimming**
2. Swim toward them: they dart away within about 8 blocks, faster when you are close. In `/gamemode creative` they ignore you; go back to `/gamemode survival`.
3. Hit a cod once with the sword: a red flash and it flees. Kill it: raw cod (sometimes bone meal) and experience. The bow kills a salmon and the M4A1 a tropical fish, with their drops.
4. Use the cod egg six times close together and wait 15 s: they swim as one school behind a leader. **fish4_school**
5. Swim beside the pufferfish: it puffs to half, then a second later to a spiky ball. **fish5_puffed** Touch it: you are hurt and poisoned for 6 s (3 s at half size). Swim away: about 3 s later it is half size, 5 s after that small. Drink the milk.
6. On the platform, build a glass pool 2 × 2 and 1 deep, fill it with a water bucket, and hatch a pufferfish and a cow in it: it puffs, and the cow is stung (red flash, green swirls). A cod in the same pool is never stung.
7. Use a cod egg on the dry glass: it lies on its side and flops with a slap; at about 16 s it takes damage and it dies at about 17 s. **fish7_flop**
8. Right-click a tropical fish with a water bucket: it is gone with a slosh, and you hold a bucket of tropical fish. Pour it into the sea: the same colours come back. A large salmon comes back large. Fly 80 blocks away and back: the poured fish is still there. **fish8_bucket**
9. `/gamemode creative`: a water bucket on a fish keeps the water bucket and adds a fish bucket every time. Place a spawner and use the tropical fish egg on it: a small fish lies turning in the cage. `/gamemode survival`.
10. `/summon minecraft:tropical_fish ~ ~ ~3 {Variant:117506305}`: a tall orange fish with grey stripes. `{Variant:65536}`: a small orange-and-white one. `/summon minecraft:pufferfish ~ ~ ~3 {PuffState:2}`: a full ball that shrinks when nobody is near.
11. Save and quit with a puffed pufferfish, a large salmon, a coloured tropical fish and a poured cod, then load: the same sizes and colours.
12. `/tp` 600 blocks into warm ocean: schools of tropical fish that match within a school, and pufferfish. Cold ocean: cod and salmon. A river: salmon. `/locate biome minecraft:lush_caves`: tropical fish in its pools. **fish12_ocean**
13. Two windows (`scripts\mp-local.ps1`): the friend sees the same fish, colours, sizes, flops and puffs; fish flee the friend; the friend is stung and poisoned; the friend's M4A1 kills one in both windows; the friend scoops a fish and pours it out, and the host sees it. **fish13-host**, **fish13-friend**

### Kit

```
IW4L_MINECRAFT_KIT=minecraft:cod_spawn_egg*16,minecraft:salmon_spawn_egg*16,minecraft:tropical_fish_spawn_egg*16,minecraft:pufferfish_spawn_egg*16,minecraft:water_bucket,minecraft:water_bucket,minecraft:water_bucket,minecraft:water_bucket,minecraft:bucket*16,minecraft:cod_bucket,minecraft:salmon_bucket,minecraft:tropical_fish_bucket,minecraft:pufferfish_bucket,minecraft:glass*64,minecraft:glass*64,minecraft:iron_sword,minecraft:bow,minecraft:arrow*64,minecraft:cow_spawn_egg*4,minecraft:spawner,minecraft:milk_bucket,minecraft:cooked_beef*32
```

22 stacks; every id is in the 26.3 `assets/minecraft/items`. The friend needs the same line.

### Depends on

- **D0 d0a:** `registry-core`, `registry-hooks` (`MonsterSpec`, `GoalExtra`, `Typed`), `registry-spawning`, `registry-client`, `registry-draw`. Not d0b.
- **M2-1, all four groups:**
  - `WaterBoundPathNavigation`/`SwimNodeEvaluator` behind `spec.walk`, used by panic, avoid, swim and `pathToLeader`;
  - `swim_ai_step`, `FishMoveControl`, `RandomSwimmingGoal`, `flop`; `Breath::Gills`; `swim_sounds` and `quiet_steps`;
  - the mob-bucket machinery (the bucket hooks, pouring, components, `bucket_entity_data`).
- **M2-2 (`squids-live`):** the obstruction line and `surface_water_animal`.
- **M1:** none. M1's armadillos and turtles use this section's shadow hook once m2b is merged (M1's brief plans the same commit and drops its copy if this one lands first).
- **E2:** `hit_boxes`, for thrown items. **E1** and **W:** nothing of their own.
- **Later M2 groups:** `frogs-tadpoles` reuses `abstract_fish.rs`, and `axolotls` hunt fish via `Prey::Kinds` (`#axolotl_hunt_targets`).

### Open decisions (recommended choice)

1. **Flocking:** a `SchoolBoard` in `GoalExtra`, applied by the tick. It keeps vanilla's same-tick `canUse`, where a signalling goal would lag a tick.
2. **The leader link and the rare-fish group end:** left for later as above. Add `max_group_reached` only if `FamilySpawn` rows get a defaulted pattern.
3. **Swim sound, silent steps and shadow by kind:** root hooks, each added once by its owner (M20): the sounds in `water-hooks`, the shadow here. Dolphins, axolotls, tadpoles, guardians and M1's turtles use them.
4. **Finalize randomness:** the level random (the fork's convention), with vanilla's odds.
5. **Shared fish code:** `water/abstract_fish.rs`, so the tadpoles reuse it.

---

## M2-4. Dolphins and dolphin's grace (groups `dolphins-live`, `dolphins-drawn`, `dolphins-treasure`, `dolphins-grace`, `dolphins-escort`)

Family `dolphins`, group `water`, hit-key byte **66** (row 31 of `docs/round10/mobs.md`), one kind `minecraft:dolphin`. Lines are `minecraft-survival` at `d1a3d028`; `track-d0` means the D0 worktree at `9eecb2ff`. Re-check every line before editing; where D0's merged code differs from `track-d0.md`, follow the code. Dolphins have no keyframe class (procedural `setupAnim`), no brain and no mount: they run on the goal framework (`MonsterKind::Registered`, D0-2.4) with M2-1's water navigation and `swim_ai_step`.

### What exists today

- **Dormant.** Not in `SIMULATED_KINDS` (`crates/minecraft_terrain/src/server_mobs.rs:433`), so `spawn_saved` returns `None` (`:1321`) and the egg hatches nothing. `tracking_range` gives the default 5, which is vanilla's.
- **Spawning:** only the category `"squid" | "dolphin" => Self::WaterCreature` (`third_party/minecraftoss/world/src/natural_spawner/spawning.rs:107`), cap 5 (`:71`), despawn 128 (`:87`). No size row.
- **The effect is data only:** the HUD row `("dolphins_grace", true, 8954814)` (`crates/hud/src/minecraft_effects.rs:49`) and the colour row (`crates/minecraft_terrain/src/potions.rs:100`, read by `swirls.rs:19`). `EffectKind` (`third_party/minecraftoss/player/src/survival.rs:36-59`, `ALL: [EffectKind; 20]` at `:62`) has no `DolphinsGrace`, so neither do `player_nbt::EFFECTS` (`crates/minecraft_survival/src/player_nbt.rs:18`), `/effect` (`commands.rs:156`) and `player_effect` (`potions.rs:230`).
- **Player swimming:** `movement_iw4::water_move` (`crates/movement_iw4/src/swim.rs:146`) keeps 0.8, or 0.9 while sprinting (`:10-11`, chosen at `:187`). Effects reach movement through `ps.mc_float` (`float.rs:12-13`: levitation in bits 0-15, `SLOW_FALLING` `0x1_0000`). It is set each frame by `minecraft_elytra::step` (`crates/render_anim/src/minecraft_elytra.rs:25-28`) through `set_float` (`crates/sim/src/carrier.rs:314`, `frame.rs:485`) and replicated to friends (`crates/net/src/transport/netfields.rs:368`).
- **Mobs cannot see who swims.** The survival `PlayerView` (`crates/minecraft_survival/src/entities.rs:129`) has `sneaking` (`:140`) but no swimming flag. The mobs' per-player look list `enderman::PlayerView` (`entities/src/enderman.rs:59`) is built at `entities.rs:1506-1517`, kept in `EntityWorld.player_views` (`world.rs:2525`) and read by goals with `ctx.view(id)` (`monster_ai.rs:717`). The host already derives a swim pose: `ps.view_height_current < CRAWLING_BELOW_VIEW_HEIGHT` (`crates/render_anim/src/minecraft_players.rs:677`).
- **Reuse:**
  - goals `AvoidEntityGoal` (`monster_ai.rs:1735`, with `.also`), `MeleeAttackGoal` (`:2239`), `LookAtPlayerGoal` (`:2366`), `RandomLookGoal` (`:2417`) and `HurtByTargetGoal` (`:3044`, `pub(crate)` fields);
  - `stroll::default_random_position_towards` and `_away` (`entities/src/stroll.rs:190`, `:224`);
  - the item entity `World` API `items_in`, `item`, `take_item`, `spawn_item` (`player/src/lib.rs:105-117`; `MobWorld` at `server_mobs.rs:169`), with the piglin pickup as the model (`world/piglins.rs:1015`);
  - `age::Age` (`feed` `age.rs:56`) and `animal::interact` (`animal.rs:12`, the golden dandelion);
  - `living::breathe` (`world/living.rs:15`, `MAX_AIR` 300 at `:10`) and `Body::travel_drowned_swimming` (`movement.rs:275`);
  - `EntityWorld.level_random` (`world.rs:2581`) and `take_entity_events` (`:4069`);
  - `locate::nearest_structure` (`crates/minecraft_terrain/src/locate.rs:144`) and `locate::tags` (`:51`), with `ServerSim.worldgen` (`server.rs:82`); the ender eye already runs this search inside the tick (`eye_of_ender.rs:247`);
  - `ambient_particles::Kind::HappyVillager`, which is vanilla's `SuspendedTownParticle` (`:51`, `:213`, tick `linger` `:420`).
- **Tags (26.3):** `#aquatic`, `#cannot_be_pushed_onto_boats`, `#not_scary_for_pufferfish`. Not undead, not fire immune, not `#can_breathe_under_water`.

### Vanilla 26.3 (javap this round)

- **`EntityTypes.DOLPHIN`:**
  - `WATER_CREATURE`, `sized(0.9, 0.6)`, `eyeHeight(0.3)`, tracking range 5, allowed on peaceful, not fire immune.
  - Baby: 0.585 × 0.39 (`scale(0.65)`), eye 0.09375; `getAgeScale` 0.65.
- **`Dolphin extends AgeableWaterCreature extends AgeableMob`** (no love):
  - Attributes: health 10, speed 1.2, attack 3, follow range 16.
  - `setCanPickUpLoot(true)`; `canBeLeashed` true.
  - Experience `1 + nextInt(3)`; ambient interval 120; WATER malus 0. Babies drop no loot and no experience (`LivingEntity`).
  - `getMaxHeadXRot` and `getMaxHeadYRot` are 1.
  - Controls (M2-1): `SmoothSwimmingMoveControl(85, 10, 0.02, 0.1, true)` and `SmoothSwimmingLookControl(10)`. Navigation: `WaterBoundPathNavigation` with `allowBreaching = mob.is(DOLPHIN)` (BREACH nodes, and its path updates out of water too). `closeToNextPos`: the target within 12.
- **Air:**
  - Max 4800, which is also the constructor's air. `increaseAirSupply` returns 4800, so it refills at once when its eyes leave the water. `handleAirSupply` is empty.
  - So `LivingEntity.baseTick` applies: eyes in water (not a bubble column) cost 1 a tick; at −20 the air becomes 0 and `drown` deals 2.
  - `finalizeSpawn`: air to max, `xRot` 0, `AgeableMobGroupData(0.1)`.
- **`tick`:**
  - NoAI: air to max, return.
  - In water or rain: `Moistness` 2400. Otherwise −1; at 0 or below, `hurt(dryOut, 1)` every tick (1 per 10 ticks after the hurt cooldown).
  - On the ground: velocity += `((r·2−1)·0.2, 0.5, (r·2−1)·0.2)` and `yRot = r·360`.
- **`travelInWater`:** `moveRelative(getSpeed())`, move, × 0.9, and `y − 0.005` without a target. No fluid-falling adjust and no `jumpOutOfFluid`. `AbstractFish.travel` is the same with a fixed 0.01.
- **Goals** (priority): 0 `BreathAirGoal`, 0 `TryFindLiquidGoal(#dolphin_tries_to_find = water)`, 1 `DolphinSwimToTreasureGoal`, 2 `DolphinSwimWithPlayerGoal(4.0)`, 4 `RandomSwimmingGoal(1.0, 10)`, 4 `RandomLookAroundGoal`, 5 `LookAtPlayerGoal(Player, 6)`, 5 `DolphinJumpGoal(10)`, 6 `MeleeAttackGoal(1.2, true)`, 7 `MoveToItemGoal`, 8 `PlayWithItemsGoal`, 8 `FollowPlayerRiddenEntityGoal` (once for `AbstractBoat`, once for `AbstractNautilus`), 9 `AvoidEntityGoal(Guardian, 8, 1.0, 1.0)`. Target: 1 `HurtByTargetGoal(Guardian).setAlertOthers()`. Babies cannot attack. The attack sound is `entity.dolphin.attack`.
  - **BreathAir** (MOVE, LOOK, not interruptable):
    - Runs while air < 140.
    - `start` stops the navigation and calls `findAirPosition`: the first block in `neighborColumn(y .. y+8)` whose fluid is empty or a bubble column and which is `isPathfindable(LAND)`, else y + 8; then `moveTo(x, y+1, z, 1.0)`.
    - `tick` repeats the search, then does an extra `moveRelative(0.02, (xxa, yya, zza))` and move.
  - **SwimWithPlayer** (MOVE, LOOK):
    - Starts when the nearest player passing `forNonCombat().range(10).ignoreLineOfSight()` (alive, not a spectator, any game mode) `isSwimming` and is not its target. A nearer player who is not swimming blocks it.
    - Continues while that player swims within 16 blocks.
    - `start`: `addEffect(DOLPHINS_GRACE, 100)`.
    - `tick`: look at the player (21, 1); within 2.5 blocks stop the navigation, else `moveTo(player, 4.0)`. While the player swims, `level.random.nextInt(6) == 0` gives the grace for 100 ticks again.
  - **SwimToTreasure** (MOVE, LOOK, not interruptable):
    - Starts when `gotFish` and air ≥ 100.
    - `start`: `treasurePos = findNearestMapStructure(#dolphin_located, pos, 50, false)`, then entity event 38; nothing found means `stuck`.
    - Continues until within 4 blocks (at its own Y), stuck, or air < 100.
    - `tick`: once it is not `closeToNextPos` or the navigation is done, pick a spot with `getPosTowards(16, 1, centre, π/8)`, else `(8, 4, π/2)`. A spot that is not water or not `isPathfindable(WATER)` is replaced by `(8, 5, π/2)`. No spot means stuck. Then look (21, 1), `moveTo(spot, 1.3)`, and with 1 in 40 send event 38.
    - `stop`: when reached or stuck, `gotFish = false`. `treasurePos` is not saved.
  - **DolphinJump** (`JumpGoal`, not interruptable):
    - `canUse`: `nextInt(5) == 0`, then for steps 0, 1, 4, 5, 6, 7 along `getMotionDirection`: water, not `#blocks_dolphin_jump` (= `#blocks_motion`), and air at +1 and +2.
    - `start`: velocity += (dx·0.6, 0.7, dz·0.6).
    - Ends on the ground, or in water with `vy² < 0.03` and `0 < |xRot| < 10`.
    - `tick`: the sticky `breached` flag means only a loaded dolphin's first leap plays `entity.dolphin.jump`. Then `xRot = rotLerp(0.2, xRot, 0)` while `vy² < 0.03`, else `atan2(−vy, horizontal)·57.2957763671875`.
    - `stop`: `xRot = 0`.
  - **Items:**
    - `ALLOWED_ITEMS`: no pickup delay, alive, in water.
    - `MoveToItem` (MOVE): starts once its cooldown is past and an item is in its box grown by 8; it `moveTo(first, 1.2)` with `entity.dolphin.play`. Stopping after a drop sets cooldown = `tickCount + nextInt(100)`.
    - `PlayWithItems` (no flags): drops the main-hand stack on its first tick, at `eyeY − 0.3` with pickup delay 40. Its velocity is `(0.3·−sin(yaw)·cos(pitch) + cos(a)·b, 0.3·sin(pitch)·1.5, 0.3·cos(yaw)·cos(pitch) + sin(a)·b)`, with `a = 2π·r` and `b = 0.02·r`.
    - `pickUpItem`: into an empty main hand, with `setGuaranteedDrop`.
- **`mobInteract`:**
  - A `#fishes` item (cod, salmon, both cooked, pufferfish, tropical fish) plays `entity.dolphin.eat`. A baby that can grow eats one and gets `Age::feed`; otherwise `gotFish = true` and one is eaten.
  - Else `AgeableMob`'s golden dandelion.
- **Saved:** `GotFish`, `Moistness` (default 2400), plus `Air`, `Age`, `ForcedAge`, `AgeLocked`, `CanPickUpLoot`, `equipment`, `drop_chances`.
- **Sounds:**
  - Ambient: `entity.dolphin.ambient_water` in water, `.ambient` on land.
  - `.hurt`, `.death`, `.eat`, `.play`, `.jump`; swim `.swim` and splash `.splash`.
  - Neutral source, movement emission ALL.
- **Event 38:** 7 `happy_villager` at `getRandomX(1)`, `getRandomY() + 0.2`, `getRandomZ(1)`, speed `gaussian·0.01`.
- **Client trail:**
  - When in water with velocity² > 0.03, two `dolphin` particles per tick at `(x − v.x·h ± f, y − v.y, z − v.z·h ± g)`. Here `v` is the view vector, `f, g = cos, sin(yRot)·0.3` and `h = 1.2 − 0.7r`.
  - `DolphinSpeedProvider`: the town particle, colour (0.3, 0.5, 1.0), alpha `1 − 0.7r`, lifetime halved, opaque, `generic_0`.
- **Render:**
  - `AgeableMobRenderer(DolphinModel, BabyDolphinModel, shadow 0.7)`, textures `entity/dolphin/dolphin` and `dolphin_baby`.
  - No-cull cutout, the living death flip, `isMoving` = horizontal velocity² > 1e-7.
  - `DolphinModel` (64²):
    - `body` (22,0) (−4,−7,0) 8×7×13 at (0,22,−5).
    - Children of `body`: `back_fin` (51,0) (−0.5,0,8) 1×4×5, rot (π/3,0,0); `left_fin` (48,20, mirrored) (−0.5,−4,0) 1×4×7 at (2,−2,4), rot (π/3,0,2π/3); `right_fin` the same box at (−2,−2,4), rot (π/3,0,−2π/3).
    - `tail` (0,19) (−2,−2.5,0) 4×5×11 at (0,−2.5,11), rot −0.10471976 about x; its child `tail_fin` (19,20) (−5,−0.5,0) 10×1×6 at (0,0,9).
    - `head` (0,0) (−4,−3,−3) 8×7×6 at (0,−4,−3); its child `nose` (0,13) (−1,2,−7) 2×2×4.
  - `BabyDolphinModel` (64², same `setupAnim`):
    - `body` (20,0) (−3,−2.5,−4) 6×5×8 at (0,21.5,0).
    - `head` (0,0) (−3,−3.5,−4) 6×5×4 at (0,1,−4); its child `nose` (0,9) (−1,−1,−2) 2³ at (0,0.5,−4).
    - `left_fin` (34,18) (−0.5,−1.5,−0.5) 1×3×6 at (1.8,0.85,−2.6), rot (0.8727,0,1.7017); `right_fin` (48,18, mirrored) at (−1.8,0.85,−2.6), rot (0.8727,0,−1.7017).
    - `tail` (0,13) (−2,−1.5,0) 4×3×7 at (0,1,4); its child `tail_fin` (22,13) (−4,−0.5,−1) 8×1×4 at (0,0,6).
    - `back_fin` (42,0) (−0.5,−1,1) 1×3×4 at (0,−1,−2.7), rot 0.8727 about x.
  - `setupAnim`: `body.xRot = xRot°`, `body.yRot = netHeadYaw°`. When moving: `body.xRot += −0.05 − 0.05·cos(0.3·age)`, `tail.xRot = −0.1·cos(0.3·age)`, `tail_fin.xRot = −0.2·cos(0.3·age)`.
  - Carrying layer: `f = |xRot|/60`; `translate(0, 1 − f/2, −1 + f/2)` if `xRot < 0`, else `(0, 1 + 0.8f, −1 + 0.2f)`; then the held item.
- **Loot:** `entities/dolphin`: 0-1 cod (+Looting), smelted when on fire; adults only.
- **Spawning:**
  - `IN_WATER`, `MOTION_BLOCKING_NO_LEAVES`, `checkSurfaceAgeableWaterCreatureSpawnRules` (the squid's rule).
  - `water_creature` spawners, count 1-2: weight 2 in `warm_ocean`, `lukewarm_ocean` and `deep_lukewarm_ocean`; weight 1 in `ocean` and `deep_ocean`.
  - `#dolphin_located` = ocean ruins (cold, warm) and shipwrecks (normal, beached).
- **Dolphin's grace** (`LivingEntity.travelInWater`): the horizontal keep becomes 0.96. The push stays 0.02 and the vertical keep 0.8, so the steady speed is 0.49 blocks a tick, against 0.196 for a sprint-swim.
- **javap during the work:**
  - `Entity.getMotionDirection`;
  - `Mob.aiStep`'s pickup (reach (1,0,1), `mob_griefing`);
  - `HurtByTargetGoal.alertOthers`;
  - the held item's display context;
  - whether `ageScale` scales the baby's model or shadow;
  - the `drop_chances` value of `setGuaranteedDrop`.

### Plan

**Files (D0-3):**
- `entities/src/world/families/water/dolphins.rs`: `pub struct Dolphin { ai: MonsterAi, age: Age, got_fish, moistness, held: Option<ItemStack>, held_guaranteed, moving, speeding }`, `impl Family`, the tick and its tests.
- `.../water/dolphins/ai.rs`: `pub(crate) static DOLPHIN: MonsterSpec` and the six dolphin goals. A `DolphinHurtBy` wrapper refuses guardian attackers, so `monster_ai.rs` stays untouched. Goals see only `&dyn World`, so the toss, the `play` sound, event 38 and the grace grants are `GoalExtra` requests that the tick carries out after `family_goal_step` (D0-2.4).
- `crates/minecraft_terrain/src/families/water/dolphins.rs` (`TerrainFamily`) and `dolphins/render.rs` (both models and the carrying layer).
- One row in `world/src/natural_spawner/families/water.rs`.
- Registration lines:
  - `pub mod dolphins;` and `dolphins: dolphins::Dolphin,` in `family_group!(water 64..=79 { .. })`;
  - `mod dolphins;` and `Dolphin` in `terrain_group!(water { .. })`;
  - the row in `ROWS`.

**Hooks (D0-2.2):**
- `KEY` 66; `KINDS` `["minecraft:dolphin"]`.
- `spawn`: `MobCore::new(kind, pos, 0.9, 0.6, 0.3, 10.0)` with `ambient_interval` 120, air 4800 and moistness 2400.
- `category` `"water_creature"`, whose default despawn (128/32) is vanilla's.
- `experience`: `1 + next_int(3)` for an adult, 0 for a baby.
- `death_drops`: the held stack.
- `leashable` true; `leash_walk` through M2-1's water navigation.
- `monster`: `Some(&ai::DOLPHIN)`.
- M2-1's hooks: `breath` `Lungs { max: 4800, refill: 4800 }` (not `breathes_water`: the dolphin is not in `#can_breathe_under_water` and still drowns); `swim_sounds` `Some(("entity.dolphin.swim", "entity.dolphin.splash"))`. Squids' `OWN_DYING_TRAVEL`: true (a dead dolphin keeps `travelInWater`'s drift).
- Defaults for the rest, `voice` included (`"dolphin"`, neutral).

**`DOLPHIN` spec:**
- The vanilla lists above, without the two follow goals (see Left for later).
- Eye 0.3, follow 16.
- `walk`: `swim::water_walk` (WATER malus 0, no float, neutral walk target) with M2-1's breaching water navigation (`WaterNav::Bound { breaching: true }`), so the shared melee, hurt-by and avoid goals path through it.
- Avoidance: `AvoidEntityGoal::new("minecraft:guardian", 8.0, 1.0, 1.0).also(&["minecraft:elder_guardian"])`. It finds nothing until `guardians` lands.
- The pod alert goes to dolphins in the follow-range box without a target, done in the tick as `vex_alerts` does (D0-7.1), through D0's `family_alert::<Dolphin>` when the merged d0a has it (M20).

**Tick, in vanilla order:**
1. `family_base_tick` with the 4800 air rule (`Breath::Lungs`).
2. NoAI: full air, then `family_after_move`.
3. Moisture, through `rain_at` at the feet and the box top. Drying out goes through `S::hurt` with `"minecraft:dry_out"`. The hop.
4. M2-1's `swim_ai_step` (speed 1.2) with `SwimControls { movement: Smooth(85, 10, 0.02, 0.1, true), look: Smooth(10), acceleration: Speed, sink_without_target: true }`, then the requests (toss, voices, event 38 into `entity_events`, grants). It travels with `travel_swimming` at the dolphin's speed in water (vanilla `travelInWater`: no fluid-falling adjust, no `jumpOutOfFluid`) and the normal air travel on land.
5. Pickup, with `mob_griefing`: one item in the box grown by (1,0,1), taken whole into an empty hand.
6. `family_after_move`. Then `age.tick`, resizing the body at the baby/adult boundary.
7. While in water, this tick's `Voice::Ambient` becomes `entity.dolphin.ambient_water`.
8. The flags: `moving` = horizontal velocity² > 1e-7; `speeding` = in water and velocity² > 0.03. The pitch lives in `core.look.pitch` and so reaches `MobPose.head_pitch` and friends through `ServerMob.x_rot` (`client_mobs.rs:31`).

**Root-file hook** (its own commit, in `dolphins-grace`; M20 owner). The air and swim-sound hooks are M2-1's (`water-hooks`):
- **"Registered Minecraft mobs can give players effects":** `PlayerEffect { player, effect: EffectKind, ticks, amplifier, display_limit, any_mode }` queued on `Families`, with `family_player_effect` (`pub(crate)`) and `take_player_effects` (`pub`). `any_mode` is vanilla's plain `addEffect` in every game mode (the dolphin, the nautilus); without it the grant follows W7's `addEffectToPlayersAround` rules with `display_limit` (the elder guardian, M2-5, which also adds a `cue`). W5's warden darkness can use it.

**Shared one-liners next to related lines:**
- `ambient_particles.rs` (M20: M2's kinds go after `Flame`, because M3's phantoms take `HappyVillager`): `Kind::Dolphin` after `Flame` (`:53`, with squids' kinds) and `Particle::dolphin` beside `flame` (`:132`), built like `happy_villager` (`:213`). Its own arms after the `Flame` arms in `add` (`:1074`), the tick (`:1106`, calling `linger`), the sprite (`:469`, `generic_0`) and the size (`:493`).
  - E1 appends at the end of `Kind`.
- E1's `puff_for`: the row `("minecraft:dolphin", 38) → (HappyVillager, 7, 0.2)` beside the villager's 14 (decision 6), in `dolphins-treasure` when E1's table is in the branch.

**Multiplayer (`TerrainFamily`):**
- **Constants:** `TRACKING_RANGE` 5, `SHADOW_RADIUS` 0.7, `MAX_HEAD_Y_ROT` 1.0, the 2 `TEXTURES`.
- **`VOLATILE`:** `["Moistness", "GotFish", "ForcedAge", "AgeLocked", "equipment", "drop_chances"]`. `Air` and `Age` are in the base list (`mob_tracking.rs:15`).
- **`read`/`fields`:**
  - `Air`: a short, default 4800, because the generic path saves no air.
  - `GotFish`, `Moistness`, `Age`, `ForcedAge`, `AgeLocked`.
  - `CanPickUpLoot`: always 1.
  - `equipment.mainhand` and `drop_chances.mainhand`.
- **`show`:** `FamilyShow { flags: moving | speeding << 1, values: [0; 8] }`. **`show_on`:** stores the flags.
- **`interact`:**
  - Fish: the eat voice, then `Age::feed` for a baby that can grow, else `got_fish = true`. One fish is consumed unless materials are infinite; returns true.
  - Else the golden dandelion through `animal::interact` with `|_| false` as food.
  - Leads use D0's generic arm.
- **`draw`:** the `MobPose` and `moving` pose the adult or baby model into `out.models`, with the death flip. The held item goes to `out.held`.
- **`client_effects`:** two `Kind::Dolphin` specks per tick per `speeding` dolphin, from `core.yaw` and `core.look.pitch`.
- **Protocol:** none. The grace reaches friends as an `EffectView` string and an `mc_float` bit, both of which exist.

**Dolphin's grace on players:**
- **The effect:**
  - `EffectKind::DolphinsGrace` after `ConduitPower` (`survival.rs:51`) and in `ALL` after `:75`, which becomes `[EffectKind; 21]`. That one-line length is M2's this round (M20); `nautiluses-riding` makes it 22 after this.
  - The name rows after `player_nbt.rs:31`, `commands.rs:170` and `potions.rs:243`.
- **Movement:**
  - `float.rs`: `pub const DOLPHINS_GRACE: i32 = 0x2_0000` after `:13` and `fn dolphins_grace(ps)`, re-exported on `lib.rs:62`.
  - `swim.rs`: `const GRACE_DRAG: f32 = 0.96` after `:11`, chosen first at `:187`.
  - `frame.rs:489` keeps the bit: `encode_float(..) | (ps.mc_float & DOLPHINS_GRACE)`.
  - A new `set_dolphins_grace(id, on)` after `set_float` (`frame.rs:485`, `carrier.rs:314`), called after `minecraft_elytra.rs:28`.
- **Who swims (decision 1, group `dolphins-escort`):** M2 adds no `PlayerView` field (M20: M1's armadillos own the player-flag lines, and M3's camels share them). Vanilla's `isSwimming` is "sprinting in water" (`Player.updateSwimming`: start with the eyes under water and water at the feet, keep it while sprinting in water, never as a passenger). Before the goal step the tick fills `GoalExtra` with the players who swim: M1's `PlayerControls.sprinting` (through `player_controls(id)`, `world.rs:5242`) and water at the player's feet and eyes. The goal reads that list, so a crawling player never counts.
- **Grants (decision 2):**
  - The goal asks for grace for 100 ticks at amplifier 0 with `any_mode`, its 1-in-6 drawn from `level_random`.
  - `ServerSim::run_tick` moves `mobs.take_player_effects()` into W7's `Output.player_effects`, one line beside W7's.
  - `EffectGrant` gains `any_mode`, which skips W7's survival-or-adventure filter, so this is vanilla's plain `addEffect` (`add_effect`, or E1's `give_effect`).
  - W7 is merged before this segment starts (M20, m2i).

**Treasure (decision 4):**
- A new `World` method `fn nearest_structure(&self, tag, origin: Pos, radius) -> Option<Pos>` (default `None`) after `can_see_sky` (`player/src/lib.rs:185`; M20: M1 takes the lines after `biome` and after `rain_at`).
- `MobWorld` answers it after its `can_see_sky` (`server_mobs.rs:353`). It reads a new `MobTables.structures: Option<&'static WorldGen>`, set by one line after `server.rs:407`.
- The tag is expanded once with `locate::tags(gen, "worldgen/structure")` and cached. The search is `locate::nearest_structure(gen, &members, origin, 50, now + 2 s)`.
- A timeout counts as stuck and is logged with its time, as the ender eye does.

**Commit groups**

| Group | Commits | Contents |
|---|---|---|
| `dolphins-live` (m2c) | 1. "Minecraft dolphins swim, leap, breathe at the surface and dry out on land" 2. "Minecraft dolphins fight back as a pod, flee guardians and play with items" 3. "Minecraft dolphins spawn in oceans" | The family, spec and goals except grace and treasure; travel, moisture, pickup. Then the row: 0.9 × 0.6, `InWater`, `MotionBlockingNoLeaves`, `Rules::Family { creature: true }`, squids' `surface_water_animal`, cluster 4, `WaterCreature`, follow 16, `baby_chance: Some(0.1)`, no finalize (spawn sets the air). In-game dolphins stay dormant until `-drawn`. |
| `dolphins-drawn` (m2c) | 1. "Minecraft dolphins are saved, hatch from eggs and are drawn like vanilla" 2. "Minecraft dolphins leave a speed trail in the water, for friends too" | `TerrainFamily`, models, carried item, feeding, flags; then `Kind::Dolphin` and `client_effects`. The row becomes `drawn`. |
| `dolphins-treasure` (m2c) | 1. "Minecraft dolphins fed a fish lead players to shipwrecks and ocean ruins" | `World::nearest_structure`, the goal, event 38 (E1's row if E1 merged). The steps and kit so far go into the log. |
| `dolphins-grace` (m2i, after W's w1) | 1. "Minecraft players swim fast with dolphin's grace" 2. "Registered Minecraft mobs can give players effects" | The effect and movement (testable with `/effect`), then the root queue into W7's output. |
| `dolphins-escort` (m2i, after M1's sprint flag) | 1. "Minecraft dolphins swim with swimming players and give them dolphin's grace" | The goal on `PlayerControls.sprinting` and the water test, and the grant. The row becomes `done`; the steps and kit go into the log. |

### Tests

- `dolphins-live` 1: `a_dolphin_is_0_9_by_0_6_with_ten_health_and_its_kind` (eye 0.3, census `water_creature`, `mob_candidates`, loot and 1-3 experience for adults only, the baby's size), `a_dolphin_holds_its_breath_4800_ticks_and_refills_at_once`, `a_dolphin_out_of_air_drowns_two_a_second`, `a_dolphin_below_140_air_swims_up_to_breathe`, `a_dolphin_dries_out_after_2400_ticks_on_land_and_rain_keeps_it_wet`, `a_stranded_dolphin_hops_about`, `a_dolphin_keeps_nine_tenths_of_its_speed_and_sinks_only_without_a_target`, `a_dolphin_at_the_surface_leaps_into_open_water_nose_along_its_arc`, `only_the_first_leap_of_a_loaded_dolphin_plays_the_jump_sound`, `dolphins_call_ambient_water_in_water_and_ambient_on_land`, `a_no_ai_dolphin_keeps_full_air`.
- `dolphins-live` 2: `a_hit_dolphin_and_its_pod_within_sixteen_bite_back_for_three`, `a_baby_dolphin_never_attacks`, `guardians_hurting_a_dolphin_are_not_its_target`, `dolphins_swim_away_from_guardians_within_eight`, `a_dolphin_noses_an_item_in_water_and_tosses_it_ahead_with_a_40_tick_delay`, `items_on_land_or_on_pickup_delay_are_ignored`, `without_mob_griefing_it_picks_up_nothing`, `a_dolphin_killed_holding_an_item_drops_it`.
- `dolphins-live` 3: `dolphins_spawn_in_the_top_thirteen_blocks_of_ocean_water`, `a_dolphin_group_after_the_first_has_a_baby_one_time_in_ten`, `summoning_a_dolphin_notes_nothing_unsupported`.
- `dolphins-drawn` 1: `a_dolphin_saves_and_loads_with_its_air_moistness_fish_age_and_held_item`, `a_friend_sees_a_dolphin_from_its_look_tag`, `render::tests::an_adult_dolphin_has_eight_cubes_on_64_by_64_and_a_baby_its_own_eight`, `a_moving_dolphin_beats_its_tail_and_rocks_its_body`, `a_dolphin_pitches_its_whole_body`, `the_baby_uses_the_baby_texture`, `a_carried_item_sits_at_the_snout_and_follows_the_pitch`, `a_fish_sends_an_adult_after_treasure_and_grows_a_baby`, `a_golden_dandelion_locks_a_baby_dolphin_s_age`.
- `dolphins-drawn` 2: `ambient_particles::tests::a_dolphin_speck_is_pale_blue_half_lived_and_slows`, `a_fast_dolphin_in_water_trails_two_specks_a_tick_behind_its_tail`, `mob_tracking::tests::a_friend_sees_a_dolphin_s_speed_trail`.
- `dolphins-grace` 1: `movement_iw4` `swim::tests::dolphins_grace_keeps_ninety_six_hundredths_of_horizontal_speed` (0.49 against 0.098 and 0.196, vertical unchanged), `float::tests::the_grace_bit_leaves_levitation_and_slow_falling_alone`, `frame::tests::dolphins_grace_reaches_the_player_state_and_survives_set_float`, `player_nbt::tests::dolphins_grace_is_saved`, `commands::tests::effect_give_dolphins_grace_works`.
- `dolphins-grace` 2: `families::tests::a_queued_player_effect_reaches_the_output_with_its_mode_and_display_limit`.
- `dolphins-escort` 1: `a_dolphin_joins_the_nearest_swimming_player_within_ten` (a sprinting player in water; a crawling or walking one is not swimming), `it_grants_grace_for_100_ticks_at_start_and_one_tick_in_six`, `it_lets_go_at_sixteen_blocks_or_when_the_player_stops_swimming`, `a_nearer_player_who_is_not_swimming_holds_it_back`, `spectators_are_never_joined_and_creative_swimmers_are`, `entities::tests::a_dolphin_s_grace_reaches_host_and_friend`.
- `dolphins-treasure` 1: `server::tests::a_fed_dolphin_heads_for_the_nearest_ocean_ruin_or_shipwreck` (generated seed), `with_nothing_within_fifty_it_gives_up_and_forgets_the_fish`, `it_stops_within_four_blocks_and_forgets_the_fish`, `it_sparkles_at_the_start_and_about_every_80_ticks`, `a_dolphin_low_on_air_breathes_first_and_then_resumes`, `mob_particles::tests::a_dolphin_s_event_38_shows_seven_happy_specks`.

### Owner's steps (survival, Normal, by day; `/locate biome minecraft:warm_ocean`, `/tp` there, stand on a 5 × 5 glass platform built on the water surface)

1. Use the dolphin egg on the water 6 blocks away three times. Grey dolphins swim with beating tails, their bodies tilting with their course. Now and then one leaps out in a nose-first arc and splashes back. A fast one trails tiny pale-blue specks. **dolphin1_swim**
2. `/summon minecraft:dolphin ~ ~-4 ~6 {Air:120s}`: it swims straight up, breathes at the surface and dives again. **dolphin2_breath**
3. (Needs `dolphins-escort`, segment m2i.) Walk into the water beside a dolphin and sprint forward so you swim. It keeps about 2 blocks from you. The dolphin's grace icon appears with pale swirls, and you swim about 2.5 times as fast as a normal sprint-swim. Stop sprinting: the icon is gone within 5 s. **dolphin3_grace**
4. (Needs `dolphins-grace`, segment m2i.) `/effect give @s minecraft:dolphins_grace 30`, then swim without a dolphin: you are fast even without sprinting. `/effect clear @s`.
5. Run `/locate structure #minecraft:dolphin_located` and note the direction. Right-click a dolphin with raw cod: a munch, and one cod is gone. Within a second it sparkles green and swims off that way, sparkling now and then. Follow it at the surface: it stops at a shipwreck or an ocean ruin. **dolphin5_treasure**
6. Throw a stick (Q) into the water 4 blocks from a dolphin. It swims to it with a squeak and keeps flicking it ahead of itself. **dolphin6_play**
7. Hit a dolphin with the sword: it and the dolphins near it come and bite (1.5 hearts each on Normal). Shoot one with the M4A1 from 8 blocks: the same happens. Kill one: 0-1 raw cod and an orb.
8. `/summon minecraft:dolphin ~2 ~1 ~ {Moistness:100}` on the glass. It flops about; after 5 s it flashes red twice a second and dies about 5 s later. An egg's dolphin on the glass lasts 2 minutes. Pour water next to it and it recovers once it flops in.
9. `/summon minecraft:dolphin ~ ~-2 ~4 {Age:-24000}` gives a small, rounder, paler dolphin. Hit it: it does not bite back. Feed it cod: one cod is eaten. A golden dandelion locks its age (you hear the lock sound).
10. Put a lead on a dolphin and walk along the platform edge: it is pulled after you.
11. Leave a fed dolphin before it reaches its treasure, the locked baby and two others. Save, quit and load: all of them are there, and the fed one sets off again.
12. `/gamemode creative`, place the spawner and use a dolphin egg on it: a small dolphin turns in the cage. Then `/gamemode survival`.
13. Stand on the platform for two minutes: dolphins appear in ones and twos, sometimes with a baby.
14. Only if `guardians` has landed: use a guardian egg 5 blocks from a dolphin. The dolphin swims away.
15. Two windows (`scripts\mp-local.ps1`). The host hatches three dolphins.
    - The friend sees the same swimming, pitching, leaps and trails.
    - (With `dolphins-escort`.) The friend sprint-swims beside a dolphin: the icon shows in the friend's HUD, and the friend swims fast in their own window without snapping back. **dolphin15_friend_grace**
    - The host feeds a dolphin: the friend sees the sparkles.
    - The friend hits a dolphin: the pod chases the friend. **dolphin15-host**, **dolphin15-friend**

### Kit

```
IW4L_MINECRAFT_KIT=minecraft:dolphin_spawn_egg*16,minecraft:cod*32,minecraft:salmon*16,minecraft:tropical_fish*8,minecraft:golden_dandelion*4,minecraft:lead*4,minecraft:stick*16,minecraft:glass*64,minecraft:water_bucket,minecraft:water_bucket,minecraft:diamond_sword,minecraft:bow,minecraft:arrow*64,minecraft:spawner,minecraft:guardian_spawn_egg*4,minecraft:cooked_beef*32,minecraft:cobblestone*64
```

The kit has 17 stacks, and every id was checked against the 26.3 `assets/minecraft/items`. The friend uses the same line.

### Depends on

- **D0 d0a:** `registry-core` (the base, goal, after-move and strike helpers, `GoalExtra`, `MonsterKind::Registered`), `registry-spawning`, `registry-client` and `registry-draw`. Not d0b; no keyframes.
- **M2-1:** the breaching water navigation, `swim_ai_step` with the smooth swimming controls and `travel_swimming`, `RandomSwimmingGoal`, `TryFindLiquidGoal`, water-aware random positions, `Breath::Lungs { max, refill }` and `swim_sounds`. They are usable from a `MonsterAi` family through `MonsterSpec.walk`, because `MeleeAttackGoal`, `AvoidEntityGoal` and the dolphin goals path through the context's navigation (guardians need the same).
- **M2 `squids`:** `surface_water_animal`, the `InWater` obstruction line and `OWN_DYING_TRAVEL`.
- **Track W:**
  - W7 (segment w1): `Output.player_effects` and `EffectGrant`, for `dolphins-grace` 2 and `dolphins-escort` (segment m2i waits for w1).
  - W5 can reuse the effect queue.
  - P7: swim and splash already post game events through W's lines in `emissions.rs`.
- **M1:** armadillos' `PlayerControls.sprinting` for `dolphins-escort`.
- **Track E:**
  - E1: the event 38 row, and `give_effect`.
  - E2: thrown things reach dolphins through `hit_boxes`.
- **M2 `guardians`:** the avoidance and the attacker filter work by kind, unchanged. **M2 `nautiluses`:** the effect queue (`nautiluses-riding`) and, optionally, the follow goal (Left for later).

### Open decisions (recommended choice)

1. **Who swims:** M1's `PlayerControls.sprinting` plus the water at the player's feet and eyes, which is vanilla's `isSwimming` rule. No new `PlayerView` or `PlayerCandidate` field (M1 owns those lines; `PlayerCandidate` has 99 literals).
2. **Effects to players:** a root queue plus W7's output with `any_mode`, because vanilla gives creative swimmers the grace too.
3. **AI:** the goal framework with M2-1's navigation, not a selector of the dolphin's own, so the shared melee, hurt-by and avoid goals are reused.
4. **Treasure search:** in the tick with a 2 s cap, as the ender eye and vanilla's server thread do. It runs only after a feed, and in an ocean it usually ends in the first rings. Not a job between ticks.
5. **Air:** M2-1's `Breath::Lungs { max: 4800, refill: 4800 }`, not `breathes_water`: the dolphin is not in `#can_breathe_under_water` and must still drown.
6. **Event 38 specks:** E1's shared `gaussian·0.02` spread, not the dolphin's 0.01.
7. **Grace in movement:** an `mc_float` bit, which friends already replicate and predict, rather than a new player-state field.

---

## M2-5. Guardians and elder guardians (groups `guardians-live`, `guardians-drawn`, `guardians-effects`, `guardians-curse`)

Family `guardians`, group `water`, hit-key byte **70** (`mobs.md` rows 35 and 36). One store holds `minecraft:guardian` and `minecraft:elder_guardian` (`ElderGuardian extends Guardian`); the state has an `elder` flag. Guardians are `Monster`s on the goal framework (`MonsterKind::Registered`, D0-2.4). Their move control and travel are their own, so the family runs its own AI step through M1's `family_goal_step_with`. They have no brain and no keyframes. Lines are `minecraft-survival` at `d1a3d028`; re-check them. Names follow `track-d0.md` and M2's `water-movement` section (M2-1).

### What exists today

- **Dormant.** Neither kind is in `SIMULATED_KINDS` (`crates/minecraft_terrain/src/server_mobs.rs:433`), so `spawn_saved` (`:580`) returns `None` (`:1321`). Eggs do nothing, and the tracking range is 5 (`mob_tracking.rs:285`).
- **Elders are placed already.** `Monument::spawn_elder` (`third_party/minecraftoss/generator/src/structure/kinds/ocean_monument.rs:457`) puts three in each monument (`:1278`, `:1479`, `:1505`). They load dormant now and come alive once registered.
- **Monument spawns are half wired.**
  - The structure overrides are loaded (`world/src/natural_spawner.rs:495-513`) and handed out by `mobs_at` (`natural_spawner/spawning.rs:503`).
  - There is no `type_info` row (`natural_spawner.rs:271`, `:324`), so a spawn logs `note_unsupported` (`spawning.rs:576`).
  - There is no `MobCategory::of_type` entry (`:97`), so the spawn is refused at `:681`.
  - `check_spawn_obstruction` refuses fluid except for striders and drowned (`natural_spawner.rs:822`). The squids and fish briefs add an `InWater` line.
- **Lists that already name both kinds:** `NOT_IN_PEACEFUL` (`generator/src/feature/mod.rs:159`) and `sleep::MONSTERS` (`crates/minecraft_survival/src/sleep.rs:53`).
- **Mining fatigue works for players:** `digging.rs:177`, `player_nbt.rs:30`, `commands.rs:169` and the HUD icon.
- **Reuse:**
  - the beam helpers `beam_pose`, `beam_spans` (which cuts a scrolling V where the atlas texture repeats) and `append_beam` (`end_crystal_render.rs:102`, `:118`, `:133`);
  - scaling a model about its feet (`cat_render.rs:455`);
  - the trader's home restriction (`entities/src/trader_ai.rs:318`) and its save keys `home_pos` and `home_radius` (`server_mobs.rs:1238`, `:1669`);
  - nested loot tables (`entities/src/loot.rs:497`);
  - the totem overlay `ItemActivation` (`render_anim/src/minecraft_hand.rs:91`), started at `minecraft_world.rs:934` (host) and `minecraft_remote.rs:791` (friend).
- **Missing:**
  - a bubble particle (`ambient_particles.rs:42`);
  - looping sounds (`render_anim/src/minecraft_sounds.rs` plays one-shots);
  - the conduit's attack, Impaling, and water breathing for players;
  - `PlayerHitKind::Magic { at }` exists but gives no knockback (`survival/entities.rs:1900`).

### Vanilla 26.3 (javap this round)

- **Types:**
  - guardian: `MONSTER`, 0.85 × 0.85, eye 0.425, tracking 8, `notInPeaceful`;
  - elder: 1.9975 × 1.9975, eye 0.99875, tracking 10, `notInPeaceful`;
  - neither is fire immune.
- **Attributes:**
  - guardian: `Monster`'s (follow range 16) plus attack 6, speed 0.5, health 30;
  - elder: speed `0.30000001192092896`, attack 8, health 80.
- **`Guardian`:**
  - `xpReward` 10 for both kinds; malus `WATER` 0; `GuardianMoveControl`; `WaterBoundPathNavigation`;
  - the constructor draws the tail clock from `random`;
  - synced `MOVING` and `ATTACK_TARGET` (an entity id, 0 for none);
  - attack duration 80 (elder 60), ambient interval 160, `getMaxHeadXRot` 180;
  - movement emission `EVENTS` (no step or swim sounds); `checkSpawnObstruction` is `isUnobstructed`;
  - walk target: in water `10 + getPathfindingCostFromLightLevels`, else `Monster`'s.
- **Goals:**
  - 4 `GuardianAttackGoal`;
  - 5 `MoveTowardsRestrictionGoal(1.0)`;
  - 7 `RandomStrollGoal(1.0, 80)` (5 and 7 are both MOVE and LOOK);
  - 8 `LookAtPlayerGoal(Player, 8)` and `(Guardian, 12, 0.01)`;
  - 9 `RandomLookAroundGoal`.
- **Target:** `NearestAttackableTargetGoal(LivingEntity, 10, mustSee)`, one roll in 5. It takes a `Player`, `Squid` (glow squid too) or `Axolotl` farther than 3. It remembers no `targetMob`, so a cleared target stops it.
- **`GuardianAttackGoal`:**
  - It runs while there is a living target. Unless it is an elder, the target must also be farther than 3.
  - `start`: `attackTime = −10`, the navigation stops, it looks (90, 90).
  - `stop`: active target 0, `setTarget(null)`, and the stroll's `trigger()`.
  - Every tick:
    1. Stop the navigation and look (90, 90). Without sight: `setTarget(null)` and stop here.
    2. `++attackTime`. At 0: the active target becomes the target's id, plus entity event 21.
    3. At the duration: `indirectMagic(self, self)` for 1 (+2 Hard, +2 elder), then `doHurtTarget` (`mob_attack`, 6 or 8), then `setTarget(null)`.
- **`GuardianMoveControl`** (with `MOVE_TO` and the path not done; else speed 0 and not moving):
  1. Yaw `rotlerp(yRot, atan2(vz,vx)·57.2957763671875 − 90, 90)`; the body follows.
  2. `speed = lerp(0.125, speed, modifier·MOVEMENT_SPEED)`.
  3. With `t = tickCount + id`, velocity `+= (sin(0.5t)·0.05·cos yaw, sin(0.75t)·0.05·(sin+cos yaw)·0.25 + speed·(vy/|v|)·0.1, sin(0.5t)·0.05·sin yaw)`.
  4. It looks at `(x+2dx, eyeY+dy/|v|, z+2dz)`. When the look control already holds a point, it lerps 0.125 from that point to this one. Limits (10, 40). `moving` is set.
- **Travel and `aiStep`:**
  - `travelInWater`: `moveRelative(0.1)`, move, ×0.9; `vy −= 0.005` when not moving and with no target.
  - In water, air 300.
  - On land and on the ground, the flop: velocity `+= ((2r−1)·0.4, 0.5, (2r−1)·0.4)`, a random yaw.
  - While beaming, `yRot = yHeadRot`.
  - It never drowns or dries out: it is in `#can_breathe_under_water` and is not a `WaterAnimal`.
- **`hurtServer`:**
  1. If it is not moving, the source is in neither `#avoids_guardian_thorns` (`magic`, `thorns`, `#is_explosion`) nor `thorns`, and the direct entity is a `LivingEntity`, that entity takes `thorns` 2.
  2. The stroll's `trigger()`.
  3. The `Monster` hurt.

  `thorns` and `indirect_magic` scale with difficulty. `indirect_magic` bypasses armour and shields. Neither is `#no_knockback`.
- **`ElderGuardian`:**
  - Persistent; stroll interval 400.
  - When `(tickCount + id) % 1200 == 0`: `addEffectToPlayersAround(50, MINING_FATIGUE 6000 amplifier 2, display limit 1200)`, under W7's `EffectGrant` rules (the radius is strict). Each player reached gets the game event `GUARDIAN_ELDER_EFFECT` (type 10; value 1, or 0 if silent).
  - With no home: `setHomeTo(blockPosition, 16)`, saved as `home_pos` and `home_radius`.
- **Sounds:**
  - `entity.{guardian,elder_guardian}.{ambient,hurt,death}`, with `_land` variants out of water;
  - `.flop`;
  - `entity.guardian.attack`, the client's beam hum;
  - `entity.elder_guardian.curse`.
- **Client events:**
  - Entity event 21 starts a looping hum with no distance falloff, at `s²` volume and `0.7 + 0.5s` pitch each tick (`s` = attack scale). The `attack_loop.ogg` sample is 2.004 s.
  - Game event 10 shows the `ELDER_GUARDIAN` particle, plus the curse sound when the value is 1.
- **Client clocks:**
  - Tail speed:
    - 2 on land;
    - moving: 4 when it is below 0.5, else `+= (0.5 − speed)·0.1`;
    - still: `+= (0.125 − speed)·0.2`;
    - the tail adds the speed each tick.
  - Spikes: random on land; moving `+= −spikes·0.25`; still `+= (1 − spikes)·0.06`.
  - The attack time counts up to the duration while the target lasts, and resets when the target changes.
  - Bubbles:
    - while moving in water, 2 a tick at `(getRandomX(0.5), getRandomY(), getRandomZ(0.5)) − view·1.5`;
    - while beaming, `d = r`, then while `d < len`: `d += 1.8 − s + r·(1.7 − s)` and a bubble at `eye + dir·d`.
- **`GuardianModel`** (64×64, `entityCutoutNoCull`; javap the boxes during the work):
  - `head` holds five boxes, `spike0..11` (from the 12-entry tables `SPIKE_X/Y/Z` and `SPIKE_*_ROT`·π), `eye`, and `tail0 → tail1 → tail2`. The elder layer is `scaling(2.35)`.
  - `setupAnim`:
    - the head's yaw and pitch;
    - spike `i` at offset `1 + cos(1.5·age + i)·0.01 − (1 − spikes)·0.55`;
    - the eye's `y` is 0 when the look point is above the eye, else 1; `x = 2√|d|·sign d`, where `d` = the horizontal view dotted with `(eye − lookAt)` normalised and turned π/2;
    - the tail `yRot = sin(tail)·π·{0.05, 0.1, 0.15}`.
- **Renderer:**
  - Shadow 0.5 (elder 1.2). Textures `entity/guardian/guardian`, `guardian_elder` and `guardian_beam`.
  - The eye looks at the active target's eye, **else the viewer's camera**.
  - **Beam** (`entityCutout`, full bright), from the eye to the target's lerped `y + h/2`:
    - length `L = |v| + 1`, turned `Y(π/2 − atan2(vz,vx))·X(acos(vy/|v|))`;
    - `a = −0.075·attackTime`; colour `(64 + 191s², 32 + 191s², 128 − 64s²)`;
    - two crossed quads of radius 0.2 at `a+π`/`a` and `a+π/2`/`a+3π/2`, u 0 to 0.4999, v from `−1 + (0.5·attackTime mod 1)` over `2.5L`;
    - an end cap at `L`: radius 0.282 at `a + {3,1,7,5}π/4`, u 0.5 to 1, v base 0.5 or 0 by the parity of `floor(attackTime)`, height 0.5.
- **The curse particle** (30 ticks): the elder model in its rest pose, translucent and full bright.
  - Alpha `0.05 + 0.5·sin(πt)`, with `t = (age + p)/30`.
  - Pose: camera rotation, then `X(60 − 150t)`°, then scale `(0.4255, −0.4255, −0.4255)`, then translate `(0, −0.56, 3.5)`.
- **`BubbleParticle`:** quad × `(0.6r + 0.2)`; velocity `0.2v + (2r−1)·0.02`; life `8/(0.8r + 0.2)`; each tick `yd += 0.002`, move, ×0.85; it dies outside water.
- **Loot:**
  - guardian: 0 to 2 prismarine shards; then cod (2, smelted when on fire), crystals (2) or nothing (1); `gameplay/fishing/fish` at 2.5% on a player kill;
  - elder: the same with cod at 3, plus a wet sponge on a player kill and a Tide trim template 1 time in 5.
- **Spawning:**
  - Both kinds are `IN_WATER` and `MOTION_BLOCKING_NO_LEAVES`. The rule is all of:
    - `nextInt(20) == 0 || !canSeeSkyFromBelowWater(pos)`, with the random drawn first;
    - not Peaceful;
    - a spawner, or water at the spot;
    - water below.
  - `canSeeSkyFromBelowWater`:
    - at or above sea level, `canSeeSky`;
    - else `canSeeSky(x, sea, z)`, with no light-dampening block that is not a `LiquidBlock` between `sea − 1` and `y + 1`.
  - No biome lists either kind. The monument's monster override (full box) is the guardian at weight 1, 2 to 4 at a time.

### Plan

**Files (D0-3):**
- Entities, `third_party/minecraftoss/entities/src/world/families/water/`:
  - `guardians.rs`: the state, `impl Family`, the tick;
  - `guardians/goals.rs`: the two `MonsterSpec`s and the goals;
  - `guardians/control.rs`: `GuardianMoveControl`, `guardian_ai_step`.
- Terrain, `crates/minecraft_terrain/src/families/water/`: `guardians.rs` (`TerrainFamily`, `client_effects`) and `guardians/render.rs` (the model, the beam, `curse_mesh`).
- World: the rows, `guardian_rule` and `sky_from_below_water` in `natural_spawner/families/water.rs`.
- Render: `crates/render_anim/src/elder_curse.rs`, in the curse group.

The state:
- `elder`, `ai: MonsterAi`, `moving`;
- `tail(_o)`, `tail_speed`, `spikes(_o)`;
- `anim: LegacyRandom`, seeded from the id, so client-side draws never touch `core.random`;
- `attack_ticks`, `beam_end(_o)`, `target_eye_dy`;
- `thorns_to`, `home`.

`GoalExtra` holds `GuardianGoals { active, attack_time, fire, stroll_trigger, look_target }`.

**Registration:**
- `pub mod guardians;` and `guardians: guardians::Guardian,` in the entities group file.
- `mod guardians;` and `Guardian` in the terrain group file.
- Two `ROWS`.

**Hooks:**
- `KEY` 70.
- `spawn`: 0.85/0.85/eye 0.425/30, or 1.9975/1.9975/0.99875/80 and `persistent` for the elder. The tail draws once from `core.random`, like the constructor.
- `experience` 10; `enemy` true; `breathes_water` true.
- `monster(kind)`: the two specs, follow range 16, `walk` = `swim::water_walk` then `WalkTarget::Guardian`.
- `hurt`:
  - `hurt_plain`;
  - out of water, the pushed hurt or death voice becomes `Voice::Event("entity.<kind>.hurt_land"/"death_land", 1, pitch)`;
  - if it is not `moving`, the source is `PlayerAttack` or `MobAttack` with an attacker, and the type is outside `#avoids_guardian_thorns`: set `thorns_to`;
  - set `stroll_trigger`.
- `emits_sounds` false (D0's hook for vanilla's `EVENTS` emission, M20): no step or swim sound, and the hostile splash stays. Only if the merged d0a lacks it: `quiet_steps` true and `swim_sounds` `Some(("", "entity.hostile.splash"))` (M2-1).
- The ambient: `spawn` sets `core.ambient_interval = 160`, and right after `family_base_tick` the tick turns the pushed `Voice::Ambient` into `entity.<kind>.ambient_land` when out of water (no ambient hook exists this round, M20).
- Everything else uses the defaults. That includes the monster despawn, under which Peaceful removes both kinds.

**Tick:**
1. `family_base_tick`.
2. Alive only:
   - the flop, with `Voice::Event("entity.<kind>.flop", 1, 1)`;
   - while beaming, the yaw follows the head;
   - a pending thorns strike: a player gets `PlayerHitKind::Typed { damage_type: "minecraft:thorns", message: "thorns", from: core position, .. }` for 2 through `family_strike`, a mob gets `mob_hits_mob(.., 2.0, ..)`.
3. `family_goal_step_with(.., guardian_ai_step)`, which follows `monster_ai_step` (`entities/src/world.rs:1022-1097`) except for:
   - Monster's light idle time, kept;
   - `tick_water` navigation (`WaterNav::Bound { breaching: false }`);
   - `GuardianMoveControl`;
   - look limits per caller;
   - in water `travel_swimming(.., 0.1)` plus the sink, elsewhere the air travel.
4. The attack goal leaves `fire` in `GoalExtra`. The tick then hits:
   - a player: `Typed { damage_type: "minecraft:indirect_magic", message: "indirectMagic", from: core position, bypasses_armor: true, bypasses_shield: true, .. }` for 1 (+2 Hard, +2 elder), then `Melee` for 6 or 8. Use `Typed` because it keeps the knockback. The player's `last_hurt` cooldown (`player/src/survival.rs:324`) gives vanilla's totals;
   - a mob: `hurt_mob_by_magic`, then `mob_hits_mob`.
5. The elder takes a home if it has none.
6. `family_after_move`.
7. Alive only: shift the clocks into `_o`, then update them (`beam_end` from `ai.state.info(active)`).

**Goals:**
- `GuardianAttackGoal`.
- `GuardianTargetGoal`: players, plus `ctx.mobs` of `squid`, `glow_squid` and `axolotl` farther than 3, through `ctx.info`, `can_attack`, `sees` and `continue_target(.., Some(30))`. It is a goal of its own because `Prey` must not be edited.
- `GuardianStrollGoal { interval }`: a trigger skips the idle check and the roll. The spot comes from `stroll::water_bound_random_position(10, 7)` with the elder's home.
- `GuardianRestrictionGoal`: the trader's goal, water-bound.
- `GuardianLookGoal { range, probability, guardians }` and `GuardianRandomLookGoal`: both with 180 pitch, where `max_head_x_rot()` is 40 (`monster_ai.rs:674`).

**Rows:**
- Both: `InWater`, `MotionBlockingNoLeaves`, `Rules::Family { creature: false }`, `Walk::Neutral` (the vanilla value in water is at least 9.5), cluster 4, `Monster`, follow range 16, no babies, no finalize, `guardian_rule`.
- The guardian is `remove_far`; the elder is not.
- `sky_from_below_water` reads `sky_brightness >= 15`, `sea_level()`, `light_dampening` and `LiquidBlock`.

**One-line shared hooks:**
- `WalkTarget::Guardian` after `Strider { .. }` (`entities/src/walk_path.rs:54-56`, before `Hoglin`), and its value after the Strider arm (`stroll.rs:22-30`, before Hoglin's): water `10 + cost`, else `−cost`. M1's turtles put `Turtle` after `Animal` and M1's bees `Air` after `Monster` (M20).
- The `InWater` obstruction line is `squids-live`'s (M2-2).
- `ambient_particles.rs` (effects group; M20: M2's kinds go after `Flame`):
  - `Kind::Bubble` after `Flame` (`:53`, with the squids' and dolphin's kinds), and `bubble` beside `flame` (`:132`), ticking like `rain` (`:222`);
  - its own arms after the `Flame` arms in `add` (`:1074`), the tick (`:1106`, a new `bubble` step like `drip`), the sprite (`:469`) and the size (`:493`), and its sprites in `textures()` (`:678`).

**Root hooks** (D0-12; M20 owners):
- Used, already merged: squids' `TerrainFamily::tracking_range(kind)` (8 and 10) and fish's `TerrainFamily::shadow_radius(mob)` (0.5 and 1.2).
- This section's, its own commit at the start of `guardians-drawn`: `FamilyDraw.camera: DVec3`, "Registered Minecraft mobs are drawn knowing where the camera is", filled from `meshes`' `camera` (`crates/minecraft_survival/src/entity_view.rs:836`). A cage passes its own centre.

**Multiplayer:**
- `TEXTURES`: the three `minecraft:entity/guardian/*` textures. `VOLATILE`: none.
- `read`/`fields`: the elder's `home_pos` and `home_radius`.
- `show`:
  - flags: bit 0 `IN_WATER` (M2-1), bit 1 `MOVING`, bit 2 `BEAM`;
  - values: `[tail, spikes as f32 bits; attack_ticks; beam_end xyz as f32 bits; target_eye_dy as f32 bits; 0]`.
- `show_on`: shift into `_o`, then store. The host computes the clocks, so host and friend draw the same pose.
- **`draw`:**
  - the model into `out.models`, the elder ×2.35 about its feet;
  - the eye on the beam target while `BEAM` is set, else on `out.camera`;
  - the beam into `out.culled` at full light, cut by `beam_spans`.
- **`client_effects`** (effects group):
  - both kinds of bubbles;
  - the hum: `fx.seen[id]` holds the `attack_ticks` of the last play. It replays on a new beam or after `40/pitch` ticks, at that tick's `s²` and `0.7 + 0.5s`.
- **Curse** (curse group; steps 1-2 are its first commit, steps 3-6 its second):
  1. Every 1200 ticks the elder queues one `PlayerEffect` per player within 50 (strict, from the tick's `players`) through `dolphins-grace`'s `family_player_effect` (M2-4): Mining Fatigue, 6000 ticks, amplifier 2, `display_limit` 1200, not `any_mode` (W7's survival-or-adventure rule).
  2. `run_tick` already moves the queue into W7's `Output.player_effects` (`dolphins-grace` 2).
  3. `PlayerEffect` and W7's `EffectGrant` gain `cue: Option<(u8, f32)>`, here `(10, 1.0)`; the cue is sent only when the grant applies, as vanilla sends the game event only to the players it affected.
  4. The survival side sends `Said { heard: Heard::Game { event, value }, only: Some(client) }`, with `Heard::Game` beside `Heard::Player` (`heard.rs:126`), mapping to a new `ToClient::GameEvent { event, value }`.
  5. The host starts `ElderCurse` beside its totem (`minecraft_world.rs:934`), a friend beside `own_totem` (`minecraft_remote.rs:791`).
  6. `ElderCurse` appends `curse_mesh` to `meshes.translucent` after `entities.view.meshes(..)` (`minecraft_world.rs:1507`) and the friend's `entities.meshes(..)` (`minecraft_remote.rs:1988`), and plays the curse sound.
- **Protocol:** only `ToClient::GameEvent`. Note it under "## Protocol"; the pufferfish sting (type 9) can reuse it. No bump.

**Commit groups**

| Group | Commits | Contents |
|---|---|---|
| `guardians-live` (m2d) | 1. "Minecraft guardians and elder guardians swim, aim and fire their beams in the entity world" 2. "Minecraft guardians spawn in ocean monuments like vanilla" | the entity family, goals, control, thorns, hits, home, `WalkTarget::Guardian`; rows and rule |
| `guardians-drawn` (m2d) | 1. "Registered Minecraft mobs are drawn knowing where the camera is" 2. "Minecraft guardians and elder guardians are saved, hatch from eggs and are drawn like vanilla, beams included" | the camera hook; `TerrainFamily`, `FamilyShow`, model, eye, beam; rows `drawn` |
| `guardians-effects` (m2d) | 1. "Minecraft guardians trail bubbles and hum while they charge, for friends too" | `Kind::Bubble`, `client_effects` |
| `guardians-curse` (m2i, after W's w1 and `dolphins-grace`) | 1. "Minecraft elder guardians curse nearby players with mining fatigue like vanilla" 2. "Minecraft players see the elder guardian's face and hear its curse, friends too" | 1: the pulse through `dolphins-grace`'s queue (curse steps 1-2 below; entities and terrain only). 2: `cue` on `PlayerEffect` and `EffectGrant`, `Heard::Game`, `ToClient::GameEvent` ("## Protocol"), `ElderCurse` for host and friend (curse steps 3-6); rows `done`, steps and kit in the log |

### Tests

- **live 1** (`world::families::water::guardians::tests`):
  - `a_guardian_is_0_85_wide_with_30_health_and_its_kind`, `an_elder_guardian_is_two_wide_with_80_health_and_stays_but_on_peaceful`;
  - `a_guardian_beams_after_10_ticks_and_hits_after_80_with_magic_then_its_bite`, `an_elder_hits_after_60_for_three_and_eight_and_keeps_beaming_up_close`;
  - `a_guardian_never_targets_within_three_blocks_or_without_sight`, `guardians_target_players_squids_glow_squids_and_axolotls_only`;
  - `a_still_guardian_pricks_a_melee_attacker_for_two_but_not_an_arrow_magic_or_a_blast`;
  - `the_guardian_move_control_turns_ninety_and_wiggles_like_vanilla`, `a_guardian_swims_at_a_tenth_and_sinks_only_when_idle`, `a_stranded_guardian_flops_half_a_block_up_and_never_drowns`;
  - `a_hurt_guardian_strolls_again_at_once`, `an_elder_stays_within_sixteen_of_where_it_first_stood`, `guardians_sound_different_out_of_water`;
  - `stroll::tests::a_guardian_prefers_lit_water`.
- **live 2** (`natural_spawner::families::water::tests`): `guardians_spawn_over_water_under_a_roof_or_one_time_in_twenty`, `the_sky_seen_from_below_water_is_blocked_by_prismarine_not_water`, `a_monument_hands_out_two_to_four_guardians_with_nothing_unsupported`, `a_guardian_spawn_is_not_obstructed_by_water`.
- **drawn:**
  - `families::tests::a_family_is_drawn_with_the_viewer_s_camera` (a cage passes its centre), `a_guardian_and_an_elder_save_and_load_with_the_elder_s_home`, `a_friend_sees_a_guardian_from_its_look_tag`, `a_guardian_tracks_at_eight_and_an_elder_at_ten_and_casts_its_own_shadow`;
  - `render::tests`: `a_guardian_is_a_head_twelve_spikes_an_eye_and_a_three_part_tail`, `the_spikes_pull_in_while_it_swims`, `the_eye_follows_the_camera_and_drops_when_looking_down`, `an_elder_is_the_guardian_scaled_two_point_three_five_from_its_feet`, `the_beam_turns_from_purple_to_yellow_and_its_cap_flips_each_tick`, `the_beam_is_cut_where_its_texture_repeats`.
- **effects:** `ambient_particles::tests::bubbles_rise_slow_down_and_pop_out_of_water`, `a_swimming_guardian_trails_two_bubbles_a_tick_for_every_viewer`, `beam_bubbles_crowd_closer_as_the_beam_charges`, `the_hum_plays_again_when_its_sample_ends_and_rises_in_pitch`.
- **curse 1:**
  - `an_elder_pulses_mining_fatigue_three_for_five_minutes_every_minute`;
  - `server::tests::an_elder_s_curse_reaches_survival_players_within_fifty_only` (1199 ticks left is renewed, 1200 is kept).
- **curse 2:**
  - `server::tests::the_curse_cue_goes_out_only_with_a_grant_that_applies`;
  - `heard::tests::the_elder_curse_goes_only_to_the_cursed_player`, a `GameEvent` codec round trip;
  - `elder_curse::tests::the_curse_face_swoops_for_thirty_ticks_and_fades_in_and_out`, `minecraft_remote::tests::a_friend_shows_only_its_own_curse`.

### Owner's steps (survival, Normal, by day)

Setup:
1. `/gamemode creative`, `/locate biome minecraft:deep_ocean`, `/tp @s <x> 70 <z>`.
2. Fly down, `/fill ~-2 62 ~-2 ~2 62 ~2 minecraft:glass` for a platform flush with the sea, and stand on it.
3. `/gamemode survival`.

Glass blocks sight, so do the beam steps from the platform's edge. Keep the armour in the backpack until step 4.

1. Use a guardian egg on the water 6 blocks out.
   - A spiky teal block with one eye and a three-part tail swims about, turning smoothly. Its spikes fold in while it swims and stand out when it stops, and bubbles trail behind it.
   - Walk around the platform: its eye follows you. **g1_guardian_swims**
2. Stand still at the edge, 5 to 10 blocks from it.
   - It turns to you, and half a second later a thin purple beam runs to your chest, with a rising hum.
   - Over 4 s the beam turns yellow and hits: 3 hearts and a small push. Then it starts again. **g2_guardian_beam**
3. Step behind the platform's middle: the beam stops at once and does no damage.
4. Put on the diamond armour: the next hit costs about 1 heart.
5. Take the armour off.
   - Hatch a guardian on the platform: it flops about with a slapping sound and does not die.
   - Hit it with the sword while its spikes are out: you lose 1 heart each time. **g5_thorns**
   - Shoot it with the bow: no thorns.
   - Kill it: prismarine shards, sometimes crystals or a cod, and a burst of experience.
6. Stand right above a guardian, within 3 blocks: it does not beam you.
7. Use an elder egg 8 blocks out: a pale grey guardian, more than twice the size and slow.
   - (The curse needs `guardians-curse`, segment m2i; so do steps 10's curse, 11's curse and 15.) Within a minute a ghostly elder face swoops across your screen with an eerie wail, and Mining Fatigue III shows 5:00. **g7_elder_curse**
   - Stone mines very slowly.
   - Milk clears the fatigue until the next minute. The curse never comes in creative.
8. Let the elder beam you: 4 hearts. Kill it (40 hearts): a wet sponge, and 1 time in 5 a Tide armour trim template.
9. A squid hatched near a guardian is beamed to death.
10. `/locate structure minecraft:monument`, `/tp` above it, and build the same platform.
    - Guardians come out 2 to 4 at a time.
    - Within a minute an elder inside curses you. **g10_monument**
11. Save and quit with a guardian mid-beam and an elder alive, then load: both are there, and the curse comes at the next minute.
12. In creative, place a spawner and use a guardian egg on it: a small guardian turns in the cage. None spawn on dry glass (vanilla needs water below). `/gamemode survival`.
13. `/difficulty peaceful`: both kinds vanish. `/difficulty normal`.
14. Friend window. The host hatches a guardian.
    - The friend sees the same tail, spikes and bubbles.
    - At the edge, both windows show the beam running to the friend and changing colour. The friend hears the hum and loses 3 hearts.
    - In the friend's window, the eye follows the friend. **g14-host**, **g14-friend**
15. Friend window: the friend stands within 50 blocks of an elder, the host farther away. Only the friend gets the face, the wail and the icon. **g15-friend**

### Kit

```
IW4L_MINECRAFT_KIT=minecraft:guardian_spawn_egg*8,minecraft:elder_guardian_spawn_egg*2,minecraft:squid_spawn_egg*4,minecraft:axolotl_spawn_egg*2,minecraft:glass*64,minecraft:diamond_sword,minecraft:bow,minecraft:arrow*64,minecraft:shield,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:milk_bucket,minecraft:milk_bucket,minecraft:diamond_pickaxe,minecraft:stone*64,minecraft:water_bucket,minecraft:spawner,minecraft:cooked_beef*32,minecraft:golden_apple*8
```

21 stacks, each checked against the 26.3 item models. The friend uses the same kit.

### Depends on

- **d0a**: every group. **d0b**: none.
- **M2-1:**
  - `water-hooks`: `quiet_steps`, and `swim_sounds` with an empty swim sound (the `emissions.rs:64` guard);
  - `water-paths`: `WaterNav::Bound`, `tick_water`, and `water_bound_random_position` with its home;
  - `water-swimming`: `travel_swimming`;
  - not buckets.
- **M1:** rabbits' `family_goal_step_with` (already waited for by m2b). No ambient hook is needed.
- **M2 squids:**
  - the obstruction line and the per-kind tracking range;
  - squids as prey (soft);
  - axolotls hunting guardians belongs to the axolotls group.
- **M2 fish:** the per-kind shadow.
- **M2 dolphins:** `dolphins-grace`'s `PlayerEffect` queue, for the curse (same segment m2i, earlier).
- **W:**
  - W7 (w1): `Output.player_effects`/`EffectGrant`, for the curse (m2i waits for w1);
  - W's game events cover the `EVENTS` steps.
- **E1:** none (event 21 is a sound). **E2:** `hit_boxes` covers thrown items.

### Open decisions (recommended choice)

1. **Look goals:** the family's own, with vanilla's 180 pitch, rather than the shared ones at 40.
2. **Thorns:**
   - Apply them for `PlayerAttack` and `MobAttack` in the guardian's next tick, which still runs while it dies.
   - MW2 bullets arrive as `PlayerAttack` (`survival/entities.rs:1412`), so they prick too. Splitting them off needs a `PlayerAttack` field at about 30 sites.
   - The alternative is an immediate root `Hurt` hook.
3. **Client clocks:** computed by the host and sent in `FamilyShow`. D0's `draw` and `client_effects` see the mobs immutably.
4. **Hum:** replay the 2.0 s sample when it ends, at that tick's vanilla volume and pitch.
5. **Curse cue:** `PlayerEffect.cue` and `EffectGrant.cue` plus a vanilla-shaped `ToClient::GameEvent`, rather than a fork-chosen `PlayerEvent` byte. The pulse uses `dolphins-grace`'s effect queue, not a channel of its own.
6. **Eye:** the `FamilyDraw.camera` root hook.

---

## M2-6. Sulfur cubes (groups `sulfur-cubes-live`, `sulfur-cubes-drawn`, `sulfur-cubes-blocks`, `sulfur-cubes-kicks`, `sulfur-cubes-items`)

**Identity.** Family `sulfur_cubes`, state `SulfurCube`, kind `minecraft:sulfur_cube`, group `water`, **key 72**, row 49 of `docs/round10/mobs.md`. Lines are `minecraft-survival` at `d1a3d028`; D0 names come from `track-d0.md` and D0's branch at `9eecb2ff`. Re-check every line before editing.

**What it is.** The 26.3 sulfur cube is a passive, hopping `AbstractCubeMob`, the slime's 26.3 base, which is now an `AgeableMob`.
- An adult swallows one block and then stops moving. The block's **archetype** decides how it slides, bounces, floats and takes kicks.
- While it holds a block, melee, arrows, MW2 bullets and blasts do not hurt it. They kick it like a ball.
- TNT inside makes it a lightable bomb. A magma block inside burns whatever touches it.
- Shears take the block out, and an empty bucket scoops the cube up.

### What exists today

- **Nothing simulates it.**
  - There is no `type_info` row (`world/src/natural_spawner.rs:324`, `_ => return None`), so sulfur caves never spawn one, even though the biome generates (`generator/src/biome_source.rs:647`) and its spawn list is read with uniform counts (`natural_spawner.rs:447`, `:480`).
  - A saved cube stays dormant, and its egg does nothing.
- **Lists that already name it:**
  - `crates/minecraft_terrain/src/dormant.rs:9` `ALLOWED_IN_PEACEFUL`;
  - `entities/src/world/riding.rs:8` `NON_CONTROLLING_RIDERS`;
  - `world/src/level/burning.rs:35` (`sulfur_cube_hot` is `#is_fire`);
  - the panic-cause lists `entities/src/feline_ai.rs:109`, `wolf_ai.rs:179` and `world/wolves.rs:140`;
  - `world/src/level/dispenser.rs:24` (`sulfur_cube_bucket` unported);
  - `player/src/dropper.rs:18-20`.
- **To reuse:**
  - the slime port: `entities/src/slime.rs` (`dimensions` `:97`), and `world/slimes.rs` with `tick_ai` `:98` (the `CubeMobMoveControl` turn and hop), `land` `:206` (the squash), the touch box in `tick_slime` `:274`, and `split_dead_slimes` `:363`;
  - the cube goals in `monster_ai.rs:1150-1267` (private structs, with the slime's attack goal) and `CubeMove` `:311`;
  - `Age` (`entities/src/age.rs:6`);
  - `PlayerControls::holds` (`controls.rs:18`) through `player_controls` (`world.rs:5242`);
  - `RecipeBook::item_in_tag` (`player/src/crafting.rs:870`);
  - `World::items_in`/`item`/`take_item` (`player/src/lib.rs:105-114`), used as the piglin pickup does (`world/piglins.rs:1015`);
  - saving the `equipment.body` slot (`server_mobs.rs:636`, `:1454-1461`);
  - `slime_render::size_and_squish` (`slime_render.rs:31`) and `cow_render::cube_scaled`;
  - the TNT swell and flash (`mesh.rs:1124-1130`).
- **Gaps:**
  - `Body::travel_air_flying` (`movement.rs:504-609`) uses raw block friction and drag 0.91/0.98, and `restitute` (`:641-671`) has no entity bounciness.
  - D0's knockback arm (`track-d0` `combat.rs:236-241`) is the plain `power·(1 − kr)` with no attacker eye or look.
  - D0's blast arm (`track-d0` `world.rs:3041-3049`) ignores explosion knockback resistance.
  - `World` (`player/src/lib.rs:95`) has no redstone signal and no `tnt_explodes`.
  - `FamilyDraw` (D0-5) has no block list; blocks inside mobs go through `append_posed_blocks` (`mesh.rs:1225`, used at `crates/minecraft_survival/src/entity_view.rs:1364-1367`).

### Vanilla 26.3 (javap this round)

**Type** (`EntityTypes.SULFUR_CUBE`):
- `MONSTER`, `sized(0.49, 0.49)`, `eyeHeight(0.175)`, `spawnDimensionsScale(2)`, `clientTrackingRange(10)`;
- **not** `notInPeaceful` (it stays on peaceful), not `fireImmune`;
- not an `Enemy`, not undead, not a `Monster` (it does not block sleep).

`SpawnPlacements`: `ON_GROUND`, `MOTION_BLOCKING_NO_LEAVES`, and `checkSulfurCubeSpawnRules`, which is always true (no light rule).

**Size.**
- Size scales the type box (no age scale).
- `setSize(n)` clamps to 1..127 and sets speed `0.2 + 0.1n` and max health **4n** (`SulfurCube.setCubeMobHealth`). `SulfurCube.setSize(1, true)` makes it a baby.
- Attributes: `Mob.createMobAttributes` plus `TEMPT_RANGE` 8; follow range 16.

| | Size | Box | Eye | Health | Speed | XP |
|---|---|---|---|---|---|---|
| Adult | 2 | 0.98 | 0.35 | 8 | 0.4 | `1 + nextInt(2)` |
| Baby | 1 | 0.49 | 0.175 | 4 | 0.3 | 0 |

- `setSpawnSize` (no random draws): a baby gets size 1, anything else size 2.
- `ageBoundaryReached`: growing up sets size 2 at full health.
- `getSplitCount`: 2, or 0 when primed. A dead cube of size above 1 splits into that many cubes of half its size, placed and turned as slimes are, and each one is a baby (`setUpSplitCube`).
- It never breeds: it is not an `Animal`.

**AI.**
- Goals, by priority:
  - 1 `CubeMobFloatGoal`, 4 `CubeMobRandomDirectionGoal`, 5 `CubeMobKeepOnJumpingGoal`;
  - then 2 `SulfurCubeTemptGoal` and 3 `SulfurCubeSearchForItemsGoal`;
  - no targets. `canDealDamage` is false, so it never hurts anyone.
- **Tempt** (`TemptGoal.ForNonPathfinders(mob, 1.0, pred, false, stop 1.0)`, LOOK flag):
  - a baby follows `#sulfur_cube_food` (slime ball), an adult `#sulfur_cube_swallowable`, in either hand of the nearest player within 8;
  - calm-down `reducedTickDelay(100)`;
  - within 1 block: `setWantedMovement(0)`; otherwise `lookAt(10, 10)` and `setDirection(yRot, true)` (aggressive, a third of the hop delay).
- **Search** (LOOK): only an adult with `pickupTimer <= 0`. It picks the nearest item entity within its box inflated by 8 that has no pickup delay, is alive and is swallowable; then `lookAt` and aggressive `setDirection`.
- **Pickup:**
  - `canPickUpLoot` holds while it has no block; `canHoldItem` needs an empty slot, a swallowable item and an adult;
  - `pickUpItem` (when `pickupTimer <= 0`): one item into the BODY slot, the absorb sound, `setGuaranteedDrop(BODY)`, `take(item, 1)`;
  - javap `Mob.aiStep` during the work for the reach `(1, 0, 1)` and the `mob_griefing` gate.
- **Holding a block:**
  - the move control is skipped (`SulfurCubeMobMoveControl`);
  - the look control sets `yRot -= wrapDegrees90(yRot)`, which snaps the facing to multiples of 90°;
  - `customServerAiStep` counts `pickupTimer` and `pushSoundCooldown` down;
  - the hop delay is `nextInt(20) + 10`.

**Swallowing** (`collectEquipmentChanges`, BODY slot).
- Swallowing a block clears all goals and sets speed 0. Emptying the slot registers fresh goals.
- The old archetype's modifiers and settings go. The defaults are knockback `0.33/0.06`, sounds `regular.hit`/`regular.push`, push threshold 0.2 and cooldown 0.5.
- Every archetype whose `items` holds the new block applies its `buoyant`, `explosion`, `contact_damage`, `knockback_modifiers`, `sound_settings`, and `attribute_modifiers` as transient modifiers. Those are not saved; they are rebuilt from the slot on load.
- Archetypes are a data registry: `data/minecraft/sulfur_cube_archetype/*.json` with the tags `#sulfur_cube_archetype/<name>`.

Values in the table are after the modifiers: KR and EKR are added to 0; friction and drag multiply 1.

| Archetype (blocks) | KR | EKR (0..1) | Bounce | Friction | Air drag | Floats | Kick h/v | Push s |
|---|---|---|---|---|---|---|---|---|
| `bouncy` (planks, logs, bamboo) | −2 | 0 | 0.9 | 0.3 | 0.01 | yes | 0.4125/0.105 | 0.3/0.7 |
| `regular` (dirts, mud, clay, concrete powder, coal and bone blocks) | −1 | 0 | 0.5 | 0.3 | 0.1 | yes | 0.4125/0.09 | 0.2/0.5 |
| `explosive` (TNT): power 3, no fire, fuse 120 | −1 | 0 | 0.5 | 0.3 | 0.3 | yes | 0.4125/0.09 | 0.1/0.7 |
| `hot` (magma block): contact 1.0 `sulfur_cube_hot`, not attributed | −1 | 0 | 0.5 | 0.3 | 0.1 | yes | 0.4125/0.09 | 0.2/0.7 |
| `light` (wool) | −1 | 0 | 1.0 | 0.3 | 1.8 | yes | 0.4125/0.18 | 0.2/0.7 |
| `fast_flat` (coral blocks, sponges, moss, resin, melon, hay, pumpkins, froglights, kelp) | −1 | 0 | 0.5 | 0.2 | 0.01 | no | 0.9125/0.09 | 0.03/0.9 |
| `fast_sliding` (blue and packed ice, snow block) | 0.5 | 0.5 | 0.1 | 0.05 | 0.01 | no | 0.6625/0.09 | 0.05/1.0 |
| `slow_sliding` (mushroom blocks, mycelium, wart, shroomlight) | 0.8 | 0.8 | 0.1 | 0.05 | 0.01 | no | 0.4125/0.09 | 0.02/1.0 |
| `slow_bouncy` (stone, deepslate, bricks, ores, terracotta, …) | 0.4 | 0.4 | 0.6 | 0.3 | 0.05 | no | 0.4125/0.24 | 0.05/0.5 |
| `slow_flat` (metal blocks, copper, ancient debris) | 0.5 | 0.5 | 0.4 | 0.4 | 0.1 | no | 0.4125/0.105 | 0.03/0.9 |
| `high_resistance` (soul sand and soil) | 0.7 | 0.7 | 0.2 | 1.0 | 0.01 | no | 0.4125/0.09 | 0.03/0.7 |
| `sticky` (honeycomb block) | −2 | 0 | 0 | 2.0 | 0.01 | no | 0.4125/0.09 | 0.05/0.5 |

**Physics.**
- Attribute ranges and defaults:
  - `bounciness` 0 (0..1);
  - `friction_modifier` and `air_drag_modifier` 1 (0..2048);
  - `explosion_knockback_resistance` 0 (0..1);
  - `knockback_resistance` −2..1.
- `travelInAir`, with `cmf(f, m) = clamp(1 − (1 − f)·m, 0, 1)`:
  - ground friction `f = cmf(block, friction_modifier)`;
  - horizontal drag `f·cmf(0.91, air)`;
  - vertical drag `cmf(0.91, air)` when `omnidirectionalAirMover` (true while it holds a block), else `cmf(0.98, air)`.
  - At m = 1, `cmf(f) == f` bit for bit for every f ≥ 0.5, so default bodies do not change.
- `restituteMovementAfterCollisions`:
  - walls: `v = −v·bounciness`;
  - floor: `max(bounciness, block restitution)` unless suppressed (`#suppresses_bounce`); a block bounce posts `bounce`.
- While holding a block:
  - `maxUpStep` 0, it cannot freeze, and it breathes under water;
  - when buoyant, `travelInFluid` adds `min(1, h)·0.04` to vy whenever `h = fluidHeight − 0.2·height + 0.2·sin(tick·0.4)` is above 0.

**Hurt and kicks.**
- While it holds a block, `hurtServer` first handles the fuse:
  - an explosive, unprimed cube hit by `#is_fire` or a burning arrow primes (long fuse);
  - hit by `#is_explosion`, it primes with a short fuse.
- If the source is in `#sulfur_cube_with_block_immune_to`, it takes `dealDefaultKnockback` (unless `#no_knockback`) and returns **true with no damage**: no hurt time, no flash, no hurt sound.
  - The tag holds `arrow, cactus, dry_out, fall, falling_anvil, falling_block, falling_stalactite, freeze, mace_smash, hot_floor, mob_attack, mob_attack_no_aggro, mob_projectile, player_attack, spear, spit, stalagmite, sting, sulfur_cube_hot, sweet_berry_bush, thrown, trident, wind_charge, #is_explosion`.
  - Lava, fire, magic and suffocation still hurt.
- `knockback(strength, x, z, src, damage, extra)` is plain without a block or without `src.getEntity()`. Otherwise:
  1. Rotate `(x, z)` by 1.6 × the signed angle between the attacker's look and its eye-to-centre line.
  2. Split by aim: `t = clampedMap(look.y, dirTop.y, dirBottom.y, −1, 1)`, using the points at centre ± height/2. With `p = sign(t)·|t/2|`: `h·(1 − p)`, `v·(1 + p)`. Aiming high skims it flat; aiming low pops it up.
  3. Rotate `(h, v)` by `−0.8·atan2(−dy, horizontal distance)` (attacker feet to cube), then rescale so neither exceeds its base.
  4. Multiply by `sqrt(damage)·(extra ? strength·0.25 : 1)·(1 − KR)`.
  5. Apply `h·0.4`, clamp to ±128, and set `vel = (vx − dir.x, vy + 1.2v, vz − dir.z)`.
  6. Play the archetype `hit_sound`.
- Blasts push it by `1 − EKR`.

**Push and contact.**
- `playerTouch` → `playerPush` needs a block, a horizontal distance under 1.3 and vertical overlap (the player's root vehicle if riding).
  - With `k = max(0, 1 − KR)` and `s = clamp(knownSpeed·2·(riding ? 0.16 : 0.3), 0, 0.5)`, it adds `(n.x·k, onGround ? 0.3k : 0, n.z·k)·s`.
  - It plays `push_sound` if `|Δ|² > threshold²` and the cooldown (seconds × 20) is over.
  - Then it applies contact damage.
- `doPush` (collisions) applies contact damage too.
- `sulfur_cube_hot` damage: the source is the cube only for players. Scaling is `when_caused_by_living_non_player`. The death line is "… died because not just the floor is lava".

**Fuse.**
- `fuse` is −1 until primed. `primeTime(short)` needs explosion data, alive, the `tnt_explodes` rule, and not already primed.
- Priming sets:
  - `fuse` to `short ? nextInt(30) + 15 : 120`;
  - the cube permanently invulnerable;
  - `MAX_FUSE` synced;
  - `entity.tnt.primed` and `prime_fuse`.
- `tick` runs `tickFuse` and the powered-position check (`getBestNeighborSignal != 0` primes) **before** the living tick.
- At 0: drop the leash; if `tnt_explodes`, explode at y + 0.0625 with power 3, no fire, interaction `mob_griefing ? TNT : NONE`; then `discard()`. No loot, no experience, no split.

**`mobInteract`, in order** (the lead is handled before it):
1. **Baby:**
   - a slime ball with `canAgeUp`: `ageUp(getSpeedUpSecondsWhenFeeding(−age), true)` and `entity.small_sulfur_cube.eat`;
   - anything else goes to `AgeableMob` (golden dandelion lock);
   - a baby never reaches steps 2-6.
2. **Primed:** PASS.
3. **Explosive with flint and steel or a fire charge:**
   - rule off: overlay `block.minecraft.tnt.disabled`;
   - else prime (long fuse); the flint wears 1, the charge is used up.
4. **Shears on a block holder:**
   - the block drops at the passenger attachment, with `entity.sulfur_cube.eject`;
   - `pickupTimer` 100, `shear`, the shears wear 1.
5. **A swallowable item:** `equipItem`.
   - Refused by a baby or when it already holds that item.
   - An old block drops.
   - The slot takes one item (`setItemSlotAndDropWhenKilled`), with `entity.sulfur_cube.absorb`; one item is used.
6. **`bucketMobPickup`:**
   - The item is **`minecraft:bucket`**: `canBePickedUpWithBucket` is overridden, so not a water bucket.
   - Sound `item.bucket.fill_sulfur_cube`.
   - The bucket stack gets `bucket_entity_data` (defaults plus `age` and `age_locked`) and `minecraft:sulfur_cube_content` (the block as an item stack, tooltip "Contains: %s").
   - Then `dropLeash` and `discard`.
- **Emptying the bucket:**
  - EMPTY fluid: no water, placed against the clicked face, `item.bucket.empty_sulfur_cube`;
  - the content becomes the body block; `age` and `age_locked` load; `from_bucket` is set.
- **Persistence and leads:**
  - kept while it holds a block or came from a bucket;
  - only a block holder takes a lead (offset `(0, h/2, 0)`).

**Saving.** `Size` (size − 1), `wasOnGround`, `pickup_timer`, `from_bucket`, `fuse`, the `AgeableMob` age keys, and `equipment.body` with its guaranteed drop chance. javap `AgeableMob.addAdditionalSaveData` and `Mob.setGuaranteedDrop` during the work.

**Sounds** (NEUTRAL, volume `0.4·size`, slime pitches):
- `entity.sulfur_cube.{hurt,death,squish,jump}`, or `entity.small_sulfur_cube.*` when tiny;
- the squish becomes `entity.sulfur_cube.bounce` and steps are silent while it holds a block;
- `absorb`, `eject`, `small_sulfur_cube.eat`, `<archetype>.hit`/`.push`, `item.bucket.fill/empty_sulfur_cube`.

**Loot and spawning.**
- The loot table `entities/sulfur_cube` is empty.
- Spawns: `sulfur_caves`, monster category, weight 100, uniform count 2-4.
- `AgeableMob.finalizeSpawn(null)` starts `AgeableMobGroupData(true)`, so the second and later members are babies with chance 0.05. Then `setSpawnSize` runs.
- `/summon` and eggs always give adults.

**Client.**
- **Models** (`PartPose.ZERO`, `entityTranslucent`):
  - `SulfurCubeModel`: shell 18³ px at uv (0, 0) and core 16³ at (0, 36), 128×128;
  - `SmallSulfurCubeModel`: shell 10³ at (0, 0) and core 8³ at (0, 20), 64×64;
  - textures `entity/sulfur_cube/sulfur_cube_{outer,inner}[_small]`.
- **`SulfurCubeRenderer.scale`**, in order:
  1. ×0.999 and +0.001;
  2. `applySizeAndSquish`, with the squish 0 while it holds a block;
  3. ×`(1 + clamp(1 − f/10)⁴·0.3)` while `0 < f < 10`;
  4. ×0.5 for an adult, ×1 for a baby;
  5. translate `(0, (baby ? 1.24 : 0.98) − 1/16, 0)`.
- **Shadow** `size·0.25`.
- **`SulfurCubeInnerLayer`** (order −1, drawn before the shell):
  - with a block: rotate X 180°, ×0.5 for a baby, translate `(−0.5, −0.518, −0.5)`, then the block's default model, flashing white while `f ≥ 0 && (int)(f/5) % 2 == 0`;
  - otherwise the translucent core;
  - with a skull: nothing (`CustomHeadLayer` draws it).
- `f = fuse − partial + 1`.
- **Landing:** `width·2·16` `sulfur_cube_goo` crumbs (`BreakingItemParticle`).
- **Keyframes:** none.

### Plan

**Files (D0-3).**
- Entities `world/families/water/sulfur_cubes.rs`: `SulfurCube`, `impl Family`, the tick, tests. It sits beside:
  - `sulfur_cubes/ai.rs`: tempt and search goals, `SPEC`, `SulfurGoals` in `GoalExtra` (tempting players, the wanted item), and the cube hop and move control, ported from `world/slimes.rs:98-200` (the slime keeps its code, D0-0);
  - `sulfur_cubes/archetypes.rs`: `Archetypes::from_jar` (12 JSON files, read like `ShearingLootBook::from_jar`), `static ARCHETYPES: OnceLock` with `install`, matched through `item_in_tag`;
  - `sulfur_cubes/kick.rs`: the three hit-angle steps, `player_push`, contact damage.
- Terrain `families/water/sulfur_cubes.rs`: `impl TerrainFamily` (`read`, `fields`, `show`, `show_on`, `draw`, `interact`, `client_effects`), with `sulfur_cubes/render.rs` for the model port.
- World: the `minecraft:sulfur_cube` row in `natural_spawner/families/water.rs`:
  - 0.49 box, `OnGround`, `MotionBlockingNoLeaves`, `Rules::Family { creature: false }`, neutral walk, removed far away, cluster 4;
  - `Monster` category, follow range 16, `baby_chance: None`, no `rule` (it always passes);
  - a `finalize` that does the AgeableMob 5% roll (the group data counts members) and sets `Size`, health and speed, before the root's `finalize_mob`.
- Registration lines:
  - entities `families/water.rs`: `pub mod sulfur_cubes;` and `sulfur_cubes: sulfur_cubes::SulfurCube,`;
  - terrain `families/water.rs`: `mod sulfur_cubes;` and `SulfurCube` in `terrain_group!`.

**Hooks set (D0-2.2):**
- `KEY` 72; `spawn` gives an adult of size 2.
- `tick`:
  1. `family_base_tick`;
  2. the fuse and the powered-position check;
  3. the cube hop when it holds nothing (through M1 rabbits' `family_goal_step_with`), or a still body when it holds a block;
  4. pickup, player push and contact, buoyancy;
  5. `family_after_move`.
- `category` `"monster"`.
- `hurt`: immunity and priming, else `hurt_plain`.
- `experience`; `death_drops` (the block).
- `despawn`: the monster rule with `peaceful_ok: true` and `kept: block || from_bucket`.
- `breathes_water` and `leashable`: while it holds a block. `leash_walk` does nothing (it cannot walk; the elastic pull is generic).
- `knockback_resistance`: the archetype's, clamped −2..1.
- `voice`: `("sulfur_cube"` or `"small_sulfur_cube", 0.4·size, "neutral_volume")`. The override is needed: D0's default gives category `monster` `"hostile_volume"`, and vanilla's source is `NEUTRAL`.
- `monster`: `Some(&SPEC)`; the goals are `cube_goals()` plus tempt and search, with no targets; eye `0.175·size`.

**Root hooks (each its own commit, logged; the defaults leave every family unchanged):**
1. **Knockback.** `Knock { power, xd, zd, damage, extra, by: Option<Knocker { feet, eye, look }> }`, plus:
   - `Family::knockback(mob, &Knock) -> bool`, default false;
   - `Family::explosion_knockback_resistance`, in `MobTraits`.
   - D0's arms ask the hook first:
     - the three `knock_back` callers in `combat.rs` (the 0.4 hit, the sprint `extra`, the sweep) build the attacker with `EntityWorld::player_knocker(player, feet)`: eye = feet + 1.62, or 1.27 sneaking; look from `player_controls`;
     - `mob_hits_mob` and `apply_arrow_hit` (shooter as `by`);
     - `hurt_by_blast`, whose push gets ×`(1 − ekr)`.
2. **A block inside a mob's model.** `FamilyDraw.blocks: &mut Vec<FamilyBlock { pose, light, id, white }>`.
   - `entity_view.rs` chains the plain ones into `carried` (`:1364`).
   - White ones go through a new `ItemVisuals::append_lit_blocks` beside `append_posed_blocks` (`mesh.rs:1225`; brightness 4, as `append_primed_tnt` uses at `:1137`).
   - `spawner_display` gets the same list.
3. **Family blasts** (the first commit of `sulfur-cubes-items`, "Registered Minecraft mobs can set off explosions"). `Families.blasts: Vec<FamilyBlast { position, power, fire, tnt }>` and `take_blasts()`.
   - Terrain `explode_creepers` (`server.rs:1593`) gains one line: TNT rules (`tnt_explosion_drop_decay`) when `mob_griefing`, else blocks are kept.
   - The entity side hurts and pushes through `apply_explosion_by(.., Blaster::Mob(id), ..)`.
4. *(Optional)* **Crumbs for families:** not a hook of this section. M1's pandas own `ClientFx.item_crumbs` ("Registered Minecraft mobs can scatter item crumbs", M20); the goo uses it when it is merged, through `spawn_item_crumbs` (`block_particles.rs:185`), else the goo waits.

**Shared lines (each beside a related one):**
- `monster_ai.rs`: `pub fn cube_goals()` (1, 4, 5) just before `slime_goals` (`:1150`).
- `movement.rs`: `BodyPhysics { bounciness, friction_modifier, air_drag_modifier, omnidirectional }` with vanilla defaults, as `Body.physics` (set in `Body::new`). It is used by `travel_air_flying` (`:516-520`, `:597-607`) and `restitute`. Squids (M2-2) have split `Body::move_self` out of `travel_air_flying` by then: the bounciness goes in `restitute`, which both paths reach.
- `player/src/lib.rs` `World`: defaults `best_neighbor_signal(pos) -> 0` and `tnt_explodes() -> true`, after `snow_golem_melts` (`:192`; M20: M1's bees take the lines after `block_in_tag`).
  - `MobWorld` (`server_mobs.rs:169`) implements them after its `snow_golem_melts` (`:361`) from `Level::best_neighbor_signal` (`redstone.rs:443`) and `Level.tnt_explodes` (`level/mod.rs:200`).
  - The tick caches the rule for `interact`, which has no level.
- Terrain `ServerSim::load_loot`: one line after `:501` installs the archetypes.
- M2's `families/water/buckets.rs` (M2-1): this family answers M2-1's `Bucketable::PICKUP_ITEM` with `minecraft:bucket`, and fills the `sulfur_cube_bucket` row's converters (`age`, `age_locked`, `sulfur_cube_content` ↔ the body block).

**Multiplayer.**
- `FamilyShow`:
  - flags bit 0 = `water::IN_WATER` (M2-1);
  - `values[0]` = fuse (−1 when not primed);
  - `values[1]` = squish bits; `show_on` moves the old value into `previous_squish`;
  - `values[2]` = the tick of its last landing (for the goo).
- `VOLATILE = ["fuse", "pickup_timer"]`.
- The block, `Size` and `Baby` ride the look tag. A swallow, a shear or growing up respawns the friend's copy within `LOOK_TICKS`, as zombie equipment does.
- Friends' actions run on the host.

**Attacks on players.** Only the hot contact: `PlayerHitKind::Typed { damage_type: "minecraft:sulfur_cube_hot", message: "sulfurCubeHot", .. }`.

**Commit groups, in order:**
1. **`sulfur-cubes-live`** (m2e)
   1. "Minecraft sulfur cubes hop about in the entity world": family, `cube_goals`, tempt and search, ageing, split, sizes, sounds.
   2. "Minecraft sulfur cubes are saved, hatched, seen by friends and spawn in sulfur caves": terrain `read`/`fields`/`show`/`show_on` and the spawn row. Row 49 becomes `live`.
2. **`sulfur-cubes-drawn`** (m2e)
   1. "Registered Minecraft mobs can show a block inside their model" (root hook 2).
   2. "Minecraft sulfur cubes are drawn like vanilla".
   3. *(Optional, only with M1's `ClientFx.item_crumbs` merged)* "Minecraft sulfur cubes splash goo when they land". Row becomes `drawn`.
3. **`sulfur-cubes-blocks`** (m2e)
   1. "Minecraft mobs slide, drift and bounce by their physics attributes".
   2. "Minecraft sulfur cubes swallow blocks and take on their archetypes": by hand, by pickup and search, shears, the archetypes from the jar, floating, the square facing, leads, persistence, the player push, hot contact.
4. **`sulfur-cubes-kicks`** (m2e; through E2's `hurt_for_player` when `ti1` is in the branch)
   1. "Registered Minecraft mobs take knockback their own way" (root hook 1).
   2. "Minecraft sulfur cubes holding a block are kicked instead of hurt": immunity, kick, hit sounds, EKR, MW2 bullets.
5. **`sulfur-cubes-items`** (m2e)
   1. "Registered Minecraft mobs can set off explosions" (root hook 3: the queue, `take_blasts`, the terrain line in `explode_creepers`, the entity side through `apply_explosion_by`).
   2. "Minecraft sulfur cubes holding TNT can be lit and blow up": `World` hooks, flint, charges, fire, the cube's blasts through hook 3, redstone, the rule, the friend's swell and flash.
   3. "Minecraft sulfur cubes go into empty buckets and baby cubes grow on slime balls": bucket pickup and emptying, feeding, golden dandelion. Row becomes `done`; the owner's steps and kit go in the log.

### Tests

**live**
- `families::water::sulfur_cubes::tests`:
  - `a_sulfur_cube_is_0_98_wide_with_8_health_and_its_kind`: size, eye, speed, `entity_type`, census, `mob_candidates`, empty table, 1-2 experience;
  - `a_baby_sulfur_cube_is_0_49_wide_with_4_health_and_gives_no_experience`;
  - `it_hops_like_a_slime_and_never_hurts_a_player`;
  - `a_dead_adult_splits_into_two_babies_and_a_baby_does_not`;
  - `a_held_swallowable_block_tempts_an_adult_and_a_slime_ball_a_baby`;
  - `a_baby_grows_to_size_two_when_its_age_runs_out`;
  - `it_stays_on_peaceful_and_despawns_like_a_monster`;
  - `a_baby_makes_the_small_sounds`.
- `natural_spawner::families::water::tests`:
  - `sulfur_cubes_spawn_two_to_four_in_sulfur_caves_at_any_light`;
  - `later_cubes_of_a_group_may_be_babies`;
  - `a_summoned_sulfur_cube_is_an_adult_of_size_two`.
- Terrain:
  - `a_sulfur_cube_saves_and_loads_with_its_size_age_block_fuse_and_bucket_flag`;
  - `a_friend_sees_a_sulfur_cube_from_its_look_tag`: the volatile keys, and the squish and `IN_WATER` through `FamilyShow`.

**drawn**
- `families::tests::a_family_block_is_drawn_inside_its_mob`.
- `sulfur_cubes::render::tests`:
  - `an_adult_has_an_eighteen_pixel_shell_over_a_sixteen_pixel_core_on_its_feet`;
  - `a_baby_uses_the_small_models_and_textures`;
  - `a_held_block_fills_the_core_and_stops_the_squish`;
  - `the_last_ten_fuse_ticks_swell_it_and_flash_the_block_white`;
  - `its_shadow_is_a_quarter_of_its_size`.
- Optional: `a_landing_cube_splashes_goo_for_every_viewer`.

**blocks**
- `movement::tests`:
  - `default_physics_move_every_body_as_before` (`cmf(f, 1) == f` for 0.6, 0.8, 0.98, 0.989, 0.91; the existing fixtures stay green);
  - `a_bouncy_body_bounces_off_walls_and_floors`;
  - `low_friction_and_drag_slide_and_drift_further`;
  - `an_omnidirectional_body_drags_alike_in_all_directions`.
- `sulfur_cubes::tests`:
  - `a_block_used_on_an_adult_is_swallowed_and_it_stops_hopping`;
  - `a_baby_or_the_same_block_is_refused_and_another_swaps_and_drops_the_old`;
  - `shears_eject_the_block_and_it_waits_100_ticks_before_taking_another`;
  - `an_adult_hops_to_a_dropped_block_and_swallows_it`;
  - `each_archetype_matches_the_jar` (gated);
  - `a_buoyant_cube_bobs_and_a_heavy_one_sinks_without_drowning`;
  - `a_walking_player_pushes_it_and_plays_the_push_sound`;
  - `a_hot_cube_hurts_a_player_and_a_mob_it_touches`;
  - `only_a_cube_holding_a_block_takes_a_lead`;
  - `a_cube_holding_a_block_never_despawns_and_drops_it_on_death`;
  - `its_facing_snaps_to_the_grid`.

**kicks**
- `families::tests::a_family_can_replace_the_plain_knockback`.
- `sulfur_cubes::tests`:
  - `melee_arrows_and_blasts_move_but_do_not_hurt_a_cube_holding_a_block`;
  - `a_kick_follows_the_aim_and_lifts_more_when_aimed_low`;
  - `a_resistant_archetype_flies_less_far`;
  - `a_blast_pushes_it_by_its_explosion_knockback_resistance`;
  - `lava_and_magic_still_hurt_it`.
- Survival: `entities::tests::an_mw2_bullet_shoves_a_cube_holding_a_block`.

**items**
- Commit 1 (the hook): `families::tests::a_queued_family_blast_hurts_and_pushes_mobs_and_is_taken_once`; `server::tests::a_family_blast_breaks_blocks_by_tnt_rules_and_keeps_them_without_mob_griefing` (a `FamilyBlast` pushed onto the queue directly).
- `sulfur_cubes::tests`:
  - `flint_and_steel_lights_an_explosive_cube_that_blows_up_after_120_ticks`;
  - `a_blast_lights_it_with_a_fuse_of_15_to_44_ticks`;
  - `fire_and_a_burning_arrow_light_it`;
  - `redstone_power_lights_it`;
  - `with_tnt_explodes_off_it_cannot_be_lit_and_a_lit_one_vanishes_without_a_blast`;
  - `a_lit_cube_cannot_be_hurt_or_split_and_drops_nothing`.
- Terrain:
  - `server::tests::a_sulfur_cube_blast_breaks_blocks_by_tnt_rules`;
  - `an_empty_bucket_scoops_an_adult_with_its_block_and_age`;
  - `a_baby_cannot_be_scooped`;
  - `an_emptied_sulfur_cube_bucket_places_no_water_and_brings_back_the_cube_with_its_block`;
  - `slime_balls_grow_a_baby_by_a_tenth_of_its_time`.

### Owner's steps

Setup: survival, Normal, by day, on flat ground in the Overworld, inside a pen 12×12 and 2 high built from the kit's glass and stone (the walls take about 90 blocks). Leave the gamerules at their defaults.

1. Use the sulfur cube egg three times in the pen. Yellow-green translucent jelly cubes, a block wide, hop about and never hurt you. **sc1_cubes**
2. Hold dirt within 8 blocks: the cubes hop after you and stop about a block away. Put the dirt away: they lose interest.
3. Use dirt on one cube. One dirt leaves your hand with a gulp, the dirt shows inside the cube, and it stops hopping, squared to the grid. **sc2_swallowed**
4. Punch it. There is no red flash, only a thud, and it slides off along your aim. Aim at its top edge: it skims flat and far. Aim at its bottom edge: it pops up. Kicked into the glass, it bounces back. Shoot it with the M4A1: no damage, but every bullet shoves it.
5. Walk into it: it is pushed ahead with a squelch. Sprint into it: it goes further.
6. Use shears on it. The dirt pops out ("Block removed"), and it hops again. Drop dirt nearby with Q: after about 5 s it hops over and swallows it.
7. Give one cube each of oak planks, blue ice, a honeycomb block, white wool, soul sand and an iron block, then punch each once. **sc3_archetypes**
   - The planks cube bounces off the walls.
   - The ice cube slides very far.
   - The honeycomb cube barely moves.
   - The wool cube floats down slowly after popping up.
   - The soul sand and iron cubes hardly move.
8. Inside the pen, run `/fill ~2 ~-2 ~2 ~4 ~-1 ~4 minecraft:water` for a pool 3 × 3 and 2 deep, and push the planks cube and the iron cube into it. The planks cube bobs on top; the iron cube sinks and does not drown.
9. Give a cube a magma block and walk into it: you take half a heart. Hatch a zombie beside it: the zombie is hurt when it bumps the cube.
10. Give a cube TNT and use flint and steel on it. It hisses, then swells and flashes white in its last half second, and explodes after 6 s, breaking the ground and the pen wall near it. **sc4_tnt_cube**
    - Give another cube TNT and light normal TNT next to it: the cube goes off soon after.
    - Give a third cube TNT and push it onto a redstone block: it lights.
    - Run `/gamerule tnt_explodes false` and use flint and steel on a fourth cube holding TNT: nothing happens. Run `/gamerule tnt_explodes true`.
11. Kill an empty adult with the sword: it splits into two small cubes and leaves a little experience. A small one leaves none. Pour lava on a cube holding wool: it dies and drops the wool.
12. Run `/summon minecraft:sulfur_cube ~ ~ ~3 {Age:-200}`: the small cube grows to full size after 10 s. Each slime ball fed to another small cube speeds its growth. An empty bucket does nothing to a small cube.
13. Use an empty bucket on an adult holding planks: you get a Bucket of Sulfur Cube. Place it on the ground: no water appears, and the cube is back with its planks. **sc5_bucket**
14. Use a lead on a cube holding a block: it is leashed and drags along behind you. An empty cube refuses the lead.
15. Save and quit, then load. The cubes keep their blocks and sizes, and a lit cube still explodes.
16. Run `/locate biome minecraft:sulfur_caves` and `/tp` there. You find groups of 2-4 cubes, and `/difficulty peaceful` keeps them. Run `/difficulty normal` afterwards. **sc6_sulfur_caves**
17. **Friend window.** The friend sees the cubes hop and squash, the blocks inside, and a lit cube swell and flash. The friend kicks a cube, feeds it dirt, shears it and buckets it, and the host sees every change. **sc7_friend_host** and **sc7_friend_friend**

### Kit

```
IW4L_MINECRAFT_KIT=minecraft:sulfur_cube_spawn_egg*16,minecraft:bucket*4,minecraft:slime_ball*32,minecraft:dirt*16,minecraft:oak_planks*16,minecraft:blue_ice*8,minecraft:honeycomb_block*8,minecraft:white_wool*8,minecraft:soul_sand*8,minecraft:iron_block*4,minecraft:magma_block*8,minecraft:tnt*8,minecraft:stone*64,minecraft:glass*64,minecraft:shears,minecraft:flint_and_steel,minecraft:fire_charge*8,minecraft:redstone_block*4,minecraft:lead*4,minecraft:iron_sword,minecraft:bow,minecraft:arrow*32,minecraft:water_bucket,minecraft:lava_bucket,minecraft:zombie_spawn_egg*2,minecraft:cooked_beef*16,minecraft:hay_block*8
```

27 stacks. Every id exists in 26.3 (`assets/minecraft/items`).

### Depends on

- **D0 d0a**, all of it, including `MonsterKind::Registered`, `GoalExtra` and `PlayerHitKind::Typed`. d0b is not needed.
- **M2-1:**
  - `water-hooks`: `IN_WATER`, the silent steps (`emissions.rs:64`);
  - M1 rabbits' `family_goal_step_with` (already waited for by m2b);
  - `water-buckets`: `Bucketable`, the bucket row, `Level::empty_mob_bucket`, `pickup::<S>`. Needed by items commit 3.
- **W:** the `prime_fuse`, `shear`, `entity_interact` and `bounce` game events. Whichever merges second adds the lines.
- **E2:** root hook 1 goes through `hurt_for_player` if it has landed. Thrown items (`thrown` is in the immune tag) do 0 damage, so they give no kick.
- **E1:** none.

### Open decisions (recommended choice)

1. **Archetypes** are read from the jar at runtime with a gated test, not pinned in code (CLAUDE.md).
2. **The cube's blast** uses vanilla's TNT rules through root hook 3, not the creeper queue's decaying MOB rules.
3. **The baby roll on natural spawns** happens in the row's finalize, using the group data as the member count. D0's monster path calls no `ageable`.
4. **The player's known speed** comes from each touching player's feet on the previous tick, kept by the family. `PlayerVitals.velocity` stays zero (`entities.rs:1524`).
5. **The block-immunity damage tag** is a pinned constant with a gated jar test, because entities reads no damage-type tags.
6. **The bucket item:** M2-1's `Bucketable::PICKUP_ITEM` (here `minecraft:bucket`), not a special case in `pickup`.

---

## M2-7. Axolotls (groups `axolotls-live`, `axolotls-drawn`, `axolotls-hunt`, `axolotls-buckets`)

Family `axolotls`, group `water`, hit-key byte **67** (row 32 of `docs/round10/mobs.md`), one kind `minecraft:axolotl`. A brain family (D0-2.4): the brain lives in the state, no `MonsterSpec`. Lines are `minecraft-survival` at `d1a3d028`; re-check them before editing. Where D0's merged code differs from `track-d0.md`, follow the code.

### What exists today

- **Dormant.** Not in `SIMULATED_KINDS` (`crates/minecraft_terrain/src/server_mobs.rs:433`), so `spawn_saved` (`:580`) returns `None` (`:1321`) and eggs hatch nothing; `tracking_range` (`mob_tracking.rs:285`) gives 5.
- **Spawn table.** A size row (`third_party/minecraftoss/world/src/natural_spawner.rs:294`: 0.75 x 0.42, `InWater`, `MotionBlockingNoLeaves`) and `remove_far` (`:365`), but `Rules::Other` (`:354`, never spawned) and `Walk::Animal` (`:362`; vanilla's value is 0, `Walk::Neutral`). `MobCategory::Axolotls` (`natural_spawner/spawning.rs:105`), cap 5 (`:71`). `check_spawn_obstruction` (`natural_spawner.rs:821-823`) refuses fluid boxes except striders and drowned (squids' decision 6 fixes it for every `InWater` row).
- **Already aware of axolotls:** villagers look at the `axolotls` category (`entities/src/villager_brain/behaviors.rs:257`); `axolotl_bucket` is on the dispenser's not-ported list (`world/src/level/dispenser.rs:18`).
- **Reuse:**
  - brain: `brain_behaviors.rs` (`Living` `:17`, `targetable_within` `:89`, `within_melee_range` `:106`, `MoveSink` `:183`); the hoglin brain's `start_attacking` (`hoglin_brain.rs:435`), love (`:616`, `:645`), `follow_adult` (`:672`), `fight` (`:699`); `breed_hoglins` (`world/hoglins.rs:628`, experience into `trade_experience` `:646-648`) and `hoglin_living` (`:462`); M1 goats' `world/families/animal_brain.rs` once it lands;
  - `navigate_amphibious_to_with_accuracy` (`navigation.rs:206`), `tick_amphibious` (`:532`); `Body.touching_water` (`movement.rs:55`), `Body.air` (`:66`), `World::rain_at` (`player/src/lib.rs:189`), `drop_leash` (`world/leashes.rs:551`);
  - `Age` (`age.rs:6`), `animal::interact` (`animal.rs:12`), the milk swap as `createFilledResult` (`cow.rs:148-181`); held items through `player_controls` (`world.rs:5242`) and `PlayerControls::holds` (`controls.rs:18`);
  - Brain memories with `ttl` in saves (`piglin_save.rs:58`); render helpers `posed_cube::append` (`posed_cube.rs:47`), `vex_render.rs:88`, `:97`, `:137`; keyframes as `bat_render.rs:162-163`.

### Vanilla 26.3 (javap this round)

- **`EntityTypes.AXOLOTL`:** `MobCategory.AXOLOTLS`, `sized(0.75, 0.42)`, `eyeHeight(0.2751)`, `clientTrackingRange(10)`, allowed on peaceful, not fire immune. A baby's `getDefaultDimensions` is `scalable(0.375, 0.21).withEyeHeight(0.09375)` (passenger attachment 0.1875), replacing the 0.5 age scale.
- **Attributes:** health 14, movement speed 1.0, attack damage 2, step height 1.0, follow range 16.
- **`Axolotl extends Animal implements Bucketable`:**
  - `getWalkTargetValue` 0; `getMaxHeadXRot` and `getMaxHeadYRot` 1; `canBeLeashed`; not pushed by fluid; `checkSpawnObstruction` = `isUnobstructed`; `removeWhenFarAway` = `!fromBucket && !hasCustomName`; `requiresCustomPersistence` adds `fromBucket`.
  - `PathType.WATER` malus 0; `SmoothSwimmingMoveControl(85, 10, 0.1, 0.5, false)` and `SmoothSwimmingLookControl(20)`, both skipped while playing dead; `AmphibiousPathNavigation`.
  - `travelInWater`: `moveRelative(speed, input)`, `move`, velocity x 0.9 (no gravity, no drag). It runs while dying too, so a dead axolotl drifts.
  - `Animal`: ambient interval 120; experience `1 + nextInt(3)`, none for babies; a hurt resets love; `aiStep` zeroes `InLove` when age != 0, else counts it down with three `nextGaussian` draws every 10 ticks. `Axolotl.customServerAiStep` skips `Animal`'s.
  - Saved: `Variant` (int, default 0), `FromBucket`, `Age`, `ForcedAge`, `AgeLocked`, `InLove`, and Brain memories `minecraft:play_dead_ticks` (int), `minecraft:has_hunting_cooldown` (bool, `ttl`), `minecraft:temptation_cooldown_ticks` (int). Variants: 0 `lucy`, 1 `wild`, 2 `gold`, 3 `cyan` (common), 4 `blue` (rare); out of range reads 0.
- **Air:** max 6000. After `Animal.baseTick` (`#can_breathe_under_water`: it never drowns), unless NoAI, from the air before it: alive and not in water or rain → air − 1, and at −20 air 0 and 2 `minecraft:dry_out`; wet → 6000. `rehydrate`: + 1800, capped. A dry axolotl lasts 6020 ticks, then takes 2 a second. Rain counts at the feet or the box top.
- **`hurtServer`**, in this order: not NoAI, `nextInt(3) == 0`, (`nextInt(3) < amount` or health/max < 0.5), `amount < health`, in water, a source or direct entity, not playing dead → `PLAY_DEAD_TICKS` 200. Then `Animal.hurtServer`.
- **`customServerAiStep`:** brain, `updateActivity`, then `setPlayingDead(PLAY_DEAD_TICKS > 0)`. Playing dead: no ambient sound, `canBeSeenAsEnemy` false.
- **Brain** (sensors nearest living, nearest adult, hurt by, `AXOLOTL_ATTACKABLES`, food temptations):
  - CORE: `LookAtTargetSink(45, 90)`, `MoveToTargetSink()`, `ValidatePlayDead` (≤ 0: erase it and `HURT_BY_ENTITY`, default activity; else −1), `CountDownCooldownTicks(TEMPTATION_COOLDOWN_TICKS)`.
  - IDLE: 0 `SetEntityLookTargetSometimes(PLAYER, 6, 30-60)`; 1 `AnimalMakeLove(AXOLOTL, 0.2, 2)`; 2 `RunOne[FollowTemptation, BabyFollowAdult(5-16, NEAREST_VISIBLE_ADULT)]`; 3 `StartAttacking` (none while breeding, else `NEAREST_ATTACKABLE`); 3 `TryFindLiquid(6, 0.15, #axolotl_tries_to_find)`; 4 `GateBehavior({WALK_TARGET absent}, ORDERED, TRY_ALL)[RandomStroll.swim(0.5), RandomStroll.stroll(0.15, false), SetWalkTargetFromLookTarget(same medium, 3), triggerIf(isInWater), triggerIf(onGround)]` (weights unused under `ORDERED`).
  - FIGHT: `StopAttackingIfTargetInvalid(onStopAttacking)`, `SetWalkTargetFromAttackTargetIfTargetOutOfReach`, `MeleeAttack(20)`, `EraseMemoryIf(isBreeding, ATTACK_TARGET)`.
  - PLAY_DEAD (erases its ticks on exit): `PlayDead` (needs the ticks and `HURT_BY_ENTITY`, in water, 200 ticks at most; `start` erases walk and look targets and gives itself Regeneration 0 for 200), `EraseMemoryIf(isBreeding, PLAY_DEAD_TICKS)`.
  - `updateActivity`: unless in PLAY_DEAD, the first valid of PLAY_DEAD, FIGHT, IDLE; leaving FIGHT sets `HAS_HUNTING_COOLDOWN` for 2400 ticks.
  - Speeds: idle and tempt 0.5 in water, 0.15 out; chase and follow-adult 0.6 in water, 0.15 out.
  - Attackables: the first visible living within 8 blocks, in water, `#axolotl_always_hostiles` (drowned, guardian, elder guardian) or, without the hunting cooldown, `#axolotl_hunt_targets` (tropical fish, pufferfish, salmon, cod, squid, glow squid, tadpole), and attackable.
  - `onStopAttacking`: a dead target whose last damage came from a player inside the axolotl's box inflated by 20 → that player, if without Regeneration or with one ending within 2399 ticks, gets Regeneration 0 for `min(2400, 100 + remaining)`; Mining Fatigue is removed either way. No game-mode check.
- **`mobInteract`:** first `bucketMobPickup` (water bucket, alive: `item.bucket.fill_axolotl`, the bucket item, `dropLeash`, `discard`); else `Animal`: tropical fish bucket (`#axolotl_food`) at age 0 → love 600; a baby → `ageUp(getSpeedUpSecondsWhenFeeding(−age), true)`; `usePlayerItem` hands back a water bucket (creative: keeps the fish bucket, adds a water bucket if none). Then the golden dandelion lock.
- **Bucket:** the default bucket data (NoAI, Silent, NoGravity, Glowing, Invulnerable, Health, name) plus `Age`, `AgeLocked`, `HuntingCooldown` (ttl, when set) in `bucket_entity_data`, and the item component `minecraft:axolotl/variant`. Pouring (`item.bucket.empty_axolotl`) spawns with reason `BUCKET` (no finalize), applies the component, loads the data, sets `FromBucket`.
- **Breeding:** the child is blue on `nextInt(1200) == 0`, else a parent's variant by `nextBoolean`, and persistent; parents age 6000, 1-7 experience.
- **Sounds** (neutral): ambient `entity.axolotl.idle_water` or `idle_air`; `hurt`, `death`; `attack` on a landed bite; swim `entity.axolotl.swim`, splash `entity.axolotl.splash`; the block's steps.
- **Spawning:** `IN_WATER`, `MOTION_BLOCKING_NO_LEAVES`, block below in `#axolotls_spawnable_on` (clay), no light test, spawners alike. Only `lush_caves`: weight 10, count 4-6; cluster 4. `finalizeSpawn` (not for `BUCKET`): a new pack draws two common variants; members from the third on (group size ≥ 2) are babies; variant `types[nextInt(2)]`; `AgeableMobGroupData(false)`, so no baby roll.
- **Loot** `entities/axolotl` is empty. Tags: `#aquatic`, `#can_breathe_under_water`. Drowned target axolotls (priority 3); guardians too (M2 `guardians`).
- **Render:** `AgeableMobRenderer(AdultAxolotlModel, BabyAxolotlModel, shadow 0.5)`, textures `entity/axolotl/axolotl_<variant>` and `_baby`, `EntityModel`'s default `entityCutout` (culled), default death flip.
  - Four `BinaryAnimator(10, IN_OUT_SINE)`s: `tick(b)` keeps `ticksOld` and moves ±1 within 0..10; factor = ease(lerp(partial, old, ticks)/10). Adults tick dead, water, ground toward one state (dead, else in water, else on ground, else air) and moving from `walkAnimation.isMoving() || xRot or yRot changed`.
  - Babies run one of seven `AnimationState`s (`soloAnimation`, started at `tickCount`): play dead; moving: swim (water, not ground), walk (ground, not water), else walk under water; still: idle under water (water, not ground), idle under water on ground, else idle on ground.
  - `AdultAxolotlModel` 64 x 64: `body` (8x4x10 and a 0x5x9 fin) at (0, 19.5, 5); `head` 8x5x5 at (0, 0, −9) with `top_gills` 8x3x0, `left_gills` and `right_gills` 3x7x0; four 3x5x0 legs at (±3.5, 1, −1) and (±3.5, 1, −8); `tail` 0x5x12 at (0, 0, 1). `setupAnim`: `body.yRot += yRot`, then swimming (`min(moving, water)`, uses `xRot`), water hovering (`min(1 − moving, water)`), ground crawling (`min(moving, ground)`), lying still (`min(1 − moving, ground)`), play dead, mirrored legs (`1 − min(ground, moving)`), each skipped under 1e−5 and lerped from the current pose (`SWIMMING_LEG_XROT` 1.8849558).
  - `BabyAxolotlModel` 32 x 32: `root` (0, 24, 0) › `body` (4x2x6 and a 0x3x5 fin) › four 3x0x1 legs (the right hind one under a rotated `right_hind_leg` with `right_leg_r1`), `tail` 0x3x8, `head` 6x3x4 with three gills. `setupAnim`: `AXOLOTL_WALK_FLOOR` by `applyWalk(pos, speed, 15, 30)` while the walk state runs, then `BABY_AXOLOTL_SWIM`, `WALK_FLOOR_UNDERWATER` (**on the walk state, not walk-under-water: a vanilla quirk**), `BABY_AXOLOTL_IDLE_FLOOR`, `IDLE_UNDERWATER`, `IDLE_FLOOR_UNDERWATER`, `BABY_AXOLOTL_PLAY_DEAD`. No head look.
  - `BabyAxolotlAnimation` lengths: 2.88, 5.12, 5.2, 1.0, 2.72, 2.04 s looping; play dead 0.04 s once; `baby_axolotl_dash` unused.
- **Entity events:** none of its own; hearts (18) and age-lock events are E1's `puff_for` rows.

### Plan

**Files (D0-3):**
- `third_party/minecraftoss/entities/src/world/families/water/axolotls.rs`: `pub struct Axolotl { variant, from_bucket, age, in_love, brain, controls, navigation, animators: [Animator; 4], baby_anim: Option<(u8, i32)>, walk_speed, supports: Vec<(u64, DVec3)> }`, `impl Family`, tick, air, breeding, bucket data, tests; the brain in `water/axolotls/brain.rs`.
- `crates/minecraft_terrain/src/families/water/axolotls.rs` (`TerrainFamily`, `interact`, `axolotl_support`, `axolotl_supports`) and `families/water/axolotls/render.rs`.
- The row in `third_party/minecraftoss/world/src/natural_spawner/families/water.rs`.

**Registration lines:** `pub mod axolotls;` and `axolotls: axolotls::Axolotl,` in `family_group!(water 64..=79 { .. })`; `mod axolotls;` and `Axolotl` in `terrain_group!(water { .. })`, plus a `pub(crate) use` of the two support functions; the row in `ROWS`.

**Hooks (D0-2.2):** `KEY` 67; `KINDS` `["minecraft:axolotl"]`; `spawn` `MobCore::new(kind, pos, 0.75, 0.42, 0.2751, 14.0)`, `ambient_interval` 120, air 6000; `category` `"axolotls"`; `experience` 1-3 for adults; `hurt`: the play-dead roll in vanilla order (a "source" is `hurt.attacker` or a projectile, player, mob or entity-blast source), `hurt_plain`, `in_love = 0` when applied; `despawn` `of_category("axolotls")` with `removable: !from_bucket`, `kept: from_bucket`; `breathes_water` and `leashable` true; `leash_walk` amphibious navigation to the holder at 1.0 (none while playing dead); `voice` and `loot` default. Merged hooks it sets: M2-1's `breath` `Own` (the dry-out rule is the family's) and `swim_sounds` (`entity.axolotl.swim`, `entity.axolotl.splash`), and squids' `OWN_DYING_TRAVEL` (a dead axolotl drifts).

**Tick**, vanilla order: keep `air`; `family_base_tick`; the dry-out rule; turn the base tick's `Voice::Ambient` into `Voice::Event("entity.axolotl.idle_water" | "idle_air", 1.0, pitch)`, or drop it while playing dead (the timer reset stays); alive with AI: sensors every 20 ticks, brain, `updateActivity`, the flag, controls unless playing dead (M2-1's `SmoothSwimmingMoveControl(85, 10, 0.1, 0.5, false)` and `SmoothSwimmingLookControl(20)`, `max_up_step` 1.0), M2-1's `tick_water` with `WaterNav::Amphibious { shallow: false }`, travel (`travel_swimming` at its speed in water), `family_after_move`; `age.tick` (resize at the boundary); love; then the animators and baby state for `show` (moving: a `WalkAnimationState` speed, `update(min(4·Δxz, 1), 0.4)`, above 1e−5, or yaw or pitch changed).

**One-line hooks in shared files:**
- `crates/minecraft_terrain/src/potions.rs`: `AxolotlSupport` after `Effect(PotionEffect),` (`:380`); its arm after the `Effect` arm of `apply_to_player` (`:425-434`) calls `axolotl_support(status)` (`SurvivalStatus::effect` `player/src/survival.rs:255`, `add_effect` `:277`, `remove_effect` `:292`). `take_potion`'s `other =>` (`crates/minecraft_survival/src/survival.rs:786`) passes it on.
- `crates/minecraft_terrain/src/server.rs`, after the witch splash line (`:4278`): `out.player_potions.extend(crate::families::water::axolotl_supports(&mut sim.mobs));`.
- `crates/minecraft_terrain/src/splash.rs` `wet` (`:28`), after the targets loop (`:53`), before `players` (`:54`): `world.rehydrate_axolotls(area.0, area.1);` (an `impl EntityWorld` in the family file; every axolotl in the box). E2 edits inside the loop; stay outside it.
- Root-file hook (D0-12, own commit): `Family::seen_as_enemy` (default true) in `MobTraits`, and D0's `mob_candidates` arm passes `alive && seen_as_enemy`, so targeting skips and drops a playing-dead axolotl.

**Multiplayer:** `TRACKING_RANGE` 10, `SHADOW_RADIUS` 0.5, `MAX_HEAD_Y_ROT` 1.0 (`client_mobs.rs:705-716`), 10 `TEXTURES`; pitch already travels as `ServerMob.x_rot` (`:1203`).
- `VOLATILE` `["ForcedAge", "AgeLocked"]` (`Age`, `InLove`, `Air`, `Brain` are base keys, `mob_tracking.rs:15-43`); `Variant` and `FromBucket` never change, so the look tag gives the friend its colour.
- `read`/`fields`: `Variant`, `FromBucket`, `Age`, `ForcedAge`, `AgeLocked`, `InLove`, `Air`, the three memories.
- `show`: flag bit 0 playing dead; `values[0]` the animators' ticks (4 bits each: dead, water, ground, moving), `values[1]` their `ticksOld`, `values[2]` the baby state (0 none, 1-7 in the order above), `values[3]` ticks since it started. `show_on` stores them; `draw` eases with the partial tick and starts the baby keyframes at `AnimationState::started(age − ticks_since)` against `MobPose.age_in_ticks`.
- `interact`: food, dandelion, bucket. No `client_effects`. **Protocol:** none (`PlayerPotion` stays on the host).

**Keyframes:** `keyframes::shared().definition("BabyAxolotlAnimation", FIELD)` (`keyframes.rs:792`), `apply_state` (`:315`), `apply_walk` (`:328`).

**Commit groups**

| Group | Commits | Contents |
|---|---|---|
| `axolotls-live` (m2f, after M1's `animal_brain.rs`) | 1. "Minecraft axolotls live, swim and play dead in the entity world" (on M1's `animal_brain.rs`). 2. "Minecraft axolotls spawn in lush caves in groups with their babies". | Family, brain (CORE, IDLE, PLAY_DEAD), air, breeding, tempting by `holds`. Row: `0.75, 0.42, InWater, MotionBlockingNoLeaves`, `Rules::Family { creature: true }`, `Walk::Neutral`, `remove_far`, cluster 4, `Axolotls`, follow 16, `baby_chance: None`, a clay rule, a finalize packing two variants and the size into the group `i32` (`a | b << 3 | size << 8`). |
| `axolotls-drawn` (m2f) | 1. "Minecraft axolotls are saved, hatch from eggs and are drawn like vanilla". 2. "Minecraft axolotls eat tropical fish from buckets and hand back the water". | `TerrainFamily`, both models in `out.culled`, `FamilyShow`; row `drawn`. Then `interact` on `animal::interact` with the water bucket swap and the dandelion. |
| `axolotls-hunt` (m2f) | 1. "Registered Minecraft mobs can hide from the mobs that hunt them". 2. "Minecraft axolotls hunt fish and squid, fight drowned and help the player who finishes their prey". | The hook; FIGHT, the sensor, bites through `family_strike` (2), the cooldown, the support (the prey's killer is its `core.hurt_by` player at death). |
| `axolotls-buckets` (m2f) | 1. "Minecraft axolotls go into water buckets and keep their colour". 2. "Splash water bottles rehydrate Minecraft axolotls". | M2-1's bucket path (dispensers included, through its arm) with the axolotl's data and component, `drop_leash`; the `splash.rs` line. Row `done`; steps and kit in the log. |

### Tests

- **live 1:** `an_axolotl_is_0_75_by_0_42_with_fourteen_health_and_its_kind` (baby size, eyes, step, census, `mob_candidates`, empty table, experience); `an_axolotl_out_of_water_dries_out_after_6020_ticks_and_rain_keeps_it_wet`; `a_hurt_axolotl_in_water_plays_dead_for_200_ticks_with_regeneration` (draw order; not on land, without a source, on a killing hit or twice); `a_playing_dead_axolotl_holds_still_and_is_silent`; `an_axolotl_swims_strolls_slowly_on_land_and_looks_for_water`; `an_axolotl_follows_a_player_holding_a_tropical_fish_bucket`; `two_axolotls_in_love_breed_a_baby_with_a_parent_colour_or_rarely_blue`; `a_baby_axolotl_follows_an_adult_and_grows_up`; `an_axolotl_calls_idle_water_in_water_and_idle_air_on_land`; `a_dying_axolotl_drifts_in_water`.
- **live 2:** `natural_spawner::families::water::tests::axolotls_spawn_in_water_over_clay_only`; `an_axolotl_pack_has_two_common_colours_and_babies_from_its_third_member`; `summoning_an_axolotl_notes_nothing_unsupported`.
- **drawn 1:** `an_axolotl_saves_and_loads_with_its_variant_age_air_and_brain_memories`; `a_friend_sees_an_axolotl_from_its_look_tag`; `a_friend_sees_an_axolotl_play_dead_at_the_same_tick`; `render::tests::an_adult_axolotl_has_eleven_cubes_on_64_by_64_and_a_baby_eleven_on_32_by_32`; `every_axolotl_picks_its_texture_by_variant_and_age`; `the_adult_pose_blends_by_the_four_animators`; `binary_animators_ease_over_ten_ticks`; `a_baby_axolotl_plays_one_keyframe_animation_at_a_time` (gated on `MINECRAFTOSS_ROOT`).
- **drawn 2:** `feeding_a_tropical_fish_bucket_puts_an_axolotl_in_love_and_hands_back_a_water_bucket` (survival and creative); `feeding_a_baby_axolotl_speeds_its_growth`; `a_golden_dandelion_locks_a_baby_axolotl_s_age`.
- **hunt:** `a_registered_mob_that_hides_is_no_target`; `an_axolotl_hunts_fish_and_squid_in_water_within_eight_blocks`; `an_axolotl_fights_drowned_even_on_hunting_cooldown`; `an_axolotl_bites_for_two_every_twenty_ticks`; `after_a_hunt_an_axolotl_waits_two_minutes`; `a_player_who_finishes_an_axolotl_s_prey_gets_regeneration`; `potions::tests::axolotl_support_adds_up_to_two_minutes_and_clears_mining_fatigue`.
- **buckets:** `a_water_bucket_scoops_an_axolotl_with_its_colour_age_and_hunting_cooldown`; `a_poured_axolotl_keeps_its_colour_and_never_despawns`; `a_dying_axolotl_cannot_be_scooped`; `splash::tests::a_water_bottle_rehydrates_every_axolotl_in_its_box`.

### Owner's steps (survival, Normal, by day, at a river or lake at least 2 deep; the kit below)

1. (Optional) `/locate biome minecraft:lush_caves`, `/tp` there, find a pool over clay and wait a minute: groups of axolotls appear, two adults and smaller babies, pink, brown, gold or pale cyan. **ax1_lush_caves**
2. Use the axolotl egg three times in the river: they swim with legs folded back and tail sweeping, tilting as they climb and dive, and hover with legs spread and gills waving. **ax2_swim**
3. Use an egg on dry ground 3 blocks from the water: it crawls slowly, lies still with its legs out, and heads back into the water. `/summon minecraft:axolotl ~ ~ ~3 {Variant:4}` gives a blue one.
4. Punch an axolotl in the water once a second: after a few punches it rolls onto its back, floats still and silent for up to 10 s while healing, then swims again. On land it never does. **ax4_play_dead**
5. Hold a tropical fish bucket at the edge: they swim after it. Use it on two adults: your hand holds a water bucket again, they meet and a baby appears with orbs. **ax5_breed**
6. The baby is small and big-headed with its own paddling. Feed it a tropical fish bucket: it grows faster. `/summon minecraft:axolotl ~ ~ ~2 {Age:-24000}`, use the golden dandelion on it: the lock sound. **ax6_baby**
7. Put a lead on one and walk along the shore and back: it follows on land and in water.
8. Far from water, `/summon minecraft:axolotl ~ ~ ~3 {Air:200s}`: after about 11 s it flashes red each second. Throw a splash water bottle on it: it stops. `/weather rain`: another one summoned like that never dries. `/weather clear`.
9. Two axolotl eggs and a squid egg in the water: they chase and bite the squid until it dies, then leave new squids alone for 2 minutes; a drowned from its egg is attacked at once. **ax9_hunt**
10. Let an axolotl bite a squid twice, then kill the squid with the sword: you get Regeneration for 5 s.
11. Use a water bucket on an axolotl: it goes in with the fill sound; pour it into other water: same colour, and it never despawns. The kit's axolotl bucket pours a pink one. **ax11_bucket**
12. Leave an adult, the locked baby, a poured one and one playing dead; save, quit, load: all are there, the baby still small.
13. `/gamemode creative`, place the spawner, use an axolotl egg on it: a small axolotl in the cage. `/gamemode survival`.
14. Two windows (`scripts\mp-local.ps1`): the host hatches three. The friend sees the same colours and poses, and a punched one rolls over at the same moment in both; the friend breeds two and scoops one, and the host sees it. **ax14-host**, **ax14-friend**

### Kit

```
IW4L_MINECRAFT_KIT=minecraft:axolotl_spawn_egg*16,minecraft:axolotl_bucket,minecraft:water_bucket,minecraft:water_bucket,minecraft:water_bucket,minecraft:tropical_fish_bucket,minecraft:tropical_fish_bucket,minecraft:tropical_fish_bucket,minecraft:tropical_fish_bucket,minecraft:bucket*16,minecraft:clay*64,minecraft:glass*64,minecraft:squid_spawn_egg*8,minecraft:drowned_spawn_egg*4,minecraft:cod_spawn_egg*4,minecraft:golden_dandelion*4,minecraft:lead*4,minecraft:splash_potion@water,minecraft:splash_potion@water,minecraft:iron_sword,minecraft:bow,minecraft:arrow*64,minecraft:spawner,minecraft:cooked_beef*32,minecraft:diamond_pickaxe,minecraft:torch*64
```

26 stacks; every id checked against the 26.3 `assets/minecraft/items`. The friend needs the same line.

### Depends on

- **D0 d0a:** `registry-core`, `registry-hooks`, `registry-spawning` (`Group::Family` data), `registry-client` (`FamilyShow`), `registry-draw` (`draw`, `interact`, `keyframes::shared()`). Not d0b.
- **M2-1:** smooth swimming controls, `AmphibiousPathNavigation` with `AmphibiousNodeEvaluator` (the drowned's `MeasuredWaterFloorTerrain`, `path_search.rs:50`, only walks the floor), `RandomStroll.swim`, `TryFindLiquid`, `Breath::Own`, `swim_sounds`, `Bucketable` and mob buckets. **M2 `squids`:** the `InWater` obstruction line, `OWN_DYING_TRAVEL`.
- **M1 goats:** `animal_brain.rs` (M20 owner; segment m2f waits for it). Its tempting sensor must take a held-food test on `controls.holds`, as M1's armadillos also ask.
- **M2 prey and enemies:** `squids`, `fish`, `frogs-tadpoles` (tadpoles land after this section; the tag lists them, so they become prey when they land); `guardians`, which target axolotls with their own target goal and so need the hide hook.
- **E1:** hearts (`particles-spit-love`, `puff_for` 18). **E2:** `hit_boxes`; its water-bottle commit in `splash.rs` `wet`. **W:** nothing.

### Open decisions (recommended choice)

1. **Animation sync:** the host ticks the animators and the baby state and sends them in `FamilyShow`; a friend's registry has no per-family client tick to derive them from synced flags as vanilla does.
2. **Support effect:** a `PlayerPotion::AxolotlSupport`, not W7's `EffectGrant`: vanilla adds 100 ticks to what remains (up to 2400), in every game mode, and clears Mining Fatigue.
3. **Hiding:** `seen_as_enemy` folded into `mob_candidates`' `alive`, as `canBeSeenAsEnemy` works.
4. **Baby walk quirk:** port it as vanilla has it.
5. **Brain:** build on M1's `animal_brain.rs` (M20 owner), not a second copy.
6. **Tempting:** `player_controls(id).holds(..)`, not two more `PlayerCandidate` flags (about ten construction sites).

---

## M2-8. Frogs, tadpoles and frogspawn (groups `frogs-live`, `frogs-drawn`, `tadpoles-live`, `tadpoles-drawn`, `frogspawn`, `frogs-leap-eat`, `frogs-froglights`)

There are two families in group `water`:
- `frogs`: byte **68**, `minecraft:frog`, row 33 of `docs/round10/mobs.md`;
- `tadpoles`: byte **69**, `minecraft:tadpole`, row 34.

Frogspawn is a block tick. Froglights come from the magma cube's loot table.

Lines are `minecraft-survival` at `d1a3d028`; check each one again before editing. Where D0's merged code differs from `track-d0.md`, follow the code. D0's first commit (`9eecb2ff` on `track-d0`) already names `Hurt::new(..).by(..)`, `Despawn::of_category` and `family_group!(water 64..=79 {})`.

### What exists today

- **Frogs are dormant.**
  - Size row: `world/src/natural_spawner.rs:286` (0.5 × 0.5, `OnGround`).
  - Rules: `_ => Rules::Other` (`:354`). Every natural pick logs `note_unsupported` and fails (`natural_spawner/spawning.rs:766-769`).
  - Category: `Creature` (`spawning.rs:103`).
  - Tadpoles appear nowhere.
- **Frogspawn is inert.**
  - It is placed on water (`player/src/lib.rs:2025`) and survives or breaks with its water (`generator/src/feature/survive.rs:578-581`, `update.rs:242`).
  - `on_place` (`world/src/level/mod.rs:677`) and `tick_block` (`:839`) have no arm for it, so it never hatches.
  - A falling block inside it is logged as unsupported (`level/entity.rs:1266`).
  - `tadpole_bucket` is an unsupported dispenser behaviour (`level/dispenser.rs:25`; M2-1 handles it).
- **Bug found while reading: magma cubes drop nothing.**
  - `damage_source_properties` accepts only `minecraft:entity_type` (`entities/src/loot.rs:636-645`).
  - The three froglight entries test `minecraft:components` → `minecraft:frog/variant`, so their condition is `None`.
  - `expand` (`:455`) passes that up with `?`, and `roll_table` (`:396`) drops the whole roll. Even a player's kill gives no magma cream.
  - The slime table only tests the killer's type, and `mob_hits_mob` (`world.rs:4179`) already credits `entity_type(attacker)`.
- **Code to reuse:**

  | Need | Where |
  |---|---|
  | Brain helpers | `brain_behaviors.rs` (`MoveSink` `:183`, `within_manhattan` `:339`) |
  | Breeding orb | hoglins: `world/hoglins.rs:628-650`, `trade_experience` |
  | SINGLE conversion | hoglin → zoglin, `world/hoglins.rs:600-625` |
  | Brain memories format | `piglin_save.rs:71-146` |
  | Age and slime-ball love | `age::Age`, `animal::interact` (`animal.rs:12`) |
  | Dandelion sound | `mob_actions.rs:122-127` |
  | Mob tag from a block | `level/golem_build.rs:215-229` → `Level.spawned` (`mod.rs:153`) → `server.rs:1016` |
  | Block removal | `destroy_block_drops` (`level/drops.rs:164`) |
  | Slime size | `slime.rs:15`, via `self.slimes` from a family file |
  | Other world helpers | `mob_candidates` (`world.rs:4079`), `level_random` (`:2581`), `entity_events` (`:2623`) |
  | Variants by biome | `VariantRegistry` (`natural_spawner.rs:127`, `:157`, pig/cow/chicken `:402-404`, `:540-542`) |
  | Eggs on water | `spawn_egg.rs:97-109` |
  | Particles and keyframes | `Kind::HappyVillager` (`ambient_particles.rs:51`); `FrogAnimation` is readable (`keyframes.rs:1359` gated test) |
  | Falls | `entities/src/world/living.rs:54-104` |

- **Already global `VOLATILE` keys** (`mob_tracking.rs:15-53`): `Age`, `InLove`, `Brain` and `Air`.

### Vanilla 26.3 (javap this round)

**Basics.** Neither kind is `notInPeaceful` or `fireImmune`.

| | Frog (`Animal`, brain only) | Tadpole (`AbstractFish`) |
|---|---|---|
| Category, size, eye | `CREATURE`, 0.5 × 0.5, eye 0.425 | `CREATURE`, 0.4 × 0.3, eye 0.195 |
| Tracking range | 10 | 10 |
| Attributes | health 10, speed 1.0, attack 10, step 1.0, follow 16, tempt 10 | health 6, speed 1.0 |
| Experience, despawn | 1 + `nextInt(3)`; never despawns | none (`shouldDropExperience` false); never (`fromBucket()` is always true) |
| Loot | empty | empty |
| Sounds | `entity.frog.{ambient,hurt,death}`, interval 120; `step` at 0.15; `eat` 2.0 `NEUTRAL` | hurt, death, `flop`; no ambient; swim `entity.fish.swim`; no step |
| Air | `#can_breathe_under_water` | gills (M2-1 `Breath::Gills`): stranded, hits at ticks 320, 340 and 360 |

**`Frog`:**
- **Controls:**
  - `FrogLookControl`: it resets the pitch only without a tongue target.
  - Maluses `WATER` 4 (M2-1's amphibious `prepare` zeroes it) and `TRAPDOOR` −1.
  - `SmoothSwimmingMoveControl(85, 10, 0.02, 0.1, true)`.
  - `getMaxHeadYRot` 5 and `getHeadRotSpeed` 35.
- **Navigation:** `FrogPathNavigation` with `FrogNodeEvaluator(prefersShallow = true)`.
  - In water the start is `floor(minX, minY, minZ)`, not the swim start's `+0.5`.
  - `OPEN` over `#frog_prefer_jump_to` (lily pad, big dripleaf).
  - No corner cutting at `WATER_BORDER`.
- **Travel:** `travelInWater` = `moveRelative(getSpeed())`, move, × 0.9 (M2-1 `acceleration: Speed`). Not pushed by fluid. `calculateFallDamage` less 5.
- **Breeding:**
  - `canBeABaby` is false, so `Age`, `ForcedAge` and `AgeLocked` are not saved or read.
  - Food: `#frog_food` (slime ball). Feeding: love 600, eat sound, event 18.
  - `spawnChildFromBreeding` spawns no child: parents to 6000, love reset, event 18, an orb of `nextInt(7) + 1`, then `IS_PREGNANT`.
- **Save:**
  - `variant` (default `minecraft:temperate`).
  - Memories: `long_jump_cooling_down` (int), `temptation_cooldown_ticks` (int), `is_in_water` and `is_pregnant` (unit), `is_panicking` (bool).
- **`finalizeSpawn`:** pick the variant (level random); `long_jump_cooling_down = UniformInt(100, 140)`; `Animal.finalizeSpawn`. The baby roll is drawn but ignored.
- **Variants:**
  - `temperate`: priority 0;
  - `warm`: priority 1, `#spawns_warm_variant_frogs` (mangrove swamp, desert, warm ocean, `#is_jungle`, `#is_savanna`, `#is_nether`, `#is_badlands`);
  - `cold`: priority 1, `#spawns_cold_variant_frogs` (snowy and frozen biomes, grove, deep dark, `#is_end`).
  - Textures: `entity/frog/frog_{temperate,warm,cold}`.
- **Client:**
  - The poses `LONG_JUMPING`, `CROAKING` and `USING_TONGUE` start the jump, croak and tongue animations.
  - Swim idle runs when `isInWater && !walk.isMoving()`.
  - `updateWalkAnimation`: target 0 mid-jump, otherwise `min(distance × 25, 1)` (not 4).

**`FrogAi`:**
- **Sensors:** nearest living, hurt-by, `FROG_ATTACKABLES`, `FROG_TEMPTATIONS` (slime ball), `IS_IN_WATER`.
- **Activity:** the first valid of `[TONGUE, LAY_SPAWN, LONG_JUMP, SWIM, IDLE]`.

| Activity | Needs | Behaviours |
|---|---|---|
| CORE | | `AnimalPanic(2.0)`, `LookAtTargetSink(45, 90)`, `MoveToTargetSink`, countdowns for temptation and the long jump |
| IDLE | not mid-jump, not in water | look at a player sometimes (6, 30-60); `AnimalMakeLove`; `FollowTemptation(1.25)`; `StartAttacking(!breeding, NEAREST_ATTACKABLE)`; `TryFindLand(6, 1.0)`; `RunOne` of stroll 1.0 (w1), walk to look target (1.0, 3, w1), `Croak` (w3), idle (w2) |
| SWIM | in water | look; temptation; attack; `TryFindLand(8, 1.5)`; ordered gate of `swim(0.75)`, `stroll(1.0, true)`, walk to look target |
| LAY_SPAWN | pregnant | look; attack; `TryFindLandNearLiquid(8, 1.0, #water)`; `TryLaySpawnOnFluidNearLand`; `RunOne` of stroll (w2), look-walk (w1), `Croak` (w2), idle (w1) |
| LONG_JUMP | no tempter, breed target, cooldown or water | `LongJumpMidJump(100-140, entity.frog.step)`; `LongJumpToPreferredBlock(100-140, 2 high, 4 wide, 3.5714288, entity.frog.long_jump, #frog_prefer_jump_to, 0.5, landing rule)` |
| TONGUE | an attack target | `StopAttackingIfTargetInvalid`, `ShootTongue` |

- **Preferred jump:**
  - `start` sets `wantPreferred = nextFloat() < 0.5`.
  - It pops candidates until one stands over a preferred block, falling back to the first refused one.
- **Landing rule:**
  - false if the spot, the block below or the block above holds fluid;
  - true over a preferred block, or on a `TRAPDOOR` path type at the spot or below an air spot (vanilla's quirk);
  - otherwise the default: solid-render below and malus 0.
- **`Croak`** (timeout 100): needs `STANDING`. In liquid `start` returns without resetting its counter; otherwise it sets `CROAKING` for 60 ticks, then `STANDING`.
- **Attackables sensor:** a visible, attackable mob within 10 that `canEat` (a cube mob of size 1 in `#frog_food`: slime, magma cube) and is not in `UNREACHABLE_TONGUE_TARGETS`.
- **`ShootTongue`** (timeout 100; needs no walk target, not panicking, not croaking):
  - **Start condition:** a path with `distToTarget < 1.75`. Otherwise erase the target and remember its UUID (up to 5, expiring after 100 ticks).
  - **`start`:** look at it, tongue target, walk target `(2.0, 0)`, `MOVE_TO_TARGET`.
  - **Within 1.75:** tongue sound (2.0), `USING_TONGUE`, the prey's motion set to `normalize(frog − prey) × 0.75`, `CATCH`.
  - **After 6 ticks:** `EAT` with the eat sound (2.0), `doHurtTarget` (10), then `remove(KILLED)` if it died. `DONE` at timer 10.
  - **`stop`:** clear the targets and set `STANDING`.
  - The walk target is renewed every 10 ticks.
- **Finding land:**
  - `TryFindLand`: in water, on a 60-tick clock. The first Manhattan-ordered block that differs horizontally, holds no fluid, has an empty shape and a sturdy top below. It sets the look and walk target `(speed, 1)`.
  - `TryFindLandNearLiquid`: out of the fluid, on a 40-tick clock. An empty spot on a collidable block with an air neighbour (N, E, S, W) over water. Walk target `(speed, 0)`.
- **`TryLaySpawnOnFluidNearLand`** (no attack target, a walk target, pregnant, on the ground and out of water): for N, E, S, W around the block under the feet, a neighbour with an empty top face, water fluid and air above gets frogspawn above it, then `block_place`, `entity.frog.lay_spawn` (`BLOCKS`) and pregnancy erased.

**`Tadpole`:**
- **Controls:** `SmoothSwimmingMoveControl(85, 10, 0.02, 0.1, true)`, `SmoothSwimmingLookControl(10)`, water-bound navigation.
- **Goals and brain:** it keeps `AbstractFish.registerGoals` (`PanicGoal(1.25)`, `AvoidEntityGoal(Player, 8, 1.6, 1.4)`, `FishSwimGoal`) **besides** its brain.
  - CORE: `AnimalPanic(2.0)`, look sink, move sink, temptation countdown.
  - IDLE: look at a player sometimes; `FollowTemptation(1.25)`; a gate of `swim(0.5)` and walk to look target (0.5, 3).
- **Growth:**
  - `aiStep` adds 1 to `age` unless locked. At 24000, `convertTo(FROG, single)`.
  - Then `finalizeSpawn(CONVERSION)`, persistence, `fudgePositionAfterSizeChange` and `entity.tadpole.grow_up` at 0.15.
- **`mobInteract`, in order:**
  1. Slime ball while unlocked: consume one and add `(int)(left / 20 × 0.1) × 20` ticks. One `HAPPY_VILLAGER` appears on the feeder's client.
  2. Golden dandelion (lock timer 0): toggle the lock, **set the age to 0**, timer 40, consume one, persistence when locked, `item.golden_dandelion.use`/`unuse` (`PLAYERS`).
  3. Otherwise `bucketMobPickup`. The bucket data adds `Age` and `AgeLocked`; the sound is `item.bucket.fill_tadpole`.
- **Save:** `Age`, `AgeLocked`, `FromBucket`, memories.

**`FrogspawnBlock`:**
- `canSurvive`: water (`#supports_frogspawn`) below and no fluid at its own spot.
- `onPlace`: tick in `nextInt(3600, 12000)`.
- `tick`:
  - If it cannot survive, `destroyBlock(pos, false)`.
  - Otherwise destroy it the same way, play `block.frogspawn.hatch` (`BLOCKS`), and add `nextInt(2, 6)` persistent tadpoles. They are created with `BREEDING`, with no finalize, at `(x + clamp(nextDouble, 0.2, 0.8), y − 0.5, z + clamp(..))` and yaw `nextInt(1, 361)`.

**Loot:**
- A magma cube drops exactly one entry:
  - magma cream (−2 to 1, looting) when not killed by a frog and size ≥ 2;
  - otherwise by the frog's `minecraft:frog/variant`: `warm` → `pearlescent_froglight`, `cold` → `verdant_froglight`, `temperate` → `ochre_froglight`.
- A size-1 slime drops slime balls 0-2, or exactly 1 when a frog killed it.

**Spawning:**
- Frog: `ON_GROUND`, `MOTION_BLOCKING_NO_LEAVES`, `checkFrogSpawnRules`: below in `#frogs_spawnable_on` (grass, mud, mangrove roots, muddy mangrove roots) and raw brightness > 8.
- Biomes: `swamp` and `mangrove_swamp`, weight 10, groups of 2-5, cluster 4.
- Tadpole: no placement entry and no biome.

**Render.** Both models use `entityCutout`, which culls back faces (`out.culled`).

`FrogModel` is 48 × 48. Each part lists its pose offset, then its cubes as uv, from, size:

| Part | Parent | Offset | Cubes |
|---|---|---|---|
| root | | (0, 24, 0) | none |
| body | root | (0, −2, 4) | (3,1) (−3.5,−2,−8) (7,3,9); (23,22) (−3.5,−1,−8) (7,0,9) |
| head | body | (0, −2, −1) | (23,13) (−3.5,−1,−7) (7,0,9); (0,13) (−3.5,−2,−7) (7,3,9) |
| eyes | head | (−0.5, 0, 2) | none |
| right_eye | eyes | (−1.5, −3, −6.5) | (0,0) (−1.5,−1,−1.5) (3,2,3) |
| left_eye | eyes | (2.5, −3, −6.5) | (0,5), same box |
| croaking_body | body | (0, −1, −5) | (26,5) (−3.5,−0.1,−2.9) (7,2,3), deformation −0.1 |
| tongue | body | (0, −1.01, 1) | (17,13) (−2,0,−7.1) (4,0,7) |
| left_arm | body | (4, −1, −6.5) | (0,32) (−1,0,−1) (2,3,3) |
| left_hand | left_arm | (0, 3, −1) | (18,40) (−4,0.01,−4) (8,0,8) |
| right_arm | body | (−4, −1, −6.5) | (0,38), same box as left_arm |
| right_hand | right_arm | (0, 3, 0) | (2,40) (−4,0.01,−5) (8,0,8) |
| left_leg | root | (3.5, −3, 4) | (14,25) (−1,0,−2) (3,3,4) |
| left_foot | left_leg | (2, 3, 0) | (2,32) (−4,0.01,−4) (8,0,8) |
| right_leg | root | (−3.5, −3, 4) | (0,25) (−2,0,−2) (3,3,4) |
| right_foot | right_leg | (−2, 3, 0) | (18,32) (−4,0.01,−4) (8,0,8) |

- That is 16 parts and 16 cubes. Flat boxes keep all six faces.
- `setupAnim`: jump, croak and tongue from their states; `FROG_SWIM` `applyWalk(.., 1.0, 2.5)` while swimming, otherwise `FROG_WALK` `(1.5, 2.5)`; `FROG_IDLE_WATER`; `croaking_body` visible only while croaking. No head look.
- Animation lengths: croak 3.0 s (scale on `croaking_body`), walk 1.25 s (looping), jump 0.5 s, tongue 0.5 s (head, tongue rotation and scale), swim 1.04167 s (looping), idle in water 3.0 s (looping).
- Renderer: shadow 0.3, texture by variant.

`TadpoleModel` (16 × 16):
- body (0,22,−3): (0,0) (−1.5,−1,0) (3,2,3);
- tail (0,22,0): (0,0) (0,−1,0) (0,2,7);
- `tail.yRot = −(inWater ? 1 : 1.5) × 0.25 × sin(0.3 × age)`;
- shadow 0.14, texture `entity/tadpole/tadpole`.

**javap during the work:**
- `ConversionType.SINGLE.convert`;
- `fudgePositionAfterSizeChange`;
- sensor scan rates;
- `StartAttacking`, `StopAttackingIfTargetInvalid`, `SetEntityLookTargetSometimes`, `SetWalkTargetFromLookTarget` and `RandomStroll.stroll(speed, mayStrollFromWater)`, if no earlier group ported them.

### Plan

**Files (D0-3):**
- Entities `world/families/water/frogs.rs`: `pub struct Frog { variant, brain: FrogBrain, pose, pose_start, in_love, love_cause, age, .. }`, `impl Family` (`KEY` 68), the tick and tests.
- `water/frogs/brain.rs`: the activities, `Croak`, `ShootTongue`, the land finders, laying, the sensor and the preferred jump.
- `water/tadpoles.rs`: `pub struct Tadpole { age, age_locked, lock_timer, fed, ai: MonsterAi, brain }`, `impl Family` (`KEY` 69).
- Terrain: `families/water/{frogs,tadpoles}.rs` with `render.rs` beside each.
- World: `level/frogspawn.rs`.
- Two `ROWS`.
- Registration: `pub mod frogs; pub mod tadpoles;`, plus `frogs: frogs::Frog,` and `tadpoles: tadpoles::Tadpole,` in `family_group!(water ..)`; terrain `mod` lines and `terrain_group!`; `mobs.md` rows 33-34.

**Hooks (D0-2.2):**
- Both:
  - `category` `"creature"`;
  - `voice` default;
  - `spawn`: the frog's `core.baby` is always false. It ignores an `Age: -24000` that `ageable` may write, and `fields` writes no `Age`.
- Frog:
  - `breathes_water` true;
  - `leashable` true, and `leash_walk` moves to the target with its navigation;
  - `panicking` from `is_panicking`;
  - `step_sound` `entity.frog.step`.
- Tadpole:
  - `experience` 0;
  - `despawn { removable: false, kept: true }`;
  - M2-1's `breath` `Gills`, `quiet_steps`, and `swim_sounds` `entity.fish.swim`;
  - no ambient: the tick drops the `Voice::Ambient` that `family_base_tick` pushes (M20: no ambient hook exists this round).

**The frog tick:**
1. `family_base_tick`.
2. Sensors.
3. The brain.
4. M2-1's amphibious navigation (`WaterNav::Amphibious { shallow: true }`) with the three frog knobs.
5. The smooth swim control.
6. The look. Use M1 foxes' `LookControl::tick_from(.., reset_pitch = no tongue target)` if it landed, else `tick_limited`; the model draws no pitch.
7. Travel: `travel_swimming` at its speed in water; mid-jump (from `frogs-leap-eat` on) M1 goats' friction-free travel from `long_jump.rs`.
8. The fall, 5 less: the frog's own rule, in the shape of goats' 10 less (M1 `goats-leap`).
9. `family_after_move`.

Breeding reuses M1 goats' `animal_brain.rs` (M20 owner; segment m2g waits for it). The child step sets the ages and resets love, pushes `(id, 18)` (E1's `broadcast_entity_event`, else `entity_events`), adds the orb through `trade_experience`, then sets pregnancy.

**The tongue:**
- The sensor walks `mob_candidates` and reads `self.slimes` for the size.
- The pull writes the prey's `body_mut` velocity.
- The bite is `mob_hits_mob(frog, prey, 10.0, ..)`, never `family_strike`: a frog does not swing.
- If the prey died: set its `death_ticks` to 18 (decision 3, `frogs-leap-eat`), and write `death.attacker_variant` into `damage_state_mut(prey)` (`frogs-froglights`, which adds that field).

**The tadpole tick:**
- M2-1's `swim_ai_step` through `family_goal_step_with`, with `SwimControls { Smooth(85, 10, 0.02, 0.1, true), Smooth(10), Fixed(0.01), sink_without_target: true }`.
- Its `MonsterSpec` holds the three `AbstractFish` goals from fish's `abstract_fish.rs` (M2-3).
- The brain runs in a new `custom` slot of `swim_ai_step` (M2-1's `swimming.rs`, added in `tadpoles-live`), after the navigation tick and before the move control. That is vanilla's `customServerAiStep`, on the same `GroundNavigation`.
- The flop sound is `entity.tadpole.flop`.
- Growth at 24000:
  - `core.removed`, then `spawn_family(Frog)` copying what the hoglin → zoglin conversion copies;
  - persistence and the long-jump cooldown from `level_random`;
  - the variant from M1 rabbits' `World::biome_in_tag(pos, cold tag)` and `(.., warm tag)` with vanilla's priority pick;
  - the grow-up sound.

**Shared-file lines** (each next to a related line):

| File and line | Change | Group |
|---|---|---|
| `natural_spawner.rs:404`, `:542` | `frog_variants: VariantRegistry` and its `load`, after the chicken lines | `frogs-live` 2 |
| `entities/src/health.rs:31`, `loot.rs:28` | `attacker_variant: Option<&'static str>` after `attacker` in `Death` and in `EntityLootContext`, after M1 turtles' "Minecraft loot tables know what killed a mob" has changed those structs (M20) | `frogs-froglights` |
| `world.rs:659-660` | `death_of` carries `attacker_variant` | `frogs-froglights` |
| `loot.rs:636-645` | `damage_source_properties`, as M1's turtles left it, also accepts `minecraft:components` holding only `minecraft:frog/variant` | `frogs-froglights` |
| `level/mod.rs:27`, `:705-706`, `:856-857` | `pub mod frogspawn;` after `pub mod fluid;`; a `FrogspawnBlock` arm after the `FrostedIceBlock` arm in `on_place` and after the coral arm in `tick_block` (M20: M1's sniffer eggs take `wet_sponge` and `FireBlock`, M3's dried ghasts `FrostedIceBlock` in `tick_block`, M3's creaking hearts the line before `ComposterBlock`) | `frogspawn` |
| M2-1's amphibious branches (`walk_path.rs`, `navigation.rs`) | the frog knobs: feet start in water, `OPEN` over preferred blocks, no corner cut at `WATER_BORDER` | `frogs-live` 1 |
| M1 rabbits' `World::biome_in_tag` (`player/src/lib.rs`, `server_mobs.rs`) | used, not added (M20 owner) | `tadpoles-live` |

**Frogspawn:**
- `frogspawn_on_place` schedules 3600-12000 ticks.
- `frogspawn_tick`:
  - If it cannot survive (`can_survive_state`), `destroy_block_drops(pos, false, LIMIT)`.
  - Otherwise the same destroy, the sound into `Level.sounds`, and 2-5 tadpole tags pushed to `spawned` in `golem_build`'s pattern (`PersistenceRequired`, the position and yaw above, `next_uuid`).
- The frog lays it through `World::set_block` (`server_mobs.rs:246` → `Level::set_block` with `ALL`), so `on_place` schedules the hatch.

**Rows:**
- Frog:
  - `TypeInfo { 0.5, 0.5, OnGround, MotionBlockingNoLeaves, false, Rules::SpawnableOn("minecraft:frogs_spawnable_on"), Walk::Animal, false, 4 }`, `Creature`, follow 16, `baby_chance: Some(0.05)`, `rule: None`.
  - The `SpawnableOn` arm (`natural_spawner.rs:761-783`) is exactly `checkFrogSpawnRules`.
  - `finalize`: `frog_variants.pick` → `variant`, then `Brain.memories."minecraft:long_jump_cooling_down" = 100 + nextInt(41)`. It works without context.
- Tadpole: `{ 0.4, 0.3, NoRestrictions, MotionBlockingNoLeaves, false, Rules::Family { creature: true }, Walk::Neutral, false, 4 }`, `Creature`, `baby_chance: None`, no rule and no finalize.

**Multiplayer (`TerrainFamily`):**

| | Frog | Tadpole |
|---|---|---|
| Constants | `TRACKING_RANGE` 10, 3 `TEXTURES`, `SHADOW_RADIUS` 0.3, `MAX_HEAD_Y_ROT` 5 | 10, 1 texture, 0.14 |
| `VOLATILE` | `["LoveCause"]` | `["AgeLocked"]` |
| `read`/`fields` | `variant`, `InLove`, `LoveCause`, `Brain.memories` | `Age`, `AgeLocked`, `FromBucket`, `Brain.memories` |
| `FamilyShow` | flags bit 0 M2-1's `IN_WATER`; `values[0]` pose (0 standing, 1 jump, 2 croak, 3 tongue), `values[1]` ticks since that pose began | bit 0 `IN_WATER`; `values[0]` feeds so far |
| `interact` | `animal::interact` with the slime ball (love, eat sound, event 18). Leads are handled before it (`mob_actions.rs:156`) | slime ball → feed; golden dandelion → lock; water bucket → M2-1 `buckets::pickup::<Tadpole>` (row keys `Age`, `AgeLocked`) |
| `client_effects` | none | one `HappyVillager` per new feed, through `fx.seen` |

The variant travels in the look tag.

**Keyframes:**
- Read through `keyframes::shared().definition("FrogAnimation", ..)`.
- The start tick is the client age less `values[1]`.
- Apply `Moved` as `bat_render.rs` does, including the scale: the pose times `Mat4::from_scale(1 + scale)` after the rotation.

**Protocol:** unchanged. The walk swing uses M3 creakings' `TerrainFamily::walk_target`, computed on each client from the family, with no `ServerMob` field.

### Commit groups

| # | Key (segment) | Commits | Rows |
|---|---|---|---|
| 1 | `frogs-live` (m2g, after M1's `animal_brain.rs`) | 1. "Minecraft frogs live in the entity world and swim, walk, croak and breed like vanilla" (pregnancy is only marked; `TerrainFamily` without drawing). 2. "Minecraft frogs spawn in swamps in three variants" | 33 `live` |
| 2 | `frogs-drawn` (m2g) | 1. "Minecraft frogs are drawn like vanilla with their jumps, croaks and tongues". When M3 creakings' `TerrainFamily::walk_target` is in the branch, the frog answers it in the same commit (`min(distance × 25, 1)`, and 0 mid-jump if the hook passes the mob; if it does not, ask M3 for that in the log) | 33 `drawn` |
| 3 | `tadpoles-live` (m2g, after M1's `World::biome_in_tag`) | 1. "Minecraft tadpoles swim, eat slime balls and grow into frogs like vanilla" (bucket pickup included) | 34 `live` |
| 4 | `tadpoles-drawn` (m2g) | 1. "Minecraft tadpoles are drawn like vanilla" | 34 `drawn` |
| 5 | `frogspawn` (m2g) | 1. "Minecraft frogs lay frogspawn that hatches into tadpoles" (steps and kit so far in the log) | |
| 6 | `frogs-leap-eat` (m2h, after M1's `long_jump.rs`) | 1. "Minecraft frogs long-jump to lily pads and big dripleaves". 2. "Minecraft frogs catch small slimes and magma cubes with their tongues" | 34 `done` |
| 7 | `frogs-froglights` (m2h, after M1 turtles' loot commit; it moves to m2i if that is not merged) | 1. "Minecraft magma cubes drop magma cream again, and froglights when a frog eats them" (`attacker_variant`, written on the tongue kill, and the components branch) | 33 `done` |

### Tests

- **`frogs-live`:**
  - `world::families::water::frogs::tests::a_frog_is_half_a_block_with_ten_health_and_its_kind`: census, candidates, empty table, 1-3 experience, `Age:-24000` ignored.
  - `a_frog_swims_fast_in_water_and_walks_slowly_on_land`.
  - `a_frog_path_prefers_shallow_water_and_lily_pads`.
  - `a_frog_croaks_on_land_for_three_seconds_but_not_in_water`.
  - `two_frogs_fed_slime_balls_breed_and_one_becomes_pregnant`.
  - `a_hurt_frog_panics_at_twice_its_speed`.
  - `a_frog_takes_five_less_fall_damage`.
  - `a_frog_never_drowns`.
  - `the_frog_brain_picks_tongue_lay_spawn_long_jump_swim_idle_in_order`.
  - Terrain `a_frog_saves_and_loads_with_its_variant_memories_and_love`, `a_friend_sees_a_frog_from_its_look_tag`, `a_slime_ball_makes_a_frog_fall_in_love`.
  - World `frogs_spawn_on_grass_mud_and_mangrove_roots_in_the_light`, `a_frog_finalize_picks_warm_in_mangrove_swamps_cold_in_snowy_plains_and_temperate_in_swamps`.
- **`frogs-drawn`:**
  - `frogs::render::tests::a_frog_has_sixteen_parts_on_a_48_texture_in_three_variants`.
  - `the_croaking_body_shows_only_while_croaking`.
  - `jump_croak_and_tongue_start_from_the_shown_ticks` (gated).
  - `a_still_frog_in_water_plays_its_swim_idle`.
  - With M3's hook: `a_frog_swings_its_legs_at_twenty_five_times_its_speed`.
- **`frogs-leap-eat`:**
  - `a_frog_long_jumps_within_four_across_and_two_up_after_its_cooldown`.
  - `half_its_jumps_look_for_lily_pads_and_big_dripleaves_first`.
  - `a_frog_may_land_on_a_trapdoor_but_never_in_water`.
  - `a_frog_eats_a_small_slime_within_ten_blocks_and_it_drops_one_slime_ball`.
  - `a_frog_ignores_big_slimes_and_marks_unreachable_ones_for_five_seconds`.
  - `the_tongue_pulls_its_prey_and_bites_after_six_ticks`.
  - `an_eaten_slime_is_gone_within_two_ticks_after_its_death_is_reported`.
  - `a_breeding_frog_does_not_hunt`.
- **`frogs-froglights`:**
  - `loot::tests::a_magma_cube_killed_by_a_player_drops_magma_cream`.
  - `a_small_magma_cube_eaten_by_a_frog_drops_the_froglight_of_its_variant`.
  - `world::families::water::frogs::tests::a_frog_s_kill_carries_its_variant_to_the_loot`.
- **`tadpoles-live`:**
  - Terrain `a_tadpole_grown_in_a_mangrove_swamp_becomes_a_warm_frog` (through M1's `biome_in_tag`).
  - `tadpoles::tests::a_tadpole_is_tiny_with_six_health_and_stays_forever`.
  - `a_tadpole_becomes_a_frog_after_24000_ticks_where_it_swam`.
  - `a_slime_ball_cuts_a_tenth_of_the_growth_left`.
  - `a_golden_dandelion_locks_a_tadpole_and_resets_its_growth`.
  - `a_stranded_tadpole_flops_and_dies_after_360_ticks`.
  - `a_tadpole_swims_away_from_players_and_follows_slime_balls`.
  - Terrain `a_tadpole_saves_and_loads_with_its_age_and_lock`, `a_friend_sees_a_tadpole_from_its_look_tag`, `a_water_bucket_scoops_a_tadpole_that_pours_back_with_its_age`.
- **`tadpoles-drawn`:**
  - `a_tadpole_is_a_body_and_a_flat_tail`.
  - `its_tail_wiggles_harder_out_of_water`.
  - `feeding_a_tadpole_sparkles_once_in_every_window`.
- **`frogspawn`:**
  - `level::frogspawn::tests::frogspawn_hatches_two_to_five_persistent_tadpoles_after_3600_to_12000_ticks`.
  - `frogspawn_without_water_below_breaks_without_hatching`.
  - `a_pregnant_frog_walks_to_the_shore_and_lays_frogspawn_on_the_water`.
  - Terrain `server::tests::laid_frogspawn_hatches_tadpoles_the_entity_world_simulates`.

### Owner's steps

Setup:
- Survival, Normal, by day, `/gamerule mob_griefing true`, with the kit below.
- On flat grass, run `/fill ~2 ~-2 ~2 ~8 ~-1 ~8 minecraft:water` for a 7 × 7 pool 2 deep in the ground (the host can run it in survival).
- Put 4 lily pads and 2 big dripleaves on the pool.
- Steps 14-15 need two windows (`scripts\mp-local.ps1`).
- The leap in step 3 and step 11 need `frogs-leap-eat` (segment m2h); steps 12-13 need `frogs-froglights` (m2h, or m2i if M1's turtles come late); the rest need segment m2g.

1. `/locate biome minecraft:swamp` and `/tp` there. You see orange-brown frogs on the grass and in the shallows. On land their throats puff out for a few seconds. **fr1_swamp**
2. `/locate biome minecraft:mangrove_swamp`: the frogs there are white. Go back to the pool.
3. Use 2 frog eggs at the pool's edge. They hop slowly on land and kick through the water. Within about 7 s one leaps up to 4 blocks, often onto a lily pad or a big dripleaf. **fr2_jump**
4. Hold a slime ball: the frogs follow you. Feed each one a slime ball: hearts, and no baby. Within a minute one walks to the bank and lays frogspawn on the water beside it, with a wet sound. **fr3_frogspawn**
5. `/tick sprint 12000`. The frogspawn pops and 2-5 tadpoles appear in the pool. **fr4_tadpoles**
6. Walk at the tadpoles: they dart away. Hold a slime ball: they follow it. Feed one: you see one green sparkle.
7. Use a water bucket on a tadpole: you get a tadpole bucket. Empty it into the pool: the tadpole is back.
8. `/summon minecraft:tadpole ~ ~ ~ {Age:23900}` in the pool. About 5 s later it becomes an orange frog, with a small sound. **fr5_grow**
9. Summon another the same way and use a golden dandelion on it at once. It stays a tadpole for at least a minute. A second dandelion plays the other sound.
10. Bucket a tadpole, empty the bucket on stone and take the water back. The tadpole flops about, flashes red at about 16 s and dies at about 18 s.
11. `/summon minecraft:slime ~4 ~ ~ {Size:0}` near a frog: its tongue snaps the slime away and leaves one slime ball. **fr6_tongue** Then `/summon minecraft:slime ~4 ~ ~ {Size:1}`: the frogs ignore it. Kill it.
12. In a 7 × 7 stone-brick pen:
    - summon `minecraft:frog` with `{variant:"minecraft:temperate"}`, `{variant:"minecraft:warm"}` and `{variant:"minecraft:cold"}`, 2 blocks apart (orange, white, green);
    - then run `/summon minecraft:magma_cube ~ ~ ~ {Size:0}` once, and again each time the last cube is eaten, until all three colours have dropped.

    Each froglight matches the frog that ate the cube: ochre from the orange frog, pearlescent from the white one, verdant from the green one. One frog may eat twice in a row, so a colour can repeat before the next appears. Place them: they glow. **fr7_froglights**
13. Use a magma cube egg and kill all its pieces with the sword: magma cream drops (it did not before).
14. Friend window: the friend sees the three frog colours, the jumps, croaks and tongue, and the tadpoles' tails. When the friend feeds a frog or buckets a tadpole, the host sees it. **fr8_friend-host**, **fr8_friend-friend**
15. Save and quit, then load. The colours stay, a pregnant frog still lays, and the tadpoles keep their age. In creative, frog and tadpole eggs on a spawner show turning mobs in the cage.

### Kit

```
IW4L_MINECRAFT_KIT=minecraft:frog_spawn_egg*8,minecraft:tadpole_spawn_egg*8,minecraft:tadpole_bucket,minecraft:frogspawn*4,minecraft:slime_ball*64,minecraft:slime_spawn_egg*4,minecraft:magma_cube_spawn_egg*4,minecraft:water_bucket,minecraft:water_bucket,minecraft:bucket*4,minecraft:lily_pad*16,minecraft:big_dripleaf*8,minecraft:golden_dandelion*8,minecraft:grass_block*64,minecraft:mud*32,minecraft:stone_bricks*64,minecraft:glass*32,minecraft:oak_trapdoor*4,minecraft:lead*4,minecraft:spawner,minecraft:iron_sword,minecraft:cooked_beef*32,minecraft:torch*64
```

That is 23 stacks. Every id was checked against `assets/minecraft/items` in the 26.3 jar. The friend needs the same kit.

### Depends on

- **D0 d0a:** registry, base tick, `spawn_family`, `removed`, rows, `TerrainFamily`/`FamilyShow`, `keyframes::shared()`. Nothing from d0b.
- **M2-1:** amphibious (shallow) and water-bound paths, smooth swim controls, `swim_ai_step`, `flop`, swim stroll, the Manhattan scan, `breath`/`quiet_steps`/`swim_sounds`, `IN_WATER`, and the tadpole bucket row and `pickup`.
- **M2 `fish`:** `abstract_fish.rs` (the three goals and the fish controls).
- **M1 (M20 owners; the segments wait for them):** goats' `animal_brain.rs` (m2g) and `long_jump.rs` (m2h); rabbits' `World::biome_in_tag` (m2g); turtles' loot commit (m2h); foxes' `tick_from` (optional, used if merged).
- **M3:** creakings' `TerrainFamily::walk_target` (optional, used if merged).
- **E1:** event 18 hearts (its default row).
- **W (P7):** `block_place` when laying and `block_destroy` at the hatch. The second of W and M2 to merge adds them (W0 rule 3).
- **M2 `axolotls`** hunt tadpoles through `Prey::Kinds`; nothing to do here.

### Open decisions (recommended choice)

1. **Spawn variant:** a `frog_variants: VariantRegistry` field, data-driven like pigs, cows and chickens.
2. **Conversion variant:** M1 rabbits' `World::biome_in_tag`, which must also resolve nested tags such as `#is_jungle` (check it in `tadpoles-live`; if it does not, widen it in its own logged commit). Rejected: a hard-coded list like `villager::type_for_biome`, which would drift from the data.
3. **Eaten prey:** `death_ticks = 18`, so it vanishes within two ticks after its death is reported, close to `remove(KILLED)`. Rejected: the 20-tick death fall.
4. **Froglights:** `Death.attacker_variant` plus components support in `damage_source_properties`. This also restores magma cream and leaves the vanilla table untouched.
5. **The tadpole runs both its fish goals and its brain**, as vanilla does, through a `custom` slot in `swim_ai_step`.
6. **The walk-speed hook** is M3 creakings' `TerrainFamily::walk_target` (M20), used when merged, with no `ServerMob` field. Without it the ×25 swing waits (Left for later).

---

## M2-9. Nautiluses, zombie nautiluses and riding them (groups `nautiluses-live`, `nautiluses-drawn`, `nautiluses-taming`, `nautiluses-riding`)

Family `nautiluses`, group `water`, hit-key byte **71** (rows 37-38 of `docs/round10/mobs.md`), kinds `minecraft:nautilus` and `minecraft:zombie_nautilus` in one store (both `AbstractNautilus`; the state has `zombie: bool`). A brain family (D0-2.4) and the first registered **mount** (D0-2.5). Lines are `minecraft-survival` at `d1a3d028`; `track-d0` at `9eecb2ff` already has the mount hooks in `families.rs` (`MOUNT`, `seats`, `passenger_point`, `dismount`, `controller`, `rider_jumps`, `rider_jump`, `rider_order`, `vehicle_attachment`). Re-check every line; where D0's merged code differs from `track-d0.md`, follow the code.

### What exists today

- **Dormant:** neither kind is in `SIMULATED_KINDS` (`crates/minecraft_terrain/src/server_mobs.rs:433`), `MobCategory::of_type` (`third_party/minecraftoss/world/src/natural_spawner/spawning.rs:97-109`) or `type_info` (`natural_spawner.rs:271`); `tracking_range` gives 5 (`mob_tracking.rs:285`, vanilla 10).
- **Already right:** `dormant.rs:11` keeps the zombie nautilus on peaceful; `wither.rs:88` makes it undead; netherite nautilus armour does not burn (`world/src/level/burning.rs:26`); the loot conditions both tables use exist (`entities/src/loot.rs:523`, `:628`).
- **Jockey stub:** `finalize_drowned` (`spawning.rs:1571`) logs `note_unsupported("zombie nautilus jockey")` at `:1583`. The chicken jockey shows the shape: the vehicle is the root with `Passengers: [rider]` (`:1451`). `/summon` takes SNBT (`minecraft_survival/src/commands.rs:494`).
- **Effect is data only:** HUD row `hud/src/minecraft_effects.rs:59`, colour `potions.rs:110`, icon `render_anim/src/minecraft_gui.rs:807`; no `EffectKind` (`player/src/survival.rs:36-59`, `ALL` length at `:62`), and `tick_air` (`:465`) knows no water breathing (conduit power keeps no breath either).
- **Mounts are cow-store only:** `riding.rs` (`carries_riders` `:223`, `controlling_mob` `:259`, `controlling_player` `:287`, `order_vehicle` `:576`, `jumpable_by` `:663`, `tick_ride_jumps` `:668`) gets D0's generic arms. The screen: `MountInventory::of` (`mount_inventory.rs:31`), `still_valid` (`:198`), written back by `Command::MountInventory` (`server.rs:4134`) into `set_mount_inventory` (`world/equines.rs:807`, M3's file); its flags travel as `MenuExtras::Mount { .., llama }` (`minecraft_net/src/messages.rs:579`) and `McScreen::Mount` (`frame/src/minecraft_ui.rs:86`), drawn on the `"horse"` sheet (`hud/src/minecraft_screens.rs:240-258`, `:328`).
- **To reuse:** `brain_behaviors.rs` (`Living` `:17`, `MoveSink` `:183`); M1 goats' `world/families/animal_brain.rs` (`AnimalPanic`, `FollowTemptation`, `AnimalMakeLove`); taming as wolves (`world/wolves.rs:394-397`) with the owner and event 7 as the horse (`equines.rs:464-472`); `burn_undead` (`world.rs:859`, reachable from `world/families/..`) and `monsters_burn` (`:2559`); `bottle::filled_result` (`bottle.rs:54`); `MeleePush { throw, lift }` (`minecraft_survival/src/impulse.rs:38`); `HorseState::on_player_jump` (`horse.rs:356`); `VariantRegistry` (`natural_spawner.rs:402-408`, `:540-546`, `pick` `:157`).

### Vanilla 26.3 (javap this round)

**`EntityTypes`:** both 0.875 × 0.95, eye 0.2751, passenger attachment 1.1375, tracking 10, not fire immune, allowed on peaceful. Nautilus `WATER_CREATURE`, zombie `MONSTER`. Baby nautilus `scale(0.5)`: 0.4375 × 0.475, eye 0.13755, passenger (0, 0.5, 0). The zombie `canBeABaby` false and is no `Enemy`.

**`AbstractNautilus extends TamableAnimal implements PlayerRideableJumping, HasCustomInventoryScreen`:**
- Attributes: health 15, speed 1.0 (zombie 1.1), attack 3, knockback resistance 0.3, tempt range 10, follow 16.
- `SmoothSwimmingMoveControl(85, 10, 0.011, 0.0, true)` (outside factor 0: it cannot move by itself on land), `SmoothSwimmingLookControl(10)`, `WaterBoundPathNavigation`, malus `WATER` 0, walk value 0, not pushed by fluid, no step sound. `travelInWater`: `moveRelative(getSpeed())`, move, × 0.9 (no gravity).
- Never poisoned. `removeWhenFarAway` true; persistent while tame. `interact` (every player click, before `mobInteract`) sets `PersistenceRequired`.
- `hurtServer`: a living attacker, if attackable ignoring sight, becomes `ANGRY_AT` for 400 ticks.
- `finalizeSpawn`: `ATTACK_TARGET_COOLDOWN` = `UniformInt(2400, 3600)` from the level random, then the `AgeableMob` baby draw (chance 0.05, not the first of a group).
- **Restriction:** tame, unleashed, unridden only; radius 32 unsaddled adult, else 16; re-homes at its block when farther than radius + 8 or when the radius changed; stroll spots outside are refused (`GoalUtils.isRestricted`). Saved `home_pos`, `home_radius`.
- **Riding:** one seat. `getControllingPassenger`: saddled with a player first → that player; else the first passenger if it is a mob and the vehicle has AI. `getRiddenInput`: strafe `xxa`; when `zza ≠ 0`, forward `cos(pitch)`, up `−sin(pitch)`, both × −0.5 backwards. `tickRidden`: yaw += `wrap(playerYaw − yaw) × 0.5`, pitch = player pitch × 0.5, head and body follow; a pending jump runs `executeRidersJump`. Speed 0.0325 × speed in water (≈ 6.5 blocks/s), 0.02 × speed on land.
- **Dash:** `canJump` = saddled. `onPlayerJump(power)` only when saddled and cooldown 0: pending = 1.0 at 90+, else 0.4 + 0.4 × power / 90. `handleStartJump`: dash sound, `entity_action`, `DASH` on. `executeRidersJump`: velocity += player look × (1.2 in water, 0.5 on land) × scale × speed × block speed factor; cooldown 40. `tick`: `DASH` off below 35, countdown, dash-ready sound at 0. The client charges the bar only at cooldown 0.
- **`applyEffects`:** a player first passenger gets `breath_of_the_nautilus` (60 ticks, amplifier 0, ambient, icon) when missing or every 40 game ticks. `hasWaterBreathing` = water breathing, conduit power or the breath; with only the breath air neither falls nor refills.
- **`mobInteract`**, in order: baby → `Animal`'s (fish ages it); tame and sneaking → its screen; untamed with `#nautilus_taming_items` (pufferfish, pufferfish bucket) → use it, `nextInt(3) == 0` tames (stop navigation, event 7) else event 6, eat sound; `#nautilus_food` (`#fishes` and the four fish buckets) below full health → heal 2 × the item's nutrition (1 without a `food` component); `interactLivingEntity` (saddle and armour through `Equippable`; slots usable when alive, adult, tame; armour for `#can_wear_nautilus_armor`); tame, not sneaking, not food → ride; else `Animal`'s love. A fish bucket used up becomes a water bucket. Breeding (nautilus only) copies a tame parent's owner.
- `canBeLeashed`: not angry and no attack target; the zombie also not while a mob rides it.
- **Sounds** (neutral source), `_land` when not under water: `entity.nautilus.{ambient,hurt,death}[_land]`, `.eat`, `.swim`, `.dash[_land]`, `.dash_ready[_land]`; babies `entity.baby_nautilus.*`; zombie `entity.zombie_nautilus.*`. Equip: `item.nautilus_saddle_equip` (`..._underwater_equip` under water), `item.armor.equip_nautilus`.
- **Client bubbles** (in water, each tick): with `s = |v|`, chance `clamp(2s, 0.15, 1)`: one `bubble` at `(x − d.x·1.1, y − d.y + 0.25, z − d.z·1.1)` (`d` the view vector, pitch clamped ±10), random speed `(r − 0.5)·0.8·nextDouble·(1 + s)` per axis.

**`Nautilus`:** air 300; out of water (`isInWater`) air − 1, at −20 air 0 and 2 `dry_out`; in water 300: first hurt at 16 s, dead at 23 s. Sensors: nearest living, adult, players, hurt by, temptations.
**`ZombieNautilus`:** no air rule (undead); `#burn_in_daylight` with sun protection in the BODY slot (armour, no durability); `variant` `minecraft:warm` in `#spawns_coral_variant_zombie_nautilus` (warm ocean), else `minecraft:temperate`, picked at its block in `finalizeSpawn`.

**Brains** (FIGHT, else IDLE):
- CORE: `AnimalPanic(1.6)` (nautilus), `LookAtTargetSink(45, 90)`, `MoveToTargetSink`, countdowns of the temptation, charge and attack-target cooldowns.
- IDLE: `AnimalMakeLove(NAUTILUS, 0.4, 2)` (nautilus); `FollowTemptation(1.3; zombie 0.9; stop 2.5 baby, 3.5 adult)`; `StartAttacking(find)`; gate on no walk target: `RandomStroll.swim(1.0)`, `SetWalkTargetFromLookTarget(1.0, 3)`.
- `find`: none while breeding, out of water, baby or tame; the `ANGRY_AT` entity in water; none during the attack-target cooldown; else re-arm it (2400-3600) and with 50 % the nearest visible pufferfish (`#nautilus_hostiles`) in water.
- FIGHT (target, no tempting player, no breed target, no charge cooldown): `ChargeAttack(80, .., 0.6 (zombie 0.5), 2.0, 12, 11, dash sound)`, at most 60 ticks while not tame, within 12 of its start, target within 11 and in sight. Velocity = direction × speed every tick; the first living in its box (not its passenger) takes `mobAttack` 3 and an extra knockback `(clamp(speed × moveSpeed, 0.2, 2) + 0.25·(speedAmp − slowAmp)) × 2` (1.2; zombie 1.1) along its yaw, its own velocity × (0.6, 1, 0.6). Stop: charge cooldown 80, target erased. Saved memories: `angry_at` (ttl), `attack_target_cooldown`, `charge_cooldown_ticks`, `temptation_cooldown_ticks`.

**`Drowned.finalizeSpawn`:** `NATURAL` or `STRUCTURE`, trident, `nextFloat() < 0.5`, adult, biome not `#more_frequent_drowned_spawns` → a `JOCKEY` zombie nautilus at its position and yaw (persistent for `STRUCTURE`), finalized, ridden. The rider steers: `Mob.getNavigation`/`getMoveControl` return the vehicle's.

**Spawning:** nautilus `IN_WATER`, `MOTION_BLOCKING_NO_LEAVES`, rule `seaLevel − 25 ≤ y ≤ seaLevel − 5`, fluid below `#water`, block above `minecraft:water` (spawners too); zombie: none. `water_creature`, count 1: weight 5 in ocean, deep ocean, (deep) lukewarm, warm; 2 in (deep) cold, (deep) frozen.

**Loot:** nautilus: killed by a player, 5 % (looting 6 % + 1 %/level) one nautilus shell. Zombie: killed by a player, 0-3 rotten flesh, looting +0-1. Experience 1 + `nextInt(3)`, none for babies. Saddle and armour drop on death.

**Render:** `NautilusRenderer` (shadow 0.7): `NautilusModel` adult and baby layers (128 × 128; root → `shell`, `body` → `upper_mouth`, `inner_mouth`, `lower_mouth`), `entity/nautilus/nautilus`, `nautilus_baby`. `setupAnim`: `body` y/x = head yaw and pitch clamped ±10°, then `NautilusAnimation.SWIMMING` (1.0 s, looping) by `applyWalk(walkPos + age/5, walkSpeed + 0.2, 2.0, 3.0)`. Layers: `NautilusArmorModel` (body mesh, shell +0.01) with `entity/equipment/nautilus_body/{copper,iron,gold,diamond,netherite}`; `NautilusSaddleModel` (shell +0.2) with `entity/equipment/nautilus_saddle/saddle`. Zombie (`MobRenderer`, 0.7): temperate `zombie_nautilus` on `NautilusModel`, warm `zombie_nautilus_coral` on `ZombieNautilusCoralModel` (corals hidden under armour). Armour defence: copper 4, iron 5, gold 7, diamond 11, netherite 19 (+3 toughness, +0.1 knockback resistance). Screen: saddle (8, 18), armour (8, 36), no chest, `gui/container/nautilus.png`, icon `container/slot/nautilus_armor_inventory`.

### Plan

**Files** (D0-3; rustfmt-clean, no comments):

| File | Holds |
|---|---|
| `third_party/minecraftoss/entities/src/world/families/water/nautiluses.rs` | `pub struct Nautilus { zombie, variant, tame, owner, saddled, body, age, love, air, home, dash_cooldown, dashing, jump_pending, granted, brain, navigation, controls, shown }`, `impl Family` (`KEY` 71, `KINDS`, `MOUNT = true`), the tick, tests |
| `water/nautiluses/brain.rs` | CORE, IDLE, FIGHT on `animal_brain.rs`, `ChargeAttack`, `find`, the restriction |
| `water/nautiluses/riding.rs` | ridden travel, dash, dismount (`Animal.getDismountLocationForPassenger`, `DismountHelper.offsetsForDirection`; javap during the work; open water falls back to the top of the vehicle), `EntityWorld::nautilus_use` (the click rules) and `set_nautilus_equipment` |
| `crates/minecraft_terrain/src/families/water/nautiluses.rs` (+ `nautiluses/render.rs`) | `impl TerrainFamily`, `nautilus_inventory`, the four model ports on `posed_cube`, layers, bubbles |
| `world/src/natural_spawner/families/water.rs` | two rows, `nautilus_rule`, both finalizes, `pub(crate) fn zombie_nautilus_jockey` (see the visibility note below) |

**Registration:** `pub mod nautiluses;` and `nautiluses: nautiluses::Nautilus,` (entities `families/water.rs`); `mod nautiluses;` and `Nautilus` in `terrain_group!(water { .. })`; the rows in `ROWS`.

**Hooks used** (D0-2.2): `spawn` (size, eye, health by kind and age), `tick`, `category` (`water_creature` / `monster`), `armor` (body item), `hurt` (`hurt_plain`, then `ANGRY_AT`), `experience` (adult 1 + `next_int(3)`), `death_drops` (saddle, armour), `despawn` (kept while tame or persistent; zombie `peaceful_ok`), `immune_to_poison`, `breathes_water` (zombie), `leashable`, `leash_walk` (water navigation, 1.0), `knockback_resistance` (0.3, 0.4 in netherite), `voice` (`"neutral_volume"` for both; the zombie's category `monster` would otherwise give D0's `"hostile_volume"`), `panicking`, `seats` 1, `passenger_point` (0, 1.1375 or baby 0.5, 0), `dismount`, `controller`, `rider_jumps` (saddled, cooldown 0), `rider_jump` (`handleStartJump` if power > 0, then `onPlayerJump`), `rider_order` (the drowned's target into the water navigation). M2-1: `breath` `Own` for the nautilus (dry-out in the tick, `minecraft:dry_out`), `Lungs` for the zombie; `swim_sounds`; `quiet_steps`; voices rewritten to the `_land` and baby events after the base tick and hurts, as axolotls do.

**Tick:** `family_base_tick`; air; the zombie's `burn_undead(.., monsters_burn, body.map(|_| false))`; restriction; brain (sensors every 20 ticks, phased as the hoglin's); a player controller → ridden travel (navigation stopped, `travelInWater` or air travel with gravity), else M2-1's `swim_ai_step`; `family_after_move`; dash flag, cooldown, ready voice; breath grants; age and love.

**One-liners in shared files** (beside a related line):
- `spawning.rs:1583`: becomes `return super::families::water::zombie_nautilus_jockey(self, level, root, pos);` (catalog tag at the drowned's position and yaw, new UUID, finalize, `finalize_mob`, `Passengers: [drowned]`).
  - **Visibility.** D0's world root `natural_spawner/families.rs` declares `mod water;` (private, `track-d0` at `4fdf76e1`), and `spawning` is a sibling of `families`, so it cannot reach `families::water` today. Right before `nautiluses-live` 2, a commit of its own makes that line `pub(super) mod water;` (one word in the root, "Water mob spawn rows can be reached from the rest of the natural spawner", logged as a hook, as M2-1 does for the terrain root); skip it if the merged root already says so. The function is `pub(crate)`: a `pub(super)` function in `families::water` would still be invisible to `spawning`.
- `natural_spawner.rs:408`, `:546`: `zombie_nautilus_variants` and its `load` after `wolf_variants` (frogs add theirs after chicken, `:404`/`:542`).
- `mount_inventory.rs` (M20: M3 owns the horse lines, and E1's `horse-dye` (e1b) changes the stack builder `:34-45` and `server.rs:4134`): `pub nautilus: bool` after `llama` (`:22`) and `nautilus: false,` after the `llama:` line of the horse literal (`:53`); a new first line in `of` (`:31`, before the cow lookup at `:32`) and in `still_valid` (`:198`) asks `crate::families::water::nautilus_inventory`; `pub fn apply(..)` after `still_valid` tries the nautilus, else `world.set_mount_inventory`. `server.rs:4134` calls `apply` with the arguments E1's `horse-dye` gave that line (decision 1); `nautiluses-taming` waits for e1b's merge because of this line.
- `nautilus` beside each `llama` of the screen (decision 2): `frame/src/minecraft_ui.rs:90`, `messages.rs:584`, `menus.rs:1597`, `:1922`, `:1935`, `render_anim/src/minecraft_inventory.rs:788`, `:856`, `:858`; the HUD picks sheet `"nautilus"` (`minecraft_screens.rs:328`) and the armour icon (`:254`). The test literals that name `llama` get `nautilus: false` beside it (`codec.rs:601`, `:832`, `:851`, `menus.rs:2883`, `:2927`, `minecraft_screens.rs:1192`, `:1205`, `:1214`), plus a nautilus case in the codec round trip.
- `EffectKind::BreathOfTheNautilus` after `SlowFalling` (`player/src/survival.rs:53`, `:77`; `ALL`'s length `:62` goes from the dolphins' 21 to 22), `player_nbt.rs:33`, `commands.rs:172`, `potions.rs:245`; `water_breathing()` beside `tick_air` (`:465`), honoured there and in `Survival::breathe` (`minecraft_survival/src/survival.rs:448`).

**Multiplayer:** `TRACKING_RANGE` 10, `SHADOW_RADIUS` 0.7, ten `TEXTURES`, `VOLATILE` `["home_pos", "home_radius", "LoveCause", "ForcedAge"]` (`Age`, `InLove`, `Air`, `Brain` are base keys). `read`/`fields`: `Owner`, `Sitting`, `Age`/`ForcedAge`/`AgeLocked`, `InLove`/`LoveCause`, `equipment.saddle`/`.body`, `drop_chances`, `home_pos`/`home_radius`, `Air`, `Brain.memories`, `variant`. Saddle, armour, owner and variant reach friends through the look tag (the mirror respawns once on a change, as horses do). `show`: `FamilyShow { flags: IN_WATER (M2-1's bit 0), values: [speed in thousandths of a block a tick, ..] }`; `show_on` stores both for the bubbles. Riding, the bar and the effect run on the host for every player. **Protocol:** `MenuExtras::Mount.nautilus` is a serialized change: "## Protocol" in the log, no bump.

**Keyframes:** `keyframes::shared().definition("NautilusAnimation", "SWIMMING")` with `apply_walk(walk + age / 5.0, speed + 0.2, 2.0, 3.0, ..)` after the ±10° body turn.

### Commit groups

| Key | Commits | Scope |
|---|---|---|
| `nautiluses-live` (m2h) | 1. "Minecraft nautiluses and zombie nautiluses swim, dry out on land and charge in the entity world" (on M1's `animal_brain.rs`). Then, only if D0's world root still has a private `mod water;`, the one-word commit "Water mob spawn rows can be reached from the rest of the natural spawner" (the visibility note above). 2. "Minecraft nautiluses spawn in oceans and drowned ride zombie nautiluses" | Family, brains, charge, air, burning, sounds, loot. Rows: 0.875 × 0.95, `InWater` / `NoRestrictions`, `MotionBlockingNoLeaves`, `Rules::Family { creature: true / false }`, `Walk::Neutral`, `remove_far`, cluster 4, `WaterCreature` / `Monster`, follow 16, `baby_chance` `Some(0.05)` / `None`; rule, finalizes, variants, jockey. Rows stay `left`. |
| `nautiluses-drawn` (m2h) | 1. "Minecraft nautiluses are saved, hatch from eggs and are drawn like vanilla" 2. "Minecraft nautiluses trail bubbles as they swim, for friends too" | `TerrainFamily`, adult, baby, coral, armour and saddle layers, spawner cage; `client_effects` with guardians' `Kind::Bubble`. Rows `drawn`. |
| `nautiluses-taming` (m2h, after E1's e1b, which commit 3 needs) | 1. "Minecraft nautiluses are tamed with pufferfish, fed fish and bred" 2. "Minecraft nautiluses wear saddles and armour" 3. "Minecraft nautiluses open their own saddle and armour screen" | 1: `nautilus_use`, `interact`, persistence, love, leads, restriction. 2: equip by click with the sounds, armour defence, death drops (entities and terrain family files only). 3: the screen: `mount_inventory.rs`, `server.rs:4134`, the `nautilus` flag beside each `llama` ("## Protocol": `MenuExtras::Mount.nautilus`). |
| `nautiluses-riding` (m2i, after `dolphins-grace`) | 1. "Minecraft players keep their breath with breath of the nautilus" 2. "Minecraft players ride saddled nautiluses and dash with the jump bar" 3. (optional) "Minecraft dolphins follow players riding nautiluses" | Effect (testable with `/effect`); controller, ridden travel, dash, dismount, grants, the drowned steering. Rows `done`; steps and kit into the log. |

### Tests

- **live** (`world::families::water::nautiluses::tests`): `a_nautilus_is_fifteen_health_and_0_95_tall_and_its_baby_half_that` (eye, `entity_type`, census, `mob_candidates`, table for adults, 1-3 experience); `a_zombie_nautilus_is_an_undead_monster_that_stays_on_peaceful`; `a_stranded_nautilus_is_hurt_at_320_ticks_and_dies_by_460`; `a_nautilus_on_land_cannot_swim_away`; `a_hit_nautilus_charges_its_attacker_for_three_and_a_push_of_1_2` (400-tick anger, 12/11 blocks, sight, cooldown 80, `Melee` with `throw`); `a_tame_or_baby_nautilus_never_starts_a_fight`; `an_idle_nautilus_hunts_a_pufferfish_half_the_time_after_its_cooldown`; `fish_tempt_a_nautilus_at_1_3_and_a_zombie_nautilus_at_0_9`; `a_zombie_nautilus_burns_by_day_on_land_unless_armoured`; `the_nautilus_speaks_with_its_land_water_and_baby_voices`. World: `nautiluses_spawn_five_to_twenty_five_below_sea_level_under_water`, `a_nautilus_spawn_draws_its_attack_cooldown_from_the_level_random`, `a_zombie_nautilus_is_coral_in_warm_oceans`, `spawning::tests::half_of_natural_trident_drowned_outside_rivers_ride_a_zombie_nautilus`.
- **drawn:** `a_nautilus_saves_and_loads_with_its_owner_saddle_armour_home_and_memories` (and `variant`); `a_friend_sees_a_nautilus_from_its_look_tag` (`spawn_tracked`, `show_on`, range 10, own `VOLATILE` keys only); `carries_loads_a_drowned_jockey_from_its_passengers` (D0-9.4's first `is_vehicle` test); `render::tests::a_nautilus_has_a_shell_body_and_three_mouth_parts_on_a_128_texture`, `the_coral_variant_hides_its_corals_under_armour`, `the_swim_animation_runs_while_it_floats_still` (gated); `a_swimming_nautilus_trails_bubbles_for_every_viewer`.
- **taming 1:** `one_pufferfish_in_three_tames_a_nautilus_with_hearts_or_smoke` (owner, bucket → water bucket, creative); `a_tame_nautilus_heals_on_fish_and_falls_in_love_at_full_health`; `two_tame_nautiluses_make_a_tame_baby`; `any_click_makes_a_nautilus_persistent`; `a_tame_nautilus_stays_within_32_blocks_or_16_saddled`; `an_angry_nautilus_refuses_a_lead`.
- **taming 2:** `a_tame_adult_takes_a_saddle_and_armour_by_click_with_their_sounds`; `nautilus_armour_takes_its_defence_off_a_hit`; `a_saddled_armoured_nautilus_drops_both`.
- **taming 3:** `mount_inventory::tests::a_nautilus_screen_has_a_saddle_and_an_armour_slot_and_no_chest` (refuses horse armour, `apply` writes back); `minecraft_screens::tests::a_nautilus_screen_uses_its_own_background_and_icon`; the codec round trip with `nautilus: true`.
- **riding:** `breath_of_the_nautilus_neither_spends_nor_refills_air` (conduit power too; saved by name); `a_player_steers_a_saddled_nautilus_by_look_at_6_5_blocks_a_second`; `an_unsaddled_nautilus_carries_a_player_but_swims_its_own_way`; `a_released_jump_bar_dashes_along_the_look_and_cools_down_for_two_seconds`; `a_rider_gets_breath_every_two_seconds_in_any_mode`; `a_drowned_steers_its_zombie_nautilus_towards_its_target`; `a_player_gets_off_beside_or_above_the_nautilus`. Optional commit 3: `dolphins::tests::a_dolphin_follows_a_player_riding_a_nautilus` (javap `FollowPlayerRiddenEntityGoal` during the work for its distances and speed).

### Owner's steps

Setup: survival, Normal, by day; `/locate biome minecraft:warm_ocean`, `/tp` there, stand on a block at the surface; the kit below. Steps 1-5 and 9-13 need segment m2h; riding (steps 6-8 and the riding in 14) needs `nautiluses-riding` (segment m2i).

1. Use a nautilus egg on the water 4 blocks away: a spiral-shelled nautilus with a tentacled mouth swims slowly, tilting up and down, mouth parts waving, a thin bubble trail behind it. **n1_nautilus**
2. Hit it once with the sword: it flashes, flees, then rams you with a whoosh (1.5 hearts, a strong push). It rams again every 4 s for about 20 s. `/summon minecraft:nautilus ~ ~ ~3 {Age:-24000}` gives a baby; hit it: it never rams. (A spawn egg used on an adult makes no baby in the fork yet.)
3. Pour water on the beach, hatch a nautilus in it, take the water back: it lies still; after 16 s it flashes red each second and dies after about 23 s.
4. Hatch another in the sea and hold a pufferfish: it follows. Right-click it with pufferfish until hearts rise (one in three; smoke otherwise). The pufferfish bucket works too and leaves a water bucket. **n2_tamed**
5. Right-click it with a saddle (an underwater clunk), then with the iron nautilus armour: both show on the shell. Crouch and right-click: a nautilus screen with a saddle and an armour slot; swap in the golden armour. **n3_screen**
6. Right-click with an empty hand to sit on it. W swims where you look (down dives, up rises), S backs slowly. A "Breath of the Nautilus" icon shows and your air stays still under water. **n4_riding**
7. Hold space, release: it dashes ahead in a burst; the bar stays empty 2 s, then a chime. On the beach it crawls and dashes short. **n5_dash**
8. Crouch under water: you get off above it; the breath lasts 3 s more. Ride 40 blocks away and leave it: it stays within about 32 blocks of that spot (16 saddled).
9. Hit it twice, right-click with cod: it eats and heals. Tame a second one; feed both a cod at full health: hearts, a tame baby and an experience orb.
10. `/summon minecraft:zombie_nautilus` in this warm ocean: coral on its rotting shell; in a plain ocean none. On the beach at noon it catches fire; tamed (pufferfish) and armoured it does not. **n6_zombie**
11. `/summon minecraft:zombie_nautilus ~ ~ ~5 {Passengers:[{id:"minecraft:drowned",equipment:{mainhand:{id:"minecraft:trident",count:1}}}]}` in deep water: the drowned rides and steers it towards you. **n7_jockey** At night some trident drowned come riding zombie nautiluses on their own.
12. Shoot one with the M4A1 and one with the bow: both flash and die. Kill the saddled one: saddle and armour drop.
13. Save and quit with the tamed one and a coral zombie nautilus, load: saddle, armour, owner and coral are kept.
14. Friend window: the friend sees them swim with bubbles, coral, saddle and armour; tames, saddles, rides and dashes their own with the breath icon and the screen; the host sees the friend ride. **n8_friend_host**, **n8_friend_friend**

### Kit

```
IW4L_MINECRAFT_KIT=minecraft:nautilus_spawn_egg*16,minecraft:zombie_nautilus_spawn_egg*8,minecraft:drowned_spawn_egg*4,minecraft:pufferfish*32,minecraft:pufferfish_bucket,minecraft:pufferfish_bucket,minecraft:cod*32,minecraft:cod_bucket,minecraft:saddle,minecraft:saddle,minecraft:copper_nautilus_armor,minecraft:iron_nautilus_armor,minecraft:golden_nautilus_armor,minecraft:diamond_nautilus_armor,minecraft:netherite_nautilus_armor,minecraft:lead*8,minecraft:diamond_sword,minecraft:bow,minecraft:arrow*64,minecraft:trident,minecraft:water_bucket,minecraft:bucket,minecraft:glass*64,minecraft:stone*64,minecraft:cooked_beef*32,minecraft:golden_apple*8
```

26 stacks, all 26.3 item ids. The friend uses the same kit.

### Depends on

- **d0a** for everything; **d0b** not.
- **M2-1:** `water-hooks`, `water-paths` (`water_bound_random_position` with its home, for the tame restriction), `water-swimming`. Not `water-buckets`.
- **M1 goats:** `animal_brain.rs` (M20 owner; segment m2h comes after m2f and m2g, which already wait for it).
- **M2 `dolphins`:** `dolphins-grace`'s effect queue into W7's `Output.player_effects`, and `EffectKind::ALL` at 21 (segment m2i, `nautiluses-riding` after `dolphins-grace`). **`guardians`:** `Kind::Bubble` (m2d). **`fish`:** pufferfish as prey.
- **W (P7):** one `entity_action` game-event line in `rider_jump` once W is merged. **E1:** default event rows 7, 6, 18 draw the taming and love particles; its `horse-dye` (e1b) changes `server.rs:4134` first. **E2:** `hit_boxes` for thrown things.
- **M3** owns `equines.rs` and the horse lines of `mount_inventory.rs` and `riding.rs`. This section adds lines beside the horse lines only in `mount_inventory.rs`; `riding.rs` is never edited (nautiluses ride through D0's generic arms, D0-2.5).

### Open decisions (recommended choice)

1. **Screen write-back:** `server.rs:4134` calls `mount_inventory::apply` (after E1's `horse-dye` changed that line), not a line in M3's `equines.rs:807`.
2. **Screen flag:** `nautilus: bool` beside every `llama` (logged under "## Protocol"), not the horse sheet.
3. **Breath grants:** when a player becomes the first passenger and every 40 game ticks, 60 ticks, any game mode.
4. **Air:** `Breath::Own` with the dry-out rule (`dry_out`, not M2-1's `drown`).
5. **Conduit power** keeps breath too (`hasWaterBreathing`).
6. **Zombie row:** `baby_chance: None` (it cannot be a baby; jockeys and summons draw nothing).
7. **Voices:** neutral for both kinds.

---

## M2-groups. Commit groups (track runner)

Run order. Every group updates `docs/round10/track-m2-log.md` (javap numbers, decisions, vendored files, protocol lines, owner's steps and kit) and its rows in the M2 section of `docs/round10/mobs.md` in the same commit as the code.

| # | Key | Segment | Commits | Section |
|---|---|---|---|---|
| 1 | `water-hooks` | m2a | 1. Registered Minecraft mobs breathe like vanilla water animals. 2. Registered Minecraft mobs make their own swim and step sounds. | M2-1 |
| 2 | `water-paths` | m2a | 1. Minecraft path search takes any node evaluator. 2. Minecraft mobs find paths through water like vanilla swimmers and amphibians. | M2-1 |
| 3 | `water-buckets` | m2a | 1. Minecraft mob buckets pour their water and let out the mob they hold. 2. Minecraft fish buckets carry the mob they caught like vanilla. 3. Dispensers empty fish buckets like vanilla. | M2-1 |
| 4 | `squids-live` | m2a | 1. Registered Minecraft mobs can move by their own rules while dying. 2. Minecraft squids swim in pulses, squirt ink and flee in the entity world. 3. Minecraft squids spawn in oceans and rivers, and glow squids in dark water underground. | M2-2 |
| 5 | `squids-drawn` | m2a | 1. Registered Minecraft mobs can have a tracking range per kind. 2. Minecraft squids and glow squids are saved, hatch from eggs and are drawn like vanilla. 3. Minecraft squids leave ink clouds and glow squids sparkle, for friends too. | M2-2 |
| 6 | `water-swimming` | m2b | 1. Minecraft mobs swim with vanilla smooth swimming and fish controls. 2. Minecraft swimmers wander the water, head for it on land and flop when stranded. | M2-1 |
| 7 | `fish-live` | m2b | 1. Minecraft cod, salmon, tropical fish and pufferfish swim in the entity world. 2. Minecraft cod, salmon and tropical fish swim in schools. | M2-3 |
| 8 | `fish-spawning` | m2b | 1. Minecraft fish spawn in oceans, rivers and lush caves like vanilla. | M2-3 |
| 9 | `fish-drawn` | m2b | 1. Registered Minecraft mobs cast their own shadow size. 2. Minecraft fish are saved, hatch from spawn eggs and are seen by friends. 3. Minecraft cod, salmon, tropical fish and pufferfish are drawn like vanilla. | M2-3 |
| 10 | `fish-pufferfish` | m2b | 1. Minecraft pufferfish puff up and sting what touches them. | M2-3 |
| 11 | `fish-buckets` | m2c | 1. Minecraft fish can be caught in water buckets and poured back out. | M2-3 |
| 12 | `dolphins-live` | m2c | 1. Minecraft dolphins swim, leap, breathe at the surface and dry out on land. 2. Minecraft dolphins fight back as a pod, flee guardians and play with items. 3. Minecraft dolphins spawn in oceans. | M2-4 |
| 13 | `dolphins-drawn` | m2c | 1. Minecraft dolphins are saved, hatch from eggs and are drawn like vanilla. 2. Minecraft dolphins leave a speed trail in the water, for friends too. | M2-4 |
| 14 | `dolphins-treasure` | m2c | 1. Minecraft dolphins fed a fish lead players to shipwrecks and ocean ruins. | M2-4 |
| 15 | `guardians-live` | m2d | 1. Minecraft guardians and elder guardians swim, aim and fire their beams in the entity world. 2. Minecraft guardians spawn in ocean monuments like vanilla. | M2-5 |
| 16 | `guardians-drawn` | m2d | 1. Registered Minecraft mobs are drawn knowing where the camera is. 2. Minecraft guardians and elder guardians are saved, hatch from eggs and are drawn like vanilla, beams included. | M2-5 |
| 17 | `guardians-effects` | m2d | 1. Minecraft guardians trail bubbles and hum while they charge, for friends too. | M2-5 |
| 18 | `sulfur-cubes-live` | m2e | 1. Minecraft sulfur cubes hop about in the entity world. 2. Minecraft sulfur cubes are saved, hatched, seen by friends and spawn in sulfur caves. | M2-6 |
| 19 | `sulfur-cubes-drawn` | m2e | 1. Registered Minecraft mobs can show a block inside their model. 2. Minecraft sulfur cubes are drawn like vanilla. 3. (optional, with M1's `ClientFx.item_crumbs`) Minecraft sulfur cubes splash goo when they land. | M2-6 |
| 20 | `sulfur-cubes-blocks` | m2e | 1. Minecraft mobs slide, drift and bounce by their physics attributes. 2. Minecraft sulfur cubes swallow blocks and take on their archetypes. | M2-6 |
| 21 | `sulfur-cubes-kicks` | m2e | 1. Registered Minecraft mobs take knockback their own way. 2. Minecraft sulfur cubes holding a block are kicked instead of hurt. | M2-6 |
| 22 | `sulfur-cubes-items` | m2e | 1. Registered Minecraft mobs can set off explosions. 2. Minecraft sulfur cubes holding TNT can be lit and blow up. 3. Minecraft sulfur cubes go into empty buckets and baby cubes grow on slime balls. | M2-6 |
| 23 | `axolotls-live` | m2f | 1. Minecraft axolotls live, swim and play dead in the entity world. 2. Minecraft axolotls spawn in lush caves in groups with their babies. | M2-7 |
| 24 | `axolotls-drawn` | m2f | 1. Minecraft axolotls are saved, hatch from eggs and are drawn like vanilla. 2. Minecraft axolotls eat tropical fish from buckets and hand back the water. | M2-7 |
| 25 | `axolotls-hunt` | m2f | 1. Registered Minecraft mobs can hide from the mobs that hunt them. 2. Minecraft axolotls hunt fish and squid, fight drowned and help the player who finishes their prey. | M2-7 |
| 26 | `axolotls-buckets` | m2f | 1. Minecraft axolotls go into water buckets and keep their colour. 2. Splash water bottles rehydrate Minecraft axolotls. | M2-7 |
| 27 | `frogs-live` | m2g | 1. Minecraft frogs live in the entity world and swim, walk, croak and breed like vanilla. 2. Minecraft frogs spawn in swamps in three variants. | M2-8 |
| 28 | `frogs-drawn` | m2g | 1. Minecraft frogs are drawn like vanilla with their jumps, croaks and tongues. | M2-8 |
| 29 | `tadpoles-live` | m2g | 1. Minecraft tadpoles swim, eat slime balls and grow into frogs like vanilla. | M2-8 |
| 30 | `tadpoles-drawn` | m2g | 1. Minecraft tadpoles are drawn like vanilla. | M2-8 |
| 31 | `frogspawn` | m2g | 1. Minecraft frogs lay frogspawn that hatches into tadpoles. | M2-8 |
| 32 | `frogs-leap-eat` | m2h | 1. Minecraft frogs long-jump to lily pads and big dripleaves. 2. Minecraft frogs catch small slimes and magma cubes with their tongues. | M2-8 |
| 33 | `frogs-froglights` | m2h | 1. Minecraft magma cubes drop magma cream again, and froglights when a frog eats them. | M2-8 |
| 34 | `nautiluses-live` | m2h | 1. Minecraft nautiluses and zombie nautiluses swim, dry out on land and charge in the entity world. (Only if needed: Water mob spawn rows can be reached from the rest of the natural spawner.) 2. Minecraft nautiluses spawn in oceans and drowned ride zombie nautiluses. | M2-9 |
| 35 | `nautiluses-drawn` | m2h | 1. Minecraft nautiluses are saved, hatch from eggs and are drawn like vanilla. 2. Minecraft nautiluses trail bubbles as they swim, for friends too. | M2-9 |
| 36 | `nautiluses-taming` | m2h | 1. Minecraft nautiluses are tamed with pufferfish, fed fish and bred. 2. Minecraft nautiluses wear saddles and armour. 3. Minecraft nautiluses open their own saddle and armour screen. | M2-9 |
| 37 | `dolphins-grace` | m2i | 1. Minecraft players swim fast with dolphin's grace. 2. Registered Minecraft mobs can give players effects. | M2-4 |
| 38 | `dolphins-escort` | m2i | 1. Minecraft dolphins swim with swimming players and give them dolphin's grace. | M2-4 |
| 39 | `guardians-curse` | m2i | 1. Minecraft elder guardians curse nearby players with mining fatigue like vanilla. 2. Minecraft players see the elder guardian's face and hear its curse, friends too. | M2-5 |
| 40 | `nautiluses-riding` | m2i | 1. Minecraft players keep their breath with breath of the nautilus. 2. Minecraft players ride saddled nautiluses and dash with the jump bar. 3. (optional) Minecraft dolphins follow players riding nautiluses. | M2-9 |

### Segments

Each segment is one track-runner run (`docs/round10/runs/m2<letter>.json`, base tag `r10-m2<letter>-base`) and can merge on its own for a daily release. The waits are M20's table; a group whose wait is not met moves to the next segment.

| Segment | Groups | Waits for | What a release with it shows |
|---|---|---|---|
| m2a | `water-hooks`, `water-paths`, `water-buckets`, `squids-live`, `squids-drawn` | D0 d0a | mob buckets pour and dispense; squids and glow squids live, spawn, ink and glow |
| m2b | `water-swimming`, `fish-live`, `fish-spawning`, `fish-drawn`, `fish-pufferfish` | M1 rabbits' `family_goal_step_with` | the four fish swim, school, spawn, flop and puff |
| m2c | `fish-buckets`, `dolphins-live`, `dolphins-drawn`, `dolphins-treasure` | m2b | fish in buckets; dolphins, treasure hunting |
| m2d | `guardians-live`, `guardians-drawn`, `guardians-effects` | m2b | guardians and elders with beams, monuments |
| m2e | `sulfur-cubes-live`, `sulfur-cubes-drawn`, `sulfur-cubes-blocks`, `sulfur-cubes-kicks`, `sulfur-cubes-items` | m2b (E2's `ti1` softly) | sulfur cubes, archetypes, kicks, TNT cubes |
| m2f | `axolotls-live`, `axolotls-drawn`, `axolotls-hunt`, `axolotls-buckets` | M1 goats' `animal_brain.rs` | axolotls |
| m2g | `frogs-live`, `frogs-drawn`, `tadpoles-live`, `tadpoles-drawn`, `frogspawn` | M1's `animal_brain.rs` and rabbits' `World::biome_in_tag` | frogs, tadpoles, frogspawn |
| m2h | `frogs-leap-eat`, `frogs-froglights`, `nautiluses-live`, `nautiluses-drawn`, `nautiluses-taming` | M1 goats' `long_jump.rs` (`frogs-leap-eat`), M1 turtles' loot commit (`frogs-froglights`), E1's e1b (`nautiluses-taming`) | frog leaps and tongues, froglights and magma cream; nautiluses tamed and equipped |
| m2i | `dolphins-grace`, `dolphins-escort`, `guardians-curse`, `nautiluses-riding` | W's w1; M1 armadillos' sprint flag (`dolphins-escort`) | dolphin's grace, the elder curse, riding nautiluses |

Groups 37-40 need W (and `dolphins-escort` M1's sprint flag), so they form the last segment. No M2 group needs d0b.

---

## Left for later

Collected from the sections. Each item names why it waits; the track log keeps the list current.

### M2-1. Water movement and buckets

- **The drowned keeps its measured route.** D0-0 rules it out of this round, and the pinned `bin/check_zombie_*` frames would move.
- **Bucket data gaps.**
  - `Silent`, `NoGravity`, `Glowing`, `Invulnerable` and `CustomName` are not modelled for registered mobs, and the fork has no name tags.
  - The bucket triggers and stats are not modelled.
  - `Health` is written as a double through JSON (`item_nbt.rs:30`).
- **Spawn reason.** The bucket runs the command's finalize, not `BUCKET`'s. Only a data-less tropical fish bucket differs.
- **The sulfur cube's dispenser behaviour** belongs to `sulfur-cubes`.
- **Fluid currents** push no mob in the fork, and they push no water animal in vanilla either.
- **`OTHER_BEHAVIOURS` still names the six water mob buckets** (`dispenser.rs:17-26`); the new arm wins, and the names go when no other track edits those lines.

### M2-2. Squids

- A spawn egg on a squid making a baby: no family in the fork has it.
- Impaling against `#aquatic`, and Looting's extra ink sac: neither reaches the fork.
- Ink and glow ink on signs: the fork draws no sign text.
- `hurt_by` clearing when the attacker dies: D0 has only the 100-tick expiry.
- The flee bubbles: vanilla shows none, so none are ported.

### M2-3. Fish

- **The spawn-group leader link** (`SchoolSpawnGroupData.leader`): it needs an entity id that the world crate never has. Natural groups school up through the goal within about 10 s instead of at once; eggs behave the same either way.
- **A rare tropical fish spawning alone** (`isMaxGroupSizeReached`): it needs a new `FamilySpawn` field, which touches every group's rows. Here the rest of such a group rolls its own variant.
- **Impaling** against `#sensitive_to_impaling` (there is no Impaling in the fork).
- **Smaller vanilla details:** the `filled_bucket` advancement, bucket tooltips, Fire Aspect cooking the drop, and the sting sound reaching only the stung player.

### M2-4. Dolphins

- `FollowPlayerRiddenEntityGoal`:
  - Boats: the fork has none.
  - Nautiluses: the `nautiluses` group adds the priority-8 goal to `DOLPHIN` once a player can ride one.
- Dolphin's grace on mobs: no mob effect changes water travel yet.
- The fed baby's green sparkles: no family shows `AgeParticle` (`world.rs:8015` drops them).
- The carried item on friends: `equipment` is volatile, and an item is held only 1-2 ticks.
- Impaling against `#aquatic`, and Looting's extra cod: neither exists in the fork.
- Following the dolphin to its treasure underwater: water breathing is not simulated.
- A spawn egg used on a dolphin making a baby: no family has this yet.
- The event 38 sparkles, if E1's `puff_for` is not merged when `dolphins-treasure` runs.

### M2-5. Guardians

- **A true looping hum.** The fork's audio has no tickable loops, and `ClientFx.sounds` always falls off with distance.
- **Thorns details:**
  - the `enchant.thorns.hit` sound, unless `Typed` plays damage effects;
  - mobs take `mob_attack`, not `thorns`;
  - MW2 bullets prick too (decision 2).
- **A friend's beam end** lags the friend's own camera by a tick.
- **Not in the fork yet:**
  - conduits attacking guardians;
  - Impaling against `#aquatic`;
  - `Silent` (the curse value is always 1);
  - parrots imitating guardians;
  - water breathing for players.

### M2-6. Sulfur cubes

- **Dispensers:** feeding cubes (`SulfurCubeBlockDispenseItemBehavior`; the world crate cannot see mobs, `dropper.rs:18`), shearing them, and emptying their buckets (`dispenser.rs:24`).
- **Small gaps:**
  - the "TNT explosions are disabled" overlay (`MobOutcome` has no overlay);
  - a held skull;
  - the spear's immunity;
  - a spawn egg used on a cube.
- **Spawn box:** `spawnDimensionsScale(2)` is ignored. `TypeInfo` has no field for it, and the slime's 4.0 is ignored too.
- **MW2 bullets:** the bullet's muzzle is taken as the feet, so the shove's vertical split is computed from 1.62 too high.
- **Potent sulfur geysers** (a block feature).
- **Goo particles,** if the optional commit is skipped (it needs M1's `ClientFx.item_crumbs`). Slimes have none either.

### M2-7. Axolotls

- **Drowned hunting axolotls:** zombies and drowned share `zombie_targets` (`monster_ai.rs:1959`), whose turtle slot M1 edits next door; a drowned-only slot needs its own list.
- **Support after a drowned kill:** the zombie store keeps no last player attacker (`ZombieEntity`, `world.rs:296`), so only registered prey count.
- An egg on an axolotl making a baby; Impaling; the bucket advancements; `LoveCause` (no animal saves it).

### M2-8. Frogs, tadpoles and frogspawn

- Age-lock and lingering love particles: the fork draws none for any animal.
- Falling blocks breaking frogspawn (`level/entity.rs:1266`).
- An eaten slime may still leave the client's death poof.
- The feed sparkle shows in every window, not only the feeder's.
- Stats and advancements, `setHatchDelay`, and frog-variant item components.
- The ×25 walk swing and the still legs mid-jump, until M3 creakings' `TerrainFamily::walk_target` is merged (and passes the mob).

### M2-9. Nautiluses

- Ocean-ruin jockeys: the generator's drowned rolls no trident (`generator/src/feature/entities.rs:117`).
- The jump bar's cooldown sprite (the player view carries only the fill, `messages.rs:501`).
- The `entity.nautilus.riding` loop and the `nautilus_one_cm` statistic (no mount counts its statistic yet).
- Shears taking off, and dispensers putting on, saddles and armour (no mount has them yet).
- A baby's own swim sound (`swim_sounds` is per kind).
- After milk the breath returns within 40 ticks, not the next tick.

