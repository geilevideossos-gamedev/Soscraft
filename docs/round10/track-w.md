# Track W: darkness, vibrations, sculk and the warden

The owner's words for round 10 (`docs/PROGRESS.md`, round 10 brief):

> D. Finish track D: the rest of d5 (trial chambers and trial spawners, lodestones), d3 (deep dark, warden, sculk), and d4 (the remaining mobs) [...]. Land the shared changes (mob list, spawn tables, render registry) first so the later merges stay small.

Track W is round 9's d3 (brief `docs/round9/track-d.md` D5) without its keyframe group, which landed in round 9 (81f6f353). The round 9 release notes still list it as a known limit: "Sculk shriekers, sensors and catalysts do nothing yet" (`docs/round9/release-2-tests.md:412`), and the darkness effect is only carried, saved and shown as an icon (round 9 track D log, group effects-omens, commit 1 decisions).

Track W has seven parts, one per commit group. The sections keep their numbers; the groups run in this order:

1. **W1, group `darkness`:** the darkness effect closes the fog in and pulses the light.
2. **W2, group `vibrations`:** the game event core and sculk sensors, with the first sources.
3. **W3, group `vibrations-more`:** every other source, amethyst resonance, and MW2 guns.
4. **W7, group `shriekers-catalysts`:** shriekers warn players and darken them; catalysts bloom and spread sculk. Until W4, a fourth warning plays vanilla's fallback (the angry warden reply sound) because no warden can be summoned yet.
5. **W4, group `warden-live`:** the warden in the entity world, saved, hatched, and summoned by shriekers.
6. **W5, group `warden-fight`:** roar, sniff, melee, sonic boom and darkness pulses.
7. **W6, group `warden-render`:** the model, its glowing layers, keyframes, heartbeat and particles.

**Segments.** W1, W2, W3 and W7 add no mob family and start from `minecraft-survival` now (segment `w1`). W4 to W6 add the warden family and must use track D0's shared mob registry, so they go last, as segment `w2`, after D0 is merged into `minecraft-survival` and `minecraft-survival` is merged into `track-w`. W7 comes before them: its only warden needs are the summon (W4 plugs `ServerSim::summon_warden` into the shrieker's response, one line) and the "warden nearby" check (a kind test on the level's entities that finds nothing until W4).

Paths and line numbers are as of 6e0e0573 (the track-d merge). Re-check them before editing: tracks D0, D, M1-M3, E1, E2 and F run at the same time. Vanilla facts were checked with javap on the 26.3 `client.jar` and against the 26.3 data in that jar this round. Every "not verified" item of round 9's D5 is resolved below (marked **resolved**). No Mojang code or data is copied: numbers and behaviour only, tags and loot read at runtime.

**Multiplayer rule for every item.** The host owns every level, mob, listener and rule. New state is per player and keyed by UUID (saved in `players/data/<uuid>.dat`) or per level (saved in that dimension's folder, in vanilla's block entity and entity formats). Nothing keys on player 0. Friends run no AI: they mirror mobs from look tags and per-tick `MobShow`, so every drawn warden state goes through `MobShow`, and every particle a friend must see goes through the entity snapshot bursts (`crates/minecraft_survival/src/mob_sender.rs:220-255`) or a level event.

---

## W0. Shared points for the track

### Coordination with the other round 10 tracks
Game events come from many places that other tracks also edit this round:

| Track | Edits near W's call sites |
|---|---|
| E1 | bells (swing), banners, crossbow, trading keys, horse items |
| E2 | doors (vindicators and zombies break and open them), fireworks bursting on mobs, thrown items hitting mobs (`thrown.rs` targets), leads, cat chests |
| D | trial spawners, vaults, breezes, lodestones |
| D0, M1-M3 | the mob registry and new mob files (`world.rs` walks, `server_mobs.rs`, `mob_tracking.rs`) |

Rules:
1. **One line per call site.** W adds one helper and calls it: `Level::game_event(&mut self, event: &'static str, pos, affected: Option<BlockStateId>)` in the vendored world crate (source: `self.acting_player`), `EntityWorld::game_event(...)` in the vendored entities crate, and `Entities::game_event(player, event, pos)` in `minecraft_survival`. A call site gains exactly one line, placed after the state change it reports. No call site is restructured.
2. **Event ids are strings** (`"minecraft:block_open"`), like sound events. The vendored world and entities crates cannot see each other or `minecraft_terrain` (`entities` depends only on `-player`, `world` only on `-core` and `-generator`; checked in their `Cargo.toml`), so the frequency and radius table lives in `minecraft_terrain`.
3. **Sites another track creates this round.** Trial spawner spawns post `entity_place` (`TrialSpawner.spawnMob`, javap) and firework rockets post `explode` (`FireworkRocketEntity.explode`, javap). Whichever of W and D (trial spawners), or W and E2 (fireworks), merges second adds that one line, and records it in its log.
4. **Doors broken by mobs (E2).** Vanilla's door breaking posts `block_destroy` through `Level.destroyBlock`, and a mob opening a door posts `block_open`/`block_close` through `DoorBlock.setOpen` (javap). If E2 lands first, W's `vibrations-more` calls the helper from E2's sites. If W lands first, E2 calls it.

### Call sites (current code)
Player side (the host runs every player's actions, friends included):
- Steps, swimming, landing: `crates/render_anim/src/minecraft_players.rs:292` `movement`, steps at `:364-397`, landing at `:412-424`; MW2 crouch and prone flags at `:420-423` stand for vanilla's `isSteppingCarefully` (`isShiftKeyDown`, javap). `PlayerView.sneaking` exists (`crates/minecraft_survival/src/entities.rs:140`).
- Block place and break: `crates/minecraft_survival/src/level.rs:172-186` `write_blocks` (every player write), `entities.rs:974` `placed`, `:1361` `mined`, `crates/minecraft_survival/src/actions.rs:1649`.
- Block use: `actions.rs:817` → `entities.rs:982` → `crates/minecraft_terrain/src/server.rs:2422` `use_block` → vendored `third_party/minecraftoss/world/src/level/redstone.rs:990-1046` `use_block_facing` (doors `:994`, trapdoors `:999`, gates `:1007`, note blocks `:1021`, levers `:1027`, buttons `:1028`).
- **Commands without a player.** `Command` (`server.rs:3397-3462`): `PlayerEdit` (`:3401`), `PlayerBreaks` (`:3402`), `PlayerMines` (`:3403`), `UseBlock` (`:3404`), `RingBell` (`:3405`), `BoneMeal` (`:3406`), `Attack` (`:3407`) and `ChestOpeners` (`:3458`) carry no player id today; `ItemOnBlock` (`:3408`) does. Their senders are the handle methods at `server.rs:3741-3791` (for example `player_edit` `:3741`, `player_mines` `:3752`, `use_block` `:3764`) and `chest_openers` (`:3972`), and their callers in `crates/minecraft_survival/src/entities.rs` (`placed` `:974`, `use_block` `:982`, `ring_bell` `:990`, `chest_openers` `:1327`, `mined` `:1361`). Every one of them gains `player: u64` the way `ItemOnBlock` has it, because the source decides sneaking, spectators, the shrieker's `tryGetPlayer` and the warden's anger. `ChestOpeners` sends a recomputed viewer count (callers `crates/minecraft_survival/src/level.rs:411`, `menus.rs:1850`, `crates/render_anim/src/minecraft_world.rs:2462`), so it gains `player: Option<u64>`: the player whose opening or closing changed the count (vanilla's `ContainerOpenersCounter` posts `container_open` and `container_close` with that player as source), `None` when a count changes without one.
- Bells: `actions.rs:796` → `server.rs:1376` → `redstone.rs:776` `ring_bell`.
- Items on blocks (buckets, flint and steel, TNT, bone meal): `server.rs:2485-2503` `item_on_block_by` (has the player) → `dispenser.rs:197` `empty_bucket_into`, `:249` `bucket_pickup`, `explosion.rs:80` `ignite_tnt_by`, `fire.rs:123` `place_fire`.
- Containers: `server.rs:1744` `set_chest_openers` → `redstone.rs:305`.
- Eating and drinking: `actions.rs:448` (`Bite::Done`).
- Death experience: `crates/render_anim/src/minecraft_players.rs:1244` (`entities.award_experience`).
- MW2 guns: `crates/render_anim/src/minecraft_world.rs:928` takes the voxel events, `:968-1012` sorts bullets, mob shots and blasts; friends' levels at `crates/render_anim/src/minecraft_levels.rs:266-326`. `sim::voxel::VoxelEvent` (`crates/sim/src/voxel.rs:164-177`) has `Ray { from, to, attacker }`, `Shot { block, attacker }`, `MobShot { key, from, attacker }` and `Explosion { center, power }`. Note `:973-980` drops a non-builder's shots before mining: take the vibrations from `all_events` first. A `Ray` exists only for a bullet whose trace met something (`crates/sim/src/combat.rs:1239-1241` skips a shot with no segments, `:1264` pushes the ray); when it met nothing solid, `to` is the trace's end in the air (`:1245-1249`).

Level and mob side:
- Explosions: vendored `world/src/level/explosion.rs:151` `explode_from` (every level explosion, MW2 blasts through `server.rs:1791` `weapon_blast` too).
- Pistons: `world/src/level/piston.rs:392` `piston_trigger_event`.
- Lightning: `world/src/level/entity.rs:609` `summon_lightning`.
- Items and falling blocks stepping on a sensor or shrieker: `world/src/level/entity.rs:1234-1238` `step_on` marks them "unsupported" today.
- Mob steps, swims and splashes: vendored `entities/src/world/emissions.rs:55` (step), `:78` (swim), `:89` (splash).
- Mob deaths: `entities/src/world.rs:4432` `take_deaths`, presented in `server.rs:1496` `drop_death_loot`.
- Thrown things and tridents: `server.rs:2089` `tick_thrown`, `:1923` `tick_tridents`.
- Wind charges: `server.rs:1889` `wind_burst` (round 9 left their `block_open`, `block_close`, `block_activate` and `block_change` for this track).

### Server tick, queues and batching
- `run_tick` (`server.rs:4195`): the level tick (`sim.tick()` at `:4222`, which advances game time), then mobs (`:4248`), then outputs. W dispatches all game events of the tick after the mobs and before the outputs, each stamped with the game time it happened in, then ticks the listeners. This equals vanilla's immediate delivery: `VibrationSelector.chosenCandidate(tick)` only hands over a candidate from an earlier tick (javap), so nothing posted this tick is received before the next.
- `TickInput` (`server.rs:3547`) gets `game_events`; `Entities::tick` (`entities.rs:1467`, the input at `:1489`) fills it from a new `PlayerQueue.game_events` (`entities.rs:191`). A tick input that replaces a running batch keeps the older events the way `carry_omens` (`server.rs:3577`) keeps omens (batching at `:4161-4179`), `remove_player` (`:3593`) drops them, and `run_tick` drains them once (a batch runs `run_tick` several times with the same input).

### Protocol
`minecraft_net::PROTOCOL` is 6 (`crates/minecraft_net/src/messages.rs:7`) and shipped in round 10 release 1. W changes serialized structs that friends decode (`EntitySnapshot` bursts in W2, `MobShow` in W4-W6). The first W commit that changes one bumps `PROTOCOL` to 7 unless another round 10 track already bumped it since the last release, in which case W rides in that number. At merge time the number goes up at most once per release; record it in `docs/round10/decisions.md` and the track log. If D0's brief sets a different rule for `MobShow`, follow D0.

### Mob registry (D0)
The warden is a new mob family. Its wiring goes **through the D0 registry (`docs/round10/track-d0.md`, plug-in checklist)**: the family's own files plus one line per registry. Before W4 starts, read that checklist and follow it exactly; it replaces round 9's 55-site chain (`docs/round9/mobs.md` "The mob pattern"). The hit-key byte comes from D0's allocation (round 9 ended with 35 for withers, `entities.rs:2071`, next free 36). Track W updates the warden's row (14) in the mob list D0 names (`docs/round9/mobs.md` today) in the same commit as the code.

### Files and docs
- New vendored files are counted in the log (patched and added) for the CLAUDE.md count at the end of the round.
- Do not edit `CLAUDE.md`, `docs/MINECRAFT-SURVIVAL.md`, `docs/MULTIPLAYER.md`, `docs/PROGRESS.md` or `docs/round10/decisions.md` from a track run; write their new wording under "State doc changes" in `docs/round10/track-w-log.md`.
- Ancient city loot already rolls: round 9's group loot-functions ported the chest functions, and `terrain::tests::ancient_cities_generate_in_the_deep_dark_with_sculk_shriekers_and_loot_chests` (`crates/minecraft_terrain/src/terrain.rs:2739`) generates a city with shriekers (`can_summon=true`), sensors, catalysts and at least 10 loot chests that roll. Nothing to build.
- **No night vision.** Night vision is not one of the simulated effects (`EffectKind`, `third_party/minecraftoss/player/src/survival.rs:36-59`, has no `NightVision`; the potion only exists as an item and a HUD icon colour), so a night vision potion does not light the deep dark. The owner's steps therefore never rely on it: the warden groups test on open ground at night or in a cave with a few torches (an egg's or `/summon`'s warden needs no darkness, `SPAWN_ITEM_USE` and `COMMAND` skip the light rule), and W7's summoning uses open ground at night, which passes vanilla's monster light check (sky light 15 against `random(32)`, block light 0, night-time raw brightness 4 against the 0-7 test). Night vision is left for later in the log.

---

## W1. Darkness (group `darkness`)

### The owner's finding
Round 9 release 2: `/effect give @s minecraft:darkness 30` shows the icon and nothing else. Wardens and shriekers need the real effect.

### Cause
- The effect exists only as data: `EffectKind::Darkness` (`third_party/minecraftoss/player/src/survival.rs:54`, in `ALL` at `:78`), saved (`crates/minecraft_survival/src/player_nbt.rs:34`), given by command (`commands.rs:173`) and drawn as an icon (`crates/hud/src/minecraft_effects.rs:52`).
- The sky picks lava or powder snow fog only: `crates/render_anim/src/minecraft_world.rs:1681-1685` (`lava_fog::in_lava`, `in_powder_snow`, `crates/minecraft_terrain/src/lava_fog.rs:24-60`). `SkyInputs` (`minecraft_world.rs:1596-1609`) has no darkness. The host fills it at `:1554-1579`, a friend at `crates/render_anim/src/minecraft_remote.rs:1665-1706` from `remote.player.effects`.
- The fog colour darkens only towards the void: `crates/minecraft_terrain/src/environment.rs:196-200`.
- The brightness option is pinned at 0.5 in `environment[10].w` (`minecraft_world.rs:1705-1706`), which the terrain lightmap reads as `environment.cloud.w` (`crates/render_gpu/src/drawsurf/minecraft_world.rs:970`); the terrain lightmap (`:957-971`, clamp at `:966`) and the model lightmap (`crates/render_gpu/src/drawsurf/minecraft_light.rs:275-293`, clamp at `:286`) subtract nothing.
- Free lanes: the `w` lanes of `environment[0..3]` are written as 0 (`minecraft_world.rs:1690-1693`), but **`environment[3].w` is taken**: the model pass overwrites it with the eye's sky light (`minecraft_light.rs:249`) and reads it at `:359`. `environment[0].w`, `[1].w` and `[2].w` (`forward`, `right`, `up` in the terrain shader's `Environment` struct, `render_gpu/src/drawsurf/minecraft_world.rs:908-916`) are read by no shader and no Rust code (grep `environment\[0\]`, `forward.w` in `crates/render_gpu`).

### Vanilla 26.3
- `MobEffects.DARKNESS`: HARMFUL, colour 2696993, `setBlendDuration(22)`, which sets blend-in, blend-out and blend-out-advance to 22 (`MobEffect.setBlendDuration(I)` calls `(III)`).
- `MobEffectInstance$BlendState.tick`: the target is 1 while the effect does **not** `endsWithin(22)` (not infinite and at most 22 ticks left), else 0; the factor moves towards it by at most 1/22 a tick; `getFactor(partial)` lerps the previous and current factor. The state belongs to the effect instance, so milk (which removes the instance) ends the look at once, while a natural end fades out over the last 22 ticks.
- `DarknessFogEnvironment.setupFog`: `d = lerp(blend, renderDistanceBlocks, 15)`; environmental start `0.75 d`, environmental end, sky end and cloud end `d`. `FogRenderer.setupFog` runs only the **first** applicable environment in the order lava, powder snow, blindness, darkness, water, atmospheric, so darkness never shortens lava or powder snow fog. Render distance in blocks is chunks × 16.
- `FogRenderer.computeFogColor` (**resolved**, "fog colour darkening"): the void darkness value becomes `getModifiedDarkness = max(blend, voidDarkness)` from the first environment that `modifiesDarkness` (blindness, then darkness); when it is above 0 and the camera is not in lava or powder snow, the fog colour is multiplied by `(1 - value)²`. Night vision's fog brightening is skipped while the player has darkness (`hasEffect(DARKNESS)`, javap); the fork has no night vision at all (W0), so nothing to do.
- `LightmapRenderStateExtractor.extract` (javap this round): `brightness = max(0, gamma - blend × darknessEffectScale)` (gamma 0.5 by default, `darknessEffectScale` option 1.0 by default; `blend` is `LocalPlayer.getEffectBlendFactor(DARKNESS, partial)`); `calculateDarknessScale(player, blend, partial) = max(0, cos((tickCount - partial) × π × 0.025) × 0.45 × blend)`, times `darknessEffectScale`. A pulse of 80 ticks, dark for half of it.
- `lightmap.fsh` (26.3 jar): `color = color - vec3(DarknessScale)` after the boss-overlay darkening and **before** `clamp(color, 0, 1)` and the `notGamma` blend by `BrightnessFactor`.

### Plan
1. **"Minecraft darkness closes in the fog like vanilla"**
   - New `crates/minecraft_terrain/src/darkness.rs`: `DarknessBlend { factor, previous }` with `tick(present_and_not_ending: bool)` and `factor(partial)`; `fog(sky, blend, render_distance) -> SkyState` (end, start, sky and cloud ends); `fog_darkening(void, blend)` for the colour.
   - `environment.rs:196-200`: the void darkness term takes `max(void, blend)` (a new `View` field, `environment.rs:90`).
   - `minecraft_world.rs`: `SkyInputs.darkness: f32` (the blend at the partial tick); `present_sky` applies `darkness::fog` after the lava and powder snow match (`:1681-1685`, only when neither applies). The host keeps one `DarknessBlend` per viewer next to its other per-frame sky state and ticks it once per client tick from `survival.status.effect(EffectKind::Darkness)`; the friend keeps its own in the remote state and ticks it from `remote.player.effects` (`minecraft_remote.rs:1668-1674` reads effects the same way; `minecraft_net::EffectView` carries `duration`, `crates/minecraft_net/src/messages.rs:439-444`, so the friend applies the same `endsWithin(22)` rule, counting the duration down between player views).
2. **"Minecraft darkness pulses the light like vanilla"**
   - `darkness::light(blend, tick, partial) -> (brightness, scale)` with vanilla's formulas.
   - `minecraft_world.rs:1705-1706`: brightness lane `max(0, 0.5 - blend)`; the scale in `environment[0].w` (`:1690`, the `forward` lane; not `[3].w`, which the model pass owns, see Cause).
   - Both lightmaps subtract the scale before the clamp: the terrain one reads `view.environment.forward.w` (`render_gpu/src/drawsurf/minecraft_world.rs:966`), the model one `mc_light.environment[0].w` (`minecraft_light.rs:286`; `light.environment` is a copy of the frame's, `:247`). The model lightmap keeps its 0.0 brightness mix (`:292`).
   - The tick count is the viewer's client tick counter; the friend uses its own.

### Tests
- `darkness::tests::the_blend_ramps_over_22_ticks_and_fades_before_the_end`: 0 → 1 in 22 ticks; an effect with 30 ticks left starts fading 8 ticks later and reaches 0 when it ends; an infinite one stays at 1; the partial lerps.
- `darkness::tests::fog_closes_to_fifteen_blocks`: blend 1 gives end 15 and start 11.25 (sky and cloud ends 15), blend 0 the render distance, blend 0.5 halfway; lava and powder snow keep their own fog.
- `darkness::tests::fog_colour_darkens_by_the_square`: blend 0.5 multiplies the colour by 0.25; the void term wins when larger.
- `darkness::tests::the_light_pulses_every_four_seconds`: scale 0.45 at tick 0, 0 at tick 20 and 40 (clamped), 0.45 at tick 80, half at blend 0.5; brightness 0.5 → 0 for blend ≥ 0.5.
- `render_anim` (host and friend): `minecraft_world::tests::darkness_reaches_the_environment_block` and `minecraft_remote::tests::a_friend_with_darkness_gets_the_same_fog`.
- `render_gpu`: both shader sources subtract `environment[0].w` before the clamp (string test, as the existing WGSL tests do), and the model pass still writes only `environment[3].w` (`minecraft_light.rs:249`).

### Owner's steps (survival, by day, open ground)
1. `/effect give @s minecraft:darkness 30`: within about a second the far land fades into black fog about 15 blocks away, and the light around you dims and brightens in a slow pulse (about 2 s dark, 2 s normal). **w01_darkness_fog**
2. Stand in a lit cave or at night by torches: the pulse makes torch-lit blocks go almost black and come back. **w02_darkness_pulse**
3. Drink a milk bucket: the view snaps back at once and the icon is gone.
4. `/effect give @s minecraft:darkness 3`: the darkness fades in and, in the last second, fades out smoothly.
5. With darkness on, stand in lava (fire resistance from `/effect give @s minecraft:fire_resistance 60`) and in powder snow: their own fog stays as before.
6. Friend window: the friend runs step 1 on themselves: the friend's view goes dark and pulses, the host's does not. Then the host gives the friend darkness with `/effect give <friend name> minecraft:darkness 30` (or the friend's own command): only the friend's view changes. **w03_friend_darkness**

```
IW4L_MINECRAFT_KIT=minecraft:milk_bucket,minecraft:milk_bucket,minecraft:milk_bucket,minecraft:lava_bucket,minecraft:powder_snow_bucket,minecraft:torch*64,minecraft:cobblestone*64,minecraft:glass*16,minecraft:cooked_beef*32
```

---

## W2. Game events and sculk sensors (group `vibrations`)

### The owner's finding
Sculk sensors do nothing (round 9 release 2 known limits). Sensors are the base of every sculk behaviour: shriekers listen to them, wardens listen to the same events.

### Cause
- No game event system: grep finds only the block entity NBT defaults (`third_party/minecraftoss/core/src/block_entity.rs:277-288` keeps `last_vibration_frequency`, `warning_level`, `listener` and `cursors`), the level entities' `step_on` that reports sensors and shriekers as unsupported (`world/src/level/entity.rs:1234-1238`) and a comment in `entities/src/movement.rs:199`.
- Sensors are plain blocks: `Level::tick_block` (`world/src/level/mod.rs:839`) and `has_ticker` (`world/src/level/container.rs:475`) cover no sculk block; redstone has no sensor kind.
- No queue: `Level` has `sounds` (`mod.rs:147`) and `level_events` (`:207`); `EntityWorld` (`entities/src/world.rs:2487`) has sounds; `TickInput` has no events.

### Vanilla 26.3
- **Game events** (`GameEvent.<clinit>`): radius 16 for every event except `jukebox_play` and `jukebox_stop_play` (10) and `shriek` (32). Frequencies (`VibrationSystem.VIBRATION_FREQUENCY_FOR_EVENT`): step, swim, flap 1; projectile_land, hit_ground, splash, bounce 2; item_interact_finish, projectile_shoot, instrument_play 3; entity_action, elytra_glide, unequip 4; entity_dismount, equip 5; entity_interact, shear, entity_mount 6; entity_damage 7; drink, eat 8; container_close, block_close, block_deactivate, block_detach 9; container_open, block_open, block_activate, block_attach, prime_fuse, note_block_play 10; block_change 11; block_destroy, fluid_pickup 12; block_place, fluid_place 13; entity_place, lightning_strike, teleport 14; entity_die, explode 15; `resonate_N` is N; everything else 0 (`sculk_sensor_tendrils_clicking`, `shriek`, `item_interact_start`, the jukebox events).
- **Tags** (26.3 data): `#vibrations` (what sensors hear) has no `shriek`, no `sculk_sensor_tendrils_clicking`, no `item_interact_start`; `#warden_can_listen` = `#vibrations` without `flap`, plus `shriek` and `#shrieker_can_listen` (= `sculk_sensor_tendrils_clicking`); `#ignore_vibrations_sneaking` = hit_ground, projectile_shoot, step, swim, item_interact_start, item_interact_finish; block `#dampens_vibrations` and item `#dampens_vibrations` = `#wool`, `#wool_carpets`, `#wool_slabs`, `#wool_stairs`; block `#occludes_vibration_signals` = `#wool`; `#vibration_resonators` = amethyst block. Read them from the data pack at load.
- **Dispatch** (`GameEventDispatcher.post`): every listener in the sections within the event's radius; UNSPECIFIED listeners (sensors, shriekers, wardens) handle it at once, BY_DISTANCE listeners (catalysts) are sorted by distance and all handled after.
- **Listener checks** (`VibrationSystem$Listener.handleGameEvent`), in order: nothing while a vibration is travelling (`currentVibration` set); `User.isValidVibration`: the listenable tag; a spectator source never; a source `isSteppingCarefully` (sneaking) with an event in `#ignore_vibrations_sneaking` never; a source that `dampensVibrations` (an item entity of a `#dampens_vibrations` item, a warden) never; an `affectedState` in block `#dampens_vibrations` never (so placing, breaking or stepping on wool is silent); then the listener's position exists; `canReceiveVibration`; then **occlusion**: from the source's block centre to the listener's block centre, one ray per direction (the start moved 1e-5 along it); occluded only if **every** ray hits an `#occludes_vibration_signals` block; then `scheduleVibration` adds `VibrationInfo(event, distance, pos, source)` to the selector at the current game time.
- **Selector** (**resolved**, tie rule, `VibrationSelector.shouldReplaceVibration`): a candidate from a newer tick replaces the kept one; in the same tick the nearer replaces, the farther is dropped, and at equal distance the higher frequency replaces. `chosenCandidate(now)` returns it only when its tick is before `now`.
- **Ticker** (`VibrationSystem$Ticker.tick`): with no current vibration, take the chosen candidate, set the travel time `floor(distance)` ticks and send the `vibration` particle (`VibrationParticleOption(listener position, travel ticks)`, from the source position, resent after a load); count it down; at 0 (with the adjacent chunks ticking for sensors) call `onReceiveVibration(event, source, projectileOwner, distance between block positions)` and clear it. Saved as `listener: {event: {game_event, distance, pos, source?, projectile_owner?}, event_delay, selector: {event?, tick}}` (`VibrationSystem$Data`, `VibrationInfo`, `VibrationSelector` codecs).
- **Sensor** (`SculkSensorBlock`, `SculkSensorBlockEntity$VibrationUser`): radius 8, block position source; `canReceiveVibration`: not its own `block_place`/`block_destroy`, frequency not 0, and `canActivate` (phase inactive). On receive: `last_vibration_frequency` = the frequency; power `getRedstoneStrengthForDistance(distance, 8) = max(1, 15 - floor(15/8 × distance))`; `activate`: phase ACTIVE with that power, scheduled tick after 30 (calibrated: 10), neighbours updated, `tryResonateVibration` (each adjacent `#vibration_resonators` block posts `resonate_<frequency>`), the game event `sculk_sensor_tendrils_clicking` **with the original source entity**, and the `block.sculk_sensor.clicking` sound at the centre. The scheduled tick: ACTIVE → COOLDOWN (power 0, `clicking_stop` sound when not waterlogged) with a tick after 10; COOLDOWN → INACTIVE. Comparator output: `last_vibration_frequency` while ACTIVE, else 0. `stepOn` (every tick an entity stands on it, sneaking or not, not a warden): if it can receive a `step`, `forceScheduleVibration` (no tag, sneak or occlusion checks). Saved `last_vibration_frequency` and `listener`.
- **Calibrated sensor**: radius 16, active 10 ticks; refuses a frequency other than the redstone signal coming in from behind it (`getBackSignal`) when that signal is not 0.
- **First sources** (javap call sites): `Entity.vibrationAndSoundEffectsFromBlock` → `step`; `applyMovementEmissionAndPlaySound` → `swim`; `doWaterSplashEffect` → `splash`; `checkFallDamage` → `hit_ground`; `BlockItem.place` → `block_place` (source the player, affected the placed state); `Level.destroyBlock` → `block_destroy` (affected the old state); `Projectile.tick` → `projectile_shoot` at the projectile with the **owner** as source; `Projectile.onHit` → `projectile_land` at the hit point with the **projectile** as source (affected the block state for a block hit), so listeners see its owner as `projectile_owner`; `ServerExplosion.explode` → `explode`.

### Plan
1. **"Minecraft game events reach their listeners like vanilla"**
   - New `crates/minecraft_terrain/src/game_events.rs`: the event table (id, frequency, radius) from the javap above, the tags, `Posted { event, pos, source, projectile_owner, affected, tick }` with `Source::Player { id, sneaking, spectator } | Mob { id } | Item { id, dampens }`, and `is_valid(listenable, posted)`.
   - Queues: vendored `world/src/level/mod.rs` (next to `level_events` `:207`): `game_events` and `acting_player: Option<u64>` with `Level::game_event`; vendored `entities/src/world.rs`: `game_events` with `EntityWorld::game_event`; `TickInput.game_events` from `PlayerQueue.game_events` through `Entities::game_event` (see W0 for batching).
   - `ServerSim` sets `level.acting_player` around each player command that reaches the level (`use_block`, `item_on_block_by`, `player_edit_block`, `player_mines`, `ring_bell`, `set_chest_openers`, bone meal, `attack_block`); the eight commands listed in W0 ("Commands without a player") gain `player: u64`, with their senders (`server.rs:3741-3791`, `:3972`) and callers (`entities.rs:974`, `:982`, `:990`, `:1327`, `:1361`).
   - New `crates/minecraft_terrain/src/vibrations.rs`: listener index per chunk section (sensor, calibrated sensor, shrieker and catalyst block entities; rebuilt on chunk load, unload and block set), the dispatcher (radius, UNSPECIFIED first, BY_DISTANCE sorted), the selector, the travel ticker, the occlusion rays, and the `listener` NBT round trip in vanilla's format in the block entity tag. Dispatched in `run_tick` after the mobs (W0).
2. **"Minecraft sculk sensors hear vibrations and power redstone"**
   - New vendored `world/src/level/sculk.rs`: phases and scheduled ticks (`tick_block`, `mod.rs:839`), `activate` with power and frequency, cooldown, sounds, the tendrils clicking event (resonance is W3's commit 3), comparator output, a redstone `Kind::SculkSensor` (signal = power; calibrated takes its back input), calibrated filtering, `stepOn` for level entities (replace `entity.rs:1234-1238` for sensors) and for players and mobs (the server forces a `step` when a body stands on one).
   - The `vibration` particle: an `EntitySnapshot` burst list (`server.rs:329-348`, kept by `mob_sender.rs:220-255`) of `{from, to, ticks}`, drawn by the host and by friends with the 26.3 `particle/vibration.png` sprite, moving to the listener over the travel ticks.
3. **"Players, mobs, blocks and explosions make Minecraft vibrations"**
   - Players (host and friends, sneaking from the MW2 crouch and prone flags): `step` and `swim` at `minecraft_players.rs:364-397`, `hit_ground` at `:412-424`, `block_place` and `block_destroy` through `level.rs:172-186` and `entities.rs:1361`.
   - Mobs: `step`, `swim`, `splash` in `entities/src/world/emissions.rs:55`, `:78`, `:89`.
   - Projectiles: `projectile_shoot` and `projectile_land` for thrown items, tridents and arrows (`server.rs:2089`, `:1923`, and the entity world's arrows).
   - Explosions: `explode` in `explosion.rs:151` (creepers, TNT, beds, anchors, crystals, MW2 blasts).

### Tests
- `game_events::tests::frequencies_and_radii_match_vanilla` (the full table), `the_tags_load_from_the_data_pack` (gated on `MINECRAFTOSS_ROOT`).
- `vibrations::tests`: `a_step_five_blocks_away_reaches_a_sensor_five_ticks_later_with_power_six` (`max(1, 15 - floor(15/8 × 5))` = 6); `a_sneaking_step_is_not_heard_but_a_sneaking_block_place_is`; `a_wool_shell_occludes_and_one_open_side_does_not`; `stepping_on_a_carpet_is_dampened`; `the_nearest_candidate_wins_and_a_tie_goes_to_the_higher_frequency`; `nothing_new_is_heard_while_a_vibration_travels`; `a_listener_with_a_travelling_vibration_round_trips_its_nbt` (vanilla field names); `other_dimensions_never_hear` (each `ServerSim` has its own).
- `level::sculk::tests` (vendored world): `phases_go_active_thirty_cooldown_ten_inactive`; `a_calibrated_sensor_is_active_ten_and_filters_by_its_back_input`; `the_comparator_reads_the_last_frequency_while_active`; `its_own_placement_and_breaking_is_not_heard`; `an_item_on_it_activates_it`.
- `server::tests::a_redstone_lamp_lights_when_a_player_walks_near_a_sensor` (through `run_tick`, host and a second player id).
- `mob_sender::tests::a_vibration_burst_reaches_friends_near_it`.

### Owner's steps (survival; build on stone, away from other noise)
1. Place a sculk sensor with a redstone lamp next to it. Walk around 4 blocks away: a ripple flies from your feet to the sensor, its tendrils light up and click, the lamp lights for a second and a half, then the sensor rests for half a second. **w04_sensor_lamp**
2. Crouch and walk (MW2 crouch or prone): nothing. Crouch and place a block 3 blocks away: it triggers (placing is never quiet).
3. Put a comparator out of the sensor into a straight line of 15 redstone dust: the number of lit dust blocks is the frequency. Walking lights 1; breaking a block nearby lights 12; placing one lights 13. Wait 2 s between actions (the sensor is busy 30 ticks plus 10 cooling down). **w05_sensor_comparator**
4. Build a closed shell of white wool around the sensor (all sides): walking outside does nothing. Open one side: it triggers again. Walk on white carpet next to it: nothing. Place or break wool next to it: nothing.
5. Stand 12 blocks from the sensor, crouch (so the throw itself is not heard) and throw a snowball so it lands 5 blocks from the sensor: it triggers and the line lights 2. Thrown standing up from 5 blocks away, the throw is heard first and the line lights 3. Shoot an arrow into the ground near it: it triggers.
6. Light TNT 7 blocks from the sensor with a cobblestone wall between them (stone does not block vibrations, only wool does) and step back: lighting it triggers the sensor (10, prime fuse); the blast 4 s later triggers it again and the line lights 15.
7. A cow from its egg wandering near the sensor triggers it with its steps.
8. Place a calibrated sculk sensor and lay 15 redstone dust from a lever so the last dust ends against the sensor's back face (that dust carries strength 1); flip the lever on: only steps (frequency 1) trigger it, and walking 12 blocks away still reaches it (radius 16).
9. Save and quit while a ripple is in flight, load: the sensor still receives it.
10. Friend window: the friend's steps trigger the sensor and the host's lamp lights; both see the same ripple. The friend crouching makes no ripple. **w06_friend_sensor**

```
IW4L_MINECRAFT_KIT=minecraft:sculk_sensor*16,minecraft:calibrated_sculk_sensor*4,minecraft:redstone_lamp*8,minecraft:redstone*64,minecraft:comparator*4,minecraft:repeater*4,minecraft:lever*4,minecraft:white_wool*64,minecraft:white_carpet*32,minecraft:snowball*16,minecraft:bow,minecraft:arrow*64,minecraft:tnt*4,minecraft:flint_and_steel,minecraft:cow_spawn_egg*4,minecraft:stone*64,minecraft:cobblestone*64,minecraft:cooked_beef*16
```

---

## W3. More vibrations, resonance and MW2 guns (group `vibrations-more`)

### The owner's finding
Round 9's D5 plan: "Containers, doors, trapdoors, gates, buttons, levers, plates, eat, drink, equip, damage, death, buckets, prime fuse, shear, entity place, teleport, elytra glide, mount, note blocks; MW2 guns; wool dampening and occlusion; amethyst resonance". Dampening and occlusion are part of W2's listener checks; the rest is here.

### Cause
None of these sources posts an event after W2: the call sites in W0 are plain state changes.

### Vanilla 26.3 (javap call sites)
- Blocks: `DoorBlock` (`useWithoutItem`, `setOpen`, `neighborChanged`) and `FenceGateBlock` (also `onExplosionHit`) → `block_open`/`block_close`; `TrapDoorBlock.playSound` → the same; `ButtonBlock.press` → `block_activate`, `checkPressed` → activate/deactivate; `LeverBlock.pull` → activate/deactivate; `BasePressurePlateBlock.checkPressed` → activate/deactivate; `NoteBlock.playNote` → `note_block_play`; `ContainerOpenersCounter` (chests, barrels, ender chests) → `container_open` when the openers go from 0 to some, `container_close` back to 0, `ShulkerBoxBlockEntity` likewise; `BucketItem` → `fluid_pickup`/`fluid_place`; `TntBlock.prime` → `prime_fuse`; `BellBlock.attemptToRing` → `block_change`; `FlintAndSteelItem` → `block_place` (fire) or `block_change` (candles, campfires); `ComposterBlock`, `CakeBlock` (also `eat`, and `block_destroy` for the last slice), `LecternBlock.resetBookState`, `DaylightDetectorBlock.useWithoutItem`, `SweetBerryBushBlock`, `DecoratedPotBlock`, `LayeredCauldronBlock`, `ChiseledBookShelfBlock.removeBook`, `AbstractCandleBlock.extinguish`, `ItemFrame` → `block_change`; `PistonBaseBlock.triggerEvent` → activate/deactivate, `moveBlocks` → `block_destroy`; `DispenserBlock.dispenseFrom` → `block_activate`; `TripWireHookBlock` → activate, deactivate, attach, detach; `JukeboxBlockEntity` → `block_change`, `jukebox_stop_play`; `LightningBolt.tick` → `lightning_strike`; `BeehiveBlock` → `shear`, `fluid_pickup`.
- Entities: `Consumable.onConsume` → `eat`/`drink`; `LivingEntity.onEquipItem` → `equip`/`unequip`; `actuallyHurt` → `entity_damage` (also `Player.actuallyHurt`, `ItemEntity.hurtServer`); `LivingEntity.die` and `ServerPlayer.die` → `entity_die`; `startUsingItem`/`stopUsingItem` → `item_interact_start`/`finish` (start is in no listenable tag); `checkTotemDeathProtection` and `BoneMealItem.useOn` → `item_interact_finish`; `updateFallFlying` → `elytra_glide`; `startRiding` → `entity_mount`, `removeVehicle` → `entity_dismount`; `Entity.interact` → `entity_interact` (or `entity_action`); `processFlappingMovement` → `flap` (chickens, bats, parrots, bees); `restituteMovementAfterCollisions` → `bounce` (slime blocks, beds); `SpawnEggItem.spawnMob`, `ArmorStandItem`, a chicken laying an egg → `entity_place`; `Shulker.teleportSomewhere`, chorus fruit (`TeleportRandomlyConsumeEffect.apply`) and `Enderman.teleport(DDD)` (class `net.minecraft.world.entity.monster.Enderman`) → `teleport` (**resolved**, javap this round); an ender pearl's landing posts only `projectile_land` (`ThrownEnderpearl.onHit` calls `Projectile.onHit`, then moves its owner with `ServerPlayer.teleport(TeleportTransition)`, which posts no game event) plus the owner's `entity_damage` from the pearl's fall damage in the same tick; `dropAllLeashConnections` and shearing → `shear`.
- **`entity_die` comes before the experience (resolved)**: `LivingEntity.die` posts `entity_die` and then calls `dropAllDeathLoot`, whose last step is `dropExperience`, which drops nothing once `wasExperienceConsumed` (W7's catalyst sets it).
- Resonance: `SculkSensorBlock.tryResonateVibration` (every sensor, not only calibrated) posts `resonate_<frequency>` from each adjacent amethyst block.

### Plan
1. **"Minecraft blocks make vibrations when used"**: the level side, one helper call per site: `redstone.rs:990-1046` (doors, trapdoors, gates, note blocks, levers, buttons, daylight detectors), button and plate ticks, `redstone.rs:305` container openers, `redstone.rs:776` bells, `dispenser.rs:197`, `:249` buckets, `explosion.rs:80` TNT, `fire.rs:123` fire, `piston.rs:392` pistons, dispensers, `entity.rs:609` lightning, the wind charge triggers in `server.rs:1889`, composters, cakes, candles, lecterns. Source: the acting player, or none.
2. **"Minecraft players and mobs make vibrations when they eat, drink, equip, ride, glide, shear, teleport, get hurt and die"**: `actions.rs:448` eat and drink; armour and hand equip changes (player inventory and mob equipment); player and mob damage and death (the entity world's hurt and `take_deaths`, `world.rs:4432`; player deaths on the survival side); mounting and dismounting (riding); elytra gliding (`crates/render_anim/src/minecraft_elytra.rs`); shearing (`crates/minecraft_terrain/src/mob_actions.rs:202`); spawn eggs (`server.rs:1557`); teleports; chicken flaps and egg laying; bouncing. Item entities of wool dampen (`Source::Item`).
3. **"Minecraft amethyst resonates sculk vibrations to calibrated sensors"**: `sculk.rs` posts `resonate_N` from adjacent amethyst blocks; a calibrated sensor set to N picks it up.
4. **"MW2 guns make Minecraft vibrations"** (decision below): in `minecraft_world.rs` before the build filter at `:973` (and `minecraft_levels.rs:266-326` for friends' levels), each bullet's `VoxelEvent::Ray` posts `projectile_shoot` at `from` with the shooter as source (once per shooter per tick, so a shotgun's pellets count once) and, only when the ray stopped at a block (a `Shot` from that attacker this tick) or a mob (a `MobShot`), `projectile_land` at `to` with the shooter as **projectile owner** and the hit block as affected state; a ray that ended in the air (`combat.rs:1245-1249`) lands nowhere, as a vanilla projectile that hits nothing. A mob hit posts `entity_damage` through the entity world as any hurt. Grenades and rockets post `explode` through `weapon_blast` (W2). The shooter's sneaking flag is the MW2 crouch or prone flag.

**Decision, MW2 guns.** A bullet is a projectile, so it behaves like an arrow: shooting while crouched or prone is not heard (`projectile_shoot` is in `#ignore_vibrations_sneaking`), the impact always is (`projectile_land` is not), and a warden hears the impact as a projectile (W4: anger 10 at a shooter beyond 30 blocks is not given; a shooter within 30 gets the projectile rule). Record it in `docs/round10/decisions.md` through the log.

### Tests
- `level::redstone::tests::doors_gates_trapdoors_levers_buttons_and_note_blocks_post_their_events` (each id, the acting player as source, `block_close` on closing).
- `server::tests::opening_a_chest_posts_container_open_once_and_closing_the_last_viewer_posts_close`.
- `server::tests::a_dead_cow_posts_entity_die_before_its_experience`.
- `vibrations::tests::eating_eight_blocks_away_powers_a_sensor_to_one` and `a_bell_ring_is_frequency_eleven`.
- `sculk::tests::an_amethyst_block_beside_a_sensor_resonates_its_frequency_to_a_calibrated_sensor`.
- `minecraft_world::tests::an_mw2_bullet_posts_its_shot_and_landing_and_a_crouched_shot_only_its_landing` (and a ray ending in the air posts no landing).

### Owner's steps (survival; sensor plus comparator into a line of redstone dust, so the signal strength shows the frequency)
Wait two seconds between actions: a sensor is busy for 2 s after each vibration (30 ticks active, 10 cooling down) and hears nothing in that time.
1. Open an oak door 5 blocks from the sensor: the line shows 10; close it: 9. Do the same with a trapdoor and a fence gate; flip a lever on (10) and off (9); press a button (10); step on a pressure plate (10). **w07_block_frequencies**
2. Open a chest and a barrel: 10; close: 9. Eat an apple: 8. Drink a milk bucket: 8. Put a helmet on: 5; take it off: 4. Fill a water bucket: 12; empty it: 13.
3. A chicken from its egg near the sensor: its wing flaps when it falls trigger 1.
4. Stand 12 blocks away, crouch and throw a snowball so it lands next to the sensor: the landing gives 2. (An ender pearl is no good here: its landing also hurts you in the same tick, and the hurt, 7, wins.)
5. Put an amethyst block against a sensor. Place a calibrated sensor 14 blocks away on one side and give it a back input of 8 (a lever, then 8 redstone dust ending against its back face). Walk to 4 blocks from the first sensor on the side away from the calibrated one (18 blocks from it, out of its 16-block reach) and eat: the first sensor triggers, and a moment later the calibrated one lights from the amethyst's resonance. **w08_amethyst_resonance**
6. MW2: stand 20 blocks from the sensor and shoot the ground beside it: the sensor triggers (2). Stand 6 blocks from it and shoot a wall 30 blocks away on the other side: the shot itself triggers (3). Go prone and shoot that wall: nothing; shoot the ground beside the sensor: it triggers (2). Throw a grenade near it: 15. **w09_mw2_vibrations**
7. Friend window: the friend opens the door and shoots near the sensor: the host's comparator line shows the same numbers. **w10_friend_vibrations**

```
IW4L_MINECRAFT_KIT=minecraft:sculk_sensor*16,minecraft:calibrated_sculk_sensor*4,minecraft:redstone_lamp*8,minecraft:redstone*64,minecraft:comparator*4,minecraft:repeater*4,minecraft:amethyst_block*8,minecraft:oak_door*2,minecraft:oak_trapdoor*2,minecraft:oak_fence_gate*2,minecraft:chest*2,minecraft:barrel*2,minecraft:lever*4,minecraft:oak_button*4,minecraft:stone_pressure_plate*4,minecraft:note_block*2,minecraft:water_bucket,minecraft:bucket,minecraft:apple*16,minecraft:milk_bucket,minecraft:diamond_helmet,minecraft:chicken_spawn_egg*4,minecraft:snowball*16,minecraft:white_wool*32,minecraft:cooked_beef*16,minecraft:cobblestone*64
```

---

## W4. The warden lives (group `warden-live`, needs D0)

### The owner's finding
"The warden" is third in the survival order. Its egg does nothing and wardens from a save stay dormant.

### Cause
- No warden entity: `minecraft:warden` is not in `SIMULATED_KINDS` (`crates/minecraft_terrain/src/server_mobs.rs:433`), so `spawn_saved` (`:580`) returns `None`, a saved warden goes dormant and `HATCHING_EGGS` (`server.rs:7717`, 50 kinds) has no warden. `mob_tracking::tracking_range` (`mob_tracking.rs:285`) has no `warden` (falls to `DEFAULT_RANGE` 5). The round 9 mob list row 14 (`docs/round9/mobs.md:26`) is `left`.
- `SpawnUtil.trySpawnMob` is ported only for golems with `LEGACY_IRON_GOLEM` (`third_party/minecraftoss/entities/src/golem_spawn.rs`); there is no `ON_TOP_OF_COLLIDER`.
- The monster light check exists for natural spawns only (`world/src/natural_spawner/spawning.rs:774` `dark_enough`).
- The brain framework exists (`entities/src/brain_behaviors.rs`, `hoglin_brain.rs`, `piglin_brain/`, `villager_brain/`).

### Vanilla 26.3
- `EntityTypes.WARDEN`: MONSTER, `sized(0.9, 2.9)`, passenger attachment 3.15, `WARDEN_CHEST` attachment (0, 1.6, 0), `clientTrackingRange(16)` (**resolved**), `fireImmune`, `notInPeaceful`. `SpawnPlacements`: `NO_RESTRICTIONS`, `MOTION_BLOCKING_NO_LEAVES`, `Monster.checkMonsterSpawnRules`. No biome spawns it.
- `Warden.createAttributes`: max health 500, movement speed 0.3 (`0.30000001192092896`), knockback resistance 1, attack knockback 1.5, attack damage 30, follow range 24. `xpReward` 5 (**resolved**). Loot `entities/warden`: one sculk catalyst.
- Navigation (**resolved**): `Warden$1` is a `GroundPathNavigation` whose `PathFinder` measures `Node.distanceToXZ`; `setCanFloat(true)`; malus UNPASSABLE_RAIL 0, DAMAGING 8, POWDER_SNOW 8, LAVA 8, FIRE 0; `getWalkTargetValue` 0 everywhere.
- Size while digging or emerging: `EntityDimensions.fixed(width, 1.0)`. `isInvulnerableTo`: everything but `#bypasses_invulnerability` while digging or emerging; `ignoreExplosion` then too. `dampensVibrations` true. `removeWhenFarAway` false. `checkSpawnObstruction`: the monster check plus no collision in its box. `getSoundVolume` 4.0.
- **Vibration user** (`Warden$VibrationUser`): radius 16 at its eye height, `#warden_can_listen`; `canReceiveVibration`: not no-AI, alive, no `VIBRATION_COOLDOWN`, not digging or emerging, inside the world border, and a source (if any) it `canTargetEntity`. On receive: `VIBRATION_COOLDOWN` 40 ticks, entity event 61 (tendrils), `entity.warden.tendril_clicks` at 5.0; with a projectile owner within 30 blocks: if `RECENT_PROJECTILE` is set and it can target the owner, disturbance at the owner and anger +35, else anger +10 at the owner; either way `RECENT_PROJECTILE` 100 ticks (an owner beyond 30 gets no anger); without a projectile owner, anger +35 at the source; when not yet angry, the disturbance location becomes the vibration's position.
- **Anger** (`AngerManagement`, `AngerLevel`): per suspect (entity, or UUID until loaded), max 150, CALM 0, AGITATED 40, ANGRY 80; decreases by 1 each time it ticks, which is when `(tickCount + id) % 20 == 0`; saved as `anger: {suspects: [{uuid, anger}]}`. `increaseAngerAt(entity, n, listening)` refreshes the dig cooldown, and plays the listening sound by level. `canTargetEntity`: living, same level, not creative or spectator, not allied, not an armour stand, not a warden, not invulnerable, alive, in the world border.
- **Brain** (`WardenAi`): activities by priority EMERGE, DIG, ROAR, FIGHT, INVESTIGATE, SNIFF, IDLE. Durations: emerge `ceil(133.6)` = 134, dig 100, roar 84, sniff `ceil(83.2)` = 84; dig cooldown 1200. Core: swim 0.8, `SetWardenLookTarget`, look 45/90, move. IDLE: `SetRoarTarget`, `TryToSniff` (no sniffing memory), stroll 0.5 or do nothing 30-60. INVESTIGATE: `SetRoarTarget`, go to the disturbance location at 0.7. SNIFF: `SetRoarTarget`, `Sniffing`. DIG (only without a roar target or dig cooldown): `ForceUnmount`, `Digging` (pose DIGGING, `entity.warden.dig` 5.0 and `agitated` 5.0, removes itself at the end).
- `finalizeSpawn`: `DIG_COOLDOWN` 1200; for a `TRIGGERED` spawn also pose EMERGING, `IS_EMERGING` 134 ticks and `entity.warden.agitated` at 5.0. A non-persistent warden digs away once the dig cooldown runs out; `tick` refreshes the cooldown every tick for a persistent one (`isPersistenceRequired` or `requiresCustomPersistence`).
- **Summoned wardens obey the monster light check (resolved).** `SpawnUtil.trySpawnMob` calls `Mob.checkSpawnRules`, which is `SpawnPlacements.checkSpawnRules` → `Monster.checkMonsterSpawnRules`, and `EntitySpawnReason.ignoresLightRequirements` is true only for `TRIAL_SPAWNER`. So a `TRIGGERED` warden needs `isDarkEnoughToSpawn` (block light at most the dimension's limit, 0 in the Overworld, and the light check with its random) and the block below valid, then `checkSpawnObstruction`. `trySpawnMob(type, reason, level, pos, attempts, xz, y, strategy, false)`: per attempt a random offset `randomBetweenInclusive(-xz, xz)` (x, z) and `(-y, y)`, inside the world border, `moveToPossibleSpawnPosition` with the strategy, then `noCollision` of the spawn box, `create`, the rules, `addFreshEntityWithPassengers` and `playAmbientSound`, else `discard`. `ON_TOP_OF_COLLIDER`: the block below has a full top face and the spot has no collision.
- Spawn eggs and `/summon` (reasons `SPAWN_ITEM_USE`, `COMMAND`) do not emerge and skip the light check.

### Plan
1. **"Minecraft wardens live in the entity world"**
   - Files by the D0 checklist, for example `entities/src/warden.rs` (attributes, sizes and poses, anger management, the vibration user's rules, `canTargetEntity`, invulnerability), `entities/src/warden_brain.rs` (the seven activities and their behaviours on the existing brain framework), the family's store file; the registry lines D0 lists.
   - The warden's vibration data lives in `minecraft_terrain` (`vibrations.rs` keeps one listener per warden, keyed by entity ID, positioned at its eye) because the entities crate cannot see the selector; arrival calls `EntityWorld::warden_hears(id, event, source, projectile_owner, distance, pos)` and the checks call `EntityWorld::warden_can_hear(id, source)`. Save and load both `anger` and `listener` in the warden's tag.
   - Mobs as suspects and targets: the warden attacks mobs that anger it (cows walking, zombies fighting), never wardens or armour stands; players by UUID.
   - `doPush` (a body touching it): anger +35 and the disturbance there, `TOUCH_COOLDOWN` 20.
   - Despawn on peaceful only; digging away removes it like a despawn (no loot, no experience).
2. **"Minecraft wardens are saved, hatch from eggs and dig out when summoned"**
   - Saving, loading, the egg and `/summon` by the D0 registry; `tracking_range` 16; the friend loads it from its look tag.
   - `golem_spawn.rs` (`try_spawn_golem`, `:169`, already takes a `blocked` closure): `ON_TOP_OF_COLLIDER` and a general `try_spawn_mob(kind, reason, centre, attempts, xz, y, strategy, rules)` used by the iron golem (unchanged results) and the warden. The spawn rules come in as a closure from `ServerSim`, because the entities crate cannot see the world crate (W0 rule 2) where the monster light check lives (`world/src/natural_spawner/spawning.rs:774` `dark_enough`: sky light against `random(32)`, the dimension's block light limit, raw brightness with the sky darkening against the light test). `ServerSim` builds it from the same level light for `TRIGGERED`, then `checkSpawnObstruction`.
   - `ServerSim::summon_warden(pos) -> bool` (20 tries, ±5, ±6, `ON_TOP_OF_COLLIDER`, `TRIGGERED`), and the shrieker's level-4 response (W7, already merged) calls it, falling back to the reply sound only when it fails, as vanilla's `tryRespond` does.
   - `MobShow`: the pose (standing, emerging, digging, roaring, sniffing) with the tick it started and the client anger level (`getActiveAnger` of the top suspect), so the friend can draw it from W6 on.

### Tests
- `warden::tests::a_warden_is_tall_strong_and_slow` (size, eye height, health, damage, knockback resistance, follow range, fire immunity, loot table, 5 experience, 1.0 high while digging).
- `warden::tests::anger_rises_by_thirty_five_per_vibration_caps_at_150_and_falls_one_per_second` and `the_angriest_suspect_is_its_target`.
- `warden::tests::a_far_projectile_angers_only_its_near_owner` (owner at 20: +10 then +35 with the recent projectile memory; owner at 40: none).
- `world::wardens::tests::a_summoned_warden_emerges_for_134_ticks_and_cannot_be_hurt` and `an_unbothered_warden_digs_away_after_1200_ticks` and `a_persistent_warden_never_digs`.
- `world::wardens::tests::it_walks_to_a_vibration_and_sniffs` and `a_warden_on_peaceful_is_removed`.
- `golem_spawn::tests::on_top_of_collider_needs_a_full_top_face` and `a_golem_still_spawns_where_it_did`.
- `server::tests::a_triggered_warden_needs_darkness_and_room` (a lit spot gives nothing, a dark cave gives a warden within ±5 and ±6 with `IS_EMERGING`).
- `server::tests::the_fourth_warning_summons_a_warden_in_the_dark_within_five_and_six` (moved here from W7) and `a_summoned_warden_nearby_stops_the_next_warning`.
- `server_mobs::tests::a_warden_saves_and_loads_its_anger_and_listener` (UUID suspects, a travelling vibration).
- `mob_tracking::tests::a_friend_sees_a_warden_emerge` (pose and start tick).
- The egg survey grows by one.

### Owner's steps (survival, Normal; flat open ground, at dusk or by day; night vision does nothing in this fork, see W0)
1. Use a warden egg on the ground: a warden stands there at once (an egg's warden does not dig out and needs no darkness). **w11_warden_egg**
2. Stand still 10 blocks away: it wanders and sniffs. Crouch (so the throw is not heard) and throw a snowball beside it: its tendrils twitch, it walks to where the snowball landed and sniffs. Walk around near it: it turns towards you and comes closer. Crouch-walk away: your steps make no ripples (it may still sniff you out when you are close). **w12_warden_investigates**
3. Stay out of its way for a minute (60 s with no vibration it can hear): it digs into the floor and is gone.
4. A cow from its egg near a warden: the cow's steps anger it and it goes for the cow (and kills it from W5 on).
5. `/summon minecraft:warden ~ ~ ~5`, save and quit, load: it is still there; a warden that was angry at you is still angry.
6. Friend window: the friend sees the warden stand, walk and sniff in the same place; the friend's steps make it turn to the friend. **w13_friend_warden**

```
IW4L_MINECRAFT_KIT=minecraft:warden_spawn_egg*4,minecraft:cow_spawn_egg*8,minecraft:zombie_spawn_egg*4,minecraft:snowball*16,minecraft:white_wool*64,minecraft:torch*64,minecraft:diamond_sword+sharpness=5,minecraft:shield,minecraft:bow+power=5,minecraft:arrow*64,minecraft:milk_bucket,minecraft:cooked_beef*32,minecraft:diamond_chestplate+protection=4,minecraft:diamond_helmet,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:totem_of_undying,minecraft:golden_apple*8,minecraft:deepslate*64,minecraft:cobblestone*64,minecraft:ender_pearl*16
```

---

## W5. The warden fights (group `warden-fight`, needs D0)

### The owner's finding
Round 9 brief: "melee knocks the shield down; from 10 blocks behind a wall a sonic boom hurts through diamond armour; darkness every 6 s".

### Cause
- No sonic boom in `PlayerHitKind` (`third_party/minecraftoss/entities/src/world.rs:607-628`; `Melee` already carries `disable_shield_seconds`). The survival side handles hits in `crates/minecraft_survival/src/entities.rs:1882` `mob_hit`; death lines come from `crates/minecraft_survival/src/death.rs:67` `MobAttack`.
- Before W7 there was no path for the level or a mob to give players an effect except splash potions (`Output.player_potions`, `server.rs:3641`); W7 adds `Output.player_effects`.

### Vanilla 26.3
- **Roar** (`SetRoarTarget`, `Roar`): from IDLE, INVESTIGATE or SNIFF, when `getEntityAngryAt` (the top suspect at ANGRY or above) exists and the warden can reach it, `ROAR_TARGET` is set; ROAR: pose ROARING, sound `entity.warden.roar` at 3.0 after 25 ticks (with a 25-tick sound cooldown, `ROAR_SOUND_DELAY`, `ROAR_SOUND_COOLDOWN`), anger +20 at the target (`ROAR_ANGER_INCREASE`), 84 ticks, then `Roar.stop` hands the roar target to `Warden.setAttackTarget`.
- **`Warden.setAttackTarget`** (**resolved**, javap this round; used by `Roar.stop` and the hurt rule below): erases `ROAR_TARGET`, sets `ATTACK_TARGET`, erases `CANT_REACH_WALK_TARGET_SINCE` and sets `SonicBoom.setCooldown(warden, 200)` (`TIME_TO_USE_MELEE_UNTIL_SONIC_BOOM`). So a warden that has just picked its target fights in melee for 10 s before its first sonic boom.
- **Sniffing** (`TryToSniff`, `Sniffing`): with a nearest attackable, no walk target and no `SNIFF_COOLDOWN`, sniff (pose SNIFFING, `entity.warden.sniff` 5.0) for 84 ticks, cooldown 100-200 ticks; at the end the nearest attackable within 6 across and 20 up or down gets anger +35, and its position becomes the disturbance location.
- **FIGHT**: `DIG_COOLDOWN_SETTER` (refreshes 1200 while fighting), stop attacking if invalid (then `clearAnger` of that target and the dig cooldown), look at the target within follow range, walk to it at 1.2, `SonicBoom`, `MeleeAttack(18)`.
- **Melee** (`doHurtTarget`): entity event 4 (the attack animation), `entity.warden.attack_impact` at 10.0 and its voice pitch, sonic boom cooldown set to 40, then the monster hit: damage 30 (`when_caused_by_living_non_player` scaling, so 16 on Easy, 30 on Normal, 45 on Hard), knockback 1.5, and a shield is disabled for 5 s (`getSecondsToDisableBlocking`).
- **Sonic boom** (`SonicBoom`): starts when the target is within 15 across and 20 up or down and no `SONIC_BOOM_COOLDOWN`; `canStillUse` is always true (**resolved**), so it runs its full 60 ticks; start: `ATTACK_COOLING_DOWN` 60, entity event 62 (the boom animation), `entity.warden.sonic_charge` at 3.0; at tick 34 (`ceil(34.0)`, `SONIC_BOOM_SOUND_DELAY`), if the target is still within 15/20 and in its line of sight: particles `sonic_boom` along the line from its chest (position + `WARDEN_CHEST` turned by its yaw) to the target's eyes, one per block for `floor(length) + 7` steps, `entity.warden.sonic_boom` at 3.0, and `damageSources().sonicBoom(warden)` for 10. Damage type `sonic_boom`: scaling `always` (6 on Easy, 10 on Normal, 15 on Hard), in `#bypasses_armor`, `#bypasses_enchantments` and, through `#bypasses_armor`, `#bypasses_shield`. If it hurt: `push(dir.x × 2.5 × (1 - kr), dir.y × 0.5 × (1 - kr), dir.z × 2.5 × (1 - kr))`. Stop: cooldown 40.
- **Hurt** (`hurtServer`, javap this round): when hurt by a living source (not no-AI, not digging or emerging): anger `AngerLevel.ANGRY.getMinimumAnger() + 20` = 100 (`ON_HURT_ANGER_BOOST` 20; round 9's "+20" was wrong) at the attacker, and with no attack target yet, a direct hit (`DamageSource.isDirect`, so not an arrow) or an attacker within 5 blocks becomes the attack target at once through `setAttackTarget` (so the boom waits 200 ticks).
- **Darkness pulses** (`customServerAiStep`): when `(tickCount + id) % 120 == 0`, `applyDarknessAround(level, position, warden, 20)`: Darkness 260 ticks, amplifier 0, not ambient, no particles, to every player in survival or adventure (`ServerPlayerGameMode.isSurvival`) within 20 blocks, not allied, that has no darkness, or a lower amplifier, or darkness ending within 199 ticks (`addEffectToPlayersAround`, display limit 200). A player who stays near therefore stays dark: each pulse finds 140 ticks left and renews it to 260; after leaving, it lasts up to 13 s.
- Death message `death.attack.sonic_boom` and `.player` (read the text from the language file).

### Plan
1. **"Minecraft wardens roar, sniff and hit hard enough to knock shields down"**: the roar, sniffing and fight behaviours in the warden brain; `set_attack_target` with its 200-tick sonic boom cooldown (used by the roar's end and the hurt rule); melee with `PlayerHitKind::Melee { disable_shield_seconds: 5.0, .. }`, the attack sound and entity event 4 (kept for W6), sonic cooldown on a hit; the hurt rule (+100, direct or within 5); MW2 bullets hurt it like arrows (indirect: the shooter becomes the target only within 5 blocks, but the anger makes it roar and come); listening and ambient sounds by anger level (`AngerLevel`: calm ambient and listening, agitated, angry).
2. **"Minecraft wardens fire sonic booms and spread darkness"**
   - `PlayerHitKind::SonicBoom { push }` in vendored `world.rs:607`; `entities.rs:1882` applies 10 scaled by difficulty with no armour, no protection and no shield, then the push; `death.rs:67` `MobAttack::SonicBoom` with the vanilla line; mobs it targets take the same damage and push.
   - The beam: an `EntitySnapshot` burst list (like W2's ripples) of sonic boom rings at each step, drawn by the host and friends with the 26.3 `sonic_boom` particle sprites.
   - The warden's darkness pulse goes through `Output.player_effects` (added by W7's shrieker commit, which runs before this group): darkness 260 ticks to players within 20 every 120 ticks, `display_limit` 200.

### Tests
- `warden_brain::tests::an_angry_warden_roars_eighty_four_ticks_then_attacks` and `sniffing_angers_it_at_the_nearest_within_six`.
- `world::wardens::tests::its_melee_hits_thirty_on_normal_and_disables_a_shield_for_five_seconds` (16 Easy, 45 Hard).
- `world::wardens::tests::a_sonic_boom_lands_at_tick_34_through_a_shield_and_armour_and_pushes` (15/20 range, line of sight, 60-tick run, cooldown 40, a melee hit sets the cooldown to 40 even when more was left).
- `world::wardens::tests::the_first_boom_waits_ten_seconds_after_the_roar` (200 ticks from `set_attack_target`, also after a hit within 5 blocks).
- `world::wardens::tests::a_hit_angers_it_by_a_hundred`.
- `server::tests::a_warden_darkens_players_within_twenty_every_six_seconds` (survival yes, creative no, 21 blocks no, an existing long darkness kept).
- `entities::tests::a_sonic_boom_ignores_armour_protection_and_shields` and `death::tests::a_sonic_boom_kill_has_the_vanilla_line`.

### Owner's steps (survival, Normal, flat open ground at dusk; full diamond armour with a Protection IV chestplate, the shield in the offhand, golden apples in the hotbar)
1. Hatch a warden 12 blocks away and hit it with an arrow: it roars (a long, deep roar, about 4 s) and comes at you. **w14_warden_roar**
2. Block with the shield as it swings: the hit lands on the shield, the shield goes down for 5 s. A hit without the shield takes about 6 of your 10 hearts even in diamond armour. **w15_warden_shield**
3. Its first boom can come 10 s after the roar, or 2 s after any swing (each swing sets the boom's cooldown to 2 s, also when more was left). Then run 10 blocks away behind a stone wall with a gap: it stops, charges for about 1.7 s, and a ring of shock waves hits you through the gap: 5 hearts through armour and the raised shield, and you fly back. Behind a full wall (no line of sight) the charge ends with no boom. **w16_sonic_boom**
4. Within 20 blocks of it your view goes dark within 6 s and stays dark, pulsing (W1's darkness), while you stay near; walk 25 blocks away and it lasts up to 13 s more. In creative it never darkens you.
5. Put the totem in the offhand: a boom that would kill you pops it. Then take it out and die to a boom: the chat says you were obliterated by a sonically-charged shriek.
6. Shoot it with an MW2 gun from 20 blocks: it roars and comes after you.
7. Friend window: the friend sees the roar, the swings and the boom rings, gets darkness when near it, and a boom aimed at the friend hurts the friend. **w17_friend_warden_fight**

Kit: W4's kit.

---

## W6. The warden is drawn (group `warden-render`, needs D0)

### The owner's finding
The warden needs its model, its glowing layers, the six keyframe animations and its heartbeat for host and friends.

### Cause
- No `warden_render.rs`. The keyframe reader exists (`crates/minecraft_terrain/src/keyframes.rs:761` `AnimationLibrary`) and reads `WardenAnimation` from the jar in a gated test, but no renderer holds a library yet (round 9 track D log, keyframes decisions); the jar path comes from `crates/render_anim/src/minecraft_world.rs:360` `data_jar`.
- The entity pass draws translucent layers at full light (the wither's armour, `crates/minecraft_survival/src/entity_view.rs:1099`).

### Vanilla 26.3
- `WardenRenderer` (**resolved**, javap this round): shadow 0.9; texture `textures/entity/warden/warden.png`; five `LivingEntityEmissiveLayer`s (translucent, full bright, alpha from a function), each drawing its **own baked `WardenModel`** whose mesh keeps only some parts (`WardenModel.createBioluminescentLayer`, `createPulsatingSpotsLayer`, `createTendrilsLayer`, `createHeartLayer`, registered in `LayerDefinitions` as `ModelLayers.WARDEN_BIOLUMINESCENT`, `WARDEN_PULSATING_SPOTS`, `WARDEN_TENDRILS`, `WARDEN_HEART`; javap each `lambda$create…Layer$0` mesh transform for the parts it keeps during the work), in this order: `warden_bioluminescent_layer.png` on the bioluminescent model (alpha 1), `warden_pulsating_spots_1.png` on the spots model (`max(0, cos(age × 0.045) × 0.25)`), `warden_pulsating_spots_2.png` on the same spots model (`max(0, cos(age × 0.045 + π) × 0.25)`), `warden.png` on the tendrils model (alpha = the tendril animation), `warden_heart.png` on the heart model (alpha = the heart animation).
- `WardenModel`: `createBodyLayer` and `setupAnim` (head look, walk, idle tendril and ribcage motion), then the six `WardenAnimation` definitions through the render state's animation states: `WARDEN_EMERGE` 6.68 s, `WARDEN_DIG` 5.0 s, `WARDEN_ROAR` 4.2 s, `WARDEN_SNIFF` 4.16 s, `WARDEN_ATTACK` 0.33333 s, `WARDEN_SONIC_BOOM` 3.0 s (none loops; round 9 keyframes log).
- `Warden.tick` (client): tendril animation from entity event 61 (10 ticks counting down); heart animation 10 each heartbeat; the heartbeat every `getHeartBeatDelay() = 40 - floor(clamp(clientAnger / 80, 0, 1) × 30)` ticks: `entity.warden.heartbeat` at 5.0, its voice pitch, unless silent; emerging and digging draw block particles (**resolved**, `clientDiggingParticles`, javap this round): while the emerge or dig animation is under 4500 ms (`DIGGING_PARTICLES_DURATION` 4.5), every client tick 30 `block` particles (`DIGGING_PARTICLES_AMOUNT`) of the block it stands on (none when that block's render shape is invisible) at `(x ± 0.7, y, z ± 0.7)` (`Mth.randomBetween(-0.7, 0.7)`, `DIGGING_PARTICLES_OFFSET`) with zero speed. Entity events: 4 attack, 61 tendrils, 62 sonic boom; pose changes start emerge, dig, roar and sniff.

### Plan
1. **"Minecraft wardens are drawn like vanilla"**: new `crates/minecraft_terrain/src/warden_render.rs` ported from `WardenModel` on `posed_cube`, the five layers in the translucent pass at full light, shadow 0.9, the three textures plus layers in `mesh.rs`, keyframes from one `AnimationLibrary::new(data_jar())` held by the client for host and friend; spawner cages; the D0 render registry line. `MobShow` (or D0's per-family show) carries the pose with its start tick, the attack and sonic boom start ticks, the tendril ticks and the client anger.
2. **"Minecraft wardens' hearts beat and their digging throws up the ground"**: each client (host and friend) plays the heartbeat from the shown anger and draws the heart layer; the tendril clicks sound with the tendrils; emerge and dig particles; the attack and boom animations start from the shown ticks.

### Tests
- `warden_render::tests::a_warden_stands_two_point_nine_blocks_with_its_parts` (part and face count against the javap'd model), `its_spots_pulse_in_opposite_phase`, `emerging_starts_the_emerge_animation_and_poses_its_body_underground_at_the_start`, `the_heart_glows_on_each_beat`.
- `mob_tracking::tests::a_friend_sees_a_warden_roar_and_boom_at_the_same_ticks`.
- `warden::tests::the_heartbeat_quickens_with_anger` (40 calm, 25 at 40, 10 at 80 or more).

### Owner's steps (survival, open ground at dusk or night, so the glowing layers show; no night vision, see W0)
1. Hatch a warden: a dark blue-green giant with glowing cyan spots on its chest and head, two horn-like tendrils, slowly pulsing patches, and a heart that glows in its ribcage with each beat you hear. **w18_warden_model**
2. Make noise: the tendrils twitch and glow with a click. It sniffs with its head raised (about 4 s). **w19_warden_sniff**
3. Anger it: the roar animation (chest open, head thrown back); the heartbeat speeds up. Its swing is a fast two-arm slam; the boom bends its body forward with its chest open. **w20_warden_roar_pose**
4. Leave an egg's warden alone for a minute: it sinks into the ground over 5 s with block particles thrown up for the first 4.5 s. A warden summoned by a shrieker (W7's steps 1-4) climbs out of the ground the same way over almost 7 s (`/summon` and eggs never emerge, as in vanilla). **w21_warden_emerge**
5. In creative (`/gamemode creative`), place a spawner and use a warden egg on it: a small warden turns in the cage with its glowing layers. `/gamemode survival` afterwards.
6. Friend window: the friend sees the same poses at the same moments and hears the heartbeat from the warden. **w22_friend_warden_render**

```
IW4L_MINECRAFT_KIT=minecraft:warden_spawn_egg*4,minecraft:cow_spawn_egg*8,minecraft:zombie_spawn_egg*4,minecraft:snowball*16,minecraft:white_wool*64,minecraft:torch*64,minecraft:diamond_sword+sharpness=5,minecraft:shield,minecraft:bow+power=5,minecraft:arrow*64,minecraft:milk_bucket,minecraft:cooked_beef*32,minecraft:diamond_chestplate+protection=4,minecraft:diamond_helmet,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:totem_of_undying,minecraft:golden_apple*8,minecraft:deepslate*64,minecraft:cobblestone*64,minecraft:ender_pearl*16,minecraft:spawner
```

---

## W7. Shriekers and catalysts (group `shriekers-catalysts`, fourth in the order, segment w1; needs W1-W3, not W4)

### The owner's finding
"`/setblock ~ ~ ~3 minecraft:sculk_shrieker[can_summon=true]` with a sensor 2 blocks away. Trigger it 4 times: a shriek, darkness and a distant warden sound each time; on the 4th a warden digs out" and "A sculk catalyst with cows killed 2-5 blocks away: it blooms, no orbs, sculk spreads" (round 9 brief D5 owner steps 4 and 6).

### Cause
- No shrieker or catalyst behaviour; their block entities keep only NBT (`core/src/block_entity.rs:277-288`).
- The player file keeps `warden_spawn_tracker` only as an unknown key (`crates/minecraft_survival/src/player_nbt.rs`, `merging_keeps_unknown_keys...` test at `:1087`); `Player` (`crates/minecraft_survival/src/player.rs:17`) has no tracker. The `spawn_wardens` rule is only stored (`third_party/minecraftoss/world/src/game_rules.rs:164`).
- Mob death experience needs a player kill: `take_deaths`' `earns` (`entities/src/world.rs:4442`) and `drop_death_loot` (`server.rs:1496`) award it straight away; player death experience goes through `minecraft_players.rs:1244` → `AwardExperience` (`server.rs:1538`, `:4083`).
- The sculk spreader exists for world generation only, private in `third_party/minecraftoss/generator/src/feature/kinds/sculk.rs` (constants `:22-25`, `Cursor` `:51`, `update` `:338`; generated shriekers are `can_summon=true`, `:89`).

### Vanilla 26.3
- **Shrieker listener** (`SculkShriekerBlockEntity$VibrationUser`): radius 8, `#shrieker_can_listen` (the tendrils clicking a sensor posts, with the original source), only when not shrieking and the source resolves to a player (`tryGetPlayer`: a player; a vehicle's controlling player; a projectile's owner player; an item entity's owner player). Not ticking adjacent-chunk checks. `SculkShriekerBlock.stepOn`: a player (resolved the same way) standing on it shrieks it, sneaking or not.
- `tryShriek(player)`: nothing while shrieking; `warningLevel` reset to 0; if it `canRespond` (`can_summon`, not peaceful, `spawn_wardens`) it needs `tryToWarn` to succeed, else no shriek; `shriek`: state `shrieking=true` (flags 2), scheduled tick after 90, level event 3007, game event `shriek` (radius 32, wardens hear it).
- Level event 3007 (client): 10 `shriek` particles at (x + 0.5, y + 0.5, z + 0.5) with delays 0, 5, ..., 45 ticks, and unless waterlogged `block.sculk_shrieker.shriek` at 2.0, pitch `0.6 + random × 0.4`.
- **`WardenSpawnTracker`** (per player; ticked by `ServerPlayer.tick` every tick in any dimension; copied by `restoreFrom` on respawn, javap): `ticksSinceLastWarning` counts up and every 12000 ticks the level drops by 1 (and the count resets); `cooldownTicks` counts down. `tryWarn(level, pos, player)`: nothing if any warden is within the 48 × 48 × 48 box around the shrieker; else the players within 16 blocks of it (alive, not spectators) plus the trigger player; nothing if any of them is on cooldown; else the one with the highest level raises it (not on cooldown: level + 1 up to 4, cooldown 200, count reset) and every one of them copies it (level, cooldown and count). Saved as `warden_spawn_tracker: {ticks_since_last_warning, warning_level, cooldown_ticks}`.
- **Response** (`tryRespond`, when the shriek's scheduled tick ends it, or when a shrieking shrieker is removed): if it can respond and the level is above 0: at level 4 `trySummonWarden` (`SpawnUtil.trySpawnMob(WARDEN, TRIGGERED, pos, 20, 5, 6, ON_TOP_OF_COLLIDER)`, W4), else or if that failed the reply sound by level at a random spot within ±10 (level 1 `entity.warden.nearby_close`, 2 `nearby_closer`, 3 `nearby_closest`, 4 `listening_angry`) at 5.0 in HOSTILE; then darkness within 40 (`applyDarknessAround`, W5). Saved `warning_level` and `listener`. Breaking a shrieker drops 5 experience with silk touch off (already in `third_party/minecraftoss/world/src/level/drops.rs:24` and `crates/minecraft_terrain/src/experience.rs:30`, `:133`).
- **Catalyst** (`SculkCatalystBlockEntity$CatalystListener`): radius 8, BY_DISTANCE; on `entity_die` from a living entity whose experience was not consumed: reward = `getExperienceReward(level, killer)` (**no player kill needed**); if `shouldDropExperience` and reward > 0, `addCursors(pos + 0.5 up, reward)`; then `skipDropExperience` (so its orbs never drop and farther catalysts see it consumed); then `bloom`: state `bloom=true`, scheduled tick after 8 (back to false), 2 `sculk_soul` particles at (x + 0.5, y + 1.15, z + 0.5) spread (0.2, 0, 0.2) speed 0, `block.sculk_catalyst.bloom` at 2.0, pitch `0.6 + random × 0.4`. Players count: `ServerPlayer.die` posts `entity_die` too.
- **Level spreader** (`SculkSpreader.createLevelSpreader`): not world-gen, `#sculk_replaceable`, growth spawn cost 10, no-growth radius 4, charge decay 10, additional decay 5; max charge 1000 per cursor, at most 32 cursors; `addCursors` splits a charge above 1000; the catalyst's server tick updates the cursors every tick (`updateCursors(level, pos, random, true)`); charge use posts level event 3006 (sculk charge particles) and sounds; growths placed at runtime are sensors and shriekers with `can_summon=false`. Saved as `cursors` (vanilla `ChargeCursor` codec fields `pos`, `charge`, `decay_delay`, `update_delay`, `facings`, **resolved** with javap this round).

### Plan
1. **"Minecraft sculk shriekers warn players and darken them"**
   - Vendored `world/src/level/sculk.rs`: the shrieker's state, scheduled tick, response hook and level event 3007; `minecraft_terrain/src/vibrations.rs`: its listener and `tryGetPlayer`; `stepOn` for players.
   - New `crates/minecraft_terrain/src/warden_tracker.rs` (`WardenSpawnTracker` with tick, `try_warn`, NBT); `Player.warden_tracker` saved in `player_nbt.rs` (no longer an unknown key), ticked every player tick, kept on respawn; `TickInput` carries each player's tracker and `Output` returns the changed ones (same pattern as omens, `server.rs:3577-3602`); `tryRespond` with the reply sound by level (level 4 plays `listening_angry`, vanilla's fallback when `trySummonWarden` fails, until W4 plugs in `ServerSim::summon_warden`); `spawn_wardens` honoured. The "any warden within the 48-block box" check reads the level's entities by kind (`minecraft:warden`), so it starts working when W4 makes wardens live.
   - New `Output.player_effects: Vec<(u64, EffectGrant)>` (`Output`, `server.rs:3619`) with `EffectGrant { effect, ticks, amplifier, display_limit }`; the survival side applies it per player (host and friends) with `MobEffectUtil.addEffectToPlayersAround`'s rules: survival or adventure only, and only when the player has no such effect, a lower amplifier, or one that `endsWithin(display_limit - 1)`. The shrieker's darkness within 40 uses it (260 ticks, display limit 200, no source, so no ally check); W5's warden reuses it.
   - The client (host and friends) handles level event 3007 (the particles and the shriek sound).
2. **"Minecraft sculk catalysts bloom when mobs die nearby"**
   - The catalyst listener in `vibrations.rs` (BY_DISTANCE, nearest first). `MobDeath` gains the raw reward computed like `experience` without the player-kill check (`world.rs:4442`; javap `LivingEntity.shouldDropExperience` and each `getBaseExperienceReward`), so a cow that dies in lava still feeds it; `drop_death_loot` (`server.rs:1496`) asks the catalysts before awarding orbs. A player's death experience reaches the server marked as a death (`Command::AwardExperience` gains the dead player's id) so a catalyst can take it.
   - Bloom state, sound and the soul particles (an `EntitySnapshot` burst like W2's) for host and friends.
3. **"Minecraft sculk spreads from catalysts like vanilla"**
   - Make the generator's spreader shareable (`sculk.rs`: `Cursor`, `update`, the vein and growth placement behind a small level trait), with the level spreader's constants and runtime growths `can_summon=false`; vendored `world/src/level/sculk.rs` ticks each catalyst's cursors every server tick and saves them in `cursors` (`block_entity.rs:288` keeps the list); level event 3006 particles and the charge sounds on the client.

### Tests
- `warden_tracker::tests`: `each_shriek_raises_the_level_with_a_ten_second_cooldown`, `players_within_sixteen_rise_together_and_one_at_twenty_does_not`, `a_player_on_cooldown_blocks_everyone`, `a_warden_within_the_box_stops_the_warning`, `the_level_drops_one_every_ten_minutes`, `it_round_trips_in_the_player_file`.
- `sculk::tests::a_shriek_lasts_ninety_ticks_and_responds_at_its_end`, `a_shrieker_without_can_summon_shrieks_but_never_warns`, `peaceful_or_spawn_wardens_off_means_no_response`, `a_shrieking_shrieker_broken_still_responds`.
- `server::tests::warnings_one_to_three_play_the_nearby_sounds_and_darken_within_forty` and `the_fourth_warning_plays_the_angry_sound_while_no_warden_can_come` (W4 adds the summon test).
- `server::tests::player_effects_follow_vanilla_s_rules` (survival yes, creative no, a long darkness kept, one ending within 199 ticks renewed).
- `server::tests::a_cow_killed_five_blocks_from_a_catalyst_blooms_it_and_drops_no_orbs`, `ten_blocks_away_it_drops_orbs`, `two_catalysts_the_nearer_takes_it`, `a_cow_dying_in_lava_near_a_catalyst_feeds_it`.
- `sculk::tests::a_catalyst_s_cursors_spread_sculk_and_round_trip`, `a_runtime_shrieker_cannot_summon`.
- `player_nbt::tests::the_warden_spawn_tracker_is_saved_per_player`.

### Owner's steps (survival, Normal; flat open ground at night, `/time set midnight`, with no torches, lava or glowing blocks within 16 blocks, so the summoned warden passes vanilla's monster light check; the moon is enough to see by)
1. `/setblock ~ ~-1 ~3 minecraft:sculk_shrieker[can_summon=true]` (a shrieker placed by hand never summons, as in vanilla). Step on it, then step off: it shrieks with rising soul rings and a scream, your view darkens (W1), and somewhere a warden sound plays. **w23_shriek_warning**
2. Step on it again within 10 s of the shriek: it stays silent (a summoning shrieker does not shriek while your warning is cooling down). Wait 10 s and step on it again: the second warning, the warden sound is closer.
3. Save and quit, load, wait 10 s and step on it: it continues at warning three (closer and angrier sound).
4. Wait 10 s and step on it a fourth time. Before the warden groups (W4) are in: it shrieks and the angriest warden sound plays (no warden can come yet). From W4 on: the ground cracks and a warden digs out within about 5 blocks, roaring. **w24_warden_summoned**
5. (From W4 on.) With that warden still within 24 blocks of the shrieker, wait 10 s and step on it: it stays silent (a warden nearby stops the warning). Walk well away (the warden digs back down after a minute without hearing anything).
6. Place a sculk shrieker from the kit (it cannot summon) 2 blocks from a sculk sensor: walking near the sensor makes this shrieker shriek every time the sensor clicks (the sensor passes your steps on); it never darkens your view or plays warden sounds.
7. `/gamerule spawn_wardens false`, wait 10 s and step on the summoning shrieker: it shrieks, but no warning, darkness or warden sound follows. `/gamerule spawn_wardens true`.
8. Place a sculk catalyst; hatch cows 2-5 blocks away and kill them with the sword: the catalyst flashes, soul wisps rise, no experience orbs drop, and a few blocks of sculk spread over the ground at the death spots (a cow's 1-3 experience is too little to grow anything). **w25_catalyst_bloom**
9. Kill a cow 10 blocks from the catalyst: orbs drop as usual.
10. Drink the 16 experience bottles (about level 7), take the totem out of your hands and `/kill @s` 3 blocks from the catalyst: no orbs drop where you died, and the bigger charge spreads sculk further, now and then growing a sculk sensor or a shrieker (one that cannot summon).
11. Friend window: with the host 5 blocks away, the friend steps on the summoning shrieker: both warning levels rise together (the host's next shriek continues from it) and both get darkness; the friend sees the warden dig out and the catalyst bloom. **w26_friend_shrieker**

```
IW4L_MINECRAFT_KIT=minecraft:sculk_shrieker*8,minecraft:sculk_sensor*16,minecraft:sculk_catalyst*4,minecraft:sculk*64,minecraft:cow_spawn_egg*16,minecraft:chicken_spawn_egg*8,minecraft:diamond_sword+sharpness=5,minecraft:shield,minecraft:deepslate*64,minecraft:cobblestone*64,minecraft:white_wool*32,minecraft:milk_bucket,minecraft:milk_bucket,minecraft:cooked_beef*32,minecraft:diamond_chestplate+protection=4,minecraft:diamond_helmet,minecraft:totem_of_undying,minecraft:ender_pearl*16,minecraft:snowball*16,minecraft:experience_bottle*16
```

---

## Commit groups

| Order | Key | Segment | Commits | Brief |
|---|---|---|---|---|
| 1 | `darkness` | w1 | 1. Minecraft darkness closes in the fog like vanilla. 2. Minecraft darkness pulses the light like vanilla. | W1 |
| 2 | `vibrations` | w1 | 1. Minecraft game events reach their listeners like vanilla. 2. Minecraft sculk sensors hear vibrations and power redstone. 3. Players, mobs, blocks and explosions make Minecraft vibrations. | W2 |
| 3 | `vibrations-more` | w1 | 1. Minecraft blocks make vibrations when used. 2. Minecraft players and mobs make vibrations when they eat, drink, equip, ride, glide, shear, teleport, get hurt and die. 3. Minecraft amethyst resonates sculk vibrations to calibrated sensors. 4. MW2 guns make Minecraft vibrations. | W3 |
| 4 | `shriekers-catalysts` | w1 | 1. Minecraft sculk shriekers warn players and darken them. 2. Minecraft sculk catalysts bloom when mobs die nearby. 3. Minecraft sculk spreads from catalysts like vanilla. | W7 |
| 5 | `warden-live` | w2 (after D0) | 1. Minecraft wardens live in the entity world. 2. Minecraft wardens are saved, hatch from eggs and dig out when shriekers summon them. | W4 |
| 6 | `warden-fight` | w2 | 1. Minecraft wardens roar, sniff and hit hard enough to knock shields down. 2. Minecraft wardens fire sonic booms and spread darkness. | W5 |
| 7 | `warden-render` | w2 | 1. Minecraft wardens are drawn like vanilla. 2. Minecraft wardens' hearts beat and their digging throws up the ground. | W6 |

## Combined kit
One kit for the whole track's test (27 stacks). W1's lava and powder snow steps keep W1's kit; W3's block list keeps W3's kit.

```
IW4L_MINECRAFT_KIT=minecraft:warden_spawn_egg*4,minecraft:sculk_sensor*16,minecraft:calibrated_sculk_sensor*4,minecraft:sculk_shrieker*8,minecraft:sculk_catalyst*4,minecraft:sculk*64,minecraft:white_wool*32,minecraft:white_carpet*16,minecraft:snowball*16,minecraft:redstone_lamp*4,minecraft:comparator*2,minecraft:redstone*32,minecraft:amethyst_block*8,minecraft:cow_spawn_egg*16,minecraft:deepslate*64,minecraft:diamond_sword+sharpness=5,minecraft:shield,minecraft:bow+power=5,minecraft:arrow*64,minecraft:milk_bucket,minecraft:cooked_beef*32,minecraft:diamond_chestplate+protection=4,minecraft:diamond_helmet,minecraft:totem_of_undying,minecraft:oak_door*2,minecraft:experience_bottle*16,minecraft:torch*64
```

## Rules
One commit per item, with its tests. Clippy on the touched crates once per segment, `rustfmt --check` on new files' staged content, no code comments of any kind, cargo only through `scripts\cargo-slot.ps1` (gentle mode when the owner plays). The log is `docs/round10/track-w-log.md`: commits, the javap numbers each group used, decisions, vendored file counts, the protocol number, state doc wording, "Left for later", and each commit's owner steps and kit line (at most 27 stacks, own code block).
