# Track M1: land animals on the shared mob registry

## The owner's request

The owner's words for round 10 (`docs/PROGRESS.md`, round 10 brief):

> D. Finish track D: the rest of d5 (trial chambers and trial spawners, lodestones), d3 (deep dark, warden, sculk), and d4 (the remaining mobs), with d4 split into two or three tracks by mob group. Land the shared changes (mob list, spawn tables, render registry) first so the later merges stay small.

Round 10 splits d4 into three mob tracks by group: **M1, land animals (this brief)**, M2, water and buckets, and M3, night, built mobs and the horse family. Track D0 (`docs/round10/track-d0.md`) is the "shared changes first" part: the mob list (`docs/round10/mobs.md`), the spawn tables and the render registry that every new family plugs into. M1 brings D0-8's ten land animals (group `land`, hit-key bytes 48-63) to life on that registry, each live, saved, loaded, hatched, spawned, drawn and seen by friends, with its vanilla behaviours.

| Section | Family (hit key, `mobs.md` row) | Groups | Segment |
|---|---|---|---|
| M1-1 | rabbits (48, row 15) | `rabbits-live`, `rabbits-drawn`, `rabbits-behaviour` | m1a |
| M1-2 | polar bears (55, row 22) | `polar-bears-live`, `polar-bears-drawn` | m1a |
| M1-3 | pandas (52, row 19) | `pandas-live`, `pandas-drawn`, `pandas-eating`, `pandas-moods`, `pandas-thunder` | m1b |
| M1-4 | foxes (49, row 16) | `foxes-live`, `foxes-drawn`, `foxes-hunt`, `foxes-items`, `foxes-trust` | m1c |
| M1-5 | goats (50, row 17) | `goats-live`, `goats-drawn`, `goats-ram`, `goats-leap`, `goat-horns` | m1d |
| M1-6 | armadillos (54, row 21) | `armadillos-live`, `armadillos-drawn`, `armadillos-scutes`, `armadillos-spiders` | m1e |
| M1-7 | sniffers (56, row 46) | `sniffers-live`, `sniffers-drawn`, `sniffers-dig`, `sniffer-eggs` | m1f |
| M1-8 | turtles (53, row 20) | `turtles-water` (optional), `turtles-live`, `turtles-drawn`, `turtles-eggs`, `turtles-hunted` | m1g |
| M1-9 | parrots (51, row 18) | `parrots-live`, `parrots-drawn`, `parrots-tame`, `parrots-shoulders`, `parrots-shoulders-seen` | m1h (after d0b) |
| M1-10 | bees (57, row 44) | `bees-live`, `bees-drawn`, `bees-hives`, `bees-homes`, `bees-pollination`, `bees-honey` | m1i (after d0b) |

The sections run in this order because each one builds on what the earlier ones add (M10, "Hooks this track adds"): rabbits create the shared animal goals and the AI-step hook, pandas the gift, weather, crumb and hold-still hooks, foxes the spared-blocks and break-effect hooks, goats the shared brain behaviours, armadillos the shadow hook; turtles wait for M2's water paths, and parrots and bees for D0's flying navigation (segment d0b). Bytes 58-63 stay free.

Paths and line numbers are `minecraft-survival` at `d1a3d028`. Re-check every line before editing: d0a and the tracks that merge before M1 move them. Vanilla facts were checked with javap on the 26.3 `client.jar` and against the 26.3 data in that jar this round, unless marked "javap during the work". No Mojang code or data is copied: numbers and behaviour only, tags, models and loot read at runtime.

**Multiplayer rule for every item.** The host owns every level, mob and rule. New per-player state is keyed by UUID (saved in `players/data/<uuid>.dat`); new per-level state is saved in that dimension's folder in vanilla's block entity and entity formats. Nothing keys on player 0. Friends run no AI: they mirror a registered mob from its look tag and its per-tick `FamilyShow`, so every drawn state goes through `FamilyShow`, and every particle a friend must see is drawn from the shown state by `client_effects` or travels as a level event or block update.

---

## M10. Shared points for the track

### Base, worktree and waiting

- Branch `track-m1`, worktree `C:\Dev\Soscraft\mc-wt\track-m1`, target dir `D:\iw4l-wt\track-m1`, track log `docs/round10/track-m1-log.md`. One run spec per segment, `docs/round10/runs/m1a.json` to `m1i.json`, with bases `r10-m1a-base` to `r10-m1i-base`.
- The track starts on `minecraft-survival` once D0's segment d0a (the registry with silverfish and endermites as its proof) is merged there.
- **Before every segment the orchestrator merges `minecraft-survival` into `track-m1`**, so each segment sees what other tracks landed. Each segment merges back on its own (full suite in the track's worktree, then a fast-forward), so it can go into the next daily release.
- What a segment waits for:

| Needs | Segments | What it gives M1 | When it is not merged yet |
|---|---|---|---|
| D0 d0a | all | registry, rows, client registry, `FamilyShow`, `keyframes::shared()` | the track does not start |
| D0 d0b (`flying-navigation`, P3) | m1h, m1i | `FlyingPathNavigation`, `FlyingMoveControl` | m1h and m1i never start before d0b is in `minecraft-survival`; run the other segments first |
| M2 P2 (`water-paths`, `water-swimming`) | m1g | amphibious paths, `Body::travel_swimming` | run m1h (if d0b is in) and come back; only if m1g is the last segment left, the optional group `turtles-water` builds them under M2's names (M1-8, decision 1) |
| E1 e1a (`particles-events`, `horse-fall`) | soft: all; m1c | love, taming and smoke hearts from E1's default `puff_for` rows (M1 adds no row); `horse-fall`'s fall helper | breeding and taming work without the particles; `foxes-live` opens `fall_multiplier`/`fall_reduction` instead (M1-4) |
| E1 e1b (`player-looks`) | soft: m1h | its lines in `body_render.rs` and `body_gear.rs` | parrots re-check those files after the merge |
| E1 e1b (`particles-spit-love`, commit 4) | soft: every group with feeding | `broadcast_entity_event` and, once D0 is in, an event 18 call in D0's generic interact arm (`track-e.md:284-289`) | each M1 family posts its own 18; if E1's generic call is in when a group runs, the family keeps exactly one of the two posts and logs which (M1-1, "Depends on") |
| E2 e2a (`ti1-targets-and-hurt`, `doors-zombies`) | soft: all; m1g | thrown things reach every family through `hit_boxes`; the zombie's egg work sits next to `apply_door_work` | steps that throw things at M1 mobs wait for a release with both |
| W w1 (`vibrations`, P7) | soft: all; m1i | `game_event` calls; the `player` on `Command::PlayerMines` | whichever of W and M1 merges second adds the one-line calls; bees use the nearest player within 4 blocks |
| W w2 (`warden-render`) | soft: m1f | block dust for registered mobs | sniffers land their own (M1-7) |

### The D0-3 checklist is the law

Every family follows `docs/round10/track-d0.md` D0-3 exactly; read D0-2 and D0-3 before the first commit, and where D0's merged code differs from the brief, follow the code (for example `family_goal_step`'s arguments and `MobCore.ambient_interval`). The rules that bite most:

1. **Own files only.** A family touches its new files plus one registration line per registry, all in M1's group files: `third_party/minecraftoss/entities/src/world/families/land.rs` (`pub mod` and `family_group!(land 48..=63 { .. })`), `crates/minecraft_terrain/src/families/land.rs` (`mod` and `terrain_group!(land { .. })`) and `third_party/minecraftoss/world/src/natural_spawner/families/land.rs` (`ROWS`). M1's shared helpers live under those modules too (`land/animal_goals.rs`).
2. **Never edit** D0-3's list: the per-family lists of vendored `world.rs`, `EntityKey`, `MobHit`, `world/{combat,witches,pushing,hazards,riding,villagers,leashes}.rs`, `MonsterKind`, `MonsterGoalContext`, `Prey` (target registered kinds with D0's `Prey::Kinds`; the one new variant this track adds is the hook commit `Prey::BabiesOnLand` in `turtles-hunted`, because `Prey::Kinds` cannot test "a baby, not in water"; it is logged as a hook), `lib.rs`; `server_mobs.rs` (`SIMULATED_KINDS`, `spawn_saved`, `mob_tags_where`, `VEHICLES`), `mob_tracking.rs`, `client_mobs.rs`, the `mesh.rs` texture list, `spawner_display.rs`, terrain `lib.rs`; survival `entity_view.rs`, `entities.rs`, `death.rs`; `HATCHING_EGGS`; `PROTOCOL`. The list is about registered families: M1's changes to existing families' behaviour (the cat's rabbit slot, the hunters' baby-turtle slots, the zombie's egg work in one `MonsterGoalContext` field beside `turtle_egg_next_start`, as E2's `door` field sits beside `can_break_doors`) follow rule 7 below. A missing hook goes into the root file (`world/families.rs`, terrain `families.rs`) in its own commit named for the hook and is logged. Every such commit of this track is in the table below; a hook the table does not name is logged the same way.
3. **Multiplayer.** Every drawn state through `TerrainFamily::show`/`show_on` (`FamilyShow`: a `u32` of flags and eight `i32`); animation clocks as ticks since the animation started; every saved field that changes while the mob lives in `VOLATILE`; the friend loads the family from its look tag.
4. **Attacks on players:** `PlayerHitKind::Melee`, or `Typed` (D0-2.6) for a damage type, effect or push of its own (the bee's sting, the goat's ram).
5. **Keyframes** only through `minecraft_terrain::keyframes::shared()`.
6. **Ambient sounds (track rule).** D0's `family_base_tick` rolls the ambient sound and pushes `Voice::Ambient`, which plays `entity.<voice family>.ambient`. A family whose vanilla ambient differs (fox sleep and screech, panda genes, parrot imitation, turtle on land, sniffer `idle`, polar bear cub, armadillo while scared, silent bees) rewrites or drops that voice in its own tick right after `family_base_tick`, keeping vanilla's roll. The interval is `core.ambient_interval` (D0's `MobCore` field, 80 by default; 120 for every `Animal`, 200 for turtles). M1 adds no ambient hook to D0's root; if D0's merged code has no interval field, add it in a root commit "Registered Minecraft mobs keep their own ambient sound interval".
7. **Shared files** (blocks, items, player state, effects, other families' goals): the logic in a new file of the family's, called from one line next to a related arm, never at the end of a list or enum. The anchors are in the sections and in the table of shared sites below.
8. **Tests** in the family's own files, as D0-3 lists them; the eggs survey, the hit keys and the spawner cages need nothing per family.
9. **`docs/round10/mobs.md`:** only the rows of the `## M1` section, updated in the same commit as the code (`left` → `live` → `drawn` → `done`, with the commit hashes).

### Hooks this track adds (one owner each)

Each is its own commit with the subject given, in the group named. A hook another track names the same way and lands first is used as it is, and M1 drops its copy; M1 never adds a second hook for the same need.

| Hook (commit subject) | Where | Owner group | Users |
|---|---|---|---|
| "Registered Minecraft mobs can run their own AI step": `family_goal_step_with(.., run)` | root `world/families.rs` | `rabbits-live` | rabbits, foxes, turtles; M2's swimmers, guardians and sulfur cubes |
| `land/animal_goals.rs`: goal-framework panic, tempt, breed, `MoveToBlockGoal`, `is_monster_class` (no commit of its own) | M1's group module | `rabbits-live`, `rabbits-behaviour` | polar bears, pandas, foxes, turtles, bees |
| `AvoidEntityGoal::matching` (in the rabbit commit) | `monster_ai.rs:1744-1752`, the kind test at `:1768` | `rabbits-live` | rabbits; armadillos' `hiding` skip joins the same test |
| `World::biome_in_tag` (in the rabbit breeding commit) | `player/src/lib.rs:140`, `server_mobs.rs:236` | `rabbits-live` | rabbits; M2's tadpoles under the same name |
| "Registered Minecraft mobs can land their melee attacks their own way": `MeleeAttackGoal::tick_with`, `CheckedMeleeAttackGoal` | `monster_ai.rs:2239`, `:2308` | `polar-bears-live` | polar bears |
| H1 "Registered Minecraft mobs can hold still while their goals run": `MonsterAi.still` | `monster_ai.rs:813`, `world.rs:1064` | `pandas-eating` | pandas, foxes |
| H2 "Registered Minecraft mobs can scatter item crumbs": `ClientFx.item_crumbs` | terrain root `families.rs`; one line where D0's `advance` builds the `ClientFx` (survival `entity_view.rs`, the hook's only touch of that never-edit file, logged as a hook) | `pandas-eating` | pandas, foxes; M2's sulfur cube |
| H3 "Living Minecraft mobs can drop gifts from loot tables": `family_gift`, `take_family_gifts`, `roll_table`, `drop_family_gifts` | root `families.rs`, `loot.rs:276`, `server.rs:1107` | `pandas-moods` | pandas, armadillos, sniffers, turtles |
| H4 "Minecraft mobs know when it rains and thunders": `World::raining`, `World::thundering`, `Level::is_thundering` | `player/src/lib.rs:189`, `level/fire.rs:65`, `server_mobs.rs:357` | `pandas-thunder` | pandas, foxes, bees |
| "Registered Minecraft mobs can be spared by the blocks they walk through": `Family::spared_blocks` | root, `hazards.rs:22`, `:90`, `inside_blocks.rs:472`, `survival.rs:599` | `foxes-live` | foxes, bees |
| `LookControl::tick_from(.., reset_pitch)` (in the fox commit) | `look.rs:144` | `foxes-live` | foxes; M2's frogs if it is there |
| Fall factors: E1's `horse-fall` helper, else `fall_multiplier`/`fall_reduction` made `pub(crate)` (in the fox commit) | `living.rs:55`, `:65` | `foxes-live`, only if e1a is not in | foxes, goats |
| "Mobs can show a block's break effect without breaking it": `World::destroy_effect` | `player/src/lib.rs:98`, `server_mobs.rs:258` | `foxes-hunt` | foxes (later the rabbit's nibble and turtle digging) |
| "Minecraft animals share vanilla's brain behaviours for panic, temptation, love and following adults": `world/families/animal_brain.rs`, one `pub mod` line in the root | root `families.rs` | `goats-live` | goats, armadillos, sniffers; M2's axolotls, frogs and nautiluses |
| "Minecraft mobs can long-jump like vanilla goats and frogs": `world/families/long_jump.rs`, one `pub mod` line in the root | root `families.rs` | `goats-leap` | goats; M2's frogs |
| `MeleePush.blocked` and the `#no_anger` check (in the ram commit) | `impulse.rs:38`, `world.rs:3476` | `goats-ram` | goats |
| "Minecraft mobs can tell when a player sprints": `PlayerControls.sprinting` | `controls.rs:2`, survival `entities.rs:129`, `:151`, `minecraft_players.rs:1294`, `minecraft_world.rs:1134` | `armadillos-live` | armadillos; M3's camels |
| "Registered Minecraft mobs cast their own shadow size": `TerrainFamily::shadow_radius` | terrain root `families.rs` | `armadillos-drawn` | armadillos, turtles; M2's fish |
| "Mobs that avoid others ignore registered mobs that are hiding": `MobCandidate.hiding` | `monster_ai.rs:341` and its five literals | `armadillos-spiders` | armadillos |
| "Registered Minecraft mobs throw up block dust": `ClientFx.dust` | terrain root, `entity_view.rs:58`, `minecraft_world.rs:1611` | `sniffers-dig` | sniffers; W's warden |
| "Minecraft loot tables know what killed a mob": `EntityLootContext.damage_type`, `damage_source_properties` by key | `loot.rs:19`, `:636-646` | `turtles-live` | turtles; M2's frogs add one arm |
| "Minecraft mob targets can pick babies on land": `Prey::BabiesOnLand` | `monster_ai.rs:415`, beside D0's `Prey::Kinds` arm | `turtles-hunted` | the turtle's hunters |
| `WalkTarget::Turtle` and `WalkTarget::Air` (in the family commits) | `walk_path.rs:44-60`, `stroll.rs:14-44` | `turtles-live`, `bees-live` | turtles, bees |
| The d0b flying wiring for a goal family (the navigation in `GoalExtra` if d0b's is walk-only) | the parrot files | `parrots-live` | parrots, bees |
| `World::beehive_occupants`, `World::gameplay_flag` (in the hive commit) | `player/src/lib.rs:238`, `server_mobs.rs:262` | `bees-hives` | bees; M3's creakings may reuse `gameplay_flag` |
| "Minecraft ticking block entities tick again after their chunk loads" | `level/mod.rs:318` (`insert_chunk`), `container.rs:475` | `bees-hives` | hives, and loaded hoppers too |

### Shared sites outside the registry, and who owns which line

The other M tracks' drafts touch some of the same files. M1 keeps to the anchors below; where another track needs the same thing, the owner named here adds it and the other uses it (or, if it lands first, the other's copy is used and M1 drops its own).

| File and anchor | M1's change (group) | Other M track nearby | Rule |
|---|---|---|---|
| `leash_render.rs:72-80` `leash_offset` | new rows after `"ocelot"` (`:76`): fox, parrot, bee; after `"iron_golem"` (`:77`): rabbit | M3's allay row after `"pig" \| "wolf" \| "strider"` (`:78`) | M1 only adds rows and never touches `:78` |
| `player/src/lib.rs` `World` | `destroy_effect` after `destroy_block` (`:98`), `biome_in_tag` after `biome` (`:140`), `raining`/`thundering` after `rain_at` (`:189`), `beehive_occupants`/`gameplay_flag` after `block_in_tag` (`:238`) | M2's tadpoles add `biome_in_tag` (same), M2's dolphins `nearest_structure` | M1 owns `biome_in_tag`; the other M tracks take other anchors |
| `server_mobs.rs` `MobWorld` | each impl right after the existing one it pairs with (`:236`, `:258`, `:262`, `:357`) | M2's `biome_in_tag` | as above |
| `monster_ai.rs:1287`, `:1309`, `:1948`, `:1965`, `:1971`; `feline_ai.rs:196`, `:200`, `:244`; `wolf_ai.rs:210` | the rabbit and turtle target slots, `RemoveEggGoal` | M2's axolotls (a drowned slot), M3's phantoms (cats scare phantoms) | M1 owns these lines; M2's drowned-only slot goes into its own list, M3's phantom lines stay off them |
| `monster_ai.rs:1735-1770` `AvoidEntityGoal` | `matching` beside `also`; the `hiding` skip in the kind test | M2's dolphins, fish and frogs build `AvoidEntityGoal::new(..)` | constructor changes only, so no literal breaks |
| `monster_ai.rs:2239` `MeleeAttackGoal` | `tick_with` and the wrapper, no new field | M2's dolphins add a `MeleeAttackGoal { .. }` literal | the wrapper keeps every literal valid |
| `monster_ai.rs:341` `MobCandidate` | `hiding` (five literals) | no other brief adds a literal | a literal that shows up at a merge gets `hiding: false` |
| `loot.rs:16-34`, `:636-646` | `damage_type` after `has_direct_attacker` (`:19`); the per-key `damage_source_properties` | M2's frogs: `attacker_variant` after `attacker` (`:28`), one `minecraft:components` arm | M1 owns the restructure, M2 adds one arm |
| `loot.rs:182`, `:276` | the `brush/` tables line (`armadillos-scutes`); `roll_table` (`pandas-moods`) | none | |
| `movement.rs:275`; `walk_path.rs`, `navigation.rs` amphibious branches | none, unless M2's P2 is still missing at m1g | M2's P2 | M2 owns them |
| `walk_path.rs:44-60` `WalkTarget`, `stroll.rs:14-44` | `Turtle` right after `Animal` (`walk_path.rs:49`, its arm after the `Animal` arm, `stroll.rs:16-19`); `Air` right after `Monster` (`:51`, its arm after `stroll.rs:20`). Neither at the end of the enum or the match (`Hoglin` is last) | M2's `Guardian` beside `Strider` | distinct anchors |
| `stroll.rs:302-315` | `air_random_position_towards` right before `hover_random_position` (`:302`) | M3's allays add theirs right after `:315` | distinct anchors |
| `world/src/level/mod.rs:19-27` module lines | `sniffer_egg`, `turtle_egg`, `beehive` after `wither_build` (`:21`) | M3's `lightning` after `fire` (`:19`), M2's `frogspawn` after `fluid` (`:27`) | distinct anchors |
| `level/mod.rs` `on_place` (`:677`) and `tick_block` (`:839`) | the sniffer egg arm after `wet_sponge` (`:690`) and after `FireBlock` (`:842`) | M2's frogspawn after `FrostedIceBlock` in `on_place` (`:705`) and after the coral arm in `tick_block` (M2 owns those; M3 holds the `FrostedIceBlock` line of `tick_block`); M3's creaking hearts and dried ghasts | M3 takes anchors other than these |
| `level/mod.rs` fields (`:196-218`) | `hive_alarms` after `bell_rings` (`:208`) | M3's `lightning_hits` after `level_events` (`:207`) | distinct anchors |
| `level_events.rs:8`, `:128-136` | 3009 (`sniffer-eggs`), then 2011 (`bees-pollination`) beside `BONE_MEAL_USE` | M3's lightning 3002 | M3 anchors elsewhere |
| `random_tick.rs:299` | the turtle egg after `CactusBlock` | none | |
| `server.rs:1105-1107` (after the mob tick) | `drop_family_gifts`, the turtle egg pass, `bees_to_level`, in that order, after `drop_cat_gifts` | none known | M1 owns the lines after `drop_cat_gifts` |
| `server.rs` `ItemOnBlock` (`:31-33`), `item_on_block_by` (`:2485-2503`), `Command` (`:3397-3462`) | `HarvestHive` next to `FillBucket`, its arm inside `item_on_block_by`; `ReleaseShoulder` beside `Reattach` (`:3454`) | M2's `Command::EmptyMobBucket` beside `ItemOnBlock` (`:3408`), `empty_mob_bucket` beside `:2485` | distinct anchors |
| `ambient_particles.rs:42` `Kind` | `Sneeze` after `Poof` (`:48`) | M2's dolphin and guardian particles | M2 takes other neighbours |
| `controls.rs:2` `PlayerControls`, survival `PlayerView` | `sprinting` (`armadillos-live`) | M3's camels | M1 owns the field; M3 adds only its ride-input line |
| terrain root `families.rs` `ClientFx` | `item_crumbs` (H2), `dust` | M2's sulfur cube planned a `crumbs` field | one field: M1's `item_crumbs` |
| `emissions.rs:64` | none (bees use M2's `quiet_steps`) | M2's `water-hooks` | M2 owns |
| `impulse.rs:38` `MeleePush` | `blocked` and its six literals (`goats-ram`) | M2's nautiluses push through `PlayerHitKind` | a literal that shows up at a merge gets `blocked: 1.0` |
| `inside_blocks.rs`, `hazards.rs` | `spared_blocks` | M3's `LightningBolt` hazard (`inside_blocks.rs:43`, `:54`, `:74`) | distinct lines |
| `dispenser.rs:93-95` | the optional hive harvest arm after the `"honeycomb"` arm | M2's fish buckets beside `"water_bucket" \| "lava_bucket"` (`:93`) | distinct anchors |

Tell M2 and M3 (through the orchestrator) the owner names above that their drafts name differently: M1 adds no ambient hook (M2's `Family::ambient`, M3's `ambient_sound`); crumbs are `ClientFx.item_crumbs`; the shadow hook is `TerrainFamily::shadow_radius`; the sprint flag is M1's `PlayerControls.sprinting`.

### Protocol

`minecraft_net::PROTOCOL` is never bumped by this track (`docs/round10/decisions.md`: the release step bumps it once when messages changed). D0's `FamilyShow` carries every family's shown state, so most groups change no serialized struct. The one that does is `parrots-shoulders-seen` (`GearView.shoulders`); its commit adds a line under "## Protocol" in the track log, and so does any other commit that changes a `ToClient`/`ToServer` message or the codec.

### Track log, docs and decisions

- `docs/round10/track-m1-log.md`: the commits, the javap numbers each group used, decisions (with the open decisions of each section as taken), the vendored files added and patched (for the CLAUDE.md count), "## Protocol", "State doc changes", "Left for later", and each group's owner steps and kit line (at most 27 stacks, in its own code block).
- `docs/round10/mobs.md`: only the M1 rows (15-22, 44, 46), in the same commit as the code.
- Never edit `CLAUDE.md`, `docs/MINECRAFT-SURVIVAL.md`, `docs/MULTIPLAYER.md`, `docs/PROGRESS.md` or `docs/round10/decisions.md` from the track run: write their new wording under "State doc changes" in the log; the orchestrator copies decisions into `decisions.md` at the merge.

### Rules

One commit per item, with its tests. `check -p` and `test -p` on the crates each commit touches (`minecraftoss-entities`, `minecraftoss-world`, `minecraftoss-player`, `minecraft_terrain`, `minecraft_survival`, `minecraft_net`, `render_anim`), and `check -p launcher` before a commit that touches game code. Clippy on the touched crates once per segment, `rustfmt --check` on new files' staged content, no code comments of any kind, cargo only through `scripts\cargo-slot.ps1` (gentle mode when the owner plays).

### Combined kit

One kit for a first look at every M1 family in one world (27 stacks). Each section's own kit is for its full steps.

```
IW4L_MINECRAFT_KIT=minecraft:rabbit_spawn_egg*16,minecraft:polar_bear_spawn_egg*8,minecraft:panda_spawn_egg*16,minecraft:fox_spawn_egg*16,minecraft:goat_spawn_egg*16,minecraft:armadillo_spawn_egg*16,minecraft:sniffer_spawn_egg*16,minecraft:turtle_spawn_egg*16,minecraft:parrot_spawn_egg*16,minecraft:bee_spawn_egg*16,minecraft:carrot*64,minecraft:bamboo*64,minecraft:sweet_berries*64,minecraft:wheat*64,minecraft:spider_eye*32,minecraft:torchflower_seeds*64,minecraft:seagrass*64,minecraft:wheat_seeds*64,minecraft:poppy*64,minecraft:brush,minecraft:beehive*4,minecraft:sniffer_egg*8,minecraft:bucket*4,minecraft:lead*4,minecraft:spawner,minecraft:iron_sword,minecraft:cooked_beef*32
```

Every id checked against `assets/minecraft/items` in the 26.3 jar. An egg of a family that has not landed yet spawns nothing.

---

## M1-1. Rabbits (segment m1a: groups `rabbits-live`, `rabbits-drawn`, `rabbits-behaviour`)

Family `rabbits`, group `land`, hit-key byte **48** (`Family::KEY`, row 15 of `docs/round10/mobs.md`). One kind, `minecraft:rabbit`; the killer bunny is `RabbitType` 99 of it. Lines are `minecraft-survival` at `d1a3d028`; re-check before editing. D0 names follow `track-d0.md`; where D0's code differs, follow the code.

Rabbits run first because they create the track's first shared pieces (M10, "Hooks this track adds"): the root hook `family_goal_step_with`, the new `land/animal_goals.rs` (goal-framework animal goals, `MoveToBlockGoal`, `is_monster_class`), `AvoidEntityGoal::matching` and `World::biome_in_tag`.

### What exists today

- **Dormant.** `minecraft:rabbit` is not in `SIMULATED_KINDS` (`crates/minecraft_terrain/src/server_mobs.rs:433`), so `spawn_saved` (`:580`) returns `None`. Worldgen and saved rabbits stand frozen, and the egg spawns nothing (`docs/MINECRAFT-SURVIVAL.md:441`, `:451`). `tracking_range` (`mob_tracking.rs:285`) gives the default 5.
- **Spawning already matches 26.3, so no world row is added** (D0-4):
  - the size row `natural_spawner.rs:282` (0.49 x 0.6, `OnGround`, `MotionBlockingNoLeaves`) and the rule `:332` (`#rabbits_spawnable_on`, light above 8);
  - creature category (`natural_spawner/spawning.rs:102`) and follow range 16 (`:397-408`);
  - finalize `natural_spawner.rs:972-999`: white 80 / white-splotched 20 in `#spawns_white_rabbits`, gold in `#spawns_gold_rabbits`, else brown 50 / salt 40 / black 10. `Group::Rabbit` (`:1118`) keeps the first rabbit's type, and `ageable` chance 1.0 (`:1137`) makes every later group member a baby. The tags load at `:550-551`.
- **Mobs that already name rabbits:**
  - Wolves hunt `Prey::WolfPrey` (`monster_ai.rs:2961`, `wolf_ai.rs:209`) once rabbits are in `mob_candidates`.
  - The cat's rabbit slot is still `Prey::Absent` (`feline_ai.rs:194-197`; javap `Cat.registerGoals`: target 1 is `NonTameRandomTargetGoal(Rabbit)`, then turtles at `:198-201`).
- **Reuse:**
  - Goals: `FloatGoal` `monster_ai.rs:2015`, `MeleeAttackGoal` `:2239`, `StrollGoal::new(speed, probability)` `:2332`, `LookAtPlayerGoal` `:2366`, `AvoidEntityGoal::new(..).also(..)` `:1735-1752`, `NearestTargetGoal` `:2909`, `HurtByTargetGoal { alert_others }` `:3044`, `is_enemy` `:364`.
  - Animal helpers: `PanicState` `panic.rs:9`, `TemptState` `tempt.rs:30`, `BreedState` `breed.rs:7`, `animal::interact` `animal.rs:12` (love, growth, golden dandelion lock), `age::Age`, `feline_ai::find_nearest_block` `:715` (`MoveToBlockGoal.findNearestBlock`), `crops::max_age` `crops.rs:9`.
  - Own AI steps instead of `monster_ai_step` (`world.rs:1022-1097`): slimes `world/slimes.rs:178`, felines `world/felines.rs:241-265`.
  - Breeding: `world/felines.rs:574`; hearts (event 18) `:861`; love-tick randoms `:826-835`; the orb `world/hoglins.rs:645-647`.
  - Food tags: `world/felines.rs:358-374` (`recipes.item_in_tag`, `controls.holds`), `mob_actions.rs:194`.
  - A goal's block change applied by the tick: the sheep, `world.rs:7430-7437`.
  - Baby model and `_baby` textures: `chicken_render.rs:121-239`. Keyframes from a start tick: `bat_render.rs:163`.
  - Names: `illager_save.rs:33` `custom_name`. Lead offsets: `leash_render.rs:72-82` (rabbit gets the default `(1.0, 0.4)`; vanilla is `(0.6, 0.4)`).

### Vanilla 26.3 (javap this round)

- **`EntityTypes.RABBIT`:** `CREATURE`, `sized(0.49, 0.6)`, `eyeHeight(0.59)`, `clientTrackingRange(8)`; not fire immune, allowed on peaceful. A baby's `getDefaultDimensions` is `scalable(0.24, 0.4).withEyeHeight(0.39)`, replacing the usual 0.5 age scale.
- **Attributes:** `createAnimalAttributes` plus health 3, speed `0.30000001192092896`, attack 3.
- **Experience:** 1 + `nextInt(3)` for adults (D0 default).
- **Sounds:** `entity.rabbit.ambient`, `.hurt`, `.death`, `.jump`, `.attack` (`playAttackSound`, `EVIL` only, pitch `(r1 - r2) * 0.2 + 1`). Source `NEUTRAL`, `HOSTILE` for `EVIL`. No step override.
- **Other:** `getLeashOffset` `(0, 0.6 * eye, 0.4 * width)`. Saved: `RabbitType` (int), `MoreCarrotTicks`, `Age`, `ForcedAge`, `AgeLocked`, `InLove`.
- **`Variant`:** `BROWN` 0, `WHITE` 1, `BLACK` 2, `WHITE_SPLOTCHED` 3, `GOLD` 4, `SALT` 5, `EVIL` 99; an unknown id gives `BROWN`.
- **Constants:**
  - speeds: stroll 0.6, breed 0.8, tempt 1.0, flee 2.2, attack 1.4;
  - jumps: height 1.5 adult / 0.5 baby; delay 10, or 3 when fleeing; duration 15;
  - idle tilt every `180 + nextInt(40)` ticks; carrots delay 40;
  - evil: attack +5 (`minecraft:evil`), armour 8.
- **Goals** (`registerGoals`, priority in insertion order):
  - 1 `FloatGoal`, 1 `ClimbOnTopOfPowderSnowGoal`, 1 `RabbitPanicGoal(2.2)` (`PanicGoal` plus `setSpeedModifier` every tick);
  - 2 `BreedGoal(0.8)`, 3 `TemptGoal(1.0, #rabbit_food, false)` (carrot, golden carrot, dandelion);
  - 4 `RabbitAvoidEntityGoal` for `Player` (8), `Wolf` (10) and `Monster` (4), all at 2.2/2.2 and never for `EVIL`;
  - 5 `RaidGardenGoal`, 6 `WaterAvoidingRandomStrollGoal(0.6)`, 11 `LookAtPlayerGoal(Player, 10)`.
  - `Monster` means `instanceof Monster`. These `Enemy` kinds are not: ghast, slime, magma cube, hoglin, shulker, phantom, ender dragon. Blaze, endermite, silverfish, guardian, vex, zoglin, ravager, breeze, creaking and warden are.
- **`setVariant(EVIL)`** (from finalize and load, after `registerGoals`):
  - armour 8 and attack 8 (`mob_attack`: 5 Easy, 8 Normal, 12 Hard);
  - goal 4 `MeleeAttackGoal(1.4, true)`;
  - targets 1 `HurtByTargetGoal().setAlertOthers()`, 2 `NearestAttackableTargetGoal(Player, true)`, 2 the same for `Wolf`;
  - `CustomName` = translatable `entity.minecraft.killer_bunny` when unnamed.
- **Movement:** a rabbit moves only while airborne.
  - `customServerAiStep` runs after goals and navigation, before the move control:
    - `jumpDelayTicks` counts down, and `moreCarrotTicks -= nextInt(3)` while above 0.
    - On landing, `setJumping(false)` and the landing delay (10, or 3 at speed 2.2 or more); the jump control is turned off.
    - `EVIL` with no delay and a target within 4 blocks (`distSqr < 16`): face it, set the wanted position to it, `startJumping`.
    - Otherwise, with no jump wanted, a wanted move and no delay: face the path's next point (or the wanted point) and `startJumping`. If a jump is wanted, turn the jump control back on.
    - `facePoint`: `yRot = atan2(dz, dx) * 57.2957763671875 - 90`.
  - `RabbitMoveControl.tick`: on the ground, not jumping and no jump wanted → `setSpeedModifier(0)`; with a wanted move or `JUMPING` → `setSpeedModifier(nextJumpSpeed)`. `setWantedPosition` forces 1.5 in water and remembers any speed above 0. `setSpeedModifier(s)` sets the navigation and move control speed.
  - `RabbitJumpControl.tick`: a wanted jump calls `startJumping`. That plays `entity.rabbit.jump` at pitch `((r1 - r2) * 0.2 + 1) * 0.8` and sets duration 15, ticks 0. `aiStep` counts the ticks up, then resets them with `setJumping(false)`.
  - `getJumpPower`: 0.3; 0.2 at speed ≤ 0.6; 0.5 if the next path point, or the wanted y while jumping, is more than 0.5 up, or on a horizontal collision. Then `getJumpPower(f / 0.42)` (javap `LivingEntity.getJumpPower(F)` during the work).
  - `jumpFromGround`: with speed above 0 and horizontal speed² below 0.01, `moveRelative(0.1, (0, 1.5 | 0.5 baby, 1))`; then entity event 1. `canSpawnSprintParticle` is false.
- **`RaidGardenGoal`** (`MoveToBlockGoal(0.699999988079071, 16)`, vertical 1):
  - `canUse` needs `mob_griefing` and `moreCarrotTicks <= 0` (`wantsToRaid`). It looks every `reducedTickDelay(200 + nextInt(200))`.
  - Target: `#supports_crops` (farmland) with a full-grown carrot on it; finding it sets `canRaid`.
  - **`canContinueToUse` asks `isValidTarget` again, which refuses once `canRaid` is set**, so the goal stops at the next full selector update. The walk it started goes on (`MoveToBlockGoal` has no `stop`), and a later start right beside the crop eats at once. Port it as is.
  - Reached (within 1.0 of the crop's centre): age 0 → air, else age − 1 (flag 2) with game event `block_change` and level event 2001. Then `moreCarrotTicks = 40` and `nextStartTick = 10`.
- **Breeding:**
  - The baby's colour: `nextInt(20) == 0` gives the biome's colour; otherwise `nextBoolean` picks between the two parents.
  - `spawnChildFromBreeding`: event 18 on the parent, parents' age 6000, and an orb of `nextInt(7) + 1` only with `mob_drops`.
- **Client animation** (client `baseTick`):
  - The idle timeout runs out (not leashed, AI on): start the head tilt and roll a new timeout.
  - Otherwise, `jumpTicks > 0` starts the hop if stopped and stops the tilt.
  - Otherwise the timeout counts down and the hop stops. A lead stops the tilt.
- **Render:**
  - `RabbitRenderer`: `AgeableMobRenderer(AdultRabbitModel, BabyRabbitModel, shadow 0.3)`; the baby model swaps in unscaled.
  - 16 textures: `entity/rabbit/rabbit_{brown,white,black,white_splotched,gold,salt,caerbannog,toast}` and each `_baby`. `EVIL` uses caerbannog, and the name `Toast` overrides the variant.
  - The default `EntityModel` render type in 26.3 is `entityCutout` (javap), so the model goes to `out.culled`.
  - Adult model (64 x 64): `body` (`tail`, `head` (2 ears), `frontlegs` (2 legs)) and `backlegs` (2 hind legs, each with a haunch); 13 parts, 9 cubes. Baby model (32 x 32): 17 parts with `*_r1` rotation parts. Port both from javap.
  - `setupAnim`: the head follows the look only while the tilt is **not** started. A finished non-looping state stays started until a hop stops it.
  - Animations: `RabbitAnimation`/`BabyRabbitAnimation` `HOP` (0.75 s, loops) and `IDLE_HEAD_TILT` (4 s).
- **Spawns:**
  - Rule: `ON_GROUND`, `checkRabbitSpawnRules` (grass block, snow, snow block or sand below; bright enough).
  - Weights: desert 12; snowy plains and ice spikes 10; grove 8; the four taigas, snowy slopes, flower forest and dappled forest 4; meadow and cherry grove 2 (groups of 2-6).
- **Loot `entities/rabbit`:** hide 0-1; one raw rabbit, smelted if it dies burning; a foot at 10% when killed by a player. Babies drop nothing.

### Plan

**Files (D0-3):**
- `entities/src/world/families/land/rabbits.rs`: the `Rabbit` state (variant, `Age`, love, carrot ticks, jump fields, `MonsterAi`) with `impl Family`, the tick, breeding, the feed function and the tests.
- `.../land/rabbits/ai.rs`: the `MonsterSpec`, the rabbit-only goals (`RabbitAvoidEntityGoal`, `RaidGardenGoal`), the evil gate and `rabbit_ai_step`.
- `.../land/animal_goals.rs` (new; `pub(crate) mod animal_goals;` in `world/families/land.rs`): the goal-framework animal goals that later goal families reuse, ported over `PanicState`/`TemptState`/`BreedState` (panic with a speed and a cause rule, tempt by item tag, breed with a child callback), and `is_monster_class` (decision 3). `rabbits-behaviour` adds the general `MoveToBlockGoal` port here (over `find_nearest_block`, `feline_ai.rs:715`, and the cat's shape at `feline_ai.rs:951-1077`); `RaidGardenGoal` is one. Polar bears, pandas, foxes, turtles and bees add what they port to this file and reuse what is there; nothing in it is rabbit-only.
- `crates/minecraft_terrain/src/families/land/rabbits.rs` (`TerrainFamily`) and `.../rabbits/render.rs`.
- Registration lines in `world/families/land.rs` and terrain `families/land.rs`. No world row.

**Hooks (D0-2.2):**
- `KEY` 48; `core.ambient_interval` 120 (vanilla `Animal`); `category` `"creature"`; `armor` 8 for evil; `voice` `("rabbit", 1.0, "neutral" | "hostile")`; `leashable` with `leash_walk`; `panicking` while the panic goal runs; `monster` → the spec. Everything else uses the defaults.
- Tick: `family_base_tick`, the goal step (`family_goal_step_with` running `rabbit_ai_step`), `family_after_move`. The killer bunny bites through `family_strike` (`Melee`, 8). Babies come from `spawn_family`.
- `MonsterSpec`: eye 0.59, follow range 16. `walk` sets `can_float`, puts back the animal fire maluses that `of_kind` clears (`monster_ai.rs:842-845`) and sets `WalkTarget::Animal`.
- `GoalExtra` carries `RabbitGoals { evil, wants_more_food, eaten, speed_modifier }`.

**AI step (decision 1).** D0's `family_goal_step` (merged code: `family_goal_step(id, level, players, movement_speed, ai)`) always calls the fixed `monster_ai_step` and then applies the generic fall damage to the distance it returns. The rabbit needs its custom step between the navigation and the move control, its own move and jump controls, and its own `jumpFromGround`. `rabbit_ai_step` mirrors `world.rs:1022-1097`, keeping `play_movement` (`:1094`) so W's step events come for free.
- `rabbits-live` therefore starts with the root hook commit "Registered Minecraft mobs can run their own AI step": `family_goal_step_with(.., run)` in `world/families.rs`, the same core sync around a `run` that has `monster_ai_step`'s signature and returns the landed fall distance; `family_goal_step` becomes a call of it with `monster_ai_step`. Skip the commit only if D0's merged code already lets a family swap the step.
- Users: turtles (`turtle_ai_step`), foxes (their 5-block safe fall), and M2's swimmers and guardians, which add it under this exact name and subject if they run first (the second track to merge drops its copy).

**One-line hooks in shared files:**
- `monster_ai.rs:1737`/`:1749`/`:1769`: `AvoidEntityGoal::matching(fn(&str) -> bool)` beside `also`. The rabbit passes `is_monster_class` = `is_enemy(kind)` minus the non-`Monster` kinds listed above (decision 3).
- `player/src/lib.rs:140`: `fn biome_in_tag(&self, _pos, _tag) -> bool { false }` after `fn biome`. The server implements it after `server_mobs.rs:236`, through `registries.biome_tags` (decision 2). M2's tadpoles need the same method at the same lines: M1 owns it under this name and signature; if M2's copy lands first, use it and drop this part.
- `leash_render.rs:77`: a new line `"rabbit" => (0.6, 0.4),` after the `"iron_golem"` row, not after the pig/wolf/strider row (`:78`), where M3's allay line goes.
- `feline_ai.rs:196`: `Prey::Absent` becomes `Prey::Kinds(&["minecraft:rabbit"])`.
- W's `block_change` for a nibble: one `game_event` line where the tick applies the crop change, added by whichever of W and `rabbits-behaviour` merges second.

**Multiplayer:**
- `TRACKING_RANGE` 8. `VOLATILE` `["MoreCarrotTicks", "ForcedAge"]` (`Age` and `InLove` are in the base list).
- `read`/`fields`: the saved keys above, plus `Baby` from a look tag. `toast` comes from the name, and an unnamed evil rabbit gets its `CustomName`.
- `FamilyShow`: flag bits 0 (hop running) and 1 (tilt started); `values[0..2]` = ticks since each started.
- `interact`: `#rabbit_food` feeds it and returns true; the entity side pushes event 18.
- Variant, toast and size reach friends through the look tag and `core.baby`.

**Keyframes:** `keyframes::shared().definition("RabbitAnimation" | "BabyRabbitAnimation", "HOP" | "IDLE_HEAD_TILT")`, applied as in `bat_render.rs:163`.

**Commit groups**

| Group | Commits | Contents |
|---|---|---|
| `rabbits-live` | 1. "Registered Minecraft mobs can run their own AI step" (root hook) 2. "Minecraft rabbits hop about, flee and follow carrots in the entity world" 3. "Minecraft rabbits breed and their babies grow up" | Entities crate, `animal_goals.rs`, `AvoidEntityGoal::matching`, `World::biome_in_tag`. No terrain registration yet, so in-game rabbits stay dormant and no half state reaches a build. **Check first:** if D0's merged `is_simulated`, eggs survey (`server.rs`, D0-5.1) or cage test enumerate the entity-side `families::kinds()` rather than the terrain families, an entity-only registration fails them (the survey expects a rabbit egg to hatch, and `test -p minecraft_terrain` runs here because of `biome_in_tag`). Then commit 3 also adds the `TerrainFamily` with save, load, show, `interact` and an empty `draw`, as every other M1 family's live group does, and `rabbits-drawn` 1 keeps the models, textures and lead line. Log which way it went. |
| `rabbits-drawn` | 1. "Minecraft rabbits are saved, hatch from eggs and are drawn like vanilla" 2. "Minecraft rabbits hop and tilt their heads with the vanilla animations, for friends too" | `TerrainFamily`, both models, 16 textures, the lead line; then the host-side animation clock (decision 4), keyframes and `FamilyShow`. Row → `drawn`. |
| `rabbits-behaviour` | 1. "Minecraft rabbits raid carrot crops" 2. "The killer bunny bites players and wolves" 3. "Minecraft cats hunt rabbits" | Raid goal on the new `MoveToBlockGoal` in `animal_goals.rs`; the evil goals (decision 5), armour, sounds, leap, name; `feline_ai.rs:196`. Row → `done`; the steps and kit go in the log. |

### Tests

- **`rabbits-live` 1:** `families::tests::a_registered_mob_runs_its_own_ai_step_with_its_core_in_sync` (with the probe: hurt-by, look and yaw synced; the fall damage uses the distance `run` returns).
- **`rabbits-live` 2:**
  - `a_rabbit_is_small_with_three_health_and_its_kind`: sizes and eyes for adult and baby, census, `mob_candidates`, the table for adults only, 1-3 experience.
  - `a_rabbit_moves_only_while_it_is_in_the_air`: jump sound pitch 0.64-0.96, 15 ticks, delay 10 or 3.
  - `a_hop_rises_and_pushes_forward_like_vanilla`: powers 0.2/0.3/0.5, the adult and baby push.
  - `a_rabbit_faces_its_next_path_point_before_each_hop`.
  - `a_rabbit_flees_players_wolves_and_monsters_but_not_slimes`: 8/10/4 blocks, creative ignored.
  - `a_hurt_rabbit_panics_at_2_2`.
  - `a_rabbit_follows_a_player_holding_a_carrot_golden_carrot_or_dandelion`.
  - `a_rabbit_in_water_floats_up_and_hops_at_1_5`.
  - `a_wild_wolf_hunts_a_rabbit`.
  - World: `rabbits_spawn_on_grass_sand_and_snow_in_daylight_as_one_adult_and_babies_of_its_biome_colour`, which pins the built-in row.
- **`rabbits-live` 3:**
  - `two_fed_rabbits_have_a_baby_that_grows_up`: love 600, event 18, ages, an orb only with `mob_drops`, growth by a tenth.
  - `a_baby_takes_a_parent_s_colour_or_one_time_in_twenty_its_biome_s`.
  - `a_golden_dandelion_locks_a_baby_rabbit_s_age`.
- **`rabbits-drawn` 1:**
  - `a_rabbit_saves_and_loads_with_its_type_carrot_ticks_and_age`: 0-5, 99 and unknown → brown; `AgeLocked`; the bunny's name.
  - `a_friend_sees_a_rabbit_from_its_look_tag` and `a_rabbit_eats_a_carrot_from_the_hand_and_falls_in_love` are the first real-family tests of D0's `FamilyShow` values and `interact` arm, which D0-9.4 leaves to the first track that uses them.
  - `render::tests::an_adult_rabbit_has_thirteen_parts_on_64_and_a_baby_seventeen_on_32`.
  - `every_rabbit_type_picks_its_texture_and_toast_wins`.
  - `a_rabbit_s_lead_ties_at_its_neck`.
- **`rabbits-drawn` 2:**
  - `a_rabbit_hops_with_the_hop_keyframes_and_tilts_its_head_when_idle`: gated on `MINECRAFTOSS_ROOT`.
  - `the_tilt_waits_180_to_219_ticks_stops_on_a_hop_and_never_runs_on_a_lead`.
  - `a_friend_sees_a_rabbit_hop_and_tilt_at_the_same_ticks`.
- **`rabbits-behaviour`:**
  - `animal_goals::tests::move_to_block_searches_in_vanilla_order_and_stops_within_reach`.
  - `a_rabbit_walks_to_a_ripe_carrot_and_nibbles_it_down_to_nothing`: stages, the 40 ticks, the early stop and the later start.
  - `no_rabbit_touches_crops_without_mob_griefing`.
  - `a_killer_bunny_has_eight_armour_and_bites_for_eight`: 5/8/12, never flees, hostile sounds, the name only when unnamed.
  - `a_killer_bunny_leaps_at_a_target_within_four_blocks`.
  - `a_killer_bunny_attacks_a_wolf_and_its_kin_join_in`.
  - `a_wild_cat_hunts_a_rabbit_and_a_tame_cat_does_not` (in `world/felines.rs`).

### Owner's steps (survival, Normal, by day on flat grass; `/gamerule mob_griefing true`; the kit below)

1. Use the rabbit egg three times on grass 8 blocks away. Small rabbits move only in hops, with a soft jump sound each time. Each hop tucks the back legs and stretches the body. **rabbit1_hop**
2. Walk at them: within about 8 blocks they flee in quick hops. Hold a carrot and stand still: they come and sit at your feet.
3. Feed two adults a carrot each: a baby appears, smaller, with a big head and its own texture. Feed the baby dandelions: it grows faster. With E1 merged, hearts rise from each fed rabbit. **rabbit3_baby**
4. Stand near one for about 10 s: it tilts its head for 4 s, then stops turning its head to you until its next hop.
5. `/summon minecraft:rabbit ~ ~ ~3 {RabbitType:1}`, then the same with 2, 3, 4 and 5: white, black, white with black patches, gold, and salt-and-pepper. `/summon minecraft:rabbit ~ ~ ~3 {CustomName:"Toast"}`: Toast. **rabbit5_colours**
6. Hoe a 5 x 5 patch by water, plant carrots and bone-meal them full, and fence three rabbits in. Within a few minutes a rabbit hops onto a carrot; each bite shrinks it a stage until it is gone. **rabbit6_carrots** Then `/gamerule mob_griefing false` and replant: no more bites. `/gamerule mob_griefing true`.
7. `/summon minecraft:rabbit ~ ~ ~6 {RabbitType:99}`: a white rabbit with red eyes leaps at you from 4 blocks and bites for 4 hearts (less in iron armour). It never flees. Use a wolf egg 5 blocks away: the two fight. **rabbit7_killer_bunny**
8. A wolf egg near two rabbits: the wolf kills one. A cat egg near two rabbits: the cat stalks and pounces on one. A zombie egg 3 blocks from a rabbit: the rabbit flees.
9. Kill rabbits with the sword: raw rabbit, often a hide, now and then a rabbit's foot. Light the grass by one with flint and steel: if it dies burning, it drops cooked rabbit. Shoot one with the bow and one with the M4A1: each flashes red and dies.
10. Lead a rabbit: it hops after you, roped at the neck.
11. Leave an adult, a baby and the killer bunny alive, save, quit and load: the same three are there and the bunny still attacks. In a world from an older build, the rabbits that stood frozen now hop.
12. `/locate biome minecraft:snowy_plains`, `/tp` there and walk into new land: groups of white or patched rabbits, one adult with babies of its colour. In a desert they are gold. **rabbit12_snowy**
13. In creative, place the spawner and use a rabbit egg on it: a small rabbit turns in the cage and rabbits spawn around it.
14. Two windows (`scripts\mp-local.ps1`). The host hatches three rabbits and a killer bunny.
    - The friend sees the same rabbits, colours, hops and head tilts at the same moments.
    - The friend holds a carrot: the rabbits come to the friend.
    - The killer bunny goes for the friend too.
    - The friend shoots a rabbit with the M4A1: it dies in both windows. **rabbit14-host**, **rabbit14-friend**

### Kit

```
IW4L_MINECRAFT_KIT=minecraft:rabbit_spawn_egg*16,minecraft:carrot*64,minecraft:golden_carrot*16,minecraft:dandelion*16,minecraft:iron_hoe,minecraft:bone_meal*64,minecraft:water_bucket,minecraft:oak_fence*64,minecraft:wolf_spawn_egg*2,minecraft:cat_spawn_egg*2,minecraft:zombie_spawn_egg*2,minecraft:lead*4,minecraft:iron_sword,minecraft:shield,minecraft:iron_helmet,minecraft:iron_chestplate,minecraft:iron_leggings,minecraft:iron_boots,minecraft:spawner,minecraft:cooked_beef*32,minecraft:bow,minecraft:arrow*64,minecraft:grass_block*64,minecraft:flint_and_steel,minecraft:lava_bucket
```

25 stacks, every id checked in the 26.3 `assets/minecraft/items`. The friend's `.env` needs the same line.

### Depends on

- **D0 d0a:** `registry-core`, `registry-hooks` (`MonsterSpec`, `GoalExtra`, `Prey::Kinds`), `registry-client` (`FamilyShow`), `registry-draw` (`keyframes::shared()`). Not d0b.
- **E1 (optional):** hearts come from `puff_for`'s default row 18, so no rabbit row is needed. If E1's commit 4 also posts 18 for registered families, keep only one of the two posts and record which.
- **E2 (none):** thrown items hit rabbits through `hit_boxes`.
- **W (optional):** the `block_change` line above. Hop steps vibrate through `play_movement`.
- **Later M1 groups** build on `animal_goals.rs` and `family_goal_step_with`. Foxes target rabbits through `Prey::Kinds`; rabbits do not flee foxes.
- **M2:** uses `family_goal_step_with` and `World::biome_in_tag` under these names.

### Left for later

- The hop's dust (event 1, `spawnSprintParticle`): `ClientFx` has no block particles.
- The nibble's level event 2001 (crumbs and break sound): no mob plays it, and the sheep's grass eating has none. Once foxes add `World::destroy_effect` (M1-4) it is one line.
- "The Killer Bunny" shown over the mob or in death lines: the fork draws no mob names, and `death.rs:689` names the type.
- Name tags (Toast only through `/summon`) and the egg's `minecraft:rabbit/variant` component.
- `AgeLocked` for the older stores (`server_mobs.rs:1536`).
- `ClimbOnTopOfPowderSnowGoal` if D0 did not port it.
- Looting and Fire Aspect, which never reach `EntityLootContext` in the fork.

### Open decisions (recommended choice)

1. **AI step:** the rabbit's own `rabbit_ai_step` in vanilla order, run through the new root hook `family_goal_step_with`.
2. **Bred baby colour:** add `World::biome_in_tag` to keep vanilla's 1-in-20 biome colour.
3. **Monsters to flee:** `AvoidEntityGoal::matching(is_monster_class)`, which also covers registered monsters; no hand list. `is_monster_class` lives in `animal_goals.rs`; worried pandas use it too.
4. **Idle head tilt:** a host-side clock on its own `LegacyRandom` (never `core.random`), sent in `FamilyShow`, so host and friend see one pose. Vanilla runs it per client.
5. **Killer bunny goals:** one spec with the melee and target goals registered last and gated on `evil`, as vanilla's later `setVariant` inserts them. `MonsterKind::Registered(kind)` cannot choose a spec by variant.

---

## M1-2. Polar bears (segment m1a: groups `polar-bears-live`, `polar-bears-drawn`)

Family `polar_bears` (`PolarBear`), group `land`, `Family::KEY` **55** (row 22 of `docs/round10/mobs.md`), one kind `minecraft:polar_bear`. It is an `Animal` and a `NeutralMob` on the goal framework (`MonsterKind::Registered`, D0-2.4). Lines are `minecraft-survival` at `d1a3d028`; re-check before editing. D0 names follow `track-d0.md`; where D0's merged code differs, follow the code. Polar bears run right after rabbits because they need only d0a and the rabbits' `animal_goals.rs`, and they add the melee-check hook.

### What exists today

- **Dormant and never spawned.** `minecraft:polar_bear` is not in `SIMULATED_KINDS` (`crates/minecraft_terrain/src/server_mobs.rs:433`), so `spawn_saved` returns `None` (`:1321`); `tracking_range` (`mob_tracking.rs:285`) gives 5. The size row `natural_spawner.rs:288` exists, but its rules fall to `Rules::Other` (`:354`), which `note_unsupported`s (`natural_spawner/spawning.rs:766`), and `finalize_spawn` (`natural_spawner.rs:896`) has no polar bear arm. Ice counts as a valid floor only for polar bears (`natural_spawner.rs:723`), and powder snow is not dangerous to them (`natural_spawner.rs:747`).
- **Reuse:**
  - Goals: `panic.rs:9`, `follow_parent.rs`, `wolf_ai.rs:183-214` (panic, melee, NeutralMob targets), `monster_ai.rs:2855` `update_enderman_anger` (`NeutralMob.updatePersistentAnger`, which golems share); the panic goal port in M1-1's `animal_goals.rs`.
  - Food and cubs: `animal.rs:12` `animal::interact` (the golden dandelion lock).
  - Saving: anger keys as `server_mobs.rs:968` reads them (`anger_end_time` is already a global `VOLATILE` key, `mob_tracking.rs:28`).
  - Render: `pig_render.rs:155` (the `QuadrupedModel` pose), `cat_render.rs:454` `scaled_root` (`MeshTransformer.scaling`).
- **Gaps:**
  - `MeleeAttackGoal` (`monster_ai.rs:2239`) checks its attack inline (`:2308`), with private `MeleeState` (`:440`).
  - `travel_water` fixes the 0.8 slow-down (`movement.rs:270`).
  - Mobs do not freeze yet (`inside_blocks.rs:86`).

### Vanilla 26.3 (javap this round)

- **Type and attributes:**
  - `CREATURE`, `immuneTo(#polar_bear_immune_to)` (powder snow), `sized(1.4, 1.4)`, eye 1.19 (the default 0.85 × height), `clientTrackingRange(10)`;
  - cub: `scalable(0.7, 0.7)`, eye 0.34375; it is in `#freeze_immune_entity_types`;
  - health 30, follow range 20, speed 0.25, attack 6, experience 1-3 for adults;
  - `getWaterSlowDown` 0.98; `isFood` false and no breed goal.
- **Goals**:
  - 0 `FloatGoal`;
  - 1 `PolarBearMeleeAttackGoal`, which is `MeleeAttackGoal(1.25, true)` with its own `checkAndPerformAttack`:
    - in reach: reset the cooldown, `doHurtTarget`, stand down;
    - else within `(targetWidth + 3)²`: when the cooldown is up, stand down and reset it; with 10 or fewer ticks left, **stand up** and `playWarningSound`;
    - else reset and stand down; `stop` stands down;
  - 1 `PanicGoal(2.0)`: a cub panics from `#panic_causes`, an adult only from `#panic_environmental_causes`;
  - 4 `FollowParentGoal(1.25)`; 5 `RandomStrollGoal(1.0)` (not water-avoiding); 6 `LookAtPlayerGoal(player, 6)`; 7 `RandomLookAroundGoal`.
- **Targets**:
  - 1 `PolarBearHurtByTargetGoal`: a hurt cub alerts others, which must be adult polar bears, and drops its own target; an adult only takes its attacker;
  - 2 `PolarBearAttackPlayersGoal` (player, interval 20, must see, must reach): not a cub, and a cub polar bear within its box inflated by (8, 4, 8); its follow distance is halved, to 10;
  - 3 `NearestAttackableTargetGoal(player, 10, true, false, isAngryAt)`;
  - 4 `NearestAttackableTargetGoal(fox, 10, true, true)`, adults only;
  - 5 `ResetUniversalAngerTargetGoal(false)`.
- **`tick`**:
  - client: current ±1 per tick, clamped to 0..6, following the synced `DATA_STANDING_ID` (`O` keeps last tick's). The client box height scales by `1 + current / 6`; the server's never does.
  - server: `warningSoundTicks--`; `updatePersistentAnger(level, true)` (20-39 s, saved as `anger_end_time` and `angry_at`).
  - The warning is `makeSound(.warning)` when its timer is 0, then the timer is 40.
- **Sounds** (`NEUTRAL`): `entity.polar_bear.ambient` (`.ambient_baby` for a cub), `.hurt`, `.death`, `.step` at 0.15, `.warning`.
- **Spawning**:
  - `ON_GROUND`, `MOTION_BLOCKING_NO_LEAVES`;
  - `checkPolarBearSpawnRules`: in `#polar_bears_spawn_on_alternate_blocks` (frozen and deep frozen ocean), raw light above 8 and ice below (`#polar_bears_spawnable_on_alternate`); else `checkAnimalSpawnRules`;
  - snowy plains, ice spikes and both frozen oceans (weight 1, 1-2);
  - `finalizeSpawn` uses `AgeableMobGroupData(1.0)`: the first of a group is an adult, the rest are cubs.
- **Loot** `entities/polar_bear`: cod (weight 3) or salmon (weight 1). Each gives `uniform(0, 2)` plus looting `uniform(0, 1)`, smelted when burning or by a `#smelts_loot` weapon.
- **Model** (`PolarBearModel`, a `QuadrupedModel`, 128 x 64, then `MeshTransformer.scaling(1.2)`, i.e. every root pose scaled and lifted by 24.016 × (1 − 1.2)); take the exact boxes from `createBodyLayer` during the work:
  - head (0, 10, -16) with mouth and ears;
  - body (-2, 9, 12), xRot π/2, two boxes;
  - legs 4 x 10 x 8 (hind) and 4 x 10 x 6 (front);
  - cub: `BabyPolarBearModel`, 64 x 64, unscaled, 9 boxes.
  - **`setupAnim`**: `QuadrupedModel` (head look, legs `cos(pos × 0.6662 [+π]) × 1.4 × speed`). Then, with `f = standScale²` and `a = ageScale` (0.5 for a cub):
    - `body.xRot -= f·0.35π`, `body.y += 2fa`;
    - front legs: `y -= 20fa`, `z += 4fa`, `xRot -= f·0.45π`;
    - `head.y -= 24f`, `head.z += 13f`, `head.xRot += f·0.15π`;
    - `standScale = lerp(partial, O, current) / 6`.
- **Renderer**: `AgeableMobRenderer(adult, cub, 0.9)` with `entity/bear/polarbear` and `polarbear_baby`. `EntityModel`'s default render type is `entityCutout` (javap during the work), so the model draws into `out.culled` (D0-5).

### Plan

**Files** (D0-3):
- Entities, in `third_party/minecraftoss/entities/src/world/families/land/`: `polar_bears.rs` (`MonsterAi`, age, `stand`, `stand_old`, `standing`, `warning_ticks`, `impl Family`, `POLAR_BEAR_SPEC`) and `polar_bears/ai.rs` (the melee check, the hurt-by and attack-players goals; panic and follow-parent from `animal_goals.rs`, ported there if missing).
- Terrain: `crates/minecraft_terrain/src/families/land/polar_bears.rs` with `polar_bears/render.rs`.
- World: one row in `natural_spawner/families/land.rs` (below).
- Registration: `pub mod polar_bears;` and `polar_bears: polar_bears::PolarBear,` in `family_group!(land 48..=63 { .. })`; `mod polar_bears;` and `PolarBear` in `terrain_group!(land { .. })`.

**Hooks used** (D0-2.2):
- `category` `"creature"` (default despawn and experience); `core.ambient_interval` 120;
- `leashable` true with a `leash_walk` along its own navigation;
- `panicking` (the panic goal running); `step_sound` `entity.polar_bear.step`; default `voice` (`neutral_volume`); a cub's `Voice::Ambient` becomes `Voice::Event("entity.polar_bear.ambient_baby", ..)` in the tick, right after `family_base_tick` (the track's ambient rule, M10);
- `monster` → `Some(&POLAR_BEAR_SPEC)`: follow 20, eye 1.19, `walk` the animal values with `can_float`;
- `GoalExtra` carries `PolarBearGoals { last_damage, standing, warn }`; `hurt` stores the damage type for the 40-tick panic window;
- strikes go through `family_strike` (`PlayerHitKind::Melee`, 6);
- anger is `update_enderman_anger` on `ai.state`;
- the tick takes `ctx.alert` and hands it to adult polar bears with no target within 20 across and 10 up or down (as `vex_alerts`);
- foxes are `Prey::Kinds(&["minecraft:fox"])`;
- the attack-players goal wraps `NearestTargetGoal::every(Prey::Player, true, 10)` behind the cub check and halves `ctx.follow_range` around its calls.

**The melee check hook** (its own commit, "Registered Minecraft mobs can land their melee attacks their own way"):
- `MeleeAttackGoal`'s inline check (`monster_ai.rs:2308`) moves into `fn tick_with(&mut self, ctx, world, check: Option<MeleeCheck>)`, and `tick_world` calls it with `None`. `type MeleeCheck = fn(&mut MonsterGoalContext, &dyn World, TargetInfo)`.
- A new `pub(crate) struct CheckedMeleeAttackGoal { goal: MeleeAttackGoal, check: MeleeCheck }` beside it, built by `MeleeAttackGoal::checked(self, check)`, delegates every `Goal` method to `goal` and ticks through `tick_with(.., Some(check))`.
- `MeleeState` gains `pub(crate)` `ticks_until_attack()`, `reset_attack_cooldown()` and `is_time_to_attack()`.
- No field is added to `MeleeAttackGoal`, so its 10 existing literals and the literals other tracks add this round (M2's dolphins) stay as they are. Existing melee tests stay unchanged.

**The row** (`natural_spawner/families/land.rs`):
- `TypeInfo { 1.4, 1.4, OnGround, MotionBlockingNoLeaves, false, Rules::Family { creature: true }, Walk::Animal, remove_far: false, cluster: 4 }`;
- `Creature`, follow range 20, `baby_chance: Some(1.0)`, `finalize: None`;
- `rule: Some(polar_bear_rule)`: it looks the two tags up by name, as `check_spawn_rules` does, and falls back to the animal rules.

**Multiplayer** (`TerrainFamily`):
- `TRACKING_RANGE` 10, `SHADOW_RADIUS` 0.9, `MAX_HEAD_Y_ROT` 75, `VOLATILE = ["angry_at"]`;
- `read`/`fields`: `anger_end_time`, `angry_at`, age and love;
- `show` = `flags: STANDING 1, values: [stand, stand_old]`. The host steps `stand` as the vanilla client does; the draw lerps.
- No `PROTOCOL` change.

**Interact** (terrain): `animal::interact` with no food (the golden dandelion lock on cubs).

**Commit groups**

| # | Key | Commits | Row 22 |
|---|---|---|---|
| 1 | `polar-bears-live` | 1. "Registered Minecraft mobs can land their melee attacks their own way" (the hook). 2. "Minecraft polar bears live, defend their cubs and stand up to strike" (family, goals, anger, save/load/show, `TEXTURES` set, `draw` empty). 3. "Minecraft polar bears spawn in snowy biomes and on sea ice" (the row). | `live` |
| 2 | `polar-bears-drawn` | "Minecraft polar bears are drawn like vanilla" (`scaled_root(1.2)` adult, cub, standing pose). `cat_render.rs:454` `scaled_root` and `pig_render.rs:155` `part_pose` are private: copy their few lines into `polar_bears/render.rs` rather than widening them in files other tracks edit. | `done` |

Each commit updates row 22 of `mobs.md` and the track log.

### Tests

- **`polar-bears-live`**:
  - `a_melee_goal_with_its_own_check_lands_hits_through_it` (the hook; the plain goal's tests unchanged);
  - `a_polar_bear_is_large_with_thirty_health_and_its_kind`;
  - `a_hit_bear_stands_warns_every_two_seconds_and_strikes_for_six`;
  - `a_hit_cub_runs_and_nearby_adults_attack`;
  - `an_adult_with_a_cub_near_attacks_players_within_ten`;
  - `an_adult_hunts_foxes_and_a_cub_does_not` (at the goal level with a fox candidate in `ctx.mobs`, because foxes are not simulated before M1-4);
  - `its_anger_lasts_twenty_to_thirty_nine_seconds_and_ends_for_creative_players`;
  - `an_adult_panics_only_from_fire_and_a_cub_from_any_hit`;
  - world: `polar_bears_spawn_on_grass_in_the_light_and_on_ice_in_frozen_oceans`, `a_polar_bear_group_is_an_adult_with_cubs` (with and without context);
  - terrain: `a_polar_bear_saves_and_loads_with_its_anger_and_age`, `a_friend_sees_a_polar_bear_stand_up_from_its_look_tag`.
- **`polar-bears-drawn`**: `the_adult_is_scaled_1_2_and_the_cub_has_its_own_model`, `a_standing_bear_lifts_its_body_front_legs_and_head`, `the_stand_clock_lerps_with_the_partial_tick`.

### Owner's steps

Setup: release build, survival, **Normal**, by day, flat grass, the kit below. Step 8 needs two windows (`scripts\mp-local.ps1`).

1. Use a polar bear egg 8 blocks away: a big white bear wanders and leaves you alone. **m1_polar_bear**
2. Punch it: it comes at you. Close up it rears onto its hind legs with a growl, then swipes: 3 hearts on Normal. Raise the shield: the swipe hits the shield. **m1_polar_bear_standing** Kill it with the iron sword: 0 to 2 raw cod or salmon and experience. Set the grass under another alight with flint and steel: if it dies burning, the fish is cooked (Fire Aspect is not modelled in the fork, `docs/MINECRAFT-SURVIVAL.md:454`).
3. `/summon minecraft:polar_bear ~ ~ ~8` and `/summon minecraft:polar_bear ~2 ~ ~8 {Age:-24000}`: a bear and a cub that follows it. Walk to within 10 blocks of the bear (the cub within 8 blocks of it): the bear comes for you unprovoked. **m1_polar_bear_cub**
4. Summon another pair 20 blocks away and hit only the cub with an arrow: the cub runs away fast, and every adult within 20 blocks attacks you.
5. Make a bear angry, run 30 blocks away and wait 40 s: it calms down. Make another angry, save and quit at once, and load: it is still angry. In creative, angry bears stop chasing you.
6. Use a fox egg 6 blocks from an adult bear (once foxes have landed, M1-4): it hunts the fox down. A cub ignores foxes.
7. `/locate biome minecraft:ice_spikes` and `/tp`: bears alone or with a cub. `/locate biome minecraft:frozen_ocean`: bears on the sea ice. **m1_polar_bear_ice** In creative, a polar bear egg on a spawner: a small bear turns in the cage.
8. Friend: the friend punches a bear: both windows show it rear up and swipe at the same moment. The friend hits a summoned cub: the adults go for the friend. **m1_polar_bear-host**, **m1_polar_bear-friend**

### Kit

```
IW4L_MINECRAFT_KIT=minecraft:polar_bear_spawn_egg*8,minecraft:fox_spawn_egg*4,minecraft:iron_sword,minecraft:flint_and_steel,minecraft:bow,minecraft:arrow*64,minecraft:shield,minecraft:cooked_beef*32,minecraft:spawner,minecraft:lead*4,minecraft:golden_dandelion*4,minecraft:grass_block*64,minecraft:ice*32,minecraft:snow_block*32,minecraft:torch*64
```

15 stacks, all ids from the drafts' checked kits. The fox egg spawns nothing until M1-4 lands. The friend needs the same kit.

### Depends on

- **D0, segment d0a**: the registry, `MonsterKind::Registered`, `GoalExtra`, `Prey::Kinds`, the leash and `mob_candidates` arms, rows with `Rules::Family` and `FamilyShow`. Not d0b.
- **M1-1 rabbits**: `animal_goals.rs` (panic).
- **M1-4 foxes**: step 6, and the fox's own fleeing from bears.
- **E2** (`ti1`): thrown items hitting bears.

### Left for later

- **`mustReach`**: ported nowhere (`monster_ai.rs:2909`), so a bear may take a target it cannot path to.
- **The bear's 0.98 water slow-down** (`movement.rs:270`): do it only if a `WalkProfile` field set by `spec.walk` is enough.
- **The standing bear's taller box**: client-only in vanilla, so the host keeps the server box.
- **Freeze immunity**: nothing freezes yet.
- **Spawn eggs used on adults to make babies**: not in the fork for any mob.

### Open decisions (recommended choice first)

1. **The bear's attack check:** the `CheckedMeleeAttackGoal` wrapper over `MeleeAttackGoal::tick_with`. A `check` field on `MeleeAttackGoal` would break every literal other tracks add this round; copying the goal's 80 lines into the family is the other alternative.
2. **Standing:** sent as the host's `stand` and `stand_old` in `FamilyShow`, stepped like vanilla's client, rather than each client stepping from a flag.

---

## M1-3. Pandas (segment m1b: groups `pandas-live`, `pandas-drawn`, `pandas-eating`, `pandas-moods`, `pandas-thunder`)

Family `pandas`, group `land`, hit-key byte **52** (`Family::KEY`, row 19 of `docs/round10/mobs.md`). One kind, `minecraft:panda`. Lines are `minecraft-survival` at `d1a3d028`; re-check before editing. Where D0's merged code differs from `track-d0.md`, follow the code. Pandas add four hooks the later groups reuse (H1-H4 below): holding still, item crumbs, gifts from loot tables, and rain and thunder.

### What exists today

- **Dormant:**
  - `minecraft:panda` is not in `SIMULATED_KINDS` (`crates/minecraft_terrain/src/server_mobs.rs:433`), so `spawn_saved` (`:580`, `_ => return None` at `:1321`) drops it. Jungle pandas stand frozen and the egg does nothing.
  - `tracking_range` (`mob_tracking.rs:285`) gives the default 5.
- **Spawning:**
  - The size row `third_party/minecraftoss/world/src/natural_spawner.rs:281` (1.3 x 1.25, `NoRestrictions`, `MotionBlockingNoLeaves`, animal rules) and the creature category (`natural_spawner/spawning.rs:102`) match 26.3.
  - There is no finalize: pandas fall into the catch-all `natural_spawner.rs:1049-1052` (`note_unsupported`, no genes, baby chance 0.05 instead of 0.2).
- **Eggs and `/summon` share one path** (terrain `server.rs:1557` `hatch_egg` → vendored world `level/spawning.rs:206` → `natural_spawner/spawning.rs:886`). Below, `server.rs` is always `crates/minecraft_terrain/src/server.rs`. NBT skips the finalize, as vanilla `/summon` does. `spawning.rs:896` also sets `CanPickUpLoot` false for every non-piglin; that is vanilla for `/summon` but not for eggs ("Left for later").
- **Reuse** (the wolf is the closest family: an animal on `MonsterGoalContext`):
  - Goals: `monster_ai.rs` `FloatGoal` `:2015`, `AvoidEntityGoal` `:1735`, `MeleeAttackGoal` `:2239`, `StrollGoal` `:2332`, `RandomLookGoal` `:2417`, `HurtByTargetGoal` `:3044`.
  - Wolf: `wolf_ai.rs` `:175` and `:216` (environmental panic), `:97` (breed), `:183` (goal list); `world/wolves.rs` `:446` (tick), `:279` (`wolf_breed`), `:425` (`alert_wolves`).
  - Animal state: `breed.rs:7`, `tempt.rs:30`, `follow_parent.rs:17`, `age.rs:6`, `animal.rs:12` (the golden dandelion lock); tempters at `world/felines.rs:353`; `reduced_tick_delay` at `feline_ai.rs:297`.
  - Items: `World::items_in`, `take_item` and `spawn_item` (`player/src/lib.rs:105-117`), as `world/piglins.rs:1015` and `:809` use them.
  - Saving: `illager_save.rs:82-90` and `:155-156` (`equipment`, `drop_chances`); `server_mobs.rs:1536` (`Age`, `ForcedAge`); `:1040` and `:1616` (`InLove`).
  - Gifts and jumps: `loot.rs:271` `roll_cat_gift`; `server.rs:1355` `drop_cat_gifts`, called at `:1107`; `movement.rs:613` `jump_power`.
  - Render: `posed_cube.rs`; `client_mobs.rs:1395` `quadruped_legs`; `cow_render.rs:45` (head look); `mesh.rs:1339` `HeldDisplay::Ground`, used at `villager_render.rs:231`.
  - Particles: `ambient_particles.rs:42` `Kind` and `:1061` `add`. Crumbs: `entity_view.rs:58` → `render_anim/src/minecraft_world.rs:1612` → `block_particles.rs:185`.
  - Weather: `level/fire.rs:65` `is_raining` (`pub(super)`); the spawner's `thundering` at `level/spawning.rs:162` (`thunder_level > 0.9`, which M3 corrects to vanilla's thunder × rain). Bone meal grows bamboo saplings into stalks: `level/bonemeal.rs:131-136`.

### Vanilla 26.3 (javap this round)

**Type.**
- `EntityTypes.PANDA`: `CREATURE`, `sized(1.3, 1.25)` with no `eyeHeight`, so the eye is at 1.0625. `clientTrackingRange(10)`, allowed on peaceful.
- Cub: the size scaled 0.5 (0.65 x 0.625), eye 0.28125.
- `createAttributes`: animal (health 20, follow 16, tempt 10), speed `0.15000000596046448`, attack 6.
- From `Animal`: ambient interval 120, `removeWhenFarAway` false, experience `1 + nextInt(3)` for adults only.
- `canBeLeashed` false. No `handleEntityEvent` override (only `Animal`'s 18).
- The constructor installs `PandaMoveControl` and `setCanPickUpLoot(true)`. The age is still 0 there, so every new panda may pick up loot. `CanPickUpLoot` is saved.

**Genes.**
- `Panda$Gene` is saved by name as `MainGene` and `HiddenGene`; unknown → `normal`. Ids: 0 `normal`, 1 `lazy`, 2 `worried`, 3 `playful`, 4 `brown`\*, 5 `weak`\*, 6 `aggressive` (\* recessive).
- `getRandom` (`nextInt(16)`): 0 lazy, 1 worried, 2 playful, 4 aggressive, 3 and 5-8 weak, 9-10 brown, 11-15 normal.
- Variant: the main gene, except that a recessive main gene shows only when the hidden one matches; otherwise `normal`.
- `setAttributes` (only from `finalizeSpawn` and `getBreedOffspring`): weak → max health base 10 (`onAttributeUpdated` clamps health), lazy → speed base `0.07000000029802322`. So `/summon` with NBT keeps 20 and 0.15, and loading keeps the saved `attributes`.
- `finalizeSpawn`: main, then hidden from the **level** random; `setAttributes`; `AgeableMobGroupData(0.2)` when no group data.
- `setGeneFromParents(a, b)`:
  - `nextBoolean` decides which parent gives the main gene (via `getOneOfGenesRandomly`: `nextBoolean`, main or hidden) and which the hidden. Without b, one side is `getRandom`.
  - Then each gene becomes `getRandom` when `nextInt(32) == 0`.
  - The cub gets `setAttributes`.

**Sounds** (`NEUTRAL`):
- Ambient by gene: `aggressive_ambient`, `worried_ambient`, else `ambient`. `hurt`, `death`, `step` at 0.15.
- `bite` (1, 1) after a landed hit. `cant_breed`, `pre_sneeze` and `sneeze` (1, 1).
- `eat`: volume `0.5 + 0.5 * nextInt(2)`, pitch `(r1 - r2) * 0.2 + 1`.
- All as `entity.panda.*`.

**Loot.**
- `entities/panda`: 1 bamboo.
- `gameplay/panda_sneeze` (gift): slime ball 1 against empty 699.
- Held item (`Mob.dropCustomDeathLoot`): a guaranteed slot always drops; otherwise only when a player killed it and `nextFloat() < 0.085`.

**Spawning.**
- `NO_RESTRICTIONS`, `MOTION_BLOCKING_NO_LEAVES`, `Animal.checkAnimalSpawnRules` (the predicate shared with cows).
- `bamboo_jungle` weight 80 and `jungle` weight 1, each 1-2.
- `#panda_food` = bamboo; `#panda_eats_from_ground` = it + cake.

**Goals.** `rtd` is `reducedTickDelay` (n/2 rounded up); full goal updates run every other tick.

| Pri | Goal | Rule |
|---|---|---|
| 0 | `FloatGoal` | |
| 2 | `PandaPanicGoal(2.0, #panic_environmental_causes)` | stops the navigation and ends while sitting |
| 2 | `PandaBreedGoal(1.0)` | below |
| 3 | `PandaAttackGoal(1.2, true)` | melee; needs `canPerformAction` |
| 4 | `TemptGoal(1.0, #panda_food, false)` | |
| 6 | `PandaAvoidGoal` from `Player` (8 blocks) and `Monster` (4 blocks), both at 2.0/2.0 | worried and `canPerformAction` |
| 7 | `PandaSitGoal` (MOVE) | below |
| 8 | `PandaLieOnBackGoal`, `PandaSneezeGoal` (no flags) | below |
| 9 | `PandaLookAtPlayerGoal(Player, 6)` | keeps its target between uses; `setTarget` lets the breed goal aim it; needs `canPerformAction` |
| 10 | `RandomLookAroundGoal` | |
| 12 | `PandaRollGoal` (MOVE, LOOK, JUMP) | not interruptable, lasts one tick |
| 13 | `FollowParentGoal(1.25)` | |
| 14 | `WaterAvoidingRandomStrollGoal(1.0)` | |
| T1 | `PandaHurtByTargetGoal().setAlertOthers()` | clears the target once `gotBamboo` or `didBite`; alerts only **aggressive** pandas |

- `canPerformAction`: not on its back, scared, eating, rolling or sitting. Without it `PandaMoveControl.tick` does nothing (the inputs keep fading, the yaw stays).
- `isScared`: worried and `isThundering`.
- **Breed:** `BreedGoal.canUse`, unhappy 0, then any `minecraft:bamboo` within x, z ±7 and y 0..2 of its block (a yes/no, so the order does not matter). Otherwise, if `unhappyCooldown <= tickCount`: unhappy 32, cooldown +600, and the look goal is set on the nearest player within 8.
- **Sit:**
  - `canUse`: `cooldown <= tickCount`, adult, not in water, `canPerformAction`, not unhappy, and either holding something or an item within 6 that `canPickUpAndEat` (tag, alive, no pickup delay).
  - Continues while not in water and (lazy or `nextInt(rtd(600)) != 1`) and `nextInt(rtd(2000)) != 1`.
  - `start`: with an empty paw it walks to the first such item within 8 at 1.2, else `tryToSit`. `tick`: holding and not sitting → `tryToSit`.
  - `stop`: drops the held stack (`spawnAtLocation`), empties the paw, cooldown `tickCount + (lazy ? 10 + nextInt(50) : 10 + nextInt(150)) * 20`, `sit(false)`.
- `tryToSit` (not in water): zza 0, navigation stop, sit.
- **Lie on back:** `cooldown < tickCount`, lazy, `canPerformAction`, `nextInt(rtd(400)) == 1`. It continues on the sit goal's rule, and `stop` sets cooldown `tickCount + 200`.
- **Sneeze:** a cub with `canPerformAction`; weak `nextInt(rtd(500)) == 1`, else `nextInt(rtd(6000)) == 1`.
- **Roll:**
  - Needs (cub or playful), on the ground and `canPerformAction`.
  - True at once when the block at (`sign(-sin yaw)` if over 0.5, -1, `sign(cos yaw)` if over 0.5) is air.
  - Else playful `nextInt(rtd(60)) == 1`, cub `nextInt(rtd(500)) == 1`.
- **Pickup** (`Mob.aiStep`, after the move):
  - Needs `CanPickUpLoot`, alive and `mob_griefing`; looks at items in its box grown by (1, 0, 1) with no pickup delay.
  - `Panda.pickUpItem`: only with an empty paw and `canPickUpAndEat`. The **whole stack** goes into the paw as a guaranteed drop, and the item is discarded.

**`hurtServer`:** first `sit(false)`, then `Animal` (an applied hit resets the love).

**`doHurtTarget`:** non-aggressive pandas set `didBite`; the hit is the mob melee, 6.

**`tick`, after `super.tick` (move, goals and pickup are done):**
1. Worried: in thunder and not in water → `sit(true)`, `eat(false)`; else, when not eating, `sit(false)`. So a worried panda never settles on picked-up bamboo; keep that.
2. No target → clear `gotBamboo` and `didBite`.
3. Unhappy above 0: look at the target (90, 90); at 29 and 14 → `cant_breed`; count down.
4. Sneezing: count up; at 1 → `pre_sneeze`; above 20 → `afterSneeze`:
   - the `SNEEZE` particle at `(x - (w + 1) / 2 * sin bodyYaw, eyeY - 0.1, z + (w + 1) / 2 * cos bodyYaw)` with motion `(dx, 0, dz)`;
   - the `sneeze` sound;
   - pandas in its box grown by 10 that are adults, on the ground, dry and `canPerformAction` jump;
   - with `mob_drops`, the gift table drops at its feet.
5. Rolling → `handleRoll`, else counter 0. `handleRoll`:
   - Counter +1; above 32 → stop.
   - At 1: `rollDelta = (dx - sin(yaw) * s, 0, dz + cos(yaw) * s)` with `s` 0.2 (cub 0.1), and the motion becomes `rollDelta + (0, 0.27, 0)`.
   - At 7, 15 and 23: motion `(0, onGround ? 0.27 : dy, 0)`.
   - Otherwise `(rollDelta.x, dy, rollDelta.z)`.
6. Sitting → `xRot = 0`. The sit, back and roll amounts move +0.15 towards 1 while their flag is set and -0.19 towards 0 otherwise, keeping the old value for the partial tick.
7. `handleEating`:
   - Not eating, sitting, not scared, holding and `nextInt(80) == 1` → eat. With an empty paw or not sitting → stop eating.
   - While eating:
     - Every 5th counter: the `eat` sound and (client) 6 `ITEM` particles of the held stack. Motion `((r - 0.5) * 0.1, r * 0.1 + 0.1, (r - 0.5) * 0.1)` turned by `-xRot`, then `-yRot`. Position `((r - 0.5) * 0.8, -r * 0.6 - 0.3, 1 + (r - 0.5) * 0.4)` turned by `-yBodyRot`, plus `(x, eyeY + 1, z)`. Added with the y motion + 0.05.
     - Server, counter above 80 and `nextInt(20) == 1`: above 100 with a tagged item → empty the paw, game event `eat`, `sit(false)`; then stop eating.
     - Otherwise the counter counts up.

**`mobInteract`:**
1. Scared → pass.
2. On its back → it gets up.
3. Bamboo:
   - A target sets `gotBamboo`.
   - A cub that can age: use one, `ageUp((-age / 20) * 0.1, true)`. Any other cub: pass.
   - An adult at age 0 that can fall in love: use one, love (event 18).
   - Otherwise, not sitting and not in water: `tryToSit`, eat, drop the held stack unless the player has infinite materials, hold **one** of the item, use one.
4. A cub and a golden dandelion → `Animal.mobInteract` (the age lock).
5. Anything else → pass.

**Client.**
- `AgeableMobRenderer(PandaModel, BabyPandaModel, 0.9)` + `PandaHoldsItemLayer`.
- 14 textures: `entity/panda/panda`, `panda_{lazy,worried,playful,brown,weak,aggressive}`, `panda_baby`, `{lazy,worried,playful,brown,weak,aggressive}_panda_baby`.
- `PandaModel`, a `QuadrupedModel` on a 64² texture (uv; from; size):
  - `head` at (0, 11.5, -17): (0,6; -6.5,-5,-4; 13,10,9), nose (45,16; -3.5,0,-6; 7,5,2), left ear (52,25; 3.5,-8,-1; 5,4,1), right ear (52,25; -8.5,-8,-1; 5,4,1).
  - `body` at (0, 10, 0), xRot π/2: (0,25; -9.5,-13,-6.5; 19,26,13).
  - Legs (40,0; -3,0,-3; 6,9,6) at (∓5.5, 15, 9) hind and (∓5.5, 15, -9) front.
- `BabyPandaModel` (64²):
  - `body` at (0, 18.5, 2.5): (0,11; -4.5,-3.5,-5.5; 9,7,11).
  - `head` at (0, 19, -3): (0,0; -3.5,-3,-5; 7,6,5), (24,6; -2,1,-6; 4,2,1), (24,0; -4.5,-4,-3.5; 3,3,1), (33,0; 1.5,-4,-3.5; 3,3,1).
  - Legs (3,2,3 from -1.5,0,-1.5): uv (0,34)/(12,34) at (∓3, 22, 6.5) hind and (0,29)/(12,29) at (∓3, 22, -1.5) front.
- `setupAnim`, after the quadruped head look and leg swing:
  - **Unhappy:** head yRot = zRot = `0.35 sin(0.6 age)`; front legs xRot `∓0.75 sin(0.3 age)`. Else head zRot 0.
  - **Sneeze** (time t): under 15 → head xRot `-π/4 * t / 14`; under 20 → `-π/4 + π/4 * ((t - 15) / 5)`. The division is in **integers**, so it holds -π/4.
  - **Sit above 0:**
    - `animateSitting` (adult): body xRot `rotLerpRad(sit, xRot, 1.7407963)`, head xRot towards π/2, front legs zRot ∓0.27079642, hind legs ±0.5707964.
    - `animateSitting` (cub): body xRot towards 0.17453292; lerps of body z → -1.5, head z → -11.5, head y → 17.5, front legs z → -5, hind legs z → 3; the same zRots.
    - Then eating: head xRot `π/2 + 0.2 sin(0.6 age)`, front legs `-0.4 - 0.2 sin(0.6 age)`.
    - Then scared: head xRot 2.1707964, front legs -0.9.
  - **Sit 0:** leg zRots 0.
  - **Back:** hind legs `∓0.6 sin(0.15 age)`, front legs `±0.3 sin(0.25 age)`, head xRot `rotLerpRad(back, xRot, π/2)`.
  - **Roll** (0 for cubs): head xRot towards 2.0561945; right hind and left front `-0.5 sin(0.5 age)`, the others `+0.5 sin(0.5 age)`.
- `setupRotations`:
  - **Roll:** `rollTime = rollCounter + partial` while above 0; h = 0.8 (cub 0.3). Four 8-tick quarters, each lerping `90q + 90(i - 8q)/7` towards the next tick's value (held at the quarter's end), turned `-angle` about x. Lifts: `(h + 0.2) angle/90`, `h + 0.2 + (h - 0.2)(angle - 90)/90`, `h + h(270 - angle)/90`, `h(360 - angle)/90`.
  - **Sit:** translate (0, 0.8s, 0), turn x `lerp(s, xRot, xRot + 90)`, translate (0, -s, 0). Scared: turn y `cos(1.25 age) * π * 0.05` degrees, and a cub moves (0, 0.8, 0.55).
  - **Back:** translate (0, (cub 0.5, adult 1.3) * b, 0), turn x `lerp(b, xRot, xRot + 180)`.
- **Held item:** only with an item, sitting and not scared. Model-space (0.1, 1.4, -0.6), less `0.09 sin(0.6 age)` in y and `0.2 sin(0.6 age) + 0.2` in z while eating. Drawn with display `GROUND`, on no part.
- `SNEEZE` is the `PlayerCloudParticle` sneeze provider: colour (0.22, 1.0, 0.53), alpha 0.4, translucent, sprites `generic_7..0`. javap the size, lifetime and drag during the work.
- No keyframe class: `keyframes::shared()` is not used.

### Plan

**Files (D0-3):**
- `third_party/minecraftoss/entities/src/world/families/land/pandas.rs`:
  - `pub struct Panda`: genes, `Age`, love, flags, counters, amounts with old values, held stack and its guaranteed flag, `can_pick_up_loot`, `speed`, `got_bamboo`, `did_bite`, `last_damage`, `sneeze_gifts`, `ai: Box<MonsterAi>`;
  - `impl Family`, the tick, breeding, `interact` and tests.
- `land/pandas/ai.rs`: the `MonsterSpec`, the goals and `PandaGoals` (the `GoalExtra` state the tick and goals share).
- `natural_spawner/families/land.rs`: the row.
- `crates/minecraft_terrain/src/families/land/pandas.rs` (`TerrainFamily`), with `pandas_render.rs` beside it.

**Registry hooks used:**
- `KEY` 52 and `KINDS`; `spawn` (adult size, eye and health; the cub size on the age boundary).
- `tick`: `family_base_tick` → `family_goal_step(.., state.speed, ..)` with any strike sent through `family_strike(.., 6.0, PlayerHitKind::Melee)` → `family_after_move` → the pickup and the tail.
- `category` "creature", which gives the default despawn, experience and loot rules; `core.ambient_interval` 120.
- `hurt`: `sit(false)`, then `hurt_plain`; an applied hit resets the love and keeps `(damage_type, tick_count)` for the panic rule (cleared after 40 ticks, as `wolf_ai.rs:175`).
- `death_drops`: the held stack. `step_sound`: `entity.panda.step`.
- `voice`: the default; the tail swaps a base-tick `Voice::Ambient` for the gene's `Voice::Event`.
- `monster`: the spec, with eye 1.0625, follow range 16 and `WalkTarget::Animal` (`walk_path.rs:49`). `leashable` stays false.

**One-line hooks in shared files** (each beside the named line):
- `ambient_particles.rs`: `Kind::Sneeze` right after `Poof` (`:48`), with its arms next to `Poof`'s; never at the end of `Kind` (M2's dolphin and guardian particles take other neighbours).
- The worried panda's `Monster` list is `animal_goals::is_monster_class` (M1-1), shared with the rabbits.
- Event 18 (love, breeding) needs no `mob_particles` row (E1's default). Push `(id, 18)` to `entity_events` (`world.rs:2623`), or use E1's `broadcast_entity_event` once it exists.

**Shared hooks this section adds** (each its own commit named for the hook, noted in the log; later groups reuse them, M10):
- **H1 "Registered Minecraft mobs can hold still while their goals run"** (`pandas-eating`):
  - `MonsterAi.still: Option<fn(&MonsterGoalContext) -> bool>` (struct at `monster_ai.rs:813`, `None` in `of_kind`, its only literal), set by the family right after it builds its `MonsterAi`. It is not a `MonsterSpec` field, so no other track's spec literal changes.
  - `monster_ai_step` skips `move_control.tick` (`world.rs:1064`) when it answers true. The check runs after `tick_goals` and keeps the faded inputs and the yaw (vanilla `PandaMoveControl`). Foxes use it for `FoxMoveControl` (M1-4).
- **H2 "Registered Minecraft mobs can scatter item crumbs"** (`pandas-eating`):
  - `ClientFx.item_crumbs: &mut Vec<(String, DVec3, Vec<Vec3>)>` (the type of `EntityView.item_crumbs`, `entity_view.rs:58`) in the terrain root, wired where `advance` builds `ClientFx`, so it drains through the existing `present_crumbs` path (`minecraft_world.rs:1612`).
  - Check that friends drain it as the host does. Foxes' eating crumbs (event 45) reuse it; M2's sulfur cube should use it too rather than a second crumbs field.
- **H3 "Living Minecraft mobs can drop gifts from loot tables"** (`pandas-moods`; turtles, armadillos and sniffers reuse it):
  - root `families.rs`: `EntityWorld::family_gift(table, this_type, at)` queues a gift, `take_family_gifts()` drains them (one list on the root `Families`);
  - `loot.rs`: `pub fn roll_table(&mut self, table, loot_type, this_type) -> Option<Vec<ItemStack>>` next to `roll_hero_gift` (`:276`), over the private `roll_of_type`;
  - one terrain line, `self.drop_family_gifts();`, right after `self.drop_cat_gifts();` (`server.rs:1107`); its body lives in terrain `families.rs`: `roll_table(table, "minecraft:gift", this_type)`, each stack through `level.spawn_at_location` (vanilla `dropFromGiftLootTable`, default pickup delay).
  - The sneeze calls `family_gift("minecraft:gameplay/panda_sneeze", "minecraft:panda", feet)` when `mob_drops` is on.
- **H4 "Minecraft mobs know when it rains and thunders"** (`pandas-thunder`; foxes and bees reuse it):
  - `player/src/lib.rs`: defaulted `fn raining(&self) -> bool { false }` and `fn thundering(&self) -> bool { false }` right after `fn rain_at` (`:189-191`);
  - world crate: `is_raining` (`level/fire.rs:65`) becomes `pub`, and `pub fn is_thundering(&self)` goes right after it with vanilla's `Level.isThundering` (`canHaveWeather` and `getThunderLevel(1)`, which is thunder × rain, above 0.9; javap during the work);
  - `MobWorld` implements both right after its `rain_at` (`server_mobs.rs:357`).
  - No `EntityWorld` field and no `world.rs` or `server.rs:1062` line: goals see `&dyn World`, so `isScared` reads `world.thundering()` where it runs. M3's lightning fixes the spawner's own value at `level/spawning.rs:162` and may call `is_thundering`; neither touches the other's lines.

**Multiplayer (`TerrainFamily`):**
- `TRACKING_RANGE` 10, `SHADOW_RADIUS` 0.9, the 14 `TEXTURES`, the other constants at their defaults.
- `read`/`fields`:
  - `MainGene`, `HiddenGene`, `InLove`, `Age`/`ForcedAge`, `CanPickUpLoot`, `equipment.mainhand`, `drop_chances.mainhand`;
  - the `max_health` and `movement_speed` bases read from `attributes` (20 and 0.15 when missing);
  - `fields` writes `attributes` only for a panda not loaded from a tag (a bred cub). Writing it for every panda would replace the loaded list and lose its follow-range spawn bonus.
- `VOLATILE`: `equipment`, `drop_chances` (`Age` and `InLove` are global, `mob_tracking.rs:15`).
- `FamilyShow`:
  - `flags`: bit 0 sitting, 1 on its back, 2 rolling, 3 sneezing, 4 scared;
  - `values`: [0] sneeze counter, [1] eat counter, [2] unhappy counter, [3] roll counter, [4] held item (0 none, 1 bamboo, 2 cake, the only items feeding and pickup put in its paw; 3 is any other item from a loaded tag, which friends do not draw).
  - `show_on` steps the three amounts from the flags, as vanilla's client does; it runs once per host tick.
- `interact`: calls the entity `pandas::interact` and fills `MobOutcome` (handled, inventory, the swapped stack in `drops`).
- `client_effects`: six crumbs every eat counter that is a multiple of 5, and one `Kind::Sneeze` when a sneeze ends. `fx.seen` makes each fire once per viewer.

**Commit groups:**

| Group | Commits |
|---|---|
| `pandas-live` | 1. Minecraft pandas live in the entity world with their genes. 2. Minecraft pandas spawn in bamboo jungles and are saved, hatched and seen by friends. |
| `pandas-drawn` | 1. Minecraft pandas are drawn like vanilla. |
| `pandas-eating` | 1. Registered Minecraft mobs can hold still while their goals run. 2. Registered Minecraft mobs can scatter item crumbs. 3. Minecraft pandas sit down to eat bamboo and cake. |
| `pandas-moods` | 1. Living Minecraft mobs can drop gifts from loot tables. 2. Minecraft panda cubs sneeze, lazy pandas lie on their backs and playful pandas roll. |
| `pandas-thunder` | 1. Minecraft mobs know when it rains and thunders. 2. Minecraft worried pandas hide from thunder. |

- **live 1:**
  - Goals 0-6, 9, 10, 13, 14 and T1, the unhappy tail, and breeding as `wolf_breed` (genes from the parents, an orb of 1-7).
  - `interact` without the eat-from-hand branch, which passes until `pandas-eating`.
- **live 2:**
  - The row:
    - `info` as today's (`Rules::Animal`, `Walk::Animal`, cluster 4), creature, follow range 16, `baby_chance: Some(0.2)`, no `rule`;
    - `finalize`: two genes from the level random; weak gets `Health` 10 and a `max_health` base of 10, lazy a `movement_speed` base of 0.07 (`spawning.rs:365` `attribute`).
  - The `TerrainFamily` with an empty `draw`. Row `live`.
- **drawn:** both models, all poses and the held item (`HeldDisplay::Ground`, into `out.held`) on `posed_cube`. The model goes into `out.culled`, because the `EntityModel` default is `entityCutout`. Row `drawn`.
- **eating:** H1, H2, the sit goal, the pickup, `handleEating`, the eat-from-hand branch, the drop on stop through `spawn_at_location`, the crumbs, and W's `eat` game event if W has merged.
- **moods:**
  - 1: H3.
  - 2: the lie, sneeze and roll goals, `handleRoll` and `afterSneeze` (the neighbours jump with `jump_power(0.42)`; the slime ball through H3).
- **thunder** (split from `moods` so each group stays one agent's work):
  - 1: H4. 2: thunder and `isScared`.
  - Row `done`; the steps and the kit go into the log.

### Tests

- **live 1** (`world::families::land::pandas::tests`):
  - `a_panda_is_a_big_twenty_health_animal_with_its_kind`: size, eye, speed, attack, `entity_type`, census, `mob_candidates`, bamboo and 1-3 experience, nothing for a cub;
  - `a_cub_is_half_size_with_its_own_eye_height`;
  - `random_genes_follow_the_sixteen_slot_table`, `a_recessive_gene_shows_only_when_both_match`;
  - `a_cub_takes_a_gene_from_each_parent_and_mutates_one_time_in_thirty_two`;
  - `weak_pandas_have_ten_health_and_lazy_ones_walk_at_0_07`;
  - `pandas_breed_only_with_bamboo_within_seven_blocks_and_sulk_otherwise`;
  - `a_hit_panda_bites_back_once_unless_aggressive`, `only_aggressive_pandas_join_a_fight`, `bamboo_calms_an_angry_panda`;
  - `a_panda_panics_from_fire_and_cactus_but_not_from_a_punch`;
  - `a_worried_panda_runs_from_players_within_eight_and_monsters_within_four`;
  - `bamboo_grows_a_cub_and_brings_an_adult_into_love`, `a_tempted_panda_follows_bamboo`, `a_panda_refuses_a_lead`.
- **live 2:**
  - `natural_spawner::families::land::tests::a_panda_spawns_with_two_genes_and_later_group_members_are_cubs_one_time_in_five` (no `note_unsupported`; weak `Health` 10);
  - terrain `a_panda_saves_and_loads_with_its_genes_held_item_love_and_attributes`, `a_summoned_weak_panda_keeps_twenty_health`;
  - `a_friend_sees_a_panda_from_its_look_tag` (flags, values, range 10, `equipment` volatile).
- **drawn** (`pandas_render::tests`):
  - `an_adult_and_a_cub_have_nine_cubes_each_on_their_own_models`;
  - `each_gene_and_age_draws_its_texture`;
  - `a_sitting_panda_sits_upright_and_an_eating_one_works_its_paws`;
  - `a_rolling_panda_turns_a_full_circle_in_thirty_two_ticks`;
  - `a_panda_on_its_back_and_a_scared_panda_pose_like_vanilla`;
  - `a_sneezing_panda_holds_its_head_back_from_tick_fifteen`;
  - `a_sitting_panda_holds_its_item_on_the_ground_display_unless_scared`.
- **eating:**
  - `a_registered_mob_held_still_keeps_its_heading_and_drifts_to_a_stop` (H1);
  - `a_registered_family_can_scatter_item_crumbs_for_every_viewer` (H2);
  - `a_panda_picks_up_a_whole_stack_of_bamboo_or_a_cake_only_with_mob_griefing_and_an_empty_paw`;
  - `a_sitting_panda_eats_after_eighty_ticks_and_finishes_after_a_hundred`;
  - `feeding_a_panda_in_love_seats_it_with_one_bamboo_and_returns_its_old_item`;
  - `a_panda_that_stops_sitting_drops_its_item_and_waits`;
  - `a_held_item_drops_on_death_always_when_picked_up`;
  - `a_worried_panda_does_not_settle_on_picked_up_bamboo`;
  - `an_eating_panda_scatters_six_crumbs_every_five_ticks`.
- **moods:**
  - `a_weak_cub_sneezes_and_adults_within_ten_jump`;
  - H3: `families::tests::a_queued_family_gift_rolls_its_table_and_drops_at_its_feet` (terrain, gated on `MINECRAFTOSS_ROOT` for the real table);
  - terrain `a_sneeze_drops_a_slime_ball_about_once_in_seven_hundred` (none without `mob_drops`);
  - `a_lazy_panda_lies_on_its_back_until_clicked`;
  - `a_playful_panda_rolls_with_three_hops_in_thirty_two_ticks`, `a_playful_panda_rolls_off_a_ledge`;
  - `ambient_particles::tests::a_sneeze_is_a_green_translucent_cloud`.
- **thunder:**
  - H4: `server_mobs::tests::mob_worlds_know_when_it_rains_and_thunders` (thunder needs rain too);
  - `a_worried_panda_sits_and_trembles_in_thunder_and_refuses_bamboo`.

### Owner's steps

Setup: release build, survival, **Normal**, by day, flat grass, `/gamerule mob_griefing true`, the kit. Name screenshots by label (two windows: `pd10-host.png`, `pd10-friend.png`).

1. Summon seven pandas 3 blocks apart with `/summon minecraft:panda ~ ~ ~3 {MainGene:"normal",HiddenGene:"normal"}`, then the same with `lazy`, `worried`, `playful`, `brown`, `weak` and `aggressive`. You see seven faces: plain, sleepy, frowning, tongue out, brown fur, runny nose, angry brows. They amble slowly and look at you within 6 blocks. **pd1_genes**
2. Use the panda egg 10 times: mostly plain faces, a few sleepy, worried, playful or angry, and brown and weak are rare. About one in ten is weak: two iron sword hits without jumping kill it, while a plain one takes four.
3. Breed:
   1. Put bamboo on grass 4 blocks from two egg pandas and bone-meal it into a stalk.
   2. Feed both bamboo: they meet, and a cub and experience appear. **pd2_cub**
   3. Feed the cub bamboo: it grows faster.
   4. Right-click it with a golden dandelion: it stops growing.
4. Break the stalk, stand within 8 blocks and feed two other adults: they shake their heads, a sad sound plays twice, they look at you, and no cub comes.
5. Feed a parent bamboo again within 5 minutes: it sits, holds the bamboo in its paws and munches, with crunching and green crumbs. It may pause and start again. Within about 5-15 s the bamboo is gone and it stands. **pd3_eating**
6. Pick-ups:
   1. `/summon minecraft:panda ~ ~ ~3 {CanPickUpLoot:1b}` and throw one bamboo (Q) 4 blocks away: it fetches it, sits and eats.
   2. Throw a cake: the same.
   3. With `/gamerule mob_griefing false` it walks over and leaves the bamboo. Set it back to true.
7. Fights:
   1. Punch the plain panda: it bites once (about 3 hearts) and wanders off.
   2. Summon two more aggressive pandas beside the first and punch one: all three keep attacking.
   3. Right-click an attacker with bamboo: it stops. **pd4_bite**
8. Light the grass beside a panda: it runs. A panda walking into a cactus (on sand) runs too. A punch never makes it run.
9. Walk at the worried panda: it runs within 8 blocks. Hatch a zombie 3 blocks from it: it flees the zombie.
10. `/weather thunder` and wait 5 s: the worried panda sits, covers its face and trembles, and bamboo does nothing. **pd5_thunder** After `/weather clear` it stands up.
11. Within a minute the lazy panda flops onto its back and kicks. Right-click it: it gets up. **pd6_lazy**
12. The playful panda somersaults every few seconds, and tumbles off the edge of a 2-high cobblestone platform. **pd7_roll**
13. Sneeze:
    1. `/summon minecraft:panda ~ ~ ~3 {Age:-24000,MainGene:"weak",HiddenGene:"weak"}` beside two adults.
    2. Within a minute or two it throws its head back with a green puff, and the adults jump. **pd8_sneeze**
    3. One sneeze in 700 drops a slime ball.
14. Kill an adult with the sword: bamboo and experience drop, plus any picked-up bamboo it held. A cub drops nothing. The M4A1 and the bow make pandas flash and die.
15. Save and quit with a cub, a lazy, a weak and a playful panda, then load: the same faces, the cub still small, and the weak one still dies to two hits.
16. `/locate biome minecraft:bamboo_jungle`, `/tp` there and walk into new chunks: pandas in ones and twos among the bamboo, sometimes a cub. **pd9_jungle**
17. In creative, put a panda egg on a spawner: a small panda turns in the cage. Switch back to survival.
18. Friend window:
    1. The friend sees the faces, eating with bamboo in the paws and crumbs, the lazy panda on its back, the roll and the green puff, at the same moments. **pd10-host**, **pd10-friend**
    2. The friend feeds a panda: the host sees it sit and eat.
    3. The friend shoots the aggressive panda with the M4A1: it chases the friend in both windows.

### Kit

```
IW4L_MINECRAFT_KIT=minecraft:panda_spawn_egg*16,minecraft:bamboo*64,minecraft:bone_meal*32,minecraft:cake,minecraft:cake,minecraft:iron_sword,minecraft:bow,minecraft:arrow*64,minecraft:shield,minecraft:flint_and_steel,minecraft:cactus*4,minecraft:sand*8,minecraft:cobblestone*64,minecraft:zombie_spawn_egg*2,minecraft:spawner,minecraft:golden_dandelion*2,minecraft:cooked_beef*32
```

17 stacks (a cake stacks to 1). Every id is in the 26.3 `assets/minecraft/items`. The friend uses the same kit.

### Depends on

- **D0 d0a** (`MonsterSpec`, `GoalExtra`, `FamilyShow`, `interact`, `client_effects`). Not d0b.
- **E1 `particles-events`** for the hearts (event 18, the default row).
- **W P7** for the `eat` game event: one line, added by whichever merges second.
- **E2 `ti1`** for thrown items (generic).
- **M1-1 rabbits**: `animal_goals.rs` (`is_monster_class`, panic, tempt, breed).
- **M3 lightning (P8)**: corrects `level/spawning.rs:162`; H4's `is_thundering` already follows vanilla, so the two never touch the same line.

### Left for later

- Eggs read `CanPickUpLoot` as false (`spawning.rs:896` serves eggs and `/summon` alike), so egg pandas do not pick up items. Vanilla eggs keep the constructor's true. The bug is fork-wide (villagers, piglins and foxes too), so the steps use `{CanPickUpLoot:1b}` instead.
- Dispensers filling a panda's paw (`canDispenserEquipIntoSlot`). The pickup's flight animation.
- D0's `monster_ai_step` adds `Monster`'s +2 idle in light for every goal family, creatures included. That is D0's fix to make.

### Open decisions (the vanilla-like choice recommended)

1. **Holding still:** use H1 (`MonsterAi.still`, checked after the goals), not a separate panda AI step that would duplicate `monster_ai_step` and D0's sync, and not a `MonsterSpec` field, which would break every other track's spec literal.
2. **Weather:** H4's `World::raining`/`World::thundering`, read through `MobWorld` from the level (vanilla `isRaining`/`isThundering`), rather than an `EntityWorld.thundering` field set beside `bright_outside`: no `world.rs` line, and goals can read it.
3. **Attributes:** keep weak and lazy in the saved `attributes` (vanilla sets them only on finalize and breeding), not derived from the genes on load.
4. **Sneeze gift:** roll from the loot book's named sequence in terrain through H3, not from the mob's random.
5. **Crumbs:** H2 into the existing item-crumb path, not a new particle kind.

---

## M1-4. Foxes (segment m1c: groups `foxes-live`, `foxes-drawn`, `foxes-hunt`, `foxes-items`, `foxes-trust`; family `foxes`, `minecraft:fox`, hit key 49)

This is row 16 of `docs/round10/mobs.md`. The fox is an `Animal` on the goal framework, like the wolf (`wolf_ai.rs`). It registers as `MonsterKind::Registered("minecraft:fox")` with a `MonsterSpec` (D0-2.4) and keeps its goal state in `GoalExtra`. It needs only D0's segment d0a, and builds on M1-1 (`animal_goals.rs` with `MoveToBlockGoal`, `family_goal_step_with`) and M1-3 (H1 holding still, H2 crumbs, H4 thunder).

### What exists today (`C:\Dev\Soscraft\mashup-crafting`, `minecraft-survival`)

- **Spawning is already complete and matches 26.3** (checked against the javap below), so there is **no `families/land.rs` row** (D0-4: built-in rows keep working):
  - `natural_spawner.rs:284` size 0.6 × 0.7, `NoRestrictions`, `MotionBlockingNoLeaves`;
  - `:331` `Rules::SpawnableOn("minecraft:foxes_spawnable_on")`, with the light check `raw_brightness > 8` at `:780`;
  - `natural_spawner/spawning.rs:102` category `Creature`;
  - `natural_spawner.rs:412`/`:549` `#spawns_snow_foxes`;
  - `natural_spawner.rs:744-746` immune to the bush when choosing a spawn block;
  - `finalize_spawn` fox arm `natural_spawner.rs:935-971`: group variant, cubs from the third fox, the 20 % mouth item with vanilla odds, follow range 32;
  - `Group::Fox` `natural_spawner.rs:1117`, with no baby draw at `:1133`.
- **Dormant today**: the fox is not in `SIMULATED_KINDS` (`server_mobs.rs:433`). Natural, chunk and saved foxes therefore stay dormant, and eggs fail. Once registered, old dormant foxes load through the generic `spawn_saved`.
- **Wolves already hunt foxes**: `Prey::WolfPrey` (`monster_ai.rs:427`, `:2961`) reads `mob_candidates`, which D0's arm fills.
- **One test uses the fox as the kind that cannot hatch**: `server.rs:7690` (`:7703` and `:7708` expect `hatch_egg("minecraft:fox")` to fail). `foxes-live` commit 3 (the one that registers the terrain family and so makes the fox hatch) switches both lines to `minecraft:illusioner`, which has no row, is never simulated and is in no round 10 track. Two other tests mention the fox but use fake registries and stay unchanged: `minecraft_commands/src/commands.rs:1724` and `render_anim/src/minecraft_chat.rs:225`.
- **Code to reuse**:
  - Goals in `monster_ai.rs`: `FloatGoal` `:2015`, `LeapGoal` `:1108`, `MeleeAttackGoal` `:2239`, `StrollGoal` `:2332`, `LookAtPlayerGoal` `:2366`, `NearestTargetGoal` `:2909`.
  - `FleeSunGoal` `:1690` uses the monster walk value.
  - `AvoidEntityGoal` `:1735` has no per-candidate filter. Precedent for a family's own avoid goal: `feline_ai.rs:495`.
  - Also `panic.rs:9`, `follow_parent.rs:17` and `breed.rs:7`.
  - Animals:
    - `animal::interact` (`animal.rs:12`): food, growth, golden dandelion;
    - `wolf_breed` (`world/wolves.rs:279`): cub, event 18 at `:309`, experience draw at `:301-303`.
  - Players:
    - `player_controls` (`world.rs:5242`): `sneaking` and `sleeping`, `controls.rs:2-15`;
    - `player_fights` (`world.rs:3444`) and `uuid_of` (`:4033`);
    - UUID saves like the wolf's `Owner` (`server_mobs.rs:1036`).
  - Items: `World::items_in`/`take_item`/`spawn_item`/`spawn_popped_item` (`player/src/lib.rs:105-121`, served by `MobWorld` `server_mobs.rs:170-218`). Equipment is saved as in `piglin_save.rs:165-233`.
  - Render:
    - `wolf_render.rs` (64×32 adult, 32×32 baby, `interested_angle` head roll `:204`);
    - `witch_render.rs:122-142` (an item at the head, `HeldDisplay::Ground`);
    - `keyframes.rs:328` `apply_walk`;
    - `block_particles.rs:185` `spawn_item_crumbs` with `entity_view.rs:58` `item_crumbs`;
    - `leash_render.rs:72` `leash_offset`, which has no fox arm.
- **Missing shared pieces**:
  - Bushes hold and prick every mob, with no kind check (`inside_blocks.rs:485-491`, `hazards.rs:87-106`).
  - `LookControl::tick_limited` (`look.rs:144`) always resets the pitch.
  - No `MoveToBlockGoal` existed before M1-1, which ports it into `animal_goals.rs`; the zombie's `TurtleEggGoal` (`monster_ai.rs:2185`) keeps only its timer.
  - `level.destroyed` (`world/src/level/mod.rs:218`) gets real breaks only.
  - `ClientFx` had no item crumbs and the entity world no thunder (`grep thunder` finds only `mooshroom.rs:91`) before M1-3's H2 and H4.

### Vanilla 26.3 (javap)

**Type and attributes**
- **`EntityTypes.FOX`**: `CREATURE`, `sized(0.6, 0.7)`, `eyeHeight(0.4)`, passenger attachment (0, 0.6375, −0.25), `clientTrackingRange(8)`, `immuneTo(#fox_immune_to)` (= `sweet_berry_bush`). Not `fireImmune`, not `notInPeaceful`.
- **Baby** (`BABY_DIMENSIONS`): 0.36 × 0.42. The eye height is set to **0.34375**, not scaled. Passenger attachment (0, 0.375, 0).
- **`createAttributes`**: animal base (temptation range 10), speed 0.30000001192092896, health 10, attack 2, **`SAFE_FALL_DISTANCE` 5**, follow range 32.
- **Constructor**: `FoxLookControl`, `FoxMoveControl`, malus `DAMAGING` and `DAMAGING_IN_NEIGHBOR` 0, `setCanPickUpLoot(true)`, `setRequiredPathLength(32)`.
- **From `Animal`**: ambient interval 120, `removeWhenFarAway` false, experience `1 + nextInt(3)`, `actuallyHurt` → `resetLove`.

**Data and saving**
- **Variant**: `DATA_TYPE_ID` 0 red, 1 snow.
- **Flags byte**: 1 sitting, 4 crouching, 8 interested, 16 pouncing, 32 sleeping, 64 faceplanted, 128 defending.
- **Saved fields**:
  - fox: `Trusted` (≤ 2 UUID int arrays), `Sleeping`, `Type` (`"red"`/`"snow"`), `Sitting`, `Crouching`;
  - inherited: `InLove`, `LoveCause`, `Age`, `ForcedAge`, `AgeLocked`, `CanPickUpLoot`, `equipment.mainhand`, `drop_chances`.
- **Loading**: `readAdditionalSaveData` clears trust, reads, then calls `setTargetGoals`.

**Goals**

| Priority | Goal |
|---|---|
| 0 | `FoxFloatGoal`: deeper than 0.25 or lava; `start` clears states |
| 0 | `ClimbOnTopOfPowderSnowGoal` |
| 1 | `FaceplantGoal` |
| 2 | `FoxPanicGoal(2.2)`: not while defending |
| 3 | `FoxBreedGoal(1.0)` |
| 4 | Avoid `Player` (16, 1.6, 1.4): not sneaking, creative or spectator, not trusted, fox not defending |
| 4 | Avoid `Wolf` (8, 1.6, 1.4): untamed, fox not defending |
| 4 | Avoid `PolarBear` (8, 1.6, 1.4): fox not defending |
| 5 | `StalkPreyGoal` |
| 6 | `FoxPounceGoal` |
| 6 | `SeekShelterGoal(1.25)` |
| 7 | `FoxMeleeAttackGoal(1.2, true)` |
| 7 | `SleepGoal` |
| 8 | `FoxFollowParentGoal(1.25)` |
| 9 | `FoxStrollThroughVillageGoal(200)` |
| 10 | `FoxEatBerriesGoal(1.2, 12, 1)` |
| 10 | `LeapAtTargetGoal(0.4)` |
| 11 | `WaterAvoidingRandomStrollGoal(1.0)` |
| 11 | `FoxSearchForItemsGoal` |
| 12 | `FoxLookAtPlayerGoal(24)`: not faceplanted or interested |
| 13 | `PerchAndSearchGoal` |

**Targets**
- **3 `DefendTrustedTargetGoal`** (interval 10):
  - picks a trusted entity's `getLastHurtByMob`, once that mob's stamp has changed;
  - the attacker must have hurt something within 600 ticks and must not be trusted itself;
  - `start`: `entity.fox.aggro`, defending, wake.
- **`setTargetGoals`** runs only from `finalizeSpawn` and loading. **A bred cub therefore has no prey goals until it is reloaded.**
  - Red fox: 4 chicken or rabbit (interval 10), 4 baby turtle on land (10), 6 schooling fish (cod, salmon, tropical fish; 20).
  - Snow fox: fish 4, land 6, turtle 6.

**Tick, aiStep and controls**
- **`aiStep`**:
  - `ticksSinceEaten++`.
  - Held food (with `food` and `consumable`, no target, on ground, not sleeping): above 600 ticks, `finishUsingItem`, then reset the count; above 560, with `nextFloat < 0.1`, `entity.fox.eat` and entity event 45.
  - No live target: crouching and interested off.
  - Sleeping: `xxa = zza = 0`, no jump.
  - Defending: `nextFloat < 0.05` → aggro.
- **`tick`**:
  - Water, a target or thunder → wake. Water or sleeping → sitting off.
  - Faceplanted: level `nextFloat < 0.2` → level event 2001 at its block.
  - `interestedAngle += (flag − angle) × 0.4`.
  - `crouchAmount` +0.2 a tick while crouching (cap 5), else 0.
- **Controls**: `FoxMoveControl` moves only if not sleeping, sitting or faceplanted. `FoxLookControl` skips its tick while sleeping. It keeps the pitch (no `resetXRotOnTick`) while pouncing, crouching, interested or faceplanted. The fox has no head-rotation overrides.

**Goal details**
- **`SleepGoal`** (MOVE, LOOK, JUMP):
  - The countdown is `nextInt(70)`.
  - `canUse` needs `xxa = yya = zza = 0` and either `canSleep` or already sleeping.
  - `canSleep`: countdown done, bright outside, `hasShelter`, not `alertable`, not in powder snow. `hasShelter` means no sky at `(x, box.maxY, z)` and a walk value ≥ 0.
  - `start`: everything off, sleeping on, path stopped. `stop`: a new countdown, `clearStates`.
- **`alertable`**: any living entity in the box grown 12/6/12 and within 12 blocks, sight not needed:
  - never a fox;
  - always chickens, rabbits and any `Monster`;
  - tame pets never;
  - players unless creative or spectator;
  - never a trusted one;
  - anything else unless it is sleeping or sneaking, so a walking cow or villager wakes it.
- **`SeekShelterGoal`**:
  - Not while sleeping or with a target.
  - Thunder under open sky → seek shelter. Otherwise every 100 ticks (the first after 50): bright, open sky, not `isVillage` → `FleeSunGoal`'s 10 tries with the **animal** walk value at 1.25.
  - `start` clears states.
- **`PerchAndSearchGoal`**: 2 % a tick, with no hurt-by, not sleeping, no target, path done, not alertable, not pouncing or crouching. It sits and looks 2–4 times at `2π·nextDouble`, each look `adjustedTickDelay(80 + nextInt(20))`.
- **`StalkPreyGoal`**:
  - Target: a live chicken or rabbit farther than 6 blocks, while the fox is not crouching, interested or jumping. It walks there at 1.5.
  - Within 6 blocks: crouch, interested, stop.
  - `stop` keeps the crouch only if `isPathClear`: six steps along the line, blocks 1–3 above replaceable.
- **`FoxPounceGoal`** (not interruptable):
  - Starts when fully crouched (`crouchAmount == 5`), the target is alive and the path is clear. The motion-direction test is always true for living targets.
  - `start`: velocity += (dx × 0.8, 0.9, dz × 0.8), look 60/30.
  - In flight the pitch is `−signum(vy·f)·acos(h/len)·57.2957763671875` (`f` 6.5 while rising), eased to 0 by `rotLerp(0.2)` when `vy² < 0.03`. Within 2 blocks it calls `doHurtTarget`.
  - Landing in a `minecraft:snow` layer: pitch 60, target cleared, faceplanted.
  - `stop` clears the crouch (and `crouchAmount`), interest and pouncing.
- **`FaceplantGoal`**: 40 ticks.
- **`FoxMeleeAttackGoal`**: not sitting, sleeping, crouching or faceplanted. Each hit plays `entity.fox.bite`.
- **`FoxEatBerriesGoal`** (`MoveToBlockGoal`):
  - Reach 2, re-path every 100 tries. Targets: a bush of age ≥ 2, or vines with berries. It waits 40 ticks at the block, and walking there it sniffs (5 % a tick).
  - With `mob_griefing`, a bush gives `1 + nextInt(2) + (age == 3)` berries (level random). One goes into an empty mouth, the rest pop out. Then `block.sweet_berry_bush.pick_berries`, age 1, game event `block_change`.
  - Vines: `CaveVines.use` (one glow berry, `block.cave_vines.pick_berries`, `berries=false`).
- **`FoxSearchForItemsGoal`**: empty mouth, no target or hurt-by, `nextInt(5) == 0`, an item within 8 blocks with no pickup delay. It walks to the first one at 1.2.
- **`FoxStrollThroughVillageGoal`**: night only, `nextInt(100) == 0`, `isCloseToVillage(pos, 6)`, `LandRandomPos.getPos(15, 7, −sectionsToVillage)`. Not while sleeping, sitting, defending or targeting.
- **`FoxBreedGoal`**:
  - The cub takes the variant of either parent (`nextBoolean`) and trusts both love causes.
  - Parents: age 6000, love reset. Cub: age −24000 at the parent.
  - Event 18, and `nextInt(7)` experience with `mob_drops`.

**Items**
- **Pickup** (`Mob.aiStep`, needs `mob_griefing`): items within its box grown (1, 0, 1), if `canHoldItem`. That means an empty mouth, or `ticksSinceEaten > 0` with new food while holding non-food.
- **`pickUpItem`**:
  - Drops the extra count at its feet.
  - Spits the held item out: at `look + (0, 1, 0)`, pickup delay 40, `entity.fox.spit`.
  - Takes one item, with a guaranteed drop, and sets `ticksSinceEaten = 0`.
- **On death**: the held item always drops, even without mob loot.

**Interact**: no override. `Animal.mobInteract` with `#fox_food` (sweet berries, glow berries). A spawn egg used on a fox makes a cub that trusts the player.

**Sounds and events**
- **Sounds**:
  - `entity.fox.ambient`;
  - `.sleep` while sleeping;
  - `.screech`: at night, 10 %, no non-spectator player within its box grown 16, volume 2, voice pitch;
  - `.hurt`, `.death`, `.eat`, `.spit`, `.bite`, `.aggro`, `.sniff` at (1, 1).
- **Source**: neutral.
- **Event 45**: 8 `ITEM` particles of the held stack at `(x + look.x/2, y, z + look.z/2)`. Velocity `((r − 0.5)·0.1, r·0.1 + 0.1, 0)`, turned by −xRot and −yRot, then y + 0.05.
- **Leash offset**: `(0, 0.55 × eye, 0.4 × width)`.

**Spawning**
- `NO_RESTRICTIONS`, `checkFoxSpawnRules`: `#foxes_spawnable_on` (grass block, snow, snow block, podzol, coarse dirt) and bright enough.
- Biomes (2–4 foxes per group): taiga, snowy taiga, old growth pine taiga and old growth spruce taiga at weight 8; grove and dappled forest at 4.
- Loot `entities/fox` is empty. The fox is in `#powder_snow_walkable_mobs`.

**Render**
- **`FoxRenderer`**: shadow 0.4.
  - Adult: `AdultFoxModel` (48×32): head with ears and nose, body turned 90° with the tail as its child, four legs.
  - Baby: `BabyFoxModel` (32×32) with `FoxBabyAnimation.FOX_BABY_WALK` (0.5 s, looping, `applyWalk(pos, speed, 1, 2.5)`).
  - Textures: `fox`, `fox_snow`, their `_sleep` and `_baby` versions.
  - `setupRotations`: rotates X by −xRot while pouncing or faceplanted.
- **`setupAnim`**:
  - head roll `lerp(interestedAngle)·0.11·π`;
  - crouch: body +6°, head lowered by `crouchAmount`, a `cos(age)·0.05` wiggle;
  - sleep: legs hidden, head −120°;
  - also sitting, pouncing, and paddling legs while faceplanted.
  - Port each `set*Pose` with javap during the work.
- **`FoxHeldItemLayer`**: the held stack shown as `GROUND`, at the head pivot.
  - Baby × 0.75, turned by roll, yaw and pitch.
  - Offset: adult (0.06, 0.27, −0.5), adult asleep (0.46, 0.26, 0.22), baby (0.06, 0.26, −0.5), baby asleep (0.4, 0.26, 0.15).
  - X 90°, and Z 90° while asleep.

### Plan

**Files (D0-3)**
- **`entities/src/world/families/land/foxes.rs`**:
  - `pub struct Fox`: variant, flags, `trusted: Vec<u128>`, `mouth`, ticks since eaten, angles and crouch amount with their old values, `age`, `in_love`, `love_cause`, `eat_events: u32`, the interested and crouch start ticks, the `MonsterAi`;
  - `impl Family for Fox`, the tick and the tests.
- **`foxes/ai.rs`**:
  - `FOX_SPEC` and the goals; `FoxEatBerriesGoal` on M1-1's `MoveToBlockGoal` (`animal_goals.rs`);
  - `FoxGoals` in `GoalExtra`: players' sneaking, sleeping and creative from `player_controls`; trusted player ids; tame wolf and cat ids; the items from `items_in`; the berry block to change; the breed partner.
- **Terrain**: `crates/minecraft_terrain/src/families/land/foxes.rs` (`TerrainFamily`) and `foxes/render.rs`.
- **Registration**: `pub mod foxes;` and `foxes: foxes::Fox,` in `land.rs`; `mod foxes;` and `Fox` in `terrain_group!`; row 16 of `mobs.md`.

**Hooks used**
- `KEY = 49`, `KINDS = ["minecraft:fox"]`.
- `spawn`: adult or baby size and eye height.
- `tick`.
- `category`: `"creature"`.
- `hurt`: love reset, then `hurt_plain`.
- `death_drops`: the mouth item, always.
- `despawn`: `removable: false`, if D0's creature default differs.
- `leashable` with `leash_walk`.
- `panicking`.
- `voice`: `("fox", 1, neutral)`; `core.ambient_interval` 120 (`Animal`). Right after `family_base_tick` the tick turns a `Voice::Ambient` into `entity.fox.sleep` while sleeping, or into `entity.fox.screech` (volume 2) by the night rule above (the track's ambient rule, M10).
- `monster`: `FOX_SPEC`.
- `family_strike`: `PlayerHitKind::Melee` for players, `mob_hits_mob` for prey.

**AI notes**
- **`FOX_SPEC.walk`**: `WalkTarget::Animal`, the two `DAMAGING` maluses 0, `can_float`, `max_visited` 512, `max_fall_distance` 5. Undo any monster-only default from `of_kind`.
- **Avoiding**: the fox's own `FoxAvoidGoal`, since `MobCandidate` has no tame flag.
- **Prey targets**:
  - land: `Prey::Kinds(&["minecraft:chicken", "minecraft:rabbit"])`;
  - fish: `Prey::Kinds(&["minecraft:cod", "minecraft:salmon", "minecraft:tropical_fish"])`;
  - turtles: `Prey::Absent`, until M1-8's `turtles-hunted` switches this slot to `Prey::BabiesOnLand`.
  - They are added at the variant's priorities from `TerrainFamily::read` (eggs, `/summon`, natural spawns, loads), never for bred cubs.
- **Looking**: a new `LookControl::tick_from(.., reset_pitch)` beside `tick_limited` (`look.rs:144`), which calls it with `true`. No look tick while sleeping. The pounce writes `look.pitch`, and `MobPose.head_pitch` carries it.
- **Falling**: D0's `family_goal_step` applies the generic fall damage (safe distance 3) itself, so the fox's goal step runs through `family_goal_step_with` (M1-1) with a `run` that calls `monster_ai_step` and shortens the landed distance it returns by `2 / reduction` (the landed block's fall reduction). That gives vanilla's safe distance of 5. The block's numbers come from E1's `horse-fall` helper in `living.rs` (segment e1a; it returns the landed block's `(fall_reduction, fall_multiplier)`); if e1a has not merged, `foxes-live` makes `fall_multiplier` and `fall_reduction` (`living.rs:55`, `:65`) `pub(crate)`, one word each, and goats reuse them.
- **Holding still**: `FoxMoveControl` moves only when the fox is not sleeping, sitting or faceplanted: M1-3's `MonsterAi.still` with a test on `FoxGoals`.
- **Thunder**: `world.thundering()` (M1-3's H4) wakes the fox in `tick`, and `SeekShelterGoal` takes its thunder branch under open sky.
- **Items**: the fox calls `World::items_in`, `take_item` and `spawn_item` directly; no drop hook is needed.

**Shared lines**
- `leash_render.rs:76`: `"fox" => (0.55, 0.4),` beside the ocelot.
- `server.rs:7703` and `:7708`: replace the fox with `minecraft:illusioner`.
- `server_mobs.rs:258`: `MobWorld::destroy_effect` beside `destroy_block`. It pushes `(pos, state)` to `level.destroyed`, which already reaches host and friends as `BREAK_EVENT` bursts with the break sound (`minecraft_world.rs:1234`, `minecraft_levels.rs:519`).

**Hook commits** (D0-3: each a missing hook in its own commit)
1. **"Registered Minecraft mobs can be spared by the blocks they walk through"** (`foxes-live` commit 1):
   - `Family::spared_blocks() -> &'static [&'static str]`, default `&[]`;
   - a defaulted `Exposed::spared_blocks` beside `fire_immune` (`hazards.rs:22`), passed on by `blocks_act` (`:90`);
   - `block_effects` and `inside_block` (`inside_blocks.rs:472`) skip spared blocks;
   - `survival.rs:599` passes `&[]`.
   - The fox gives `["minecraft:sweet_berry_bush"]`, and bees reuse it.
2. **"Mobs can show a block's break effect without breaking it"** (`foxes-hunt` commit 1): a defaulted `World::destroy_effect` beside `destroy_block` (`player/src/lib.rs:98`). This is vanilla's level event 2001.

No ambient hook: the fox remaps `Voice::Ambient` in its own tick (M10). The crumbs are M1-3's H2.

**Multiplayer**
- **Constants**: `TRACKING_RANGE = 8`, `SHADOW_RADIUS = 0.4`, the eight textures.
- **`VOLATILE`**: `Sleeping`, `Sitting`, `Crouching`. `Age` and `InLove` are global volatile keys already. `Type`, `Trusted` and `equipment` stay in the look tag: they change seldom.
- **`read`/`fields`**: `Type`, `Trusted`, `Sleeping`, `Sitting`, `Crouching`, `InLove`, `LoveCause`, `CanPickUpLoot`, `equipment.mainhand` and `drop_chances`. `read` also sets the target goals.
- **`show`**: `FamilyShow { flags: vanilla byte, values: [interested_start, interested_angle_at_start × 10000, crouch_start (−1 when not crouching), eat_events, 0, 0, 0, 0] }`. From these clocks, host and friend each rebuild the angle (0.4 a tick toward the flag) and the crouch amount (0.2 a tick, cap 5), lerped with the partial tick.
- **`client_effects`**: event 45's crumbs through H2's `ClientFx.item_crumbs` when `eat_events` grows (tracked in `fx.seen`).
- **Elsewhere**: sounds go through `voices`, faceplant bursts through `level.destroyed`, hearts through E1's event 18.

**Keyframes**: `keyframes::shared().definition("FoxBabyAnimation", "FOX_BABY_WALK")` with `apply_walk(pos, speed, 1.0, 2.5)`.

**Commit groups**

| # | Key | Commits | Status |
|---|---|---|---|
| 1 | `foxes-live` | 1. "Registered Minecraft mobs can be spared by the blocks they walk through". 2. **"Minecraft foxes live in the entity world"**: size; float; powder snow climb; panic; avoiding players, wolves and polar bears; shelter; sleeping by day; perching; village stroll; stroll; looking; following the parent; sounds; leash; the mouth item as state; the fall; `tick_from`; thunder; holding still. 3. **"Minecraft foxes are saved, hatched and seen by friends"**: `TerrainFamily` with an empty `draw`; the egg test switched; a spawn test. | `live` |
| 2 | `foxes-drawn` | 1. **"Minecraft foxes are drawn like vanilla"**: both models; poses; sleep textures; baby walk keyframes; the tilt; the leash arm. 2. **"Minecraft foxes show what they carry in their mouths"**. | `drawn` |
| 3 | `foxes-hunt` | 1. **"Mobs can show a block's break effect without breaking it"**. 2. **"Minecraft foxes stalk, pounce on and bite chickens and rabbits"**: prey goals by variant; stalk; pounce; melee and leap with the bite; faceplant in snow. | |
| 4 | `foxes-items` | 1. **"Minecraft foxes pick up, spit out and eat items"** (crumbs through H2). 2. **"Minecraft foxes pick sweet berries and glow berries"**. | |
| 5 | `foxes-trust` | 1. **"Minecraft foxes breed on berries and their cubs trust the players who fed them"**: `interact` through `animal::interact`; the breed goal; trust. 2. **"Minecraft foxes defend the players they trust"**. The owner's steps and kit go into the log. | `done` |

### Tests

**`foxes-live`**
- Hook: `inside_blocks::tests::a_spared_block_neither_holds_nor_pricks`, `families::tests::a_registered_mob_spared_by_berry_bushes_walks_through_unhurt`.
- `a_fox_wakes_in_a_thunderstorm_and_seeks_cover_under_open_sky`
- `a_fox_is_small_with_ten_health_and_its_kind`: sizes, eyes 0.4 and 0.34375, attributes, `entity_type`, census, candidates, loot table, experience 1–3 (0 for a baby).
- `a_fox_flees_a_walking_player_but_not_a_sneaking_or_creative_one`
- `a_fox_flees_wild_wolves_and_polar_bears_but_not_tame_wolves`
- `a_fox_sleeps_by_day_under_cover_and_wakes_when_something_walks_near`: within 70 ticks; a cow wakes it, a sneaking or trusted player does not; it does not sleep at night.
- `a_fox_seeks_shelter_from_the_sun_but_not_in_a_village`
- `a_fox_wakes_and_stands_up_in_water`
- `a_fox_perches_and_looks_around_two_to_four_times`
- `a_hurt_fox_panics_and_falls_out_of_love`
- `a_fox_screeches_at_night_only_with_no_player_within_sixteen`
- `a_fox_walks_through_sweet_berry_bushes_unhurt_and_unslowed`
- `a_fox_takes_no_damage_from_a_five_block_fall`
- `a_fox_cub_follows_its_parent`
- `a_leashed_fox_is_pulled`
- Terrain:
  - `a_fox_saves_and_loads_with_its_type_trust_sleep_sit_crouch_love_and_mouth_item`;
  - `a_friend_sees_a_fox_from_its_look_tag` (flags and clocks);
  - `a_dormant_fox_from_an_old_save_wakes`.
- World: `foxes_spawn_on_podzol_in_light_in_groups_with_a_snowy_variant_in_snowy_biomes`.

**`foxes-drawn`**
- `an_adult_and_a_cub_have_their_parts_and_textures`
- `a_sleeping_fox_hides_its_legs_and_closes_its_eyes`
- `a_crouching_fox_lowers_its_head_by_its_crouch_clock`
- `the_cub_walk_matches_the_jar` (gated)
- `a_held_item_sits_at_the_mouth_turned_sideways_when_asleep`

**`foxes-hunt`**
- Hook: `server_mobs::tests::a_destroy_effect_shows_the_break_burst_and_keeps_the_block` (the default `World::destroy_effect` does nothing; `MobWorld`'s pushes `(pos, state)` to `level.destroyed` and the block stays)
- `a_red_fox_stalks_a_chicken_crouches_within_six_and_pounces_when_fully_crouched`
- `a_pounce_that_lands_within_two_bites`
- `a_pounce_into_snow_faceplants_for_forty_ticks_with_break_effects`
- `a_snow_fox_puts_fish_first`
- `a_blocked_pounce_paths_instead`

**`foxes-items`**
- `a_fox_picks_up_only_with_mob_griefing`
- `a_fox_spits_a_non_food_item_for_food`
- `a_fox_eats_its_food_after_600_ticks_with_crumbs_from_560`
- `a_dead_fox_drops_what_it_held_even_without_mob_loot`
- `a_fox_picks_a_ripe_bush_for_one_to_three_berries`
- `a_fox_picks_glow_berries_from_vines`
- `a_friend_sees_fox_crumbs`

**`foxes-trust`**
- `two_fed_foxes_make_a_cub_that_trusts_both_feeders`
- `a_cub_does_not_flee_the_player_it_trusts`
- `a_bred_cub_has_no_prey_goals_until_loaded`
- `a_fox_attacks_whoever_hurts_its_trusted_player`
- `berries_make_a_fox_love_and_grow_a_cub`

### Owner's steps

Survival, Normal, `/time set 1000`, flat grass in a plains or taiga biome, `mob_griefing` on.

1. Use a fox egg on the grass: an orange fox with a white chest and black legs appears. About one fox in five holds something in its mouth (an emerald, egg, rabbit foot or hide, wheat, leather or feather). Then `/locate biome minecraft:snowy_taiga`, `/tp` there and use another egg: the fox is white. **m1_fox_red_and_snow**
2. Walk at a fox: it runs off from up to 16 blocks away (vanilla runs a little slower, 1.4 against 1.6, once you are within 7 blocks). Sneak toward it: it lets you come close.
3. Under a 7×7 oak plank roof 3 blocks above the grass, use a fox egg, then walk 13 blocks away. Within 4 s it curls up: eyes closed, legs tucked, soft snores. **m1_fox_sleeping** Sneak up to it: it sleeps on. Walk within 12 blocks without sneaking: it jumps up and runs. At night (`/time set 14000`) it does not sleep. From 20 blocks away you sometimes hear a screech. `/time set 1000`, `/weather thunder`: a sleeping fox wakes, and foxes in the open head for cover. `/weather clear`.
4. Build a 9×9 stone pen, 2 blocks high, with three chickens and a fox in it. Watch from outside while sneaking. The fox walks toward a chicken, crouches with its head tilted and rear wiggling for about 1 s, then leaps onto it and bites. **m1_fox_crouch**, **m1_fox_pounce**
5. Cover the pen floor with snow layers and repeat: a pounce that lands in snow leaves the fox head-down for 2 s, legs paddling, snow bits flying. **m1_fox_faceplant**
6. Drop (Q) an emerald 5 blocks from a fox: it fetches it and carries it in its mouth. **m1_fox_mouth_item** Drop sweet berries next to it: it spits the emerald out and takes a berry. About 28 s later it chews (crunching, crumbs) and the berry is gone. Kill a fox that holds something: the item drops. With `/gamerule mob_griefing false` it picks nothing up; turn the rule back on afterwards.
7. Plant sweet berry bushes 6 blocks from a fox and bone-meal them until they are red. The fox walks into a bush without slowing or flashing, sniffs, and picks it: the bush turns small, 1–3 berries pop out and one stays in its mouth. Glow berries hanging under a stone overhang, bone-mealed until they glow, get picked too. **m1_fox_berries**
8. Feed two adult foxes sweet berries: a cub appears. Walk straight at the cub: it does not run. `/summon minecraft:zombie ~4 ~ ~` and let it hit you: the cub yips and attacks it. **m1_fox_cub_defends**
9. Use a wolf egg near a fox: the fox flees and the wolf kills it. Put a lead on a fox: it follows you.
10. In creative, use a fox egg on a spawner: a small fox turns in the cage.
11. Leave a sleeping fox, a fox holding an emerald and the trusting cub. Save, quit and load: everything is as before.
12. `/locate biome minecraft:taiga` and `/tp` into new chunks: groups of 2–4 foxes, some of them cubs (white ones in snowy taiga). Foxes in old saves now move.
13. Friend window (`scripts\mp-local.ps1`): the friend sees the same sleeping, crouching, pounces, faceplants, mouth items and crumbs at the same moments. The friend walking up wakes a fox. The friend's M4A1 hit makes a fox flash and flee. **m1_fox_friend_host**, **m1_fox_friend_friend**
14. The friend feeds two foxes: their cub trusts the friend but runs from the host.

### Kit

```
IW4L_MINECRAFT_KIT=minecraft:fox_spawn_egg*16,minecraft:chicken_spawn_egg*16,minecraft:wolf_spawn_egg*2,minecraft:zombie_spawn_egg*4,minecraft:rabbit_spawn_egg*4,minecraft:polar_bear_spawn_egg*2,minecraft:sweet_berries*64,minecraft:glow_berries*32,minecraft:bone_meal*64,minecraft:emerald*8,minecraft:feather*8,minecraft:oak_planks*64,minecraft:grass_block*64,minecraft:stone*64,minecraft:snow*64,minecraft:snow_block*32,minecraft:lead*4,minecraft:spawner,minecraft:iron_sword,minecraft:bow,minecraft:arrow*64,minecraft:shield,minecraft:cooked_beef*32,minecraft:torch*64
```

24 stacks. Every id is in the 26.3 item models. The rabbit and polar bear eggs work only once those groups have landed.

### Depends on

- **D0 d0a** (not d0b). Reuse D0's `ClimbOnTopOfPowderSnowGoal` if `silverfish-live` ported it; otherwise port it beside `FloatGoal`.
- **M1-1** (`animal_goals.rs`, `MoveToBlockGoal`, `family_goal_step_with`) and **M1-3** (H1, H2, H4).
- **E1**: the event 18 hearts (the fox needs no `puff_for` row), and `horse-fall`'s fall helper (else the widening above).
- **W**: one `game_event` line each for `block_change` (berries) and `eat`, if W merged first. Otherwise W adds them.
- **M1 groups**:
  - `rabbits` (prey);
  - `turtles` (switch the baby-turtle slot later);
  - `polar_bears` (the fox flees them);
  - `bees` (reuse `spared_blocks`);
  - `goats` (reuse the fall factors).
- **M2 `fish`**: fish prey.
- **Protocol**: no change of its own. The hooks are not serialized.

### Left for later

- **Walking on powder snow**: a shared collision flag for the rabbit, fox, silverfish and endermite.
- **Spawn eggs and dispensers**: an egg used on a fox makes a trusting cub, but the fork has no egg-on-mob offspring at all. Dispensers cannot fill a fox's mouth.
- **Eating effects**: golden apple effects, the chorus teleport and stew bowls. The fork has no mob consume effects, so the eaten item simply vanishes.
- **Smaller pieces**: the item-pickup flight animation; breeding experience orbs (the fork draws but spawns none, as for wolves).

### Open decisions (vanilla-like choice first)

1. **Bush immunity**: one `spared_blocks` hook (it touches "never edit" `hazards.rs` once, and bees need it too). The alternative is a fox-only workaround.
2. **Event 45 crumbs**: M1-3's `ClientFx.item_crumbs` (H2) plus a `FamilyShow` counter. The alternative, an E1 `PuffKind`, does not work: `MobPuff` carries no item.
3. **Faceplant bursts**: `World::destroy_effect` into `level.destroyed` (vanilla level event 2001, which already reaches friends). The alternative is to leave them for later.
4. **No living item-drop hook**: `World::spawn_item` (position, stack, velocity, pickup delay) already spawns items from a family tick (`server_mobs.rs:182`); goats drop their horns the same way (M1-5).
5. **Smaller choices**: keep vanilla's bred-cub quirk; `minecraft:illusioner` for the egg test; the fall is shortened in the fox's `run` (D0's `family_goal_step` applies the fall damage itself, which is why the fox runs through `family_goal_step_with`).

---

## M1-5. Goats (segment m1d: groups `goats-live`, `goats-drawn`, `goats-ram`, `goats-leap`, `goat-horns`)

Family `goats` in group `land`, kind `minecraft:goat` (mob list row 17). Its hit-key byte and `Family::KEY` are **50**. It is a brain family (D0-2.4): the brain lives in the state type, with no `MonsterSpec`. Vanilla facts are from javap on the 26.3 `client.jar` and its data this round, unless marked "javap during the work". Goats run before armadillos and sniffers because they create the shared `animal_brain.rs`, and they create `long_jump.rs` for M2's frogs.

### What exists today (`C:\Dev\Soscraft\mashup-crafting`, `minecraft-survival`)

Paths under `entities/` are in `third_party/minecraftoss/entities/src/`.

- **Not simulated.** `minecraft:goat` is not in `SIMULATED_KINDS` (`crates/minecraft_terrain/src/server_mobs.rs:433`). It stays dormant and its egg does nothing (`docs/MINECRAFT-SURVIVAL.md:441`).
- **Spawning.**
  - `third_party/minecraftoss/world/src/natural_spawner.rs:285` has the size row (0.9 × 1.3, `OnGround`, `MotionBlockingNoLeaves`). The rules fall to `Rules::Other` (`:354`), so goats never spawn.
  - Category creature: `natural_spawner/spawning.rs:102`.
  - `summon` (`spawning.rs:886`) skips finalize when given NBT, like vanilla. The owner's steps rely on this.
- **Horns.** The eight instruments are in creative (`crates/minecraft_terrain/src/creative_variants.rs:287-299`). `set_instrument` (`third_party/minecraftoss/core/src/loot.rs:1134`) makes the same `nextInt(size)` pick as `Goat.createHorn`. Nothing plays a horn.
- **Closest brain: hoglins.**
  - `entities/hoglin_brain.rs`: `look_sometimes` :458, `idle_movement` :485, `start_love` :616, `tick_love` :645, `follow_adult` :672.
  - `entities/world/hoglins.rs`: `think` :123, `steer` :183, `travel` :219, `tick_body` :236.
  - `hoglin_living` (`entities/world/hoglins.rs:462`, `pub(super)`): players and mobs as `Living`, with boxes.
  - `breed_hoglins` (`entities/world/hoglins.rs:628`): age 6000, `1 + nextInt(7)` experience via `trade_experience`.
  - `entities/brain_behaviors.rs`: `Living` :17, `Walker` :165, `MoveSink` :183.
- **Animal helpers.**
  - `entities/tempt.rs:6`: `PlayerCandidate`, including `main_hand_cow_food`.
  - `entities/age.rs:6`: `Age`.
  - `entities/animal.rs:12`: `interact`.
  - `entities/cow.rs:148-181`: milking. Its sound is at `crates/minecraft_terrain/src/mob_actions.rs:198`.
- **Hits.**
  - `entities/world.rs:612`: `PlayerHitKind::Melee { throw }`.
  - `crates/minecraft_survival/src/impulse.rs:45`: `MeleePush::landing` applies the full throw even when shielded.
  - `crates/minecraft_survival/src/entities.rs:1875`: `melee_push`.
  - `entities/world.rs:3476`: `player_hurt` sets the `hurt_by` that the player's wolves avenge.
  - `entities/world.rs:4179`: `mob_hits_mob` always blames.
  - `entities/world/combat.rs:233`: `knock_back` (`pub(super)`).
- **Shared plumbing.**
  - `entities/world/living.rs:104`: `fall_damage`. Its `fall_multiplier` :55 and `fall_reduction` :65 are private.
  - Entity events: queued at `entities/world.rs:2623`, dropped at terrain `server.rs:1102` (E1 draws them).
  - Items with a velocity: `World::spawn_item(position, stack, velocity, pickup_delay)` (`third_party/minecraftoss/player/src/lib.rs:117`, served by `MobWorld` at `server_mobs.rs:182`) spawns them straight from a family tick.
  - Brain memories in NBT: `crates/minecraft_terrain/src/piglin_save.rs:106` `memory_tag`, :115 `brain_tag`.
  - Models: `crates/minecraft_terrain/src/hoglin_render.rs:153` `append_model`; `cow_render.rs:82` (26.3 baby model at scale 1); `client_mobs.rs:1395` `quadruped_legs`.
  - Item cooldowns: `crates/minecraft_survival/src/cooldowns.rs`. The spyglass arm is `actions.rs:1265`.

### Vanilla 26.3 (javap)

**Entity**
- `EntityTypes.GOAT`: `CREATURE`, `sized(0.9, 1.3)` (eye 1.105), `passengerAttachments(1.1125)`, `clientTrackingRange(10)`. It is allowed on peaceful and not fire immune.
- Kids: `BABY_DIMENSIONS = scalable(0.45, 0.65).withEyeHeight(0.59375)`, `getAgeScale` 0.55. Pose `LONG_JUMPING` scales the dimensions (and the eye) by 0.7.
- `createAttributes`: health 10, speed 0.2, attack damage 2. `ageBoundaryReached` sets the damage to 1 for a kid, 2 for an adult.
- Movement: navigation `setCanFloat(true)`; malus −1 for `POWDER_SNOW` and `ON_TOP_OF_POWDER_SNOW`.
- Head: `getMaxHeadYRot` 15, and `setYHeadRot` clamps the head to the body ±15.
- Fall: `calculateFallDamage` is `Animal`'s less 10.
- `Animal`:
  - ambient interval 120;
  - love lasts 600 ticks and broadcasts event 18;
  - after breeding both parents go to age 6000, and the breeder gets `1 + nextInt(7)` experience;
  - `isBrightEnoughToSpawn`: raw brightness > 8;
  - javap during the work: `actuallyHurt` (forgets love) and `getBaseExperienceReward` (1-3 for an adult).

**Saved and synced data**
- `IsScreamingGoat` defaults to false.
- `HasLeftHorn` and `HasRightHorn` default to true.
- `Brain.memories` holds `minecraft:ram_cooldown_ticks`, `minecraft:long_jump_cooling_down` and `minecraft:temptation_cooldown_ticks` (`Codec.INT`). `ram_target` and `long_jump_mid_jump` are not saved.
- Plus the usual `Age`, `ForcedAge`, `InLove`, `LoveCause` and `AgeLocked`.

**Events and sounds**
- Event 58 lowers the head and 59 raises it. `aiStep` moves `lowerHeadTick` by +1 while lowering and −2 otherwise, kept within 0-20.
- `getRammingXHeadRot = lowerHeadTick / 20 × (kid ? 52.5° : 30°)`.
- Sounds are `entity.goat[.screaming].{ambient,hurt,death,eat,milk,prepare_ram,ram_impact,long_jump}`, `entity.goat.horn_break`, and `entity.goat.step` at a fixed 0.15. The leap lands with the step at 2.0.
- Eating is NEUTRAL at pitch `randomBetween(0.8, 1.2)`. Milking is `Player.playSound`, i.e. the PLAYERS source.

**`finalizeSpawn`** draws from the level random in this order:
1. `initMemories`: `long_jump_cooling_down = UniformInt(600, 1200)` and `ram_cooldown_ticks = UniformInt(600, 6000)`. A `sample` is `nextInt(max − min + 1) + min`.
2. `nextDouble() < 0.02`: a screaming goat.
3. `ageBoundaryReached`.
4. Before `super`, so even a future kid rolls this: `nextFloat() < 0.1` loses a horn, the left on `nextBoolean()`, else the right.
5. `Animal`/`AgeableMob` group data with a 0.05 baby chance.

`getBreedOffspring` runs `initMemories`, then picks a parent with `nextBoolean`. The kid screams if that parent does, else with chance `nextDouble() < 0.02`.

**`mobInteract`**
- A bucket on an adult: the milk sound and `createFilledResult(MILK_BUCKET)`.
- Otherwise `Animal.mobInteract` with `#goat_food` (wheat), plus the eat sound when the item was used up.

**`dropHorn`**
- A kid, or a goat with no horn left, drops nothing.
- One horn left: that one. Both: the left on `nextBoolean`, else the right.
- The item: `createHorn()` seeds a legacy LCG with `uuid.hashCode()` (Java: `(int)(hilo >> 32) ^ (int) hilo`, where `hilo = most ^ least`). It then takes `nextInt(4)` over the tag order:
  - `#regular_goat_horns`: ponder, sing, seek, feel;
  - `#screaming_goat_horns`: admire, call, yearn, dream.
- It drops at the feet with velocity `randomBetween(−0.2, 0.2)`, `(0.3, 0.7)`, `(−0.2, 0.2)` from the goat's random.
- `data/minecraft/instrument/*.json`: `item.goat_horn.sound.N`, range 256, use duration 7 s.

**Brain (`GoatAi`)**
- Sensors: nearest living entities, nearest players, nearest items (unused), nearest adult, hurt by, food temptations (range from the `TEMPT_RANGE` attribute, 10).
- `updateActivity = setActiveActivityToFirstValid([RAM, LONG_JUMP, IDLE])`. It keeps the current activity when none is valid.
- **CORE**:
  - `Swim(0.8)`;
  - `AnimalPanic(2.0)`: `#panic_causes` from `HURT_BY`, or freezing or burning; 100-120 ticks; 5 across, 4 up. `mob_attack_no_aggro` is no panic cause;
  - `LookAtTargetSink(45, 90)` and `MoveToTargetSink`;
  - count-downs for the temptation, long jump and ram cooldowns.
- **IDLE** needs both the ram and long-jump cooldowns *present* (the 26.3 conditions):
  - priority 0: `SetEntityLookTargetSometimes(PLAYER, 6, 30-60)` and `AnimalMakeLove(GOAT)`;
  - priority 1: `FollowTemptation` at 1.25;
  - priority 2: `BabyFollowAdult(5-16, 1.25)`;
  - priority 3: one of stroll 1.0 (weight 2), `SetWalkTargetFromLookTarget(1.0, 3)` (weight 2) or `DoNothing(30, 60)` (weight 1).
- **LONG_JUMP** needs no tempting player, no breed target, no walk target and no cooldown. It runs `LongJumpMidJump` and `LongJumpToRandomPos(600-1200, 5, 5, 3.5714288)`.
- **RAM** needs no tempting player, no breed target and no ram cooldown. It runs `RamTarget` (speed 3.0, knockback 2.5, or 1.0 for a kid) and `PrepareRamNearestTarget(min 4, max 7, walk 1.25, prepare 20)`.
  - Time between rams: 600-6000 ticks; a screamer 100-300.
  - The cooldown after a failed preparation is the minimum: 600, or 100 for a screamer.
  - Targets: `forCombat()`, never goats, armour stands only with `mob_griefing`, inside the world border.

**Preparing a ram** (`PrepareRamNearestTarget`, gives up after 160 ticks)
1. Pick the closest visible target.
2. Find start spots: from the target's block, walk N, E, S, W up to 7 blocks while each block is walkable (`isStableDestination` and malus 0). Keep a spot at Manhattan distance ≥ 4 from the target.
3. Sort the spots by `distSqr` to the goat. Take the first one with `createPath(pos, 0).canReach()`.
4. Walk there at 1.25 while looking at the target.
   - If the target changes block: event 59 and choose again.
   - At the spot: event 58. After 20 ticks there, set `RAM_TARGET` to `getEdgeOfBlock` (the target block's bottom centre plus 0.5 × the sign of each axis toward it) and play `prepare_ram` at the voice pitch.
5. Stopping without a ram target: event 59 and the failed-preparation cooldown.

**The ram** (`RamTarget`, gives up after 200 ticks)
- `ramDirection = normalize(goatBlock − ram point)` in x and z; walk to the ram point at 3.0.
- **It touches a living target** (the first `getNearbyEntities` hit overlapping the goat's box):
  1. `hurtServer(mob_attack_no_aggro, damage)`. This damage type is the only member of `#no_anger`; it scales with difficulty and its death message is `mob`.
  2. `s = clamp(speed × 1.65, 0.2, 3.0) + 0.25 × (speed amplifier − slowness amplifier)`. During a ram the speed is 0.6, so `s` is 0.99.
  3. `s` is halved when `applyItemBlocking > 0`.
  4. `knockback(s × force, ramDirection.x, ramDirection.z)` (standard: `×(1 − kr)`, the old velocity halved, y `min(0.4, y/2 + s)` on the ground).
  5. Finish, then `ram_impact`.
- **It runs into a horn-snapping block** (`#snaps_goat_horn`: `#overworld_natural_logs`, stone, packed ice, and iron, coal, copper and emerald ore), at `position + normalize(horizontal velocity)` or the block above it: `ram_impact`, `dropHorn`, then `horn_break` if a horn fell. Then finish.
- **Otherwise** it finishes once the walk target is gone or it is within 0.25 of the ram point.
- Finishing: event 59, a new ram cooldown sample, and the ram target erased.

**Long jump**
- `LongJumpToRandomPos` (prepares for 40 ticks, gives up after 200):
  - Start: on the ground, not in water, lava or a honey block; otherwise the cooldown becomes sample/2.
  - Candidates are every block within ±5 x/z/y except its own, weighted by `ceil(distSqr)`; 20 tries.
  - A candidate is refused if it is in the same column, if the block below is not `isSolidRender`, or if its malus is not 0.
  - It also needs a jump vector, and the goat leaps **only if `createPath(pos, 0, 8)` cannot reach it**.
  - After 40 ticks: `yRot = yBodyRot`, friction off, velocity = vector × (len + jump boost)/len, the jump sound.
  - Stopping without a jump: cooldown sample/2.
- `LongJumpUtil.calculateJumpVectorForAngle`:
  - Angles 65, 70, 75 and 80 in shuffled order; maximum speed 0.42 × 3.5714288 = 1.5.
  - It aims half a block short of the target's centre, with g = 0.08.
  - `d11 = d1·g / (d2·sin 2a − 2·dy·cos²a)` must be ≥ 0, and `√d11` ≤ the maximum.
  - The arc is checked in `ceil(d2/h) × 2` steps with `isClearTransition` over the `LONG_JUMPING` box.
  - The result is scaled by 0.95.
- `LongJumpMidJump` (gives up after 100 ticks):
  - Start: friction off, pose `LONG_JUMPING`.
  - Landing: velocity × (0.1, 1, 0.1), the step sound at 2.0, `STANDING`, a new cooldown sample.

**Spawning and loot**
- Placement: `ON_GROUND` with `MOTION_BLOCKING_NO_LEAVES`. The block below must be in `#goats_spawnable_on` (grass block, stone, snow, snow block, packed ice, gravel) and the spot bright enough.
- Biomes: frozen peaks, jagged peaks and snowy slopes, in the creature list, weight 5, groups of 1-3.
- Loot table `entities/goat` has no pools. `#goat_food` = `#cow_food` = `[wheat]`.

**Model (`GoatModel` extends `QuadrupedModel`, 64×64; box = uv, from, size; pose = offset, rotation)**
- **head** (1, 14, 0):
  - ear (2, 61) (−6, −11, −10) (3, 2, 1);
  - mirrored ear (2, 61) (2, −11, −10) (3, 2, 1);
  - goatee (23, 52) (−0.5, −3, −14) (0, 7, 5). It is still mirrored: the builder keeps `mirror()`.
- **left_horn** (12, 55) (−0.01, −16, −10) (2, 7, 2) and **right_horn** (12, 55) (−2.99, −16, −10) (2, 7, 2); both are head children at offset 0.
- **nose**: head child at (0, −8, −8), xRot 0.9599, box (34, 46) (−3, −4, −8) (5, 7, 10).
- **body** (0, 24, 0), not rotated: (1, 1) (−4, −17, −7) (9, 11, 16) and (0, 28) (−5, −18, −8) (11, 14, 11).
- **Hind legs**: offsets (1, 14, 4) left and (−3, 14, 4) right, uv (36, 29) and (49, 29), box (0, 4, 0) (3, 6, 3).
- **Front legs**: offsets (1, 14, −6) left and (−3, 14, −6) right, uv (49, 2) and (35, 2), box (0, 0, 0) (3, 10, 3).

**`BabyGoatModel`** (64×64, no mesh transform, drawn at scale 1)
- **Legs**, box (−1, −0.5, −1) (2, 5, 2):
  - hind: left (29, 12) at (1.5, 19.5, 3), right (21, 12) at (−1.5, 19.5, 3);
  - front: right (21, 5) at (−1.5, 19.5, −2), left (29, 5) at (1.5, 19.5, −2).
- **body** (0, 17.8, 0): (0, 10) (−3, −2.3, −4.5) (6, 5, 9) and (0, 24) (−2.5, −2.2, −4) (5, 4, 8).
- **head** (0, 15.5, −3), xRot 0.4363: (0, 0) (−2, −3.8126, −5.1548) (4, 4, 6).
- **Horns**: both at (−1.5, −1.5, −1), xRot −0.3927, uv (24, 0), mirrored. Right horn (0, −4.5, 0) (1, 2, 1); left horn (2, −4.5, 0) (1, 2, 1).
- **Ears**: at (∓1.7, −2.3126, 0.1452), yRot ∓0.5236, uv (0, 12). Right ear (−2, −0.5, −0.5) mirrored; left ear (0, −0.5, −0.5); both (2, 1, 1).
- **HeadMain**: at (0, −1.3126, −1.1548), box (0, 0) (−2, −2.5, −4) (4, 4, 6). It repeats the head box exactly; port it as is.

**Posing and rendering**
- `setupAnim`:
  - the quadruped head look and leg swing;
  - each horn is visible only while the goat has it;
  - if `rammingXHeadRot` is not 0, it becomes the head's xRot;
  - the kid's head is fixed at xRot 0.3927 (22.5°) whenever it is not ramming.
- `GoatRenderer`: an `AgeableMobRenderer` with shadow 0.7, textures `minecraft:entity/goat/goat` and `minecraft:entity/goat/goat_baby`. It has no layers and no keyframe class.

### Plan

**Files (D0-3)**
- `third_party/minecraftoss/entities/src/world/families/land/goats.rs` holds `pub struct Goat`, `impl Family for Goat` and the tests. The state has:
  - age and love: `age: Age`, `in_love`, `love_cause`;
  - looks: `screaming`, `left_horn`, `right_horn`, `lowering_head`, `lower_head_tick`;
  - the leap: `long_jumping`, `discard_friction`;
  - `last_hurt: Option<(&'static str, i32)>`;
  - movement: `brain: GoatBrain`, `navigation`, `move_control`, `body_rotation`, `speed`, `forward`, `sideways`, `jumping`.
- `land/goats/brain.rs` has the sensors, the four activities and `update_activity`. `land/goats/ram.rs` has preparing and ramming.
- `crates/minecraft_terrain/src/families/land/goats.rs` (`TerrainFamily`) and `families/land/goats_render.rs`.
- The goat's row and `goat_finalize` go in `third_party/minecraftoss/world/src/natural_spawner/families/land.rs`.
- Registration, in the group's own files only:
  - `pub mod goats;` and `goats: goats::Goat,` in `family_group!(land 48..=63 { .. })`;
  - `mod goats;` and `Goat` in `terrain_group!(land { .. })`;
  - the `ROWS` row.

**Hooks (D0-2.2)**
- `KEY` 50 and `KINDS ["minecraft:goat"]`.
- `spawn`: `MobCore::new(kind, pos, 0.9, 1.3, 1.105, 10.0)` with `ambient_interval` 120.
- `category`: `"creature"`.
- `hurt`: the default, then forget love and store the damage type in `last_hurt`. `#panic_causes` needs that type, and `core.hurt_by` only names the attacker.
- `leashable`: true. `leash_walk`: walk to the holder with its `GroundNavigation`.
- `panicking`: the brain's `IS_PANICKING`.
- `voice`: `("goat.screaming" | "goat", 1.0, "neutral_volume")`.
- `step_sound`: `Some("entity.goat.step")`.
- `experience`, `despawn` and `loot` keep their defaults (1-3 for an adult creature; an empty table).

**Tick**
1. `family_base_tick`.
2. Without `no_ai`:
   1. Age and the love countdown.
   2. The brain's sensors, behaviours and `update_activity`.
   3. The head-lowering ramp.
   4. `steer` and `travel`, as hoglins do.
      - While `discard_friction` is on, undo the drag that `travel_air` applied: x and z divided by the drag, y by 0.98 after gravity. `Body` is not edited.
      - The `LONG_JUMPING` pose sets `core.body` and `eye_height` to 0.7 of the normal size until it lands.
   5. Fall damage on landing.
3. `family_after_move`.

**Breeding** follows `breed_hoglins`:
- the child comes from `spawn_family(Goat::spawn(..))` with age −24000, `initMemories` and the screaming rule;
- both parents go to age 6000;
- the experience goes to `trade_experience`;
- falling in love pushes entity event `(id, 18)` for E1's hearts.

**Inputs**
- Tempting reads `PlayerCandidate.main_hand_cow_food` and `offhand_cow_food`. The tags are equal in 26.3, and a gated test pins that, so there is no new field.
- Nearby players and mobs come from `hoglin_living(id, players)`. The ram's overlap test uses those boxes.

**Ram hits**
- **On a player**: a `PlayerHitKind::Typed` (D0-2.6) hit:
  - `damage_type: "minecraft:mob_attack_no_aggro"`, `message: "mob"`, sourced from the goat, so the death line names it;
  - `from`: the goat;
  - `push`: vanilla's two knockbacks combined, about `0.2 + s × force` horizontally along the ram, and up 0.4 when on the ground;
  - no bypasses.
- **On a mob**: `mob_hits_mob`, then `knock_back(victim, s × force, ramDirection.x, ramDirection.z)`.
- **A snapped horn**: clear the horn flag and drop the stack `{"minecraft:instrument": "minecraft:<name>"}` from the tick with `level.spawn_item(feet, stack, velocity, 10)` (vanilla `spawnAtLocation`'s pickup delay), the velocity drawn as above. No drop hook is needed.

**Two one-line damage-type rules** (in `goats-ram`):
- `world.rs:3476` `player_hurt`: skip `hurt_by` for `"minecraft:mob_attack_no_aggro"` (`#no_anger`). A rammed player's wolves then leave the goat alone.
- `impulse.rs:38` `MeleePush`: add `blocked: f64`, the share of `throw` a raised shield lets through (1.0 everywhere else). `landing` (`:48-49`) scales the throw by it when shielded. The `Typed` arm of `melee_push` (`entities.rs:1875`) sets 0.5 for that damage type. Every existing literal gets `blocked: 1.0` in the same commit (`entities.rs:1877`, the test at `:2392`, `impulse.rs:75`, `:85`, `pvp.rs:122`, and D0's `Typed` arm). M2's and M3's drafts build pushes only through `PlayerHitKind`, so they add no `MeleePush` literal; one that shows up at a merge gets `blocked: 1.0`.
- javap during the work: `LivingEntity.blockUsingItem`, which knocks a melee attacker back from a raised shield. Check that the fork does this for `Typed` too.

**Falls**: vanilla less 10. Pass `fall_damage` a distance shortened by `10 / (reduction × multiplier)`. This gives exactly 10 less damage with the matching sound, and none on a zero multiplier. It needs the landed block's numbers:
- E1's `horse-fall` helper (`docs/round10/track-e.md:628`), or the `pub(crate)` `fall_multiplier` and `fall_reduction` that M1-4's `foxes-live` opened if e1a had not merged then;
- `goats-leap` never edits `living.rs` itself.

**Multiplayer** (`TerrainFamily for Goat`)
- `TRACKING_RANGE` 10, `MAX_HEAD_Y_ROT` 15.0, `SHADOW_RADIUS` 0.7, `TEXTURES` = the two goat textures.
- `VOLATILE = ["HasLeftHorn", "HasRightHorn"]`. `Age`, `InLove` and `Brain` are already global volatile keys (`mob_tracking.rs:15`).
- `read` and `fields`: `IsScreamingGoat`, both horns (default true), `Age`, `ForcedAge`, `InLove`, `LoveCause`, `AgeLocked`, and `Brain.memories` with the three cooldowns (written as `piglin_save.rs:106`/`:115` do).
- `show` = `FamilyShow { flags: LEFT_HORN 1 | RIGHT_HORN 2, values: [lower_head_tick, ..] }`. Vanilla never interpolates `lowerHeadTick`, so the raw 0-20 value goes over; host and friend then draw the same angle. `show_on` sets both.
- The kid flag and the shrunken leap box travel in the generic `ServerMob` (`baby`, width and height).
- No keyframes. The hearts are E1's event 18.

**Interact** (terrain `TerrainFamily::interact`)
- A bucket on an adult:
  - gives a milk bucket. The goat's own file mirrors `cow.rs:148-181`; the cow is not touched;
  - plays `entity.goat[.screaming].milk` at `players_volume`.
- Otherwise `animal::interact` with `item_in_tag("minecraft:goat_food")`.
  - When the food is used up: the eat sound.
  - Falling in love: event 18.

**Draw**
- With `posed_cube`, like `hoglin_render.rs`: the two tables above, horns shown by flag, and the head xRot rules into `out.models`.
- Spawner cages come through the generic arm.

**Spawn row**
- Type info: `TypeInfo { 0.9, 1.3, OnGround, MotionBlockingNoLeaves, fire false, rules: Rules::SpawnableOn("minecraft:goats_spawnable_on"), Walk::Animal, remove_far false, cluster 4 }`.
- `MobCategory::Creature`, follow range 16, `baby_chance: Some(0.05)`.
- `rule: None`: the existing `SpawnableOn` arm (`spawning.rs:732` → `natural_spawner.rs:761`) is exactly `checkGoatSpawnRules`. Use `Rules::Family { creature: true }` plus a rule only if D0's rows require it.
- `finalize: Some(goat_finalize)` writes both cooldowns into `Brain.memories`, `IsScreamingGoat` and a lost horn, in vanilla order. It must work with no context (creature and chunk-generation spawns).

**Root-file commits** (each its own commit, named for the change, D0-12):
- **`world/families/animal_brain.rs`**, one `pub mod` line in root `families.rs`. The brain versions of `AnimalPanic`, `FollowTemptation` (with the tempting sensor and its cooldown), `AnimalMakeLove`, `BabyFollowAdult` (with the adult sensor) and the idle `RunOne`. It generalises the hoglin's code without changing it. M1's armadillos and sniffers and M2's axolotls, frogs and nautiluses reuse it.
  - The tempting sensor takes the tempting test as a function of the player (a `fn` or closure over `PlayerCandidate` and `PlayerControls`): goats pass the cow-food flags, armadillos and sniffers an item-tag test on `controls.holds` (`world/felines.rs:353-376`), so neither widens it later.
  - M2 names this module and its types exactly as here; whichever of M1 and M2 merges second drops its copy.
- **`world/families/long_jump.rs`**, one `pub mod` line. `LongJumpToRandomPos`, `LongJumpMidJump`, `LongJumpUtil` and travel without friction. M2's frogs reuse it with their own landing rule.
- No living item-drop hook: `World::spawn_item` already does it (M1-4, decision 4).

**Commit groups**

| # | Key | Commits | Row 17 after |
|---|---|---|---|
| 1 | `goats-live` | 1. "Minecraft animals share vanilla's brain behaviours for panic, temptation, love and following adults". 2. "Minecraft goats live, breed and wander like vanilla" (the family with CORE and IDLE, finalize, the spawn row, `TerrainFamily` save/load/show, with an empty `draw` but `TEXTURES` set). 3. "Minecraft goats give milk and eat wheat". | `live` |
| 2 | `goats-drawn` | "Minecraft goats are drawn like vanilla" | `drawn` |
| 3 | `goats-ram` | "Minecraft goats ram players and mobs and snap their horns on stone and logs" (with the two damage-type rules). | |
| 4 | `goats-leap` | 1. "Minecraft mobs can long-jump like vanilla goats and frogs". 2. "Minecraft goats leap up to five blocks and shrug off short falls". | `done` |
| 5 | `goat-horns` (optional) | "Minecraft goat horns play their eight calls" | |

`goat-horns`:
- New file: `crates/minecraft_survival/src/goat_horns.rs`, called from one arm next to the spyglass arm (`actions.rs:1265`).
- Instrument data: read the stack's `minecraft:instrument`, then the jar's `data/minecraft/instrument/<name>.json`.
- Sound: `sound_event` at volume 16 (range/16), `records_volume`, heard by everyone in range.
- Cooldown: `cooldowns.start("minecraft:goat_horn", 140)`.
- javap during the work: `InstrumentItem.use`.
- Vibrations: one `instrument_play` game-event line, once W has merged.

Every group updates row 17 of `docs/round10/mobs.md` and the track log in the same commit as the code.

### Tests

| Commit | Tests |
|---|---|
| `goats-live` 1 | `animal_panic_runs_from_panic_causes_for_100_to_120_ticks` (not `mob_attack_no_aggro`), `follow_temptation_walks_to_a_player_holding_food_and_cools_down`, `animal_make_love_breeds_after_60_to_110_ticks_within_two_blocks`, `baby_follow_adult_keeps_five_to_sixteen_blocks` |
| `goats-live` 2 | `a_goat_is_large_with_ten_health_and_its_kind` (both sizes and eyes, census, `mob_candidates`, table and 1-3 experience for adults only), `the_idle_activity_needs_both_cooldowns_and_a_tempted_goat_keeps_its_activity`, `two_fed_goats_make_a_kid_that_may_scream_like_a_parent`, `a_goat_turns_its_head_at_most_fifteen_degrees`, `a_hurt_goat_panics_at_double_speed`; world: `goats_spawn_on_stone_snow_and_ice_in_the_light_in_groups_of_one_to_three`, `a_goat_finalize_rolls_cooldowns_screaming_and_a_lost_horn_in_vanilla_order` (seeded, with and without context); terrain: `a_goat_saves_and_loads_with_its_horns_screaming_age_and_cooldowns`, `a_friend_sees_a_goat_from_its_look_tag` (flags, `lower_head_tick`, horn keys dropped only for goats), the gated `#goat_food == #cow_food` check |
| `goats-live` 3 | `milking_an_adult_goat_fills_a_bucket_and_a_kid_gives_nothing`, `wheat_puts_a_goat_in_love_and_feeds_a_kid` |
| `goats-drawn` | `an_adult_goat_has_its_parts_textures_and_two_horns`, `a_kid_uses_its_own_model_with_the_head_tilted_down`, `a_ramming_goat_lowers_its_head_by_up_to_thirty_degrees` (52.5° for a kid), `a_hornless_goat_hides_its_horns` |
| `goats-ram` | `a_goat_lines_up_four_to_seven_blocks_away_and_lowers_its_head_for_twenty_ticks` (start spots, events 58/59, fail cooldown 600/100), `a_ram_hits_a_player_for_two_and_throws_them_by_its_speed` (push about 0.2 + 2.475; a kid's force 1.0), `a_raised_shield_halves_the_ram_push`, `a_rammed_player_does_not_set_its_wolves_on_the_goat`, `a_ram_throws_a_cow`, `a_ram_into_stone_or_a_log_snaps_one_horn_then_the_other_and_never_from_a_kid`, `a_snapped_horn_is_the_goats_own_instrument`, `a_screaming_goat_rams_every_five_to_fifteen_seconds`, `a_snapped_horn_drops_at_the_feet_with_pickup_delay_ten` |
| `goats-leap` | `long_jump_vectors_match_vanilla_for_each_angle`, `a_long_jump_needs_a_clear_arc`, `a_goat_leaps_only_where_it_cannot_walk`, `a_leaping_goat_shrinks_to_seven_tenths_and_lands_with_its_step`, `a_goat_takes_ten_less_fall_damage` (12 blocks: none; 20 blocks: 7) |
| `goat-horns` | `a_goat_horn_plays_its_instrument_and_cools_down_for_seven_seconds` |

### Owner's steps

Setup: release build, survival, **Normal**, by day, flat open ground of at least 20 × 20 in the Overworld, with the kit below. Name each screenshot by its label, e.g. `goat01_wander.png`; for the two-window steps, `goat09-host.png` and `goat09-friend.png`.

1. Use a goat egg 6 blocks away. A white goat with two grey horns, ears and a beard appears. It wanders, stops and sometimes looks at you; its head turns only a little before the body follows. **goat01_wander**
2. Hold wheat within 10 blocks: goats walk after you. Feed wheat to two adults: they walk together, and within about 5 seconds a kid appears with experience orbs. The kid is small, holds its head tilted down, has tiny horns and follows an adult. **goat02_kid**
3. Feed the kid wheat. Each wheat is used up and speeds its growth; an unfed kid grows up after 20 minutes. Use a bucket on an adult: you get a milk bucket and hear the milking bleat. On a kid: nothing.
4. Punch a goat once: it runs off fast for about 5 seconds.
5. `/summon minecraft:goat ~ ~ ~6 {IsScreamingGoat:1b}` and stand still. The goat:
   - walks to a spot 4-7 blocks away, in a straight line with you;
   - lowers its head for a second, screams and charges;
   - takes a heart and throws you a long way back;
   - screams now and then, and rams again every 5-15 seconds.

   **goat03_ram**
6. Raise the shield toward it as it charges: you take no damage and are thrown only about half as far.
7. Build a stone wall 3 high and 5 wide, stand right in front of it and face the goat.
   - Stay still while it lowers its head; moving now makes it pick a new spot.
   - The moment it screams and runs, strafe two blocks aside. It cracks into the wall and a goat horn pops out. If it catches you, try again. **goat04_horn**
   - Again: the second horn comes off. After that, nothing.
   - A cobblestone wall breaks no horns; an oak log wall breaks them like stone.
8. Spawn a cow 5 blocks from the screaming goat and walk 20 blocks away: the goat rams the cow and throws it.
9. Put a goat in a 5 × 5 pen walled 3 high with the stone and cobblestone (or a 5 × 5 pit, 3 deep), with no way out, and wait up to a minute. It stands still for 2 seconds, then leaps onto the rim with a leap sound and a thump. It never leaps where it can walk. **goat05_leap**
10. `/summon minecraft:goat ~ ~12 ~`: it lands unhurt. `/summon minecraft:goat ~ ~20 ~`: it lands with the fall sound and loses several hearts.
11. Tie a goat to a lead: it follows you.
12. Place a spawner and use a goat egg on it: a small goat turns in the cage. **goat06_cage** Break the spawner.
13. Save and quit with a one-horned goat, the screaming goat and a kid alive. Load the world: the horns, the kid and the screams are all as before.
14. `/locate biome minecraft:snowy_slopes` and `/tp` there (or jagged or frozen peaks): goats stand in groups of 1-3 on snow and stone. **goat07_peaks**
15. (`goat-horns` only) Use a horn from step 7: a long call that a friend hears from far away. The slot shows a 7-second cooldown. A horn from the screaming goat plays a screaming call. **goat08_horn_call**

Two windows (`scripts\mp-local.ps1`; the friend loads the same kit from `.env`):

16. The host hatches two goats and a kid. The friend sees them in the same places, with the kid's tilted head and the horns. **goat09-host / goat09-friend**
17. The host summons a screaming goat near the friend. Both windows show it lining up, lowering its head and throwing the friend. When it snaps a horn on the wall, the horn is gone in both windows. **goat10-host / goat10-friend**
18. The friend milks a goat and breeds two with wheat. Both windows show the kid.

### Kit

```
IW4L_MINECRAFT_KIT=minecraft:goat_spawn_egg*16,minecraft:wheat*64,minecraft:bucket*4,minecraft:stone*64,minecraft:oak_log*32,minecraft:cobblestone*64,minecraft:packed_ice*16,minecraft:snow_block*32,minecraft:shield,minecraft:iron_sword,minecraft:bow,minecraft:arrow*32,minecraft:cow_spawn_egg*4,minecraft:lead*4,minecraft:golden_dandelion*2,minecraft:spawner,minecraft:cooked_beef*32
```

17 stacks; every id checked against `assets/minecraft/items` in the 26.3 jar.

### Depends on

- **D0 segment d0a**: the registry, hooks, spawn rows, client registry and drawing. Goats need nothing from d0b; they don't fly.
- **E1**: the love hearts (event 18). Breeding works without them.
- **E2**: `hit_boxes`, so snowballs and eggs hit goats.
- **E1 `horse-fall`** or the fall factors M1-4 opened (see Falls).
- **W**: only the horn's `instrument_play` vibration.
- **Shared modules**: `long_jump.rs` (M2's frogs) and `animal_brain.rs` (M2's axolotls and frogs, M1's armadillos and sniffers) reuse what this section creates. Whichever of M1 and M2 merges second rebases onto the other's module.

### Left for later

- **A rammed mob still blames the goat.** `mob_hits_mob` (`world.rs:4179`) takes no damage type, so a rammed iron golem or wolf may fight back. Vanilla's `#no_anger` prevents that. Fixing it means touching every old family.
- **Goats in boats**: the fork's boats carry no mobs.
- **The held horn's tooting pose** on the MW2 body.
- **The nearest-items sensor**: goats never use it.
- **Rams on armour stands**: the fork has no armour stands.

---

## M1-6. Armadillos (segment m1e: groups `armadillos-live`, `armadillos-drawn`, `armadillos-scutes`, `armadillos-spiders`)

Family `armadillos` (`Armadillo`) in group `land`, `Family::KEY` **54**, row 21 of `docs/round10/mobs.md`, one kind. Polar bears, which share this section's research, are M1-2. Lines are `minecraft-survival` at `d1a3d028`; re-check before editing. D0 names follow `track-d0.md`; where D0's merged code differs, follow the code. Brushing an armadillo is vanilla `Armadillo.mobInteract`, so it goes through `TerrainFamily::interact`, not a new item-use path.

### What exists today

- **Dormant.** `minecraft:armadillo` is not in `SIMULATED_KINDS` (`crates/minecraft_terrain/src/server_mobs.rs:433`), so `spawn_saved` returns `None` (`:1321`). `tracking_range` (`mob_tracking.rs:285`) gives 5.
- **Armadillo spawning already matches 26.3, so it needs no world row** (D0-4):
  - size row `third_party/minecraftoss/world/src/natural_spawner.rs:291`;
  - rule `:333` `SpawnableOn("minecraft:armadillo_spawnable_on")`, checked in `check_spawn_rules` `:761` (raw light above 8);
  - finalize `:1019` `ageable(.., 0.05)`; creature category `natural_spawner/spawning.rs:103`.
- **Spiders:** `spider_goals` (`entities/src/monster_ai.rs:1083`) keeps a `NeverGoal` placeholder for `AvoidEntityGoal(Armadillo)` at `:1086-1087`.
- **Scutes:** the item exists, and the anvil repairs wolf armour with it (`crates/minecraft_terrain/src/stations.rs:434`). Wolves cannot wear armour.
- **Reuse:**
  - Brain: `hoglin_brain.rs` (sensors phased from the mob's random, `MoveSink`, `Ticker`, love, adults), `world/hoglins.rs:462` `hoglin_living` (players plus `mob_candidates` as `Living`).
  - Food: `animal.rs:12` `animal::interact` (love, growth, golden dandelion), `mob_actions.rs:194` (food), `:298` (tool wear and `broken`), held food through `controls.holds` and `recipes.item_in_tag` (`world/felines.rs:353-376`).
  - Saving: brain memories with a TTL as `piglin_save.rs` writes them.
  - Loot:
    - `loot.rs:182` loads `loot_table/gameplay/` (with `armadillo_shed`), but not `brush/`;
    - `roll_hero_gift` `:276` and the private `roll_of_type` `:281`;
    - the cat-gift pipeline: `world/felines.rs:383` `take_cat_gifts` → `server.rs:1107` `drop_cat_gifts` (`:1355`).
  - Render: `cow_render.rs:433` `cube_tinted` with `uv_dimensions` (inflated boxes, tested `:728`; `posed_cube::Cube` has no grow), `keyframes.rs` `AnimationState` (`start_if_stopped`, `fast_forward`) and `Definition::apply_walk`.
- **Gaps:**
  - `PlayerControls` (`entities/src/controls.rs:2`) has no sprint flag. The host knows it from `pm_flags::SPRINTING` (`render_anim/src/minecraft_players.rs:503`).
  - `MobCandidate` (`monster_ai.rs:341`) carries no state an avoid goal can test.

### Vanilla 26.3 (javap this round)

**Armadillo** (`animal/armadillo/Armadillo`, an `Animal` on a brain)
- **Type and attributes:**
  - `CREATURE`, `sized(0.7, 0.65)`, `eyeHeight(0.26)`, `clientTrackingRange(10)`;
  - baby: the type's size `scale(0.6)`, eye 0.21875, `getAgeScale` 0.6;
  - health 12, speed 0.14, animal base (tempt range 10), `setCanFloat(true)`, experience 1-3 for adults.
- **States**:

  | State | id | Threatened | Ticks | Hides in its shell |
  |---|---|---|---|---|
  | `idle` | 0 | no | 0 | never |
  | `rolling` | 1 | yes | 10 | when in-state ticks > 5 |
  | `scared` | 2 | yes | 50 | always |
  | `unrolling` | 3 | yes | 30 | when in-state ticks < 26 |

  - Saved as the string `state`. `inStateTicks` resets on each change and counts up in `tick`.
  - `isScared` = not `idle`. While scared, the head is clamped to the body, `getMaxHeadYRot` is 0 (else 32), and the body control does not turn it.
- **Brain** (`ArmadilloAi`):
  - **Sensors**: living entities, hurt by, food temptation (`#armadillo_food` = spider eye), nearest adult, and `MobSensor(5, isScaredBy, canStayRolledUp, DANGER_DETECTED_RECENTLY, 80)`:
    - every 5 ticks: if it cannot stay rolled up, the memory is erased;
    - else any scary living entity sets the memory for 80 ticks.
  - **CORE**:
    - `Swim(0.8)`;
    - `ArmadilloPanic(2.0)`: `AnimalPanic` on `#panic_environmental_causes`, with `rollOut` first;
    - `LookAtTargetSink(45, 90)`; `MoveToTargetSink` (not while scared);
    - count-downs of `temptation_cooldown_ticks` and `gaze_cooldown_ticks`;
    - roll-out (no danger memory, scared → `rollOut`).
  - **IDLE**:
    - 0 `SetEntityLookTargetSometimes(player, 6, 30-60)`;
    - 1 `AnimalMakeLove(1.0)`;
    - 2 `RunOne[FollowTemptation(1.25; close 1 for a baby, else 2), BabyFollowAdult(5-16, 1.25)]`;
    - 3 `RandomLookAround(150-250, 30, 0, 0)`;
    - 4 `RunOne{no walk target}[stroll(1.0), SetWalkTargetFromLookTarget(1.0, 3), DoNothing(30, 60)]`.
  - **PANIC**: `ArmadilloBallUp`, which needs `danger_detected_recently` and no `is_panicking`. `updateActivity` takes the first valid of PANIC, IDLE.
- **`ArmadilloBallUp`** (at most 6000 ticks):
  - start: on the ground and not in a liquid; `start` = `rollUp`;
  - keeps running while the state is threatened; `stop` → `rollOut` if it cannot stay rolled up;
  - **tick**:
    1. Count `nextPeekTimer` down.
    2. Rolling for more than 10 ticks: switch to `scared` (plus `.land` on the ground) and end the tick.
    3. Otherwise `danger` = the memory's TTL > 75. When it differs from last tick, `nextPeekTimer = 50 + nextIntBetweenInclusive(100, 400)`.
    4. Scared, timer 0, on the ground and in `danger`: entity event **64** and a new timer.
    5. TTL < 30: `.unroll_start`, then `unrolling`. Unrolling with TTL > 30: back to `scared`.
- **`isScaredBy(e)`**: `e`'s box meets its own box inflated by (7, 2, 7), and one of these holds:
  - `e` is `#undead`;
  - `e` is its `lastHurtByMob`;
  - `e` is a non-spectator player who is **sprinting or riding**.
- **`canStayRolledUp`**: not panicking, not in a liquid, not leashed, not a passenger and carrying none.
- **`rollUp`** (when not scared): `stopInPlace`, `resetLove`, `entity_action`, `makeSound(.roll)`, then `rolling`.
- **`rollOut`** (when scared): `entity_action`, `makeSound(.unroll_finish)`, then `idle`.
- **Hurt**:
  - while scared, the damage is `(amount - 1) / 2`. javap during the work: `LivingEntity.hurtServer` at 0 or less.
  - `actuallyHurt` (nothing more with no AI or when dying): a living attacker sets the danger memory for 80 ticks and calls `rollUp` if it can stay rolled up. Else `#panic_environmental_causes` calls `rollOut`.
- **`mobInteract`**:
  - **brush** → `brushOffScute`:
    - a baby gives nothing;
    - otherwise it rolls `brush/armadillo` (`entity_interact`, one `armadillo_scute`) at its position (`spawnAtLocation`), plays `playSound(.brush)` (1, 1) and posts `entity_interact`;
    - then `hurtAndBreak(16)` and `SUCCESS`;
    - it also works while rolled. The brush has 64 durability, so the fourth use breaks it.
  - else, scared → `FAIL` (the click is taken, nothing happens);
  - else `Animal.mobInteract` with `#armadillo_food`, with `canFallInLove` needing it unscared and eat sound `.eat`.
- **Shedding** (`customServerAiStep`, after the brain):
  - `scute_time` = `6000 + nextInt(6000)`, drawn from its own random and saved;
  - while alive it counts down; at 0, `shouldDropLoot` (not a baby, `mob_drops` on) drops the gift table `gameplay/armadillo_shed` (one scute);
  - when something dropped: `.scute_drop` (volume 1, pitch `(r1 - r2) * 0.2 + 1`) and `entity_place`;
  - the timer is re-rolled either way.
- **Sounds** (`NEUTRAL`):
  - `entity.armadillo.ambient` (none while scared), `.hurt` (`.hurt_reduced` while scared), `.death`, `.step` at 0.15;
  - `.eat`, `.roll`, `.land`, `.unroll_start`, `.unroll_finish`, `.brush`, `.scute_drop`;
  - `.peek`, played on the client by event 64.
- **Client animation**:
  - `rolling` starts roll-up if stopped; `unrolling` starts roll-out if stopped; `idle` stops all three;
  - `scared` stops both rolls. On its first tick it starts the peek and `fastForward(50, 1)`, so the ball holds the end pose. Event 64 restarts the peek from 0.
- **Spawning**:
  - `ON_GROUND`, `MOTION_BLOCKING_NO_LEAVES`, `checkArmadilloSpawnRules` (below in `#armadillo_spawnable_on`, light above 8);
  - savanna biomes (weight 10, 2-3) and the three badlands (weight 6, 1-2); no `finalizeSpawn` override.
- **Loot**: `entities/armadillo` is empty.
- **Spiders** (`Spider.registerGoals`, inherited by cave spiders): 2 `AvoidEntityGoal(Armadillo, 6.0, 1.0, 1.2, a -> !a.isScared())`.
- **Models** (`AdultArmadilloModel`, `BabyArmadilloModel`, 64 x 64; take the exact boxes from `createBodyLayer` during the work):
  - Tree:
    - `body` → [`tail`, `head` → [`head_cube`, `right_ear` → `right_ear_cube`, `left_ear` → `left_ear_cube`]];
    - four legs, and a separate `cube` (the ball: 10³ for the adult, 6³ inflated by 0.3 for the baby).
  - Notes for the port:
    - the adult body is two 8 x 8 x 12 boxes, one inflated by 0.3 (the shell);
    - the ears are zero-depth planes;
    - the baby's tail box is the part Mojang names `right_ear_cube`, and its ears sit directly under `head_cube`.
  - **`setupAnim`**:
    - **hiding**: the body's own boxes are skipped (`skipDraw`; the head still draws), the hind legs and tail are hidden, and the cube is shown;
    - **otherwise**: the cube is hidden, head xRot = `clamp(pitch, -22.5, 25)°`, head yRot = `clamp(yaw, -32.5, 32.5)°`, and the walk animation `applyWalk(pos, speed, 16.5, 2.5)` runs;
    - **then**: roll-out, roll-up and peek by their states.
- **Keyframes**:
  - `ArmadilloAnimation.ARMADILLO_ROLL_UP` 0.5 s, `ARMADILLO_WALK` 1.4583 s (looping), `ARMADILLO_PEEK` 2.5 s, `ARMADILLO_ROLL_OUT` 1.5 s;
  - `BabyArmadilloAnimation.ARMADILLO_BABY_*` with the same lengths.
- **Renderer**: `AgeableMobRenderer(adult, baby, shadow 0.4)` with textures `entity/armadillo/armadillo` and `armadillo_baby`. The shadow is multiplied by `ageScale` (`MobRenderer.getShadowRadius`).

- `EntityModel`'s default render type is `entityCutout`, so the armadillo draws into `out.culled` (D0-5).

### Plan

**Files** (D0-3):
- Entities, in `third_party/minecraftoss/entities/src/world/families/land/`:
  - `armadillos.rs`: state, in-state ticks, peek tick, `scute_time`, the cached `can_stay_rolled_up`, age and love, `impl Family`;
  - `armadillos/brain.rs`: memories, sensors, CORE, IDLE, PANIC and `ArmadilloBallUp`, on the goats' `animal_brain.rs`;
- Terrain, in `crates/minecraft_terrain/src/families/land/`: `armadillos.rs` with `armadillos/render.rs`.
- World: no row; the armadillo keeps its built-in row.
- Registration:
  - `pub mod` lines, and `armadillos: armadillos::Armadillo,` in `family_group!(land 48..=63 { .. })`;
  - terrain `mod` lines, and `Armadillo` in `terrain_group!(land { .. })`.

**Hooks used** (D0-2.2):
- `category` `"creature"` (default despawn and experience); `core.ambient_interval` 120;
- `leashable` true with a `leash_walk` along its own navigation;
- `panicking` (the `is_panicking` memory);
- `step_sound` `entity.armadillo.step`; default `voice` (`neutral_volume`);
- `hurt`: the scared reduction, `Voice::Event(".hurt_reduced")` in place of `Voice::Hurt`, and the `actuallyHurt` rules, with the roll-up gated on the cached `can_stay_rolled_up` (`hurt` has no world);
- the tick drops `Voice::Ambient` while scared (its timer was already reset, as vanilla's null sound does).

**Hooks into shared files** (each its own commit, named for the hook, D0-12):
1. **Sprinting** ("Minecraft mobs can tell when a player sprints", `armadillos-live`):
   - `PlayerControls.sprinting` (`controls.rs:2`, `Default` derived);
   - survival `PlayerView.sprinting` (`entities.rs:129`), copied in `player_controls` (`:151`);
   - set from `pm_flags::SPRINTING` next to the `sneaking:` lines of the two non-test literals, `render_anim/src/minecraft_players.rs:1274` (`sneaking:` at `:1294`, every player, friends included) and `minecraft_world.rs:1123` (`sneaking:` at `:1134`, host);
   - `false` in the test literals: survival `entities.rs:2188`, `:2321`, `:2411`, `:2923`, `:2971`, `actions.rs:2187`, `level.rs:919`, `remote.rs:1700` and `the_end/dragon_fight.rs:1142` (`grep -rn "PlayerView {" crates` finds them all; `views.rs` and `minecraft_net`'s `PlayerView` are other types).
   - Riding is `vehicle_of(Rider::Player(id))`. No wire change.
   - M3's camels need the same field for a rider's sprint. M1 owns the field, these lines and this subject; M3 adds only its ride-input line. If M3's copy lands first, use it and drop this commit.
2. **Hiding** ("Mobs that avoid others ignore registered mobs that are hiding", `armadillos-spiders`):
   - `MobCandidate.hiding` (`monster_ai.rs:341`), set `false` in the `mob_candidates` closure (`world.rs:4084`), `golems.rs:302`, `illagers.rs:317`, `:327` and `world.rs:6166`. No other track's brief adds a `MobCandidate` literal; one that shows up at a merge gets `hiding: false`;
   - D0's family arm sets it from a new trait hook `fn hiding(mob) -> bool { false }` (in `MobTraits`);
   - `AvoidEntityGoal`'s kind test (`monster_ai.rs:1768`, where M1-1's `matching` already sits) skips hiding mobs. Only spiders avoid armadillos, and only uncurled ones, so this equals the vanilla predicate.
3. **Shadow size** ("Registered Minecraft mobs cast their own shadow size", `armadillos-drawn`):
   - `TerrainFamily::shadow_radius(mob) -> f32`, defaulting to D0's rule (`SHADOW_RADIUS`, halved for babies), called by the root's walk that feeds `shadow_casters` (`client_mobs.rs:1343`), so `client_mobs.rs` itself is not edited;
   - the armadillo answers `0.4 × ageScale` (0.24 for a baby, `MobRenderer.getShadowRadius`). M1-8's baby turtle (0.174) reuses it, and M2's fish want the same hook: M1 owns this name and subject, and the second track to merge drops its copy.

The scutes need no hook of their own: shedding queues M1-3's H3 gift (`family_gift("minecraft:gameplay/armadillo_shed", "minecraft:armadillo", feet)`), and the brush rolls H3's `roll_table` once `loot.rs:182` gains `|| name.starts_with("data/minecraft/loot_table/brush/")` next to the `gameplay/` line.

**Multiplayer** (`TerrainFamily`):
- `TRACKING_RANGE` 10, `SHADOW_RADIUS` 0.4, `MAX_HEAD_Y_ROT` 32, `VOLATILE = ["state", "scute_time"]`;
- `read`/`fields`: `state`, `scute_time`, the age and love keys, and `Brain.memories` (`danger_detected_recently`, `is_panicking` with TTL; `temptation_cooldown_ticks`, `gaze_cooldown_ticks`);
- `show` = `values: [state id, in_state_ticks, peek_ticks (-1 none)]`. `peek_ticks` is `in_state_ticks + 50` until an event 64, then the ticks since it;
- the peek sound is a host `Voice::Event("entity.armadillo.peek", 1.0, 1.0)` sent with the event, so friends hear it.
- No `PROTOCOL` change.

**Keyframes**: `keyframes::shared().definition("ArmadilloAnimation", "ARMADILLO_ROLL_UP")` and the rest (`BabyArmadilloAnimation` for babies), with `AnimationState` (`fast_forward(50, 1.0)` for the peek) and `apply_walk(pos, speed, 16.5, 2.5)`.

**Interact** (terrain):
- **Brush**:
  - `entity_loot.roll_table("minecraft:brush/armadillo", "minecraft:entity_interact", kind)` into `outcome.drops` at the mob;
  - `wear_tool(selected, 16)` unless `infinite`, `"minecraft:brush"` into `broken`;
  - still handled when `entity_loot` is `None` (the `would_interact` probe).
- **Otherwise**: scared → handled, nothing changes; else `animal::interact` with `#armadillo_food`, the eat sound and event 18.

**Commit groups**

| # | Key | Commits | Row 21 |
|---|---|---|---|
| 1 | `armadillos-live` | 1. "Minecraft mobs can tell when a player sprints" (hook 1). 2. "Minecraft armadillos live, curl up when scared and peek like vanilla" (family, brain on `animal_brain.rs`, hurt rules, food and breeding, save/load/show, `draw` empty with `TEXTURES` set). | `live` |
| 2 | `armadillos-drawn` | 1. "Registered Minecraft mobs cast their own shadow size" (hook 3). 2. "Minecraft armadillos are drawn like vanilla" (both models, hiding, the state animations and the walk, culled pass, 0.3 shells through `cube_tinted`'s `uv_dimensions`). | `drawn` |
| 3 | `armadillos-scutes` | "Minecraft armadillos shed scutes and give them up to a brush" (H3 and the brush tables line). | |
| 4 | `armadillos-spiders` | 1. "Mobs that avoid others ignore registered mobs that are hiding" (hook 2). 2. "Minecraft spiders keep away from armadillos that are not curled up" (`:1087` becomes `AvoidEntityGoal::new("minecraft:armadillo", 6.0, 1.0, 1.2)`). | `done` |

Each commit updates row 21 of `mobs.md` and the track log.

### Tests

- **`armadillos-live`**:
  - `entities::tests::a_sprinting_player_sprints_in_the_mob_controls`;
  - `an_armadillo_is_small_with_twelve_health_and_its_kind`;
  - `a_sprinting_or_riding_player_within_seven_curls_it_up_and_a_walking_one_does_not`;
  - `an_undead_mob_nearby_scares_it`;
  - `it_curls_scared_after_ten_ticks_peeks_while_danger_stays_and_unrolls_as_it_fades`;
  - `a_curled_armadillo_takes_half_of_one_less_and_sounds_muffled`;
  - `a_hit_curls_it_up_and_fire_uncurls_it_and_makes_it_panic`;
  - `it_never_curls_while_leashed_in_water_or_riding`;
  - `spider_eyes_tempt_breed_and_grow_armadillos`;
  - terrain: `an_armadillo_saves_and_loads_with_its_state_scute_time_and_danger`, `a_friend_sees_an_armadillo_curl_and_peek_from_its_look_tag`, `a_curled_armadillo_refuses_food`.
- **`armadillos-drawn`**:
  - `the_adult_and_baby_models_have_their_boxes_and_textures`;
  - `a_hiding_armadillo_draws_its_ball_head_and_front_legs_only`;
  - `the_head_turns_within_32_5_degrees_and_tilts_from_minus_22_5_to_25`;
  - `the_armadillo_animations_read_from_the_jar_last_10_30_50_ticks` (gated on `MINECRAFTOSS_ROOT`);
  - hook 3: `families::tests::a_family_can_cast_its_own_shadow_size` (the default is D0's halving; a baby armadillo's is 0.24).
- **`armadillos-scutes`**:
  - `brush_and_gift_tables_roll_one_scute` (gated on `MINECRAFTOSS_ROOT`);
  - `an_adult_sheds_a_scute_every_five_to_ten_minutes_unless_mob_drops_is_off`;
  - `brushing_an_adult_drops_a_scute_and_wears_the_brush_by_sixteen` (the fourth use breaks it, creative no wear, a cub nothing, a curled one works, `would_interact`).
- **`armadillos-spiders`**: `a_hiding_registered_mob_is_marked_for_avoiding_goals`, `a_spider_keeps_six_blocks_from_an_uncurled_armadillo_and_ignores_a_curled_one`.

### Owner's steps

Setup:
- Release build, survival, **Normal**, by day, flat grass. Build a 12 x 12 oak-fence pen.
- The kit below goes in `.env`.
- The sprint is MW2's sprint.
- Step 14 needs two windows (`scripts\mp-local.ps1`).

Armadillos:
1. Use four armadillo eggs in the pen. Small brown armadillos with banded shells, pink ears and a short tail walk slowly. Their heads turn only a little towards you. **m1_armadillo**
2. Walk around the pen 3 blocks away: nothing happens. Sprint along the fence: each armadillo within about 7 blocks curls into a ball with a click and a thud. Keep sprinting back and forth: now and then one peeks out with a squeak. **m1_armadillo_curled**
3. Stand still 10 blocks away: about 2.5 s after your last sprint they start to uncurl, and about 4 s after it they walk again.
4. Hit one with the iron sword: it curls up at once, so every later hit lands on the ball, with a muffled hurt sound, for (6 - 1) / 2 = 2.5. Sprint so a fresh one curls first, then hit it: it dies on the fifth hit. A leashed one never curls (step 5): it dies on the second. Shoot others with the bow and the M4A1: each one curls up.
5. Tie a lead to one and sprint around it: it never curls. `/time set night`, use a zombie egg 4 blocks outside the fence and back off 15 blocks: the armadillos near the zombie curl up and stay curled. Kill the zombie: about 4 s later they uncurl. `/time set day`.
6. Hold a spider eye: they come to you. Feed two adults one each: a baby appears, with experience orbs (hearts once E1 has landed). Feed a curled one: nothing happens and the eye stays. **m1_armadillo_baby**
7. Use the brush on an adult: a scute pops out with a brushing sound. Brush it three more times: a scute each time, and the brush breaks on the fourth. Brush the baby: nothing. Brush a curled one: a scute. **m1_armadillo_brush**
8. Brush the other adults with the second brush until you hold six scutes, then craft them in the wolf armour shape (top-left; the whole middle row; both bottom corners): wolf armour.
9. Stay by the pen for 10 minutes: each adult drops one scute on its own with a pop, somewhere between 5 and 10 minutes. Then `/gamerule mob_drops false` and wait 10 more minutes: none drop. `/gamerule mob_drops true`.
10. `/gamemode creative`, `/time set night`. Use a spider egg 2 blocks from an uncurled armadillo: the spider backs off and keeps about 6 blocks away. Sprint past the armadillo so it curls: the spider now walks near it. **m1_armadillo_spider** Kill the spider. `/time set day`.
11. Still in creative, place a spawner and use an armadillo egg on it: a small armadillo turns in the cage. `/gamemode survival`.
12. Sprint past an armadillo, save and quit at once, then load: it is still a ball and uncurls by itself a few seconds later.
13. `/locate biome minecraft:savanna`, `/tp` there: armadillos come in groups of 2 or 3. In badlands they come in ones and twos.

Friend:
14. The host hatches two armadillos. The friend sprints past them: both windows show them curl, peek and uncurl at the same moments. The friend brushes one and picks up the scute. **m1_armadillo-host**, **m1_armadillo-friend**

```
IW4L_MINECRAFT_KIT=minecraft:armadillo_spawn_egg*16,minecraft:brush,minecraft:brush,minecraft:spider_eye*32,minecraft:spider_spawn_egg*4,minecraft:zombie_spawn_egg*4,minecraft:crafting_table,minecraft:iron_sword,minecraft:bow,minecraft:arrow*64,minecraft:cooked_beef*32,minecraft:spawner,minecraft:lead*4,minecraft:golden_dandelion*4,minecraft:oak_fence*64,minecraft:grass_block*64,minecraft:torch*64
```

17 stacks (a brush does not stack, so each brush is its own slot); all ids checked against the 26.3 item models. The friend needs the same kit.

### Depends on

- **D0, segment d0a**: the registry, the leash and `mob_candidates` arms, `FamilyShow` and `keyframes::shared()`. Not d0b.
- **M1-5 goats**: `world/families/animal_brain.rs`, whose tempting sensor already takes a held-food test.
- **M1-3 pandas**: H3 (gifts from loot tables).
- **Later users of this section's hooks**: M1-7 sniffers and M1-8 turtles (the shadow hook), M3's camels (`PlayerControls.sprinting`).
- **E1**: event 18 hearts. **E2** (`ti1`): thrown items hitting armadillos.
- **W**: game events `entity_action` (curl and uncurl), `entity_place` (shed) and `entity_interact` (brush), one line each once W has merged.

### Left for later

- **Wolf armour on wolves**: wolves have no body slot, no cracking and no repair. Crafting the armour already works.
- **Brushing blocks** (`BrushItem.use`).
- **Spawn eggs used on adults to make babies**: not in the fork for any mob.

### Open decisions (recommended choice first)

1. Sprint scares: add `PlayerControls.sprinting` (hook 1, vanilla). The alternative is riding, undead and hits only.
2. The spider predicate: `MobCandidate.hiding` (hook 2, vanilla). The alternative is spiders always avoiding armadillos.
3. Scutes: from the loot tables (M1-3's H3 and the brush tables, like cat gifts). The alternative hard-codes one scute.
4. Baby shadow: hook 3 gives vanilla's 0.24 (`0.4 × ageScale 0.6`). The alternative keeps D0's halving (0.2).

---

## M1-7. Sniffers (segment m1f: groups `sniffers-live`, `sniffers-drawn`, `sniffers-dig`, `sniffer-eggs`)

Family `sniffers` in group `land`, kind `minecraft:sniffer` (mob list row 46). Its hit-key byte and `Family::KEY` are **56**. It is a brain family (D0-2.4): the brain lives in the state type, with no `MonsterSpec`. Sniffer eggs are a block with a scheduled tick in the vendored world crate (D0-3, "Behaviour that still needs a shared file"). Vanilla facts are from javap on the 26.3 `client.jar` and its data this round, unless marked "javap during the work". Sniffers build on M1-5's `animal_brain.rs`, M1-3's gift hook (H3) and M1-6's shadow hook.

### What exists today (`C:\Dev\Soscraft\mashup-crafting`, `minecraft-survival`)

Paths under `entities/`, `world/`, `player/`, `core/` and `generator/` are in `third_party/minecraftoss/<crate>/src/`.

- **No mob.** No code names the sniffer: not in `SIMULATED_KINDS` (`crates/minecraft_terrain/src/server_mobs.rs:433`), no `type_info` row (`world/natural_spawner.rs:271`); its spawn egg spawns nothing (`docs/MINECRAFT-SURVIVAL.md:441`).
- **The egg block is inert.** `minecraft:sniffer_egg` places and draws from its three `hatch` models, but `on_place` (`world/level/mod.rs:677`) and `tick_block` (`:839`) have no `SnifferEggBlock` arm, and `local_shapes` (`player/lib.rs:795`) gives it the full cube (`:973`).
- **Its seeds already grow**: planting (`entities/crops.rs:40-41`), random ticks (`world/level/random_tick.rs:285`, `:486`), bone meal (`world/level/bonemeal.rs:57-76`).
- **No natural source.** Warm ocean ruins place suspicious sand with `archaeology/ocean_ruin_warm` (`generator/structure/kinds/ocean_ruin.rs:123`), the only table with `sniffer_egg`, but nothing brushes.
- **Closest brain: hoglins** (an `Animal` on a hand-written brain): `entities/hoglin_brain.rs` `look_sometimes` :458, `idle_movement` :485, `start_love` :616, `tick_love` :645; `entities/world/hoglins.rs` `hoglin_living` :462, `breed_hoglins` :628. Helpers: `entities/brain_behaviors.rs` `Living` :17, `shuffled` :138, `Walker` :165, `MoveSink` :183; `entities/stroll.rs:119` `land_random_position`; `entities/navigation.rs:119` `plan_walk_path(.., reach_range)` with `PlannedPath::reached` :111. Ravagers (1.95 wide) already path.
- **Animal helpers**: `entities/animal.rs:12` `interact`, `entities/age.rs:6` `Age`; tempting by item tag in `feline_people` (`entities/world/felines.rs:353`: `player_controls(id).holds`, `recipes.item_in_tag`).
- **Items and loot**: `World::spawn_item` (`player/lib.rs:117`, implemented `server_mobs.rs:182`), `World::block_in_tag` (`player/lib.rs:238`). Gift tables roll only in terrain: `roll_cat_gift`/`roll_hero_gift` (`entities/loot.rs:271`, `:276`), drained by `drop_cat_gifts` (`crates/minecraft_terrain/src/server.rs:1355`, called `:1107`).
- **Entity world fields** a family file reads: `dimension` (`entities/world.rs:2515`), `mob_drops` (`:2575`), `trade_experience` (`:2618`), `entity_events` (`:2623`; terrain drops them at `server.rs:1102`, E1 draws them).
- **Level side**: `Level::spawned` (`world/level/mod.rs:153`) becomes mobs via `take_spawned` (`server.rs:1016`); tag pattern `world/level/golem_build.rs:215-229`. `level_events` (`mod.rs:207`) reach host and friend (`crates/render_anim/src/minecraft_remote.rs:906`) and are drawn by `level_event` (`crates/minecraft_terrain/src/level_events.rs:87`). Block ticks save with the chunk (`core/chunk_nbt.rs:259`).
- **Client side**: `BlockParticles` (`crates/minecraft_terrain/src/block_particles.rs:25`) lives in render_anim (`minecraft_mining.rs:30`); the view queues `item_crumbs`/`potion_crumbs` (`crates/minecraft_survival/src/entity_view.rs:58-59`) for `present_crumbs` (`crates/render_anim/src/minecraft_world.rs:1611`; host `:1233`, friend `minecraft_remote.rs:1975`). Block sound types: `scene.rs:356`. Keyframes: `keyframes.rs:761` `AnimationLibrary`, `apply_state` :315, `apply_walk` :328. 26.3 baby meshes at full scale: `cow_render.rs:82`, `hoglin_render.rs:153`.

### Vanilla 26.3 (javap)

**Entity** (`Sniffer extends Animal`)
- `EntityTypes.SNIFFER`: `CREATURE`, `sized(1.9, 1.75)`, eye 1.05, `clientTrackingRange(10)`; peaceful-safe, not fire immune. No `SpawnPlacements` entry, no biome spawner.
- Attributes: animal base (tempt range 10), speed `0.10000000149011612`, health 14. `getBabyStartAge` **−48000**. Baby scale 0.5 (0.95 × 0.875, eye 0.525). In state `DIGGING`: `scalable(1.9, 1.35).withEyeHeight(0.81)` × age scale.
- Navigation `setCanFloat(true)`; maluses `WATER`, `ON_TOP_OF_POWDER_SNOW`, `DAMAGE_CAUTIOUS` −1 (`WATER` 0 while a path starts on fire or in water). `jumpFromGround`: if the speed modifier > 0 and horizontal speed² < 0.01 after the jump, `moveRelative(0.1, (0, 0, 1))`. `getMaxHeadYRot` 50. `supportQuadLeash` true.
- `Animal`: never removed far away, experience `1 + nextInt(3)`, ambient interval 120, food `#sniffer_food` (torchflower seeds). `actuallyHurt` resets love.
- **State** (synced, never saved): `IDLING` 0, `FEELING_HAPPY` 1, `SCENTING` 2, `SNIFFING` 3, `SEARCHING` 4, `DIGGING` 5, `RISING` 6. Each change restarts its own `AnimationState` and stops the rest. `transitionTo` sounds (volume 1): scenting `entity.sniffer.scenting` (pitch 1.3 for a baby), sniffing `.sniffing`, rising `.digging_stop`, happy `.happy`. Digging runs `onDiggingStart`: `DATA_DROP_SEED_AT_TICK = tickCount + 120` and entity event **63**.
- `canSniff`: not tempted, panicking, in water, in love, a passenger or leashed, and on the ground. `canDig()`: not panicking, tempted, a baby, in water or a passenger, on the ground, and `canDig(headBlock.below())`: the block in `#sniffer_diggable_block` (dirt, coarse and rooted dirt, mud, muddy mangrove roots, moss and pale moss, grass block, podzol), no explored `(dimension, pos)` equal to it, and `createPath(pos, 1)` exists and `canReach` (a path on every call).
- `getHeadBlock`: `containing(head.x, getY() + 0.2, head.z)`, `head = position + getForward() × 2.25`.
- `calculateDigPosition`: for i in 0..5, lazily, `LandRandomPos.getPos(this, 10 + 2i, 3)`, non-null, `containing`, inside the world border, `.below()`, `canDig`; the first that passes.
- Explored list: `storeExploredPosition` keeps the newest 20 and inserts the new `(dimension, pos)` first (up to 21). `onDiggingComplete(timedOut)` stores `getOnPos()` (under its feet) only when timed out; the check uses the head block. Port that quirk as is.
- `tick()`, before `super.tick()` advances `tickCount`: digging runs `emitDiggingParticles` then `dropSeed`; searching plays `entity.sniffer.searching` every 20 client ticks.
- `dropSeed` (server): at `DATA_DROP_SEED_AT_TICK == tickCount`, `dropFromGiftLootTable(gameplay/sniffer_digging)`, each stack as `new ItemEntity(level, headBlock.x, .y, .z, stack)` (the block's **corner**, default throw) with the default pickup delay, then `entity.sniffer.drop_seed`. With that order the seed falls on the 121st tick of the dig.
- `emitDiggingParticles`: while the dig animation is past 1700 ms and before 6000 ms (ticks 35-119) and the block under the head block is not `INVISIBLE`: 30 `BLOCK` particles of it at `atCenterOf(headBlock) − 0.65 y`, zero speed; on `tickCount % 10 == 0` its `SoundType` hit sound at 0.5, pitch 0.5 (client only). While digging, `gameEvent(ENTITY_ACTION, headBlock)` every 10 ticks.
- Client event 63 plays `SnifferSoundInstance`: `entity.sniffer.digging`, NEUTRAL, not looping, follows the sniffer, stops when it leaves digging or searching.
- Ambient `entity.sniffer.idle`, none while digging or searching (the roll and reset still run); hurt, death (`die` goes `IDLING` first), step `entity.sniffer.step` at 0.15.
- `mobInteract`: `Animal.mobInteract` (love at age 0, or growth for a baby) plays the eat sound, then the sniffer plays it again: two `entity.sniffer.eat` (NEUTRAL, 1, `randomBetween(0.8, 1.2)`).
- Breeding: `canMate` also needs both in idling, scenting or happy. `spawnChildFromBreeding` drops a `sniffer_egg` item at its position (default delay), `finalizeSpawnChildFromBreeding(level, partner, null)` (parents age 6000, love reset, event 18 on this one, `1 + nextInt(7)` experience if `mob_drops`), `block.sniffer_egg.plop` (1, `(r1 − r2) × 0.2 + 0.5`).

**Brain** (`SnifferAi`; brain tick, then the first valid of `DIG`, `SNIFF`, `IDLE`)
- Sensors `NEAREST_LIVING_ENTITIES`, `HURT_BY`, `NEAREST_PLAYERS`, `FOOD_TEMPTATIONS`. Saved memories: `sniffer_explored_positions` (`[{dimension, pos}]`), `sniff_cooldown` (Unit, ttl), `temptation_cooldown_ticks`, `is_panicking`. Not saved: `sniffer_sniffing_target`, `sniffer_digging`, `sniffer_happy`.
- `resetSniffing`: erase digging and sniffing target, go `IDLING`.
- **CORE**: `Swim(0.8)`, `AnimalPanic(2.0)`, `MoveToTargetSink(500, 700)`, `CountDownCooldownTicks(TEMPTATION_COOLDOWN_TICKS)`.
- **IDLE** (no `SNIFFER_DIGGING`): 0 `AnimalMakeLove(SNIFFER)`; 1 `FollowTemptation(1.25, close 2.5 baby / 3.5)`; 2 `LookAtTargetSink(45, 90)`; 3 `FeelingHappy(40-100)` (needs `SNIFFER_HAPPY`; stop erases it); 4 `RunOne`: `SetWalkTargetFromLookTarget(1.0, 3)` ×2, `Scenting(40-80)` ×1, `Sniffing(40-80)` ×1, `SetEntityLookTarget(PLAYER, 6)` ×1, `RandomStroll.stroll(1.0)` ×1, `DoNothing(5, 20)` ×2. The panic, temptation and love behaviours run `resetSniffing` on start (`SnifferAi$1/$2/$3`).
  - `Scenting`: no panic, digging, sniffing target, happiness or breed target, not tempted; runs its full time.
  - `Sniffing`: no walk target, sniffing target or `SNIFF_COOLDOWN`, not a baby, `canSniff` (also to continue). Stop goes `IDLING`; only if it ran its full time, `calculateDigPosition` sets the sniffing target and `WALK_TARGET(pos, 1.25, 0)`.
- **SNIFF** (no panic; sniffing target and walk target): `Searching`, 600 ticks, continues while `canSniff` (else `IDLING`) and the walk target is the sniffing target. Stop: `canDig()` and `canSniff` set `SNIFFER_DIGGING`; erase both targets.
- **DIG** (no panic, no walk target, `SNIFFER_DIGGING`): `Digging(160-180)` (no `SNIFF_COOLDOWN`, `canSniff`; continues while digging is set, `canDig()` and not in love; timed out sets `SNIFF_COOLDOWN` **9600**, else `resetSniffing`), then `FinishedDigging(40)` (needs the cooldown; `RISING`; stop `IDLING`, `onDiggingComplete(timedOut)`, erase digging, set `SNIFFER_HAPPY`).

**Model and render**
- `SnifferRenderer` (`AgeableMobRenderer`): `SnifferModel` on layer `SNIFFER` (192 × 192, `entity/sniffer/sniffer`) and on `SNIFFER_BABY` = `SniffletModel.createBodyLayer` (128 × 128, `entity/sniffer/snifflet`); shadow **1.1**. `BABY_TRANSFORM` and `SNIFFER_BABY_FALL` are unused by 26.3: port neither.
- Tree (both): `bone` holds `body` and six legs; `body` holds `head`; `head` holds `left_ear`, `right_ear`, `nose`, `lower_beak`.
- Adult: `bone` (0, 5, 0). `body` (0, 0, 0): uv (62, 68) (−12.5, −14, −20) 25 × 29 × 40; (62, 0) same 25 × 24 × 40 inflated 0.5; (87, 68) (−12.5, 12, −20) 25 × 0 × 40. Legs (−3.5, −1, −4) 7 × 10 × 8, right uv (32, 87/105/123), left (0, 87/105/123), at (∓7.5, 10, −15/0/15). `head` (0, 6.5, −19.48): (8, 15) (−6.5, −7.5, −11.5) 13 × 18 × 11; (8, 4) (−6.5, 7.5, −11.5) 13 × 0 × 11. Ears 1 × 19 × 7: left (2, 0) (0, 0, −3) at (6.51, −7.5, −4.51), right (48, 0) (−1, 0, −3) at (−6.51, −7.5, −4.51). `nose` (10, 45) (−6.5, −2, −9) 13 × 2 × 9 at (0, −4.5, −11.5). `lower_beak` (10, 57) (−6.5, −7, −8) 13 × 12 × 9 at (0, 2.5, −12.5).
- Snifflet: `bone` (0, 24, 0). `body` (6, −3, −9.5): (0, 35) (−13, −14, −0.5) 14 × 14 × 20 inflated 0.25; (0, 0) same 14 × 15 × 20; (68, 0) (−13, 0, −0.5) 14 × 0 × 20. `head` (−6, −4.75, 0): (68, 20) (−5, −4.25, −7.5) 10 × 9 × 9; (88, 20) (−5, 3.75, −7.5) 10 × 0 × 9. Ears 1 × 11 × 3: left (104, 38) (0, 0, −2) at (5, −4.25, −1.5), right (96, 38) (−1, 0, −2) at (−5, −4.25, −1.5). `nose` (68, 47) (−5, −3, −2) 10 × 3 × 4 at (0, −1.25, −9.5). `lower_beak` (68, 38) (−5, −2.5, −2) 10 × 5 × 4 at (0, 1.25, −9.5). Legs (−2, −1, −2) 4 × 5 × 4, right uv (0, 69/78/87), left (16, 69/78/87), at (∓4, −4, −7/0/7).
- `setupAnim`: head x/y rotation from the look; `SNIFFER_SNIFF_SEARCH` while searching else `SNIFFER_WALK`, `applyWalk(pos, speed, 9, 100)`; then `SNIFFER_DIG`, `_LONGSNIFF` (sniffing), `_STAND_UP` (rising), `_HAPPY`, `_SNIFFSNIFF` (scenting) from their states. Lengths: SNIFFSNIFF 8 s looping, LONGSNIFF 1 s, WALK and SNIFF_SEARCH 2 s looping, DIG 8 s, STAND_UP 3 s, HAPPY 2 s looping.

**Egg block** (`SnifferEggBlock`, `hatch` 0-2, shape `column(14, 12, 0, 16)`)
- `onPlace` (every state change): `boost` = block below in `#sniffer_egg_hatch_boost` (moss block). Boosted: level event **3009** (`EGG_CRACK` on every face, `UniformInt(3, 6)` each, 0.55 out; its sprite is `minecraft:glint`, as happy villager's). Then `gameEvent(BLOCK_PLACE)` and `scheduleTick((boost ? 12000 : 24000) / 3 + level.random.nextInt(300))`.
- `tick`: below 2, `block.sniffer_egg.crack` (BLOCKS, 0.7, `0.9 + r × 0.2`) and `setBlock(hatch + 1, flag 2)`, whose `onPlace` schedules the next stage. At 2: `block.sniffer_egg.hatch`, `destroyBlock(pos, false)`, a `BREEDING` sniffer, `setBaby(true)`, at `atCenterOf(pos)`, yaw `wrapDegrees(level.random.nextFloat() × 360)`. About 24000 ticks in all, 12000 on moss.
- Loot: `blocks/sniffer_egg` the egg (`survives_explosion`); `entities/sniffer` empty; `gameplay/sniffer_digging` (gift) torchflower seeds or pitcher pod.

### Plan

**Files (D0-3)**
- `third_party/minecraftoss/entities/src/world/families/land/sniffers.rs`: `pub struct Sniffer`, `impl Family for Sniffer`, tests. State: `age: Age`, `in_love`, `love_cause`, `last_hurt: Option<(&'static str, i32)>` (for `#panic_causes`), `state`, `state_ticks`, `drop_seed_at`, `brain`, and the movement fields hoglins have.
- `land/sniffers/brain.rs`: memories (`Slot`s), the activities and the six sniffer behaviours, `calculate_dig_position`, `can_sniff`, `can_dig`, `head_block`, `update_activity`.
- `crates/minecraft_terrain/src/families/land/sniffers.rs` (`TerrainFamily`) and `families/land/sniffers_render.rs`.
- `third_party/minecraftoss/world/src/level/sniffer_egg.rs`: `impl Level<'_>` with `sniffer_egg_on_place` and `sniffer_egg_tick`.
- Registration in the group's files only: `pub mod sniffers;` and `sniffers: sniffers::Sniffer,` in `family_group!(land 48..=63 { .. })`; `mod sniffers;` and `Sniffer` in `terrain_group!`; the row in `natural_spawner/families/land.rs`: `TypeInfo { 1.9, 1.75, NoRestrictions, MotionBlockingNoLeaves, fire false, Rules::Family { creature: true }, Walk::Animal, remove_far false, cluster 4 }`, `MobCategory::Creature`, follow range 16, `baby_chance: Some(0.05)`, no rule (no placement), no finalize.

**Hooks (D0-2.2)**
- `KEY` 56, `KINDS ["minecraft:sniffer"]`; `spawn`: `MobCore::new(kind, pos, 1.9, 1.75, 1.05, 14.0)`, `ambient_interval` 120. `category` `"creature"`; `despawn`, `experience`, `loot` default.
- `hurt`: default, then reset love and store the damage type in `last_hurt`. `leashable` true, `leash_walk` by its `GroundNavigation`; `panicking` = `is_panicking`.
- `voice` `("sniffer", 1.0, "neutral_volume")`; `step_sound` `Some("entity.sniffer.step")`.
- **Ambient, in the family file:** after `family_base_tick`, each `Voice::Ambient(pitch)` becomes `Voice::Event("entity.sniffer.idle", 1.0, pitch)`, or is dropped while digging or searching.

**Tick**: (1) while digging and `drop_seed_at == core.tick_count`, queue the seed gift and push `entity.sniffer.drop_seed`; (2) `family_base_tick` and the ambient mapping; (3) without `no_ai`: age and love, `state_ticks += 1`, the brain (a state change sets `state_ticks` 0), `steer`/`travel` as hoglins with the jump nudge, and the body and eye switch to 1.35 / 0.81 (× age scale) while digging; (4) `family_after_move`.

**Brain**: goats' `world/families/animal_brain.rs` gives `AnimalPanic`, `FollowTemptation` and `AnimalMakeLove`; the sniffer calls `reset_sniffing` when one reports a start, and passes `AnimalMakeLove` its `can_mate` and breed callback. Its RunOne adds `Scenting` and `Sniffing` to the shared entries in vanilla order and weights (`shuffled`). Tempting: `player_controls(id).holds(|i| recipes.item_in_tag("minecraft:sniffer_food", i))` as `feline_people`; no new `PlayerCandidate` field. Nearby living as `hoglin_living` builds it. `can_dig` uses `plan_walk_path(.., pos, 1)` and `reached()`.

**Breeding** (callback): `level.spawn_item(position, sniffer_egg, default throw, 10)`; parents age 6000, love 0; `(id, 18)` into `entity_events`; `1 + next_int(7)` into `trade_experience` with `mob_drops`; `Voice::Event("block.sniffer_egg.plop", 1.0, ..)`.

**Interact** (`TerrainFamily::interact`): `animal::interact` with `#sniffer_food` (love, growth, the golden dandelion lock); two `entity.sniffer.eat` on a used seed; falling in love pushes event 18.

**Saving** (`read`/`fields`): `Age`, `ForcedAge`, `InLove` as the other animals (`server_mobs.rs:1536`, `:1616`), and `Brain.memories` (explored positions as `{dimension, pos: [I; x, y, z]}`, `sniff_cooldown` with ttl, `temptation_cooldown_ticks`, `is_panicking`) in the form of `piglin_save.rs:106` (private: copy its few lines). The state is not saved.

**Multiplayer**: `TRACKING_RANGE` 10, `MAX_HEAD_Y_ROT` 50.0, `SHADOW_RADIUS` 1.1, both textures, `VOLATILE = ["ForcedAge"]` (`Age`, `InLove`, `Brain` are global, `mob_tracking.rs:15`). `show` = `FamilyShow { flags: 0, values: [state id, state_ticks, ..] }`; `show_on` sets both. Baby flag and the digging box come in the generic `ServerMob`. Server sounds go through `core.voices`; eggs and seeds are level items.

**Draw** (`posed_cube` into `out.models`): adult or snifflet mesh and texture; head look from `MobPose`; `SNIFF_SEARCH` or `WALK` through `apply_walk(.., 9.0, 100.0)`; the state's definition through `apply_state` from `start = age − state_ticks`. Definitions from `keyframes::shared()`.

**Client effects** (host and every friend): digging with `state_ticks` 35-119: 30 dust at `centre(head block) − 0.65 y` of the block below the head block, with its hit sound (0.5, 0.5) when the viewer's `ClientMob` age is a multiple of 10; `entity.sniffer.digging` once per dig (keyed in `fx.seen`); `entity.sniffer.searching` every 20 ticks while searching. Event 63 is not pushed: E1 has no row for it, and the shown state marks the dig.

**Shared-file lines** (next to a related arm, never at the end)
- `world/level/mod.rs`: `pub mod sniffer_egg;` after `pub mod wither_build;` (`:21`); a `SnifferEggBlock` arm after the `wet_sponge` arm in `on_place` (`:690-691`) and after the `FireBlock` arm in `tick_block` (`:842-843`). Not next to `ScaffoldingBlock`: M2's frogspawn arms go after `FrostedIceBlock` in `on_place` (`:705`) and after the coral arm in `tick_block`. Turtle eggs tick in `random_tick.rs` (M1-8); M3's creaking hearts and dried ghasts take other anchors. Note the anchors in the log.
- `crates/minecraft_terrain/src/level_events.rs`: `EGG_CRACK` 3009 beside `BONE_MEAL_USE` (`:8`), its arm after that arm (`:128-136`) calling `sniffers::egg_crack_particles` with `Kind::HappyVillager`.
- `player/lib.rs`: the egg's column (x 1-15, z 2-14) beside the cactus arm (`:812`).
- With W merged: `block_place` in `sniffer_egg_on_place`, `entity_action` every 10 dig ticks (W's helpers, from the family's files).

**Hooks** (D0-12)
- **The seed** uses M1-3's H3: `family_gift("minecraft:gameplay/sniffer_digging", "minecraft:sniffer", at)` with `at` the head block's corner, which is where vanilla's `new ItemEntity(level, headBlock.x, .y, .z, stack)` puts it (default throw and pickup delay, as H3 drops).
- **"Registered Minecraft mobs throw up block dust"** (own commit in `sniffers-dig`; W6's warden can use it with a 0.7 spread): `ClientFx.dust: &mut Vec<BlockDust { block, at, spread, count, hit_sound }>`, a `block_dust` list beside `item_crumbs` (`entity_view.rs:58`), drained in `present_crumbs` (`minecraft_world.rs:1611`, given the scene by both callers) into a new `BlockParticles::spawn_dust`, skipping air and invisible blocks, with `sound_type(block).hit` when asked. javap during the work: `TerrainParticle.Provider` and the `Particle` velocity jitter. If W6 has already landed its own block dust for registered mobs, use that instead and log it.

**Commit groups**

| # | Key | Commits | Row 46 |
|---|---|---|---|
| 1 | `sniffers-live` | "Minecraft sniffers live, wander and lay eggs like vanilla" (family, CORE and IDLE with scenting and the long sniff, breeding, feeding, the row, `TerrainFamily` save/load/show/interact, `TEXTURES` set, empty `draw`). | `live` |
| 2 | `sniffers-drawn` | "Minecraft sniffers and snifflets are drawn like vanilla" (both meshes, keyframes, head, searching walk). | `drawn` |
| 3 | `sniffers-dig` | 1. "Registered Minecraft mobs throw up block dust" (unless W6 landed one). 2. "Minecraft sniffers sniff out and dig up ancient seeds" (SNIFF and DIG, the seed, rising, happiness, explored list, cooldown, dust and sounds). | |
| 4 | `sniffer-eggs` | "Minecraft sniffer eggs crack and hatch snifflets, faster on moss" (block tick, 3009 sparkles, egg shape). | `done` |

Each group updates row 46 of `docs/round10/mobs.md` and the track log in the same commit. No serialized struct changes, so nothing under "## Protocol".

### Tests

| Commit | Tests |
|---|---|
| `sniffers-live` | entities: `a_sniffer_is_huge_with_fourteen_health_and_its_kind` (sizes and eyes adult and baby, speed, `entity_type`, census, `mob_candidates`, empty table, 1-3 experience adults only), `a_snifflet_starts_at_minus_48000`, `a_tempted_sniffer_follows_torchflower_seeds_and_cannot_sniff`, `two_fed_sniffers_drop_a_sniffer_egg_not_a_snifflet` (plop, ages 6000, event 18, 1-7 experience only with `mob_drops`), `sniffers_mate_only_while_idling_scenting_or_happy`, `a_hurt_sniffer_panics_at_double_speed_and_forgets_its_sniffing`, `a_snifflet_scents_at_pitch_1_3`, `a_sniffer_says_idle_and_keeps_quiet_while_digging_or_searching`, `a_sniffer_turns_its_head_at_most_fifty_degrees`, `a_stalled_sniffer_nudges_forward_after_a_jump`; world: `a_sniffer_row_has_its_size_and_never_spawns_naturally`; terrain: `a_sniffer_saves_and_loads_with_its_age_love_and_memories`, `a_friend_sees_a_sniffer_from_its_look_tag` (state, state ticks, `ForcedAge` dropped only for sniffers), `torchflower_seeds_put_a_sniffer_in_love_and_feed_a_snifflet` |
| `sniffers-drawn` | `an_adult_sniffer_has_its_parts_on_a_192_texture`, `a_snifflet_uses_its_own_mesh_and_texture`, `a_searching_sniffer_walks_with_its_nose_down`, `each_sniffer_state_plays_its_keyframes_from_its_start`, gated `the_sniffer_tables_match_the_jar` |
| `sniffers-dig` | `a_sniffing_sniffer_picks_a_diggable_spot_ten_to_eighteen_blocks_away` (seeded), `a_sniffer_walks_there_and_digs_for_160_to_180_ticks_at_1_35_high`, `a_digging_sniffer_drops_a_seed_at_its_nose_121_ticks_in`, `an_interrupted_dig_drops_nothing_and_sets_no_cooldown`, `a_finished_dig_rises_forty_ticks_feels_happy_and_waits_9600_ticks`, `a_sniffer_never_digs_stone_or_an_explored_spot`, `the_explored_list_keeps_the_newest_twenty_one`, `snifflets_never_sniff_long_or_dig`; terrain `a_digging_sniffer_throws_up_dust_and_plays_its_sounds_for_every_viewer`; with the hook, `registered_mob_dust_reaches_the_block_particles` |
| `sniffer-eggs` | world: `a_sniffer_egg_cracks_twice_and_hatches_a_snifflet` (8000 + `nextInt(300)` a stage, sounds, no drop, Age −48000, random yaw), `a_sniffer_egg_on_moss_hatches_twice_as_fast_and_sparkles_each_stage`, `a_sniffer_egg_keeps_its_stage_and_tick_across_save_and_load`; terrain `the_egg_crack_event_sparkles_three_to_six_on_each_face`; player `a_sniffer_egg_is_fourteen_by_twelve` |

### Owner's steps

Setup: release build, survival, **Normal**, by day, open grass of at least 40 × 40 (plains or meadow) in the Overworld, the kit below. Name screenshots by label, e.g. `sniffer01_wander.png`; two windows `sniffer09-host.png` and `sniffer09-friend.png`.

1. Use a sniffer spawn egg 8 blocks away: a huge reddish six-legged sniffer with a long nose and moss on its back wanders slowly; its head turns a good way before its body follows. **sniffer01_wander**
2. Watch for up to two minutes. It sometimes lifts its nose and sniffs the air in puffs, or lowers it for one long sniff. After a long sniff it walks nose-down to a spot 10-18 blocks away, sinks its nose into the ground and digs for about 8 seconds, with dirt bits flying in front of its nose and digging sounds. About 6 seconds in, a torchflower seed or pitcher pod pops out by its nose. It stands up and wiggles happily. **sniffer02_dig**
3. Plant the seed on farmland next to water; bone meal grows a torchflower or pitcher plant.
4. That sniffer does not dig again for 8 minutes (`/tick sprint 9600`, then it can). Lay a 7 × 7 stone floor on the grass, put oak fences on its outer ring, lead a fresh sniffer in and take the lead off: for three minutes it scents and sniffs but never walks off or digs.
5. Hold torchflower seeds within 10 blocks: sniffers follow and stop about 3.5 blocks away, and do not sniff meanwhile. Feed two adults a seed each: hearts, two eating sounds each, they meet, and a few seconds later a sniffer egg pops out with a plop and experience orbs. **sniffer03_bred_egg**
6. Place one egg on grass and one on a moss block (green sparkles at once on moss). `/tick sprint 13000`: the moss egg cracks twice (crack sounds, sparkles, more cracks on the texture) and hatches a snifflet as it breaks; the grass egg has cracked once. `/tick sprint 12000`: the grass egg hatches too. **sniffer04_hatch**
7. The snifflet is small, with its own rounder shape and texture; it scents at a higher pitch and never digs. Feeding it seeds speeds its growth. `/summon minecraft:sniffer ~ ~ ~5 {Age:-200}` grows up within 10 seconds.
8. Punch a sniffer: it runs off fast for a few seconds and stops sniffing or digging. Kill one: no item, 1-3 experience orbs.
9. Tie a sniffer to a fence with a lead: it never sniffs or digs while tied.
10. In creative, use a sniffer spawn egg on a placed spawner: a small sniffer turns in the cage. **sniffer05_cage** `/gamemode survival`, break it.
11. Save and quit with a sniffer on its cooldown and a once-cracked egg; load. The egg keeps its crack and hatches on time; the sniffer still waits out its cooldown.
12. Shoot a sniffer with the M4A1 and an arrow: red flash, panic, and it dies after enough hits.

Two windows (`scripts\mp-local.ps1`; the friend's `.env` has the same kit):

13. The host hatches two sniffers near the friend and waits for a dig. The friend sees the walk, the scenting, the nose-down search, the dig with its dirt and sounds and the seed popping out, in the same place. **sniffer09-host / sniffer09-friend**
14. The friend feeds two sniffers: both windows show hearts and the egg. The friend places it on moss; both see the sparkles, and after `/tick sprint 13000` the snifflet.
15. The friend shoots a sniffer with the M4A1: both windows show the flash and the panic.

### Kit

```
IW4L_MINECRAFT_KIT=minecraft:sniffer_spawn_egg*16,minecraft:sniffer_egg*8,minecraft:torchflower_seeds*64,minecraft:pitcher_pod*8,minecraft:moss_block*16,minecraft:grass_block*64,minecraft:podzol*32,minecraft:mud*16,minecraft:stone*64,minecraft:diamond_hoe,minecraft:water_bucket,minecraft:bone_meal*64,minecraft:lead*4,minecraft:oak_fence*32,minecraft:golden_dandelion*2,minecraft:spawner,minecraft:iron_sword,minecraft:bow,minecraft:arrow*32,minecraft:cooked_beef*32
```

20 stacks; every id checked against `assets/minecraft/items` in the 26.3 jar.

### Depends on

- **D0 d0a**: registry, hooks, spawn rows, client registry, drawing, keyframe holder (P1). Not d0b: sniffers do not fly.
- **M1-5 goats**: `animal_brain.rs`. **M1-3 pandas**: H3 (gift loot). **M1-6 armadillos**: the shadow hook (the snifflet keeps D0's halving, 0.55).
- **W**: W6 may land the block-dust hook first; the egg's `block_place` and the dig's `entity_action` wait for W's game events (P7).
- **E1**: love hearts (event 18). **E2**: `hit_boxes`, so snowballs and eggs hit sniffers.

### Left for later

- **Brushing suspicious sand and gravel**, the only natural source of sniffer eggs: a feature of its own with no owner this round. The kit hands out eggs.
- **The four-rope leash** (`supportQuadLeash`): one rope is drawn; happy ghasts need the same.
- **A digging sound that follows and cuts off**: here it plays once from where the dig starts.
- **The game events**, if W has not merged when the group runs.

### Open decisions (vanilla-like choice first)

1. **Ambient**: remap `Voice::Ambient` to `entity.sniffer.idle` in the family file, keeping vanilla's roll; this is the track's ambient rule (M10), so no family adds an `ambient` hook to D0's root.
2. **Dust and hit sound per client** through the block-dust hook (vanilla's are client side, per viewer); rather than server particles, which need a new message.
3. **Event 63**: push nothing; the shown state starts the dig sound.
4. **Egg collision**: add vanilla's column next to the cactus arm, rather than keep the full cube.
5. **Anchors**: egg arms after the `wet_sponge` and `FireBlock` arms, `pub mod sniffer_egg;` after `wither_build`, leaving the `ScaffoldingBlock`/`FrostedIceBlock` neighbourhood to M2's frogspawn and other neighbours to M3.

---

## M1-8. Turtles (segment m1g: groups `turtles-water` (optional), `turtles-live`, `turtles-drawn`, `turtles-eggs`, `turtles-hunted`)

Family `turtles`, group `land`, hit-key byte **53** (`Family::KEY`, row 20 of `docs/round10/mobs.md`), one kind `minecraft:turtle`. Turtle eggs are a block with a random tick in the level, not a mob. Lines are `minecraft-survival` at `d1a3d028`; re-check before editing. Where D0's code differs from `track-d0.md`, follow the code. Turtles run late: they build on M1-1 (`family_goal_step_with`, `animal_goals.rs` with `MoveToBlockGoal`), M1-3 (gifts, H3), M1-4 (the fox's turtle slot) and M1-6 (the shadow hook), and on M2's amphibious paths and `travel_swimming` (P2).

### What exists today

- **Dormant:** not in `SIMULATED_KINDS` (`crates/minecraft_terrain/src/server_mobs.rs:433`); `tracking_range` (`mob_tracking.rs:285`) gives 5.
- **Spawning never runs:** the size row is right (`natural_spawner.rs:289`) and the category is creature (`natural_spawner/spawning.rs:103`), but the rule falls to `Rules::Other` (`natural_spawner.rs:354`), which refuses with `note_unsupported` (`spawning.rs:766-769`). Turtles need a D0 row.
- **Hunters hold empty slots** (`Prey::Absent`, `monster_ai.rs:418-420`): zombie `monster_ai.rs:1965`, skeleton `:1287`, wither skeleton `:1309`, cat `feline_ai.rs:198-201` (second slot; the first is the rabbit's), ocelot `:244`, wolf `wolf_ai.rs:210`. The drowned's targets are hand-written (`world.rs:1645-1659`), as is the legacy zombie path (`:1297-1300`).
- **Zombie egg goal is a stub:** `TurtleEggGoal` (`monster_ai.rs:2185-2205`, added at `:1948` and `:1971`) finds an egg (`find_turtle_egg` `:2210`) and only keeps its timer (`turtle_egg_next_start` `:517`).
- **Egg block:** no random tick (`random_tick.rs:276-333` ends in `unsupported`); the hatch attribute is registered (`core/src/environment.rs:145`) and readable like `cat_gift_chance` (`level/time.rs:234-236`, `biome_value` `:261`).
- **Reuse:** amphibious walking (`navigation.rs:206` `navigate_amphibious_to_with_accuracy`, `:532` `tick_amphibious`, `stroll.rs:213` `amphibious_random_position_towards`; the drowned's A* mode `path_search.rs:48-73` knows four blocks only; no `AmphibiousNodeEvaluator`, decision 1); water travel `movement.rs:275-279`; `Body.step_after` (`movement.rs:76`); `PanicState` (`panic.rs:16`, water search only when burning, range 5), `TemptState` (`tempt.rs:30`), `BreedState` (`breed.rs:7`), `animal::interact` (`animal.rs:12`), `age::Age`; food tags via `player_controls` (`world/felines.rs:353-376`); the breed orb (`world/hoglins.rs:645-647`); `MoveToBlockGoal` (M1-1's port in `animal_goals.rs`); `WalkTarget` (`walk_path.rs:44-60`, `stroll.rs:14-44`); `Level.spawned` (`level/mod.rs:153`, pushed as in `golem_build.rs:215-229`, taken at `server.rs:1016-1020`); `Level.sea_level` (`level/mod.rs:173`), which the entity world already keeps as `EntityWorld.sea_level` (`world.rs:2569`, set every tick at `server.rs:1066`); the `playerDestroy` site (`server.rs:2287-2289`, `level/ice.rs:42`); adult and baby quadrupeds (`pig_render.rs:153-216`).
- **Loot gap:** `damage_source_properties` accepts only `source_entity` (`entities/src/loot.rs:636-646`), so `entities/turtle` (asks `#is_lightning`) rolls nothing today.
- Turtle helmet model (`armor_render.rs:18-25`), turtle master potions (`potions.rs:33-41`). No `EffectKind::WaterBreathing` (`player/src/survival.rs:36-59`).

### Vanilla 26.3 (javap this round)

- **`EntityTypes.TURTLE`:** `CREATURE`, `sized(1.2, 0.4)`, passenger attachment (0, 0.55625, -0.25), `clientTrackingRange(10)`, default eye 0.34, not fire immune, peaceful allowed. Baby: `BABY_DIMENSIONS` = scaled 0.3 → 0.36 x 0.12, eye 0.102; `getAgeScale` 0.3.
- **Attributes:** `createAnimalAttributes` (follow 16, tempt range 10) + health 30, speed 0.25, step height 1.0. Maluses `WATER` 0, the three door types -1, plus `Animal`'s fire ones.
- **Saved:** `home_pos` (int array; missing → `blockPosition()`), `has_egg`, the animal age and love fields. Not saved: travel point, going home, lay counter. Synced: `HAS_EGG`, `LAYING_EGG`. `finalizeSpawn`: `home_pos` = block position, then `AgeableMob` (0.05 babies after the first).
- **Goals:** 0 `TurtlePanicGoal(1.2)`, 1 `TurtleBreedGoal(1.0)`, 1 `TurtleLayEggGoal(1.0)`, 2 `TemptGoal(1.1, #turtle_food = seagrass)`, 3 `TurtleGoToWaterGoal`, 4 `TurtleGoHomeGoal`, 7 `TurtleTravelGoal`, 8 `LookAtPlayerGoal(Player, 8)`, 9 `TurtleRandomStrollGoal(1.0, 100)`. No targets, no float goal.
  - **Panic:** `shouldPanic`, then always `lookForWater(7)` (nothing solid at its feet; `findClosestMatch(pos, 7, 1, water)`), else a random spot.
  - **Breed** (`!hasEgg`): no baby. The goal's runner gets `has_egg`; both ages 6000, love reset; orb `nextInt(7) + 1` with `mob_drops`.
  - **Lay** (`MoveToBlockGoal(1.0, 16)`; egg, home within 9; target sand, air above): reached out of water → laying on; past 200 ticks: `entity.turtle.lay_egg` (0.3, 0.9 + 0.2 level random), `turtle_egg[eggs = nextInt(4) + 1]` above the sand, `block_place`, egg and laying off, `setInLoveTime(600)`.
  - **Go to water** (`MoveToBlockGoal(baby ? 2.0 : 1.0, 24)`, vertical start -1): babies on land always; adults not going home, no egg. Continues out of water, `tryTicks <= 1200`; repath every 160.
  - **Go home:** never a baby; with an egg always, else `nextInt(350) == 0` and home 64+ away. Ends within 7, stuck, or 300 ticks within 16. Waypoints `getPosTowards(16, 3, pi/10)`, else `(8, 7, pi/2)`; a spot 16+ from home that is not water re-rolls `(16, 5, pi/2)`.
  - **Travel** (in water, not going home, no egg): target = position + (`nextInt(1025) - 512`, `nextInt(9) - 4` → 0 above sea level - 1, `nextInt(1025) - 512`); waypoints as above, dropped unless chunks load 34 around; ends when the path ends, stuck, going home, in love or with an egg.
  - **Stroll:** not in water, not going home, no egg.
- **`TurtleMoveControl`:** first in water `dy += 0.005`, speed `max(s/2, 0.08)` when home is 16+ away, `max(s/3, 0.06)` for a baby; on ground `max(s/2, 0.06)`. No `MOVE_TO` or path done → speed 0. Else yaw `rotlerp(.., 90)` toward the point, body yaw = yaw, speed `lerp(0.125, speed, mod * 0.25)`, `dy += speed * (wy/len) * 0.1`.
- **Navigation:** `AmphibiousPathNavigation` (`AmphibiousNodeEvaluator(false)`); stable destination: water only while travelling, else not above air.
- **`travelInWater`:** `moveRelative(0.1)`, move, x 0.9, then `dy -= 0.005` unless it has a target or is going home within 20. Not pushed by fluid (the fork pushes no mob).
- **Walk value:** water (not going home) 10, sand below 10, else the light cost.
- **`aiStep`:** laying, every 5th tick on sand: level event 2001 below and `entity_action`. **`ageBoundaryReached`:** grown with `mob_drops` → gift `gameplay/turtle_grow` (one scute) at its feet.
- **Other:** no love with an egg; not leashable; `thunderHit` kills; breathes under water; `#aquatic`.
- **Sounds:** ambient every 200, `entity.turtle.ambient_land` only for an adult on ground out of water; `.hurt_baby`/`.death_baby` for babies; step `.shamble`/`.shamble_baby` at 0.15 every 0.15 walked; swim `.swim` at 1.5x; `.lay_egg`, `.egg_crack`, `.egg_hatch`, `.egg_break`; `entity.zombie.destroy_egg`.
- **`TurtleEggBlock`** (`HATCH` 0-2, `EGGS` 1-4):
  - `randomTick`: `turtle_egg_hatch_chance` at the position (day timeline: 1.0 from 21062 to 21905, else 0.002) vs `level.random.nextFloat()`, and sand below. Hatch < 2: crack sound, `HATCH + 1`, `block_change`. Hatch 2: hatch sound, block removed, `block_destroy`; per egg i: 2001 and a baby (age -24000, home = the block) at (x + 0.3 + 0.2i, y, z + 0.3).
  - `stepOn` (not sneaking): `destroyEgg(100)`; `fallOn` (not a `Zombie`): `destroyEgg(3)`. Never turtles or bats; players always (no spawn protection here); other mobs only with `mob_griefing`; `level.random.nextInt(n) == 0` → one egg fewer with the break sound, or gone at one.
  - `playerDestroy`: drops (silk touch only), then one egg fewer with the old state: mining takes one egg at a time. Eggs stack by hand to 4; `onPlace` on sand fires level event 2012.
- **Hunters** of `BABY_ON_LAND_SELECTOR` (`isBaby() && !isInWater()`): zombie, husk, zombie villager and drowned at 5 `(10, mustSee)`; every `AbstractSkeleton` at 3 `(10, mustSee)`; cat 1 and wolf 6 `NonTameRandomTargetGoal` (re-tests the selector to continue); ocelot 1 `(10, false)`; fox `(10, false)`.
- **`ZombieAttackTurtleEggGoal`** (`RemoveBlockGoal`, `MoveToBlockGoal(1.0, 24, 3)`): `mob_griefing`; 10 ticks after a find; two air blocks above; reach 1.14. Reached: hop (dy 0.3, -0.3 on even ticks), `destroy_egg` (0.5) every 6 ticks, after 60 the whole block removed with `egg_break` (0.7). `stop`: fall distance 1.
- **Render:** `AgeableMobRenderer(AdultTurtleModel, BabyTurtleModel, 0.7)`, `entityCutout`; shadow 0.7 x 0.3 x 0.83 for a baby. Textures `entity/turtle/turtle` (128 x 64), `turtle_baby` (16 x 16). State: on land, laying, egg (adult). No keyframe class.
  - Adult: `head` (3,0) 6x5x6 at (0,19,-10); `body` at (0,11,-10) x pi/2 with `shell` (7,37) 19x20x6 and `belly` (31,1) 11x18x3; `egg_belly` (70,33) 9x18x1; hind legs 4x1x10 at (±3.5,22,11); front legs 13x1x5 at (±5,21,-4).
  - Baby: `body` 4x2x4, `head` 3x3x3, four flat 2x0x1 legs (negative u at (-1,0) and (-1,1)).
  - `setupAnim` after `QuadrupedModel`: on land `f` = laying ? 4 : 1, `g` = laying ? 2 : 1; front `yRot = ∓cos(f·5·pos)·8·speed·g`, hind `yRot = ∓cos(5·pos)·3·speed`; in water `l = cos(pos·0.6662·0.6)·0.5·speed`, hind `xRot = l, -l`, front `zRot = -l, l`. Egg belly shown with `has_egg`, and then `root.y -= 1`.
- **Loot `entities/turtle`:** seagrass (weight 3) 0-2; a bowl only for `#is_lightning`. **Spawning:** `ON_GROUND`, `MOTION_BLOCKING_NO_LEAVES`, `y < seaLevel + 4`, sand below, raw brightness > 8; `beach` only, weight 5, 2-5.

### Plan

**Files (D0-3):** `entities/src/world/families/land/turtles.rs` (`Turtle` state: `Age`, love, `home`, `has_egg`, `laying`, `lay_counter`, `going_home`, `travel_pos`, last damage type, `MonsterAi`; `impl Family`; tick; tests); `.../turtles/ai.rs` (`TURTLE_SPEC`, the goals, `TurtleMoveControl`, `turtle_ai_step`: `monster_ai_step` `world.rs:1022-1097` with `tick_amphibious`, the turtle control and travel, keeping `play_movement`); `.../turtles/eggs.rs` (`RemoveEggGoal`, applying its work, the step pass); `world/src/level/turtle_egg.rs` (random tick, `playerDestroy`, `decrease_eggs`); terrain `families/land/turtles.rs` and `turtles/render.rs`. Registration lines in the three `land.rs` files; row 20 of `mobs.md`.

**Hooks:** `KEY` 53; `spawn` (step height 1.0, `step_after` 0.15); `category` creature; `hurt` (reset love, keep the damage type, `hurt_plain`, baby voices); `breathes_water` true; `voice` `("turtle", 1.0, "neutral")`; `loot` (damage type); `panicking`; `monster` → `TURTLE_SPEC` (eye 0.34, follow 16; `walk`: the maluses, step 1.0, `WalkTarget::Turtle`). Others default. The tick runs `family_base_tick`, `family_goal_step_with` (rabbits' hook) with `turtle_ai_step`, `family_after_move`; `GoalExtra` holds `TurtleGoals` (the state above, tempting players, partners within 8, the egg to place); the tick places the egg with `level.set_block` and keeps `core.baby` in step with `Age`. Swim and baby sounds: the tick rewrites its own `core.voices` after the step. The ambient follows the track's rule (M10): `core.ambient_interval` 200, and the tick turns `Voice::Ambient` into `entity.turtle.ambient_land` for an adult on the ground out of water and drops it otherwise. The tick copies `EntityWorld.sea_level` into `TurtleGoals` for the travel goal.

**One-line hooks in shared files:**
- Row in `natural_spawner/families/land.rs`: the size above, `Rules::Family { creature: true }`, `Walk::Animal`, kept far away, cluster 4, follow 16, `baby_chance: Some(0.05)`, the turtle rule (spawners too), finalize writing `home_pos`.
- `walk_path.rs:49`: `Turtle { going_home: bool }` right after `Animal` (a turtle is an `Animal` whose `getWalkTargetValue` overrides it); its arm right after the `Animal` arm (`stroll.rs:16-19`). Not after `Hoglin`, which is the last variant and the last arm (D0-3: never at the end of a list), and away from M2's `Guardian` beside `Strider`.
- Sea level: no new hook (decision 4).
- Water travel: M2's P2 `Body::travel_swimming(world, input, yaw, 0.1)` (beside `movement.rs:275`; it is `travel_water_mode(0.1, 0.9, false)`). No `travel_turtle` (decision 1).
- `panic.rs:16`: `can_use_with_water(.., range)` beside `can_use` (`manhattan_ordered(feet, 7, 1, 7, 15)` every time).
- `loot.rs`: `EntityLootContext.damage_type: Option<&'static str>` right after `has_direct_attacker` (`:19`), not after `attacker` (`:28`), where M2's frogs add `attacker_variant`. `damage_source_properties` (`:636-646`) becomes a match over the predicate's keys: `tags` (the damage type tags against `context.damage_type`) and `source_entity`, whose own keys are matched one arm each (`minecraft:entity_type` today). M1 owns this restructure, and M2's frogs then add one `minecraft:components` arm; if M2's change lands first, fold `tags` into its shape instead. The turtle's `loot` hook sets `damage_type` from its last damage. The scute uses M1-3's H3 (`roll_table`), so there is no `roll_gift`.
- `random_tick.rs:299`: a `TurtleEggBlock` arm after `CactusBlock`; `level/mod.rs`: `pub mod turtle_egg;` right after M1-7's `pub mod sniffer_egg;` (after `wither_build`, `:21`), away from M3's `pub mod lightning;` after `fire` (`:19`).
- `server.rs:2288`: `self.level.turtle_egg_player_destroyed(pos, old);` after `ice_player_destroyed` (survival breaks only).
- `monster_ai.rs:415`: `Prey::BabiesOnLand(&'static [&'static str])`, its arm next to D0's `Prey::Kinds` arm (`:2957` area; in water = `FluidFrame::sample(world, m.position, m.width, m.height).in_water()`) and in the `wild_only` branch (`:2993-2999`).
- Hunter slots → `Prey::BabiesOnLand(&["minecraft:turtle"])`: `monster_ai.rs:1287`, `:1309`, `:1965`; `feline_ai.rs:200`, `:244`; `wolf_ai.rs:210`; the fox's slot in `land/foxes/ai.rs`. M1 owns these lines and `:1948`, `:1971`; M2's drowned-only axolotl slot goes into its own list, not into `zombie_targets`.
- `monster_ai.rs:1948`, `:1971`: `RemoveEggGoal` in place of `TurtleEggGoal` (same priority and name, so `ZOMBIE_GOAL_NAMES` and `retain_goals` stay); its work (`egg: Option<Pos>`, the hop through `ctx.leap`) in a field beside `turtle_egg_next_start` (`:517`, default `:968`), applied after the zombie's `monster_ai_step` (`world.rs:6181`) next to E2's `apply_door_work` line; if E2's `doors-zombies` has not merged yet, right after that call, and E2's line then goes next to it.

**Multiplayer:** `TRACKING_RANGE` 10; `VOLATILE` `["has_egg"]`; `read`/`fields` `home_pos`, `has_egg` and the animal fields (as rabbits reads them); `FamilyShow.flags` bit 0 on land (`!touching_water && on_ground`), bit 1 laying, bit 2 egg (adult), no values (the flippers run off the pose's walk); `interact` feeds `#turtle_food` through `animal::interact`, refused with an egg. Eggs, cracks and hatchlings reach friends as blocks and look tags. `PROTOCOL`: no change.

**Keyframes:** none.

| Group | Commits | Contents |
|---|---|---|
| `turtles-water` (optional) | Only if M2's P2 has not merged when m1g starts: the amphibious paths and `travel_swimming`, one commit each under M2's names and subjects (decision 1). With P2 merged, no commit; the log says so. | Entities crate only. |
| `turtles-live` | 1. "Minecraft loot tables know what killed a mob" 2. "Minecraft turtles swim, walk the beach and find their way home in the entity world" 3. "Minecraft turtles mate for an egg and their babies grow up into scutes" | Entities and world crates, on M2's P2 or `turtles-water`. Row stays `left`. The same check as `rabbits-live` (M1-1): if D0's eggs survey or `is_simulated` read the entity-side kinds, commit 3 also adds the `TerrainFamily` with an empty `draw`, and `turtles-drawn` 1 keeps `interact` and the models. |
| `turtles-drawn` | 1. "Minecraft turtles are saved, hatch from spawn eggs and eat seagrass" 2. "Minecraft turtles are drawn like vanilla, for friends too" | `TerrainFamily`, both models, two textures, the shadow (decision 8). Row → `drawn`. |
| `turtles-eggs` | 1. "Minecraft turtles lay eggs on their home beach" 2. "Minecraft turtle eggs crack and hatch at dawn" 3. "Minecraft turtle eggs break under careless feet" | Lay goal; random tick, hatchlings via `Level.spawned`, one egg per mined block; the step pass (decision 5). |
| `turtles-hunted` | 1. "Minecraft mob targets can pick babies on land" 2. "Minecraft zombies, skeletons, wolves, cats, ocelots and foxes hunt baby turtles" 3. "Minecraft zombies trample turtle eggs" | Row → `done`; steps and kit in the log. |

### Tests

- **live 1:** `loot::tests::a_turtle_drops_seagrass_and_a_bowl_only_to_lightning`.
- **live 2:** `a_turtle_is_wide_and_flat_with_thirty_health_and_its_kind` (adult and baby sizes and eyes, step 1.0, census creature, `mob_candidates` baby flag, adults-only table, 1-3 experience, never drowns); `a_turtle_swims_with_its_own_move_control`; `a_hurt_turtle_runs_for_water_within_seven_blocks`; `a_turtle_follows_a_player_holding_seagrass`; `a_baby_turtle_on_land_hurries_to_water_at_twice_the_speed`; `a_turtle_far_from_home_heads_back_and_one_with_an_egg_always_does`; `a_turtle_in_water_travels_toward_a_random_point_below_sea_level`; `turtles_value_water_and_sand_as_walk_targets`; world `turtles_spawn_on_sand_below_sea_level_plus_four_in_daylight_and_remember_home`.
- **live 3:** `two_turtles_fed_seagrass_mate_and_one_carries_an_egg`; `a_turtle_with_an_egg_cannot_fall_in_love`; `a_baby_turtle_grows_up_and_drops_one_scute_with_mob_drops`.
- **drawn 1:** `a_turtle_saves_and_loads_with_its_home_and_egg`; `a_friend_sees_a_turtle_from_its_look_tag`; `seagrass_makes_a_turtle_fall_in_love_and_feeds_a_baby_up`.
- **drawn 2:** `render::tests::an_adult_turtle_has_seven_parts_on_128_by_64_and_a_baby_six_on_16`; `the_egg_belly_shows_only_on_an_adult_with_an_egg_and_lifts_it_a_pixel`; `turtle_flippers_paddle_on_land_and_in_water` (hand-computed angles).
- **eggs 1:** `a_turtle_with_an_egg_digs_on_sand_near_home_and_lays_one_to_four_eggs`; `a_turtle_lays_neither_in_water_nor_far_from_home`.
- **eggs 2:** world `turtle_eggs_on_sand_crack_twice_and_hatch_babies_at_dawn`; `turtle_eggs_off_sand_never_crack`; terrain `hatched_turtles_join_the_entity_world_as_babies_with_their_home`; `mining_turtle_eggs_takes_one_egg_at_a_time`.
- **eggs 3:** `walking_on_turtle_eggs_breaks_one_now_and_then_but_never_while_sneaking`; `mobs_break_turtle_eggs_only_with_mob_griefing_and_turtles_never_do`.
- **hunted:** `prey_babies_on_land_skips_adults_and_swimmers`; `a_zombie_hunts_a_baby_turtle_on_land_but_not_an_adult`; `skeletons_wolves_cats_and_ocelots_hunt_baby_turtles` (not when tame); `a_zombie_tramples_turtle_eggs_after_three_seconds_on_them`; `zombies_leave_turtle_eggs_alone_without_mob_griefing`. The zombie fixtures (`bin/check_zombie_full_ai.rs`) stay green.

### Owner's steps (survival, Normal, by day, `/gamerule mob_griefing true`; at a beach via `/locate biome minecraft:beach` and `/tp`, or on sand beside a 6 x 6 pool two deep)

1. Use the turtle egg twice on sand by the water: two big flat turtles plod slowly, front flippers sweeping sideways; in the water they paddle and sink gently. **turtle1_beach**
2. Hold seagrass: they come. Feed each one: they meet, and soon one has a pale bulge under its shell and stands a pixel higher. **turtle2_egg_belly**
3. That turtle goes back to where it hatched, onto sand there, flaps its front flippers fast for 10 s, and 1-4 eggs appear with a soft sound; the bulge is gone. **turtle3_laying**
4. `/time set 21000`, `/gamerule random_tick_speed 100`: within about 45 s the eggs crack, crack more and pop into one tiny turtle each, which scurry to the water. **turtle4_hatch** `/gamerule random_tick_speed 3`. Eggs on stone never crack.
5. `/summon minecraft:turtle ~ ~ ~2 {Age:-200}`: 10 s later it is grown and a scute lies at its feet. An adult killed by the sword drops 0-2 seagrass and experience, a baby nothing. The bow and the M4A1 each kill one with the red flash.
6. Hit a turtle on land a few blocks from water: it rushes into the water.
7. `/setblock ~2 ~ ~ minecraft:turtle_egg[eggs=4]` on sand. Walk over it for 10 s: now and then one egg cracks away. Crouching, none do. Mine it with the shovel: one egg goes, three stay; with the silk touch pickaxe you get one egg item.
8. `/time set midnight`, `/gamemode creative`. A baby (`{Age:-24000}`) and an adult on dry sand, a zombie 12 blocks away: it kills the baby and ignores the adult; a baby in water is left alone. **turtle8_zombie_hunt** The same with a skeleton (it shoots), a wolf, a cat, an ocelot and a fox.
9. Eggs on sand and a zombie 8 blocks away: it walks over, hops on them with a crunch three times a second, and after 3 s they are gone. **turtle9_zombie_eggs** With `/gamerule mob_griefing false` it ignores them. `/gamemode survival`, `/time set day`.
10. Save and quit with a turtle carrying an egg, a baby and cracked eggs; load: all three as they were.
11. In creative, a turtle egg on a spawner: a small turtle turns in the cage.
12. `/tp` along the beach into new land: groups of two to five turtles on the sand. **turtle12_beach_spawn**
13. Two windows (`scripts\mp-local.ps1`): the friend sees the turtles, the bulge, the digging and the hatching; seagrass in the friend's hand tempts and feeds them; the friend's M4A1 kills one in both windows. **turtle13-host**, **turtle13-friend**

### Kit

```
IW4L_MINECRAFT_KIT=minecraft:turtle_spawn_egg*16,minecraft:seagrass*64,minecraft:seagrass*64,minecraft:sand*64,minecraft:sand*64,minecraft:water_bucket,minecraft:water_bucket,minecraft:turtle_egg*16,minecraft:zombie_spawn_egg*4,minecraft:skeleton_spawn_egg*2,minecraft:wolf_spawn_egg*2,minecraft:cat_spawn_egg*2,minecraft:ocelot_spawn_egg*2,minecraft:fox_spawn_egg*2,minecraft:diamond_pickaxe+silk_touch=1,minecraft:iron_shovel,minecraft:iron_sword,minecraft:bow,minecraft:arrow*64,minecraft:spawner,minecraft:shield,minecraft:cooked_beef*32,minecraft:glass*64,minecraft:oak_planks*64
```

24 stacks, every id in the 26.3 `assets/minecraft/items`. The friend's `.env` needs the same line.

### Depends on

- **D0 d0a** (`registry-core`, `-hooks`, `-spawning`, `-client`, `-draw`). Not d0b.
- **Earlier M1 groups:** M1-1's `family_goal_step_with` and `animal_goals.rs` (panic, tempt, breed, `MoveToBlockGoal`); M1-3's H3 (the scute); M1-4's fox prey slot; M1-6's shadow hook.
- **M2 `water-movement` (P2):** the amphibious paths (`WalkProfile.amphibious`, `WaterNav::Amphibious`) and `Body::travel_swimming` (decision 1). The orchestrator merges `minecraft-survival` into `track-m1` before m1g (M10).
- **E1:** hearts (event 18), no turtle row. **E2:** `hit_boxes` for thrown items; `apply_door_work` places the zombie's egg work.
- **W:** `game_event` lines for `block_place`, `block_change`, `block_destroy`, `entity_action`, added by whichever merges second.

### Left for later

- Lightning (`thunderHit`, the bowl): M3's P8 adds the root hook `Family::thunder_hit`; the turtle's override (killed outright) goes in once both have merged. The loot condition answers false meanwhile.
- Turtle shell Water Breathing: no `EffectKind`, and M2 edits that enum.
- Stacking eggs by hand (candles and sea pickles do not stack either); landing on eggs (`fallOn`, per-family landing paths, as farmland).
- Particles 2012 and 2001 (sparks, digging dust, crumbs) and the zombie's egg crumbs and poofs.
- The drowned's turtle hunting and egg goal (hand-written, `world.rs:1645-1659`); Impaling against `#aquatic`; stats.

### Open decisions (recommended choice)

1. **Amphibious paths and water travel:** M2's P2 owns them (`WalkProfile.amphibious` and its branches in `walk_path.rs`, `WaterNav::Amphibious` in `navigation.rs`, `Body::travel_swimming` in `movement.rs`; axolotls and frogs need them too). Turtles wait for them (M10). Only if P2 is still missing when m1g must run, build them here under M2's names and subjects, one commit each, and the second track to merge drops its copy.
2. **Baby-on-land prey:** `Prey::BabiesOnLand` beside `Prey::Kinds`, testing water at the candidate. A `MobCandidate.in_water` field would touch five struct literals, a `NearestTargetGoal` field every literal.
3. **Walk value:** `WalkTarget::Turtle { going_home }`, like `Strider` and `Hoglin`.
4. **Sea level:** `EntityWorld.sea_level`, which the entity world already keeps (`world.rs:2569`); no `World::sea_level`.
5. **Trampling:** one pass after the mob tick (right after M1-3's `self.drop_family_gifts();`, which follows `drop_cat_gifts`, `server.rs:1107`): players not crouching or prone, and mobs other than turtles and bats (only with `mob_griefing`), whose feet rest on an egg top roll `level.random.nextInt(100) == 0`.
6. **Sounds:** rewrite the turtle's own voices in its tick, the ambient included; no hook.
7. **Scute:** M1-3's H3 gift (`gameplay/turtle_grow`), never a hard-coded item.
8. **Shadow:** M1-6's `shadow_radius` hook gives the baby vanilla's 0.174 (D0's halving would give 0.35).

---

## M1-9. Parrots (segment m1h, after d0b: groups `parrots-live`, `parrots-drawn`, `parrots-tame`, `parrots-shoulders`, `parrots-shoulders-seen`)

Family `parrots`, kind `minecraft:parrot`, hit-key byte **51**, group `land`. It is row 18 of `docs/round10/mobs.md`.

- Every group needs D0 d0a.
- `parrots-live` also needs d0b (P3).
- Paths are as of `minecraft-survival` `d1a3d028`. Re-check every line before editing.

### What exists today

- **Spawning.** The size row is at `third_party/minecraftoss/world/src/natural_spawner.rs:287` (`0.5, 0.9, OnGround, MotionBlocking`).
  - Its rules fall to `Rules::Other` (`:354`), so parrots never spawn.
  - Leaves are already a valid spawn block for parrots (`:720`), and the category is Creature (`spawning.rs:103`).
  - Parrots from vanilla-generated jungles stay dormant tags.
- **Nothing in `crates/`.** There is no entity, save, model or interaction.
- **Player files.** `PlayerData::merged` (`crates/minecraft_survival/src/player_nbt.rs:385-401`) keeps unknown keys, so `ShoulderEntityLeft/Right` from a vanilla file survive a save. Nothing reads or draws them yet. The owned keys are `OWNED` (`:151-179`).
- **Code to reuse** (the Plan cites the exact lines it edits):
  - **Tame cat goals.** Walk-based and private to `entities/src/feline_ai.rs`: `PanicGoal` `:249` with `panic_cause` `:103`, `teleport_to_owner` `:631`, `SitGoal` `:787` and `FollowOwnerGoal` `:1087`.
  - **Taming.** `EntityWorld::feline_interact` (`world/felines.rs:446`, events 7/6 at `:542-551`), with tags read through `recipes.item_in_tag` (`player/src/crafting.rs:870`).
  - **Flying stroll.** The wither's `FlyingStrollGoal` (`wither_ai.rs:250`) on `stroll.rs:302` and `:315`.
  - **No fall damage.** `body.fall_distance = 0`, as in `world/vexes.rs:123`.
  - **Client flap.** The client already runs the chicken flap for every mob (`client_mobs.rs:694-701`, `MobPose.flap` `:786`). `chicken_render.rs:155` turns it into `flapAngle`.
  - **An entity carried as a tag by a player.** The root vehicle path: `ServerSim::root_vehicle` and `reattach` (`server.rs:601`, `:618`) on `server_mobs::mob_tags` (`:1493`) and `spawn_saved_all` (`:501`).
  - **Per-player outputs.** Mounts are routed with `survival/entities.rs:362` and taken with `:1017`.
  - **Player vitals.** `PlayerVitals` (`monster_ai.rs:251`).
  - **Friends' gear.** `GearView` (`messages.rs:622`) and `append_body_gear` (`body_render.rs:42`, elytra on `rig.armor.body.upper` at `:70-76`).
  - **The hurt counter.** `Survival::hurts` (`survival.rs:533`), already compared by sleep (`sleep.rs:471`).

### Vanilla 26.3 (javap)

#### Entity type and spawn placement

- `EntityTypes.PARROT`: `CREATURE`, `sized(0.5, 0.9)`, `eyeHeight(0.54)`, `passengerAttachments(0.4625)`, `clientTrackingRange(8)`. It is not fire immune and is allowed in peaceful.
- `SpawnPlacements`: `ON_GROUND`, `MOTION_BLOCKING`, `checkParrotSpawnRules`. The rule is the block below in `#parrots_spawnable_on` (`grass_block`, `air`, `#leaves`, `#logs`) and raw brightness above 8.

#### `Parrot` (extends `ShoulderRidingEntity`, which extends `TamableAnimal`, which extends `Animal`)

- **Attributes.** Animal attributes (follow range 16), max health 6, flying speed 0.4, movement speed 0.2.
- **Controls and navigation.**
  - `FlyingMoveControl(this, 10, false)`.
  - Maluses −1 for `FIRE_IN_NEIGHBOR`, `FIRE` and `COCOA`.
  - `FlyingPathNavigation` with `setCanOpenDoors(false)` and `setCanFloat(true)`.
- **Overrides.**
  - `canBeABaby`, `isFood` and `canMate` are false.
  - `checkFallDamage` is empty, so there is no fall damage.
  - `omnidirectionalAirMover` is true (vertical air drag 0.91, 3D walk animation).
  - `canFlyToOwner` is true.
  - `doPush` skips players: it never pushes a player, but others push it.
  - The sound source is `NEUTRAL`. The lead offset is `(0, 0.5·eye, 0.4·width)`.
- **Goals** (no target goals):
  - **0, `TamableAnimalPanicGoal(1.25)`:** `#panic_causes`. Its `tick` first teleports to the owner when it can move to the owner and the owner is at least 12 blocks away.
  - **0, `FloatGoal`.**
  - **1, `LookAtPlayerGoal(Player, 8)`.**
  - **2, `SitWhenOrderedToGoal`** (JUMP, MOVE):
    - It needs tame or ordered to sit, not in water, and on the ground.
    - With no owner, or the owner in another level, it may start.
    - Within 12 blocks of an owner whose `getLastHurtByMob` is set, it may not.
    - Otherwise it starts when ordered to sit.
  - **2, `FollowOwnerGoal(1.0, 5, 1)`** (MOVE, LOOK):
    - It starts at 5 blocks or more and stops at 1 block or less, or when the path is done.
    - Every `adjustedTickDelay(10)` it either teleports (12 blocks or more) or does `moveTo(owner)`.
  - **2, `ParrotWanderGoal(1.0)`:** a `WaterAvoidingRandomFlyingGoal` with probability 0.001, interval 120 and the no-action check. Its `getPosition`:
    1. In water: `LandRandomPos(15, 15)`.
    2. When `nextFloat() >= 0.001`: `getTreePos`, the first spot in `betweenClosed(±3, ±6, ±3)` (x fastest, skipping its own block) with a `LeavesBlock` or `#logs` block below and empty air at and above it, aimed at the bottom centre.
    3. Otherwise the hover position, then the air-and-water position.
  - **3, `LandOnOwnersShoulderGoal`** (no flags):
    - `canUse`: the owner is a player in this level, not a spectator, not flying, not in water and not in powder snow. The parrot is not ordered to sit, and `rideCooldownCounter > 100` (counted up every tick, not saved).
    - `tick`: when the parrot is not seated, not in the sitting pose, not leashed, and its box meets the owner's, it calls `setEntityOnShoulder`. That saves the parrot as `saveWithoutId` plus `id`, hands the tag to the player, and calls `discard()` on success.
  - **3, `FollowMobGoal(1.0, 3, 7)`:**
    - It picks the first visible mob of another class (any mob but a parrot) within 7 blocks of its box.
    - It continues while the mob is more than 3 blocks away and the path is running.
    - Every 10 ticks it moves to the mob, or within 3 blocks stops. Within √3 blocks, or when the mob looks exactly at it, it steps to the mirrored point.
    - It does nothing while leashed.
- **`aiStep`:**
  1. Imitation: with `level.random.nextInt(400) == 0`, `imitateNearbyMobs`.
  2. `super.aiStep`.
  3. Flapping:
     - `flapSpeed += (onGround||passenger ? −1 : 4)·0.3`, clamped to 0..1;
     - `flapping = 1` when airborne and below 1, then `flapping ×= 0.9`;
     - **airborne and falling: `velocity.y ×= 0.6`**;
     - `flap += flapping·2`.
- **Flap sound.** `Entity.applyMovementEmissionAndPlaySound` adds `flyDist += |move|·0.6`. Over an air block, when `flyDist > nextFlap`, `onFlap` plays `entity.parrot.fly` at 0.15 and sets `nextFlap = flyDist + flapSpeed/2`. That is also where `GameEvent.FLAP` is posted.
- **`imitateNearbyMobs(level, entity)`:**
  - It needs the entity alive and not silent, and `nextInt(2) == 0` on the level random.
  - It picks a random mob of a mapped type within 20 blocks of the entity's box. Unless that mob is silent, it plays the mapped sound at the entity, with the entity's source, volume 0.7 and pitch `(r−r)·0.2+1`.
  - The 42 mapped types: blaze, bogged, breeze, camel_husk, cave_spider (plays the spider sound), creaking, creeper, drowned, elder_guardian, ender_dragon, endermite, evoker, ghast, happy_ghast (`EMPTY`), guardian, hoglin, husk, illusioner, magma_cube, parched, phantom, piglin, piglin_brute, pillager, ravager, shulker, silverfish, skeleton, slime, spider, stray, vex, vindicator, warden, witch, wither, wither_skeleton, zoglin, zombie, zombie_horse, zombie_nautilus, zombie_villager.
  - The sounds are `entity.parrot.imitate.<name>`, all of them in 26.3.
- **Other sounds.**
  - `getAmbient`: when not peaceful and `level.random.nextInt(1000) == 0`, a random imitation; otherwise `entity.parrot.ambient`.
  - Also `entity.parrot.hurt`, `.death`, `.step` (0.15) and `.eat`. The voice pitch is `(r−r)·0.2+1`.
- **`mobInteract`**, checked in this order:
  1. **Not tame and holding `#parrot_food`** (wheat, melon, pumpkin, beetroot and torchflower seeds, pitcher pod):
     - use one item;
     - unless silent, play `eat` at 1 with pitch `1+(r−r)·0.2`;
     - `random.nextInt(10) == 0` means `tame(player)` (no order to sit) and event **7**; otherwise event **6**.
  2. **`#parrot_poisonous_food` (cookie):** use one, add Poison for 900 ticks, then, if the player is in creative or the parrot is not invulnerable, `hurt(playerAttack, Float.MAX)`. It dies with loot and experience.
  3. **On the ground, tame and owned:** toggle `orderedToSit`.
  4. **Otherwise:** pass.
- **Hurt and spawn.**
  - `hurtServer`: when not invulnerable, `setOrderedToSit(false)`, then the parent call.
  - `finalizeSpawn`: `Variant = values[level.random.nextInt(5)]` and `AgeableMobGroupData(false)`, so no baby draw.
- **Saved fields.** `Variant` (Int, clamped: 0 red_blue, 1 blue, 2 green, 3 yellow_blue, 4 gray), `Owner` (UUID ints, only when tame) and `Sitting` (Byte). On load, an owner means tame, and the sitting pose equals `Sitting`.
- **From `TamableAnimal`:**
  - `handleEntityEvent`: 7 gives 7 `HEART`, 6 gives 7 `SMOKE`.
  - `canBeLeashed` is true.
  - `unableToMoveToOwner`: ordered to sit, a passenger, `mayBeLeashed`, or a spectator owner.
  - Teleport: 10 tries, with dx and dz in −3..3 (not both under 2) and dy in −1..1. The spot must be `WALKABLE` and free of collision. **Leaves below are allowed, because `canFlyToOwner` is true.** It snaps to `(x+0.5, y, z+0.5)`.

#### `ServerPlayer` and the shoulders

- **Data.** `shoulderEntityLeft/Right` (compound tags), synced as `Player.DATA_SHOULDER_PARROT_LEFT/RIGHT` (the variant id). Saved as `ShoulderEntityLeft`/`ShoulderEntityRight`, only when not empty.
- **`setEntityOnShoulder`.** It refuses when the player is riding, off the ground, in water or in powder snow. Otherwise it fills the left shoulder if empty, then the right, and records the game time.
- **`handleShoulderEntities`, every tick.**
  - For each non-empty, non-`Silent` parrot shoulder, with `nextInt(200) == 0`: imitate around the player, or failing that play `getAmbient` (PLAYERS source, volume 1).
  - Then, when `fallDistance > 0.5`, or the player is in water, flying, sleeping or in powder snow: `removeEntitiesOnShoulder`.
- **`removeEntitiesOnShoulder`.** It only acts when `timeSat + 20 < gameTime`. It creates each parrot from its tag (`LOAD`), calls `setOwner(player)`, places it at the player's position + (0, 0.7, 0) with `addWithUUID`, then clears the shoulder.
- **Also called from** `Player.hurtServer` (any hurt past the invulnerability checks), riptide start, `ServerPlayer.die` and spectator mode. `restoreFrom` keeps both shoulders across dimensions.

#### Client

- **`ParrotRenderer`.** Shadow 0.3. Textures `entity/parrot/parrot_{red_blue,blue,green,yellow_blue,grey}`. `flapAngle = (sin(lerp flap)+1)·lerp flapSpeed`.
- **Pose.** PARTY (client-only jukebox flag), else SITTING (`isInSittingPose`), else FLYING (`!onGround`), else STANDING.
- **`ParrotModel.createBodyLayer`** (32×32 texture, 11 cubes; the head row's four children hang off the head):

  | Part | `texOffs` | Box | Pose |
  |---|---|---|---|
  | body | 2,8 | `(−1.5,0,−1.5, 3,6,3)` | `(0,16.5,−3)`, rot x 0.4937 |
  | tail | 22,1 | `(−1.5,−1,−1, 3,4,1)` | `(0,21.07,1.16)`, rot x 1.015 |
  | left_wing / right_wing | 19,8 | `(−0.5,0,−1.5, 1,5,3)` | `(±1.5,16.94,−2.76)`, rot (−0.6981, −π, 0) |
  | head | 2,2 | `(−1,−1.5,−1, 2,3,2)` | `(0,15.69,−2.76)` |
  | head2 | 10,0 | `(−1,−0.5,−2, 2,1,4)` | `(0,−2,−1)` |
  | beak1 | 11,7 | `(−0.5,−1,−0.5, 1,2,1)` | `(0,−0.5,−1.5)` |
  | beak2 | 16,7 | `(−0.5,0,−0.5, 1,2,1)` | `(0,−1.75,−2.45)` |
  | feather | 2,18 | `(0,−4,−2, 0,5,4)` | `(0,−2.15,0.15)`, rot x −0.2214 |
  | left_leg / right_leg | 14,18 | `(−0.5,0,−0.5, 1,2,1)` | `(±1,22,−1.05)`, rot x −0.0299 |

- **`setupAnim`.** It runs `prepare(pose)` and sets the head's `xRot`/`yRot` from the look. Then by pose:
  - **FLYING:** legs `xRot` +0.6981317, then the common block.
  - **STANDING:** legs `xRot += cos(walkPos·0.6662 (+π for the right leg))·1.4·walkSpeed`, then the common block.
  - **SITTING** (`prepare` only, no common block):
    - head, tail, body, wings and legs move down 1.9 (y);
    - tail `xRot` +π/6;
    - wing `zRot` ∓0.0873;
    - legs `xRot` +π/2.
  - **ON_SHOULDER:** the common block.
  - **The common block:**
    - `bob = flapAngle·0.3` is added to the y of every part;
    - tail `xRot += cos(walkPos·0.6662)·0.3·walkSpeed`;
    - wing `zRot = ∓(0.0873 + flapAngle)`.
- **`ParrotOnShoulderLayer`.** It translates by `(left ? 0.4 : −0.4, crouch ? −1.3 : −1.5, 0)` in player-model space, which puts the parrot's feet on the body top. It draws the parrot in pose ON_SHOULDER with the player's age, walk, head yaw and head pitch, `flapAngle` 0 and no overlay.
- **No keyframes.** There is no `*Animation` class, so the parrot does not use `keyframes::shared()`.

#### Data

- **Loot `entities/parrot`:** feathers, uniform 1-2, plus 0-1 per Looting level. Experience 1-3 (`Animal`).
- **Spawners:** `jungle` and `bamboo_jungle`, weight 40, count 1-2. None in `sparse_jungle`.
- **Dancing:** client-only (`LevelEventHandler.notifyNearbyEntities`, level events 1010/1011, the jukebox's block box grown by 3).

### Plan

#### Files (D0-3)

- **Entities:** `entities/src/world/families/land/parrots.rs`, plus `land/parrots/ai.rs` (goals) and `land/parrots/shoulder.rs`. They hold:
  - `pub struct Parrot { variant, tame, owner: Option<u128>, ordered_to_sit, sitting_pose, ride_cooldown, flap_speed, flapping, fly_dist, next_flap, landing: Option<(u64, bool)>, panicking }`;
  - `impl Family` and the `MonsterSpec`;
  - `IMITATIONS: [(&str, Option<&str>); 42]`;
  - `EntityWorld::parrot_interact(id, player, inventory, hand, infinite)`, shaped like `feline_interact`;
  - the tests.
- **Terrain:** `crates/minecraft_terrain/src/families/land/parrots.rs` (`TerrainFamily`, landing and release helpers) and `land/parrots/render.rs` (model, shoulder drawing).
- **World:** one row.
- **Survival:** new `crates/minecraft_survival/src/shoulders.rs` and `pub mod shoulders;`.

#### Registration lines

- `pub mod parrots;` and `parrots: parrots::Parrot,` in `family_group!(land 48..=63 {..})`.
- `pub(crate) mod parrots;` and `Parrot` in `terrain_group!(land {..})`. It must be `pub(crate)` because `body_render.rs` calls it. If D0's root keeps `land` private, widen that one line in its own commit.
- The `minecraft:parrot` row in `ROWS`.

#### Registry hooks (D0-2.2)

- **`KEY` and `KINDS`.** `KEY = 51`, `KINDS = ["minecraft:parrot"]`.
- **`spawn`.** 0.5×0.9, eye 0.54, health 6.
- **`tick`**, in this order:
  1. `ride_cooldown += 1`;
  2. `family_base_tick`;
  3. the 1-in-400 imitation;
  4. `family_goal_step`;
  5. d0b's `FlyingMoveControl(10, false)` and `FlyingPathNavigation`. If d0b leaves `MonsterGoalContext.navigation` walk-only, keep the flying navigation in `GoalExtra` and run it after the goal step; the bees (M1-10) reuse whichever wiring this group lands;
  6. the flapping fall (×0.6);
  7. the flap clock and its sound;
  8. `fall_distance = 0`;
  9. `family_after_move`.
- **Hooks that differ from the defaults.**
  - `category`: `"creature"`.
  - `hurt`: `ordered_to_sit = false`, then `hurt_plain`.
  - `leashable`: true. `leash_walk`: the flying navigation towards the holder.
  - `panicking`: from the panic goal.
  - `step_sound`: `entity.parrot.step`.
  - `monster`: `Some(&PARROT)`, with eye 0.54, follow 16, and a `walk` that sets the three maluses, `can_float` and the animal walk values.
- **Defaults kept.** `voice`, experience, loot and despawn.
- **Ambient.** D0's base tick rolls and plays `entity.parrot.ambient`; right after it, the tick replaces that `Voice::Ambient` with a random imitation when `level.random.nextInt(1000) == 0` and the difficulty is not peaceful (the track's ambient rule, M10). No ambient hook. `core.ambient_interval` 120.
- **A hook that may be missing.** If D0 does not provide it, add it to the root in its own commit, named for the hook: a "pushes players" trait, if `pushes_on_local_player` reads `pushable`.
- **`TerrainFamily`.**
  - `TRACKING_RANGE`: 8.
  - `VOLATILE`: `["Owner", "Sitting"]`.
  - `TEXTURES`: the five.
  - `SHADOW_RADIUS`: 0.3.
  - `read`/`fields`: `Variant` (clamped), `Owner`, `Sitting`.
  - `interact`: calls `parrot_interact`.

#### Multiplayer

- **`FamilyShow`.** `flags` bit 0 is `SITTING`. Bit 1 is `FLYING`, which is `!on_ground` on the host. There are no values: the variant comes from the look tag, and every viewer computes the flap from its own `MobPose`.
- **Events.** Events 7 and 6 go to `world.entity_events`. E1's default `puff_for` row (7 Heart, 6 Smoke) covers them, so no new row is needed.
- **Protocol.** `GearView.shoulders` is a serialized change, so it goes under "## Protocol" in the log. No bump.

#### One-line hooks in shared files (logic stays in the parrot files)

1. **`leash_render.rs:76`:** a new line `"parrot" => (0.5, 0.4),` right after the `"ocelot"` row (M1-4's fox line sits there too); the ocelot row itself stays.
2. **Perch bits.**
   - `monster_ai.rs:251`/`:259`: `PlayerVitals.perch: u8` (1 OPEN, 2 LEFT, 4 RIGHT, 8 LEFT_TAKEN, 16 RIGHT_TAKEN), default 0.
   - Survival `entities.rs:1524`: `perch: self.players.queue(actor.id).perch,`.
   - A new `PlayerQueue.perch` (`:191`), set by `Entities::set_perch`.
3. **Terrain `server.rs`.**
   - `take_shoulder_landings` next to `root_vehicle` (`:601`), and `release_shoulder` next to `reattach` (`:618`). Each is a few lines that call into the parrot file.
   - `Command::ReleaseShoulder` next to `Reattach` (`:3454`), with its dispatch next to `:4136` and its handle method next to `:3886`.
   - `Output.shoulder_landings: Vec<(u64, bool, Tag)>` next to `root_vehicles` (`:3662`).
   - `out.shoulder_landings.extend(sim.take_shoulder_landings());` next to `out.dismounts.extend(sim.take_vehicle_losses());` (`:4262`).
4. **Survival `entities.rs`.** The landings loop next to the mounts loop (`:362`), `take_shoulder_landings` next to `take_mounts` (`:1017`), and `release_shoulder` next to `reattach` (`:1120`).
5. **Survival per tick.** `crate::shoulders::tick(globals, level, players);` after each per-game-tick `sleep::tick`: `sleep.rs:770` and `render_anim/src/minecraft_levels.rs:117`. These run for every level and player, friends included.
6. **Player and save.**
   - `player.rs`: a `shoulders` field with its default, and its line in `data()` (`:265`).
   - `survival/server.rs:871`: one line next to `player.reattach`.
   - `player_nbt.rs`: `PlayerData.shoulders: [Option<Tag>; 2]`, read and written as `ShoulderEntityLeft/Right` (skipped when empty), and both keys added to `OWNED` (`[&str; 29]`).
7. **Gear.**
   - `GearView.shoulders: [Option<u8>; 2]` (`messages.rs:622`) and `BodyGear.shoulders` (`body_gear.rs:29`).
   - `gear.rs`: `gear_view`, `gear_of` and `player_gear`.
   - `body_render.rs`: after the elytra block (`:73-76`, before the held-items loop at `:77`), one call to `families::land::parrots::render::append_shoulders(mesh, atlas, &rig.armor, &gear.shoulders, sky, block)`.
   - E1 (`player-looks`, segment e1b) also edits `body_render.rs` and `body_gear.rs`: re-check them after `minecraft-survival` is merged in, and keep the shoulder lines right after the elytra lines.

#### The shoulder flow

1. **Survival sets the perch bits** (`shoulders::tick`, each tick):
   - OPEN means not a spectator, not flying, not in water and not in powder snow (`Survival::in_powder_snow`, `survival.rs:284`).
   - LEFT and RIGHT mean OPEN, that shoulder free, on the ground (`Place.on_ground`) and not riding.
   - TAKEN means that shoulder is occupied.
2. **The goal lands the parrot.** The owner comes from `players` via `uuid_of`, and the bits come from `world.player_vitals`. The player's box is 0.6 wide and 1.8 high (1.5 when crouching). On contact, the goal sets `landing = Some((player, left))` and clears that bit in the vitals copy, so a second parrot in the same tick takes the other shoulder.
3. **The server turns it into a tag.** `take_shoulder_landings` builds the parrot's tag with `mob_tags` (the same way `root_vehicle` does), removes the mob and its `mob_tags` entry, and returns `(player, left, tag)`.
4. **Survival seats it** on the asked side, or the other free side, or else releases it again. It records `since`.
5. **Release.** Survival releases both shoulders, subject to the 20-tick guard, when:
   - the fall distance is over 0.5;
   - the player is in water, flying, sleeping or in powder snow;
   - `survival.hurts()` has changed;
   - the player died or is a spectator.

   Releasing sends `ReleaseShoulder`. The server spawns the tag with `spawn_saved_all`, sets `Owner` and puts the parrot at feet + 0.7 with the same UUID. A riptide start can call it with one optional line in `trident.rs`.
6. **Shoulder ambient** (1 in 200 per TAKEN bit) runs in the parrot file's landings pass on the server. It imitates within 20 blocks of the player, or plays the ambient, into `Output.mob_sounds` (category `players`).
7. **Drawing on the MW2 body.**
   - The frame is `rig.armor.body.upper`, the one the elytra uses. The parrot's feet sit on the body top, ±6.4 px out; left is the left arm's side.
   - The crouch shift is skipped, because the MW2 spine already crouches and goes prone.
   - The head yaw and pitch come from the head matrix relative to the body. Walk and flap are 0.
   - The owner sees their own parrots only in a friend's window, because the fork has no F5 view.

#### COMMIT GROUPS

| # | Group key | Commits | Needs |
|---|---|---|---|
| 1 | `parrots-live` | 1. "Minecraft parrots fly, wander to trees and imitate monsters like vanilla" (any missing-hook root commits first; family, float/panic/look/wander/follow-mob goals, flight, slow fall, no fall damage, flap sound, imitation, unsit on hurt, lead, no pushing players). 2. "Minecraft parrots spawn in jungles in five colours and are saved, hatched and seen by friends" (row and finalize, `TerrainFamily` with an empty `draw`). Row 18 becomes `live`. | d0a, d0b |
| 2 | `parrots-drawn` | "Minecraft parrots are drawn like vanilla" (model, textures, poses, flap from `MobPose`, shadow, cages, lead offset). Row 18 becomes `drawn`. | 1 |
| 3 | `parrots-tame` | "Minecraft parrots are tamed with seeds, sit when told and die of cookies" (`parrot_interact`, events 7/6, sit and follow-owner goals, the leaves-allowed teleport, the panic teleport). | 1 |
| 4 | `parrots-shoulders` | 1. "Minecraft parrots ride on their owners' shoulders and fly off when they jump" (goal, perch bits, landings, release and drop rules, `ReleaseShoulder`, shoulder sounds; the shoulders live in `player.rs` for the session). 2. "Minecraft players keep their shoulder parrots in their player files" (`player_nbt.rs` `ShoulderEntityLeft/Right`, `OWNED`, `data()`, the `survival/server.rs:871` load line). | 2, 3 |
| 5 | `parrots-shoulders-seen` | "Friends see parrots on each other's shoulders" (`GearView`, `BodyGear`, `gear.rs`, the MW2 body drawing after the elytra, the protocol note). Split from `parrots-shoulders`, which already touches about a dozen shared sites. Row 18 becomes `done`. Steps and kit go to the log. | 4 |

### Tests

**`parrots-live`**
- **Entities:**
  - `a_parrot_is_half_a_block_wide_with_six_health_and_its_kind` (size, eye, health, kind, census as creature, `mob_candidates`, the `entities/parrot` table, 1-3 experience);
  - `a_parrot_flies_with_ten_degree_turns_and_falls_at_six_tenths`;
  - `a_parrot_takes_no_fall_damage_from_twenty_blocks`;
  - `a_wild_parrot_wanders_to_the_air_above_leaves_or_logs_nearby`;
  - `a_parrot_follows_another_mob_within_seven_but_not_another_parrot`;
  - `a_parrot_imitates_a_zombie_within_twenty_blocks_at_seven_tenths` (seeded; no imitation without a mapped mob, and none for a happy ghast);
  - `the_ambient_is_an_imitation_one_time_in_a_thousand_except_on_peaceful`;
  - `a_flying_parrot_flaps_with_a_sound_every_half_flap_speed`;
  - `a_hurt_parrot_stops_sitting`;
  - `a_parrot_never_pushes_a_player`;
  - `a_lead_pulls_a_flying_parrot`.
- **World:** `parrots_spawn_on_leaves_logs_and_grass_in_the_light_with_a_random_colour` (the row overrides the size row; stone below and light 8 are refused; `Variant` comes from the level random; no baby draw).
- **Terrain:**
  - `a_parrot_saves_and_loads_with_its_variant_owner_and_sitting`;
  - `a_friend_sees_a_parrot_from_its_look_tag` (the volatile keys are dropped; `SITTING` and `FLYING` are shown).

**`parrots-drawn`**
- `a_parrot_has_eleven_cubes_on_a_32_texture_in_five_colours`;
- `a_flying_parrot_bends_its_legs_forty_degrees_and_beats_its_wings`;
- `a_sitting_parrot_drops_one_point_nine_pixels_and_tucks_its_legs`;
- `a_standing_parrot_swings_its_legs_with_the_walk`.

**`parrots-tame`**
- `seeds_tame_a_parrot_one_time_in_ten_with_hearts_or_smoke` (roll, events, item used unless infinite, eat sound);
- `a_cookie_poisons_and_kills_a_parrot_blamed_on_the_player`;
- `the_owner_sits_a_tame_parrot_on_the_ground_but_not_in_the_air`;
- `a_tame_parrot_follows_from_five_and_teleports_from_twelve_even_onto_leaves`;
- `a_sitting_parrot_stands_when_its_owner_is_hurt_nearby`;
- terrain `a_player_tames_a_parrot_with_seeds_through_mob_actions` (M1-1's rabbits added the first `families::interact` test of D0-9.4; this one covers taming).

**`parrots-shoulders`**
- 1: `a_tame_parrot_lands_on_its_owners_free_shoulder_after_a_hundred_ticks` (left first, then right);
- 1: `a_parrot_does_not_land_while_sitting_leashed_or_on_a_flying_swimming_or_spectating_owner`;
- 1: `server::tests::a_landed_parrot_leaves_as_a_tag_and_comes_back_at_its_owner` (`id`, `UUID`, `Variant` and `Owner` in the tag; released at feet + 0.7 with the same UUID);
- 1: `shoulders::tests::parrots_leave_on_a_fall_water_flight_sleep_powder_snow_hurt_death_or_spectator_but_not_in_the_first_second`;
- 2: `player_nbt::tests::shoulder_parrots_save_and_load_as_vanilla_compounds`.

**`parrots-shoulders-seen`**
- `gear::tests::a_player_shows_its_shoulder_parrots_to_friends`;
- a `minecraft_net` round trip of `Equipment` with shoulders.

### Owner's steps

Survival, Normal, by day, flat open ground. P-12 and P-13 need two windows (`scripts\mp-local.ps1`).

1. Use a parrot egg five times on the ground 6 blocks away. Small parrots in random colours appear (red-blue, blue, green, yellow-blue, grey). They flap up and fly about with a soft flutter sound, and walk with small leg swings on the ground. **p1_parrots**
2. Build a 3-high jungle log pillar topped with jungle leaves, 5 blocks away. Within about a minute, parrots land on it. Hatch a zombie 8 blocks away: a parrot follows it at about 3 blocks.
3. Use wheat seeds on a parrot again and again. Each use takes a seed and plays the eat sound. Once E1 is merged, failures show grey smoke and success shows hearts. About one seed in ten tames it. Tame a second one. **p2_tamed**
4. Sprint 8 blocks away: the tame parrots follow. Sprint 15 blocks away, round a wall: they appear beside you.
5. Stand still on flat ground for 5 s, then walk into a tame parrot. It vanishes onto your shoulder. Walk into the second: it takes the other shoulder. Now and then you hear a parrot near you. Near a zombie, you sometimes hear a zombie groan that the zombie didn't make.
6. Jump. Both parrots appear at chest height and flutter down. They land on you again when you walk into them. Wading into water or lying in a bed at night also sends them off.
7. With a parrot on your shoulder, save and quit, then load. Jump: the parrot comes off.
8. Use an empty hand on a tame parrot on the ground. It sits (body lowered, legs tucked) and no longer follows you or lands on you. **p3_sitting** Hit it once: it stands up.
9. Feed a cookie to a parrot. It dies at once and drops feathers.
10. Kill parrots with the M4A1, the bow and the iron sword: each flashes red and drops 1-2 feathers (Looting never reaches the fork's loot context, "Left for later"). Put a lead on a parrot: it flies after you.
11. `/locate biome minecraft:jungle`, `/tp` there: parrots perch on the treetops. Shoot one: it falls slowly, flapping. **p4_jungle**
12. Friend window. Put a parrot on the host's left shoulder. The friend sees it there in the same colour, turning its head with the host's head. **p5_friend_shoulder** The host jumps: the parrot flies off in both windows.
13. The friend tames a parrot and walks into it. The host sees it on the friend's shoulder. The friend sits another parrot: the host sees it sitting. **p6_friend_tame**
14. `/gamemode creative`, place a spawner and use a parrot egg on it: a small parrot turns in the cage. `/gamemode survival`.

### Kit

```
IW4L_MINECRAFT_KIT=minecraft:parrot_spawn_egg*16,minecraft:wheat_seeds*64,minecraft:melon_seeds*16,minecraft:pumpkin_seeds*16,minecraft:beetroot_seeds*16,minecraft:torchflower_seeds*8,minecraft:pitcher_pod*8,minecraft:cookie*8,minecraft:jungle_log*32,minecraft:jungle_leaves*32,minecraft:zombie_spawn_egg*4,minecraft:skeleton_spawn_egg*4,minecraft:creeper_spawn_egg*2,minecraft:lead*4,minecraft:iron_sword,minecraft:bow,minecraft:arrow*64,minecraft:spawner,minecraft:red_bed,minecraft:water_bucket,minecraft:powder_snow_bucket,minecraft:stone_bricks*64,minecraft:cooked_beef*32
```

23 stacks. Every id was checked against 26.3 `assets/minecraft/items`. The friend in P-13 uses the same kit.

### Depends on

- **D0 d0a:** registry, rows, `TerrainFamily`, `FamilyShow`, `families::interact`, the lead, `MonsterSpec` and `GoalExtra`.
- **D0 d0b:** P3 flying, for `parrots-live`.
- **E1 `particles-events`:** taming hearts and smoke. Taming works without it.
- **E2 `ti1`:** snowballs, eggs and tridents hitting parrots.
- **W (P7):** the `minecraft:flap` vibration. It is one line in the flap clock once W is merged.
- **Other M1 groups:** none needed; the bees (M1-10) reuse the d0b wiring this section lands.

### Left for later

- **Dancing to jukeboxes** (`PARTY`). The fork plays no jukebox discs (`docs/MINECRAFT-SURVIVAL.md:351`), and vanilla's flag is client-only. Once discs play, it needs one `FamilyShow` flag and the PARTY branch.
- **The owner's death message for a tame parrot** (`TamableAnimal.die`). No fork pet has one yet.
- **The 3D walk animation.** `client_mobs.rs:687` is horizontal only.
- **The flap of a parrot riding a boat or minecart.**
- **Silent mobs.** Imitation does not skip them.
- **Advancements.**
- **Vanilla's crouch shift on the shoulder.** The MW2 spine stands in for it.

### Open decisions (recommended choice first)

1. **Spawn rule.** Use `TypeInfo.rules = Rules::SpawnableOn("minecraft:parrots_spawnable_on")` with no `FamilyRule`. `check_spawn_rules` (`natural_spawner.rs:761`) already matches `checkParrotSpawnRules` exactly. The row is walk `Animal`, not removed far away, cluster 4, `baby_chance: None`.
2. **Perch path.** Put one field on `PlayerVitals`, fed through `PlayerQueue`. The alternatives touch more code: a new `TickInput` field (3 constructions) or `PlayerView` fields (about 11).
3. **Shoulder frame.** Use `rig.armor.body.upper`, with the head turn taken from the head matrix. The fallback is a fixed head.
4. **Client flap.** Reuse `MobPose.flap`/`flap_speed` instead of sending floats in `FamilyShow`.
5. **Shoulder ambient.** Run it on the server from the TAKEN bits, so imitation searches the authoritative world.
6. **Ambient imitation.** Remapped in the tick from the base tick's roll (M10), not a root `ambient_sound` hook. M3's allays draft asks for such a hook; it should follow the same rule.

---

## M1-10. Bees, hives and honey (segment m1i, after d0b: groups `bees-live`, `bees-drawn`, `bees-hives`, `bees-homes`, `bees-pollination`, `bees-honey`)

Family `bees`, group `land`, byte and `Family::KEY` **57**, kind `minecraft:bee` (row 44). Bees fly with D0's P3 (segment d0b). Their homes are bee nests and beehives, which are block entities. A hive ticks in the vendored level and hands its bees back to the entity world as saved tags. Lines refer to `C:\Dev\Soscraft\mashup-crafting` at `d1a3d028`; names follow `track-d0.md`. Re-check each line before editing. Bees run last: they reuse M1-9's flying wiring, M1-4's `spared_blocks` and M1-3's `World::raining` (H4).

### What exists today

- **No bee entity.**
  - `minecraft:bee` is not in `SIMULATED_KINDS` (`server_mobs.rs:433`), so eggs, `/summon` and saved bees go dormant (`server.rs:911` `add_saved_entity`).
  - No spawn table names the bee.
- **Nests generate with bees stored in vanilla format.**
  - `generator/src/feature/tree/decorator.rs:209-215`: 2-3 bees per nest, `ticks_in_hive` 0..598.
  - `generator/src/feature/region.rs:258` `add_bees`: `min_ticks_in_hive` 600.
  - `world/src/level/grow.rs:28-39`: the sapling `*_bees_005` features.
  - `core/src/block_entity.rs:223`: the hive tag keeps `bees` and `flower_pos`.
  - Nothing ticks a hive, so its bees never leave.
- **POIs.** `PoiType::Beehive`/`BeeNest` (`entities/src/poi.rs:37-38`, 0 tickets at `:108`) are loaded per chunk (`server.rs:881`) into `EntityWorld.pois` (`world.rs:2588`). `PoiManager::in_range` is at `poi.rs:728`.
- **`gameplay/bees_stay_in_hive`.**
  - It is registered (`core/src/environment.rs:151`), and rain or thunder sets it true (`:668`).
  - The day timeline sets it from tick 12542 to 23460.
  - There is no level accessor yet. The pattern is `monsters_burn` (`world/src/level/time.rs:227`); `is_raining` (`level/fire.rs:65`) is public since M1-3's H4.
- **Block-entity tickers.**
  - `has_ticker` (`level/container.rs:475`) and `tick_block_entities` (`level/piston.rs:606`, arms `:621-626`) cover hoppers, moving pistons and daylight detectors.
  - Only `ensure_block_entity` (`level/mod.rs:496`) registers one. `insert_chunk` (`:318`) registers none.
- **Hives in shared code.**
  - Explosions only log `unsupported` (`level/explosion.rs:409`).
  - The comparator has no hive arm (`level/redstone.rs:494`; the composter is at `:506`).
  - Loot `copy_components` is a no-op (`core/src/loot.rs:718`).
- **Honey items exist:** the honey bottle cures poison (`player/src/food.rs:96`), along with honey-block physics, honeycomb waxing and the recipes.
- **Closest code:**
  - Animals on the goal framework: felines (`world/felines.rs:574` `feline_breed`, `:842` `animal_interact`; `feline_ai.rs:302` tempt, `:1154` breed) and wolves (`wolf_ai.rs`).
  - Shared state machines: `age.rs`, `animal.rs:12`, `breed.rs`, `tempt.rs`, `follow_parent.rs`.
  - `NeutralMob` anger: the enderman's (`monster_ai.rs:278`, `:735` `angry`, `:762` `stop_being_angry`, `:2869` the 20-39 s timer).
  - Crop ages: `crops.rs:9`.
  - Flying random positions: `stroll.rs:302` `hover_random_position` and `:315` `air_and_water_random_position`. Their parameters already equal `BeeWanderGoal`'s (8/7/π/2, 1-3 above; 8, 4, -2).
  - Baby model ports: `chicken_render.rs`, `pig_render.rs`, `wolf_render.rs`. Small flyer: `bat_render.rs:134`.
  - Item use: the chain at `survival/src/actions.rs:1074-1110`, with `ItemOnBlock` at `server.rs:31` and `item_on_block_by` at `:2485`.
  - Breaking: `player_mines` (`server.rs:2276`; `ice_player_destroyed` per block; `HowBroken::SilkTouched` at `:26`).
  - Level-to-mob feeds: `server.rs:1062-1067`. After the mob tick: `drop_cat_gifts` (`:1107`). Entity events are dropped at `:1102` until E1.
  - Level events: 1505 only (`level_events.rs:128`).

### Vanilla 26.3 (javap)

**Entity type.** `EntityTypes.BEE`: CREATURE, `sized(0.55, 0.5)`, `eyeHeight(0.3)`, `clientTrackingRange(8)`. Not fire-immune; peaceful-safe.
- There is no `SpawnPlacements` entry, so the rules pass.
- No biome spawns bees.
- Loot `entities/bee` has no pools. `#beehive_inhabitors` = bee.

**`Bee extends Animal implements NeutralMob`.**
- **Attributes:** health 10, flying speed 0.6, movement speed 0.3, attack 2. `Animal` adds follow range 16 and tempt range 10.
- **Controls:** `FlyingMoveControl(this, 20, true)`. `BeeLookControl` is skipped while angry, and does not reset xRot while pollinating.
- **Maluses:** FIRE -1, WATER -1, WATER_BORDER 16, COCOA -1, FENCE -1.
- **Navigation:** `Bee$1 extends FlyingPathNavigation` with no doors, no float, `requiredPathLength` 48. `isStableDestination` = the block below is not air. Its `tick` is skipped while pollinating.
- **Overrides:**
  - `getWalkTargetValue`: air 10, else 0.
  - Silent: `playStepSound` is empty and there is no ambient sound.
  - `checkFallDamage` is empty.
  - `getSoundVolume` 0.4; `entity.bee.hurt` and `entity.bee.death`.
  - `jumpInLiquid` +0.01 y; `omnidirectionalAirMover` true.
  - `getLeashOffset` (0, 0.5 × eye, 0.2 × width).
- **Saved data:**
  - `hive_pos` and `flower_pos` (int arrays, nullable), `HasNectar`, `HasStung`, `TicksSincePollination` (the no-nectar ticks), `CannotEnterHiveTicks`, `CropsGrownSincePollination`;
  - `anger_end_time` and `angry_at` (`NeutralMob`).
  - Synced flags: roll 2, stung 4, nectar 8.
  - `updatePersistentAnger` starts the 20-39 s timer once the target differs from the anger target, so `setTarget` alone makes a bee angry.
- **`doHurtTarget`:**
  - `sting(bee)` (`minecraft:sting`, scaling `when_caused_by_living_non_player`; "was stung to death") for `(int) attack` = 2.
  - On a hit: post-attack enchantments, the stinger count +1, and Poison 10 s on Normal or 18 s on Hard (none on Easy).
  - Then `setHasStung`, `stopBeingAngry` and `entity.bee.sting`.
- **`customServerAiStep`:**
  - Under water for more than 20 ticks: 1 drown damage per tick.
  - Once stung, every 5 ticks: `nextInt(clamp(1200 - t, 1, 1200)) == 0` deals generic damage equal to its health.
  - No nectar: `ticksWithoutNectar++`.
  - Then `updatePersistentAnger`.
- **`aiStep`:**
  - Counts down the stay-out time and the hive and flower cooldowns (200; the flower cooldown starts at 20-60).
  - Rolling = angry, not stung, and the target within 2 blocks.
  - Every 20 ticks an invalid hive (none there, or 48 or more blocks away) is cleared.
- **`tick`:** roll +0.2 or -0.24, clamped to 0..1. On the client, with nectar and fewer than 10 crops: a 5% chance of 1-2 `falling_nectar` drips.
- **`hurtServer`** stops pollinating.
- **`mobInteract`:**
  - A `#bee_food` `FlowerBlock` with a bee effect is used up and gives the effect: wither rose Wither 40 ticks, open eyeblossom Poison 25 ticks.
  - Anything else goes to `Animal.mobInteract` (love 600, or a baby grows).
- **`wantsToEnterHive`:**
  - Stay-out ≤ 0, not pollinating, not stung, no target.
  - Then nectar, or more than 3600 no-nectar ticks, or `BEES_STAY_IN_HIVE` at its position.
  - And no `minecraft:fire` (a `FireBlock`) in the hive's 3×3×3.

**Goals.** Every `BaseBeeGoal` also needs `!isAngry()`.

| Priority | Goal |
|---|---|
| 0 | `BeeAttackGoal(1.4, true)`: melee while angry and not stung |
| 1 | `BeeEnterHiveGoal` |
| 2 | `BreedGoal(1.0)` |
| 3 | `TemptGoal(1.25, #bee_food)`, `ValidateHiveGoal`, `ValidateFlowerGoal` |
| 4 | `BeePollinateGoal` |
| 5 | `FollowParentGoal(1.25)`, `BeeLocateHiveGoal`, `BeeGoToHiveGoal` |
| 6 | `BeeGoToKnownFlowerGoal` |
| 7 | `BeeGrowCropGoal` |
| 8 | `BeeWanderGoal` |
| 9 | `FloatGoal` |
| target 1 | `BeeHurtByOtherGoal().setAlertOthers()`: the alerted bees need line of sight to the attacker. Continues only while angry |
| target 2 | `BeeBecomeAngryTargetGoal`: Player, interval 10, must see, predicate `isAngryAt`, only while angry and not stung |
| target 3 | `ResetUniversalAngerTargetGoal`: a never-goal, as for wolves |

- **Locate hive:**
  - Can use: cooldown 0, no hive, wants to enter.
  - Start: cooldown 200. Then the `#bee_home` POIs within 20 (any occupancy) whose hive is not full, sorted by `distSqr`.
  - It takes the first that is not blacklisted. When all are blacklisted, it clears the list and takes the first.
- **Go to hive** (MOVE):
  - Can use: a hive under 48 blocks away in `#beehives`, no home restriction, wants to enter, not yet reached.
  - Over 2400 travelling ticks: drop and blacklist (at most 3). A path in progress makes it wait.
  - Within 16 blocks: a direct path, node multiplier 10, accuracy 1 within 3 blocks, else 2. A failure drops and blacklists; the same path for over 60 ticks drops the hive.
  - Beyond 16: drop at 48 or more, else `pathfindRandomlyTowards`.
  - `pathfindRandomlyTowards`: y offset ±4 when more than 2 above or below; size 6/8, or half the Manhattan distance when that is under 15; `AirRandomPos.getPosTowards(…, π/10)`, multiplier 0.5, speed 1.
- **Enter hive:** the hive centre within 2.0 and the hive present. A full hive clears `hivePos`; otherwise start = `addOccupant`.
- **Pollinate:**
  - Can use: no cooldown, no nectar, not raining. It takes the first flower within Manhattan distance 5. `attractsBees` means `#bee_attractive`, not waterlogged, and only a sunflower's upper half.
  - The flower must be reachable (`createPath(pos, 1)`), with a 600-tick unreachable cache. It saves the flower and moves to it at 1.2; otherwise the cooldown is 20-60.
  - Tick:
    - Hovers at the flower's bottom centre + 0.6 through the move control at 0.35.
    - On arrival (within 0.1), a 1/25 chance of a new offset within ±0.333, and it looks at the flower.
    - `entity.bee.pollinate` with a 5% chance, at least 60 ticks after the last one.
    - Over 600 ticks: drop the flower, cooldown 200.
  - Past 400 successful ticks it continues with only a 20% chance per tick.
  - Stop: past 400, nectar; cooldown 200.
- **Go to known flower:** after more than 600 no-nectar ticks, when not within 2 blocks. Gives up after 2400 ticks or beyond 48 blocks; moves with `pathfindRandomlyTowards`.
- **Validate hive and flower:** every 20-40 ticks, they drop an invalid hive or flower that is loaded.
- **Grow crop:**
  - Can use: crops grown < 10, a 70% roll, nectar, a valid hive.
  - Tick, with `nextInt(adjustedTickDelay(30)) == 0`, for the 1 and 2 blocks below in `#bee_growables`:
    - a crop below max age, a stem below age 7 and a sweet berry bush below age 3 grow +1;
    - cave vines get bone meal (MOB).
  - On a change: level event 2011 (data 15), the block set, and crops +1.
- **Wander:**
  - Can use: its navigation is done and `nextInt(10) == 0`.
  - It heads for the hive when it is valid and farther than 48 - (24 with a hive or a flower, else 16) blocks; otherwise along the view vector.
  - Hover position, else the air-and-water position. It moves there with `createPath(pos, 1)` at 1.0.

**`BeehiveBlockEntity`.**
- **Format:** `bees: [{entity_data, ticks_in_hive, min_ticks_in_hive}]` plus `flower_pos`. At most 3.
- **`addOccupant`:**
  - Refuses at 3. Otherwise the bee stops riding and drops its lead.
  - The bee's tag is stored without `IGNORED_BEE_TAGS`. javap the 25 keys; they include Pos, Motion, Rotation, UUID, NoGravity, hive_pos, CannotEnterHiveTicks, TicksSincePollination, CropsGrownSincePollination, leash and Passengers.
  - `min_ticks_in_hive` is 2400 with nectar, 600 without.
  - The flower is copied when the hive has none, else on `nextBoolean`.
  - `block.beehive.enter`, game event `block_change`, the bee discarded, `setChanged`. If there is fire nearby, EMERGENCY.
- **`serverTick`:**
  - Every occupant gets `ticksInHive++`. Past its minimum it is released: HONEY_DELIVERED with nectar, else BEE_RELEASED.
  - `block.beehive.work` with a 0.005 chance per tick while occupied.
- **`releaseOccupant`:**
  - Refused for a release other than EMERGENCY while the stay-in attribute holds, or when the front block (FACING) has a collision shape.
  - The bee comes back from its tag with `NoGravity` and `setHivePos`.
  - Age moves toward 0 by `ticksInHive` unless `AgeLocked`, and in-love time drops by it.
  - With 90%, the hive's flower if the bee has none.
  - HONEY_DELIVERED: the nectar is dropped and `honey_level` +1 (+2 with a 1% chance), capped at 5.
  - Position: centre + (0.55 + width/2) × facing (0 when blocked); y = pos.y + 0.5 - height/2.
  - `block.beehive.exit` and `block_change`.

**`BeehiveBlock`.**
- Properties: `honey_level` 0-5, `facing`. The comparator reads the honey level.
- **`useItemOn` at level 5:**
  - Shears: loot `harvest/beehive` (3 honeycomb) popped at the hive, `block.beehive.shear` at the player, 1 durability, game event `shear`.
  - Glass bottle: a honey bottle to the hand, else the inventory, else dropped; `item.bottle.fill`; game event `fluid_pickup`.
  - Then, unless smoky (`CampfireBlock.isSmokeyPos`: a lit campfire up to 5 below, smoke stopped by a block crossing the centre post): with bees inside, `angerNearbyBees`, then an EMERGENCY release for the player. Else only the honey is reset.
- **`emptyAllLivingFromHive(player)`:** released bees within 4 blocks (`distSqr` ≤ 16) of the player target them; if sedated, they stay out for 400 ticks instead.
- **`angerNearbyBees`:** in the block's box inflated by (8, 6, 8), every bee without a target takes a random player found in that box.
- **`playerDestroy`**, unless the tool is in `#prevents_bee_spawns_when_mining` (Silk Touch): an EMERGENCY release for the player, then `angerNearbyBees`.
- **Explosions:** `onExplosionHit` calls `angerNearbyBees`. `getDrops` from a primed TNT, creeper, wither skull, wither or TNT minecart does an EMERGENCY release.
- **`updateShape`:** a fire neighbour causes an EMERGENCY release.

**Client.**
- **Renderer:** `BeeRenderer` (an `AgeableMobRenderer`, shadow 0.4). `AdultBeeModel` and `BabyBeeModel`.
  - Textures: `entity/bee/bee`, `bee_angry`, `bee_nectar`, `bee_angry_nectar`, each with a `_baby` twin.
  - State: the lerped roll, `hasStinger` = !stung, `isOnGround` = onGround and Δ² < 1e-7, angry, nectar.
- **Adult model** (64×64):
  - `bone` at (0, 19, 0) holds `body` 7×7×10 at (-3.5, -4, -5);
  - the body's `stinger` 0×1×2 (uv 26,7) and the antennae 1×2×3 at (0, -2, -5);
  - the wings 9×0×6 with deformation 0.001, uv 0,18, rotated y ∓0.2618, the left one mirrored;
  - the leg strips 7×2×0 at uv 26,1, 26,3 and 26,5.
- **Baby model:** 32×32, with different offsets. javap it during the work.
- **`setupAnim`:**
  - The stinger shows only while `hasStinger`.
  - In flight: wing zRot = cos(age × 120.32113°) × π × 0.15, mirrored; legs π/4.
  - When not angry: bob b = cos(age × 0.18).
    - bone xRot = 0.1 + b × π × 0.025, and bone y -= b × 0.9.
    - The front leg is -b × π × 0.1 + π/8, the back leg -b × π × 0.05 + π/4, and the adult's antennae b × π × 0.03.
  - Roll: bone xRot = `rotLerpRad(roll, xRot, 3.0915928)`.
- **Sounds:** the looped `entity.bee.loop` and `loop_aggressive` (NEUTRAL) switch with anger.
  - At horizontal speed ≥ 0.01: pitch = lerp(clamp(speed)) in 0.7-1.1 (baby 1.1-1.5), and volume = lerp(clamp(speed, 0, 0.5), 0, 1.2).
  - Otherwise both are 0.
- **No keyframes:** there is no `BeeAnimation`, so no `keyframes::shared()`.

### Plan

**Files (D0-3).**
- **`entities/src/world/families/land/bees.rs`:**
  - `pub struct Bee` holds:
    - the hive and flower positions; the nectar, stung and roll flags; the roll and its previous value;
    - the counters (no-nectar ticks, stay-out, crops, `time_since_sting`, under-water ticks, the two cooldowns);
    - `anger_end_time` and the anger target UUID; `Age`, in-love time and love cause;
    - the goal state, plus `enter_hive: Option<Pos>` and `grown: Vec<Pos>` for the terrain hooks.
  - `impl Family`.
  - `land/bees/goals.rs`: the goals on `MonsterGoalContext` and `GoalExtra`. It reuses `MeleeAttackGoal`, `FloatGoal`, `HurtByTargetGoal`, `NearestTargetGoal`, `breed.rs`, `tempt.rs` and `follow_parent.rs`.
  - `land/bees/hive.rs`: the POI search, validity and fire checks.
- **The `minecraft:bee` row** in `world/src/natural_spawner/families/land.rs`: 0.55 × 0.5, `NoRestrictions`, `MotionBlockingNoLeaves`, `Rules::Family { creature: true }`, `Creature`, follow range 16, `baby_chance: Some(0.05)`, no rule, no finalize.
- **New `world/src/level/beehive.rs`:**
  - occupants in the vanilla tag, `beehive_tick`;
  - `release_occupant`: the rebuilt tag with `Pos` goes onto `level.spawned`, so `take_spawned` (`server.rs:4229`) loads it through `spawn_saved` with a fresh UUID;
  - the honey level, `bees_stay_in_hive()`, `beehive_occupants(pos)`, `is_smoky(pos)`, `harvest_hive`;
  - `empty_hive(pos, cause: Option<(uuid, feet)>, emergency)`: bees aimed at a player get `angry_at` = the player's UUID; sedated bees get `CannotEnterHiveTicks` 400;
  - `beehive_exploded`, the fire handling and `take_hive_alarms()`.
- **Terrain:**
  - `crates/minecraft_terrain/src/families/land/bees.rs`: `impl TerrainFamily for Bee`.
  - `bees/render.rs`: both models and `setupAnim` on `posed_cube`.
  - `bees/hives.rs`: the server hooks.
  - `bees/hive_use.rs`: shears and bottle prediction, returning `bucket::ItemUse`.

**Registration lines.**
- `entities/.../families/land.rs`: `pub mod bees;` and `bees: bees::Bee,`.
- Terrain `families/land.rs`: `pub(crate) mod bees;` (the server and actions hooks reach it) and `Bee` in `terrain_group!`.
- The world row, and row 44 in `docs/round10/mobs.md`.

**Registry hooks (D0-2.2).**
- `KEY` 57, `KINDS` `["minecraft:bee"]`, `category` "creature".
- `hurt`: stops pollinating, then the default.
- `spared_blocks`: `["minecraft:sweet_berry_bush"]` (M1-4's hook; vanilla's berry bush spares bees as it spares foxes).
- `experience`, `despawn` (the creature tuple; `Bee` has no `removeWhenFarAway`), `undead` and `fire_immune` keep their defaults.
- `leashable`: true. `leash_walk`: the flying navigation to the holder at 1.0.
- `voice`: `("bee", 0.4, neutral)`. The tick drops the base tick's `Voice::Ambient`: bees have no ambient sound (the track's ambient rule, M10).
- Steps are silent. If the helpers always play the block step when `step_sound` is `None`, use M2's `Family::quiet_steps(kind) -> bool` (its `water-hooks` group, with the zero-volume guard at `emissions.rs:64`); if that has not merged, add `quiet_steps` under that name with the same guard in its own root commit, "Registered Minecraft mobs can walk without step sounds", and the second track to merge keeps M2's.
- `monster`: a `MonsterSpec` with the 14 goal names and 3 target names, eye 0.3, follow 16, and a `walk` that sets the maluses, no float, and d0b's flying navigation with `FlyingMoveControl(20, true)`. If d0b's navigation does not plug into `MonsterGoalContext`, keep it in `GoalExtra` and run it after the goal step, the same choice as the parrots.
- Stings use `family_strike` with `PlayerHitKind::Typed { damage_type: "minecraft:sting", message: "sting", effect: Some((Poison, 200 or 360, 0)), .. }`. On mobs, the poison goes through `mob_effects_mut` (`world/witches.rs:478`).
- Love pushes `(id, 18)` onto `world.entity_events` (`world.rs:2623`), and E1's default row draws the hearts.

**One-line hooks in shared files** (the logic stays in the bee files).
- **`player/src/lib.rs` `World`** (next to `block_in_tag`, `:238`): `beehive_occupants(&self, pos) -> Option<usize>` and `gameplay_flag(&self, name, pos) -> bool`, both defaulted; rain is M1-3's `World::raining` (H4). `MobWorld` (`server_mobs.rs:262`) implements them through public functions in `beehive.rs`.
- **`level/mod.rs`:** `mod beehive;` right after M1-8's `pub mod turtle_egg;` (or after M1-7's `pub mod sniffer_egg;` if m1g has not run yet, since m1g may come last when M2's P2 is late; M1-8 then puts its line between the two), and `pub hive_alarms: Vec<BlockPos>` right after `bell_rings` (`:208`, its init after `:304`), not after `level_events` (`:207`), where M3's `lightning_hits` goes.
- **`level/container.rs:475` `has_ticker`:** `|| self.is_beehive(state)`.
- **`level/piston.rs:625`:** `else if self.is_beehive(state) { self.beehive_tick(pos); }`, after the daylight detector.
- **`level/redstone.rs:506`:** the hive and nest honey level, next to the composter.
- **`level/explosion.rs:409`:** drop `"BeehiveBlock"` from the unsupported list and call `self.beehive_exploded(state, pos, ex)`.
- **The hive's `updateShape`** (fire neighbour): next to a related neighbour-shape arm; find it during the work.
- **On load:** if no ticker is registered for loaded or generated block entities, register every `has_ticker` block entity in `insert_chunk` (`mod.rs:318`). This is vanilla `registerAllBlockEntitiesAfterLevelLoad`; make it its own commit, because it also fixes loaded hoppers.
- **`walk_path.rs` `WalkTarget::Air`** (air 10, else 0) right after `Monster` (`:51`), its arm right after the `Monster` arm of `walk_target_value` (`stroll.rs:20`). M2's `Guardian` goes beside `Strider` and M1-8's `Turtle` right after `Animal`.
- **`stroll.rs`:** `air_random_position_towards` (`AirRandomPos.getPosTowards`, with sizes, y offset and target) right before `hover_random_position` (`:302`), not after `:315`, where M3's allays add `air_and_water_random_position_within`.
- **`server.rs`:**
  - Next to `set_cat_gift_chance` (`:1065`): `bees::hives::level_to_bees(&mut self.level, &mut self.mobs, players)`. It drains the hive alarms and angers the bees around each one, with a random player found in the (8, 6, 8) box.
  - After M1's earlier after-tick lines (M1-3's `drop_family_gifts` and M1-8's egg pass, which follow `drop_cat_gifts`, `:1107`; right after `drop_family_gifts` if M1-8 has not landed, and M1-8 then inserts its pass before this line): `bees::hives::bees_to_level(&mut self.mobs, &mut self.level, &mut self.mob_tags)`.
    - For each `enter_hive`, the tag from `mob_tags_where(.., |id, _| id == bee, ..)` (`server_mobs.rs:1497`) becomes an occupant.
    - The enter sound goes to `level.sounds` (`mod.rs:147`).
    - Then `core.removed`, and the `mob_tags` entry is dropped.
    - Level event 2011 for each `grown` position.
  - `ItemOnBlock::HarvestHive { shears: bool }` next to `FillBucket` (`:33`), and its arm in `item_on_block_by` (`:2489`). M2 adds `Command::EmptyMobBucket` beside `ItemOnBlock`'s command (`:3408`) and `ServerSim::empty_mob_bucket` beside `item_on_block_by` (`:2485`); keep the hive arm inside the function body, away from M2's new function.
  - `player_mines` (`:2276`): `bees::hives::hive_broken(.., pos, old, how, player)` next to `ice_player_destroyed`, after D0's infested-block call.
- **`level_events.rs:128`:** a 2011 arm (bee growth: happy-villager sparkles, no sound) beside `BONE_MEAL_USE`, after M1-7's 3009 arm.
- **`leash_render.rs:76`:** a new line `"bee" => (0.5, 0.2),` after the `"ocelot"` row (vanilla `getLeashOffset` (0, 0.5 × eye, 0.2 × width)).
- **`survival/src/actions.rs:1100`:** `.or_else(|| …bees::hive_use::hive_use(&player, &outlined, stack))` next to `bone_meal_use`. Make sure `bottle_use` (`:998`) does not take a glass bottle aimed at a hive first.

**Multiplayer.**
- `FamilyShow.flags`: bit 0 nectar, bit 1 stung, bit 2 angry, bit 3 resting. `values[0]` = roll × 10000, `values[1]` = previous roll × 10000, lerped by the renderer. The wings and the bob come from each viewer's `ClientMob` age.
- `VOLATILE`: `HasNectar`, `HasStung`, `TicksSincePollination`, `CannotEnterHiveTicks`, `CropsGrownSincePollination`, `hive_pos`, `flower_pos`, `angry_at`. `anger_end_time`, `Age` and `InLove` are already global.
- `TRACKING_RANGE` 8, `SHADOW_RADIUS` 0.4, `TEXTURES` the 8 bee textures, default head turn and cage margin.
- Hives, honey and their sounds reach friends as block updates and through `level.sounds`. Every player path carries the player's id; nothing keys on player 0.
- **Protocol:** nothing beyond D0's `FamilyShow`. `ItemOnBlock` stays on the host.

**COMMIT GROUPS**

| # | Group | Commits | Needs |
|---|---|---|---|
| 1 | `bees-live` | 1. "Minecraft bees fly, wander and are saved like vanilla" (the family, flight, wander, float, look, roll, drowning, no fall damage, silent steps, the lead, saving, the row, `WalkTarget::Air`, `TerrainFamily` with an empty `draw`; row 44 becomes `live`). 2. "Minecraft bees sting with poison and die after stinging" (anger, attack, alerts, `Typed` sting, the death clock). 3. "Minecraft bees breed and follow flowers, and wither roses and eyeblossoms hurt them". | d0a, d0b |
| 2 | `bees-drawn` | 1. "Minecraft bees are drawn like vanilla" (both models, 8 textures, `FamilyShow`, cages; row 44 becomes `drawn`). 2. "Minecraft bees buzz while they fly" (decision 3). | 1 |
| 3 | `bees-hives` | 1. "Minecraft ticking block entities tick again after their chunk loads" (decision 6). 2. "Minecraft beehives and bee nests keep their bees and let them out like vanilla" (`beehive.rs`, ticker, release, honey, comparator, fire, the stay-in flag, the `World` reads). | 1 |
| 4 | `bees-homes` | "Minecraft bees find, enter and stay in their hives at night and in rain" (the hive goals in `land/bees/hive.rs` and `goals.rs`, `bees_to_level` in `bees/hives.rs` with its `server.rs` line; `level_to_bees` and the hive alarms come with `bees-honey`). Split from `bees-hives`, whose block-entity commit already touches about ten world-crate sites. | 3 |
| 5 | `bees-pollination` | 1. "Minecraft bees pollinate flowers and bring honey home". 2. "Minecraft bees grow crops on their way home". | 4 |
| 6 | `bees-honey` | 1. "Minecraft beehives give honeycomb and honey bottles" (campfire smoke included). 2. "Minecraft bees defend their hives" (harvest, breaking, explosions). 3. (optional) "Minecraft dispensers harvest beehives with shears and glass bottles" (next to `"honeycomb"`, `dispenser.rs:95`). Row 44 becomes `done`; the owner's steps go in the log. | 5, W (decision 4) |

### Tests

- **`bees-live`:**
  - `a_bee_is_small_with_ten_health_and_its_kind`
  - `a_bee_hovers_after_its_path_ends_and_takes_no_fall_damage`
  - `a_bee_wanders_back_toward_a_far_hive`
  - `a_bee_drowns_after_a_second_under_water`
  - `a_hit_bee_and_the_bees_that_see_the_attacker_turn_on_it`
  - `a_bee_stings_once_with_poison_by_difficulty_then_calms_and_dies`
  - `an_angry_bee_calms_after_twenty_to_thirty_nine_seconds`
  - `bees_breed_on_flowers_and_a_baby_follows_and_grows`
  - `a_wither_rose_withers_a_bee_and_an_open_eyeblossom_poisons_it`
  - `a_lead_pulls_a_flying_bee`
  - terrain `a_bee_saves_and_loads_with_its_hive_flower_nectar_and_anger` and `a_friend_sees_a_bee_from_its_look_tag`
  - world `a_bee_row_gives_its_size_category_and_baby_chance`
- **`bees-drawn`:**
  - `an_adult_bee_has_its_body_stinger_antennae_wings_and_legs_on_a_64_texture`
  - `a_baby_bee_uses_its_own_model_and_textures`
  - `an_angry_bee_with_nectar_picks_the_angry_nectar_texture`
  - `wings_beat_in_flight_and_rest_on_the_ground`
  - `a_rolling_bee_turns_over`
  - `a_stung_bee_has_no_stinger`
  - `a_friend_sees_a_bee_angry_with_nectar_and_rolling`
  - commit 2: `a_flying_bee_restarts_its_loop_one_shot_at_the_sample_length_within_sixteen_blocks` (angry bees use `loop_aggressive`; a resting bee is silent; volume and pitch from the speed at each start)
- **`bees-hives`:**
  - `a_hive_lets_a_bee_out_its_front_after_its_time`
  - `a_bee_with_nectar_raises_the_honey_level_once_or_twice`
  - `bees_stay_in_at_night_and_in_rain_but_leave_in_an_emergency`
  - `a_blocked_front_keeps_bees_in`
  - `fire_beside_a_hive_throws_its_bees_out`
  - `a_hive_holds_three`
  - `the_comparator_reads_the_honey_level`
  - `a_generated_nest_lets_its_bees_out`
  - `a_loaded_hive_ticks`
- **`bees-homes`:**
  - `a_bee_finds_the_nearest_hive_with_room_within_twenty`
  - `a_bee_drops_and_blacklists_a_hive_it_cannot_reach`
  - `a_bee_near_a_burning_hive_stays_out`
  - server `a_bee_enters_its_hive_and_comes_out_later_with_its_fields` (a fresh UUID, `hive_pos`, nectar dropped, age advanced)
- **`bees-pollination`:**
  - `a_bee_pollinates_a_flower_past_four_hundred_ticks_and_gains_nectar`
  - `rain_stops_pollination`
  - `a_bee_skips_a_waterlogged_or_lower_sunflower`
  - `a_bee_returns_to_its_known_flower_after_six_hundred_ticks`
  - `a_bee_with_nectar_grows_crops_below_it_at_most_ten_times`
  - `bee_growth_sparkles_without_a_sound`
- **`bees-honey`:**
  - `shears_on_a_full_hive_drop_three_honeycomb_and_empty_it`
  - `a_glass_bottle_on_a_full_hive_gives_honey`
  - `harvesting_without_smoke_angers_the_bees_and_lets_them_out_at_the_player`
  - `a_campfire_up_to_five_below_keeps_them_calm_and_out_for_400_ticks`
  - `breaking_a_hive_lets_angry_bees_out_but_silk_touch_does_not`
  - `a_blast_lets_the_bees_out_angry`
  - (optional) `a_dispenser_harvests_a_full_hive`

### Owner's steps

Survival, Normal, by day, on flat grass. Name the screenshots `bee1.png`; with two windows, `bee8-host.png` and `bee8-friend.png`.

1. Use a bee egg 4 blocks away. A fuzzy yellow-and-black bee with antennae and fast see-through wings hovers about a block up, bobbing, and never lands. Hatch 8 more. **bee1**
2. Punch one bee. Its face turns angry, it flies at you and stings once: one heart and 10 s of poison. It loses its stinger, stops chasing, and drops dead within about a minute. The bees that saw the punch join in. **bee2**
3. `/difficulty easy`: no poison. `/difficulty hard`: 18 s of poison. Then `/difficulty normal`.
4. Hold a poppy: nearby bees follow you. Use poppies on two bees: hearts (with E1), and a small round baby that follows them. Poppies make it grow faster. **bee3**
5. Use a wither rose on a bee: it is hurt for 2 s. Put a bee on a lead: it flies after you.
6. Place a beehive with 10 poppies within 5 blocks and hatch 3 bees beside it. Each bee hovers on a flower for about 20 s, comes away with pollen on its back, and flies into the hive (enter sound). It comes out the front about 2 minutes later. After a few trips the front shows honey, and a comparator gives 1 to 5. **bee4**
7. `/time set night`: the bees go in and stay. `/time set day`: they come out. `/weather rain` keeps them in; then `/weather clear`.
8. Plant wheat around the hive: bees with pollen grow it a stage, with green sparkles.
9. `/setblock ~3 ~ ~ minecraft:beehive[honey_level=5]`, hatch 3 bees next to it and `/time set night`: they move in. Use the shears on it: 3 honeycomb pop out, the honey empties, and the bees burst out angry and sting you, even at night. **bee5**
10. Lit campfire, one air block, then a `/setblock` full hive above it. Let 3 bees move in at night, then use a glass bottle on it: you get a honey bottle and the bees stay calm. Drink it while poisoned: the poison ends. **bee6**
11. Break a hive with bees inside using the plain axe: angry bees come out. With the Silk Touch axe: none come out (they are lost, see "Left for later"). TNT beside a third hive: the bees fly out angry.
12. Flint and steel on the block beside an occupied hive: the bees leave at once.
13. In a flower forest, meadow or plains, find a nest on an oak or birch: bees come out of it by day. Bone meal on oak saplings beside flowers: about 1 in 20 grows with a nest. **bee7**
14. Save and quit with bees both out and in a hive, then load: the hive bees still come out, and an angry bee is still angry.
15. Friend window: the friend sees the same bees, pollen, angry faces, roll and honey. When the friend shears a full hive, the bees chase the friend. **bee8-host, bee8-friend**
16. `/gamemode creative`, then a spawner with a bee egg: a small bee turns in the cage. Then `/gamemode survival`.

### Kit

```
IW4L_MINECRAFT_KIT=minecraft:bee_spawn_egg*16,minecraft:beehive*4,minecraft:bee_nest*2,minecraft:poppy*64,minecraft:dandelion*64,minecraft:cornflower*16,minecraft:wither_rose*4,minecraft:open_eyeblossom*4,minecraft:glass_bottle*16,minecraft:shears,minecraft:campfire*2,minecraft:flint_and_steel,minecraft:wheat_seeds*32,minecraft:bone_meal*32,minecraft:oak_sapling*8,minecraft:diamond_hoe,minecraft:iron_axe,minecraft:diamond_axe+silk_touch=1,minecraft:comparator*2,minecraft:redstone*16,minecraft:lead*4,minecraft:tnt*4,minecraft:water_bucket,minecraft:oak_planks*64,minecraft:dispenser*2,minecraft:spawner,minecraft:cooked_beef*32
```

27 stacks; every id was checked against the 26.3 `assets/minecraft/items`. The friend needs the same kit.

### Depends on

- **D0 d0a:** the registry, rows, `TerrainFamily`, `FamilyShow`, `interact`, the lead, `MonsterSpec`, `GoalExtra` and `Typed`.
- **D0 d0b:** P3 flight, for `bees-live`.
- **W:**
  - the game events `block_change`, `shear` and `fluid_pickup`: one line each, added by whichever merges second;
  - the `player` id on `PlayerMines` (decision 4).
- **E1:** hearts from its default row 18.
- **E2 `ti1`:** thrown things hit bees.
- **M1-9 parrots:** the d0b wiring they landed. **M1-4 foxes:** `spared_blocks`. **M1-3 pandas:** H4 (`World::raining`).
- **M3:** may reuse `World::gameplay_flag` for creakings.

### Left for later

- Honey and nectar drips (`DripParticle`). The fork has no drip particles at all.
- Stingers drawn in players (`BeeStingerLayer`): players are MW2 models.
- Keeping bees in a silk-touched nest. `copy_components`/`copy_state` are no-ops, and placing an item with `minecraft:bees` would have to refill its block entity: shared loot and placement work.
- The creative hive drop, the `Bees` tooltip, debug renderers and advancements.
- A true moving loop sound, if decision 3 approximates it.

### Open decisions (recommended choice first)

1. **Hive reads.** Two defaulted `World` methods implemented by `MobWorld`, plus M1-3's `World::raining`. Fire nearby comes from `World::block`. `EntityWorld` setters would need fields in `world.rs`.
2. **Entering and leaving.** The AI sets `enter_hive`, and the terrain hook stores the tag and removes the bee in the same tick. Releases use `level.spawned`. That way no `Tag` reaches the entities crate.
3. **The buzz.** The fork has no moving looping sounds. Restart `entity.bee.loop`/`loop_aggressive` one-shots per bee within 16 blocks at the sample length, with vanilla's speed-based volume and pitch sampled at each start, in `client_effects`. Record it in `decisions.md` as an approximation, in its own commit. The alternative, silence, loses the main way players find bees.
4. **The breaking player.** Use W's `player` on `PlayerMines`. Before W's `vibrations` group lands, use the nearest survival or adventure player within 4 blocks.
5. **MW2 blasts on hives.** Release the bees as for TNT. Vanilla releases only for TNT, creepers, wither skulls, withers and TNT minecarts.
6. **Ticker registration on load.** Register every ticking block entity, as vanilla does, not only hives, in its own commit.

---

## M1-groups. Commit groups (track runner)

Every group updates `docs/round10/track-m1-log.md` and its rows in the M1 section of `docs/round10/mobs.md` in the same commit as the code, and writes its owner steps and kit line into the log.

| # | Group key | Commits | Section |
|---|---|---|---|
| 1 | `rabbits-live` | 1. Registered Minecraft mobs can run their own AI step. 2. Minecraft rabbits hop about, flee and follow carrots in the entity world. 3. Minecraft rabbits breed and their babies grow up. | M1-1 |
| 2 | `rabbits-drawn` | 1. Minecraft rabbits are saved, hatch from eggs and are drawn like vanilla. 2. Minecraft rabbits hop and tilt their heads with the vanilla animations, for friends too. | M1-1 |
| 3 | `rabbits-behaviour` | 1. Minecraft rabbits raid carrot crops. 2. The killer bunny bites players and wolves. 3. Minecraft cats hunt rabbits. | M1-1 |
| 4 | `polar-bears-live` | 1. Registered Minecraft mobs can land their melee attacks their own way. 2. Minecraft polar bears live, defend their cubs and stand up to strike. 3. Minecraft polar bears spawn in snowy biomes and on sea ice. | M1-2 |
| 5 | `polar-bears-drawn` | Minecraft polar bears are drawn like vanilla. | M1-2 |
| 6 | `pandas-live` | 1. Minecraft pandas live in the entity world with their genes. 2. Minecraft pandas spawn in bamboo jungles and are saved, hatched and seen by friends. | M1-3 |
| 7 | `pandas-drawn` | Minecraft pandas are drawn like vanilla. | M1-3 |
| 8 | `pandas-eating` | 1. Registered Minecraft mobs can hold still while their goals run. 2. Registered Minecraft mobs can scatter item crumbs. 3. Minecraft pandas sit down to eat bamboo and cake. | M1-3 |
| 9 | `pandas-moods` | 1. Living Minecraft mobs can drop gifts from loot tables. 2. Minecraft panda cubs sneeze, lazy pandas lie on their backs and playful pandas roll. | M1-3 |
| 10 | `pandas-thunder` | 1. Minecraft mobs know when it rains and thunders. 2. Minecraft worried pandas hide from thunder. | M1-3 |
| 11 | `foxes-live` | 1. Registered Minecraft mobs can be spared by the blocks they walk through. 2. Minecraft foxes live in the entity world. 3. Minecraft foxes are saved, hatched and seen by friends. | M1-4 |
| 12 | `foxes-drawn` | 1. Minecraft foxes are drawn like vanilla. 2. Minecraft foxes show what they carry in their mouths. | M1-4 |
| 13 | `foxes-hunt` | 1. Mobs can show a block's break effect without breaking it. 2. Minecraft foxes stalk, pounce on and bite chickens and rabbits. | M1-4 |
| 14 | `foxes-items` | 1. Minecraft foxes pick up, spit out and eat items. 2. Minecraft foxes pick sweet berries and glow berries. | M1-4 |
| 15 | `foxes-trust` | 1. Minecraft foxes breed on berries and their cubs trust the players who fed them. 2. Minecraft foxes defend the players they trust. | M1-4 |
| 16 | `goats-live` | 1. Minecraft animals share vanilla's brain behaviours for panic, temptation, love and following adults. 2. Minecraft goats live, breed and wander like vanilla. 3. Minecraft goats give milk and eat wheat. | M1-5 |
| 17 | `goats-drawn` | Minecraft goats are drawn like vanilla. | M1-5 |
| 18 | `goats-ram` | Minecraft goats ram players and mobs and snap their horns on stone and logs. | M1-5 |
| 19 | `goats-leap` | 1. Minecraft mobs can long-jump like vanilla goats and frogs. 2. Minecraft goats leap up to five blocks and shrug off short falls. | M1-5 |
| 20 | `goat-horns` (optional) | Minecraft goat horns play their eight calls. | M1-5 |
| 21 | `armadillos-live` | 1. Minecraft mobs can tell when a player sprints. 2. Minecraft armadillos live, curl up when scared and peek like vanilla. | M1-6 |
| 22 | `armadillos-drawn` | 1. Registered Minecraft mobs cast their own shadow size. 2. Minecraft armadillos are drawn like vanilla. | M1-6 |
| 23 | `armadillos-scutes` | Minecraft armadillos shed scutes and give them up to a brush. | M1-6 |
| 24 | `armadillos-spiders` | 1. Mobs that avoid others ignore registered mobs that are hiding. 2. Minecraft spiders keep away from armadillos that are not curled up. | M1-6 |
| 25 | `sniffers-live` | Minecraft sniffers live, wander and lay eggs like vanilla. | M1-7 |
| 26 | `sniffers-drawn` | Minecraft sniffers and snifflets are drawn like vanilla. | M1-7 |
| 27 | `sniffers-dig` | 1. Registered Minecraft mobs throw up block dust (unless W6 landed one). 2. Minecraft sniffers sniff out and dig up ancient seeds. | M1-7 |
| 28 | `sniffer-eggs` | Minecraft sniffer eggs crack and hatch snifflets, faster on moss. | M1-7 |
| 29 | `turtles-water` (optional) | Only if M2's P2 (`water-paths`, `water-swimming`) is not in `minecraft-survival` when m1g starts: the amphibious paths and `Body::travel_swimming` under M2's names and subjects, one commit each (M1-8, decision 1). Otherwise no commit; the log says the group was skipped. | M1-8 |
| 30 | `turtles-live` | 1. Minecraft loot tables know what killed a mob. 2. Minecraft turtles swim, walk the beach and find their way home in the entity world. 3. Minecraft turtles mate for an egg and their babies grow up into scutes. | M1-8 |
| 31 | `turtles-drawn` | 1. Minecraft turtles are saved, hatch from spawn eggs and eat seagrass. 2. Minecraft turtles are drawn like vanilla, for friends too. | M1-8 |
| 32 | `turtles-eggs` | 1. Minecraft turtles lay eggs on their home beach. 2. Minecraft turtle eggs crack and hatch at dawn. 3. Minecraft turtle eggs break under careless feet. | M1-8 |
| 33 | `turtles-hunted` | 1. Minecraft mob targets can pick babies on land. 2. Minecraft zombies, skeletons, wolves, cats, ocelots and foxes hunt baby turtles. 3. Minecraft zombies trample turtle eggs. | M1-8 |
| 34 | `parrots-live` | 1. Minecraft parrots fly, wander to trees and imitate monsters like vanilla. 2. Minecraft parrots spawn in jungles in five colours and are saved, hatched and seen by friends. | M1-9 |
| 35 | `parrots-drawn` | Minecraft parrots are drawn like vanilla. | M1-9 |
| 36 | `parrots-tame` | Minecraft parrots are tamed with seeds, sit when told and die of cookies. | M1-9 |
| 37 | `parrots-shoulders` | 1. Minecraft parrots ride on their owners' shoulders and fly off when they jump. 2. Minecraft players keep their shoulder parrots in their player files. | M1-9 |
| 38 | `parrots-shoulders-seen` | Friends see parrots on each other's shoulders. | M1-9 |
| 39 | `bees-live` | 1. Minecraft bees fly, wander and are saved like vanilla. 2. Minecraft bees sting with poison and die after stinging. 3. Minecraft bees breed and follow flowers, and wither roses and eyeblossoms hurt them. | M1-10 |
| 40 | `bees-drawn` | 1. Minecraft bees are drawn like vanilla. 2. Minecraft bees buzz while they fly. | M1-10 |
| 41 | `bees-hives` | 1. Minecraft ticking block entities tick again after their chunk loads. 2. Minecraft beehives and bee nests keep their bees and let them out like vanilla. | M1-10 |
| 42 | `bees-homes` | Minecraft bees find, enter and stay in their hives at night and in rain. | M1-10 |
| 43 | `bees-pollination` | 1. Minecraft bees pollinate flowers and bring honey home. 2. Minecraft bees grow crops on their way home. | M1-10 |
| 44 | `bees-honey` | 1. Minecraft beehives give honeycomb and honey bottles. 2. Minecraft bees defend their hives. 3. (optional) Minecraft dispensers harvest beehives with shears and glass bottles. | M1-10 |

A root hook a section marks "only if missing" (the parrot's "pushes players" trait, the bees' `quiet_steps`, the ambient interval field) goes first in its group as its own commit when it is needed.

### Segments

Each segment ends at a family boundary, so a release never holds a family that lives but is not drawn.

| Segment | Groups | Waits for | Release gets |
|---|---|---|---|
| m1a | `rabbits-live`, `rabbits-drawn`, `rabbits-behaviour`, `polar-bears-live`, `polar-bears-drawn` | d0a | rabbits, polar bears |
| m1b | `pandas-live`, `pandas-drawn`, `pandas-eating`, `pandas-moods`, `pandas-thunder` | m1a | pandas |
| m1c | `foxes-live`, `foxes-drawn`, `foxes-hunt`, `foxes-items`, `foxes-trust` | m1b; E1 e1a soft (fall helper) | foxes |
| m1d | `goats-live`, `goats-drawn`, `goats-ram`, `goats-leap`, `goat-horns` | m1c | goats |
| m1e | `armadillos-live`, `armadillos-drawn`, `armadillos-scutes`, `armadillos-spiders` | m1d | armadillos |
| m1f | `sniffers-live`, `sniffers-drawn`, `sniffers-dig`, `sniffer-eggs` | m1e; W w2 soft (block dust) | sniffers |
| m1g | `turtles-water` (optional), `turtles-live`, `turtles-drawn`, `turtles-eggs`, `turtles-hunted` | m1f (M1-3's H3, M1-4's fox slot, M1-6's shadow hook); M2 P2 (`water-paths`, `water-swimming`), else `turtles-water` builds it; E2 e2a soft | turtles |
| m1h | `parrots-live`, `parrots-drawn`, `parrots-tame`, `parrots-shoulders`, `parrots-shoulders-seen` | m1f (or m1g); **D0 d0b**; E1 e1b soft | parrots |
| m1i | `bees-live`, `bees-drawn`, `bees-hives`, `bees-homes`, `bees-pollination`, `bees-honey` | m1h (and M1-3's H4, M1-4's `spared_blocks`); **D0 d0b**; W w1 soft | bees |

Before each segment the orchestrator merges `minecraft-survival` into `track-m1` and tags the result as the segment's base (`r10-m1a-base`, ...). m1g and m1h may swap: m1h needs d0b, m1g needs M2's P2, and neither needs the other. m1i always comes after m1h (it reuses the parrots' flying wiring).

---

## Left for later

Collected from the sections; the track log keeps them as they stand when each group ends.

**Shared, more than one family**
- Spawn eggs used on a mob to make a baby (fox, armadillo, polar bear): the fork has no egg-on-mob offspring for any mob.
- Eggs read `CanPickUpLoot` as false (`spawning.rs:896` serves eggs and `/summon` alike), so egg pandas and foxes do not pick up items; vanilla eggs keep the constructor's true. Fork-wide (villagers and piglins too); the steps use `{CanPickUpLoot:1b}`.
- D0's `monster_ai_step` adds `Monster`'s +2 idle in light for every goal family, creatures included: D0's fix to make.
- Walking on powder snow: a shared collision flag for the rabbit, fox, silverfish and endermite.
- Looting and Fire Aspect never reach `EntityLootContext` in the fork.
- Dispensers filling a mob's hand or paw (fox, panda).
- The item-pickup flight animation (fox, panda).
- Advancements and stats (parrots, bees, turtles).
- The game events of every family, where W had not merged when the group ran: one line each, added by whichever of W and M1 merges second.

**Rabbits (M1-1)**
- The hop's dust (event 1, `spawnSprintParticle`): one call once M1-7's `ClientFx.dust` is in.
- The nibble's level event 2001: one line with M1-4's `World::destroy_effect`.
- "The Killer Bunny" shown over the mob or in death lines (the fork draws no mob names, `death.rs:689` names the type); name tags (Toast only through `/summon`) and the egg's `minecraft:rabbit/variant` component.
- `AgeLocked` for the older stores (`server_mobs.rs:1536`).
- `ClimbOnTopOfPowderSnowGoal`, if D0 did not port it.

**Polar bears (M1-2)**
- `mustReach` (ported nowhere, `monster_ai.rs:2909`); the 0.98 water slow-down (`movement.rs:270`); the standing bear's taller client box; freeze immunity (nothing freezes yet).

**Pandas (M1-3)**
- (Only the shared items above.)

**Foxes (M1-4)**
- Eating effects (golden apples, the chorus teleport, stew bowls): the fork has no mob consume effects.
- Breeding experience orbs (the fork draws but spawns none, as for wolves).

**Goats (M1-5)**
- A rammed mob still blames the goat: `mob_hits_mob` (`world.rs:4179`) takes no damage type.
- Goats in boats; the held horn's tooting pose on the MW2 body; the nearest-items sensor; rams on armour stands (the fork has none).

**Armadillos (M1-6)**
- Wolf armour on wolves (no body slot, cracking or repair; crafting it works); brushing blocks (`BrushItem.use`).

**Sniffers (M1-7)**
- Brushing suspicious sand and gravel, the only natural source of sniffer eggs; the four-rope leash (`supportQuadLeash`, happy ghasts need it too); a digging sound that follows the sniffer and cuts off.

**Turtles (M1-8)**
- Lightning (`thunderHit`, the bowl): M3's P8 adds `Family::thunder_hit`; the turtle's override goes in once both have merged.
- Turtle shell Water Breathing (no `EffectKind`; M2 edits that enum); stacking eggs by hand; landing on eggs (`fallOn`); particles 2012 and 2001 and the zombie's egg crumbs and poofs; the drowned's turtle hunting and egg goal (`world.rs:1645-1659`); Impaling against `#aquatic`.

**Parrots (M1-9)**
- Dancing to jukeboxes (the fork plays no discs); the owner's death message for a tame parrot; the 3D walk animation (`client_mobs.rs:687`); the flap while riding a boat or minecart; silent mobs still imitated; vanilla's crouch shift on the shoulder.

**Bees (M1-10)**
- Honey and nectar drips (no drip particles in the fork); stingers drawn in players; bees kept in a silk-touched nest (`copy_components`/`copy_state` are no-ops); the creative hive drop, the `Bees` tooltip and debug renderers; a true moving loop sound, if the buzz is approximated.
