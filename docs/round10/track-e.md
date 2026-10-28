# Round 10, track E: the known limits of release 2

Track E fixes the known limits at the end of `docs/round9/release-2-tests.md`. It runs as **two parallel tracks**:

- **E1, screens, visuals and sounds:** particles, banner patterns and the crossbow pull, swinging bells, the trading screen and the effect list, horses.
- **E2, gameplay and loot:** doors, thrown items, leads and cat chests, explorer maps, fireworks, projectiles ringing bells.

All file:line references are to the merged code in `C:\Dev\mc-wt\track-d` (detached at 6e0e0573). Check them again before editing: D0 may have moved them. Vanilla facts were checked with javap on the 26.3 `client.jar`, and data facts against the 26.3 packs.

## Overview

### The owner's list

The owner asked for these (round 10 request, track E):

1. The missing particles: taming hearts, bucking smoke, trust hearts, raid sweat, llama spit.
2. Banner patterns, and the crossbow pull with its loaded arrow.
3. Vindicators and zombies breaking and opening doors.
4. Swinging bells.
5. Fireworks bursting on mobs, with knockback from the burst.
6. Thrown items hitting the new mobs.
7. The trading screen's keys and drags, and effects listed beside the inventory.
8. Horse fall damage, the gallop sound, vehicle hearts and dyed leather horse armour.
9. Punching off lead knots, and cats keeping chests shut.
10. Phantoms. **Not in track E:** phantoms are a new mob family and go to track M3 (`docs/round10/decisions.md`).
11. Shipwreck and ocean ruin loot with explorer maps.

### The split

| Track | Sections, in build order | Why here |
|---|---|---|
| **E1** | E-kit (component parser), E-particles, E-trading, E-effects, E-omen, E-horses, E-banners, E-bells (swing and outline) | Client drawing, HUD and screens. Banners and bells share `special_models.rs` and `mesh.rs`. The effect list and vehicle hearts share `frame/src/minecraft_ui.rs` and `hud/src/minecraft_hotbar.rs`. Dyed horse armour and banner looks share `item_tint.rs` and `minecraft_hand.rs`. |
| **E2** | E-thrown, E-doors, E-maps (loot first), E-leads-cats, E-fireworks, E-bell-hits | Server gameplay. Thrown items, fireworks and bell hits share `thrown.rs`, `tick_thrown`, `tick_tridents` and `tick_rockets`. Doors, thrown items and fireworks share the vendored mob world. |

Three pieces moved from where the cluster research put them, so that the two tracks never edit the same function:

- **The kit and `/give` component parser for fireworks** moved from E2 (fireworks) into E1's first group. One commit in E1 adds all five components (`banner_patterns`, `base_color`, `charged_projectiles`, `fireworks`, `firework_explosion`) to `args.rs` and `kit_components`. E2's code tests build their stacks in code. E2's owner steps with starred rockets need E1's first group merged.
- **Projectiles ringing bells** moved from E1 (bells) to E2. It edits `tick_thrown`, `tick_tridents`, `tick_rockets` and the vendored projectile sites, which thrown items and fireworks rewrite. The swing stays in E1.
- **The shield decoration recipe** moved from E1 (banners, optional) to E2's firework crafting group. Both are new special recipes in the vendored `player/src/crafting.rs`.

### Worktrees

| Track | Worktree | Branch | Target dir | Log |
|---|---|---|---|---|
| E1 | `C:\Dev\mc-wt\track-e1` | `track-e1` | `D:\iw4l-wt\track-e1` | `docs/round10/track-e1-log.md` |
| E2 | `C:\Dev\mc-wt\track-e2` | `track-e2` | `D:\iw4l-wt\track-e2` | `docs/round10/track-e2-log.md` |

(The orchestrator may put one target on `C:\iw4l-wt\` because of disk space; see `docs/round10/decisions.md`.)

### Files each track owns

**E1 only:**
- `crates/minecraft_terrain/src/mob_particles.rs` (new), `item_look.rs` (new), `banner_look.rs` (new)
- `crates/minecraft_terrain/src/ambient_particles.rs`, `special_models.rs`, `model.rs`, `lids.rs`, `item_tint.rs`, `item_icons.rs`, `interface.rs`, `illager_render.rs`, `piglin_render.rs`, `horse_render.rs`, `body_gear.rs`, `body_render.rs`, `worn_head.rs`, `item_frame.rs`, `mount_inventory.rs`, `mob_actions.rs`
- `crates/minecraft_terrain/src/mesh.rs` (atlas list, `ItemVisuals::model`, `append_open_lids`)
- `crates/minecraft_commands/src/args.rs` and `crates/minecraft_terrain/src/crafting.rs` (the kit parser and the `@dye` tag)
- `crates/minecraft_survival/src/mob_sender.rs`, `gear.rs`, `survival.rs`, `commands.rs`, `views.rs`
- `crates/hud/src/*` except the map overlay job; `crates/frame/src/minecraft_ui.rs` `McStack`; `crates/render_anim/src/minecraft_gui.rs`, `minecraft_inventory.rs`, `minecraft_hand.rs` (line 195 only; E2 adds the held-map skip at 176-178)
- `crates/render_gpu/src/drawsurf/minecraft_world.rs` and `glow_render.rs` (optional outline)
- Vendored: `entities/src/merchant.rs`, `world/merchants.rs`, `horse.rs`, `world/equines.rs`, `world/riding.rs`, `world/emissions.rs`, `world/living.rs`, `player/src/inventory.rs` (`queue_drop`), `player/src/loot.rs` (banner `copy_components`), `world/src/level/` (the block entity components command)

**E2 only:**
- `crates/minecraft_terrain/src/maps.rs` (new); vendored `entities/src/door_goals.rs` (new)
- `crates/minecraft_terrain/src/thrown.rs`, `splash.rs`, `wind.rs`, `firework.rs`, `firework_rocket.rs`, `firework_particles.rs`, `trident.rs`, `leash_render.rs`, `level_events.rs`, `locate.rs`, `containers.rs`, `bucket.rs`, `end_crystal.rs` (optional TI4)
- `crates/minecraft_survival/src/session.rs` (`level_cracks`), `death.rs`, `save.rs`, `server.rs` (`Globals`)
- `crates/minecraft_net/src/messages.rs` and `codec.rs` (`ToClient::MapData`; E1 adds `GearItem.look` and `PlayerView.vehicle_health` in other structs)
- Vendored: `core/src/loot.rs` (`exploration_map`), `entities/src/monster_ai.rs`, `illager_ai.rs`, `world/illagers.rs`, `world/combat.rs`, `world/witches.rs`, `world/snow_golems.rs`, `world/leashes.rs`, `world/felines.rs`, `feline_ai.rs`, `projectile.rs`, `world/fireballs.rs`, `world/shulker_bullets.rs`, `world/src/level/dispenser.rs`, `player/src/crafting.rs`, `player/src/lib.rs` (`World::chest_open_count`)

### Files both E tracks edit, and how to stay apart

| File | E1 edits | E2 edits |
|---|---|---|
| `crates/minecraft_terrain/src/server.rs` | `EntitySnapshot.mob_puffs`, `ServerSim.mob_puffs`, the entity events line `:1100`; `MerchantOp`/`MerchantUpdate`/`Server::merchant`; `Output.block_events` and `rider_falls`; `hear_bells`; the block entity components command; `Command::MountInventory` | rockets (`tick_rockets`, `rocket_blast`, `Command::LaunchRocket`); `tick_thrown`/`tick_tridents`; mob level events next to `infect_villagers`; `Command::Knot`; the dispensed rocket drain |
| `crates/minecraft_survival/src/actions.rs` | banner placement `:1499` and `kept` `:1656`; effect sound drain in `act()` | knot punch in the attack press `:373`; shears before `leashed_fence` `:775`; the chest check `:833`; the rocket item use `:1131`; the empty map use |
| `crates/minecraft_survival/src/entities.rs` | `apply_merchant_answer`, `rider_falls`, block events in `present_level_events`, banner block entity feeds | `punch_knot`, `mob_distance`, `cat_on_chest`, `launch_rocket`, pvp filter for rockets |
| `crates/minecraft_survival/src/entity_view.rs` | a `mob_puffs()` helper called with one line from `bursts()`, `spit_puffs`, placed banners next to the lids | the firework loop inside `bursts()` (`:321`) and the emitters (`:805`) |
| `crates/minecraft_survival/src/menus.rs` | `merchant_op`, `merchant_local` | `open_station` (map store) |
| `crates/frame/src/minecraft_ui.rs` | `McStack.effects` + `Default`; `MinecraftUi.vehicle_health` next to `jump_bar` | `MinecraftUi.held_map` next to the spyglass fields |
| `crates/render_anim/src/minecraft_world.rs` | draw-apart loop `:1518`, `vehicle_health` `:1260` | host crack list `:1550` |
| `crates/render_anim/src/minecraft_players.rs` | `struck()` rider falls, `vehicle_health` `:869` | carried map ticking |
| Vendored `entities/src/world.rs` | `broadcast_entity_event` next to `take_entity_events` `:4068`; the horse fall and gallop lines `:7748`, `:7750`, `:7933`, `:7940` | zombie door flag `:6178` and AI-step tail, `take_level_events` (next to `take_villager_infections` `:3823`), `door_cracks()`, `hit_boxes`/`arrow_targets`, `firework_blast`, `PlayerHitKind::Fireworks`, `take_projectile_block_hits` |
| Vendored `world/equines.rs` | horse fall, gallop, dye | one push line at the llama spit block hit `:532` (bell hits) |
| `docs/MINECRAFT-SURVIVAL.md` | the paragraphs of its own features | the paragraphs of its own features |

Rules for both tracks:
- **New enum variants and match arms go next to a related one, never at the end** of a shared enum (`Command`, `ToClient`, `Change`, `MerchantOp`), so two tracks never add adjacent lines. The one exception is `ambient_particles::Kind`, where the convention is to append; a conflict there is two lines and trivial.
- **No track-E commit bumps `minecraft_net::PROTOCOL`.** Several tracks change the wire this round (E1: `EntitySnapshot.mob_puffs`, `GearItem.look`, `PlayerView.vehicle_health`; E2: firework `Burst`, `ToClient::MapData`). The release step bumps `PROTOCOL` once (`messages.rs:7` and the asserts at `codec.rs:882`, `:912`, `:930`, `:1004`) when anything on the wire changed since the last release. The relay `iw4l-master` does not depend on `minecraft_net` (decisions.md), so it needs no change, but every release still builds and copies it.
- **Decisions go in the track log** while the track runs. The merge into `minecraft-survival` copies them into `docs/round10/decisions.md` under a "Track E1" or "Track E2" heading. This keeps both tracks off the same file.
- Every commit: one item, plain-English subject, no code comments, minimal diffs to existing files, new files `rustfmt --check` clean, no Mojang files.

### Files shared with the other round 10 tracks

- **D0 (shared mob registry):** vendored `world.rs`, `world/combat.rs`, `world/witches.rs`, `world/leashes.rs` (`body_of`), `illager_render.rs`, `piglin_render.rs`, `mob_actions.rs`, `thrown.rs`, `trident.rs`, `mob_tracking.rs`, `entity_view.rs` per-kind loops. **Agreement with D0** (E-thrown): after E2's group `ti1`, per-store code lives only in the vendored `EntityWorld`. D0 adds its one generic arm in `hit_boxes` (or `arrow_targets`' match if it lands first), `hurt_by_player`/`hurt_for_player`, `apply_arrow_hit` and the four `witches.rs` potion sites, **never in `thrown.rs`, `splash.rs` or `wind.rs`**. D0's `body_of` must cover every new family, or new mobs' particle events are dropped (E-particles).
- **D (trial spawners, vaults, breezes, lodestones):** `dials.rs` is D's (lodestone compass); E does not touch it. `core/src/loot.rs` (vault loot), `containers.rs`, `dispenser.rs`, `locate.rs`, `level_events.rs`, `ambient_particles.rs`. D's breeze wind charge calls E2's `ServerSim::projectile_hit_block` and E2's `NO_ANGER_FROM_WIND_CHARGE` check, and D's Bad-to-Trial Omen turn calls E1's `Survival::give_effect`. E1's block entity components command can serve D's placed spawners and vaults.
- **W (darkness, game events, sculk, warden):** W adds `BLOCK_OPEN`/`BLOCK_CLOSE` in `set_door_open` (E2 doors route through it), `BLOCK_CHANGE` in `Level::ring_bell` (`redstone.rs:776`), `EXPLODE` at the `Exploded` arm of `tick_rockets`, and game events in `actions.rs`, `tick_thrown` and `tick_tridents`. E adds no game events. `render_gpu .../minecraft_world.rs` (E1 outline) may meet W's darkness.
- **M1-M3 (remaining mobs, phantoms):** `monster_ai.rs`, `horse.rs`/`equines.rs`/`riding.rs` (skeleton and zombie horses, camels), `feline_ai.rs`/`felines.rs` (cats scare phantoms), `emissions.rs`, `living.rs`, `death.rs`. New seams for them: `mob_particles::puff_for` (one row per mob event), `HorseKind::can_gallop`, `hit_boxes`.
- **F (Elden Ring boss):** `entities.rs` targets on the ray, `frame/src/minecraft_ui.rs` and `hud` (boss bar), bursts in `mob_sender.rs`.
- **Everyone:** `server.rs`, `entities.rs`, `render_anim` `minecraft_levels.rs`/`minecraft_world.rs`/`minecraft_remote.rs`.

**Order against D0.** The groups that touch D0's per-kind sites are E1 `particles-spit-love` (commit 4) and E2 `ti1`, `ti2`, `fw-burst` and `bell-hits` (commit 2). They come late enough in each track that D0 has usually merged. When D0 is on `minecraft-survival`, the orchestrator merges `minecraft-survival` into the E track branch before those groups, and the group uses D0's generic arms.

---

# Track E1: screens, visuals and sounds

## E-kit. Kits and /give take banner patterns, loaded crossbows and firework stars

One commit in E1's first group serves both tracks' kits.

### Cause

- `crates/minecraft_commands/src/args.rs:404` lets through only book, ominous-amplifier, custom-data and enchantment components. Anything else is `arguments.item.component.unknown`.
- `crates/minecraft_terrain/src/crafting.rs:483-513` `kit_components` keeps only custom data, book content and the ominous amplifier. Other components are dropped without a word.
- So `banner_patterns`, `base_color`, `charged_projectiles`, `fireworks` and `firework_explosion` can't be given for a test.

### Plan

**Commit: "Minecraft item commands and kits take banner patterns, shield colours, loaded crossbows and firework stars"**
- `args.rs:390-441`: accept
  - `banner_patterns`: a list of `{pattern, color}`. `pattern` is an id with or without `minecraft:`, or an inline `{asset_id, translation_key}`.
  - `base_color`: a dye name, quoted or not.
  - `charged_projectiles`: a list of `{id, count?}`.
  - `fireworks`: `{flight_duration, explosions}`.
  - `firework_explosion`: `{shape, colors, fade_colors, has_trail, has_twinkle}`.
- Read the SNBT with the existing `nbt` reader and turn it into JSON with `item_nbt::tag_json`, as `custom_data` does. Unknown dyes, shapes or malformed lists give `arguments.item.component.malformed`.
- `crafting.rs` `kit_components`: the same five.

### Tests

- `parse_item` for each component, and an error for a bad dye, a bad shape and a malformed list.
- `kit_components`: the banner, shield, crossbow and both firework entries of the kits below give the right JSON.
- A kit entry `minecraft:firework_rocket[fireworks={flight_duration:1,explosions:[{shape:large_ball,colors:[I;11743532],has_trail:1b}]}]*64` gives 64 rockets with that component, and the star survives a save and load.

---

## E-particles. Taming hearts, bucking smoke, trust hearts, raid sweat and the llama spit puff

### The owner's finding

> "**Particles:** taming hearts, bucking smoke, ocelot trust hearts, the villagers' sweat in raids and the llama spit puff are not drawn yet." (`docs/round9/release-2-tests.md:404`)

D2-13 in the same file also says "(The taming hearts are not drawn yet.)". The round 9 log says the same for wolves, cats and ocelots (`track-d-log.md:747`), horses (`:944`), raid sweat (`:1895`) and the spit (`:868`, `:883`).

### Cause in the current code

**Entity events are recorded, then thrown away.**
- The vendored mob world already records vanilla's `broadcastEntityEvent` calls in `EntityWorld.entity_events` (`third_party/minecraftoss/entities/src/world.rs:2621-2623`), and `take_entity_events` hands them out (`world.rs:4068-4071`).
- What it records today:
  - **7 (tamed, hearts) and 6 (failed try, smoke):** horses, donkeys, mules and llamas (`world/equines.rs:472` and `:479`, where `out.buck` is the buck), cats (`world/felines.rs:548` and `:550`) and wolves (`world/wolves.rs:401` and `:403`).
  - **41 (trusts, hearts) and 40 (smoke):** ocelots (`world/felines.rs:495` and `:497`, constants in `feline.rs:43-44`).
  - **18 (in love):** a fed tame horse (`world/equines.rs:692`), cats and ocelots (`world/felines.rs:862`), wolves (`world/wolves.rs:374` and `:409`), and the parent after breeding (`wolves.rs:309`, `felines.rs:631`).
  - **Villagers:** 12 hearts (`world/villagers.rs:709`, `villager_brain/behaviors.rs:2158`), 13 anger (`world.rs:4046`, `behaviors.rs:2105`), 14 happy after a trade (`world/villagers.rs:559`), 42 raid sweat (`world/villagers.rs:843`, through `Remote::EntityEvent` at `:659`), and 18 to both breeding villagers (`behaviors.rs:2141`). Vanilla does nothing on a villager's 18.
  - **Ravagers:** 39 and 69 (`world/ravagers.rs:311` and `:438`). These are already drawn from the shown clocks (`crates/minecraft_survival/src/entity_view.rs:240-258`).
- The server drops all of them at `crates/minecraft_terrain/src/server.rs:1100-1102`:
  ```rust
  // Entity events (a villager's hearts, anger, happiness) are for the
  // client's particles, which it does not draw yet.
  let _ = mobs.take_entity_events();
  ```
  This comment is upstream-vendored style in server.rs. Delete it with the line. Add no new comments.

**Cows, sheep, pigs, chickens, mooshrooms, striders and hoglins don't even record 18.** `crates/minecraft_terrain/src/mob_actions.rs` gets `CowEvent::Hearts` / `AnimalEvent::Hearts` back from `interact`, but only reads milk and the age lock out of it: cow `:191-200`, sheep `:219-227`, pig `:229-245`, strider `:247-262`, hoglin `:269-277`, chicken `:279-287`, mooshroom `:311-328`.

**How one-off effects reach the host's view and friends today.** Use the firework route, which already works for both:
- **Server:** `ServerSim.firework_bursts` (`server.rs:119`, set up at `:492`) is filled during the tick and taken into `EntitySnapshot.firework_bursts` (`server.rs:347`) in `finish_batch` (`server.rs:4363`).
- **Host:** `entities.rs:1556-1566` hands the snapshot to `EntityView::receive`, which calls `EntityView::bursts` (`entity_view.rs:304-325`). For friends, `mob_sender::keep_bursts` keeps it until the next send (`entities.rs:1558-1562`).
- **Friends:** `session.rs:1369-1397` uses `take_bursts` and `bursts_message` (`mob_sender.rs:208-256`) to cut the bursts out of the droppable state frame and send them on the reliable `ToClient::Bursts` (`minecraft_net/src/messages.rs:315`, codec tag 28). Each friend's copy goes through `near()` (`mob_sender.rs:159-205`). The friend's handler is `remote.rs:510-514`, which calls the same `EntityView::bursts`.

**Llama spit.** Vanilla draws the spit's puff on the client when the entity appears (`LlamaSpit.recreateFromPacket`). Our view never does.
- The view's mob world holds the spit. Host: `entities.rs:1674` (`self.view.world = *mobs`). Friend: hatched from the tag with `Motion` (`server_mobs.rs:1162-1179`, written at `:1944-1960`).
- The model is drawn (`entity_view.rs:1141-1143`, `llama_render.rs:371`), but no particles are.
- The view already has a "fire once per new id" pattern: `fang_crits` and `ravager_roars` (`entity_view.rs:113-114`, `:219-258`).

**The particle engine.**
- `crates/minecraft_terrain/src/ambient_particles.rs` has `Kind` (`:42-59`), the constructors, a tick for each kind (`:1087-1135`), sprites (`:451-484`), sizes (`:486-505`) and the atlas list `textures()` (`:678-690`, loaded at `mesh.rs:720`).
- It already has `Smoke`, `Poof`, `HappyVillager` and `Rain` (the water drop), and `generic_*`/`splash_*` are in the atlas (`mesh.rs:705-711`).
- It has no heart, angry-villager, splash or spit kind. `heart.png` and `angry.png` are not in the atlas yet.

### Vanilla 26.3 facts

**Taming, trust and love.** `TamableAnimal.spawnTamingParticles(boolean)`, `AbstractHorse.spawnTamingParticles(boolean)` and `Ocelot.spawnTrustingParticles(boolean)` use the same loop:
- 7 particles: `HEART` when true, `SMOKE` when false.
- For each one, in this order: motion `nextGaussian() * 0.02` three times (x, y, z); then the position `getRandomX(1)`, `getRandomY() + 0.5`, `getRandomZ(1)`.
- `getRandomX(1)` is `x + bbWidth * (2 * nextDouble - 1)`, and `getRandomY()` is `y + bbHeight * nextDouble`.

**Which event triggers what:**
- `TamableAnimal.handleEntityEvent` and `AbstractHorse.handleEntityEvent`: 7 is hearts, 6 is smoke, anything else goes to `Animal`.
- `Ocelot.handleEntityEvent`: 41 is hearts, 40 is smoke.
- `Animal.handleEntityEvent(18)`: the same loop with 7 `HEART`.
- `Animal.setInLove` sets `inLove` to 600 and broadcasts 18. `Animal.aiStep` also adds a heart every 10 ticks while `inLove > 0`, but only where `inLove` is set (the server, where `addParticle` does nothing), and `inLove` is not synced. So clients only see the burst of 7.

**Villagers.**
- `Villager.handleEntityEvent`: 12 `HEART`, 13 `ANGRY_VILLAGER`, 14 `HAPPY_VILLAGER`, 42 `SPLASH`. All go through `AbstractVillager.addParticlesAroundSelf`: **5** particles, gaussian * 0.02 motion first, then `getRandomX(1)`, `getRandomY() + 1.0`, `getRandomZ(1)`.
- Event 18 has no handler on a villager. `LivingEntity` and `Entity` switch only on 2, 3, 46-55, 60, 65, 67, 68 and 53.

**Llama spit.** `LlamaSpit.recreateFromPacket`: 7 `SPIT` particles at the spit's position. Particle `i` (0 to 6) gets motion `(mx * (0.4 + 0.1 * i), my, mz * (0.4 + 0.1 * i))`, where `m` is the packet's movement. Y is not scaled.

**Particle classes:**
- **`HeartParticle`** (`HEART`, sprite `minecraft:heart`): `super(level, x, y, z, 0, 0, 0)`, so the passed motion is ignored and the base random direction and speed are used. `speedUpWhenYMotionIsBlocked` true, friction 0.86. Velocity * 0.009999999776482582, then y + 0.1. quadSize * 1.5, lifetime 16, hasPhysics false, OPAQUE layer. `getQuadSize = quadSize * clamp((age + partial) / lifetime * 32, 0, 1)`.
- **`HeartParticle$AngryVillagerProvider`** (`ANGRY_VILLAGER`, sprite `minecraft:angry`): the same particle at `y + 0.5`, `setColor(1, 1, 1)`.
- **`SplashParticle`** (`SPLASH`, sprites `splash_0..3`): a `WaterDropParticle` with gravity 0.04 instead of 0.06. If `ya == 0` and (`xa != 0` or `za != 0`), the velocity becomes `(xa, 0.1, za)`. `WaterDropParticle`: xd * 0.3, yd = `nextFloat * 0.2 + 0.1`, zd * 0.3, size 0.01, lifetime `(int)(8 / (nextFloat * 0.8 + 0.2))`. Its tick counts the lifetime down, `yd -= gravity`, moves, * 0.98, on the ground a 50 % removal and * 0.7, and it is removed inside a collision shape or fluid. This is exactly `drip()` (`ambient_particles.rs:389-413`).
- **`SpitParticle`** (`SPIT`, sprites `generic_7..0`): an `ExplodeParticle` (the poof: gravity -0.1, friction 0.9, motion + 0.05 jitter, colour 0.7-1.0, `0.1 * (r * r * 6 + 1)` size, lifetime `16 / (r * 0.8 + 0.2) + 2`) with **gravity 0.5**. It is `Particle::poof` (`ambient_particles.rs:179-194`) with the gravity changed.
- **`SmokeParticle`** (`SMOKE`): the fork already has it as `Kind::Smoke`. **`HAPPY_VILLAGER`** (`glint`): the fork already has `Kind::HappyVillager`.

**Assets** (26.3 resource pack): `particles/heart.json` = `["minecraft:heart"]`, `angry_villager.json` = `["minecraft:angry"]`, `splash.json` = `splash_0..3`, `spit.json` = `generic_7..generic_0`. The textures `textures/particle/heart.png` and `angry.png` exist.

### Plan

**Design.**
- The server turns each entity event into a typed, positioned puff list at the end of the tick, the way fireworks work. It reads the mob's body with `EntityWorld::body_of(id)` (`world/leashes.rs:111`: feet, width and height) and the tracking range with `mob_tracking::tracking_range(entity_type)`.
- The list rides in `EntitySnapshot`'s burst set: to the host's own view, and to each friend through `ToClient::Bursts`, filtered by `near()` with the puff's range.
- Each viewer draws the specks with the view's own random. Host and friend see the same kind, count and place, not the same specks (as with the wither smoke, `track-d-log.md:2823`).
- Events 39 and 69 (ravager) stay with the clock-driven effects and map to nothing.
- The spit puff needs no server change. The view fires it once per new spit id.

#### Group `particles-events` (M)

**Commit 1: "Minecraft taming shows hearts, and a failed try or a bucking horse shows smoke"**
- **New `crates/minecraft_terrain/src/mob_particles.rs`** (rustfmt-clean, no comments):
  - `#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)] pub enum PuffKind { Heart, Smoke, AngryVillager, HappyVillager, Splash }`.
  - `pub struct MobPuff { pub kind: PuffKind, pub count: u8, pub feet: DVec3, pub width: f32, pub height: f32, pub lift: f64, pub range: i32 }` (serde).
  - `pub fn puff_for(entity_type: &str, event: u8) -> Option<(PuffKind, u8, f64)>`, the vanilla table:
    - `minecraft:villager` first: 12 Heart, 13 AngryVillager, 14 HappyVillager, 42 Splash, each `(5, 1.0)`; 18 and anything else give None.
    - `minecraft:ocelot`: 41 Heart, 40 Smoke, each `(7, 0.5)`.
    - Any other type: 7 Heart, 6 Smoke, 18 Heart, each `(7, 0.5)`; everything else None.
  - `pub fn mob_puffs(world: &EntityWorld, events: &[(u64, u8)]) -> Vec<MobPuff>`: skips an id with no `entity_type`, no `body_of` or no table row.
  - `pub mod mob_particles;` in `minecraft_terrain/src/lib.rs`.
  - Commit 1 adds the 7/6/18 and ocelot rows. Commit 2 adds the villager rows.
- **`server.rs`:**
  - a `mob_puffs: Vec<crate::mob_particles::MobPuff>` field beside `firework_bursts` (`:119`, `:492`);
  - replace `:1100-1102` with `let events = mobs.take_entity_events(); self.mob_puffs.extend(crate::mob_particles::mob_puffs(&self.mobs, &events));` (use whatever binding the surrounding code holds);
  - `pub mob_puffs: Vec<crate::mob_particles::MobPuff>` on `EntitySnapshot` (`:347`), next to `firework_bursts`;
  - `mob_puffs: std::mem::take(&mut sim.mob_puffs)` in `finish_batch` (`:4363`).
- **`ambient_particles.rs`:**
  - `Kind::Heart`: `Particle::heart(at, random)` = `base(Kind::Heart, at, ZERO)`, friction 0.86, velocity * `0.01_f32`, y + 0.1, quad_size * 1.5, lifetime 16.
  - Tick it with `coast()`: gravity 0, no collision, which equals vanilla's no-physics tick, because y motion never hits exactly 0.
  - Sprite `"heart"`, size like the `Smoke | Crit` arm (grows over 1/32 of its life), opaque. Add `"heart"` to `textures()`.
  - Append the new variants at the end of `Kind` and add separate match arms.
- **`crates/minecraft_survival/src/mob_sender.rs`:**
  - `near()`: `mob_puffs: filtered(&snapshot.mob_puffs, |puff| has_chunk(chunk_of(puff.feet)) && tracks(puff.range, view_distance, feet.to_array(), puff.feet))`;
  - `take_bursts`, `add_bursts` and `has_bursts` take, extend and check `mob_puffs`.
- **`entity_view.rs`:** a new `fn mob_puffs(&mut self, puffs: &[MobPuff])`, called with one line at the top of `bursts()` (E2 rewrites the firework part of `bursts()`, so keep the E1 code in its own function):
  - for each puff, `count` times: three `next_gaussian() * 0.02`, then `x = feet.x + width * (2 * nd - 1)`, `y = feet.y + height * nd + lift`, `z = feet.z + width * (2 * nd - 1)`;
  - then `self.ambient.add(kind, at, motion)`, with `PuffKind` mapped to `Kind` (Heart, Smoke, HappyVillager; AngryVillager and Splash come in commit 2).

**Commit 2: "Minecraft villagers show hearts, anger, happiness and raid sweat"**
- **`ambient_particles.rs`:**
  - `Kind::AngryVillager`: the heart constructor at `at + (0, 0.5, 0)`, sprite `"angry"`, colour white, coast tick, size like Heart. Add `"angry"` to `textures()`.
  - `Kind::Splash`: `rain()`'s draws with gravity 0.04, plus the `ya == 0` override, ticked by `drip()` (the same arm as `Rain`), sprite `splash_{n}`.
- **`mob_particles.rs`:** the villager rows (12, 13, 14, 42; none for 18).
- **`entity_view.rs`:** map `AngryVillager` and `Splash`.

**Protocol.** `EntitySnapshot.mob_puffs` changes the wire layout, so host and friend must run the same build. No bump in this group: the release step bumps `PROTOCOL` once (see the overview).

**Tests:**
- `minecraft_terrain::mob_particles::tests`:
  - `taming_shows_seven_hearts_and_a_failed_try_seven_smokes`: horse, llama, wolf and cat, events 7/6 give `(Heart|Smoke, 7, 0.5)`.
  - `an_ocelot_shows_hearts_when_it_trusts_and_smoke_when_it_does_not`.
  - `villager_events_give_five_particles_a_block_up`: 12, 13, 14, 42.
  - `a_villager_in_love_and_a_ravager_s_roar_draw_nothing_here`: villager 18, ravager 39 and 69 give None.
  - `an_event_of_a_mob_that_is_gone_is_dropped`.
  - `a_puff_sits_on_the_mob_s_body`: a spawned wolf; the feet, width and height come from `body_of`; range 10.
- `minecraft_terrain::server::tests::a_tamed_wolf_s_hearts_reach_the_snapshot`: generated terrain, a wolf, `wolf_interact` with bones until it is tamed; the next snapshot has a Heart puff of 7 at the wolf; the snapshot after it has none.
- `ambient_particles::tests`:
  - `a_heart_rises_slowly_through_blocks_for_sixteen_ticks`: lifetime 16, it passes through a stone block, the size grows over the first half tick, sprite `minecraft:particle/heart`.
  - `an_angry_cloud_starts_half_a_block_higher_and_is_white`.
  - `a_splash_drop_falls_at_0_04_and_pops_on_the_ground`.
  - `textures()` contains heart and angry.
- `minecraft_survival::mob_sender::tests::mob_puffs_go_on_the_reliable_stream_and_stay_with_their_range`: `keep_bursts` keeps them, the state frame carries none, a friend outside range gets none, and nothing is sent before the chunk.
- `entity_view::tests::a_heart_puff_draws_seven_hearts_on_the_mob`: `kinds_at` gives 7 Heart within the box.
- `remote::tests::a_friend_sees_the_hosts_taming_hearts`: decode a `ToClient::Bursts` with a puff and check the friend's ambient particles.

#### Group `particles-spit-love` (S), late in E1

**Commit 3: "Minecraft llama spit leaves a puff"**
- **`ambient_particles.rs`:** `Kind::Spit` = `Particle::poof(...)` with `gravity = 0.5`, the default `tick()` arm, sprites like Poof (`generic_7..0` by age).
- **`entity_view.rs`:**
  - a `spit_puffs: HashSet<u64>` field, set up next to `fang_crits`;
  - a `fn spit_puffs(&mut self)` called in `advance` beside `fang_crits()` (`:147-149`). For every alive spit in `self.world.llama_spits()` whose id is new, it adds 7 `Kind::Spit` at `spit.position` with motion `(v.x * (0.4 + 0.1 * i), v.y, v.z * (0.4 + 0.1 * i))`;
  - the set keeps only the ids of spits that still exist.
- Host and friend both have the spit with its velocity in `view.world`. On the host, the first view may show the spit one tick after launch. That is close enough, because the specks come from each viewer's own random anyway.
- **Test:** `entity_view::tests::a_new_spit_puffs_seven_specks_once_that_fall`: spawn a spit in `view.world` and tick: 7 Spit particles; tick again: no new ones; after a few ticks their y has dropped.

**Commit 4: "Minecraft cows, sheep, pigs, chickens, mooshrooms, striders and hoglins show hearts when fed for love"**
- **Vendored `world.rs`:** add `pub fn broadcast_entity_event(&mut self, id: u64, event: u8)` next to `take_entity_events` (`:4068`).
- **`mob_actions.rs`:** one small helper `fn love(world: &mut EntityWorld, id: u64, hearts: bool)`, and a one-line call in the cow, sheep, pig, strider, hoglin, chicken and mooshroom arms (`:191-328`), using the `Hearts` already in their `events`.
- This goes beyond the quoted list. It is the same missing vanilla 18 and costs one line per arm. The owner's round 9 steps already promise "hearts" when feeding (`track-d-log.md:845`, `:889`).
- **Merge order:** D0 rewrites these per-kind arms. If D0 has merged, put the call in D0's generic animal-interact arm instead.
- **Test:** `mob_actions::tests::feeding_an_adult_cow_wheat_records_the_love_event` records `(id, 18)`; a baby fed wheat records none.

### The owner's in-game steps

Start in a new survival world on plains with the kit below. `/time set day`. Hearts are small red hearts that rise slowly for under a second. Smoke is a few grey puffs that drift up.

- **P-1. A horse.**
  1. Use the horse egg and right-click the horse with an empty hand: you ride it while it runs about.
  2. When it rears and throws you off, 7 grey smoke puffs rise around it. **Screenshot P1.**
  3. Feed it wheat a few times and try again until it stays calm: 7 red hearts rise around it, half a block above its body. **Screenshot P2.**
- **P-2. A llama.** Use the llama egg and tame it the same way: smoke when it throws you, hearts when it is tamed.
- **P-3. A wolf.** Use the wolf egg and right-click it with a bone. Each failure shows smoke. The success shows hearts and the wolf sits with a red collar. **Screenshot P3.**
- **P-4. A cat.**
  1. Use the cat egg. Hold cod and stand still until it creeps close, then right-click it with cod.
  2. A failure shows smoke and a success shows hearts. **Screenshot P4.**
  3. Tame a second cat and feed both cod at full health: hearts on each.
- **P-5. An ocelot.** Use the ocelot egg, hold cod and stand still. When it is within 3 blocks, right-click it with cod. A failure shows smoke. The success shows hearts, and it stops running from you. **Screenshot P5.**
- **P-6. Love hearts.**
  1. `/summon minecraft:horse ~ ~ ~ {Tame:1b}` and feed it a golden carrot: hearts.
  2. Use the cow egg twice and feed each cow wheat: hearts on each, then a calf. **Screenshot P6.** (Needs `particles-spit-love`.)
  3. Do the same with two sheep and wheat.
- **P-7. A llama spits.** Use the llama egg and hit the llama once with the iron sword. When it spits, a small white puff flies from its mouth towards you and drops. **Screenshot P7**, just after a spit. (Needs `particles-spit-love`.)
- **P-8. Villagers.**
  1. Use the villager egg. Punch it once: a dark grey cloud puffs over its head.
  2. Place the composter beside a second villager. When it has become a farmer (a straw hat), make one trade: green sparkles over its head. **Screenshot P8.** A 26.3 farmer offers 2 of its 5 novice trades at random (20 wheat, 26 potatoes, 22 carrots or 15 beetroot for an emerald, or an emerald for 6 bread), so it always wants at least one crop. The kit has all four crops.
- **P-9. Raid sweat.**
  1. `/locate structure minecraft:village_plains` and `/tp` there.
  2. Walk 60 blocks out, drink an ominous bottle and walk back in. Wait for the raid.
  3. While the villagers hide in the houses, each one now and then (about every 5 seconds) shows a few light-blue water drops around its head, which fall and vanish. **Screenshot P9**, through a door or window.
- **P-10. Optional, villager hearts.** Put down three beds near two villagers and throw each one 6 bread. After a while hearts rise over both, and a baby villager appears.

**Two windows** (host H, friend F; start them with `scripts\mp-local.ps1` or over the relay):
- **P-11.** F tames a wolf with a bone while H stands 10 blocks away. Smoke and then hearts show in **both** windows. **Screenshot P10**, both windows.
- **P-12.** H rides an untamed horse until it bucks, with F watching from 10 blocks: F sees the smoke. H tames it: F sees the hearts.
- **P-13.** F hits a llama: in both windows the puff flies from the llama's mouth towards F. **Screenshot P11**, F's window.
- **P-14.** F walks 200 blocks away, and H tames a cat: F sees nothing. F comes back, and H feeds the tame cat cod at full health: F sees the hearts.
- **P-15.** During a raid with both in the village, F also sees the villagers' water drops.

```
IW4L_MINECRAFT_KIT=minecraft:horse_spawn_egg*2,minecraft:llama_spawn_egg*2,minecraft:wolf_spawn_egg*2,minecraft:cat_spawn_egg*2,minecraft:ocelot_spawn_egg*2,minecraft:villager_spawn_egg*4,minecraft:cow_spawn_egg*2,minecraft:sheep_spawn_egg*2,minecraft:wheat*64,minecraft:potato*64,minecraft:carrot*64,minecraft:beetroot*64,minecraft:golden_carrot*16,minecraft:bone*32,minecraft:cod*64,minecraft:bread*32,minecraft:white_bed,minecraft:white_bed,minecraft:white_bed,minecraft:composter,minecraft:emerald*64,minecraft:ominous_bottle*2,minecraft:oak_fence*32,minecraft:iron_sword,minecraft:shield,minecraft:cooked_beef*32
```

26 stacks (26 backpack slots; a count above an item's stack size takes more slots, and the kit only fills the 27 backpack slots). Every id was checked against `assets/minecraft/items/*.json` of 26.3.

### Files shared

- `server.rs` (all tracks): one or two lines each at `:119`, `:347`, `:492`, `:1100-1102`, `:4363`.
- `mob_sender.rs` (W, D and F may add bursts): `near`, `take_bursts`, `add_bursts`, `has_bursts`.
- `ambient_particles.rs` (W sculk and shriek, D trial spawner and vault, F): variants appended at the end of `Kind`, separate arms.
- `entity_view.rs` (E2 fireworks in `bursts()`, M1-M3 render, F, W): one call line in `bursts()`, a field and one call in `advance`.
- Read only: `mob_tracking::tracking_range` (D0 extends it) and `EntityWorld::body_of` (D0 must cover new families).
- Commit 4 only: `mob_actions.rs` and vendored `world.rs` (D0 rewrites the per-kind arms).
- New seam for M1-M3: `mob_particles::puff_for`. A new mob's entity event (a parrot's 7/6, a dolphin's 38, a fox's 45) is one table row plus, if needed, a `PuffKind`.

---

## E-trading. Trading screen: number keys, F, Q, drags and double-click

### The owner's finding

> **Trading:** number keys, Q, double-click and drags on the payment and result slots do nothing. (`docs/round9/release-2-tests.md:405`)

### Cause

- The HUD already sends the generic clicks on every screen, the trading screen included:
  - `McClick::Swap` for number keys and F (`hud/src/minecraft_clicks.rs:256-272`);
  - `McClick::Throw` for Q and Ctrl+Q (same place);
  - `McClick::Gather` for a double-click (`minecraft_clicks.rs:183-207`);
  - `McClick::Spread` for drags (`minecraft_clicks.rs:217-231`).
  - `render_anim/src/minecraft_inventory.rs:806-818` turns each into a `MenuAction`. A friend's client sends the same `MenuAction` to the host (`render_anim/src/minecraft_remote.rs:1476`), so **no net protocol change is needed**.
- Chests, benches and mounts handle every action kind: `block_action`, `bench_action` and `mount_action` in `minecraft_survival/src/menus.rs:595-722` and `853-885`.
- The trading screen goes through `merchant_action` (`menus.rs:887-907`):
  - `merchant_op` (`menus.rs:814-838`) only maps `Slot` on the payments, `Slot` on the result, shift-click on the inventory and `Choose`.
  - `merchant_local` (`menus.rs:840-851`) lets through only actions that touch the player's own slots.
  - Everything else is dropped without a word: `Swap` or `Throw` on `Container(0|1)` or `Output`, any `Spread` that includes a payment slot, and `Gather` on a payment slot.
- The server side has no operation for these clicks either:
  - `MerchantOp` (`minecraft_terrain/src/server.rs:265-274`) is `Select`, `Payment`, `Result`, `QuickMove` and `Close`.
  - `Server::merchant` (`server.rs:689-707`) dispatches only those.
  - `MerchantMenu` (`third_party/minecraftoss/entities/src/merchant.rs`) has only `click_payment`, `click_result`, `quick_move_*` and `select_trade`. The world wrappers are in `entities/src/world/merchants.rs:263-282`.
- Double-click on an inventory slot is wrong the other way: `merchant_local` sends `Gather` on an inventory slot to `grid_action`, which gathers. Vanilla never gathers on this screen.

### Vanilla 26.3 (`AbstractContainerMenu.doClick`, `MerchantMenu`, `MerchantResultSlot`, `MerchantContainer`)

- **SWAP** (button 0-8 = hotbar, 40 = offhand). The client only sends it when the cursor is empty.
  - Both empty: nothing.
  - Hotbar empty: if `slot.mayPickup`, the hotbar slot gets the slot's stack, then `slot.onSwapCraft(count)`, `slot.setByPlayer(EMPTY)` and `slot.onTake`.
  - Slot empty: if `mayPlace`, `max = slot.getMaxStackSize(stack)` (min(99, the item's max)). The slot takes `split(max)` when there is more than that, else the whole stack and the hotbar slot empties.
  - Both full: needs `mayPickup && mayPlace`. When the hotbar stack is over max, the slot takes `split(max)`, `onTake` runs, and the old slot stack goes back to the inventory with `inventory.add`, else it drops. Otherwise the two swap and `onTake` runs.
  - The result slot's `mayPlace` is false, so a number key on the result works only into an **empty** hotbar or offhand slot.
  - `MerchantResultSlot.mayPickup` needs an active offer that the payments satisfy, either way round.
  - `onTake` pays: `offer.take(a,b)`, else `offer.take(b,a)`, then `notifyTrade` and `setItem` on both payments, which re-reads the offers and refills the result.
- **THROW** (needs an empty cursor).
  - Button 0 takes `safeTake(1, MAX)`. Button 1 (Ctrl) takes the whole count and repeats while `isSameItem(slot.getItem(), dropped)`.
  - Each take is dropped with `player.drop(stack, true)`, the same toss as Q.
  - On the result, `MerchantContainer.removeItem(2, n)` always removes the **whole** result stack. Then `onTake` pays and the result refills. So Q makes one trade and drops it. Ctrl+Q keeps trading and dropping until the payments run out or the offer is out of stock.
  - On a payment slot, Q drops 1 and Ctrl+Q drops the stack.
- **QUICK_CRAFT** (drags). Payment slots accept drags: `mayPlace` is true and `canDragTo` keeps its default. The result slot does not (`mayPlace` false).
  - Each slot ends up with `min(stack max, slot max, existing + place count)`.
  - The place count is `floor(count / slots)` for a left drag and 1 for a right drag. Type 2 (middle drag) is creative only.
  - The cursor keeps the rest. Each `setByPlayer` on a payment re-reads the offers.
- **PICKUP_ALL** (double-click). `MerchantMenu.canTakeItemForPickAll` returns **false** (bytecode `iconst_0; ireturn`). The client checks it in `mouseReleased` before sending PICKUP_ALL, and before the shift-double-click quick-move loop. So on the trading screen a double-click never gathers, on any slot. The second click falls through to an ordinary click: the stack picked up by the first click is put back.

### Plan: group `trading-keys` (L), 4 commits

1. **"Minecraft trading screen takes number keys and the offhand key on its payment and result slots"**
   - `merchant.rs`: add `MerchantMenu::swap_payment(slot, hotbar, inventory, merchant)` and `swap_result(hotbar, inventory, merchant)`, following the SWAP rules. The result swap reuses `may_take_result` and `take_result`. A full-slot leftover goes through `inventory.add_item`; what does not fit is returned for dropping.
   - World wrappers in `world/merchants.rs` next to `merchant_click_payment`.
   - `server.rs:265`: `MerchantOp::Swap { slot: usize, hotbar: usize }` next to `Payment` (slot 0/1 is a payment, 2 the result). Dispatch it at `server.rs:691`.
   - `menus.rs:814`: map `Swap { slot: Container(0|1|2) | Output, hotbar }` to it.
   - `keeps_gun` (`menus.rs:497-500`) already refuses moving a gun from the hotbar into `Container(_)`. Keep that.
2. **"Minecraft trading screen throws from its payment and result slots with Q"**
   - `merchant.rs`: `throw_payment(slot, whole)` and `throw_result(whole)`, returning the stacks to drop. The result loops with Ctrl as described.
   - `MerchantOp::Throw { slot: usize, whole: bool }`.
   - `MerchantUpdate` (`server.rs:280-287`) gets `thrown: Vec<ItemStack>`. Set it in `Server::merchant`, and fill it with empty lists at `server.rs:723` and in the tests.
   - The thrown stacks reach the player through the inventory's pending drops, so the host, the friend and a forced close all use one path:
     - `pub fn queue_drop(&mut self, stack)` in `third_party/minecraftoss/player/src/inventory.rs`, next to `take_pending_drops` (line 659).
     - `apply_merchant_answer` (`minecraft_survival/src/entities.rs:222-229`) queues `update.thrown`.
     - `pump_merchant` (`menus.rs:1266`) and `apply_input` (`render_anim/src/minecraft_inventory.rs:373`) already take pending drops into `thrown`. The host throws them from the eye (`minecraft_world.rs:1349`) and the friend's from the friend's eye (`minecraft_players.rs:2087-2118`).
     - The death drain (`inventory.rs:124`) also covers pending drops.
3. **"Minecraft trading screen shares a dragged stack over its payment slots"**
   - `merchant.rs`: `spread(players: &[usize], payments: &[usize], right, inventory, merchant)` using `Inventory::distribute_limited` (`inventory.rs:811`), the way `containers.rs:1306-1328` does. External cells are the two payments, with capacity `min(99, max)`. Then `set_payment_by_player` per changed payment.
   - `MerchantOp::Spread { players: u64, payments: u8, right: bool }` uses bit masks so that `MerchantOp` stays `Copy`. The inventory has 41 slots, which fit in a u64.
   - `menus.rs`: map any `Spread` that touches `Container(0|1)` to it. Spreads only over inventory slots stay local.
4. **"Minecraft trading screen never gathers on a double click, like vanilla"**
   - `hud/src/minecraft_clicks.rs`: `fn picks_all(&self) -> bool { true }` on the `Menu` trait. `MenuView` (`hud/src/minecraft_inventory.rs:107-128`) returns `self.layout.screen != McScreen::Merchant`. `release` (line 183) checks it next to `takes_all`, so the second click falls through to a plain click as in vanilla.
   - `menus.rs:840-851`: `merchant_local` returns false for `Gather`, so a friend's stray Gather does nothing either.

### Tests

- `merchant.rs` unit tests, with the `Fake` merchant at `merchant.rs:497`: payment to an empty hotbar and back; payment to the offhand (40); the result into an empty hotbar pays once and refills; the result onto a filled hotbar does nothing; Q on the result makes one trade; Ctrl+Q on a 2-use offer trades twice and then the result is gone; Q and Ctrl+Q on a payment; a left drag of 64 wheat over 2 payments and 2 inventory slots gives 16 each and re-reads the result; a right drag gives 1 each.
- `server.rs`: `MerchantOp::Throw` returns `thrown` and spawns the trade's orbs (copy the setup of the test at `server.rs:5105`).
- `menus.rs` (next to the mapping test at 2673): Swap, Throw and Spread map to the new ops; `Gather` sends nothing and changes nothing; an `Answer` with `thrown` ends up in `MerchantShown.thrown` for the host and for a friend.
- `hud/src/minecraft_clicks.rs`: on a menu with `picks_all` false, a double-click release puts the carried stack back with `Slot` and sends no `Gather`.

### Decisions (for the log)

- A double-click on the trading screen gathers nothing, on any slot, because vanilla's `MerchantMenu.canTakeItemForPickAll` is false. The owner's "double-click does nothing" is vanilla. We remove the gathering we did on inventory slots.
- `MerchantOp::Spread` uses bit masks so `MerchantOp` stays `Copy`.
- Stacks thrown from the trading screen come back through the inventory's pending drops (`Inventory::queue_drop`). The toss is the player's own Q toss from the eye. The server never tosses.

---

## E-effects. Effects listed beside the inventory

### The owner's finding

> **Effects:** the inventory screens don't list the effects beside them, so the HUD icons stay. (`docs/round9/release-2-tests.md:406`)

### Cause

- `hud/src/minecraft_hotbar.rs:95` always calls `minecraft_effects::draw_effects` (`hud/src/minecraft_effects.rs:141-153`, the top-right HUD icons). That happens before the screen pass in `hud/src/minecraft_inventory.rs:356`.
- The screen pass (`minecraft_inventory.rs:418-722`) never draws effects.
- The atlas does not load the list's frames: `SPRITE_FOLDERS` (`render_anim/src/minecraft_gui.rs:40-58`) has no `container/inventory/`.
- `TEXT_PREFIXES` (`minecraft_gui.rs:80-95`) does not keep the `effect.` or `enchantment.level.` names.
- `McEffect` (`frame/src/minecraft_ui.rs:143-148`) already carries id, amplifier, duration and ambient, for the host and the friend.

### Vanilla 26.3

- **Which screens.** Only `InventoryScreen` and `CreativeModeInventoryScreen` own an `EffectsInInventory`. Chest, crafting, merchant and the other container screens do not, so their HUD icons stay as now.
- **When the HUD icons hide.** `Hud.extractEffects` returns early when `screen.showsActiveEffects()`, which is `effects.canSeeEffects()`.
- **Geometry**, in GUI pixels:
  - `x0 = leftPos + imageWidth + 2`, and `room = width - x0`.
  - Nothing is drawn when there are no effects or `room < 32`. `canSeeEffects` is `room >= 32`.
  - `maxWidth = room >= 120 ? room - 7 : 32`.
  - `step = 33`, but `132 / (n - 1)` when there are more than 5 effects, so the boxes overlap.
  - `y` starts at `topPos`.
  - The effects are sorted in **natural** order (`MobEffectInstance.compareTo`, our `minecraft_effects::compare(a, b)`). The HUD uses the reverse.
  - With the recipe book open, `leftPos` moves right, so the list gets narrow (compact) or disappears.
  - `imageWidth` is 176 for the inventory (`minecraft_screens.rs:126`) and 195 for creative.
- **Each effect:**
  - `name` = `effect.minecraft.<id>`, plus `" " + enchantment.level.<amp+1>` for amplifiers 1..9.
  - `time` = `effect.duration.infinite` ("∞") for an infinite effect. Otherwise `floor(ticks / tickrate)` seconds as `%02d:%02d`, or `%02d:%02d:%02d` from one hour up.
  - Box width `w = min(maxWidth, max(32 + width(name) + 7, 32 + width(time) + 7))`.
  - The frame is `container/inventory/effect_background` (or `_ambient`) at (x0, y), size w x 32. Both are nine-slice 32x32 with border 4 (checked in the .png.mcmeta).
  - Text room is `w - 39`. When it is above 0, the name is drawn at (x0+32, y+7) in white, cut with `...` when wider than the room (`ComponentRenderUtils.clipText`: cut to room minus the ellipsis width, then add "..."), and the time at (x0+32, y+16) in `0xFF808080`.
  - The icon (`mob_effect/<id>`) is 18x18 at (x0+7, y+7).
  - Hover tooltip: when the name was cut, or the box has no text room (compact), hovering `[x0, x0+w] x [y, y+step]` shows the name and time.
  - The inventory list does **not** skip effects with `showIcon` off. The HUD does.

### Plan: group `effects-list` (M), 2 commits

1. **"Minecraft inventory loads the effect list frames and effect names"**
   - `render_anim/src/minecraft_gui.rs`: add `"container/inventory/"` to `SPRITE_FOLDERS` (the array length becomes 18), and `"effect."` and `"enchantment.level."` to `TEXT_PREFIXES` (16).
   - Extend the test at `minecraft_gui.rs:811`: `container/inventory/effect_background` and `_ambient` load as `NineSlice` 32x32 with border 4.
2. **"Minecraft inventory lists the active effects beside it"**
   - `hud/src/minecraft_effects.rs`:
     - `pub(crate) fn listed(effects, left, image_width, top, width, font) -> Option<Vec<Listed>>`, returning boxes, text, clipping and hover rectangles from the rules above.
     - `fn name(gui, effect)` and `fn time(gui, ticks)`, using 20 ticks a second.
     - `draw_listed` draws with `p.sprite` (nine-slice), `p.text(.., shadow: true)` and `p.sprite`.
     - `listed_tooltip` returns the hovered box's lines.
   - `hud/src/minecraft_inventory.rs`: in the screen pass, after `origin` (line 424) and before the gradient (line 646), draw the list when `ui.screen` is `Inventory` or `Creative(_)`. `leftPos` is `origin[0]`, from `minecraft_recipe_book::origin`, which already shifts for the book; `topPos` is `origin[1]`. Add `listed_tooltip` at the end of the tooltip chain (line 702).
   - `hud/src/minecraft_hotbar.rs:95`: skip `draw_effects` when `ui.inventory_open`, the screen is Inventory or Creative, and `room >= 32`. Share one helper with the screen pass so the two can't disagree.

### Tests (in `minecraft_effects.rs`)

- Geometry at width 480: inventory at left 152 gives x0 330, room 150, maxWidth 143. Boxes are 33 apart from `topPos`, in natural order (shortest time first).
- 7 effects give a step of 22.
- Room 73 (recipe book open) gives 32-wide boxes with no text and a tooltip on hover. Room 31 gives nothing, and the HUD icons stay.
- Names: "Regeneration II", "Bad Omen V". Amplifier 10 shows no level.
- Times: "00:05", "08:00", "01:40:00", "∞".
- A long name is cut with "...".
- `draw_effects` is skipped exactly when the list shows.

### Decision (for the log)

- The effect list and the bottle tooltip (E-omen) count 20 ticks a second. Vanilla divides by the `/tick rate`, which the HUD does not know. Under `/tick rate 10`, vanilla would show twice the time.

---

## E-omen. Omen sounds from `/effect give`, and the ominous bottle tooltip

### The owner's finding

> `/effect give` of an omen plays no sound. The ominous bottle's tooltip has no "Bad Omen" line. (`docs/round9/release-2-tests.md:406`)

### Cause

- **Sound.** `effect_give` (`minecraft_survival/src/commands.rs:418-468`) calls `status.add_effect` (`third_party/minecraftoss/player/src/survival.rs:277`) and plays nothing. Only the drink plays the omen sound: `survival.rs:1044-1049` returns `BAD_OMEN_SOUND` in `heard`, and `actions.rs:472-475` turns it into `Effect::Sound`.
- **Tooltip.** `McStack` (`frame/src/minecraft_ui.rs:33-51`) has no effect lines. `convert` in `render_anim/src/minecraft_inventory.rs:460-493` never reads `minecraft:ominous_bottle_amplifier`, and `tooltip_lines` (`hud/src/minecraft_inventory.rs:878-909`) has nothing to draw.

### Vanilla 26.3

- **Sound.** `MobEffects` registers `bad_omen`, `trial_omen` and `raid_omen` with `withSoundOnAdded`. The sounds are `minecraft:event.mob_effect.bad_omen`, `.trial_omen` and `.raid_omen`.
  - `LivingEntity.addEffect` calls `MobEffectInstance.onEffectAdded` only when the effect was **not already present**. An upgrade goes through `onEffectUpdated`, which plays no sound.
  - `MobEffect.onEffectAdded` calls `level.playSound(null, x, y, z, sound, entity.getSoundSource(), 1.0, 1.0)`, so everyone nearby hears it.
- **Tooltip.** `OminousBottleAmplifier.addToTooltip` builds `new MobEffectInstance(BAD_OMEN, 120000, value, false, false, true)` and passes it to `PotionContents.addPotionTooltip(list, consumer, 1.0, tickRate)`.
  - The line is `potion.withDuration` ("%s (%s)") of `getPotionDescription`: `effect.minecraft.bad_omen`, wrapped in `potion.withAmplifier` ("%s %s") with `potion.potency.<amp>` when amp > 0, and the duration "01:40:00".
  - The colour comes from `MobEffectCategory.getTooltipFormatting`. Bad Omen is **NEUTRAL**, which is BLUE (0x5555FF). HARMFUL would be RED.
  - Bad Omen has no attribute modifiers, so there is no "When Applied:" block.
  - Results: "Bad Omen (01:40:00)" for the plain bottle and "Bad Omen V (01:40:00)" for amplifier 4.

### Plan: group `omen-sound-tooltip` (S), 2 commits

1. **"Minecraft omens given by command play their sound"**
   - `minecraft_survival/src/survival.rs`: `effect_sounds: Vec<&'static str>`; `pub fn give_effect(&mut self, kind, ticks, amplifier) -> bool`, which calls `status.add_effect` and, when the effect was absent before and present after, pushes the kind's added sound from a table `added_sound(EffectKind) -> Option<&'static str>` (the three omens); `take_effect_sounds()`.
   - Move the drink's fresh-omen check at `survival.rs:1044-1047` onto `give_effect`, so there is one rule. Track D's Bad-to-Trial Omen turn gets its sound for free by calling it.
   - `commands.rs:466`: use `give_effect`.
   - `actions.rs` `act()`: drain `player.survival.take_effect_sounds()` into `acted.effects` as `sound(event, None, 1.0, 1.0)`, the same route as the drink.
   - Leave the raid omen path (`render_anim/src/minecraft_players.rs:922-927`) as it is.
2. **"Minecraft ominous bottles show their Bad Omen line"**
   - `frame/src/minecraft_ui.rs`: `pub effects: Vec<(String, bool)>` (text, beneficial-or-neutral) on `McStack`, and derive `Default` so later fields don't touch every literal. Fill the literals at `frame/src/minecraft_recipe_book.rs:182`, `hud/src/minecraft_screens.rs:911/1336`, `minecraft_recipe_book.rs:764`, `minecraft_creative.rs:738`, `minecraft_clicks.rs:309`, `minecraft_merchant.rs:306/349`, `minecraft_inventory.rs:1086/1123` and `minecraft_gui.rs:1107/1169/1239`.
   - `render_anim/src/minecraft_inventory.rs` `convert`: for `minecraft:ominous_bottle`, `effects = [bad_omen_line(lang, minecraft_terrain::bottle::ominous_amplifier(components))]`. The amplifier defaults to 0 for a bottle with no components.
   - `hud/src/minecraft_inventory.rs:887`: draw `stack.effects` right after the name, in BLUE 0x5555FF or RED 0xFF5555. Potion lines can use the same field later.

### Tests

- `commands.rs`: a first `/effect give` of each omen queues its sound once; a second give of the same omen, even at a higher level, queues none; `speed` queues none; the drink test at `survival.rs:2236-2242` still passes.
- `render_anim/src/minecraft_inventory.rs`: the plain bottle gives `("Bad Omen (01:40:00)", true)` and amplifier 4 gives `("Bad Omen V (01:40:00)", true)`.
- `hud`: the tooltip draws the line in blue under the name.

### The owner's steps for E-trading, E-effects and E-omen

World `r10te`, a fresh world. Start by day on open grass, at least 60 blocks from any village. Do the effect steps (TE-1 to TE-6) before placing the composter, because Bad Omen near a job site can start a raid. Default GUI scale, recipe book closed unless a step says otherwise.

**Effects (host)**
- **TE-1.** Drink a night vision potion and eat a golden apple. Run `/effect give @s minecraft:hero_of_the_village 600 0`. The icons show at the top right.
- **TE-2.** Open the inventory (E). To the right of the inventory there is a column of grey boxes, each with an icon, a white name and a grey time. From the top: "Regeneration II" (a few seconds, while it lasts), "Absorption 01:5x", "Night Vision 07:5x", "Hero of the Village 09:5x". The icons at the top right are gone. **Screenshot T1.**
- **TE-3.** Click the recipe book button. The inventory moves right and the boxes shrink to icons only. Hover one: a tooltip shows its name and time. **Screenshot T2.** Close the book: the full boxes return.
- **TE-4.** Close the inventory. Run `/effect give @s minecraft:speed 60`, `/effect give @s minecraft:haste 60 1`, `/effect give @s minecraft:strength 60` and `/effect give @s minecraft:luck infinite`. Open the inventory. With 7 or more effects the boxes overlap, packed into the same height. Luck shows "∞" and Haste shows "Haste II". **Screenshot T3.**
- **TE-5.** Place the chest and open it: the HUD icons stay at the top right with no list (vanilla). Run `/gamemode creative` and press E: the list sits beside the creative screen. **Screenshot T4.** Run `/gamemode survival`.
- **TE-6. Omen sounds and the bottle tooltip.**
  - Run `/effect clear @s`, then `/effect give @s minecraft:bad_omen 60`: the eerie omen sound plays. Run the same command again: no sound (the omen is already there).
  - Run `/effect give @s minecraft:trial_omen 60`: the trial omen sound. Run `/effect give @s minecraft:raid_omen 5`: the raid omen sound. Run `/effect clear @s`.
  - Open the inventory and hover the plain ominous bottle: "Ominous Bottle", then a blue "Bad Omen (01:40:00)". Hover the other bottle: "Bad Omen V (01:40:00)". **Screenshot T5.** Drink a milk bucket.

**Trading (host)**
- **TE-7.** Place the composter, fence a 5 x 5 pen around it and hatch a villager inside. Wait until it is a farmer. Right-click it and pick its crop offer (wheat, potatoes, carrots or beetroot: a 26.3 farmer picks 2 of 5 novice trades at random, so it may not want wheat, but it always wants one crop). The crop moves into the first payment slot and an emerald shows as the result. **In TE-8 to TE-11, "wheat" means that crop.** Keep hotbar slots 1 and 2 empty.
- **TE-8. Number keys.**
  - Hover the first payment slot and press 1: the wheat goes to hotbar slot 1 and the result disappears. Press 1 again: the wheat goes back and the result returns.
  - Hover the payment and press F: the wheat goes to the offhand. Press F again to bring it back.
  - Hover the result and press 2: one trade is made, the emerald lands in hotbar slot 2, and the payment shrinks by the price.
  - Press 2 again over the result: nothing happens, because a filled hotbar slot doesn't take the result. **Screenshot T6.**
- **TE-9. Q.** Hover the result and press Q: one trade, and the emerald is thrown in front of you. Press Ctrl+Q: it keeps trading and throwing until the wheat runs out or the offer is sold out (red cross). Emerald orbs drop as usual. Pick the wheat offer again. Hover the payment and press Q: one wheat is thrown. Ctrl+Q throws the rest of that stack. **Screenshot T7.**
- **TE-10. Drags.** Pick up a full wheat stack from the inventory (the kit has three of each crop; TE-7 to TE-9 use up about two). Hold the left button and drag across both payment slots and two empty inventory slots, then let go: 16 wheat lands in each. Pick up the rest of a stack and right-drag over the same slots: one more in each. The result updates from the new payments. **Screenshot T8.**
- **TE-11. Double-click.** Click a wheat stack in the inventory, then click the same slot again quickly: the stack is put back, and other wheat stacks are **not** gathered (vanilla on this screen). Do the same in the normal inventory (E): there it gathers.

**Friend (two windows, both near the farmer)**
- **TE-12.** The friend repeats TE-8 to TE-10 on the same farmer, after the host closes the screen. Number keys, F, Q, Ctrl+Q and drags work the same. Items the friend throws fly from the friend's eye; the host sees them land in front of the friend. **Screenshot T9**, both windows.
- **TE-13.** Away from the village, the friend drinks a night vision potion, and the host runs `/effect give <friend's name> minecraft:bad_omen 30` (commands need operator rights; the friend may run it as `@s` if the friend is an operator). The friend hears the omen sound, and the host, standing next to the friend, hears it too. The friend opens the inventory: the list shows beside it and the friend's HUD icons hide. **Screenshot T10**, the friend's window. The host runs `/effect clear <friend's name>`.

```
IW4L_MINECRAFT_KIT=minecraft:wheat*192,minecraft:potato*192,minecraft:carrot*192,minecraft:beetroot*192,minecraft:emerald*64,minecraft:villager_spawn_egg*4,minecraft:composter,minecraft:oak_fence*32,minecraft:torch*32,minecraft:golden_apple*8,minecraft:potion@long_night_vision,minecraft:potion@long_night_vision,minecraft:ominous_bottle*2,minecraft:ominous_bottle[ominous_bottle_amplifier=4],minecraft:milk_bucket,minecraft:milk_bucket,minecraft:bread*16,minecraft:chest
```

26 backpack slots: each `*192` fills three 64-stacks (`give_kit` splits a count above the stack size, as `stone*70` in the `commands.rs:2035` test). The friend gets the same kit (`/kit @a` from the host).

### Files shared

- `server.rs`: `MerchantOp`, `MerchantUpdate` (265-287) and `Server::merchant` (689-707).
- `minecraft_survival/src/survival.rs`: `give_effect`, `effect_sounds` (D's omen turn calls it; W's darkness may touch effects).
- `actions.rs` `act()`: the effect sound drain (W adds game events here).
- `frame/src/minecraft_ui.rs` `McStack`: `effects` and `Default`. Banner and dyed-armour tooltip lines should build on `Default`.
- `hud/src/minecraft_inventory.rs`, `minecraft_clicks.rs`, `minecraft_effects.rs`, `minecraft_hotbar.rs:95`; `render_anim/src/minecraft_inventory.rs` `convert`; `render_anim/src/minecraft_gui.rs`.
- Vendored `player/src/inventory.rs` (`queue_drop`), `entities/src/merchant.rs` and `world/merchants.rs`.
- `menus.rs` (`merchant_op`, `merchant_local`; E2 edits `open_station`) and `entities.rs` (`apply_merchant_answer`).

---

## E-horses. Horse falls, the gallop, vehicle hearts and dyed leather horse armour

### The owner's finding

> **Horses:** the mount is not drawn in its inventory screen, only one player can open a mount's inventory at a time, the rider takes no fall damage, there is no gallop sound and no vehicle hearts. Leather horse armour is undyed. (`docs/round9/release-2-tests.md:407`)

This section fixes four of these: fall damage, the gallop, vehicle hearts and dyed leather armour. The mount in its screen and the one-viewer limit are left out (see the end).

### 1. The rider takes no fall damage (and the horse takes the wrong damage)

**Cause**
- The horse uses the generic mob fall: `third_party/minecraftoss/entities/src/world/equines.rs:186-191` (`ride_tick`), `world.rs:7748` (still horse) and `world.rs:7933` (AI horse). All three call `living::fall_damage` (`world/living.rs:104-133`), which uses safe distance 3, multiplier 1 and the generic `small_fall`/`big_fall` sound. The horse constants `SAFE_FALL_DISTANCE = 6.0` and `FALL_DAMAGE_MULTIPLIER = 0.5` already exist in `horse.rs:556-558`, but nothing uses them.
- So a horse is hurt from a 4-block fall today, and a strong horse's full jump on flat ground (peak up to about 5.5 blocks) also hurts it.
- Nothing passes the landing on to the rider. While seated, the player's own fall is switched off: `crates/render_anim/src/minecraft_riding.rs:39-42` clears `steps.air_peak` every tick, and `counts_steps` (`minecraft_riding.rs:82`, used at `minecraft_players.rs:333-334`) stops step and fall counting. That part is correct: vanilla also resets the rider's own fall distance (`LivingEntity.rideTick` calls `resetFallDistance`).
- Mob riders: `world/riding.rs:567-571` (`end_ride_tick`) zeroes their fall too.

**Vanilla 26.3**
- `AbstractHorse.causeFallDamage(d, mult, src)`:
  1. If `d > 1`, play `entity.horse.land` (baby: `entity.baby_horse.land`) at volume 0.4 and pitch 1.
  2. `dmg = calculateFallDamage(d, mult)`. If `dmg <= 0`, stop: the riders take nothing.
  3. Otherwise `hurt(src, dmg)`, then `propagateFallToPassengers(d, mult, src)`, then `playBlockFallSound()`.
  4. There is no generic `small_fall`/`big_fall` sound for the horse.
- `createBaseHorseAttributes`: `SAFE_FALL_DISTANCE` 6.0, `FALL_DAMAGE_MULTIPLIER` 0.5, `JUMP_STRENGTH` 0.7, `STEP_HEIGHT` 1.0. Donkeys, mules and llamas inherit the fall values.
- `LivingEntity.calculateFallDamage`: `floor((d + 1e-6 - SAFE_FALL_DISTANCE) * mult * FALL_DAMAGE_MULTIPLIER)`. AbstractHorse does not override it, so jump strength is **not** part of the 26.3 formula.
- `Llama.causeFallDamage`: no land sound. It hurts and propagates only when `dmg > 0` and `d >= 6`, then plays the block fall sound. In practice the numbers are the same, because damage needs `d >= 8` anyway.
- `Entity.propagateFallToPassengers`: every passenger gets `causeFallDamage(d, mult, src)` with the **horse's** distance and block multiplier.
- `Player.causeFallDamage`: a flying-capable player (`mayfly`, creative) takes none. Otherwise safe distance 3, multiplier 1, the fall sound and the hurt.
- The rider's own fall distance is 0, so the death message is `death.attack.fall` ("hit the ground too hard").
- Numbers on stone (a bed halves the distance first, `living.rs` `fall_reduction`):

| Fall | Horse damage | Rider damage |
|---|---|---|
| 4-7 | 0 | 0 |
| 8 | 1 | 5 |
| 10 | 2 | 7 |
| 20 | 7 | 17 |
| 10 onto hay (×0.2) | 0 | 0 |
| onto slime (×0) | 0 | 0 |

**Plan: group `horse-fall` (M)**
1. **"Minecraft horses fall like vanilla horses"**
   - `living.rs`: a `pub(crate)` helper that returns the landed block's `(fall_reduction, fall_multiplier)` and its block fall sound.
   - `equines.rs`: new `horse_fall(horse, body, world, fallen, baby, voices) -> Option<f32>`. It plays the land sound when the reduced distance is above 1 (not for llamas), computes damage with `horse::SAFE_FALL_DISTANCE` and `FALL_DAMAGE_MULTIPLIER`, records `horse.pending_fall = Some((distance, multiplier))` only when damage is above 0, and then plays the block fall sound at half volume and ¾ pitch, with no generic small or big fall sound.
   - Use it at `equines.rs:190`, `world.rs:7748` and `world.rs:7933`. Each is a one-line swap of `fall_damage(...)` for `equines::cow_fall(entity.horse.as_mut(), ...)`, which falls back to `fall_damage` for cows. Keep the edit inside those lines.
   - `horse.rs`: `HorseState.pending_fall: Option<(f64, f32)>`, default `None`.
2. **"Minecraft riders share their horse's fall"**
   - `world/riding.rs`: `pub fn take_rider_falls(&mut self) -> Vec<(u64, f64, f32)>`. It drains `pending_fall` from every horse and, for each `Rider::Player` in `passengers_of`, returns `(player, distance, multiplier)`. Mob riders are skipped.
   - `server.rs`: a `rider_falls` field on the tick output beside `dismounts` (`:3659`), filled after the mob tick next to `take_vehicle_losses` (`:4260`).
   - `minecraft_survival/src/entities.rs`: route it like `dismounts` (`entities.rs:357-359`) into `PlayerOutputs.rider_falls` (`entities.rs:176`).
   - `minecraft_survival/src/survival.rs`: `Survival::rider_fall(distance, multiplier) -> i32`, with the same rule checks as `fall` (`survival.rs:483`: the gamerule, invulnerability, immunity, Feather Falling) and `fall_damage(distance, multiplier)` (`survival.rs:1146`). It records `DamageCause::Fall` with fall distance 0.
   - `render_anim/src/minecraft_players.rs` `struck()`: apply each rider fall and play `fall_sound(damage)` and `entity.player.hurt`, as the landing code does at lines 424-450. `struck()` runs for the host (`minecraft_world.rs:1245`) and every friend (`minecraft_players.rs:1661`).

**Tests**
- `world::equines::tests::a_ridden_horse_falling_ten_blocks_takes_two_and_hands_its_rider_the_fall`: `take_rider_falls` returns `(player, 10.0, 1.0)`.
- `a_horse_landing_from_seven_blocks_takes_nothing_and_hands_nothing_on`.
- `a_horse_landing_on_hay_spares_itself_and_its_rider`.
- `a_full_jump_on_flat_ground_never_hurts_a_strong_horse` (jump strength 1.0).
- `a_horse_landing_plays_the_horse_land_sound_but_no_generic_fall`.
- `a_llama_landing_makes_no_land_sound`.
- `survival::tests::a_rider_takes_the_horse_s_fall_from_three_blocks_up_and_hits_the_ground_too_hard`: 10 blocks gives 7, the death message key is `death.attack.fall`, and creative takes 0.
- `entities::tests::rider_falls_reach_the_host_and_the_friend`, in the style of `entities.rs:2816`.

### 2. No gallop sound

**Cause**
- `equines.rs:67-80` `horse_movement_sounds` only picks `entity.horse.step` / `entity.baby_horse.step`, or `entity.horse.step_wood` on wood (`world/emissions.rs:57-62`).
- `HorseState` (`horse.rs:166-208`) has no gallop counter, so a ridden horse sounds like a walking one.

**Vanilla 26.3**
- `AbstractHorse.playStepSound(pos, state)`: returns at once on a liquid block; snow on the block above swaps in snow's sound type.
  - **Carrying anyone (`isVehicle`) and `canGallop`:** `gallopSoundCounter++`. Above 5 and divisible by 3: `playGallopSound(type)`. At most 5: `entity.horse.step_wood` at `type.volume × 0.15` and `type.pitch`, even on grass. Otherwise silence.
  - **Carrying no one:** wood types (`WOOD`, `NETHER_WOOD`, `STEM`, `CHERRY_WOOD`, `BAMBOO_WOOD`) play `step_wood`; other blocks play `step` (baby: `entity.baby_horse.step`), all at `volume × 0.15`.
- `AbstractHorse.playGallopSound`: `entity.horse.gallop` at `type.volume × 0.15` and `type.pitch`.
- `Horse.playGallopSound` adds `random.nextInt(10) == 0` → `entity.horse.breathe` (baby: `entity.baby_horse.breathe`) at `type.volume × 0.6` and `type.pitch`. That is 4× the step volume our emission already carries (`movement.rs:208` multiplies by 0.15).
- `AbstractHorse` sets `canGallop = true`; `AbstractChestedHorse` sets it false, so donkeys, mules and llamas never gallop. `Llama.playStepSound` always plays `entity.llama.step` at 0.15 and pitch 1.
- `AbstractHorse.tickRidden`: the counter resets to 0 whenever the ridden input `z <= 0` (no forward key).
- All four events exist in the 26.3 `sounds.json`.

**Plan: group `horse-gallop` (S), one commit "Minecraft ridden horses gallop and breathe"**
- `horse.rs`: `gallop_counter: i32` and `HorseKind::can_gallop()` (true for `Horse` only; skeleton and zombie horses from track M join later).
- `emissions.rs`: turn `horse_step` into a small `HorseStep` enum: `Walk { step, wood }`, `Wood`, `Gallop { breathe }` and `Silent`. `play_movement` plays the gallop, then draws `random.next_int(10)` for the breath, at the block volume ÷ 0.15 × 0.6.
- `equines.rs`: `horse_movement_sounds(&mut HorseState, &Body, baby)` looks at `body.emission`. On a step with `horse.ridden && kind.can_gallop()`, it counts and picks `Wood`, `Gallop` or `Silent`.
- `ride_tick` resets the counter when `input.z <= 0`.
- `world.rs:7750` and `world.rs:7940`: change `&entity.horse` to `entity.horse.as_mut()` and pass the body. These are the only `world.rs` lines touched here.

**Tests**
- `world::equines::tests::a_ridden_horse_clops_five_times_then_gallops_every_third_step` (5× `step_wood`, then gallop on 6, 9, 12, silence in between).
- `stopping_resets_the_gallop`.
- `a_ridden_donkey_and_an_unridden_horse_only_step`.
- `a_gallop_sometimes_breathes` (seeded random, at the breath volume).

### 3. No vehicle hearts

**Cause**
- The HUD has no mount health. `crates/hud/src/minecraft_hotbar.rs:139-176` draws hearts, drumsticks and bubbles from `ui.vitals` only.
- `frame::MinecraftUi` (`crates/frame/src/minecraft_ui.rs:278`) carries `jump_bar` but nothing for the vehicle.
- `minecraft_net::PlayerView` (`crates/minecraft_net/src/messages.rs:481-502`) has no field for it.

**Vanilla 26.3 (`net.minecraft.client.gui.Hud`)**
- `getPlayerVehicleWithHealth`: the camera player's vehicle when it is a `LivingEntity`. Horses, donkeys, mules, llamas and striders all count.
- `getVehicleMaxHearts`: `(int)(maxHealth + 0.5) / 2`, capped at 30. A horse has 15-30 health, so 7-15 hearts; a strider has 20, so 10.
- `extractVehicleHealth`: `health = ceil(getHealth())`. Rows of up to 10 hearts start at `guiHeight - 39` and go **up** by 10. Heart `i` sits at `x = guiWidth/2 + 91 - i×8 - 9`. Each heart draws `hud/heart/vehicle_container`, plus `vehicle_full` when `2i + 1 + rowOffset < health` and `vehicle_half` when it equals `health`. `rowOffset` grows by 20 per row. It is called outside the `canHurtPlayer` check, so creative players see it too.
- `extractPlayerHealth`: if `getVehicleMaxHearts != 0`, the food bar is **not** drawn. The air row is `y - (ceil(hearts/10) - 1) × 10`, just above the top heart row.
- The sprites `hud/heart/vehicle_container|full|half` are in the 26.3 pack and already load through the `"hud/heart/"` prefix (`render_anim/src/minecraft_gui.rs:54`).

**Plan: group `vehicle-hearts` (M), two commits**
1. **"Minecraft riders see their mount's hearts in place of the food bar"**
   - `world/riding.rs`: `pub fn vehicle_health(&self, player) -> Option<(f32, f32)>`: `cow.health, cow.max_health` for the horse family, `strider.health, strider::MAX_HEALTH` for striders.
   - `minecraft_survival/src/player.rs`: `vehicle_health: Option<(f32, f32)>`, set next to `jump_bar` at `minecraft_players.rs:869`.
   - `frame/src/minecraft_ui.rs`: `MinecraftUi.vehicle_health: Option<(f32, f32)>`, next to `jump_bar`.
   - `hud/src/minecraft_vitals.rs`: `vehicle_hearts(health, max) -> Vec<(col, row, &'static str)>`.
   - `minecraft_hotbar.rs`: draw the hearts inside the `shows_hotbar` block but outside the `shows_vitals` filter. Skip the drumsticks while they show, and lift the bubbles by `(rows - 1) × 10`.
   - Host: `minecraft_world.rs:1260`.
2. **"Friends see their mount's hearts"**
   - `PlayerView.vehicle_health: Option<[f32; 2]>` (default `None`), filled in `minecraft_survival/src/views.rs:92` and read in `render_anim/src/minecraft_remote.rs:1299`.
   - No `PROTOCOL` bump in the commit (the release step bumps once).

**Tests**
- `hud::minecraft_vitals::tests::a_thirty_health_horse_shows_fifteen_vehicle_hearts_in_two_rows`, `a_fifteen_health_horse_shows_seven`, `a_half_heart_shows_vehicle_half`.
- `minecraft_hotbar` tests: vehicle hearts hide the drumsticks, and the bubbles move up one row for two heart rows.
- `world::riding::tests::vehicle_health_names_horse_and_strider_health_and_none_on_foot`.
- `minecraft_net::codec` round trip with `vehicle_health: Some([22.0, 26.0])`.

### 4. Leather horse armour is undyed

**Cause**
- The horse keeps only the item id: `HorseState.body_item: Option<String>` (`horse.rs:198`). The dye is lost at every step:
  - right-click equip: `equines.rs:767-768`;
  - the mount screen rebuilds the stack with `components: None` (`crates/minecraft_terrain/src/mount_inventory.rs:34-45`), and `body()` returns only the id (`:81-86`);
  - `set_mount_inventory` takes `Option<String>` (`equines.rs:807`, `server.rs:4134` `Command::MountInventory`);
  - death drop: `equines.rs:84`;
  - load: `server_mobs.rs:636` reads only the id;
  - save: `body_equipment` (`server_mobs.rs:1453-1470`) keeps the old tag when the id matches, so a different dye under the same id would come back wrong;
  - friends get the horse from that same tag through the look (`mob_tracking.rs:1199-1209`).
- On the horse, `horse_render.rs:53-61` gives leather the fixed tint `#A06540` and draws one layer only (`horse_render.rs:387-389`). The `leather_overlay` layer is missing.
- Icons and dropped items: `item_tint::of_stack` (`crates/minecraft_terrain/src/item_tint.rs:24-26`) keys only potion colours; `interface.rs:72-75` always uses the `minecraft:dye` default, and the test at `interface.rs:157` pins that. So every dyed leather item shows brown in inventories, in the hand (`render_anim/src/minecraft_hand.rs:195`) and on the ground (`mesh.rs:1006`).
- The kit's `@dye` (`crafting.rs:473`) checks the item tag `minecraft:dyeable`, and **that tag does not exist in 26.3** (not in the datapack and not in `client.jar`). So `leather_horse_armor@red` in a real kit gets `potion_contents` instead of a dye. Its test passes only because of a stand-in tag (`crafting.rs:872`).

**Vanilla 26.3 (data)**
- `assets/minecraft/equipment/leather.json` `horse_body`: 1. `minecraft:leather` with `dyeable.color_when_undyed = -6265536` (#A06540), tinted; 2. `minecraft:leather_overlay`, not tinted. Textures `textures/entity/equipment/horse_body/leather.png` and `leather_overlay.png`.
- `items/leather_horse_armor.json`: `tints: [{type: minecraft:dye, default: -6265536}]` on layer0 (`item/leather_horse_armor`). Layer1 `item/leather_horse_armor_overlay` is not tinted.
- The component is `minecraft:dyed_color`: an int, or `{rgb}`. `body_gear::dyed_color` (`body_gear.rs:68`) already reads both forms.
- Tag `#minecraft:cauldron_can_remove_dye`: the four leather armour pieces, `leather_horse_armor` and `wolf_armor`. That is the 26.3 list of dyeable items.
- Recipe `leather_horse_armor_dyed` is `minecraft:crafting_dye`, which our grid does not support (left out).
- The leatherworker level 4 trade `emerald_dyed_leather_horse_armor` uses `set_random_dyes`, which `trading.rs:495` already handles.

**Plan: group `dye-icons` (S), two commits (before `horse-dye`)**
1. **"Minecraft dyed items show their colour in icons, hands and on the ground"**
   - `item_tint::of_stack` keys `body_gear::dyed_color` when there is no potion colour. Split it as `pub fn of_stack_at(base: &str, stack: &ItemStack) -> String` (the base is the id or a dial key) with `of_stack(stack) = of_stack_at(&stack.id, stack)`.
   - `interface.rs`: `minecraft:dye` uses the keyed colour or else its default (split it out of the `dye|firework|map_color|team|custom_model_data` arm). Update the `interface.rs:157` assertion, which pinned the old limit (the owner's test policy allows this).
   - `render_anim/src/minecraft_hand.rs:195` and `icon_source` (`render_anim/src/minecraft_inventory.rs:771`) use `item_tint::of_stack_at(&dials.key(&stack.id), stack)`. **Do not switch them to plain `of_stack`:** today both build `item_tint::key(dials.key(..), potion)`, and dropping the dial key would freeze a held compass, recovery compass or clock. (E-banners' `player-looks` builds on this: `of_stack_at` carries the item look from `item-states` on, so the hand line needs no second change.)
   - Test: a held compass key still splits back to its dial model through `dials::split` after `item_tint::split`, and a red leather horse armour key carries `b02e26`.
2. **"Minecraft kits dye leather with the 26.3 dye tag"**
   - `crafting.rs:473` uses `minecraft:cauldron_can_remove_dye`, and the fixture at `crafting.rs:872` is renamed to match.

**Plan: group `horse-dye` (M), two commits**
1. **"Minecraft horses keep the dye of their leather armour"**
   - `HorseState.body_components: Option<serde_json::Value>`, next to `body_item`, so the many `body_item.as_deref()` sites stay unchanged.
   - Set it from the held stack on equip (`equines.rs:767`; the carpet path clears it).
   - `set_mount_inventory` and `Command::MountInventory` take the body as `Option<ItemStack>`. `MountInventory::of` builds the body stack with its components, and a new `body_stack()` sits beside `body()`. `minecraft_survival` `entities.rs:1025` passes it on.
   - `horse_death_equipment` drops the stack with its components.
   - Load the body's `components` (`tag_json`). `body_equipment` writes `components` with `json_tag` and keeps the old tag only when the id **and** the components match (caller at `server_mobs.rs:1741`).
   - The friend's copy follows through the look tag with no other change.
2. **"Minecraft dyed leather horse armour shows its colour on the horse"**
   - `horse_render.rs`: the tint is `dyed_color(body)` or else #A06540. Then draw the untinted `leather_overlay` over it in the translucent pass, as the markings are drawn (`horse_render.rs:385`), to avoid z-fighting.
   - `all_textures` adds `horse_body/leather_overlay`.

**Tests**
- `server_mobs::tests::a_horse_keeps_dyed_leather_armour_through_save_and_load` (`dyed_color` 0xB02E26 survives; a re-dyed armour under the same id saves its new colour).
- `mount_inventory::tests::the_body_slot_keeps_its_dye_both_ways`.
- `world::equines::tests::equipping_and_dropping_dyed_leather_keeps_the_colour`.
- `mob_tracking` look test: the friend's copy has `body_components`.
- `horse_render::tests::dyed_leather_tints_the_armour_and_draws_the_plain_overlay`.
- `item_icons::tests::a_red_leather_horse_armour_icon_is_red_on_layer_zero_and_plain_on_layer_one`.
- `crafting::tests::kit_dyes_leather_horse_armour_with_the_26_3_tag`: `leather_horse_armor@red` gets `minecraft:dyed_color` 0xB02E26 and no `potion_contents`.

### Left out (mention in the log and in MINECRAFT-SURVIVAL.md)

- **The mount is not drawn in its inventory screen.** Vanilla uses `InventoryScreen.extractEntityInInventoryFollowsMouse` (box `leftPos+26..78`, `topPos+18..70`, scale 17). We have no 3D-model-in-GUI pass at all; the player inventory has no figure either.
- **One viewer per mount** (`menus.rs:1972-1980`, `entities.rs:1034-1042`). Each viewer edits a private copy (`menus.rs:958-966` queues `mount_updates`, applied at `menus.rs:2002`). Two viewers would overwrite each other and could duplicate items. A real fix needs clicks applied against the world's container directly. Keep the lock.
- **Mob riders** (a skeleton or zombie on a horse) take no share of the horse's fall. There is no generic "hurt mob by id" yet; add it once D0's registry lands.
- **Dyeing leather in the crafting grid** (`minecraft:crafting_dye`) is not supported for any leather item. Dyed armour comes from the kit and leatherworker trades for now.

### The owner's steps

Get the kit below. Then `/gamemode survival` and `/time set day`.

**Vehicle hearts**
- **E-H1.** Use a horse spawn egg. Right-click the horse with an empty hand until it is tame (hearts and no more bucking). Right-click it with a saddle, then mount it. The food bar is replaced by a row of mount hearts on the right: 7 to 15 hearts, with a second row above for a horse with 22 or more maximum health (`getVehicleMaxHearts` is `(int)(max + 0.5) / 2`, so 21 health is still 10 hearts). Your own hearts stay on the left. **Screenshot H1.**
- **E-H2.** Dismount (crouch) and punch the horse once with an empty hand (1 damage), then mount it again: the mount hearts are half a heart lower. Dismount again: the food bar comes back. **Screenshot H2.**
- **E-H3.** Use the strider egg, saddle the strider and mount it holding the warped fungus on a stick: 10 mount hearts. **Screenshot H3.**

**Gallop**
- **E-H4.** Ride the tamed horse forward over grass holding W. The first five hoofbeats are hollow wooden clops. Then you hear a gallop every third step, and now and then a snort. Let go of W, then press it again: the wooden clops come back. Ride the same way on oak planks: the same pattern.
- **E-H5.** Use a donkey egg, tame, saddle and ride it: ordinary hoof steps, no gallop.

**Falls**
- **E-H6.** Stand on flat ground. Run `/fill ~3 ~ ~-3 ~9 ~9 ~3 stone` (a stone block 10 high), then `/tp @s ~6 ~10 ~`. Run `/summon minecraft:horse ~ ~ ~2 {Tame:1b}`, saddle it and ride it. Eat a golden apple if you are below full health, then wait until it wears off.
- **E-H7.** Ride straight off the edge. On landing you hear a heavy thud and the big fall sound. You lose 3.5 hearts (7 damage), and the mount hearts drop by one (2 damage). **Screenshot H4.**
- **E-H8.** Dismount. At a fresh spot on flat ground, run `/fill ~3 ~ ~-3 ~9 ~6 ~3 stone` (7 high), then `/tp @s ~6 ~7 ~` and `/summon minecraft:horse ~ ~ ~2 {Tame:1b}`. Saddle the new horse, ride it and ride off the edge. You hear the thud, but neither you nor the horse loses a heart. **Screenshot H5.**
- **E-H9.** Go back to the foot of the 10-high block, to the spot where you landed in E-H7. Standing there on the ground, run `/fill ~-1 ~-1 ~-1 ~1 ~-1 ~1 hay_block` (a 3 x 3 hay patch in the ground under you). Then `/tp` back onto the block as in E-H6, ride the E-H6 horse (or a new tame one) and ride off at the same place, onto the hay. No damage to either of you.
- **E-H10.** Jump with a full jump bar ten times on flat ground. The mount hearts never drop (before this fix, a strong horse hurt itself).

**Dyed leather armour**
- **E-H11.** Open the inventory (the kit fills the backpack, not the hotbar). The red, blue and lime leather horse armour icons are red, blue and lime, and the plain one is brown. Move the red one to the hotbar and hold it: it is red in your hand. **Screenshot H6.**
- **E-H12.** Right-click your tamed horse with the red armour. It wears red armour with plain brown buckles and trims on top. **Screenshot H7.**
- **E-H13.** Crouch and right-click the horse. The armour slot shows the red icon. Take it out: it is red in your inventory. Put the lime one in instead: the horse turns lime. **Screenshot H8.**
- **E-H14.** Save and quit, then load. The horse still wears lime. Dismount and kill the horse with the iron sword (`/kill` takes only players in the fork): the dropped armour on the ground is lime, and so is the item when picked up. **Screenshot H9.**

**Friend (two windows)**
- **E-H15.** The friend tames, saddles and rides their own horse. The friend's HUD shows mount hearts instead of the food bar. **Screenshot H10**, the friend's window.
- **E-H16.** The friend rides off the 10-high block. The friend loses 3.5 hearts and hears the fall, and the host hears the horse land and gallop. **Screenshot H11**, both windows.
- **E-H17.** The host puts blue armour on the host's horse, and the friend puts lime on theirs. Each window shows both colours correctly. **Screenshot H12**, both windows.
- **E-H18.** The friend needs the new build: the protocol changed (vehicle hearts). After the release step bumps `PROTOCOL`, an old friend build is refused at `Hello`.

```
IW4L_MINECRAFT_KIT=minecraft:horse_spawn_egg*4,minecraft:donkey_spawn_egg*2,minecraft:strider_spawn_egg*2,minecraft:warped_fungus_on_a_stick,minecraft:saddle*4,minecraft:leather_horse_armor@red,minecraft:leather_horse_armor@blue,minecraft:leather_horse_armor@lime,minecraft:leather_horse_armor,minecraft:iron_horse_armor,minecraft:golden_carrot*16,minecraft:hay_block*16,minecraft:oak_planks*64,minecraft:stone*64,minecraft:water_bucket,minecraft:golden_apple*4,minecraft:cooked_beef*16,minecraft:iron_sword,minecraft:lead*2
```

22 backpack slots (saddles do not stack, so `saddle*4` takes four). The `@red`/`@blue`/`@lime` dyes need commit 2 of `dye-icons`; before it, those three come out undyed.

### Files shared

- Vendored `world.rs` (one line at 7748, and 7750, 7933, 7940): D0, M1-M3, W, D, E2 (other regions).
- `horse.rs`, `world/equines.rs`, `world/riding.rs`: the M track that adds skeleton horses, zombie horses and camels (all `AbstractHorse`; skeleton and zombie horses gallop); D0 may touch `riding.rs`; E2 adds one push line at `equines.rs:532`.
- `world/emissions.rs` and `world/living.rs`: M tracks and W.
- `server.rs` (tick output field) and `server_mobs.rs` (horse load and save): D0, M, D; E2 adds `chest_open_count` to the `MobWorld` impl elsewhere in `server_mobs.rs`.
- `entities.rs` (`PlayerOutputs`) and `render_anim` `minecraft_players.rs` (`struck`): D0, W, D.
- `minecraft_net` `messages.rs`/`codec.rs` (`PlayerView`): F, W.
- `frame/src/minecraft_ui.rs` and `hud/src/minecraft_hotbar.rs`: E-effects (same track), F (boss bar), E2 (`held_map`).
- `interface.rs`, `item_tint.rs`: E-banners (same track).

---

## E-banners. Banner patterns and crossbow states

### The owner's findings

- `docs/round9/release-2-tests.md` (Known limits, Illagers): "banners show plain white (no patterns), a held crossbow shows no arrow or pull".
- `docs/MINECRAFT-SURVIVAL.md:401`: "There are no banner or shield patterns". Line 385: "the banners on tower tops have no patterns" (end cities). Line 456: "the crossbow is drawn uncharged (only the arm poses show loading and aiming)" (piglins).
- `docs/round9/track-d-log.md` (Left for later): "Banner patterns are not drawn (placed banners, held banners and the captain's ominous banner show their base colour only)". Also: "A mob's held crossbow is drawn with its plain item model: a loaded crossbow shows no arrow and a charging one no pull (`mesh::HeldItem` carries no components)".
- Found while reading the code: **a placed banner drops nothing when broken in survival.** The banner loot table's `copy_components` makes the player loot engine give up (Causes, banners, point 6).

### How item models are chosen today

- `special_models.rs:476` `collect_leaves` is the main walker. It is used by `model.rs:138-176` (`resolve_item_model`, `item_geometry` at :163, which always passes `using_item = false` at :165) and by the display transforms (`model.rs` `item_display_model`).
  - `condition` (:530) reads only `minecraft:using_item`, from a bool. No caller passes true except the hand transform.
  - `select` and `range_dispatch` (:542) honour only `minecraft:display_context`. Otherwise they take `fallback` first, then the entries in order. No property value and no threshold is read.
  - Result: `items/crossbow.json` (select `charge_type` → fallback → condition `using_item` → on_false) always gives `item/crossbow`, and `items/bow.json` always gives `item/bow`.
- `dials.rs:207` `Dials::leaf` is the only code that reads thresholds: the last entry with `threshold <= value`, else `fallback` (:230-245). It does this only for `minecraft:compass` and `minecraft:time` (`value`, :198), and only for the three `DIAL_ITEMS` (:9). Its `condition` always takes `on_false` (:210). Track D's lodestone compass will want `has_component` here, so **E does not edit `dials.rs`**. Dial results travel as keys of the form `id#model` (`dials.rs:22-31`); `mesh.rs:930` and `item_icons.rs:22` split these keys and call `model::resolve_item_leaf`.
- A drawn item is a string key: `item_tint::of_stack` (`item_tint.rs:24`, id plus `~rrggbb` potion colour, plus the dye from `dye-icons`), optionally wrapped by a dial key.
  - The key builders are: dropped items (`mesh.rs:1006`), inventory icons (`minecraft_inventory.rs:462`, `:664`, `icon_source` at `:769`), the first-person hand (`minecraft_hand.rs:195`), the third-person body (`body_render.rs:33` from `GearItem`), item frames (`mesh.rs:1267` from `FrameView.item` and `.potion`, built at `item_frame.rs:363`) and every mob's `HeldItem.id`.
  - The mob held items are `illager_render.rs:348` (head) and `:374` (hands), `piglin_render.rs:669`, skeletons and zombies.
  - `HeldItem` (`mesh.rs:1352`) carries only that string. `ItemVisuals::model` (`mesh.rs:927`) caches by key and takes tints from the item definition plus the potion.

### Causes, banners

1. `special_models.rs:369` `item_model(special)` builds `minecraft:banner` and `minecraft:shield` without the stack. `push_banner` (:934) draws one flag tinted with the base colour. `push_shield` (:1083) always uses `entity/shield/shield_base_nopattern`.
2. `special_models.rs:430` `special_tints` gives only the base colour. `Sheet` has a `tinted: bool` that always becomes `tint_index: Some(0)` (:824), so a model cannot hold more than one tint.
3. The atlas holds only `entity/banner/banner_base`, `entity/banner/base` and `entity/shield/shield_base_nopattern` (`special_models.rs` `textures()`, used at `mesh.rs:483`). None of the 43 pattern textures is loaded.
4. Placed banners are baked into section meshes from the block state alone: `special_models.rs:294-304` and the `TintKind::Dye` at `mesh.rs:2077`. The block entity's `patterns` never reach the mesher. The block entity does keep them (`core/src/block_entity.rs:186` keeps `patterns` and `CustomName`, and :104 keeps `components`), and friends receive them (whole chunks via `chunk_wire.rs`, and `ToClient::BlockEntity`, `session.rs:400`, `remote.rs:413`).
5. Placing a banner item writes only the block state (`actions.rs:1495-1508`, `level.rs:172`). Nothing copies `banner_patterns` or `custom_name` into the block entity (vanilla `applyImplicitComponents`). Only shulker boxes get their components (`containers.place_shulker`).
6. Breaking a banner:
   - `third_party/minecraftoss/player/src/loot.rs:337` returns `None` for any `copy_components` whose `include` lists more than `custom_name`. The `?` at :304 then voids the whole roll, so `entities.rs:741` drops nothing.
   - Even with loot, the `kept` components are matched by `drop.id == block.id` (`entities.rs:753`), which fails for `*_wall_banner` → `*_banner`.
   - `kept` is built only for shulker boxes (`actions.rs:1656-1667`; blasts at `minecraft_levels.rs:485` and `minecraft_world.rs:1200`).
7. Commands and kits rejected the components; E-kit fixes that.
8. The shield decoration recipe (`crafting_special_shielddecoration`) is not implemented, and there is no loom (`menus.rs:1907`). In survival, the only sources of patterns are captains' ominous banners and structure banners (end cities). The shield recipe is built in **E2** (`fw-craft`), next to the firework recipes.

### Causes, crossbows

1. The walker never evaluates `charge_type` or `crossbow/pull`.
2. Mob held items pass only `stack.id` (`illager_render.rs:374`, `piglin_render.rs:669`). The data is all there: the stack's `minecraft:charged_projectiles` is set by `illager.rs:322` and `piglin.rs:286`; `entity.using_item` holds the ticks used (`illager_render.rs:332`, piglins the same). Friends get both: `using_item` through `MobShow` (`mob_tracking.rs:436`), and the equipment through the look tag, re-sent when the equipment changes (`mob_tracking.rs:266-270`).
3. Players: `GearItem` (`body_gear.rs:9`, wire copy `minecraft_net/src/messages.rs:615`) has no field for the item's components, so third-person and friends' views cannot show a loaded crossbow or patterns.
   - **Players never load or pull a crossbow in this fork.** Bows and crossbows have no player use (`offhand.rs:21`, `:93-105`), because MW2 guns fill that role. A player's crossbow can only be loaded by `/give`, a kit, or picking up a pillager's dropped loaded crossbow.

### Vanilla 26.3

- **BannerRenderer**: `MAX_PATTERNS` 16, `SIZE` 0.6666667. Model translation (0.5, 0, 0.5) and scale (0.6666667, -0.6666667, -0.6666667), turned by `RotationSegment.convertToDegrees(rotation)` (ground) or `Direction.toYRot` (wall).
  - Order of drawing: `Sheets.BANNER_BASE` (`entity/banner/banner_base`, pole and bar), then the flag, then `submitPatterns`.
  - `submitPatterns` draws the base layer `Sheets.BANNER_PATTERN_BASE` (`entity/banner/base`) tinted with `DyeColor.getTextureDiffuseColor` of the base colour, then up to 16 layers `entity/banner/<asset_id path>`, each tinted with its own colour (the `DYES` values already at `special_models.rs:19`).
  - Each layer is the same flag model and goes through `RenderTypes.bannerPattern` (translucent), in order.
  - Wave (not in scope): phase `(floorMod(x*7 + y*9 + z*13 + gameTime, 100) + partial) / 100`.
- **ShieldSpecialRenderer**: the plate and handle use `Sheets.SHIELD_BASE` (`entity/shield/shield_base`) when `banner_patterns` is non-empty or `base_color` is set, otherwise `SHIELD_BASE_NO_PATTERN`. When patterned, `submitPatterns` runs on the plate with shield sprites: `entity/shield/base` tinted with the base colour (`WHITE` when unset), then `entity/shield/<asset>` per layer.
- **BannerBlockEntity**: saves `patterns` (only when not empty) and `CustomName`. `applyImplicitComponents` and `collectImplicitComponents` handle `BANNER_PATTERNS` and `CUSTOM_NAME`. Every other component of the placed stack stays in the block entity's `components` (the base `BlockEntity` does this).
- **Loot** `blocks/white_banner` (all 16 colours): `copy_components` from `block_entity`, including `custom_name`, `item_name`, `tooltip_display`, `banner_patterns` and `rarity`. Condition: `survives_explosion`.
- **Registry** `data/minecraft/banner_pattern/`: 43 entries. Every `asset_id` equals its id. Inline patterns carry `{asset_id, translation_key}`. The pack's `textures/entity/banner/` holds 44 PNGs (the 43 plus `banner_base`), and `textures/entity/shield/` holds 45 (the 43 plus `shield_base` and `shield_base_nopattern`). Atlases: `atlases/banner_patterns.json` and `shield_patterns.json`, both directory sources.
- **Ominous banner** (`Raid.getBannerComponentPatch`, already ported as `world/natural_spawner/spawning.rs` `ominous_banner()`): white, with layers rhombus cyan, stripe_bottom light_gray, stripe_center gray, border light_gray, stripe_middle black, half_horizontal light_gray, circle light_gray, border black. It also has `item_name` `block.minecraft.ominous_banner`, `rarity` uncommon, and `tooltip_display` hiding the patterns.
- **Crossbow** `items/crossbow.json`: select `minecraft:charge_type`: `arrow` → `item/crossbow_arrow`, `rocket` → `item/crossbow_firework`. Fallback: condition `minecraft:using_item`. on_false → `item/crossbow`. on_true → range_dispatch `minecraft:crossbow/pull`: fallback `crossbow_pulling_0`, from 0.58 `crossbow_pulling_1`, from 1.0 `crossbow_pulling_2`. All six models share the `display` block of `item/crossbow`.
- **Charge.get**: null or empty `charged_projectiles` → NONE. Contains `firework_rocket` → ROCKET. Otherwise ARROW.
- **CrossbowPull.get**: 0 with no living owner or when the crossbow is charged. Otherwise `UseDuration.useDuration` (whole ticks used) divided by `CrossbowItem.getChargeDuration` (25 ticks, 5 fewer per Quick Charge level).
- **IsUsingItem**: `entity.isUsingItem() && entity.getUseItem() == stack`. Only the stack in the using hand counts.
- **Bow** `items/bow.json`: condition `using_item` → range_dispatch `minecraft:use_duration` with scale 0.05: fallback `bow_pulling_0`, 0.65 `bow_pulling_1`, 0.9 `bow_pulling_2`.
- **Fork's own charge**: `CROSSBOW_CHARGE` 25 (`illager.rs:28`, `piglin_brain/fight.rs:17`). The AI ignores Quick Charge, so the model pull uses the same 25.

### Plan (five groups, in order)

**Group `item-states` (M), two commits** (needs E-kit for its kit only).
1. **"Minecraft item models pick crossbow and bow states like vanilla"**
   - New `crates/minecraft_terrain/src/item_look.rs`:
     - `ItemLook { charge: Charge (None, Arrow, Rocket), use_ticks: Option<u32>, base: Option<dye>, layers: Vec<(asset, dye)> }` (up to 16 layers, unknown patterns skipped).
     - `ItemLook::of(stack components, use_ticks)`.
     - One key encode and split pair that sits between the id and the dial and potion/dye suffixes. `item_tint::split` and `dials::split` must still work on the result. Names (`item_icons::item_name`, `minecraft_inventory.rs` `ui.names` and `icon_source` at `:769`) stay keyed by the plain id.
   - `item_tint::of_stack_at` (from `dye-icons`; `of_stack` calls it) appends the look, so the hand and `icon_source` keep their dial keys. `special_models.rs:476` `collect_leaves` takes an item state (using flag, charge, use ticks, charge duration):
     - `select` `minecraft:charge_type`;
     - `range_dispatch` `minecraft:crossbow/pull` and `minecraft:use_duration` (times `scale`), with the threshold rule of `dials.rs:230-245`;
     - `condition` `minecraft:using_item`.
     - Keep the `item_leaves(model, bool)` wrappers for the current callers.
   - `mesh.rs:927` `ItemVisuals::model` and `item_icons.rs:22` split the look and resolve the chosen leaf with `model::resolve_item_leaf`.
2. **"Minecraft pillagers and piglins show their crossbow's pull and loaded arrow"**
   - `illager_render.rs:348` and `:374`, and `piglin_render.rs:669`: key each held stack with `ItemLook::of`. The use ticks go only on the using hand (`Illager::crossbow_hand`, the piglin's crossbow hand) while `using_item` is `Some`.
   - Whole ticks, as vanilla does: the key cache grows by at most 26 entries.
   - The captain's head banner is keyed the same way (its patterns show from `banner-items` on).

Tests:
- `special_models`: the real crossbow and bow tree shapes (like the shield test at :1459). No state → `crossbow`. Using for 0, 15 and 25 ticks → `pulling_0`, `pulling_1`, `pulling_2`. Arrow → `crossbow_arrow` even while using. Rocket → `crossbow_firework`. Bow at 13 and 18 ticks → `pulling_1` and `pulling_2`.
- `item_look`: keys split back with potion, dye and dial combinations.
- With the real pack (skipped without it): a loaded crossbow's icon uses the `crossbow_arrow` texture.
- Illager and piglin held keys while charging and when loaded.
- `mob_tracking`: a friend's copy keys the same after the look event.

**Group `banner-items` (M), one commit.**
1. **"Minecraft banners and shields show their patterns in hand, on heads, on the ground and as icons"**
   - `mesh.rs:483`: add every `textures/entity/banner/` and `textures/entity/shield/` texture via `packs.list`.
   - `special_models.rs`:
     - `Sheet` gets a tint index instead of a bool (:87-107, :824).
     - `push_banner` (:934) adds one flag cube per layer after the base, with texture `entity/banner/<asset>` and tint index i+1 (at most 16).
     - `push_shield` (:1083) switches to `shield_base` plus the plate layers (`entity/shield/base` at tint 0, white by default, then the layers) when there is a base colour or patterns; otherwise it keeps `shield_base_nopattern`.
     - `item_model(special, look)` (:369); `special_tints` (:430) gives [base, layer colours...].
   - `ItemVisuals::model` uses the look's tints.
   - Layers draw in submit order in the same mesh, so coplanar faces resolve to the later layer under `GreaterEqual`. They are cut out at alpha 0.1 like every entity texture (`render_gpu .../minecraft_world.rs:1281`).
   - The captain's head banner now shows the ominous pattern.

Tests:
- A patterned banner item has the base plus N layer cubes with the right textures and tints; layers above 16 are dropped; unknown patterns are skipped.
- The ominous banner from `ominous_banner()` gives 8 layers.
- Shield with patterns, with only a base colour, and plain.
- With the real pack: the atlas holds all 43 banner and 43 shield pattern textures, and the ominous banner icon differs from the plain one.

**Group `banner-roundtrip` (M), two commits.**
1. **"Minecraft banners keep their patterns and name when placed"**
   - A generic server command, for example `ServerSim::block_entity_components(pos, components)`, inserted next to the lectern command, into a new vendored `Level` function next to `level/lectern.rs:79`. It uses `ensure_block_entity` and `set_block_entity_value`.
   - For banners it writes `patterns` from `banner_patterns` (only when non-empty) and `CustomName` from `custom_name`, as `BannerBlockEntity.applyImplicitComponents` does, and puts every other component in `components`. Name it generically so track D can reuse it for placed spawners and vaults.
   - Call it from `actions.rs` after `write_blocks`, beside `place_shulker` (:1499-1508), for any `#minecraft:banners` item with components.
2. **"Minecraft banners drop with their patterns and name"**
   - `third_party/minecraftoss/player/src/loot.rs:337`: `copy_components` from `block_entity` stops voiding the roll (it copies nothing itself). (E2's maps edit a different file, `core/src/loot.rs`.)
   - `actions.rs:1656` and the two blast sites (`render_anim` `minecraft_levels.rs:485`, `minecraft_world.rs:1200`) build `kept` for banners from the block entity tag before the break: `banner_patterns`, `custom_name`, and `item_name`, `tooltip_display` and `rarity` from `components`. Read the tag from the stream's loaded chunk, as `session.rs:627` does.
   - `entities.rs:753` applies kept components to the block's loot drop (the standing banner for a wall banner) instead of matching `drop.id == block.id`. The extra boxed stack stays shulker-only.

Tests:
- With real data: a placed ominous banner's block entity holds the 8 patterns and `components` with `item_name`, `rarity` and `tooltip_display`.
- Breaking it standing, on a wall and by TNT drops one "Ominous Banner" with the same components.
- A plain banner drops a plain banner (the regression for today's empty drop). Creative breaking drops nothing.
- `session::block_updates` sends the patterned tag to friends.

**Group `player-looks` (S), one commit.**
1. **"Minecraft players and friends show loaded crossbows and patterned banners in their hands"**
   - `body_gear.rs:9` `GearItem` gets `look: String` (the `item_look` key part), filled in `GearItem::of`. The wire copy is `minecraft_net/src/messages.rs:615`, mapped in `minecraft_survival/src/gear.rs:14-30`. No `PROTOCOL` bump in the commit.
   - `body_render.rs:33` and `worn_head.rs` key with the look. The first-person hand (`minecraft_hand.rs:195`) already uses `of_stack_at` from `dye-icons`, which now carries the look (no use ticks for players).
   - `FrameView` (`item_frame.rs:147`, :363) carries the look for `mesh.rs:1267`.
   - `docs/MULTIPLAYER.md`: one line on `GearItem.look`.

Tests: `GearItem::of` for a loaded crossbow and an ominous banner; a codec round trip with the look; `gear_view`/`gear_of` keep it; a frame view of a patterned banner.

**Group `placed-banners` (M), one commit.**
1. **"Minecraft placed banners show their patterns"**
   - New `crates/minecraft_terrain/src/banner_look.rs`, modelled on `spawner_display.rs:505-545` `SpawnerDisplays`:
     - `BannerLooks` with `load_chunk`, `unload_chunk` and `block_entities(changes)`. It reads `patterns` in both forms (pattern id or inline `{asset_id}`, colour) and takes the base colour from the block id (`white_wall_banner` → white). It keeps only banners with at least one layer.
     - `take_apart_changes()` feeds `lids::ApartCells` via `Terrain::draw_apart` (`terrain.rs:1541`), as open chest lids do.
     - `posed(scene, ...)` uses the pose of `special_models.rs:294-304` (translate 0.5,0,0.5, turn by rotation segment or wall facing, scale 2/3,-2/3,-2/3) plus the layer cubes from `banner-items`.
   - `EntityView` (`entity_view.rs:24`) holds the store and draws it into `out.items` next to `append_open_lids` (:1288), lit at the cell, using `mesh::append_elements`.
   - Feeds: host at `entities.rs:698-705` (load and unload) and `:1640` (`output.block_entities`); friend at `remote.rs:417` (`BlockEntity` message), `:651` (chunk batch load and forget) and `:708` (a block that stops being a banner removes its entry).
   - Draw-apart loops: `render_anim` `minecraft_levels.rs:536`, `minecraft_world.rs:1518` and `minecraft_remote.rs:1971`.
   - End city tower banners and any structure banner now show their patterns.

Tests: both pattern forms; the posed model equals the block model for a ground and a wall banner plus the layers; apart changes only for patterned banners; the store follows chunk loads and block entity changes; a terrain test that an apart banner leaves its section cell empty (like `terrain.rs:2337`); a friend-side test that a `BlockEntity` message with patterns gives a posed banner.

### Decisions (for the log)

- Pattern layers are cut out at alpha 0.1, like every entity texture here. Vanilla blends them (`bannerPattern`), so the gradient patterns get a hard edge.
- Patterned placed banners are drawn apart from the section mesh, as moving lids are. Plain banners stay baked. No banner waves yet: the vanilla phase formula is noted for later.
- Players have no crossbow use in the fork, so a player's crossbow shows standby, arrow or rocket and never the pull. Quick Charge changes neither the mobs' loading nor the pull (25 ticks, as the AI uses).
- There is no loom and no banner duplicate recipe. Patterns come from captains, structures, commands, kits and E2's shield recipe.
- Not done here, but the same evaluator can do it: skeletons' bow pull (`skeleton_render.rs:173`) and the drowned's raised trident. Both belong to the mob tracks.

### The owner's steps (host, then friend)

Use a new world with the kit below. `item-states` and `player-looks` must be built for the crossbow steps, `banner-items` and `player-looks` for the held banners, and `banner-roundtrip` and `placed-banners` for the placed banners.

- **BC-1.** Open the inventory: the white banner shows a black creeper face with a red border, the blue banner shows a white fade with a lime globe and a yellow bottom stripe, and the first shield is red with a yellow cross. One crossbow shows an arrow loaded and one a firework. **Screenshot BC1.**
- **BC-2.** Hold the patterned white banner, then the arrow crossbow, then the firework crossbow, each in first person. **Screenshot BC2.** Press F5 for third person with each one. **Screenshot BC3.** Hold the patterned shield in the offhand and right-click to raise it: the cross shows on the raised shield. **Screenshot BC4.**
- **BC-3.** Press Q to drop the patterned banner and the loaded crossbow: on the ground the banner shows its pattern and the crossbow its arrow. Pick them up again.
- **BC-4.** Build a small wall of planks. Place the white banner on the ground and the blue banner on the wall. Both show their patterns from the front and the back, in daylight and by torchlight. **Screenshot BC5.**
- **BC-5.** `/gamemode survival` and break both banners with the axe: each drops a banner with the same pattern (in release 1 nothing drops at all). Place three more a few blocks apart and blow them up with TNT. A blast keeps each drop only one time in four (`entities.rs:744-748`, chance `1 / power`), so expect about one banner; every banner that drops keeps its pattern. **Screenshot BC6.**
- **BC-6. (E2's `fw-craft`.)** Put a black banner and the plain shield into the crafting table: the result is a black shield. Put the white patterned banner and the second plain shield in instead: the shield gets the creeper and border on white. A shield that already has a pattern does not combine. **Screenshot BC7.**
- **BC-7.** On open ground use pillager eggs until one is a captain (about one in 17): the banner on its head shows the ominous pattern (cyan diamond, grey and black stripes, black border). **Screenshot BC8.** Kill it: the dropped "Ominous Banner" shows the pattern on the ground and as an icon. Place it and break it: it drops as "Ominous Banner" again.
- **BC-8.** Stand about 12 blocks from a pillager in survival: while it loads, the string on its crossbow pulls back in three steps. When loaded, the arrow lies in the crossbow until it shoots, and then the crossbow is empty again. **Screenshot BC9.** Put an item frame on the wall and put the ominous banner in it: the frame shows the pattern.
- **BC-9.** Optional: `/execute in minecraft:the_end run locate structure minecraft:end_city`, travel there, and look at a tower top: its banners show their black patterns on magenta. **Screenshot BC10.**
- **BC-10. Friend window.** The friend sees the host's placed banners with their patterns, the patterned banner and the loaded crossbow in the host's hand, the raised patterned shield, the captain's banner, and a pillager's crossbow pulling and then loaded. **Screenshot BC11.** The host gives the friend a patterned banner (`/kit @a`, or `/give <friend's name>` with the white banner entry of the kit), the friend places it, and the host sees the pattern. **Screenshot BC12.**

```
IW4L_MINECRAFT_KIT=minecraft:white_banner[banner_patterns=[{pattern:"minecraft:creeper",color:"black"},{pattern:"minecraft:border",color:"red"}]]*4,minecraft:blue_banner[banner_patterns=[{pattern:"minecraft:gradient",color:"white"},{pattern:"minecraft:globe",color:"lime"},{pattern:"minecraft:stripe_bottom",color:"yellow"}]]*4,minecraft:black_banner*4,minecraft:red_banner*2,minecraft:shield[base_color=red,banner_patterns=[{pattern:"minecraft:cross",color:"yellow"}]],minecraft:shield*2,minecraft:crossbow[charged_projectiles=[{id:"minecraft:arrow"}]],minecraft:crossbow[charged_projectiles=[{id:"minecraft:firework_rocket"}]],minecraft:crossbow,minecraft:pillager_spawn_egg*16,minecraft:crafting_table,minecraft:oak_planks*64,minecraft:diamond_axe,minecraft:diamond_sword,minecraft:tnt*4,minecraft:flint_and_steel,minecraft:item_frame*4,minecraft:diamond_chestplate,minecraft:cooked_beef*32
```

20 backpack slots (`shield*2` takes two: BC-6 uses both plain shields). The kit needs E-kit to parse these components. All ids are checked against the 26.3 `items/` folder.

### Files shared

- `mesh.rs` (atlas list, `ItemVisuals::model`): D0, M1-M3, W and F add textures.
- `special_models.rs`, `mesh.rs` `append_open_lids`, `lids.rs`: E-bells (same track).
- `illager_render.rs` and `piglin_render.rs`: D0's registry.
- `entity_view.rs`, `entities.rs` and `remote.rs` (block entity feeds): D (spawners, vaults), W (sculk), D0.
- `render_anim` `minecraft_levels.rs`, `minecraft_world.rs` and `minecraft_remote.rs`: everyone.
- `server.rs` (new command) and vendored `world/src/level/`: D and W.
- `minecraft_net` `messages.rs`/`codec.rs` (`GearItem.look`).
- Vendored `player/src/loot.rs`: E1 only.
- `actions.rs` (placement and `kept`): E2 edits other functions.
- Docs: `docs/MULTIPLAYER.md` and `docs/MINECRAFT-SURVIVAL.md` (lines 385, 401 and 456 and the Illagers known limit).
- **Not touched:** `dials.rs` (track D's lodestones).

---

## E-bells. Bells swing when rung; the glow outline

The projectile half of the round 9 bell limit (arrows and wind charges ringing bells) is **E2's E-bell-hits**.

### The owner's finding

From `docs/round9/release-2-tests.md`, Known limits:

> **Bells** don't swing, and arrows and wind charges don't ring them. The glow is a white hull, not vanilla's outline.

Since release 2, track D's breezes group (f142f9c1) already rings a bell from a wind charge **burst**: `redstone.rs:954-956` sends `Kind::Bell` to `ring_bell(pos, None)`. Bells still don't swing, no projectile rings a bell by hitting it (E2), and the glow is still the hull.

### Cause in the current code

- **Static bell body.** `special_models.rs:1008-1021` `push_bell` bakes vanilla's `BellModel` body and lip into the chunk mesh with no rotation. It is appended to every bell variant through `extra_elements` (`special_models.rs:335-338`) in `model.rs:381` and `model.rs:441-452`. Nothing ever re-poses it.
- **No ring reaches a client.**
  - On the server, a ring is `Level::ring_bell` (`world/src/level/redstone.rs:776-787`). It queues block event 1 with the direction's 3D value and plays `block.bell.use` at volume 2.
  - `piston.rs:386` turns that event into `level.bell_rings`.
  - `ServerSim::hear_bells` (`minecraft_terrain/src/server.rs:1369-1374`, called from `ServerSim::tick` at `:3157`) hands it only to `Bells::ring` (`bells.rs:106-118`), which is server state (listeners, resonance, glow).
  - No block event leaves the level thread. `Output` (`server.rs:3619-3650`, drained at `:4288-4291`) has `level_events` but no block events.
- **The animation path exists but bells aren't in it.**
  - The client already animates chest and shulker lids from block event 1. Host: `Entities::lid_event` (`minecraft_survival/src/entities.rs:566-570`) feeds `view.lids` and sends `Heard::Block` to friends. Friend: `ToClient::BlockEvent` goes to `lids.block_event` (`render_anim/src/minecraft_remote.rs:928-935`).
  - `Lids` (`minecraft_terrain/src/lids.rs:209-324`) draws a moving block apart from its section. `take_apart_changes` feeds `stream.draw_apart` (`render_anim/src/minecraft_levels.rs:536`, `minecraft_remote.rs:1971`, `minecraft_world.rs:1518`). `terrain.rs:746` skips apart cells. `entity_view.rs:1288-1292` poses the block every frame through `ItemVisuals::append_open_lids` (`mesh.rs:1178-1196`).
  - `LidKind::of` (`lids.rs:20-31`) knows only shulkers and chests, so a bell's event is dropped at `lids.rs:227`.
- **The glow.** `glow_render.rs:3-19` copies a glowing mob's model as a white hull 1/16 bigger on each side, plus a cover copy. The glowing mobs are pillagers, vindicators and evokers, ravagers and witches (`entity_view.rs:1108-1121`). `render_gpu/src/drawsurf/minecraft_world.rs:877-884` draws the hull with `CompareFunction::Always` (`:1716`, `glow_fragment` at `:1291-1297`), then the cover with the normal depth test. A hidden raider therefore shows as a solid white shape, while vanilla shows only its outline.

### Vanilla 26.3

- **`BellBlockEntity.triggerEvent(1, b)`**, on the server and on every client within 64 blocks of the block event: `updateEntities`, then `resonationTicks = 0`; `clickDirection = Direction.from3DDataValue(b)` (0 down, 1 up, 2 north, 3 south, 4 west, 5 east); `ticks = 0`, `shaking = true`. A ring during a swing restarts it from 0.
- **`clientTick` and `serverTick` share `tick`:** while shaking, `ticks++`; at `ticks >= 50` (`DURATION`), `shaking = false` and `ticks = 0`. Resonance: from 5 ticks, with no resonance yet and a raider near. The client's `ResonationEndAction` is `showBellParticles`, the server's is `makeRaidersGlow`.
- **`BellRenderer.extractRenderState`:** `ticks = be.ticks + partialTick`, and `shakeDirection = shaking ? clickDirection : null`. `submit` poses `BellModel` with the `entity/bell/bell_body` sprite at the block's light. It adds no facing rotation and no Y flip: model space is block pixels.
- **`BellModel.createBodyLayer`** (texture 32×32): `bell_body` box (-3,-6,-3) size 6×7×6 at uv 0,0, pivot `PartPose.offset(8, 12, 8)`; its child `bell_base` box (4,4,4) size 8×2×8 at uv 0,13, `offset(-8,-12,-8)`. The lip swings with the body. The fork's static `push_bell` already matches this geometry.
- **`BellModel.setupAnim(State(ticks, dir))`:** with a direction, `a = Mth.sin((double)(ticks / 3.1415927f)) / (4.0f + ticks / 3.0f)`. NORTH gives `xRot = -a`, SOUTH `xRot = a`, EAST `zRot = -a`, WEST `zRot = a`. Up and down leave it still. With no direction both are 0.
  - `Mth.sin` is the 65536-entry table (`minecraftoss_player::mth::sin`). The ModelPart order is Z·Y·X, as `special_models::part` (`:758-763`) already uses.
  - The peak is about 0.177 rad (10°) near tick 5. Its period is 2π² ≈ 19.7 ticks.
  - The lip first swings away from the struck face: a north hit moves the bottom south, an east hit moves it west.
- **The resonance particles are invisible in 26.3.** `showBellParticles` uses `ColorParticleOption.create(ENTITY_EFFECT, 16700985 + 5n)`. `create(type, int)` takes the int as ARGB, so the alpha is 0, and `SpellParticle` lerps the alpha towards 0. Vanilla draws nothing visible, so the fork draws none.
- **The glow outline.** A `GLOWING` entity is drawn into `LevelRenderer`'s outline target in its team colour (white without a team), and the `entity_outline` post chain draws only the edge, through blocks.

### Plan

**Group `bells-swing` (M), one commit: "Minecraft bells swing when rung"**
- `crates/minecraft_terrain/src/lids.rs`:
  - `LidKind::Bell` (`minecraft:bell`) and `Lid::Bell(BellSwing { shaking, ticks, direction })`.
  - `block_event(.., 1, param)` on a bell calls `ring(direction from the 3D value)`: ticks 0, shaking.
  - `tick` counts to 50 and stops. Tick bell entries in every `Drawn` state, not only `Apart`, so the swing keeps time with the sound.
  - `opening()` means shaking, and `at_rest()` means not shaking. `lid_closed` is always true for bells.
  - `PosedLid` gains the swing: `[x_rot, z_rot]` from vanilla's formula at `ticks + partial`, zero when still.
- `special_models.rs`: `push_bell(elements, [x_rot, z_rot])` with `part([8,12,8], [x_rot, 0, z_rot])`. The lip stays a child of the body. `extra_elements` passes `[0, 0]`.
- `model.rs`: `swung_bell_variants(pack, block, swing)`: the bell's blockstate model **without** the static body (`resolve_variants`, whose blockstate rotation is baked into the elements) plus the swung body, left unrotated as in vanilla.
- `mesh.rs` (`append_open_lids`, `:1178-1196`): bells take their variants from `swung_bell_variants`. Chests and shulkers stay on `lid_model`.
- `server.rs`: `hear_bells` (`:1369`) also queues `(pos, 1, direction.index())` into a new `Output.block_events: Vec<((i32, i32, i32), u8, i32)>`, next to `level_events`. Drain it with the level events at `:4288`. Only bells fill it for now; name it generically, because vanilla broadcasts every block event within 64 blocks.
- `minecraft_survival/src/entities.rs`: queue `output.block_events` beside `level_events` (`:1660-1663`). Present them in `present_level_events` (`:1278`, where the scene is at hand) through `lid_event`. That reaches the host's `view.lids` and sends `Heard::Block` to friends within `BLOCK_EVENT_RANGE` 64.
- Friends: no change. `minecraft_remote.rs:928` already feeds `lids.block_event`. No protocol change: `ToClient::BlockEvent` exists.
- Every ring source gets the swing: a hand use (`actions.rs:785-800`), redstone (`redstone.rs:762-771`), villagers (`server.rs:1454`), wind bursts, and E2's projectile hits.
- Tests:
  - `lids::tests::a_rung_bell_swings_for_fifty_ticks_away_from_the_struck_side`: the angle is 0 at tick 0 and peaks near 0.177 rad around tick 5. North gives -x, south +x, east -z, west +z. It is at rest after 50 ticks, and a second ring restarts it.
  - `lids::tests::a_bell_is_drawn_apart_while_it_swings_and_back_in_its_section_after`.
  - `mesh::tests::a_still_bell_drawn_apart_matches_its_section_and_a_swung_one_keeps_its_pivot`: the frame is unchanged, and the body's top centre (8,12,8)/16 stays fixed while its lip moves.
  - `server::tests::a_rung_bell_sends_block_event_one_with_its_direction`.
  - `entities::tests::a_bell_rung_on_the_host_swings_for_the_host_and_reaches_friends_as_a_block_event`.

**Group `bells-outline` (S, optional; drop it if time is short), one commit: "Glowing Minecraft mobs show a white outline like vanilla"**
- `render_gpu/src/drawsurf/minecraft_world.rs`: the Minecraft depth is `Depth24PlusStencil8` (`scene_depth.rs:9`), and its stencil is cleared with the depth (`scene_depth.rs:23-38`). Draw three passes:
  1. The cover mesh (`entities[5]`): depth `Always`, no depth write, no colour writes, stencil `Replace` on one bit.
  2. The hull (`entities[4]`): depth `Always`, stencil `NotEqual` on that bit. Only the rim outside the silhouette turns white, in the open and through walls.
  3. The cover again with stencil op `Zero` and no colour, to give the bit back. Check `colour_submit`'s `stencil_state` bits (`scene_depth.rs:102`) so the MW2 passes never read it.
- `glow_render.rs`: the cover is now a plain copy used as the mask, since the mob itself is already drawn.
- Test: `glowing_mobs_draw_an_outline_through_walls_masked_by_their_own_shape` (the pipelines' stencil states).

### Decisions (for the log)

- A bell's swing is a client animation, drawn apart from its section like a chest lid, from block event 1. Friends get it through the existing `ToClient::BlockEvent`.
- Vanilla's resonance particles have alpha 0 in 26.3 and are invisible, so the fork draws none. Remove the item from the round 9 "Left for later" list.
- The outline rim is world-space (1/16 of a block), not vanilla's screen-space post effect. It is still an outline, not a filled shape.
- `BLOCK_CHANGE` belongs to track W: added once in `Level::ring_bell` (`redstone.rs:776`), every ring source gets it.

### The owner's steps (survival, Normal, by day, flat ground)

- **B-1. Floor bell.** Look straight ahead and place a bell on cobblestone in front of you. A floor bell faces the way you looked when you placed it (`BellBlock.getStateForPlacement`), and only its front and back ring. Right-click the side facing you, below the rim: it rings, and the bell body swings away from you, back and forth, dying out over 2.5 seconds. **Screenshot bell_01_swing** mid-swing. Walk round and right-click the opposite side: it swings the other way. Right-click one of the two other sides: nothing (wrong side). Right-click the top: nothing.
- **B-2. Wall and ceiling bells.** Place a bell against a wall and one under a ceiling. Ring each from its allowed sides: each swings away from the side you hit. Pull a lever next to one: it swings along its facing. **Screenshot bell_02_wall_ceiling.**
- **B-3. Villages.** In a village with villagers, drink an ominous bottle and walk in. While the raid bar fills, villagers ring their bell and you see it swing.
- **B-4. Outline.** Hatch two pillagers about 20 blocks from the bell, one behind a house, and ring the bell. Two seconds later both glow for 3 seconds. The one in the open has a thin white rim. The hidden one shows as a white **outline** through the house wall, with the wall visible inside it, not a filled white shape. **Screenshot bell_06_glow_outline.** (Only with `bells-outline`.)
- **B-5. Friend window.** The friend stands near the bell while the host rings it: the friend sees the same swing. The friend rings it: both windows show the swing. The friend sees the raiders' outline too. **Screenshot bell_07_friend_swing.**

```
IW4L_MINECRAFT_KIT=minecraft:bell*4,minecraft:lever*2,minecraft:cobblestone*64,minecraft:oak_planks*64,minecraft:pillager_spawn_egg*4,minecraft:villager_spawn_egg*4,minecraft:ominous_bottle*2,minecraft:white_bed*2,minecraft:shield,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*8,minecraft:cooked_beef*16
```

16 backpack slots (beds do not stack).

### Files shared

- `server.rs`: `hear_bells`, `Output` and its drain.
- `lids.rs`, `special_models.rs`, `model.rs`, `mesh.rs`: E-banners (same track).
- `entities.rs`: every track.
- `entity_view.rs:1108-1121`: the per-kind glow loops, which D0's render registry may replace.
- `render_gpu/src/drawsurf/minecraft_world.rs`: may meet W's darkness.

---

# Track E2: gameplay and loot

## E-thrown. Thrown things hit every mob

### The owner's finding

> **Thrown things:** a player's tridents, snowballs, eggs and splash potions can't hit pillagers, vindicators, evokers, vexes, ravagers, hoglins, piglins, striders, ghasts or blazes. Arrows and guns can. (`docs/round9/release-2-tests.md:418`)

The research found more. The same gap covers withers, and every system that uses the same list: lingering clouds, splash and water bottles, wind-charge bursts, the dragon's wings and fireballs, and the effect swirls drawn over mobs. It also breaks a round 9 step: the snow golem's snowballs can't hit a blaze (`docs/round9/track-d-log.md:1180` says they hurt it). Potions also skip ghasts and blazes in four more stores, and harming hurts zoglins and withers instead of healing them.

### Cause

**1. The target list is hand-written and stops at round 7's mobs.**
- `crates/minecraft_terrain/src/thrown.rs:493-626` `targets(world)` lists these stores one by one: bats, zombies, skeletons, creepers, spiders, slimes, endermen, witches, iron golems, wolves, felines, wandering traders, snow golems, villagers, cows (with the horse family and llamas), sheep, pigs, chickens and shulkers.
- It leaves out `illagers` (pillager, vindicator, evoker), `vexes`, `ravagers`, `withers`, `ghasts`, `blazes`, `striders`, `hoglins` (hoglin, zoglin) and `piglins` (piglin, brute).
- `server.rs:2095` `tick_thrown` and `:1928` `tick_tridents` pass the flight only these boxes (`thrown.rs:446` → `minecraftoss_entities::potion::ThrownPotion::tick`, `third_party/minecraftoss/entities/src/potion.rs:208`). So a snowball flies through a pillager.
- The same list feeds `server.rs:1817` (lingering clouds), `:2599` (the dragon's wing and body hits), `:2684` (dragon fireballs), `:4343` `mob_swirls` (effect particles), `wind.rs:139` (the push of a wind burst), `splash.rs:35` (water bottles) and `splash.rs:119` (splash effects). All of these skip the same ten families.

**2. The hurt dispatch is a second hand-written chain.**
- `thrown.rs:655-770` `hurt_by` has arms for the round 7 families plus `wither` (`:751`). It has none for illagers, vexes, ravagers, ghasts, blazes, striders, hoglins or piglins, and returns `false` for them (`:757`).
- It does its own knockback (`thrown.rs:628`), so it skips the hoglin's 0.6 and the ravager's 0.75 knockback resistance. It also skips the cow's `note_attacker`, the slime's armour and the hoglin and piglin anger (`note_hoglin_hurt`).
- **Why arrows and guns work:** arrows use `EntityWorld::arrow_targets` (`third_party/minecraftoss/entities/src/world.rs:3272-3413`), which walks `self.order` through a `match` over every `EntityKey`, and `apply_arrow_hit` (`:3486`) has an arm for every store. Guns hit through the MW2 collision band.
- The player's melee hit already has a full per-store chain with the right reactions: `world/combat.rs:300-459` `hurt_by_player`, with knockback through `knock_back` (`:233`, resistances included).

**3. Snowballs don't hurt blazes.** The player's snowball always passes 0 (`server.rs:2231` → `thrown::hurt` → `hurt_by(.., 0.0, ..)`). Only the snow golem's path knows the 3 damage (`world/snow_golems.rs:263`, `snow_golem::SNOWBALL_BLAZE_DAMAGE`).

**4. Potion stores are missing families.**
- `world/witches.rs:478` `mob_effects_mut`, `:562` `mob_effects`, `:627` `heal_mob` and `:693` `hurt_mob_by_magic` have no arm for `ghasts` or `blazes`. Both stores hold `effects` (`world/ghasts.rs:12`, `world/blazes.rs:12`) and tick them (`ghasts.rs:211`, `blazes.rs:311`). So splash, lingering and harming potions do nothing to them, even once they are targets.
- `witches.rs:599` `splash_instant` decides "undead" only from the `zombies` and `skeletons` stores. So harming hurts zoglins, withers and skeleton or zombie horses instead of healing them. `crate::wither::is_undead` (`wither.rs:73-95`) already holds the full `#minecraft:undead` tag.
- `hurt_mob_by_magic` never calls `note_hoglin_hurt`, so a harming potion doesn't anger hoglins or piglins.

**5. Water bottles hurt only endermen.** `splash.rs:28-66` `wet` hurts endermen by 1. It skips blazes, striders and snow golems.

**6. A thrown thing can strike its thrower's mount.** The flight skips only the thrower's own box (`thrown.rs:248` `owner_box`, `potion.rs:209`). Horses, llamas and camels are already targets; striders and ravagers will be once (1) is fixed. So a snowball thrown down from a saddle can stop on the player's own mount.

**7. Wind charges anger every mob.** `hurt_by` sets `hurt_by` (the attacker memory) for every family, whatever the damage type.

**8. (Optional group) The End.** End crystals (`end_crystal.rs`) and dragon parts (`ender_dragon.rs:871` `part_box`) are not thrown targets at all. Only MW2 shots and blasts reach them (`server.rs:2926` `end_hit`).

### Vanilla 26.3

- `Snowball.onHitEntity`: `hurt(thrown(this, owner), entity instanceof Blaze ? 3 : 0)`.
- `ThrownEgg.onHitEntity` and `ThrownEnderpearl.onHitEntity`: `hurt(thrown(this, owner), 0)`.
- `ThrownTrident.onHitEntity`: 8.0, `trident(this, owner)` source, then `EnchantmentHelper.modifyDamage` and `hurtOrSimulate`.
- `Projectile.calculateHorizontalHurtKnockbackDirection`: the projectile's motion (x, z). The fork's `-velocity` call in `thrown.rs:766` matches it. Knockback power is 0.4 (`KNOCKBACK`, `thrown.rs:20`), scaled by the victim's knockback resistance (`LivingEntity.knockback`).
- `Projectile.canHitEntity`: `canBeHitByProjectile()` and, until `leftOwner`, not `owner.isPassengerOfSameVehicle(entity)`. `isOutsideOwnerCollisionRange` uses the projectile box `expandTowards(motion).inflate(1.0)` against `owner.getRootVehicle().getSelfAndPassengers()`. `leftOwnerChecked` is reset every tick.
- `AbstractThrownPotion.affectEntitiesAround`: the box `inflate(4, 2, 4)`, then `distanceToSqr < 16`. Potions in `#hurts_water_sensitive_entities` deal `indirectMagic(this, owner)` 1.0 to `isSensitiveToWater()` mobs. Only `Blaze`, `Enderman`, `SnowGolem` and `Strider` override it to `true`.
- `#inverted_healing_and_harm` = `#undead` = `#skeletons`, `#zombies` (with `zoglin`, `zombie_horse`, `camel_husk`, `zombie_nautilus`), `wither` and `phantom`.
- `LivingEntity.hurtServer` skips `setLastHurtByMob` when the source is in `#no_anger` (`mob_attack_no_aggro`) or is `wind_charge` and the victim is in `#no_anger_from_wind_charge`: breeze, skeleton, bogged, stray, zombie, husk, spider, cave_spider and slime. The player is still credited for the drops.
- `WitherBoss.hurtServer`: when powered (half health or less), it refuses an `AbstractArrow` direct entity (tridents too) or a `WindCharge`. `hurt_typed` already does this (`world/withers.rs:181`).
- `#is_projectile` damage types: arrow, trident, mob_projectile, unattributed_fireball, fireball, wither_skull, thrown and wind_charge. An enderman dodges all of them, as `hurt_by` does today.
- `#deflects_projectiles` = breeze. This is track D's.
- `EndCrystal.hurtServer`: any source that isn't the dragon and isn't one it is invulnerable to removes the crystal, even at 0 damage. A snowball, egg or pearl breaks a crystal.
- The dragon takes `damage / 4 + min(damage, 1)` on a part other than the head and neck (`ender_dragon.rs:763` `part_damage` already does this). So a snowball does nothing to it, and a trident deals 8 to the head and 3 to the body.

### Plan

Goal: after this section, `thrown.rs`, `splash.rs` and `wind.rs` hold **no per-mob code**. Every per-store arm lives in the vendored `EntityWorld`, at sites that D0's registry must already cover for melee and arrows. A new family then needs D0's one generic arm per site and nothing in the thrown code. E-fireworks and E-bell-hits reuse the same list.

**Shape that doesn't collide with D0:**
- One public list: `EntityWorld::hit_boxes(&self) -> Vec<crate::potion::PotionTarget>`. It holds every living, hittable mob (`owner: false`). The shulker uses `bounding_box()`, and a creeper that has exploded is left out. It loops over each store once, like `mob_candidates` (`world.rs:4080`), so it stays O(n). `arrow_targets` maps it. `thrown::targets(world)` becomes `world.hit_boxes()`.
- One public hurt: `EntityWorld::player_projectile_hurt(&mut self, id, ProjectileHit { amount, kind: &'static str, player, velocity }) -> Option<DamageResult>` in `world/combat.rs`.
  - The body of today's private `hurt_by_player` (`:300-459`) becomes `fn hurt_for_player(&mut self, id, amount, player, kind, source) -> Option<DamageResult>`: the store chain, the blame, `credit`, `villager_hurt_by` and `note_hoglin_hurt`.
  - Melee wraps it with the knockback away from the attacker. The projectile wraps it with the enderman dodge, the closed shulker refusing tridents, `knock_back(id, 0.4, -velocity.x, -velocity.z)` (resistances included) and the wind-charge calm list.
- Guard test: `hit_boxes` ids equal `mob_ids()` minus thrown potions and dead mobs. A store that `mob_ids` knows about but `hit_boxes` forgets fails the test.

**Group `ti1-targets-and-hurt` (M), four commits**
1. **"Minecraft thrown things hurt every mob family"**
   - `world/combat.rs`: the `hurt_for_player` split; the `kind` is passed through instead of the fixed `"minecraft:player_attack"` (wither `hurt_typed`, villagers, wolves, felines and traders use it); the new `player_projectile_hurt` and `ProjectileHit` (re-export them in `world.rs` next to `PlayerAttack`).
   - `thrown.rs`: `hurt_by` becomes a call to `player_projectile_hurt`, and its private `knockback` and store chain (`:628-770`) go away.
   - The `random` argument is no longer needed. Drop the seed draws in `server.rs:1964` and `:2144` only if no test pins the later `sound_random` draws; otherwise keep the draw and ignore the value.
2. **"Minecraft thrown things hit pillagers, ravagers, withers and the Nether mobs"**
   - `world.rs`: `hit_boxes`, and `arrow_targets` filtering it by owner.
   - `thrown.rs:493-626` becomes one line. All nine consumers listed in the cause pick it up unchanged.
3. **"Minecraft snowballs hurt blazes"**
   - `world/snow_golems.rs`: `pub fn snowball_damage(&self, victim) -> f32` (3 for a blaze, else 0), used by `snowball_hits_mob`.
   - `server.rs` `tick_thrown`: the player's snowball passes it (`:2231`).
4. **"Minecraft wind charges don't anger the mobs vanilla keeps calm"**
   - A `NO_ANGER_FROM_WIND_CHARGE` list of the nine ids in the vendored entities crate.
   - In `player_projectile_hurt`, when `kind == "minecraft:wind_charge"` and `entity_type(id)` is in the list, skip the blame but keep `credit`. Track D's breeze wind charges use the same check.

**Group `ti2-potions` (M), three commits**
1. **"Minecraft potions work on ghasts and blazes"**: `world/witches.rs` adds `ghasts` and `blazes` arms to `mob_effects_mut`, `mob_effects`, `heal_mob` (`ghast::MAX_HEALTH` 10, `blaze::MAX_HEALTH` 20) and `hurt_mob_by_magic`, with `ai.state.hurt_by` set from the owner like the vex arm (`:745`). Call `note_hoglin_hurt` there on an applied hit with an owner.
2. **"Minecraft harming heals every undead mob"**: `splash_instant` (`witches.rs:599`): `undead = self.entity_type(id).is_some_and(crate::wither::is_undead)`. `mob_effects_mut`'s undead flag for the `cows` store comes from the same check (skeleton and zombie horses ignore poison and regeneration). Read `entity_type` before the mutable borrow.
3. **"Minecraft water bottles hurt blazes, striders and snow golems"**: `splash.rs` `wet`: a water-sensitive list (`enderman`, `blaze`, `strider`, `snow_golem`) checked through `entity_type`. The non-endermen take 1.0 `indirect_magic` from the owner through a new public `EntityWorld::splash_water_hurt(id, owner)` that wraps `hurt_mob_by_magic`. Endermen keep today's hurt and dodge.

**Group `ti3-mounts` (S), one commit: "Minecraft thrown things pass the mount their thrower rides"**
- `server.rs` `tick_thrown` and `tick_tridents`: for a player owner, take `self.mobs.vehicle_of(Rider::Player(owner))`, walk up to the root vehicle and mark the root and all its passengers (`passengers_of`) as `owner: true` targets. Grow `owner_box` to cover those boxes as well (vanilla `getRootVehicle().getSelfAndPassengers()`). Mob throwers keep today's rule.

**Group `ti4-end-targets` (M, optional; last in E2, cut it if time is short): "Minecraft snowballs and tridents hit end crystals and the ender dragon"**
- `end_crystal.rs`: an `END_TARGET` id range below `item_frame::FRAME_TARGET` (`u64::MAX / 4`), `end_targets(crystals, dragons) -> Vec<PotionTarget>` from the crystal pick box and `ender_dragon::part_box`, and `EndTarget::of_target(u64)`.
- `server.rs`: `tick_thrown` and `tick_tridents` add them. A hit runs `end_hit(target, EndDamage::attack(amount, owner))` with `from` set to the thrower's position.
- A snowball, egg or pearl deals 0 (it breaks a crystal and does nothing to the dragon). A trident deals 8, and a wind charge 1. Splash potions break on a crystal without hurting it.

### Tests

- Vendored entities:
  - `world::tests::hit_boxes_cover_every_live_mob`: one mob of each store; the ids equal `mob_ids()` minus potions; a dead mob and an exploded creeper are absent; the shulker box is its `bounding_box`.
  - `world::combat::tests` (new module): `a_thrown_hit_lands_on_pillagers_vexes_ravagers_and_the_nether_mobs`, `a_projectile_knocks_a_hoglin_and_a_ravager_back_less`, `a_trident_angers_a_hoglin_and_a_piglin_at_the_thrower`, `a_closed_shulker_turns_a_trident_away`, `an_enderman_dodges_every_thrown_thing`, `a_wind_charge_leaves_a_zombie_calm_but_angers_a_pillager` (the player is still credited).
  - `world::witches::tests`: `a_poison_splash_poisons_a_blaze_and_a_ghast`, `harming_heals_a_zoglin_a_wither_and_a_skeleton_horse`, `a_harming_splash_angers_a_hoglin_at_the_thrower`.
- `minecraft_terrain`:
  - `thrown::tests::a_snowball_hurts_a_blaze_by_three_and_nothing_else`.
  - `splash::tests::a_water_bottle_hurts_a_blaze_a_strider_and_a_snow_golem_by_one`.
  - `wind::tests::a_burst_pushes_a_pillager_and_a_ravager`.
  - `server::tests::every_hatching_mob_is_a_thrown_target_and_takes_a_trident`: summon each `HATCHING_EGGS` entry (`server.rs:7717`, read only) except `ender_dragon`. Each id is in `thrown::targets(&server.mobs)`, and `thrown::hurt_by` with 8 trident damage lands. The only allowed refusals are the enderman's dodge and the closed shulker. Future families are covered automatically.
  - `server::tests::a_player_snowball_stops_on_a_pillager_and_knocks_it_back` (a real flight through `tick_thrown`).
  - `server::tests::a_snow_golem_s_snowball_hits_a_blaze`.
  - `server::tests::a_snowball_thrown_from_a_strider_passes_its_strider_but_hits_another`.
  - `server::tests::mob_swirls_show_a_poisoned_blaze`.
  - TI4: `server::tests::a_snowball_breaks_an_end_crystal_and_a_trident_hurts_the_dragon_head_by_eight`.
- The existing tests of the `hurt_by` callers (frames, tridents, wind charges, potions, snow golems) must pass unchanged.

### The owner's steps

Setup: a flat overworld area, Normal difficulty, by day. Pen each mob in with cobblestone. Use the kit below, the same kit for the friend. Ghasts need open sky, so do step 8 outside or in the Nether.

- **T-1.** Use a pillager egg 8 blocks away and throw a snowball at it. The snowball breaks on the pillager (white crumbs), pushes it back without hurting it, and the pillager comes at you. **Screenshot T1.**
- **T-2.** Use the vindicator and evoker eggs and throw eggs at both. Each egg breaks on them and pushes them back (sometimes a chick hatches). When the evoker summons vexes, throw a snowball at a vex in the air: it stops on the vex. **Screenshot T2.**
- **T-3.** Use the ravager egg and throw the loyalty trident at it. The ravager flashes red, loses about 4 hearts and hardly moves back. The trident flies back to you. **Screenshot T3.**
- **T-4.** Use a hoglin egg and throw the loyalty trident. The hoglin flashes red and charges you, and the trident flies back. Use a piglin egg and throw a snowball (wear no gold): the piglin gets angry and attacks. **Screenshot T4.**
- **T-5.** Use a blaze egg inside a cobblestone pen and throw snowballs one by one, about a second apart. Every hit flashes the blaze red, and about 7 hits kill it. **Screenshot T5.**
- **T-6.** Use the snow golem egg 6 blocks from a new blaze (the second blaze egg). The golem's snowballs hit the blaze and hurt it. Round 9 asked for this, but it could not work before. **Screenshot T6.**
- **T-7.** Use a strider egg next to a new blaze (the third egg) and splash a water bottle between them. Both flash red once. **Screenshot T7.**
- **T-8.** Throw a poison splash at a blaze (the T-7 blaze, or the fourth egg): green swirls rise from it and it flashes until it has 1 heart left. Throw a harming splash under a ghast: it flashes red. **Screenshot T8.**
- **T-9.** Use a zoglin egg and throw a harming splash at it: no red flash, because harming heals it. Throw a healing splash: now it flashes red. **Screenshot T9** after the healing splash.
- **T-10.** Throw a wind charge into a group of pillagers: they are flung away. **Screenshot T10.**
- **T-11.** Use two more strider eggs. Saddle one strider and ride it with the warped fungus on a stick. Look straight down and throw a snowball: it falls past your strider to the ground. Throw one at the other strider: it hits it. **Screenshot T11.**
- **T-12.** Throw the lingering poison at two pillagers: the cloud poisons both (swirls over them). **Screenshot T12.**

Friend (two windows):
- **T-13.** The friend throws snowballs at a pillager and a blaze the host spawned. Both windows show the same knockback and the blaze's red flash. **Screenshots T13-host and T13-friend.**
- **T-14.** The friend throws the trident at a hoglin. The hoglin charges the friend, not the host. **Screenshot T14-friend.**
- **T-15.** The friend rides a strider and throws a snowball straight down. It passes the strider. **Screenshot T15-friend.**

Wither and End (second kit):
- **T-16.** Use the wither egg far away from your base. Throw the plain trident: the wither flashes and loses health. Once it is at half health (blue armour shimmer), tridents and wind charges bounce off, while snowballs still stop on it. **Screenshots T16 and T17.**
- **T-17.** (Only if TI4 is built.) In the End with the dragon alive, throw a snowball at an uncaged crystal: it explodes. Throw the loyalty trident at the dragon's head: it flashes red. **Screenshot T18.** The friend's snowball breaks another crystal. **Screenshot T18-friend.**

The wind-charge calm rule (zombies, skeletons, spiders and slimes) is covered by unit tests, not by a screenshot step.

```
IW4L_MINECRAFT_KIT=minecraft:trident+loyalty=3,minecraft:snowball*32,minecraft:egg*16,minecraft:wind_charge*16,minecraft:splash_potion@harming*2,minecraft:splash_potion@healing,minecraft:splash_potion@poison,minecraft:splash_potion@water*2,minecraft:lingering_potion@poison,minecraft:pillager_spawn_egg*4,minecraft:vindicator_spawn_egg*2,minecraft:evoker_spawn_egg,minecraft:ravager_spawn_egg,minecraft:hoglin_spawn_egg*2,minecraft:zoglin_spawn_egg,minecraft:piglin_spawn_egg*2,minecraft:strider_spawn_egg*4,minecraft:blaze_spawn_egg*4,minecraft:ghast_spawn_egg,minecraft:snow_golem_spawn_egg,minecraft:saddle,minecraft:warped_fungus_on_a_stick,minecraft:cobblestone*64,minecraft:golden_apple*16
```

27 backpack slots: splash and lingering potions do not stack, so `splash_potion@harming*2` and `splash_potion@water*2` take two each, and `snowball*32` takes two 16-stacks. The vexes come from the evoker (T-2), so there is no vex egg. T-10 and T-12 use the four pillager eggs.

Second kit, for the wither and the End:

```
IW4L_MINECRAFT_KIT=minecraft:wither_spawn_egg*2,minecraft:trident+loyalty=3,minecraft:trident,minecraft:snowball*16,minecraft:egg*16,minecraft:ender_pearl*16,minecraft:wind_charge*16,minecraft:end_crystal*4,minecraft:obsidian*8,minecraft:splash_potion@harming*2,minecraft:splash_potion@healing*2,minecraft:shield,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*16,minecraft:milk_bucket,minecraft:cooked_beef*32,minecraft:cobblestone*64,minecraft:totem_of_undying
```

### Files shared

- Vendored `world.rs` (D0, D, W, M1-M3: new stores, `mob_ids`, `entity_type`), `world/combat.rs` and `world/witches.rs` (D0's generic arms), `world/snow_golems.rs` (low risk).
- `thrown.rs`: D0 must not add arms here; D (breeze deflection, breeze wind charges); E-fireworks reuses `thrown::targets`.
- `wind.rs`: D's breezes. `splash.rs`: none known.
- `server.rs` `tick_thrown`, `tick_tridents` and the test module: everyone; W adds projectile game events in these two functions.
- `end_crystal.rs` and `ender_dragon.rs`: TI4 only.

---

## E-doors. Zombies and vindicators break and open doors

### The owner's finding

> **No door breaking:** vindicators in raids and zombies on Hard don't break or open doors. (`docs/round9/release-2-tests.md:414`)

Round 9 left a note (`docs/round9/track-d-log.md:3149`): the fork has no `DoorInteractGoal`, `BreakDoorGoal` or `OpenDoorGoal` for any mob. Build them once and use them for zombies and vindicators. Clients need the crack progress and level events 1019, 1021 and 2001.

### Cause in the current code

All paths below are under `third_party/minecraftoss/entities/src/` unless they name a crate.

- **Zombies have no door goal.** `monster_ai.rs:1946-1955` `zombie_goals()` has no slot-1 `BreakDoorGoal`. `ZOMBIE_GOAL_NAMES` (`monster_ai.rs:140-148`) lists 7 goals. `zombified_piglin_goals()` (`monster_ai.rs:1969-1977`) has none either.
- **`can_break_doors` is loaded but does little.** It is loaded and saved (`minecraft_terrain/src/server_mobs.rs:833` and `:1543`), rolled at spawn (`world/src/natural_spawner/spawning.rs:1422` `special * 0.1`, `:1471` leader bonus, `:1489` conversion) and copied into the AI every tick (`world.rs:6178`). Only `VillageGoal` reads it (`monster_ai.rs:2141-2150`), to plan a path through doors. Nothing sets the walk profile's `can_open_doors` from it; vanilla's `setCanBreakDoors(true)` calls `navigation.setCanOpenDoors(true)`.
- **The vindicator's door slots are placeholders.** Slots 2 and 3 are `NeverGoal`s (`illager_ai.rs:177-183`; names `VindicatorBreakDoorGoal` and `RaiderOpenDoorGoal` at `illager_ai.rs:39-40`). `IllagerState.near_raid` (`illager_ai.rs:140`, computed at `world/illagers.rs:555-570`) is read by nothing. It is vanilla's `ServerLevel.isRaided`, which `customServerAiStep` copies into `setCanOpenDoors` every tick. The illager walk profile keeps `can_open_doors = false` (`walk_path.rs:72-73`), so raid paths stop at closed doors.
- **Path finding is already ported.** `walk_path.rs:674-679` turns `DoorWoodClosed` into `WalkableDoor` when `can_open_doors && can_pass_doors`. Iron doors stay `DoorIronClosed` (`walk_path.rs:443`). Only the flags need setting.
- **Mobs can't touch the world from their goals.** `Goal` gets `&dyn World` (`goals.rs:38-56`). Illager `tick_ai` takes `&impl World` (`world/illagers.rs:127-129`). Door changes must come out as requests and be applied by the store's tick, which has `&mut impl World`: zombies `world.rs:5968-5976` and `world.rs:6100-6290`, illagers `world/illagers.rs:536`.
- **What villagers and piglins already do** (reuse it, don't copy it):
  - Villagers: `InteractWithDoor` (`villager_brain/behaviors.rs:3745-3797`) opens doors on its path and shuts them behind. Its door set is `#mob_interactable_doors` (`mob_door`, `behaviors.rs:3653`). `set_door` overlays pending changes so the same tick sees them (`behaviors.rs:3667-3688`, `door_open` at `:3702`).
  - `world/villagers.rs:828-835` applies the requests through `set_door_open` (`world/villagers.rs:1046-1068`). It sets both halves, returns the open or close sound by wood type, and the pitch comes from the level random (`nextFloat * 0.1 + 0.9`).
  - Other villagers' paths recompute through `recompute_others` and `recompute_for_block` (`world/villagers.rs:1083-1125`, private).
  - Piglins do the same (`piglin_brain/behaviors.rs:85-143`, applied at `world/piglins.rs:1779-1793`).
- **Breaking a block already works end to end.** `World::set_block(pos, None)` in the server goes to `level.set_block(AIR, update::ALL)` (`minecraft_terrain/src/server_mobs.rs:245-256`). That runs the door shape rule (`generator/src/feature/update.rs:754-766`), which turns the other half to air through `update_or_destroy` (`world/src/level/mod.rs:663-672`). Then `destroy_block_drops(drop = true)` (`world/src/level/drops.rs:164-180`) drops the door item and records `destroyed`. The host turns `destroyed` into particles, the break sound and `BREAK_EVENT` for friends (`render_anim/src/minecraft_world.rs:1234-1245`, `render_anim/src/minecraft_levels.rs:519-532`). Removing the upper half is therefore enough, as in vanilla.
- **Level events have a pipeline.** `server.rs:1163-1175` (`ZOMBIE_INFECT`) pushes into `level.level_events`; `server.rs:4288` adds them to `Output`; `minecraft_survival/src/entities.rs:1660-1663` sends them to the host's view and to friends (`Heard::Event`, 64 blocks); `minecraft_terrain/src/level_events.rs:87-140` plays them, but has no arm for 1019 or 1021.
- **Cracks have a pipeline.** `minecraft_survival/src/session.rs:504-522` `level_cracks` gathers bullet and player dig cracks, keyed by breaker `u32`, with `BULLETS = u32::MAX` (`session.rs:35`). `session.rs:305-330` sends a stage on each change and `-1` when a crack is gone (32 blocks, `CRACK_RANGE`). The host draws its cracks at `render_anim/src/minecraft_world.rs:1550-1553`, the friend at `render_anim/src/minecraft_remote.rs:745-756`. Nothing feeds mob cracks into either.
- **Swings reach clients** through `ctx.melee.swings` (zombies `minecraft_terrain/src/client_mobs.rs:1233`, illagers `:1237`).
- **Collision state:** `Body.horizontal_collision` exists (`movement.rs:35`). The AI sees a copy (`ai.state.body = body.clone()`, `world/illagers.rs:157`).

### Vanilla 26.3

**`DoorInteractGoal`**
- The constructor throws unless `GoalUtils.hasGroundPathNavigation` (`navigation.canNavigateGround()`).
- `canUse`: needs ground navigation, `mob.horizontalCollision`, and a path that is not done. For node `i` in `0 .. min(nextNodeIndex + 2, nodeCount)`: `doorPos = (node.x, node.y + 1, node.z)`. If `mob.distanceToSqr(doorPos.x, mob.y, doorPos.z) <= 2.25` (block corner, not centre), then `hasDoor = DoorBlock.isWoodenDoor(doorPos)`; return true if set. Otherwise `doorPos = blockPosition().above()` and the result is `isWoodenDoor` there.
- `start`: `passed = false`, `doorOpenDir = (door.x + 0.5 - x, door.z + 0.5 - z)` as floats.
- `tick`: `passed` turns true once the dot product of the new direction with the start direction is below 0.
- `canContinueToUse`: `!passed`. `requiresUpdateEveryTick`: true.
- `isOpen`: false without a door. A block that is no longer a `DoorBlock` clears `hasDoor`; otherwise it reads `OPEN`.
- `setOpen(b)`: `DoorBlock.setOpen(mob, level, state, doorPos, b)`. That does nothing if `OPEN` already equals `b`. Otherwise it calls `setBlock(.., 10)`, `playSound` and `gameEvent(BLOCK_OPEN or BLOCK_CLOSE)`.
- `DoorBlock.isWoodenDoor`: any `DoorBlock` whose `BlockSetType.canOpenByHand()` is true. In 26.3 that is every door in `#doors` except `iron_door`, which equals `#mob_interactable_doors`: 13 wooden doors (poplar included) and the 8 copper doors.

**`BreakDoorGoal`**
- `DEFAULT_DOOR_BREAK_TIME 240`. `lastBreakProgress` and `doorBreakTime` start at -1. `getDoorBreakTime() = max(240, doorBreakTime)`.
- `canUse`: `super.canUse` and `mob_griefing` and `validDifficulties.test(level difficulty)` and `!isOpen()`.
- `start`: `super.start()`, `breakTime = 0`.
- `canContinueToUse`: `breakTime <= getDoorBreakTime()` and `!isOpen()` and `doorPos.closerToCenterThan(mob.position(), 2.0)` and a valid difficulty. Mob griefing is not checked again.
- `stop`: `super.stop()`, then `level.destroyBlockProgress(mob.getId(), doorPos, -1)`.
- `tick`:
  1. `super.tick()`.
  2. On `random.nextInt(20) == 0` (the mob's own random): `levelEvent(1019, doorPos, 0)`, and if the mob is not swinging, `swingForAttack(getUsedItemHand())`.
  3. `breakTime++`; `progress = (int)(breakTime / (float)getDoorBreakTime() * 10.0F)`. On a change: `destroyBlockProgress(id, doorPos, progress)`.
  4. When `breakTime == getDoorBreakTime()` with a valid difficulty: `removeBlock(doorPos, false)`, `levelEvent(1021, doorPos, 0)`, then `levelEvent(2001, doorPos, Block.getId(getBlockState(doorPos)))`. The block is already air at that point, so this last event draws and plays nothing.
- `doorPos` is the upper half. `removeBlock` clears flags 1 and 32 for the neighbour shape update (`flags & -34`), so the lower half is destroyed with drops: one door item, its 2001 particles and break sound, and `BLOCK_DESTROY`.
- No flags of its own.

**`OpenDoorGoal(mob, closeDoor)`**: `start`: `forgetTime = 20`, `setOpen(true)`. `tick`: `forgetTime--`, then `super.tick()`. `canContinueToUse`: `closeDoor && forgetTime > 0 && super.canContinueToUse()`. `stop`: `setOpen(false)`. No flags.

**Zombies**
- `DOOR_BREAKING_PREDICATE`: `difficulty == HARD`. The goal is `new BreakDoorGoal(this, DOOR_BREAKING_PREDICATE)` (break time 240).
- `setCanBreakDoors(b)`, when `navigation.canNavigateGround()` and the value changes: stores it, calls `navigation.setCanOpenDoors(b)`, and adds the goal at priority 1 for true or removes it for false. Without ground navigation it removes the goal and clears the flag.
- It is called from `finalizeSpawn` (`nextFloat < special * 0.1`), from `handleAttributes` for the leader bonus (`special * 0.05`, then true), and on load (`CanBreakDoors`).
- 26.3 has no `supportsBreakDoorGoal` override in `ZombifiedPiglin`, `Drowned`, `Husk` or `ZombieVillager`. Every zombie kind on ground navigation can break doors.
- `MoveThroughVillageGoal` sets `setCanOpenDoors(canDealWithDoors)` for its path, then `setCanOpenDoors(true)` after it. The fork's "ever after" note at `monster_ai.rs:2083-2092` is right.

**Vindicators**
- `registerGoals`: 2 `VindicatorBreakDoorGoal`, 3 `AbstractIllager.RaiderOpenDoorGoal`.
- `VindicatorBreakDoorGoal`: `BreakDoorGoal(mob, 6, DOOR_BREAKING_PREDICATE)` (NORMAL or HARD; break time still 240), `setFlags(MOVE)`. `canUse`: `hasActiveRaid()` and `random.nextInt(reducedTickDelay(10)) == 0` (`nextInt(5)`, the vindicator's random) and `super.canUse`. `canContinueToUse`: `hasActiveRaid()` and `super.canContinueToUse`. `start`: `super.start()`, `setNoActionTime(0)`.
- `RaiderOpenDoorGoal`: `OpenDoorGoal(raider, false)` whose `canUse` adds `hasActiveRaid()`.
- `Raider.hasActiveRaid`: `getCurrentRaid() != null && raid.isActive()`.
- `customServerAiStep` runs after the goals and the navigation tick and before the move control: when not NoAI and on ground navigation, `navigation.setCanOpenDoors(level.isRaided(blockPosition()))`. `finalizeSpawn`'s `setCanOpenDoors(true)` is overwritten on the first tick.
- Pillagers, evokers, witches and ravagers have no door goals, and `Raider.RaiderMoveThroughVillageGoal` never touches the door flag.

**Client (`LevelEventHandler`)**: 1019 `entity.zombie.attack_wooden_door`, 1021 `entity.zombie.break_wooden_door`. Both play at the block as `HOSTILE` sounds, volume 2.0, pitch `(r.nextFloat() - r.nextFloat()) * 0.2 + 1.0` from the client random. Both are in the 26.3 `sounds.json`.

### Plan

#### Group `doors-zombies` (M), two commits

**Commit 1: "Minecraft zombies break wooden doors on Hard like vanilla"**

New vendored file `door_goals.rs`, registered in `lib.rs` with one line. It holds:
- `pub struct DoorWork`, in `MonsterGoalContext` as `door` (one field and its default near `monster_ai.rs:997`): `toggles: Vec<(Pos, bool)>`, `events: Vec<(i32, Pos)>`, `broken: Option<Pos>`, `crack: Option<(Pos, u8)>`, `swing_ticks: i32`.
- `is_wooden_door(block)`: a `DoorBlock` that is not `iron_door`, the same test as `VillageGoal` at `monster_ai.rs:2156`.
- `door_open(ctx, world, pos)`: reads pending `toggles` before the world, like the villagers' `door_open` at `behaviors.rs:3702`.
- `DoorInteract` core: `can_use` per the vanilla facts (`ctx.body.horizontal_collision`, `ctx.navigation.nodes` / `next`, the 2.25 corner check, the above-feet fallback), `start`, `tick` (the passed test).
- `BreakDoorGoal { difficulty: fn(i32) -> bool, raid_only: bool, move_control: bool, .. }`:
  - `every_tick() = true`, which the fork's selector needs so the 240 ticks are real ticks.
  - The `nextInt(20)` knock pushes event 1019.
  - The goal keeps its own 6-tick swing clock in place of `isSwinging`, and adds to `ctx.melee.swings`.
  - It sets `crack` when the stage changes and clears it in `stop`.
  - At 240 it sets `broken = Some(doorPos)` and pushes 1021.
  - Zombies: Hard only (`ctx.difficulty == 3`), `mob_griefing` on `can_start` only, and the goal is inert unless `ctx.can_break_doors`. That stands in for vanilla's add and remove at slot 1; the goal has no controls, so the slot makes no difference.
- `apply_door_work(world, ...)`, shared by zombies and vindicators: toggles through `super::villagers::set_door_open` with the level-random pitch and a door voice at the block centre; `broken` as `world.set_block(upper, None)` (the real level drops the lower half); events into a new `EntityWorld` queue drained by `take_level_events()` (put the queue's `take_level_events` next to `take_villager_infections`, vendored `world.rs:3823`, not next to `take_entity_events` at `:4069`, where E1 adds `broadcast_entity_event`); other villagers recompute through `recompute_for_block` for each changed half (make it `pub(super)`).

Changes to existing files:
- `zombie_goals()` and `zombified_piglin_goals()` add `selector.add(1, BreakDoorGoal::zombie())` last. `"BreakDoorGoal"` is appended at the end of `ZOMBIE_GOAL_NAMES` (8) and `ZOMBIFIED_PIGLIN_GOAL_NAMES` (7), so goal ids and `retain_goals` stay stable.
- `world.rs:6178`: with `can_break_doors`, also set `ai.state.walk.can_open_doors = true` (`setCanBreakDoors(true)`).
- After `monster_ai_step`, apply `ai.state.door` with `apply_door_work`.
- `minecraft_terrain/src/server.rs`: next to `infect_villagers` (`:1116`), drain `self.mobs.take_level_events()` into `self.level.level_events`.
- `minecraft_terrain/src/level_events.rs`: constants `ZOMBIE_ATTACK_WOODEN_DOOR = 1019` and `ZOMBIE_BREAK_WOODEN_DOOR = 1021` with their sound ids, and an arm like `ZOMBIE_INFECT`'s (`:118-125`: volume 2.0, the same pitch draw). Host and friends then hear both through the existing pipeline.
- `docs/MINECRAFT-SURVIVAL.md`, zombie paragraph: zombies that can break doors break wooden and copper doors on Hard.

**Commit 2: "Minecraft door cracks from mobs show for the host and friends"**
- `EntityWorld::door_cracks() -> Vec<(u64, Pos, u8)>` over zombies and illagers with `ai.state.door.crack`, read from the snapshot the host already keeps (`entities.view.world`, `minecraft_survival/src/entities.rs:1674`).
- `minecraft_survival/src/session.rs::level_cracks` (`:504`) adds them under a mob breaker key that cannot meet `BULLETS` or a player id: `MOB_BREAKER | (id as u32 & 0x3FFF_FFFF)` with `MOB_BREAKER = 0x4000_0000`.
- `render_anim/src/minecraft_world.rs:1550-1552` extends the host's crack list the same way.
- The friend side needs no change: `ToClient::Crack` already draws, and `-1` clears.

#### Group `doors-raiders` (M), one commit (plus an optional one)

**Commit 3: "Minecraft vindicators open and break doors in raids"**
- `illager_ai.rs:177-183`: slot 2 becomes `BreakDoorGoal::vindicator()`: Normal or Hard (`difficulty >= 2`), `Control::Move`, needs `ctx.illager.raid.is_some_and(|r| r.active)`, the `ctx.random.next_int(5) == 0` roll before the door checks, and `start` sets `ctx.illager.reset_idle = true` (the existing no-action reset).
- Slot 3 becomes `OpenDoorGoal { close_door: false }` with the same raid check: `start` opens through `toggles`, `stop` closes, `forget_time` 20.
- `world/illagers.rs`: in `tick_ai` right after the navigation `tick_in` (`:192-201`), a vindicator sets `ai.state.walk.can_open_doors = ai.state.illager.near_raid`. This is `customServerAiStep`, so it applies to the next path.
- In `tick_illager` after `tick_ai` (`:668-676`, where `world` is `&mut`): `apply_door_work` as for zombies.
- `docs/MINECRAFT-SURVIVAL.md`, vindicator paragraph: "Door breaking in raids is not there yet" becomes "In raids they open wooden doors, and on Normal and Hard they sometimes chop at them".

**Commit 4, optional:** only if the review finds the door flapping (open, shut, open within two ticks) breaks paths. The fix is to recompute the vindicator's own path on its door toggles, as `set_door` does for villagers.

### Tests

Vendored entities (a test world with a door map; the fake `World::set_block` records calls):
- `door_goals::tests::a_door_breaking_zombie_knocks_cracks_and_breaks_a_wooden_door_after_240_ticks_on_hard`: crack stages 0 to 9 in order, every knock is event 1019, the swing count rises, and at tick 240 `set_block(upper, None)` and event 1021.
- `door_goals::tests::no_door_breaking_on_normal_without_mob_griefing_at_an_iron_door_or_without_can_break_doors`.
- `door_goals::tests::switching_to_normal_stops_the_break_and_clears_the_crack`.
- `door_goals::tests::opening_the_door_stops_a_break`, through a pending toggle in the same tick.
- `world::tests::a_zombie_that_can_break_doors_paths_through_a_wooden_door_and_one_that_cannot_goes_around`.
- `world::illagers::tests::a_raiding_vindicator_opens_a_wooden_door_and_shuts_it_behind`: `set_raids` with an active `RaidView` and `raid_id`, as at `world/illagers.rs:954`.
- `world::illagers::tests::a_vindicator_outside_a_raid_paths_around_doors_and_never_opens_them`.
- `world::illagers::tests::a_vindicator_near_a_raid_it_is_not_in_paths_through_doors_but_does_not_open_them`.

`minecraft_terrain`:
- `server::tests::a_zombie_breaks_a_door_and_the_door_drops`: `door_world()` at `server.rs:6076`, a zombie with `CanBreakDoors`, Hard; both halves gone, one `minecraft:oak_door` item; `take_destroyed` has the lower half; `level_events` has 1021 and at least one 1019.
- `server::tests::a_copper_door_breaks_and_an_iron_door_holds`.
- `level_events::tests::door_knocks_and_breaks_play_the_zombie_door_sounds`: volume 2.0, pitch within 0.8 to 1.2.

`minecraft_survival`:
- `session::tests::a_friend_sees_a_zombies_door_crack_grow_and_clear`: `level_cracks` with a mob crack, `send_cracks` emits stages under the mob key, then `-1`.

### Decisions (for the log)

1. **Door changes are applied after the mob's AI step.** Vanilla sets the door at once. `door_open` reads the pending changes, so later goals in the same tick see the same door state vanilla would.
2. **Drowned don't break doors yet.** In 26.3 any zombie kind on ground navigation can, drowned included. The fork's drowned run their own controller (`world.rs:1467-1900`), not `zombie_goals`. Zombies, husks, zombie villagers and zombified piglins get the goal.
3. **A mob's crack lasts only as long as the mob.** It is keyed per mob in its own breaker range and vanishes as soon as the mob is gone or stops. Vanilla clients keep an orphaned crack for up to 400 ticks.
4. **The last 2001 event is not sent.** Vanilla sends it after the block is already air, so it shows nothing. The particles and break sound come from the lower half's destroy, as in vanilla.
5. **A swing clock stands in for `isSwinging`.** The break goal keeps its own 6-tick clock and does not see swings from the melee goal.
6. **The vindicator's spawn-time door flag is not modelled.** `finalizeSpawn`'s `setCanOpenDoors(true)` is overwritten on the first tick.
7. **Kept as vanilla: vindicators seldom finish breaking a door.** `RaiderOpenDoorGoal` opens the door within the same goal pass, which stops the break.

### The owner's steps

Setup: `/difficulty hard`, `/gamerule keep_inventory true`, `/time set night`. On flat ground, build a closed room of oak planks, about 5 x 5 inside and 3 high, with a roof. Put one oak door in a wall and a glass window next to it. Stand inside, three blocks from the door, facing it.

- **E-DOOR-1.** Facing the door from 3 blocks inside, run `/summon minecraft:zombie ^ ^ ^6 {CanBreakDoors:1b,PersistenceRequired:1b}` (local coordinates: 6 blocks ahead, so 3 blocks outside the door). It walks to the door and swings its arms at it. Now and then a loud knock plays. Cracks spread over the upper half of the door. **Screenshot doors-1** (cracks about half way). About 12 seconds after it starts, the door breaks with the zombie break sound, wood bits fly, an oak door item drops, and the zombie walks in. **Screenshot doors-2.**
- **E-DOOR-2.** Put in a copper door and repeat: it breaks the same way. Put in an iron door and repeat: no knocks and no cracks, and the zombie waits outside.
- **E-DOOR-3.** Put in an oak door. Summon a door-breaking zombie and wait until the cracks show, then `/difficulty normal`: the knocking stops and the cracks vanish at once. The zombie presses against the door and never breaks it. `/difficulty hard`.
- **E-DOOR-4.** `/gamerule mob_griefing false` and summon another one: it never starts knocking. `/gamerule mob_griefing true`.
- **E-DOOR-5.** `/summon minecraft:zombie ^ ^ ^6` without the tag: it walks around the house looking for a way in and does not knock. Most zombies from eggs behave like this; about one in ten or fewer break doors.
- **E-DOOR-6.** Leave a door breaker outside, save and quit, then load: it starts knocking again, because the tag is saved.
- **E-DOOR-7. Raid.** `/locate structure minecraft:village_plains` and `/tp` there. With `/difficulty hard`, walk at least 60 blocks out, drink an ominous bottle and walk back. When wave 2 comes (its vindicators carry axes), stand inside a village house with its wooden door shut. A vindicator chasing you swings the door open and pushes in; the door swings shut behind it, with the door sounds. **Screenshot doors-3.** Sometimes cracks flash on a door and vanish as it opens.
- **E-DOOR-8.** Away from the raid (more than 100 blocks), hatch a vindicator egg outside a closed house with an oak door and stand inside: it walks around the house and never opens the door.
- **E-DOOR-MP (two windows).** Repeat E-DOOR-1 with the friend outside the house, within 30 blocks of the door. The friend sees the zombie swing, the cracks grow on the door and the break particles, and hears the knocks and the break. The friend can pick up the dropped door item. **Screenshot doors-4**, both windows.

```
IW4L_MINECRAFT_KIT=minecraft:zombie_spawn_egg*8,minecraft:oak_door*8,minecraft:copper_door*4,minecraft:iron_door*2,minecraft:oak_planks*64,minecraft:oak_planks*64,minecraft:glass*32,minecraft:torch*16,minecraft:vindicator_spawn_egg*2,minecraft:villager_spawn_egg*4,minecraft:ominous_bottle*2,minecraft:milk_bucket,minecraft:diamond_sword,minecraft:shield,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*16,minecraft:cooked_beef*32,iw4l:m4,iw4l:rifle_ammo*64
```

22 stacks. The `iw4l:` ids are the ones from the round 9 kits.

### Files shared

- Vendored `monster_ai.rs` (D0, M1-M3): one context field and its default, two goal lines, two names.
- Vendored `world.rs` (D0): the zombie tick at `:6178` and the AI-step tail, a `take_level_events` queue, `door_cracks()`.
- Vendored `illager_ai.rs` and `world/illagers.rs` (D0): slots 2 and 3, the `tick_ai` door flag, `tick_illager`.
- Vendored `world/villagers.rs` (W adds `BLOCK_OPEN`/`BLOCK_CLOSE` in `set_door_open`, so mob doors get W's vibrations for free): E routes its toggles through it and makes `recompute_for_block` `pub(super)`.
- Vendored `lib.rs`: one `mod` line.
- `level_events.rs` (W and D may add events), `server.rs` (one drain line), `session.rs` (`level_cracks`), `render_anim` `minecraft_world.rs:1550`.
- `docs/MINECRAFT-SURVIVAL.md`: the zombie and vindicator paragraphs.

---

## E-maps. Explorer maps in shipwreck, ocean ruin and camp chests

### The owner's finding

> "**Chests:** abandoned camps, shipwreck map chests and ocean ruins still roll nothing (explorer maps are missing)." (`docs/round9/release-2-tests.md:412`)

All five tables use `exploration_map`: `chests/abandoned_camp_common_chest`, `chests/abandoned_camp_secret_chest`, `chests/shipwreck_map`, `chests/underwater_ruin_big` and `chests/underwater_ruin_small` (`docs/round9/track-d-log.md:1258`, `:3146`).

### Cause

- `third_party/minecraftoss/core/src/loot.rs:840`: an unknown function returns `Err("loot function … is not supported")`. `exploration_map` has no match arm, so the whole roll fails.
- `crates/minecraft_terrain/src/containers.rs:712-715` (`structure_loot_in`): the roll ends in `.ok()?`, so one error leaves the chest empty. This is why the whole chest is empty, not only the map slot.
- `loot.rs:991-999` (`unknown_functions_still_fail`) uses `exploration_map` as its example of an unported function. `loot.rs:1213-1255` (the chest-table survey) allows that one function as the only exception.
- There is no map support anywhere. Nothing reads or writes `minecraft:map_id` except the `filtered` predicate (`loot.rs:604-605`). There is no map data, no map saving, no held-map view and no empty-map use. `creative_tabs.rs:2866` lists `filled_map` only as an item in no creative tab.
- The pieces we can reuse:
  - the hook pattern: `loot.rs:19-22` (the `Enchanting` trait), `loot.rs:24-39` (`LootParams.enchanting`) and its fork implementation `containers.rs:729-766`;
  - the structure search: `crates/minecraft_terrain/src/locate.rs:144-235` (`nearest_structure`, rings of `potential_chunk`, `generating_at` :114-125), which already takes a radius;
  - zoomed biomes: `terrain.rs:213-248` (`zoom::quart_for_block`) and `gen.terrain.biome_at_quart` (`locate.rs:379`);
  - world-root saved data: `third_party/minecraftoss/world/src/level_storage.rs:171-185` (`saved_data_path` and `write_saved` resolve `data/<namespace>/<path>.dat`);
  - reading classes from the jar at runtime: `minecraft_terrain/src/class_file.rs`, used by `creative_variants.rs` and `keyframes.rs`;
  - a dynamic Bevy image: `render_anim/src/minecraft_minimap.rs:60-80`;
  - a HUD overlay drawn from an image handle in `MinecraftUi`: `hud/src/minecraft_spyglass.rs` and `frame/src/minecraft_ui.rs:253-266`.
- Loot rolls only when a chest is opened, at `crates/minecraft_survival/src/menus.rs:299-323` (`open_station`). It is called for the host at `render_anim/src/minecraft_world.rs:1351` and for friends at `render_anim/src/minecraft_players.rs:1753`.
- The structures generate: shipwreck `map_chest` → `chests/shipwreck_map` (`generator/src/structure/kinds/shipwreck.rs:175`), ocean ruins (`ocean_ruin.rs:213`), buried treasure → `chests/buried_treasure` (`buried_treasure.rs:75`), abandoned camps (jigsaw).
- Chunk NBT writes empty `structures.starts` (`third_party/minecraftoss/core/src/chunk_nbt.rs:246`). Vanilla keeps structure `references` there, so there is nowhere vanilla-shaped to store which structures a map already points to.
- Cartographer trades have the same gap: `third_party/minecraftoss/entities/src/trading.rs:542-543` skips `exploration_map`, and the trade's `filtered` modifier then discards the map. Left for later.

### Vanilla 26.3

**`ExplorationMapFunction`** (`net.minecraft.world.level.storage.loot.functions`)
- Fields and defaults: `destination` (structure holder set, required), `decoration` (default `MapDecorationTypes.WOODLAND_MANSION`, registry id `minecraft:mansion`), `zoom` (byte, default 2), `search_radius` (int, default 50), `skip_existing_chunks` (bool, default true).
- `run(stack, ctx)`:
  1. An empty stack is returned as it is. With no `ORIGIN` the stack is returned as it is. The item is not checked.
  2. `pos = BlockPos.containing(origin)`.
  3. If `skip` is set, `claimStructureAt(level, pos)` runs: it takes the structure start at `pos` that matches `destination`, and when `isValid() && canBeReferenced()`, calls `addReference`. This stops a camp's own map from pointing at that camp.
  4. `found = level.findNearestMapStructure(destination, pos, searchRadius, skip)`. When nothing is found, the stack is returned unchanged; the `filtered` step after it then discards the map.
  5. `MapItem.applyNewSavedData(level, stack, found.x, found.z, zoom, trackingPosition=true, unlimitedTracking=true)` sets `minecraft:map_id`.
  6. `MapItem.renderBiomePreviewMap(level, stack)` runs.
  7. `MapItemSavedData.addTargetDecoration(stack, found, "+", decoration)` adds `minecraft:map_decorations` entry `"+"` = `{type, x: found.x, z: found.z, rotation: 180.0f}`. The `MapDecorations.Entry` codec fields are `type`, `x`, `z` and `rotation`.
- `ServerLevel.findNearestMapStructure` → `ChunkGenerator.findNearestMapStructure`: the same search `/locate` uses (radius 100, skip false). With `skip`, a structure counts only if `tryAddReference` succeeds: `StructureStart.canBeReferenced` is `references < getMaxReferences()`, and `getMaxReferences()` is `1`.

**Loot data**
- `shipwreck_map` has pool 1 = one `minecraft:buried_treasure_map` with `exploration_map{decoration: minecraft:red_x, destination: #minecraft:on_treasure_maps, skip_existing_chunks: false, zoom: 1}` then `filtered{map_id}` → `discard`.
- `underwater_ruin_big` gives the same map at weight 10 of 23 in its second pool, `underwater_ruin_small` at weight 5 of 12.
- `#on_treasure_maps` = `[minecraft:buried_treasure]`.
- The `buried_treasures` structure set is `random_spread`, spacing 1, separation 0, frequency 0.01, `legacy_type_2`, `locate_offset [9,0,9]`. The X marks the chest column.
- The abandoned camp tables use `set_name`, then `exploration_map{decoration: minecraft:abandoned_camp, destination: #minecraft:on_abandoned_camp_<biome>}` (skip defaults to true), under an inverted `location_check`. The secret chest also has ancient city, trial chambers, mineshaft, desert pyramid and jungle temple maps.

**Explorer maps are their own items** in 26.3. `Items` registers 17 `MapItem`s with `mapProperties()` = `MAP_DECORATIONS: MapDecorations.EMPTY`: `filled_map`, `ocean_monument_map`, `woodland_mansion_map`, `buried_trial_chambers_map`, `jungle_pyramid_map`, `swamp_hut_map`, `desert_village_map`, `plains_village_map`, `savanna_village_map`, `snowy_village_map`, `taiga_village_map`, `buried_treasure_map`, `buried_ancient_city_map`, `buried_mineshaft_map`, `desert_pyramid_map`, `abandoned_camp_map`, `warm_ocean_ruins_map`. Each has its own item model and texture. `MapColorComponentRemovalFix` shows the old `map_color` tint is gone.

**`MapItem`**
- `create(level, x, z, scale, tracking, unlimited)` makes a `filled_map`.
- `createNewSavedData`: `MapItemSavedData.createFresh(x, z, scale, …)`, then `getFreeMapId()`, then `setMapData`.
- `EmptyMapItem.use`: consumes 1 map and awards `ITEM_USED`; plays `ui.cartography_table.take_result` (volume 1, pitch 1, player's sound source); calls `MapItem.create(level, blockX, blockZ, 0, true, false)`; when the stack runs out, the result replaces it in the hand, otherwise it is added to the inventory or dropped.
- `inventoryTick`: `tickCarriedBy(player, stack, null)` for any carried map. When the map is not locked and is in a HAND slot, `update(level, player, data)` also runs.

**`MapItemSavedData`**
- `createFresh`: `i = 128 << scale`; `centerX = floor((x + 64.0) / i) * i + i/2 - 64` (same for z).
- Codec keys: `dimension`, `xCenter`, `zCenter`, `scale` (byte, default 0), `colors` (16384 bytes), `trackingPosition` (default true), `unlimitedTracking` (default false), `locked` (default false), `banners` (default []), `frames` (default []).
- Storage is the server-wide `MinecraftServer.getDataStorage()`, not per level. `MapId.key()` = `"maps/" + id`, so the file is `data/minecraft/maps/<id>.dat`. `MapIndex.TYPE` = `maps/last_id`, so the index is `data/minecraft/maps/last_id.dat` with field `map` (default -1). `getNextMapId` pre-increments, so the first map is id 0.
- Placing a decoration: `f = (worldX - centerX) / (1 << scale)`. `clampMapCoordinate`: f ≤ -63 → -128; f ≥ 63 → 127; otherwise `(byte)(int)(f*2 + 0.5)`. `isInsideMap`: |f| ≤ 63 on both axes. Rotation byte = `(int)((rot + (rot < 0 ? -8 : 8)) * 16 / 360)`.
- Non-player decorations show only inside the map, or anywhere when `unlimitedTracking` is set. Outside the map they are clamped to the edge.
- A player outside the map gets `player_off_map` when |f| < 320 on both axes, `player_off_limits` when `unlimitedTracking` is set, and no marker otherwise. In the Nether the player marker spins.
- Worked example: scale 1, target (1000, -300). The centre is (1088, -192) and the marker is x -87, z -107, rotation byte 8.
- `TRACKED_DECORATION_LIMIT` exists; `trackCount` types count towards it.

**`renderBiomePreviewMap`**
- `s = 1 << scale`. It samples `getBiome(((cx/s - 64) + col) * s, seaLevel, ((cz/s - 64) + row) * s)` for 128×128 cells and asks whether each biome is in `#minecraft:water_on_map_outlines` (`#is_ocean`, `#is_river`, `swamp`, `mangrove_swamp`).
- Inner cells 1..126 count their 8 watery neighbours, n.
- A watery cell is `COLOR_ORANGE` (id 15): n > 7 and an even column: brightness from `((row + (int)(Mth.sin(col) * 7)) / 8) % 5` (0 and 4 → LOW, 1 and 3 → NORMAL, 2 → HIGH); n > 7 and an odd column: LOWEST; n > 7 otherwise gives NONE (transparent); then n > 5 → NORMAL, n > 3 → LOW, n > 1 → LOW, else LOWEST.
- A dry cell with n > 0 is `COLOR_BROWN` (id 26): NORMAL if n > 3, else LOWEST.
- Packed byte = `id << 2 | brightness`. Brightness ids and modifiers: LOW 0 (180), NORMAL 1 (220), HIGH 2 (255), LOWEST 3 (135).
- `MapColor` has 62 colours (ids 0-61) in `MapColor.<clinit>`, as `(id, rgb)` pairs. ARGB = `rgb channel * modifier / 255`.

**`MapDecorationTypes`** (code registry, `register(id, asset, showOnItemFrame, trackCount)`): `player`, `frame`, `red_marker`, `blue_marker`, `target_x`, `target_point`, `player_off_map`, `player_off_limits`, `mansion` → asset `woodland_mansion`, `monument` → `ocean_monument`, 16 `banner_<colour>` → `<colour>_banner`, `red_x`, `village_<desert|plains|savanna|snowy|taiga>` → `<kind>_village`, `jungle_temple`, `swamp_hut`, `trial_chambers`, `abandoned_camp`, `ancient_city`, `desert_pyramid`, `mineshaft`, `ocean_ruin_warm` → `warm_ocean_ruins`. Their textures are `textures/map/decorations/<asset>.png` (all present in the 26.3 pack), next to `textures/map/map_background.png` and `map_background_checkerboard.png`.

**`MapRenderer`**: each decoration is `translate(x/2 + 64, y/2 + 64)`, then rotate `rot * 360/16`, then `scale(4, 4)`, then `translate(-0.125, 0.125)`: an 8×8 map-pixel sprite.

**First-person map**: `net.minecraft.client.renderer.FirstPersonHandsAndItemsRenderer` has `MAP_BACKGROUND`, `MAP_HANDS_POS_X`, `MAP_SWING_*` and the two-handed and one-handed map paths. Javap it for the exact poses and the background border.

**`MapItem.update`** (walk-to-fill): the radius is 128/scale blocks around the holder; a 16-column stripe per tick is chosen by `step & 15`; dimensions with a ceiling get the speckled pattern (`231871`, `31287121`, `11`, `20`; dirt ×10 vs stone ×100 on the multiset); it reads the `WORLD_SURFACE` heightmap; water depth gives brightness through the `0.1`/`0.2`/`0.5`/`0.9` thresholds, and land height delta through `4.0`/`0.4`/`0.6`/`-0.6`; `checkBanners` runs; `updateColor` marks dirty patches.

### Plan

#### Group `maps-loot` (L), four commits (early in E2: it fixes the empty chests)

1. **"Minecraft keeps map data in the vanilla 26.3 map files"**
   - New `crates/minecraft_terrain/src/maps.rs`: `MapData` with the codec fields above and a `colors: Box<[u8; 16384]>`; `create_fresh(x, z, scale, dimension, tracking, unlimited)`; the pure decoration placement (`clamp_coordinate`, `rotation_byte`, `inside`, `player_marker`); a `MapStore` holding loaded maps, `last_id` (starts at -1) and a dirty set; reading and writing `data/minecraft/maps/<id>.dat` and `data/minecraft/maps/last_id.dat` through `WorldSave` / `level_storage.write_saved`.
   - `Globals` (`crates/minecraft_survival/src/server.rs:36-56`) gains `maps: MapStore`, loaded at start, written by the save writer and autosave next to `weather.dat` (`save.rs`).
2. **"Minecraft shipwreck, ocean ruin and camp chests roll explorer maps"**
   - `core/src/loot.rs`: a `pub trait Exploring: Send + Sync + Debug` with `fn explore(&self, destination: &[String], origin: [i32; 3], radius: i32, skip_known: bool, zoom: u8) -> Option<(i32, [i32; 3])>` (map id and target); `LootParams.exploring: Option<Arc<dyn Exploring>>`; an `exploration_map` arm next to `filtered`. It reads the defaults above and resolves `destination` with `holder_set("tags/worldgen/structure", …)`. On success it sets `minecraft:map_id` (Int) and merges `minecraft:map_decorations` `{"+": {type, x, z, rotation: 180.0f}}`. Without an origin, a hook or a result, it returns the stack unchanged.
   - Switch `unknown_functions_still_fail` to a function that is still unported, picked from the survey. In the survey test, drop the `exploration_map` exception.
   - Fork side, in `maps.rs` or `containers.rs`: a `LootExplorer` holds `Arc<WorldGen>`, the dimension, the next free id and a `Mutex<Vec<(i32, MapData)>>` of new maps. It calls `locate::nearest_structure(gen, targets, origin, radius, deadline)` with a 2 s deadline; a timeout counts as not found and is logged.
   - After the roll, `structure_loot_in` returns the new maps. `open_station` (`menus.rs:299-323`) moves them into `globals.maps` and commits `last_id`, so it needs `&mut` access to the map store. Pass the explorer in from `menus.rs`, where `terrain.stream.world_gen()` is at hand.
   - Update the `structure_loot` / `structure_loot_in` signatures (`containers.rs:667-727`) and their tests.
3. **"Minecraft explorer maps show the coastline around their target"**
   - Port `renderBiomePreviewMap` into `maps.rs` as `biome_preview(&mut MapData, watery: impl Fn(x, z) -> bool)`.
   - The fork closure samples `zoom::quart_for_block(zoom_seed, x, sea_level, z)`, then `gen.terrain.biome_at_quart`, cached by quart (only about 66×66 unique quarts are needed), and checks `#water_on_map_outlines` through `locate::datapack_tags`.
   - The explorer runs it right after creating the map.
4. **"Minecraft explorer maps skip structures another map already points to"**
   - `skip_known` in the search (a variant of `generating_at` that asks a claim set); `claimStructureAt` for the chest's own structure; claims kept per level as `(structure id, start chunk) → references`, max 1.
   - The claims go in a fork file, `dimensions/minecraft/<dim>/data/iw4l/map_claims.dat`, which vanilla ignores.
   - Shipwrecks and ruins use `skip false`; camps use the default `true`.

#### Group `maps-held` (M), two commits (commits 1-2), then group `maps-view` (M), one commit (commit 3)

Split so that one agent does the server markers and the wire, and a second the first-person picture.

1. **"Minecraft maps mark their holder and their target"**
   - Port `tickCarriedBy` for every map stack in a player's inventory: holder markers (`player`, `player_off_map`, `player_off_limits`, Nether spin), the `minecraft:map_decorations` entries added once carried, `trackCount` and the dimension check. Hook it where the server ticks player inventories.
   - The decorations live on the server `MapData` and are not saved, as in vanilla (only banners and frames are saved).
2. **"Minecraft friends get the pictures of the maps they carry"**
   - `ToClient::MapData { id, scale, locked, decorations: Vec<(type, x, y, rot)>, patch: Option<(x, y, w, h, Vec<u8>)> }` in `crates/minecraft_net/src/messages.rs` and `codec.rs`, inserted next to a related variant with its own codec tag. No `PROTOCOL` bump in the commit.
   - A per-player dirty patch (vanilla `HoldingPlayer`). The first send is the full 128×128.
   - A client `MapCache` filled straight from `globals.maps` on the host and from messages on a friend.
3. **Group `maps-view`. "Minecraft shows the held map in first person"**
   - Compose an RGBA image at 2× (so half-pixel positions stay sharp): `map_background.png`, the colours through the 62-entry palette read from `MapColor.<clinit>` with `class_file.rs`, then the decoration sprites from `textures/map/decorations` at 8×8 map pixels, rotated by `rot*22.5°`.
   - Draw it the way the spyglass scope is drawn: a new `MinecraftUi.held_map` field (next to the spyglass fields in `frame/src/minecraft_ui.rs`) and a HUD job in `hud/src`. Two-handed (main hand, empty offhand) or one-handed, following `FirstPersonHandsAndItemsRenderer`. Hide the hand item mesh for that stack in `render_anim/src/minecraft_hand.rs` `held_hand_mesh`: one `continue` for a `filled_map` (or explorer map) stack right after the `let Some((slot, stack)) = stack else { continue; };` at `:176-178`. E1 edits line 195 only.

#### Group `maps-colours` (M), one commit (commit 1), then group `maps-explore` (M), two commits (commits 2-3)

Parsing `Blocks.<clinit>` is a large job on its own, so it is its own group.

1. **Group `maps-colours`. "Minecraft reads block map colours from the 26.3 jar"**: parse `Blocks.<clinit>` with `class_file.rs`, following `keyframes.rs` / `creative_variants.rs`. Cover `Properties.mapColor(MapColor)` and `mapColor(DyeColor)`, the state lambdas for logs (axis), beds (part) and the rest, `ofFullCopy` / `ofLegacyCopy`, and the helper builders (logs, leaves, stems, candles, shulker boxes, stained glass). Missing means `NONE`. The result is a per-state colour id table.
2. **"Minecraft maps fill in as their holder walks"**: port `MapItem.update` over loaded chunks (`HandcraftedScene`) for maps held in a hand: stripe stepping, water depth, height shading and the ceiling speckle. Mark the patches dirty so `maps-held` sends them.
3. **"Minecraft empty maps turn into maps of where you stand"**: `minecraft:map` use in `crates/minecraft_survival/src/actions.rs` as `EmptyMapItem.use`: scale 0, tracking on, unlimited off, the sound, then held-item replacement or adding to the inventory.

### Tests

- `core::loot`: with a stub `Exploring`, `exploration_map` writes `map_id` and the `"+"` red_x entry (rotation 180.0); the defaults parse as `minecraft:mansion`, zoom 2, radius 50 and skip true; with no origin, the map is discarded by `filtered`; the `MINECRAFTOSS_ROOT` survey: every chest, spawner, equipment and gameplay table rolls with no unsupported function at all.
- `maps.rs`: `create_fresh(1000, -300, scale 1)` → centre (1088, -192), `(0, 0, scale 0)` → (0, 0); target (1000, -300) → bytes (-87, -107) with rotation 8; f = ±63 clamps to -128 / 127; off-map at 300 map units, off-limits at 400 with unlimited set, no marker without it; NBT round trip with the vanilla keys, the first id is 0 and `last_id.dat` holds `map: 0`; `biome_preview` on a hand-made watery grid gives the packed orange and brown bytes from the formula, including the `sin` stripe.
- Fork, gated on `MINECRAFTOSS_ROOT`: rolling `chests/shipwreck_map` at a seeded ocean origin gives a `minecraft:buried_treasure_map` with `map_id`; its target equals `nearest_structure(["minecraft:buried_treasure"], origin, 50)`, and that chunk's generated block entities hold a `chests/buried_treasure` chest in that column. Two `skip_known` rolls at one origin point to different starts, and the claims survive a save and load.
- `maps-held`: the `MapData` message codec round trip; the patch is only the dirty rectangle; the palette: GRASS (id 1, `0x7FB238`) at NORMAL = (109, 153, 48); the overlay layout picks two-handed only with an empty offhand.
- `maps-explore`: every 26.3 block state gets a colour id; spot checks `grass_block` 1, `sand` 2, `stone` 11, `water` 12, `oak_log[axis=y]` 13 and `[axis=x]` 34 (PODZOL), red bed head 3 (WOOL); a flat grass stub fills the stripe for `step & 15` only; an empty map leaves 1 fewer map and a `filled_map` with scale 0 centred by `create_fresh`.

### Decisions (for the log)

- Our chunks save no structure starts (`chunk_nbt.rs:246`), so map claims go in `dimensions/minecraft/<dim>/data/iw4l/map_claims.dat`.
- The held map is a screen overlay at vanilla's place, not a textured quad in the 3D hand mesh. The hand mesh only samples the block atlas.

### Left for later (in `docs/MINECRAFT-SURVIVAL.md` limitations)

- Cartographer explorer-map trades (`trading.rs:542-543`). Trades are rolled in the entity sim, which has no `WorldGen`, so they need an async search hook.
- Maps in item frames (frame markers, `showOnItemFrame`).
- The cartography table: copy, scale, lock.
- Banner markers on maps.
- The map tooltip (`filled_map.id`, scale, locked).

### The owner's steps

Host (screenshots `E-maps-N`):
1. Load a world with the kit. Run `/locate structure minecraft:shipwreck` and `/tp` to it. Drink water breathing.
2. Open the shipwreck's chests. The map chest is usually in the cabin at the back. It now holds paper, feathers, a compass, map or clock, and a **Buried Treasure Map** (**E-maps-1**). If this wreck has no map chest (half wrecks), locate the next one.
3. Run `/locate structure minecraft:ocean_ruin_warm` (or `ocean_ruin_cold`) and open the ruin chests. They roll coal, wheat and the like again, and about 4 in 10 hold a Buried Treasure Map (**E-maps-2**).
4. (`maps-held`.) Hold the treasure map with nothing in the offhand. A map fills the lower screen with orange coastline outlines, a red X and your white arrow, which sits at the edge when you are off the map (**E-maps-3**). Put a torch in the offhand: the map moves to one side (**E-maps-4**).
5. Boat to the X, watching the arrow move. Dig down at the X with the shovel. A chest under the sand holds a heart of the sea (**E-maps-5**).
6. Save and quit. Check that `data/minecraft/maps/0.dat` and `data/minecraft/maps/last_id.dat` are in the world folder. Reload, and the map still shows the X (**E-maps-6**).
7. (`maps-explore`.) Right-click an empty map. It turns into a Map of where you stand and fills in as you walk (**E-maps-7**). Do the same in the Nether for the speckled pattern (**E-maps-8**).
8. (`maps-explore`.) Open a second treasure map's chest elsewhere and walk into the mapped area. The terrain fills in around the X (**E-maps-9**).

Friend:
1. Join with the friend's install. Open a different shipwreck map chest. The map lands in the friend's inventory (**E-maps-F1**).
2. Hold it. The friend sees the same coastline, X and their own arrow (**E-maps-F2**).
3. The host drops their map with Q and the friend picks it up. Both now see the same map, with both arrows when they are near each other (**E-maps-F3**).

```
IW4L_MINECRAFT_KIT=minecraft:potion@long_water_breathing,minecraft:potion@long_water_breathing,minecraft:potion@long_water_breathing,minecraft:potion@night_vision,minecraft:potion@night_vision,minecraft:oak_boat,minecraft:iron_shovel,minecraft:diamond_shovel,minecraft:diamond_pickaxe,minecraft:map*8,minecraft:compass,minecraft:torch*64,minecraft:cobblestone*64,minecraft:cooked_beef*32,minecraft:turtle_helmet,minecraft:diamond_sword,minecraft:shield,minecraft:red_bed,minecraft:spyglass
```

19 stacks.

### Files shared

- `core/src/loot.rs` (D: vault and trial spawner loot), `containers.rs` (D vaults), `menus.rs` `open_station` (E1 edits `merchant_op`/`merchant_local`; D vault menus), `minecraft_survival` `server.rs` `Globals` and `save.rs` (anyone adding saved state), `locate.rs` (D trial chamber tests).
- `minecraft_net` `messages.rs`/`codec.rs` (`ToClient::MapData`).
- `frame/src/minecraft_ui.rs` (`held_map`; E1 adds `vehicle_health` and `McStack.effects`) and `hud/src` job registration in `gpu_list.rs` (F boss bar).
- `render_anim` `minecraft_world.rs`, `minecraft_players.rs` and `minecraft_hand.rs` (everyone).
- `actions.rs` item use (D lodestone compass, W).

---

## E-leads-cats. Punching off lead knots, cats keeping chests shut

### Item 1. Lead knots can be punched off (and cut with shears)

**The owner's finding** (`docs/round9/release-2-tests.md:408`): "Leads: a knot can't be punched off; break the fence and the mob is free within 5 seconds."

**Cause**
- In our game a knot is not an entity. It is a fence position in the leash store: `self.knots`, read by `has_knot` and `knots()` in `third_party/minecraftoss/entities/src/world/leashes.rs:365-379`. Nothing can hit it.
- A left click goes through `crates/minecraft_survival/src/actions.rs:373-432`, in this order: `punch_player` (`:399`), then `entities.punch` (`:413`), then a miss or `hand.dig.press()` (`:419-431`). `Entities::punch` (`entities.rs:862-910`) checks item frames, End targets, shulker bullets, fireballs and mobs, never knots. So the click falls through to digging and the fence is dug.
- The digging paths decide between a block and an entity with `mob_distance` (`entities.rs:912-916`, used at `actions.rs:502-504` for creative and `:528` and `:553` for survival). Knots are not part of that distance.
- A right-click on a fence with a knot goes to `leash_fence` (`actions.rs:775-784` → `entities.rs:1297-1304` → `ServerHandle::leash_fence` `server.rs:3980` → `Command::LeashFence` `:3460`, `:4154` → `ServerSim::leash_fence` `:1477-1481` → `EntityWorld::leash_fence` `leashes.rs:659-699`). That function has no shears branch.
- The only way a knot goes today is `tick_knots` (`leashes.rs:719-736`): every 100 ticks, if the fence is gone, the knot is removed with `item.lead.untied` and each mob drops its lead (`drop_leash(mob, true)`, `:551-568`). `ServerSim::drop_leads` (`server.rs:1471-1475`) then spawns the lead items.
- Friends: a friend's input runs through the same `actions::act` on the host (`render_anim/src/minecraft_players.rs:1923`). The friend sees knots through the mobs' `leash` look tag. One fix covers both sides, and the protocol does not change.

**Vanilla 26.3**
- `LeashFenceKnotEntity extends BlockAttachedEntity`. Its box is 0.375 x 0.5. `recalculateBoundingBox` puts it at (x + 0.5, y + 0.375, z + 0.5), so the box runs from (x + 0.3125, y + 0.375, z + 0.3125) to (x + 0.6875, y + 0.875, z + 0.6875). The fence post's outline is 6/16 to 10/16 across, so the knot box sticks out 1/16 on each side of the post. From the side, the ray meets the knot first. From straight above, the post's top (y + 1) is met first, so the block wins.
- `BlockAttachedEntity.isPickable` is true. `Player.attack` → `cannotAttack` → `skipAttackInteraction(player)`. That returns true (attack skipped) when `!level.mayInteract(player, pos)`, which is spawn protection on dedicated servers only. Otherwise it calls `hurtOrSimulate(playerAttack, 0.0)`.
- `BlockAttachedEntity.hurtServer`: returns false if the entity is invulnerable to the source, or if `mob_griefing` is off and the source's entity is a `Mob`. Otherwise `kill(level, source entity)`: `remove(KILLED)` and the `ENTITY_DIE` game event, then `markHurt`, then `dropItem`. `LeashFenceKnotEntity.dropItem` only plays `LEAD_UNTIED` at volume 1 and pitch 1; it drops no item itself.
- Because `cannotAttack` returns early, the player gets no exhaustion and the weapon takes no wear. The client still calls `resetAttackStrengthTicker`.
- The mobs let go on their next tick: in `Leashable.tickLeash`, a holder that is `!canInteractWithLevel()` (removed) leads to `dropLeash()` when `entity_drops` is on (a lead item at each mob), else `removeLeash()`.
- Shears on a knot (`LeashFenceKnotEntity.interact`): shears first go to `Entity.interact` → `shearOffAllLeashConnections(player)` → `dropAllLeashConnections`. Every mob tied to the knot gets `dropLeash()` (a lead each) and the `SHEAR` game event fires. Then `SHEARS_SNIP` plays at the knot's block in the player's sound source, and `hurtAndBreak(1)` wears the shears. The knot discards itself when its last mob leaves; there is no untied sound on that path. With no mobs tied, the click falls through to the normal knot interact (tie or hand back).
- Sounds `item.lead.untied` and `item.shears.snip` are in the 26.3 `sounds.json`. The tag `#fences` is `#wooden_fences` (13 woods, poplar included) plus `nether_brick_fence`; `is_fence` (`ends_with("_fence")`) already matches all of them.

**Plan: group `lead-knots` (M), two commits**

Commit 1: **"Minecraft lead knots can be punched off"**
- Vendored `world/leashes.rs`: `pub fn break_knot(&mut self, pos: Pos) -> bool`. It removes the knot, plays `UNTIED_SOUND` at `knot_position(pos)` (`friendly_volume`), runs `drop_leash(mob, true)` for every mob tied to it, drops (with a lead) any leash still only saved to `SavedLeash::Knot(pos)`, and returns whether anything was there. `tick_knots` then calls it, with the same behaviour as before.
- `crates/minecraft_terrain/src/leash_render.rs`: `pub fn knot_box(pos) -> (DVec3, DVec3)` from `knot_position`, `KNOT_WIDTH` and `KNOT_HEIGHT`, and `pub fn knot_on_ray(knots: &[BlockPos], eye, look, reach) -> Option<(BlockPos, f64)>` using `crate::end_crystal::ray_box` (`end_crystal.rs:158`), like `item_frame::on_ray` (`item_frame.rs:630`).
- `server.rs`: `Command::Knot { player, pos, action: KnotAction }` with `enum KnotAction { Punch }`, next to `LeashFence`; `ServerHandle::knot_action`; `ServerSim::knot_action`, which calls `mobs.break_knot(pos)` and then `drop_leads()` (pattern `leash_fence` `:1477`).
- `entities.rs`: `pub fn punch_knot(&mut self, player, eye, look, reach, block: Option<f64>) -> bool`. It fires when a knot from `self.view.world.knots()` is on the ray and nearer than the block hit and nearer than `mob_distance` (mobs, frames, players, End targets), then sends `KnotAction::Punch`. Add the knot distance into `mob_distance` (`:912`), so neither the creative instant break nor survival digging hits the fence behind a knot while the knot is in the view.
- `actions.rs:373-432`: try `entities.punch_knot(...)` first in the attack press, before `punch_player`. On a hit, set `punched = true` and `hand.attack_ticks = 0`, with no `exhaust` and no weapon wear (vanilla's `cannotAttack` path).

Commit 2: **"Minecraft shears cut every lead off a lead knot"**
- `leashes.rs`: `pub fn shear_knot(&mut self, pos: Pos) -> bool`. Every mob tied to the knot (or saved to it) runs `drop_leash(mob, true)`, and `SNIP_SOUND` plays at the block centre (`players_volume`). The knot goes through `forget_empty_knot`, with no untied sound. It returns false when nothing was tied.
- `server.rs`: `KnotAction::Shear` → `mobs.shear_knot(pos)`, then `drop_leads()`.
- `entities.rs`: `pub fn shear_knot(&mut self, player, pos) -> bool`, tried on a copy of `view.world` first like `leash_fence` (`:1297`), and sent only when the copy says true.
- `actions.rs`, before `leashed_fence` (`:775`): when `main`, the held item is `minecraft:shears`, the targeted block is a fence, `has_knot(pos)` and `shear_knot(...)` returns true: swing, and in survival `inventory.wear_tool(slot, 1)`, pushing `"minecraft:shears"` to `entities.broken_items` if it breaks (pattern `mob_actions.rs:164-168`). Gate `leashed_fence`, the bell and `used` on `!sheared`.

**Tests**
- Vendored `world::leashes::tests`: `punching_a_knot_frees_its_mobs_with_a_lead_each_and_the_untied_sound` (two cows on one knot; `break_knot` makes `knots()` empty, gives two `take_lead_drops`, plays `item.lead.untied` and leaves both cows with no holder); `a_knot_known_only_from_a_save_can_be_punched_off_too`; `shears_cut_every_lead_off_a_knot_with_a_snip_and_no_untie`; `shears_on_an_empty_knot_do_nothing`; `a_knot_lets_its_mobs_go_at_its_check_once_its_fence_is_gone` stays green after the refactor.
- `leash_render::tests::a_knot_is_hit_from_the_side_before_its_fence_post_but_not_from_above`: compare `knot_on_ray` with the fence's outline hit through `doors::Outlined` on a scene, or with the post box 0.375-0.625.
- `server::tests::punching_a_knot_drops_a_lead_for_each_tied_cow_and_leaves_the_fence` (pattern of `server.rs:12255`): the fence block is still there, two `minecraft:lead` items lie by the cows, and the save has no `leash` tag.
- `actions::tests` (gated on game data, like `a_door_opens_when_used` `:2129`): `a_left_click_at_a_knot_breaks_the_knot_and_not_the_fence` (no `Effect::Dig`, `hand.dig` not pressed, no exhaustion); `a_left_click_at_the_post_below_the_knot_still_digs_the_fence`; `shears_on_a_knot_cut_the_leads_and_wear_one_point`.

### Item 2. A cat sitting on a chest keeps it shut

**The owner's finding** (`docs/round9/release-2-tests.md:409`): "Cats: a cat on a chest does not keep it shut." Round 9 left it for later (`docs/round9/track-d-log.md:3128`): "the level's containers cannot see the entity world's cats yet".

**Cause**
- The open check is `actions.rs:833-873`. For `Station::Chest` and `Station::Ender` it only checks for a redstone conductor above any cell (`conductor`, `:835-848`). Cats are never asked.
- `Station::Chest` covers `chest`, `trapped_chest` and the 8 copper chests (`containers.rs:56-74`). That matches vanilla, where `TrappedChestBlock` and `CopperChestBlock` extend `ChestBlock`.
- The cat's sitting pose is `Feline.sitting` (`third_party/minecraftoss/entities/src/feline.rs:139`): `SitGoal` sets it on `start` and clears it on `stop` (`feline_ai.rs:809-815`); `BlockGoal` sets `sitting = reached` (`feline_ai.rs:1081`) and clears it on `start` and `stop`; `BlockGoal` needs a tame cat that is not ordered to sit (`:1015`), as vanilla does.
- The host's `Entities.view.world` is the server's mob world, copied each tick (`entities.rs:1674`). The felines store is `EntityWorld::felines()` (`world/felines.rs:345`).
- `can_sit_on` (`feline_ai.rs:756-774`) treats every `minecraft:chest` as closed (round 9 decision). Vanilla skips a chest with an opener.
- The server's level already knows each chest's openers: `Level.chest_openers` (`world/src/level/mod.rs:206`), set by `ServerSim::set_chest_openers` (`server.rs:1744`) from `Command::ChestOpeners`.
- Friends open chests through the host's `act`, so the same check covers them.

**Vanilla 26.3**
- `ChestBlock.useWithoutItem` → `getMenuProvider` → `combine(state, level, pos, false)`. The blocked predicate is `ChestBlock::isChestBlockedAt`, and `DoubleBlockCombiner.combineWithNeigbour` tests it on the clicked half and, for a double chest, on the other half. Either half blocked means no menu. The use still returns `SUCCESS`: the arm swings, no screen, no sound, no stat. `getContainer(..., true)` (hoppers, comparators) ignores the block.
- `isChestBlockedAt = isBlockedChestByBlock || isCatSittingOnChest`. `isBlockedChestByBlock`: the block above `isRedstoneConductor` (we have this). `isCatSittingOnChest`: `getEntitiesOfClass(Cat.class, new AABB(x, y + 1, z, x + 1, y + 2, z + 1))`, and any of them `isInSittingPose()`. The AABB overlap is strict, so a cat on the floor beside the chest does not count. Ocelots do not count (only the `Cat` class), and lying does not count.
- `EnderChestBlock` only checks the block above, not cats.
- `CatSitOnBlockGoal` (speed 0.8, range 8): `canUse = isTame && !isOrderedToSit && MoveToBlockGoal.canUse`. `isValidTarget`: air above, `#cats_can_sit_on` (`furnace`, `chest`, `#beds`) and `BLOCK_TYPE_VALIDATION`: for `Blocks.CHEST` (the plain chest only) `ChestBlockEntity.getOpenCount(level, pos) < 1`; furnace `lit`; beds not the head. `MoveToBlockGoal.canContinueToUse` re-checks `isValidTarget`, so a cat stands up and leaves a chest once someone has it open.

**Plan: group `cat-chests` (S), two commits**

Commit 1: **"Minecraft cats sitting on a chest keep it shut"**
- Vendored `world/felines.rs`: `pub fn cat_sits_on(&self, pos: Pos) -> bool`: any feline with `is_cat()` and `feline.sitting` whose body box (position ± width/2, y to y + height) strictly overlaps (x, y + 1, z)-(x + 1, y + 2, z + 1).
- `entities.rs`: `pub fn cat_on_chest(&self, pos) -> bool`, returning `self.view.world.cat_sits_on(pos)`.
- `actions.rs:845-848`: split `Station::Chest` from `Station::Ender`. A chest is blocked if any cell has a conductor above it or `entities.cat_on_chest(cell)`. The ender chest keeps the conductor check only. The swing stays.

Commit 2: **"Minecraft cats leave chests that someone has open"**
- Vendored `player/src/lib.rs`: the `World` trait (`:95`) gets `fn chest_open_count(&self, _pos: Pos) -> i32 { 0 }`.
- `crates/minecraft_terrain/src/server_mobs.rs`: the `MobWorld` impl (`:242` area) overrides it from `level.chest_openers` (pub).
- `feline_ai.rs:765`: `minecraft:chest` becomes `world.chest_open_count(pos) < 1`. `BlockGoal::can_continue` already re-checks `valid`, so a cat whose chest is opened stands up.

**Tests**
- Vendored `world::felines::tests`: `a_sitting_cat_on_a_chest_is_found_but_not_a_standing_or_lying_cat_nor_an_ocelot`; `a_cat_sitting_on_the_floor_beside_a_chest_does_not_count`; `a_tame_cat_does_not_sit_on_an_open_chest_and_gets_up_when_it_is_opened` (a test `World` with `chest_open_count`; the round 9 test `a_tame_cat_lies_down_on_a_bed` stays green).
- `actions::tests` (gated): `a_chest_with_a_sitting_cat_on_it_stays_shut_but_the_arm_swings` (`open_block` is `None` and `hand.swing` is set); `either_half_of_a_double_chest_with_a_cat_on_it_keeps_both_shut`; `an_ender_chest_ignores_cats`.
- `server_mobs::tests::the_mob_world_reports_a_chest_s_openers`.

### Decisions (for the log)

- A knot stays a fence position, not an entity. The punch is tested as a ray against the vanilla knot box, nearer than both the block outline and every entity.
- A punched knot plays the untie sound and drops one lead per mob, as vanilla's `dropItem` and `tickLeash` do. Shears snip with no untie sound.
- No new game events (`ENTITY_DIE`, `SHEAR`, `BLOCK_ATTACH`): track W wires game events.
- No protocol change: friends' clicks run on the host, and the knot leaves friends' views with the mobs' `leash` look tag.
- Cats block chest menus only. Hoppers and comparators ignore them, as vanilla's `ignoreBlocked` does.

### Left for later

- Arrows, tridents and thrown items breaking knots (the projectile sites, D0's generic arms).
- Explosions breaking knots (`ignoreExplosion`, `push`).
- MW2 bullets passing through knots.
- The `entity_drops` and `mob_griefing` rules on knots.

### The owner's steps (survival, by day, on flat grass)

Leads (`lead-knots`):
- **L-1.** Use the cow egg twice. Put a lead on each cow and right-click an oak fence: a knot appears on the post with two ropes.
- **L-2.** Stand 2 blocks from the fence, aim at the knot from the side (the upper half of the post), and left-click once with the sword. The knot vanishes with the untie sound. A lead drops by each cow and both cows are free. The fence is still there and you are no hungrier. **Screenshot E-L1** (fence without knot, two leads on the ground).
- **L-3.** Tie a cow to the fence again. Left-click the bottom third of the post, below the knot (the knot box starts 0.375 above the block): the fence is dug as before, and the cow goes free within 5 seconds.
- **L-4.** Tie both cows to a nether brick fence. Hold the shears and right-click the knot: snip sound, two leads drop and the knot is gone. A fresh pair of shears now shows a durability bar (one point of 238 used; the bar itself looks full). There is no untie sound. **Screenshot E-L2.**
- **L-5.** Right-click a fence knot with the shears when nothing is tied there: nothing is cut. Then with an empty hand and a cow on your lead: it ties as before.
- **L-6.** `/gamemode creative`. Tie a cow, left-click the knot: the knot goes, the fence stays and a lead drops. `/gamemode survival`.
- **L-7.** Tie a cow, punch the knot off, save and quit, load. The cow is loose and there is no knot.
- **L-8. Friend window.** The host ties a cow to a fence and the friend punches the knot. Both see the knot vanish within half a second and the lead drop. **Screenshot E-L3** (the friend's view). The friend ties their own cow and cuts the knot with shears; the friend's fresh shears now show a durability bar.

Cats (`cat-chests`):
- **C-1.** Use the cat egg and tame it with cod until it sits with a collar. Right-click it with an empty hand so it stands.
- **C-2.** Place two chests side by side (a double chest) 2 blocks from the cat. Within about 10 seconds it walks onto one half and sits.
- **C-3.** Right-click the other half: the arm swings, the chest stays shut and makes no sound. Right-click the half under the cat: the same. **Screenshot E-C1.**
- **C-4.** Right-click the cat twice (sit, then stand) and walk 15 blocks away so it follows you off the chest. Go back and open the chest: it opens.
- **C-5.** Cats only sit on plain chests by themselves (`#cats_can_sit_on`), so put one on the others by command. Place a trapped chest, jump onto it and run `/summon minecraft:cat ~ ~ ~ {Sitting:1b}` (it loads sitting, `server_mobs.rs:1069-1070`), then step off: the trapped chest stays shut. Do the same on a copper chest: it stays shut. Do the same on an ender chest: it still opens (vanilla checks only the block above an ender chest).
- **C-6.** Open a chest and stay in its screen while the standing tame cat is near: it does not climb onto it. Close it: within about 10 seconds it does.
- **C-7. Friend window.** The cat sits on the host's chest and the friend right-clicks it: it stays shut for the friend too. **Screenshot E-C2** (the friend's view). The friend holds the chest open while the host watches the cat: it does not sit on it.

```
IW4L_MINECRAFT_KIT=minecraft:lead*16,minecraft:cow_spawn_egg*4,minecraft:oak_fence*16,minecraft:nether_brick_fence*8,minecraft:shears,minecraft:iron_sword,minecraft:wheat*16,minecraft:cat_spawn_egg*4,minecraft:cod*64,minecraft:chest*4,minecraft:trapped_chest*2,minecraft:copper_chest*2,minecraft:ender_chest,minecraft:cooked_beef*16
```

14 stacks.

### Files shared

- Vendored `world/leashes.rs`: D0 (per-kind leash sites such as `can_be_leashed`, `leash_yaw` and `body_of`), M1-M3 (new leashable mobs). This section only adds functions plus a small refactor of `tick_knots`.
- `entities.rs` (`punch`, `mob_distance`, `entity_distance`): D0 (generic hit arms), F (a boss target on the ray).
- `actions.rs`: the attack press `:373-432`, the fence and bell block `:775-797`, the chest check `:833-873` (W adds player-action game events; E1 edits other functions).
- `server.rs` `Command` and `ServerHandle`: every track.
- Vendored `player/src/lib.rs` `World` trait: W and D (default-method additions only).
- `server_mobs.rs` `MobWorld` impl: W, D0.
- Vendored `feline_ai.rs` and `world/felines.rs`: the M track with phantoms (cats scare and hiss at phantoms).
- The vendored patch list in the track log: `leashes.rs`, `felines.rs` and `feline_ai.rs` are fork-added; `player/src/lib.rs` is patched.

---

## E-fireworks. Players and dispensers launch rockets, rockets burst on what they hit, bursts knock mobs back

### The owner's finding

> **Fireworks:** only celebrating villagers launch rockets. A rocket does not burst on a mob in its path, and mobs hurt by a burst are not knocked back. (`docs/round9/release-2-tests.md:417`)

Round 9 also left these for later (`docs/round9/track-d-log.md:3160`): the other burst shapes, trails and twinkles, saving rockets in flight, players launching rockets from the ground, and dispensers and crossbows firing them.

### Cause

- **Only villagers make rockets.** `ServerSim::raid_villager_events` (`server.rs:1457-1460`) is the only place that pushes a `Rocket`, from `EntityWorld::take_fireworks` (`third_party/minecraftoss/entities/src/world/raiders.rs:182`).
- **Players can't launch rockets.** `firework::rocket_use` (`crates/minecraft_terrain/src/firework.rs:270-286`, called at `actions.rs:1131`) only works while gliding. Its `Change::Rocket` (`actions.rs:1173`) only boosts the body (`body.launch_rocket`). No rocket entity exists, so nothing bursts when the boost ends. There is no `useOn` for clicking a block.
- **Dispensers and crossbows don't fire them.** `firework_rocket` is in the dispenser's `OTHER_BEHAVIOURS` list (`third_party/minecraftoss/world/src/level/dispenser.rs:21`), so a dispenser only logs it as unsupported (`:106-108`). Players have no bow or crossbow at all.
- **No entity hits.** `Rocket` (`crates/minecraft_terrain/src/firework_rocket.rs:31-41`) keeps the colours of one star and a mob id as `owner`. `Rocket::tick` (`:70-90`) only clips blocks (`sight::clip`, `:79`), so nothing tests mobs or players. It also bursts on any block hit (`:85`), stars or not, and always speeds up by 1.15, even against a wall.
- **Damage ignores the stars.** `Rocket::damage_to` (`:113-126`) hard-codes one star (`let explosions = 1`, `:114`), so a starless rocket would still hurt.
- **No knockback, no owner.** `ServerSim::rocket_blast` (`server.rs:1422-1443`) hurts mobs with `self.mobs.player_blast(None, id, damage, rocket.position, 0.0)` (`:1438`): impact 0.0 means no push; the owner is dropped, so the owner gets no kill credit and the mob has no one to retaliate against; the damage type is `minecraft:explosion` (`Blaster::damage_type`, vendored `world.rs:1118-1130`), not `minecraft:fireworks`. `hurt_by_blast` (`world.rs:2992`) never applies the default knockback. Players already get the knockback, through `PlayerHitKind::Fireworks { at }` (`world.rs:627`; `entities.rs:1890`, `:1897`).
- **One burst shape for everything.** `FireworkParticles::burst` (`crates/minecraft_terrain/src/firework_particles.rs:97-121`) draws only vanilla's BURST shape with all the colours. A burst without colours is drawn black (1973019, `:101`) and still bangs.
- **Starred rockets can't be crafted.** The vendored `parse_recipe` (`third_party/minecraftoss/player/src/crafting.rs:1007-1041`) knows only shaped and shapeless recipes. It drops `crafting_special_firework_star`, `_star_fade` and `_rocket`. (Kits and `/give` are fixed by E1's E-kit.)

### Vanilla 26.3

**`FireworkRocketEntity`**
- Size 0.25 x 0.25, client tracking range 4, update interval 10.
- **Launch:** velocity `(triangle(0, 0.002297), 0.05, triangle(0, 0.002297))`; lifetime `10 * (1 + flight_duration) + nextInt(6) + nextInt(7)`. Our `Rocket::launch` already matches this.
- **`tick`, for a rocket that is not attached:**
  1. Unless the rocket was shot at an angle: `velocity *= (h, 1, h)` with `h = 1.0` on `horizontalCollision`, else 1.15. Then add 0.04 to y.
  2. `hit = ProjectileUtil.getHitResultOnMoveVector(this, canHitEntity)`, taken **before** the move.
  3. `move(SELF, delta)` with block collision.
  4. `hitTargetOrDeflectSelf(hit)`.
  5. On `life == 0`: `entity.firework_rocket.launch`, AMBIENT, volume 3.0, pitch 1.0.
  6. `life++`; when `life > lifetime`, `explode`.
- **`getHitResultOnMoveVector`:** block clip from the position to the position plus delta (COLLIDER, no fluid); entity search box: the rocket's box expanded by delta and inflated by 1; each target's box inflated by the margin `max(0, min(0.3, (tickCount - 2) / 20))`; a target whose box holds the start point is hit at once; the nearest entity beats the block.
- **`Projectile.canHitEntity`:** the entity can be hit by projectiles, and it is not the owner (or riding with the owner) until the rocket has left the owner (`checkLeftOwner`).
- **What a hit does:** `onHitEntity` always calls `explode`. `onHitBlock` calls `explode` only when the rocket has stars. A starless rocket waits out its lifetime.
- **`explode`:** entity event 17 (the client burst), `GameEvent.EXPLODE` with the owner, `dealExplosionDamage`, `discard`.
- **`dealExplosionDamage`:** strength = `5 + 2 * stars`; with no stars it deals nothing at all. An attached entity (the elytra rider) takes the full strength. Every other `LivingEntity` with `distanceToSqr <= 25` (feet) takes `strength * sqrt((5 - distance) / 5)`, if a COLLIDER clip from the rocket to its feet, or to its feet plus half its height, is clear. The damage source is `fireworks(rocket, owner)`.
- **Knockback:** `minecraft:fireworks` is not in `#no_knockback`. On a full hit (outside the damage cooldown and not shield-blocked), `LivingEntity.hurtServer` calls `dealDefaultKnockback`: `knockback(0.4000000059604645, rocket.x - x, rocket.z - z)`, so the mob is pushed away from the burst. It is scaled by `1 - knockback_resistance`: iron golem 1, shulker 1, ravager 0.75, hoglin 0.6.
- **Damage type:** `data/minecraft/damage_type/fireworks.json`: exhaustion 0.1, `scaling: when_caused_by_living_non_player`. It is in `#is_explosion` and `#panic_causes`, so animals panic.
- **Death messages:** `death.attack.fireworks` "%1$s went off with a bang"; `.player` "%1$s went off with a bang while fighting %2$s"; `.item` "... due to a firework fired from %3$s by %2$s".

**`FireworkRocketItem`**
- **`useOn` (click a block), when not gliding:** a rocket at the click point plus the face normal times 0.15, owned by the player; the stack shrinks by 1.
- **While gliding:** `useOn` passes and `use` makes the attached rocket (our boost). It also drops leash connections with `entity.lead.break`.
- **In a dispenser** (`ProjectileItem`): the rocket is made with `shotAtAngle = true` at the block centre plus `facing * 0.5000099999997474`; `shoot(facing, power 0.5, uncertainty 1.0)`; level event 1004 replaces the dispense click: `entity.firework_rocket.shoot`, NEUTRAL, volume 1.0, pitch 1.2. A rocket shot at an angle gets no speed-up, no lift and no gravity.
- **Crossbow (for later):** rocket power 1.6 (arrows 3.15), spawned at the eye y minus 0.15000000596046448, 3 durability per rocket (`CrossbowItem`).

**The client burst**
- **No stars** (`ClientLevel.createFireworks`): `2 + nextInt(3)` `poof` particles with velocity `(gaussian * 0.05, 0.005, gaussian * 0.05)`, and no sound.
- **With stars:** a `FireworkParticles.Starter`. Lifetime `stars * 2 - 1`, plus 15 when any star twinkles. First tick sound: `large_blast` (or `large_blast_far`) when there are 3 or more stars or any `large_ball`, else `blast` (or `blast_far`); "far" means the camera is 16 or more blocks away; AMBIENT, volume 20, pitch `0.95 + rand * 0.1`. One star every second tick: `small_ball` `createParticleBall(0.25, 2)`, `large_ball` `(0.5, 4)`, `star` `createParticleShape(0.5, STAR, false)`, `creeper` `(0.5, CREEPER, true)`, `burst` `createParticleBurst`; then a `flash` particle in the star's first colour. At the end, with twinkle: `twinkle` (or `twinkle_far`), volume 20, pitch `0.9 + rand * 0.15`.
- `SparkParticle` handles the fade colour, trail and twinkle. Port them from javap.

**Components and villagers**
- `minecraft:fireworks {flight_duration (0-255, default 0), explosions (up to 256)}`.
- `minecraft:firework_explosion {shape: small_ball|large_ball|star|creeper|burst, colors, fade_colors, has_trail, has_twinkle}`.
- A celebrating villager's rocket (`CelebrateVillagersSurvivedRaid.getFirework`) is one BURST star of one dye colour, with no fade, trail or twinkle. Our port already does this (`villager_brain/behaviors.rs:791`). The DyeColor firework colours are `FIREWORK_COLORS` (`:751`), in DyeColor order; for example white 15790320, yellow 14602026, lime 4312372, blue 2437522, red 11743532, black 1973019.

**Crafting** (data-driven in 26.3)
- `recipe/firework_star.json`: dye `#minecraft:dyes` (the colour comes from the item's `minecraft:dye`), fuel `gunpowder`; shapes: `large_ball` fire_charge, `star` gold_nugget, `creeper` `#skulls`, `burst` feather; trail `diamond`, twinkle `glowstone_dust`.
- `recipe/firework_rocket.json`: shell `paper`, fuel `gunpowder` (1 to 3 pieces, flight = the count), any number of `firework_star`. The result is 3 rockets.
- `firework_star_fade`: one star plus dyes.
- The build agent javaps `FireworkStarRecipe`, `FireworkStarFadeRecipe` and `FireworkRocketRecipe` (`matches` and `assemble`) for the exact rules (for example, `ingredientCount() < 2` fails).
- **ShieldDecorationRecipe** (`recipe/shield_decoration.json`: banner `#minecraft:banners`, target `minecraft:shield`; moved here from E-banners): it matches when the grid holds exactly 2 items, one banner and one shield whose `banner_patterns` is empty. The result is a copy of the shield with its original components (`TransmuteRecipe.createWithOriginalComponents`), plus `banner_patterns` set to the banner's (possibly empty) and `base_color` set to the banner's colour.

### Plan

The rocket target list is E-thrown's `thrown::targets` / `hit_boxes` (no new list). If D0 has merged, use its generic hurt and knockback entries.

**Group `fw-burst` (M): rockets carry stars, burst on what they hit, and knock back. Three commits.**
1. **"Minecraft firework rockets carry their stars"**
   - `firework.rs`: `Explosion { shape, colors, fade_colors, trail, twinkle }` and `Fireworks { flight, explosions }`, read from the stack. `has_trail` and `has_twinkle` may be a bool or a 0/1 number.
   - `Rocket` gets `explosions` and `owner: RocketOwner { Mob(u64), Player(u64), Nobody }`.
   - `damage_to` uses `5 + 2 * stars` and returns `None` with no stars.
   - `Burst` carries `explosions` instead of `colors`. A villager rocket is one BURST star.
   - `entity_view.rs` (`:321`, the firework part of `bursts()`): a starless burst shows 2 to 4 poof puffs and no bang. Starred bursts keep today's drawing until `fw-shapes`.
   - `Burst` travels to friends in the snapshot; no `PROTOCOL` bump in the commit.
   - Tests: a starless rocket hurts no one; a 3-star rocket hurts 11 at the centre; the existing villager rocket test (`server.rs:12590`) still passes.
2. **"Minecraft firework rockets burst on mobs and players in their path"**
   - `Rocket::tick` takes targets: `thrown::targets(&mobs)` plus `thrown::player_targets(players, owner)`, with the villager owner flagged.
   - It follows vanilla's order: the hit test on the move vector before the move, the margin, a 0.25 box, and the owner-left check against the owner's box.
   - A starless rocket that meets a block doesn't burst. A horizontal collision stops the 1.15 speed-up. The burst happens at the position after the move.
   - Tests: a rocket launched at a zombie's feet bursts on the first tick at the zombie; a rocket launched at the owner's feet does not hit the owner; a starless rocket under a roof waits out its lifetime.
3. **"Mobs hurt by a firework burst are knocked back"**
   - Vendored `world.rs`: `pub fn firework_blast(&mut self, owner: FireworkOwner, id: u64, damage: f32, at: DVec3) -> Option<DamageResult>`. It runs `hurt_by_blast`'s per-kind hurt with damage type `minecraft:fireworks` (a typed variant with a zero push, so `hurt_by_blast` keeps its behaviour). When `applied`, it credits and angers the owner: for a player, `blasted_by_player` and `credit(.., "minecraft:player", true)`; for a mob, the `Blaster::Mob` attacker. On `full`, it calls `knock_back(id, 0.4000000059604645, at.x - x, at.z - z)` (`world/combat.rs:233`; the resistances are already there).
   - `server.rs:1438` calls it.
   - Tests: a grounded zombie gets about -0.4 horizontally, away from the burst, and +0.4 in y; an iron golem is hurt but not moved; a ravager gets 0.1; a cow panics; a second burst inside the damage cooldown doesn't push.

**Group `fw-launch` (M): players and dispensers launch rockets, and a player's rocket blames the player. Three commits.**
1. **"Minecraft firework rockets launch from a clicked block"**
   - `firework.rs` `rocket_place(player, world, held, gliding)` places the rocket at `player.target(world, reach)` `hit.point + face * 0.15`. It does nothing while gliding.
   - New `Change::LaunchRocket(RocketLaunch { at, fireworks })` in `bucket.rs:142`, next to `Change::Rocket`.
   - `actions.rs`: it joins the chain just before `rocket_use` (`:1131`). Its match arm calls `entities.launch_rocket(player_id, launch, sent)` (next to `throw_eye`, `entities.rs:999`). That sends `Command::LaunchRocket { owner, launch }` (`server.rs`, next to `ThrowEye` at `:3440`, `:3953`, `:4117`), and `ServerSim` pushes `Rocket::launch(RocketOwner::Player(owner), ..)`.
   - The rocket is used up, except in creative.
   - Test (`level.rs`, like `an_eye_thrown_from_the_hand_leaves_the_stack_and_flies`): clicking grass leaves 63 rockets, and a rocket view appears 0.15 above the click.
2. **"Minecraft dispensers shoot firework rockets"**
   - `dispenser.rs`: `firework_rocket` leaves `OTHER_BEHAVIOURS`. Its branch takes one rocket and pushes `DispensedRocket { at: centre + facing * 0.5000099999997474, facing, stack }` into a new `Level::dispensed_rockets` queue, kept beside `unsupported`. It plays `entity.firework_rocket.shoot` (volume 1, pitch 1.2) in place of the click.
   - `ServerSim` drains the queue after the level tick into `Rocket::shot(at, facing, 0.5, 1.0, fireworks)`: `getMovementToShoot` with the 0.0172275 triangle spread, `shot_at_angle = true`, and the owner `Nobody`. Such a rocket flies straight, with no lift or speed-up.
   - Test: an east-facing dispenser's rocket stays level and bursts on a zombie 6 blocks away. A plain rocket flies and puffs at the end of its life.
3. **"A player's firework rocket blames the player"**
   - `PlayerHitKind::Fireworks { at, by: Option<u64> }` (`world.rs:627`). `rocket_blast` sets `by` for a player owner and `source` only for a mob owner.
   - `entities.rs`: with `pvp` off, a player's rocket spares players, including the owner, as `ServerPlayer.canHarmPlayer` does: no hurt and no push. This follows `spared_by_blasts_without_pvp` (`:1851`).
   - `death.rs`: a new `DamageCause::PlayerFireworks { client }`, with type `minecraft:fireworks`, the line "went off with a bang" and kill-feed credit to the owner.
   - Tests: with pvp off, there is no hit; with pvp on, a friend killed by the host's rocket credits the host.

**Group `fw-shapes` (M): bursts look and sound like vanilla. Two commits** in `firework_particles.rs` and `entity_view.rs` (`:321`, `:805`).
1. **"Minecraft firework stars burst in their own shapes"**: port `FireworkParticles.Starter` (javap `net.minecraft.client.particle.FireworkParticles$Starter`) as an emitter per burst: star i bursts on tick 2i; small and large balls, the STAR and CREEPER coordinate shapes, and burst; a flash in the first colour for each star; `blast` or `large_blast` (and their `_far` forms), and the twinkle sound at the end. `entity_view.rs` keeps the emitters and plays their sounds. Tests: the Starter's lifetime; the sound choice for 1 small ball, a large ball, and 3 stars; the spark count per shape, taken from the loops; a twinkle adds 15 ticks.
2. **"Minecraft firework sparks fade, trail and twinkle"**: port `SparkParticle`'s fade colour, trail (sparks left behind) and twinkle (flicker) from javap. Tests: the fade colour reached at the end of life, and the trail spawning.

**Group `fw-craft` (S): stars, rockets and patterned shields from the crafting table. Two commits** in the vendored `player/src/crafting.rs`.
1. **"Minecraft firework stars and rockets are crafted like vanilla"**
   - `Kind` (`:35`) gets `FireworkStar`, `FireworkStarFade` and `FireworkRocket`, read from the three special types with their JSON fields (`parse_recipe`, `:1007`). `matching` (`:788`) builds the result from the grid: the `firework_explosion` component on the star, or `fireworks {flight = gunpowder count 1-3, explosions in grid order}` with a count of 3.
   - A dye's colour comes from its `<color>_dye` id and the DyeColor firework colour (`FIREWORK_COLORS` order).
   - Tests: gunpowder, red dye, fire charge and diamond make a red `large_ball` star with a trail; paper, 2 gunpowder and 2 stars make 3 rockets with flight 2 and both stars in grid order; paper with 4 gunpowder makes nothing; a star plus blue dye gets the fade.
2. **"Minecraft shields take a banner's pattern in the crafting grid"**
   - `Kind::ShieldDecoration`, read from `minecraft:crafting_special_shielddecoration` (`recipe/shield_decoration.json`), with the rules in the vanilla facts above.
   - Tests: the match rules (exactly 2 items; a patterned shield is refused); the result keeps the shield's name, damage and enchantments; a black plain banner gives `base_color` black and empty patterns.

**Group `fw-elytra` (S), one commit: "An elytra rocket goes off at the end of its boost"**
- Alongside `Change::Rocket` (`actions.rs:1173`), the host launches an attached `Rocket`: its owner and rider is the player, and its lifetime is the boost's (`boost_ticks - 1`); it follows the player's feet (plus the hand offset while gliding); it never hits its rider.
- At the end: the rider takes `5 + 2 * stars` (`PlayerHitKind::Fireworks` with `by` = the player, so the pvp rule applies as vanilla); others within 5 blocks are hurt and knocked back as usual; a starless rocket only puffs. Others see its trail through `rocket_views`.
- Friends: a friend's rocket use runs through the same `actions::act` on the host (`minecraft_players.rs:1923`), and the rider's position comes from the `players: &[PlayerBody]` that `tick_rockets` already gets, so the same code serves the friend. The `PlayerHit` reaches the friend through the existing `players_struck`/`rocket_hits` route.
- Tests: a gliding player with a 1-star rocket gets a `PlayerHit` of 7.0 at the end of the boost; with a starless rocket, nothing; the same for a second (friend) player id, whose rocket follows that player's body and not the host's.

### Left for later (in the log)

- **Crossbow-fired rockets.** Players have no crossbow or bow yet. The shot-at-angle rocket from `fw-launch` is the path a player crossbow will use (power 1.6, eye minus 0.15, 3 durability).
- **Saving rockets in flight.** A rocket lives at most 52 ticks at flight 3 (vanilla saves `Life`, `LifeTime`, `FireworksItem` and `ShotAtAngle`). It is dropped on save, as in round 9.
- **The angled rocket model.** Rockets stay billboards. Vanilla turns a rocket shot at an angle along its flight.
- **Early bursts and the boost.** An elytra rocket that bursts early on a mob still boosts for its full time.
- **Tooltips.** The firework tooltip lines ("Flight Duration", the stars) are not drawn.
- **Game events.** `GameEvent.EXPLODE` from a burst belongs to track W, at the `Exploded` arm of `tick_rockets` (`server.rs:1392-1420`).

### The owner's steps

Setup: the kit below (the starred rockets need E1's E-kit merged). `/gamemode survival`, `/time set day`, open flat ground, `/gamerule pvp true` (the default). The kit fills the backpack, so first move the four rocket stacks into the hotbar: 1 plain rocket, 2 "red" (one large ball with a trail), 3 "two-star" (a yellow star fading to blue and a lime twinkling creeper, flight 2), 4 "three-star" (burst, small ball and large ball, flight 0).

Host:
- **FW-1. A plain rocket.** Right-click the grass 3 blocks ahead with the plain rocket. A rocket rises with a whoosh and a white trail, then pops in 2 to 4 grey puffs with no bang. The stack shows 63. **Screenshot FW-1.**
- **FW-2. The red rocket.** Place it the same way. It rises and bursts into one big red ball whose falling sparks leave trails, with the deep "large blast" bang. **Screenshot FW-2.**
- **FW-3. The two-star rocket.** It climbs higher. First a yellow star shape bursts and fades to blue, then a lime creeper face that crackles a moment later. **Screenshot FW-3.**
- **FW-4. The three-star rocket.** It stays low. Three bursts come one after another with the large bang.
- **FW-5. A rocket at a zombie's feet.** Hatch a zombie 10 blocks away. When it is about 4 blocks away (block reach is 4.5, so you cannot click the ground further out), click the ground right under it with the red rocket. The rocket bursts at once on the zombie: it flashes red and hops back about a block, away from the burst, then comes at you again. You lose about 1.5 hearts too, because you stand within 5 blocks of the burst (7 x sqrt((5 - 4) / 5) = 3.1). **Screenshot FW-5.**
- **FW-6. Your own rocket hurts you.** Build a cobblestone roof 3 blocks above you and stand under it. Place the red rocket next to you. It bursts on the roof: you lose about 2 to 3 hearts and are pushed away from it. **Screenshot FW-6.**
- **FW-7. Animals.** Hatch 3 cows close together and click the ground under one of them with the red rocket (7 damage; the three-star rocket's 11 would kill a 10-health cow). All three are hurt (the one it bursts on the most, about 3.5 hearts), pushed away from the burst, and run in panic.
- **FW-8. An iron golem.** Hatch an iron golem and place the red rocket at its feet. It flashes red but does not move, then turns on you, because a player's rocket blames the player. You lose a little health too (you are within 5 blocks). Run or kill it. **Screenshot FW-8.**
- **FW-9. A dispenser.** A placed dispenser faces the player who places it. Stand at one end of a flat 10-block lane with your back to the lane, look straight ahead (level) and place the dispenser on the ground in front of you: its hole faces you and the lane behind you. Put a lever on its side. Put 4 red rockets in it. Hatch a zombie 7 blocks down the lane, and stand beside the dispenser, not in front of it. Flip the lever on. A rocket shoots out level, with no arc, and a sharp shoot sound. It bursts on the zombie, which hops back. **Screenshot FW-9.** With plain rockets in the dispenser, flip the lever off and on: the rocket flies level 10 to 15 blocks and puffs.
- **FW-10. Crafting.** At the crafting table: gunpowder, red dye, fire charge and diamond make a firework star. **Screenshot FW-10.** Then paper, 2 gunpowder and that star make 3 rockets. Place one: a red large ball with a trail, higher than the red kit rocket. (The shield recipe step is BC-6 in E-banners, with the banner kit.)
- **FW-11. The elytra rocket.** Put on the elytra (chest slot). `/tp ~ ~60 ~`, jump and glide (jump again in the air), then use the red rocket. You boost, and after 1 to 1.5 s the rocket bursts at you: a bang, red sparks, and about 3.5 hearts lost. **Screenshot FW-11.** With a plain rocket you get the boost, a puff and no damage.

Friend (two windows, `scripts\mp-local.ps1`; `/kit @a` from the host):
- **FW-12. The friend's rockets.** The friend places a red rocket at a zombie's feet. Both windows show the rocket rising, the same red ball, and the zombie hopping back. **Screenshot FW-12a (host), FW-12b (friend).**
- **FW-13. Rockets and pvp.** `/gamerule pvp false`. The host places the three-star rocket under a roof next to the friend. The friend takes no damage and is not pushed, while a zombie next to the friend is hurt and pushed. `/gamerule pvp true` and repeat. The friend loses hearts and is pushed away. A kill reads "Friend went off with a bang", and the kill feed names the host. **Screenshot FW-13.**
- **FW-14. The friend's dispenser.** The friend flips the FW-9 lever. The host sees the level rocket hit the zombie.
- **FW-15. The friend's elytra rocket.** The friend puts on the elytra, glides from `/tp ~ ~60 ~` and uses the red rocket. The friend boosts, and at the end of the boost the burst goes off at the friend: the friend loses about 3.5 hearts, and the host sees the red burst at the friend. **Screenshot FW-15**, both windows.

```
IW4L_MINECRAFT_KIT=minecraft:firework_rocket*64,minecraft:firework_rocket[fireworks={flight_duration:1,explosions:[{shape:large_ball,colors:[I;11743532],has_trail:1b}]}]*64,minecraft:firework_rocket[fireworks={flight_duration:2,explosions:[{shape:star,colors:[I;14602026],fade_colors:[I;2437522]},{shape:creeper,colors:[I;4312372],has_twinkle:1b}]}]*64,minecraft:firework_rocket[fireworks={flight_duration:0,explosions:[{shape:burst,colors:[I;15790320,11743532]},{shape:small_ball,colors:[I;2437522]},{shape:large_ball,colors:[I;14602026]}]}]*64,minecraft:dispenser*4,minecraft:lever*4,minecraft:zombie_spawn_egg*16,minecraft:cow_spawn_egg*8,minecraft:iron_golem_spawn_egg*2,minecraft:elytra,minecraft:gunpowder*64,minecraft:paper*32,minecraft:fire_charge*8,minecraft:gold_nugget*16,minecraft:feather*16,minecraft:creeper_head,minecraft:diamond*8,minecraft:glowstone_dust*16,minecraft:red_dye*16,minecraft:blue_dye*16,minecraft:lime_dye*16,minecraft:crafting_table,minecraft:cobblestone*64,minecraft:oak_planks*64,minecraft:totem_of_undying,minecraft:golden_apple*16,minecraft:cooked_beef*32
```

27 stacks.

### Files shared

- Vendored `world.rs` (`hurt_by_blast`, `player_blast`, `Blaster`, `PlayerHitKind`) and `world/combat.rs` (`knock_back`): D0 rewrites the per-kind arms, M1-M3 and W add mobs. Use D0's generic hurt and knockback entry once it has merged.
- `thrown.rs` (`targets`, `player_targets`): E-thrown (same track) and D0.
- `server.rs` (rockets, `Command`): every track.
- `actions.rs` (the item-use chain), `bucket.rs` (`Change`) and `entities.rs` (pvp filters): W (vibrations from item use).
- Vendored `world/src/level/dispenser.rs`: D (trial chamber dispensers) and W (dispense game events).
- Vendored `player/src/crafting.rs`: E2 only (firework recipes and the shield recipe).
- `entity_view.rs` `bursts()`: E1 adds one call line for mob puffs at the top; E2 owns the firework loop.
- `death.rs`: M tracks (new death causes).

---

## E-bell-hits. Projectiles ring bells

The second half of the bell limit. It needs no E1 code: a ring already plays its sound, wakes the villagers and makes raiders glow. Once E1's `bells-swing` is merged, the ring also swings the bell.

### The owner's finding

> **Bells** don't swing, and arrows and wind charges don't ring them. (`docs/round9/release-2-tests.md`, Known limits)

### Cause

Each projectile hits blocks in its own code, and none of them asks the struck block to react:
- the player's and mobs' thrown items (`server.rs:2121-2185`, `Landed { block, face, at }` at `thrown.rs:316-323`);
- tridents (`trident.rs:552-569`);
- mob arrows (`entities/src/projectile.rs:218-230`, ticked at `entities/src/world.rs:6693-6745`);
- fireballs and wither skulls (`entities/src/world/fireballs.rs:259-273`, `FireballHit { location, block }` at `fireball.rs:84-87`);
- shulker bullets (`shulker_bullet.rs:52-55`), llama spit (`world/equines.rs:532`), witch potions (`world.rs:6650`, `tick_potion`);
- dragon fireballs (`server.rs:2679`) and firework rockets (`firework_rocket.rs:87`, `RocketEvent::Exploded`).

The only ring sources are a hand use (`actions.rs:785-800`), redstone (`redstone.rs:762-771`), a villager (`server.rs:1454`) and a wind burst (`redstone.rs:954-956`).

### Vanilla 26.3

- **`BellBlock.onProjectileHit(level, state, hit, projectile)`:** `onHit(level, state, hit, owner instanceof Player ? owner : null, true)`. `onHit` checks `isProperHit(state, hit.getDirection(), hit.getLocation().y - pos.y)`, which is the fork's `bells::proper_hit` (`bells.rs:40-52`): not up or down, at most 0.8124 above the block, and a side that fits the attachment. Then `attemptToRing(player, level, pos, hit.getDirection())`: `BellBlockEntity.onHit(dir)` (block event 1), `block.bell.use` (BLOCKS, volume 2, pitch 1) and `GameEvent.BLOCK_CHANGE`. The `bell_ring` stat goes to a player owner.
- **Every projectile reaches it.** `Projectile.onHitBlock` calls `state.onProjectileHit(level, state, hit, this)`, and every override calls `super.onHitBlock`: `AbstractArrow` (arrows and tridents), `ThrowableItemProjectile` and `AbstractThrownPotion` (snowballs, eggs, pearls, bottles, potions), `Fireball`/`SmallFireball`, `AbstractWindCharge`, `ShulkerBullet`, `LlamaSpit`, `FireworkRocketEntity` and `FishingHook`. `DragonFireball` and `WitherSkull` inherit it. Hit tests use `ClipContext.Block.COLLIDER`, and the bell's collision shape is its `getVoxelShape`.
- **A wind charge rings twice.** `AbstractWindCharge.onHitBlock` runs `super.onHitBlock` first, the proper-side ring with the struck face. It then explodes at the hit plus 0.25 × the face normal, and `BellBlock.onExplosionHit` with `canTriggerBlocks()` calls `attemptToRing(level, pos, null)`, which uses the bell's facing. Both block events run at the end of the tick, and the facing one is last.

### Plan: group `bell-hits` (M), two commits (after `fw-burst`, so `tick_rockets` is already rewritten)

1. **"Snowballs, tridents and wind charges ring Minecraft bells"**
   - `bells.rs`: `rings_from(block, face, hit_y) -> bool` (`is_bell && proper_hit`).
   - `server.rs`: one seam, `ServerSim::projectile_hit_block(pos, face, at)`, which is vanilla's `BlockState.onProjectileHit`. On a proper hit it calls `self.ring_bell(pos, Some(face_name(face)))`. Later block reactions (target blocks, chorus flowers, pots) plug in here too. Track D's breeze charge calls the same seam.
   - Call it from: `tick_thrown` (`server.rs:2137`, from `landed.block` and `landed.face`: snowballs, eggs, pearls, bottles, potions, snow golem and witch throws, and a wind charge before its burst at `:2165-2168`, so the facing ring comes last as in vanilla); `tick_tridents` (a new `TridentEvent::Block { pos, at, velocity }` at `trident.rs:552`, with the face from `thrown::struck_face`); `tick_rockets` (a rocket exploding on a block); `tick_dragon_fireballs`.
   - Tests (check the ring through `level.bell_rings` / the `Bells` state and the use sound; once E1's `Output.block_events` is on the branch, also check the block event direction):
     - `server::tests::a_snowball_on_a_bells_side_rings_it_and_one_on_its_top_does_not` (the use sound, the direction, and the villager hearing it);
     - `a_thrown_trident_rings_a_hanging_bell`;
     - `a_wind_charge_on_a_bells_side_rings_it_twice_and_ends_on_its_facing`;
     - `a_wind_burst_beside_a_bell_rings_it` (the existing burst ring has no test yet).
2. **"Skeleton arrows and fireballs ring Minecraft bells"** (after D0, or keep it to one push line per site)
   - Vendored `projectile.rs` keeps the arrow's block hit `(block, hit point, velocity)` with a `take_block_hit` like `take_hit_sound`.
   - Vendored `world.rs` collects `projectile_block_hits` from arrows (`:6693-6745`), fireballs and wither skulls including a large fireball before it explodes (`world/fireballs.rs:259-273`), shulker bullets (`world/shulker_bullets.rs`), llama spit (`world/equines.rs:532`, one push line; E1 owns the rest of the file) and witch potions (`tick_potion`).
   - `EntityWorld::take_projectile_block_hits()`, drained in `server.rs` after `tick_mobs` (`:4248`) into the seam, with the face from `thrown::struck_face`.
   - Tests: `server::tests::a_skeletons_arrow_into_a_bell_rings_it`; `a_blazes_fireball_into_a_bell_rings_it`; an entities test that an arrow's block hit is reported once.

### Decisions (for the log)

- A projectile's ring is heard one tick later than in vanilla. Projectiles tick after `ServerSim::tick` runs the block events (`server.rs:4222` against `:4248-4253`).
- There is no `bell_ring` statistic, because the fork has no statistics.
- MW2 bullets don't ring bells, because vanilla has no bullets. The player has no bow in the fork, so the player's projectile rings come from snowballs, eggs, potions, pearls, tridents and wind charges.

### The owner's steps (survival, Normal, by day, flat ground)

- **BH-1. Throws.** Place a bell on cobblestone. Place it facing you as in B-1. From 5 blocks away, in front of it, throw a snowball at the side facing you, below the rim: it rings (and, with E1's swing merged, swings). Throw one onto its top: no ring. Throw an egg, a splash water potion and the trident at the same side: each rings. A throw at one of the two other sides does not ring it. **Screenshot bell_03_snowball.**
- **BH-2. Wind charge.** Throw a wind charge at the bell's side: it rings (a doubled ring) and ends swinging along the bell's facing. Throw one at the ground right next to it (1 block away; the burst only reaches about 1.2 blocks): it rings once along its facing. **Screenshot bell_04_wind_charge.**
- **BH-3. Mobs' shots.** Put a bell between you and a hatched skeleton (in the dark or under a roof): an arrow that hits the bell rings it. Do the same with a blaze: its fireball rings it. **Screenshot bell_05_skeleton_arrow.**
- **BH-4. Friend window.** The friend throws a snowball at the bell: it rings in both windows.

```
IW4L_MINECRAFT_KIT=minecraft:bell*4,minecraft:snowball*16,minecraft:egg*16,minecraft:splash_potion@water*4,minecraft:trident+loyalty=3,minecraft:wind_charge*32,minecraft:cobblestone*64,minecraft:oak_planks*64,minecraft:skeleton_spawn_egg*2,minecraft:blaze_spawn_egg*2,minecraft:shield,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*8,minecraft:cooked_beef*16,minecraft:ender_pearl*8
```

21 backpack slots (splash potions do not stack, so `splash_potion@water*4` takes four).

### Files shared

- `server.rs` `tick_thrown`, `tick_tridents`, `tick_rockets`, `tick_dragon_fireballs`: E-thrown and E-fireworks (same track), W (projectile game events).
- Vendored `world.rs` (arrow tick), `projectile.rs`, `world/fireballs.rs`, `world/shulker_bullets.rs`, `world/equines.rs`: D0's projectile-hit arms touch the same sites.
- `trident.rs`: D0.
- Vendored `world/src/level/redstone.rs` (`ring_bell`): W adds `BLOCK_CHANGE` there.

---

# Combined kits

One kit per track, for a quick look at every section in one world. Each section above has its full kit.

**E1** (27 backpack slots; needs E-kit for the banner, shield and crossbow entries and `dye-icons` for the dyed armour):

```
IW4L_MINECRAFT_KIT=minecraft:horse_spawn_egg*4,minecraft:wolf_spawn_egg*2,minecraft:ocelot_spawn_egg*2,minecraft:llama_spawn_egg*2,minecraft:villager_spawn_egg*4,minecraft:cow_spawn_egg*2,minecraft:wheat*64,minecraft:bone*32,minecraft:cod*32,minecraft:golden_carrot*16,minecraft:saddle,minecraft:leather_horse_armor@red,minecraft:leather_horse_armor@lime,minecraft:composter,minecraft:emerald*64,minecraft:ominous_bottle*2,minecraft:ominous_bottle[ominous_bottle_amplifier=4],minecraft:white_banner[banner_patterns=[{pattern:"minecraft:creeper",color:"black"},{pattern:"minecraft:border",color:"red"}]]*4,minecraft:shield[base_color=red,banner_patterns=[{pattern:"minecraft:cross",color:"yellow"}]],minecraft:crossbow[charged_projectiles=[{id:"minecraft:arrow"}]],minecraft:pillager_spawn_egg*8,minecraft:bell*2,minecraft:lever,minecraft:potion@long_night_vision,minecraft:hay_block*16,minecraft:iron_sword,minecraft:cooked_beef*32
```

**E2** (27 backpack slots; the starred rocket needs E1's E-kit merged):

```
IW4L_MINECRAFT_KIT=minecraft:snowball*16,minecraft:egg*16,minecraft:trident+loyalty=3,minecraft:splash_potion@poison,minecraft:splash_potion@water,minecraft:wind_charge*16,minecraft:pillager_spawn_egg*2,minecraft:blaze_spawn_egg*2,minecraft:hoglin_spawn_egg*2,minecraft:zombie_spawn_egg*8,minecraft:oak_door*8,minecraft:oak_planks*64,minecraft:lead*8,minecraft:cow_spawn_egg*4,minecraft:oak_fence*16,minecraft:shears,minecraft:cat_spawn_egg*2,minecraft:cod*32,minecraft:chest*2,minecraft:firework_rocket[fireworks={flight_duration:1,explosions:[{shape:large_ball,colors:[I;11743532],has_trail:1b}]}]*64,minecraft:firework_rocket*64,minecraft:dispenser*2,minecraft:bell*2,minecraft:potion@long_water_breathing,minecraft:oak_boat,minecraft:iron_shovel,minecraft:cooked_beef*32
```

Every id in both lines is in the 26.3 `assets/minecraft/items` list. Counts are backpack slots, not entries: `give_kit` (`crafting.rs:425-446`) splits a count above the item's stack size over several slots, so unstackable items (saddles, beds, potions, splash potions, shields) take one slot each. The kits in this brief were recounted that way.

---

# Commit groups

Each group continues from what is already committed. The build agent reads the section named in the table. Sizes: S about one commit, M two to four, L three or more with new files.

## E1, in order

| # | Key | Size | Section | Commits |
|---|---|---|---|---|
| 1 | `kit-components` | S | E-kit | Minecraft item commands and kits take banner patterns, shield colours, loaded crossbows and firework stars |
| 2 | `particles-events` | M | E-particles | Minecraft taming shows hearts, and a failed try or a bucking horse shows smoke; Minecraft villagers show hearts, anger, happiness and raid sweat |
| 3 | `trading-keys` | L | E-trading | number keys and the offhand key; Q; drags; no double-click gather |
| 4 | `effects-list` | M | E-effects | effect list frames and names; the list beside the inventory |
| 5 | `omen-sound-tooltip` | S | E-omen | omens by command play their sound; ominous bottles show their Bad Omen line |
| 6 | `horse-fall` | M | E-horses 1 | horses fall like vanilla horses; riders share their horse's fall |
| 7 | `vehicle-hearts` | M | E-horses 3 | riders see their mount's hearts; friends see their mount's hearts |
| 8 | `horse-gallop` | S | E-horses 2 | ridden horses gallop and breathe |
| 9 | `dye-icons` | S | E-horses 4 | dyed items show their colour; kits dye leather with the 26.3 tag |
| 10 | `horse-dye` | M | E-horses 4 | horses keep the dye; dyed leather horse armour shows its colour |
| 11 | `item-states` | M | E-banners | item models pick crossbow and bow states; pillagers and piglins show the pull and loaded arrow |
| 12 | `banner-items` | M | E-banners | banners and shields show their patterns in hand, on heads, on the ground and as icons |
| 13 | `banner-roundtrip` | M | E-banners | banners keep their patterns and name when placed; banners drop with their patterns and name |
| 14 | `player-looks` | S | E-banners | players and friends show loaded crossbows and patterned banners |
| 15 | `placed-banners` | M | E-banners | placed banners show their patterns |
| 16 | `bells-swing` | M | E-bells | bells swing when rung |
| 17 | `particles-spit-love` | S | E-particles | llama spit leaves a puff; farm animals show hearts when fed for love (after D0) |
| 18 | `bells-outline` | S, optional | E-bells | glowing mobs show a white outline |

## E2, in order

| # | Key | Size | Section | Commits |
|---|---|---|---|---|
| 1 | `ti1-targets-and-hurt` | M | E-thrown | thrown things hurt every mob family; hit pillagers, ravagers, withers and the Nether mobs; snowballs hurt blazes; wind charges don't anger calm mobs |
| 2 | `ti2-potions` | M | E-thrown | potions on ghasts and blazes; harming heals every undead mob; water bottles hurt blazes, striders and snow golems |
| 3 | `ti3-mounts` | S | E-thrown | thrown things pass the thrower's mount |
| 4 | `doors-zombies` | M | E-doors | zombies break wooden doors on Hard; door cracks show for host and friends |
| 5 | `doors-raiders` | M | E-doors | vindicators open and break doors in raids |
| 6 | `maps-loot` | L | E-maps | map files; chests roll explorer maps; coastline preview; skip claimed structures |
| 7 | `lead-knots` | M | E-leads-cats | knots can be punched off; shears cut every lead off a knot |
| 8 | `cat-chests` | S | E-leads-cats | sitting cats keep chests shut; cats leave open chests |
| 9 | `fw-burst` | M | E-fireworks | rockets carry their stars; burst on mobs and players; burst knockback |
| 10 | `fw-launch` | M | E-fireworks | launch from a clicked block; dispensers shoot rockets; a player's rocket blames the player |
| 11 | `fw-shapes` | M | E-fireworks | stars burst in their own shapes; sparks fade, trail and twinkle |
| 12 | `fw-craft` | S | E-fireworks | stars and rockets crafted like vanilla; shields take a banner's pattern |
| 13 | `fw-elytra` | S | E-fireworks | an elytra rocket goes off at the end of its boost |
| 14 | `bell-hits` | M | E-bell-hits | snowballs, tridents and wind charges ring bells; skeleton arrows and fireballs ring bells |
| 15 | `maps-held` | M | E-maps | maps mark holder and target; friends get map pictures |
| 16 | `maps-view` | M | E-maps | the held map in first person |
| 17 | `maps-colours` | M | E-maps | block map colours from the jar |
| 18 | `maps-explore` | M | E-maps | maps fill in as you walk; empty maps |
| 19 | `ti4-end-targets` | M, optional | E-thrown | snowballs and tridents hit end crystals and the ender dragon |
