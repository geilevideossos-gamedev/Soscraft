# Track W log: darkness, vibrations, sculk and the warden

Branch `track-w`, worktree `C:\Dev\mc-wt\track-w`, target dir `D:\iw4l-wt\track-w`. Brief: `docs/round10/track-w.md` (W0 holds the shared points).

A commit can't name its own hash, so each entry names the commit before it. Its own hash is in `git log` under its subject.

## Group darkness

### 1. A friend sees an effect given again with its new length

After 382bf959.

- **Cause:**
  - `Session::send_view` (`crates/minecraft_survival/src/session.rs`) sends a friend's `PlayerView` only when its key changes, and `views::sent_key` zeroes every effect's duration so a running effect is not sent every tick. An effect given again with a longer duration (`/effect give` twice, a conduit's refresh, and later the warden's and the shrieker's darkness, which vanilla gives again with 260 ticks once fewer than 200 are left) changed nothing in the key, so the friend was never told.
  - The friend's HUD (`views::HudClock::effects`) counts an effect down from the last view and restarts only when the view's effects differ from the ones it saw. A warden's second pulse gives 260 ticks again, so the view sent for it is the same as the first one, and the friend kept counting down from the first grant: the icon left early while the effect still ran on the host.
- **Vanilla:** every `MobEffectInstance.update` that changes an effect sends `ClientboundUpdateMobEffectPacket` with the new duration (`ServerPlayer.onEffectUpdated`), and the client takes that duration as it is.
- **Changed:**
  - `views::lengthened(before, now)`: an effect of the same kind whose duration went up.
  - `Session` keeps the effects it saw at the last `send_view` call (`effects`) and sends the view again when one was lengthened, even when the key is the same. Durations only go down between calls (one call a frame), so a rise is always a new grant.
  - `RemoteWorld.player_views` counts the player views the friend received, and `HudClock::effects(view, received)` restarts its count-down for every new view, also one equal to the last.
- **Tests:**
  - `session::tests::an_effect_given_again_is_sent_again_with_its_new_length` (darkness 260, 61 ticks later given 260 again: sent again; a shorter grant that `add_effect` refuses is not sent).
  - `views::tests::a_friend_s_effect_given_again_with_the_same_length_counts_down_from_the_start` (the same view received again shows 260 again, not the 200 left of the first).
  - `remote::tests::a_player_view_sent_again_counts_as_a_new_one`.
- **Decisions:** no protocol change: the existing `ToClient::Player` is sent once more, nothing in the message or codec changed. The friend tells a new view from an old one by counting arrivals, because two views can be equal.
- **Test steps:** with the next two commits (step 7 of commit 3).

### 2. Minecraft darkness closes in the fog like vanilla

After the commit above (its subject: "A friend sees an effect given again with its new length").

- **Vanilla (javap and the 26.3 jar):**
  - `MobEffectInstance$BlendState.tick`: the previous factor takes the current one, the target is 1 while `!endsWithin(blendOutAdvance)` (22 for darkness: infinite, or more than 22 ticks left) and 0 otherwise, and the factor moves towards it by at most `1 / 22` a tick. `tickClient` counts the duration down first and then ticks the blend. `getFactor(partial)` lerps previous to current. The state lives in the effect instance, so milk (which removes the instance) ends the look at once and a fresh effect starts from 0, while an effect given again keeps its blend.
  - `DarknessFogEnvironment.setupFog`: `d = lerp(blend, renderDistanceBlocks, 15)`, environmental start `0.75 d`, environmental, sky and cloud end `d`. It is applicable whenever the camera entity has darkness (`MobEffectFogEnvironment.isApplicable`), so it replaces the atmospheric fog for the whole effect, even at blend 0. `FogRenderer` runs only the first applicable environment, in the order lava, powder snow, blindness, darkness, water, atmospheric.
  - `FogRenderer.computeFogColor`: the void darkness becomes `getModifiedDarkness = max(blend, void)` from the first applicable environment that `modifiesDarkness` (darkness does, `MobEffectFogEnvironment`); when it is above 0 (and the camera is not in lava or powder snow, whose fog colours the fork sets anyway) the fog colour is multiplied by `(1 - value)²`. Darkness `providesColor` false: the base colour stays the atmospheric one.
- **Changed:**
  - New `crates/minecraft_terrain/src/darkness.rs`: `DarknessBlend` (vanilla's blend state), `holds(remaining)` (`!endsWithin(22)`), `Darkness` (one viewer's look: the blend while the effect is on, ticked on its own 20 Hz clock from the frame time, counting the given duration down between updates the way the vanilla client does), `fog` (the darkness fog), `camera_fog` (lava, then powder snow, then darkness, else the atmospheric sky) and `fog_darkening(void, blend)`.
  - `Darkness::advance(seconds, duration, view)` restarts its count-down when the duration or the view it came in changes. The host passes its live duration and view 0; the friend passes the duration of its last `PlayerView` and `RemoteWorld.player_views`, so a warden's second pulse (the same 260 ticks in a view equal to the first) restarts it.
  - `environment.rs`: `View.darkness` (the blend), and the void darkening of the fog colour takes `fog_darkening(void, blend)`.
  - `render_anim/src/minecraft_world.rs`: `SkyInputs.darkness` carries the viewer's `Darkness`; `present_sky` gives its blend to the sky's colour and its fog to `camera_fog`, which replaces the lava and powder snow match. The host keeps one `Darkness` in its `Runtime` (reset when a world loads, kept across dimension switches like the effect), advanced every frame from `survival.status.effect(EffectKind::Darkness)`.
  - `render_anim/src/minecraft_remote.rs`: the friend keeps its own `Darkness` in `Friend` and advances it from the darkness `EffectView` of its last `PlayerView` (`darkness_ticks`). Nothing new is sent: each window draws its own player's darkness.
- **Tests:**
  - `darkness::tests::the_blend_ramps_over_22_ticks_and_fades_before_the_end` (0 to 1 in 22 ticks, the partial lerps, an effect with 30 ticks left fades from its 8th tick and is at 0 when it ends, an infinite one stays at 1).
  - `darkness::tests::the_viewer_counts_the_duration_down_between_updates_and_milk_ends_it_at_once` (a viewer given the duration once matches one given the live count-down every tick).
  - `darkness::tests::the_same_length_given_again_in_a_new_view_counts_down_from_the_start`.
  - `darkness::tests::fog_closes_to_fifteen_blocks` (blend 1: start 11.25, end, sky and cloud end 15; blend 0: the render distance; 0.5 halfway; lava and powder snow keep their own fog).
  - `darkness::tests::fog_colour_darkens_by_the_square` (0.5 multiplies by 0.25, the void wins when larger).
  - `environment::tests::darkness_darkens_the_fog_colour_by_the_square_of_its_blend` (real overworld noon sky).
  - `minecraft_world::darkness_tests::darkness_reaches_the_environment_block` (`present_sky` on a real overworld level: fog start 11.25 and end 15, black fog, sky fog end 15; lava keeps its 0.25 to 1 fog).
  - `minecraft_remote::tests::a_friend_with_darkness_gets_the_same_fog` (a friend fed the host's `PlayerView` the way `send_view` sends it, with a second grant of the same 260 ticks 61 ticks in, gets the same blend and fog as the host every tick until the effect ends; milk clears it at once).
- **Decisions:**
  - `SkyInputs.darkness` is the viewer's whole `Darkness` state (small and `Copy`) rather than only the blend, so the pulse of the next commit reads its tick count from the same state.
  - The blend has its own 20 Hz clock instead of the sky's environment tick, which runs at most once a frame and restarts on a dimension switch; vanilla's blend runs with the player's client tick.
  - The darkness fog replaces the atmospheric fog for as long as the effect is on, also at blend 0 (vanilla's applicability rule). The cloud alpha and the skybox stay as they are: darkness does not touch them in vanilla.
- **Test steps:** see commit 3 (both commits are tested together).

### 3. Minecraft darkness pulses the light like vanilla

After the commit above (its subject: "Minecraft darkness closes in the fog like vanilla").

- **Vanilla (javap and the 26.3 jar):**
  - `LightmapRenderStateExtractor.extract`: `blend = getEffectBlendFactor(DARKNESS, partial) × darknessEffectScale` (the option, 1.0 by default), `brightness = max(0, gamma - blend)` (gamma 0.5 by default), and `darknessEffectScale` of the render state is `calculateDarknessScale(player, blend, partial) × darknessEffectScale`.
  - `calculateDarknessScale`: `max(0, cos((tickCount - partial) × π × 0.025) × 0.45 × blend)`: a pulse of 80 ticks (4 s), dim for half of it, at most 0.45.
  - `lightmap.fsh`: `color = color - vec3(DarknessScale)` after the boss overlay darkening and before `clamp(color, 0, 1)` and the `notGamma` blend by `BrightnessFactor`.
- **Changed:**
  - `darkness.rs`: `light(blend, tick, partial) -> (brightness, scale)` with vanilla's formulas, and `Darkness::light()`, which takes the viewer's own tick count (`Darkness.ticks`, one a tick of its 20 Hz clock, with or without the effect) for vanilla's `tickCount`.
  - `render_anim/src/minecraft_world.rs` `present_sky`: the brightness lane (`environment[10].w`, the terrain's `environment.cloud.w`) is `max(0, 0.5 - blend)` instead of a fixed 0.5, and the scale goes into `environment[0].w` (`forward.w`), which no shader read before. Not `environment[3].w`: the model pass overwrites that one with the eye's sky light (`render_gpu/src/drawsurf/minecraft_light.rs`, `light.environment[3][3]`) and reads it as the light of a pixel before the light volume arrives. The host and the friend both reach it through `SkyInputs.darkness`, so each window pulses with its own player's darkness and its own tick count.
  - `render_gpu/src/drawsurf/minecraft_world.rs` (terrain, block entities, particles and weather, which share `lightmap`) and `minecraft_light.rs` (`mc_lightmap`, for MW2 models, weapons and the view model in a Minecraft world) subtract the scale before the clamp. The model lightmap keeps its brightness mix of 0, so only the pulse reaches it, as before.
- **Tests:**
  - `darkness::tests::the_light_pulses_every_four_seconds` (scale 0.45 at tick 0 and 80, 0.318 at tick 10, 0 at 20 and 40, half at blend 0.5, the partial runs it back; brightness 0.5 without darkness, 0.25 at blend 0.25, 0 from blend 0.5; a viewer at full darkness is dimmed on 39 of 80 ticks; milk ends the pulse at once).
  - `minecraft_world::darkness_tests::darkness_reaches_the_environment_block` now also checks the lanes: no pulse and brightness 0.5 without darkness, the scale 0.45 in `environment[0].w` and brightness 0 with it.
  - `render_gpu`: `minecraft_world::tests::darkness_dims_the_terrain_lightmap_before_its_clamp` and `minecraft_light::tests::darkness_dims_models_from_a_lane_the_eye_light_leaves_alone` (the hook shader validates, subtracts `environment[0].w` before its clamp, still reads the eye light from `environment[3].w`, and the Rust side writes only `light.environment[3][3]`).
- **Decisions:**
  - The darkness effect scale option stays at its default 1.0 and gamma at the fork's fixed 0.5: the fork has no video settings for either.
  - The tick count is taken modulo 80 (the pulse's period) before it becomes a float, so the pulse keeps its shape in a long session; vanilla's `tickCount` is an int turned into a float and loses that precision after days of play.
  - The pulse's phase is each viewer's own, as in vanilla, where every client has its own `tickCount`: the host's and a friend's pulses need not line up.
  - The sky, the clouds and the fog colour are not lightmapped in vanilla, so the pulse leaves them alone.
- **Test steps (survival, by day, open ground; commits 1 to 3):**
  1. `/effect give @s minecraft:darkness 30`: within about a second the far land fades into black fog about 15 blocks away, and the light around you dims and brightens in a slow pulse (about 2 s dim, 2 s normal). **w01_darkness_fog**
  2. Stand in a lit cave, or at night by the torches of the kit (place a few on cobblestone): the pulse makes torch-lit blocks go almost black and come back. **w02_darkness_pulse**
  3. Drink a milk bucket: the view snaps back at once and the icon is gone.
  4. `/effect give @s minecraft:darkness 3`: the darkness fades in and, in the last second, fades out smoothly.
  5. `/effect give @s minecraft:fire_resistance 60`, then `/effect give @s minecraft:darkness 30` and stand in lava (place the lava bucket in a cobblestone pit), and in powder snow (place the powder snow bucket): their own orange and white fog stays as before, not the black one.
  6. Friend window: the friend runs step 1 on themselves (or the host runs `/effect give <friend name> minecraft:darkness 30`): the friend's view goes dark and pulses, the host's does not. Then the host gives itself darkness: only the host's view changes. **w03_friend_darkness**
  7. Friend window: the host runs `/effect give <friend name> minecraft:darkness 13`, waits about 4 seconds and runs it again. The friend's view stays dark until about 13 seconds after the second command (about 17 s in all), and its darkness icon stays as long; before commit 1 the friend's darkness ended about 13 s after the first command.

```
IW4L_MINECRAFT_KIT=minecraft:milk_bucket,minecraft:milk_bucket,minecraft:milk_bucket,minecraft:lava_bucket,minecraft:powder_snow_bucket,minecraft:torch*64,minecraft:cobblestone*64,minecraft:glass*16,minecraft:cooked_beef*32
```

## Group vibrations

### 1. Minecraft game events reach their listeners like vanilla

After 4b09009f.

- **Vanilla (javap on the 26.3 jar, and its data pack):**
  - `GameEvent.<clinit>`: every event has radius 16 but `jukebox_play` and `jukebox_stop_play` (10) and `shriek` (32). `VibrationSystem.VIBRATION_FREQUENCY_FOR_EVENT` gives the frequencies of the brief (W2), `resonate_N` is N, everything else 0.
  - Tags read from the data pack at load: game event `#vibrations`, `#warden_can_listen`, `#shrieker_can_listen`, `#ignore_vibrations_sneaking`, item `#dampens_vibrations`, block `#dampens_vibrations` and `#occludes_vibration_signals` (26.3 adds wool slabs and stairs to the dampening tags).
  - `GameEventDispatcher.post`: the sections from `(x - r) >> 4` to `(x + r) >> 4` (r the event's radius) of loaded chunks, chunk x, then chunk z, then section y; a listener is in range when the squared distance between the block positions of event and listener is at most its own radius squared (`EuclideanGameEventListenerRegistry.getPostableListenerPosition`); `UNSPECIFIED` listeners handle it at once, `BY_DISTANCE` ones are sorted by distance (a stable sort) and handled after.
  - `VibrationSystem$Listener.handleGameEvent`: nothing while a vibration travels; `isValidVibration` (the listenable tag; a spectator source never; a source stepping carefully with an event in `#ignore_vibrations_sneaking` never; a dampening source never; an affected state in block `#dampens_vibrations` never); `canReceiveVibration`; occlusion; then `scheduleVibration` adds `VibrationInfo(event, Vec3.distanceTo, pos, source)` to the selector at the game time.
  - `isOccluded`: both ends moved to their block centres, then one `isBlockInLine` per `Direction.values()` (the start nudged `9.999999747378752e-6` along it); occluded only when every ray meets an `#occludes_vibration_signals` block. `traverseBlocks` tests the start block too.
  - `VibrationSelector.shouldReplaceVibration` (**the brief was wrong here**): a candidate from another tick than the kept one never replaces it; in the same tick the nearer replaces, the farther does not, and at equal distance the higher frequency does. `chosenCandidate(now)` only hands over a candidate from before `now`.
  - `VibrationSystem$Ticker.tick`: with no current vibration, the chosen candidate becomes it with `floor(distance)` travel ticks, the particle is sent and the selector starts over (`onDataChanged`); then the reloaded particle (after a load), the travel time counts down (`max(0, t - 1)`), and at 0 (sensors only with the 3 × 3 chunks around ticking) `onReceiveVibration(event pos, event, entity, projectile owner, distanceBetweenInBlocks)` and the vibration clears.
  - Saved as `listener: {event?: {game_event, distance, pos, source?, projectile_owner?}, selector: {event?, tick}, event_delay}`; a loaded `Data` reloads its particle.
- **Changed:**
  - New `crates/minecraft_terrain/src/game_events.rs`: the event table, `Source` (player with its sneaking and spectator state, mob, item entity with its dampening, other entity), `Posted`, `PlayerEvent` (what a client tick hands the server), `EventTags` loaded from the data pack and the block tags, and `is_valid`.
  - New `crates/minecraft_terrain/src/vibrations.rs`: listener kinds (sensor, calibrated sensor, shrieker, catalyst) by block entity id, `Info`, `Selector`, `Listener` with its NBT round trip, `Vibrations` (the listeners by position and by chunk section, `post`, `force` for `forceScheduleVibration`, `tick`), the `User` trait the level side answers, `occluded` with vanilla's block traversal and `redstone_strength`.
  - New vendored `world/src/level/game_event.rs`: `Level::game_event`, `block_game_event`, `game_event_from` and `force_game_event` queue `GameEvent`s stamped with the game time; `Level.acting_player` is the source of `game_event`. `level/mod.rs` gains the two fields.
  - New vendored `entities/src/world/game_events.rs`: `EntityWorld::game_event` and `take_game_events`.
  - `TickInput.game_events` (from each player's `PlayerQueue.game_events`, filled by `Entities::game_event`): a tick input replacing a running batch keeps the older ones first (`carry_game_events`), `remove_player` drops a leaving player's, and `run_tick` takes them once.
  - Commands with a player: `UseBlock`, `RingBell`, `BoneMeal`, `Attack` and `PlayerMines` gain `player: u64`; `PlayerEdit`, `PlayerBreaks` and `ChestOpeners` gain `player: Option<u64>`. The server loop sets `Level.acting_player` around every command that reaches the level (`ServerSim::acting`), `ItemOnBlock` included. The survival side and render_anim pass the acting player: `Body::write_blocks` of an MW2 body passes its client, `level::write_blocks` takes the player, `level::leave` takes the leaving player's id, menus pass the player whose screen opened or closed a trapped chest and `None` on the openers' recheck (vanilla's `recheckOpeners` posts with no source).
  - `ServerSim` keeps a `Vibrations` index (block entities of loaded chunks, kept up to date from the level's block log in `sync_pois`, chunks unloaded take theirs), and `run_tick` dispatches every game event of the tick after the mobs (`hear_game_events`): the players' (stamped with the game time before the level tick, as vanilla handles their packets between ticks), the level's, the entity world's; then, on a normal tick, the listeners tick. A player source takes its sneaking flag from the tick's controls and its spectator flag from `mob_players`; its UUID is the entity world's.
  - Until the next commit a sensor accepts every frequency other than 0 except its own placing or breaking, and shriekers and catalysts accept nothing.
- **Tests:**
  - `game_events::tests::frequencies_and_radii_match_vanilla` (the full table), `the_tags_load_from_the_data_pack` (gated on the 26.3 data), `sneaking_spectating_and_dampening_sources_are_not_valid`.
  - `vibrations::tests::a_step_five_blocks_away_reaches_a_sensor_five_ticks_later_with_power_six`, `a_sneaking_step_is_not_heard_but_a_sneaking_block_place_is`, `a_wool_shell_occludes_and_one_open_side_does_not`, `stepping_on_a_carpet_is_dampened`, `the_nearest_candidate_wins_and_a_tie_goes_to_the_higher_frequency` (also: the older candidate is kept), `nothing_new_is_heard_while_a_vibration_travels`, `a_listener_with_a_travelling_vibration_round_trips_its_nbt` (vanilla field names), `a_catalyst_hears_by_distance_after_the_others`, `listeners_leave_with_their_chunk_and_their_block`.
  - `server::tests::a_player_s_step_reaches_a_sensor_s_listener_through_the_tick` (stamped before the tick, its UUID, saved in the block entity, received after `floor(distance)` ticks, sneaking and out of reach), `other_dimensions_never_hear` (each `ServerSim` has its own listeners), `game_events_wait_in_a_replaced_tick_input_and_leave_with_their_player`, `a_broken_sensor_stops_listening_and_its_chunk_takes_it_away`.
- **Decisions:**
  - Game events are dispatched once a tick, after the mobs, instead of at the moment they are posted. Each keeps the game time it was posted in, so the selector sees what vanilla sees: nothing posted in a tick is picked before the next.
  - `PlayerEdit` and `PlayerBreaks` take an `Option<u64>`, not a `u64`: every block the client writes goes through them, also writes no player made (container ticks, frost walker's ice, anchors used up on respawn), and those carry `None`. The block a player places posts its own `block_place` where the player places it (commit 3).
  - The listener NBT is written to the block entity when a vibration is picked and when it arrives, not on every tick of its travel: every write is a block entity change sent to the client and the friends. A save while a vibration travels keeps its whole travel time, so after a load it arrives up to 16 ticks later than in vanilla.
  - Unlike vanilla, which retries until a player is near enough to receive it, a reloaded vibration's particle is sent once.
  - `Posted.dampened` holds "the affected state is in block `#dampens_vibrations`", worked out where the event is posted, instead of the state itself.
  - The ticker runs for a listener in a loaded chunk of the level's simulation area (`Level.ticking_chunks`), and a sensor receives only when the 3 × 3 chunks around it are loaded and in it.
- **Vendored files:** added `world/src/level/game_event.rs` and `entities/src/world/game_events.rs`; patched `world/src/level/mod.rs` and `entities/src/world.rs`.
- **Test steps:** with commit 3 (the first sources).

### 2. Minecraft sculk sensors hear vibrations and power redstone

After 21c929e1 ("Minecraft game events reach their listeners like vanilla").

- **Vanilla (javap on the 26.3 jar):**
  - `SculkSensorBlock.activate(entity, level, pos, state, power, frequency)`: phase `active` with that power (`setBlockAndUpdate`), a scheduled tick after `getActiveTicks` (30, calibrated 10), `updateNeighbours` (the sensor and the block under it), `tryResonateVibration` (W3), the game event `sculk_sensor_tendrils_clicking` with the vibration's own source entity, and, unless waterlogged, `block.sculk_sensor.clicking` at the centre (volume 1, pitch `0.8 + random × 0.2`).
  - `SculkSensorBlock.tick`: `active` → `deactivate` (`cooldown`, power 0, a tick after 10, neighbours); `cooldown` → `inactive` and, unless waterlogged, `clicking_stop`. `onPlace`: another block replaced by a powered sensor with no tick waiting loses its power (flags 18). `affectNeighborsAfterRemoval`: an active one updates its neighbours. `ownSignal` is the power, `getDirectSignal` only upwards, and the comparator reads `last_vibration_frequency` while `active`, else 0.
  - `SculkSensorBlockEntity$VibrationUser`: radius 8; `canReceiveVibration`: not its own `block_place` or `block_destroy` (the event's block is the sensor's), a frequency other than 0, and `canActivate` (`inactive`); `onReceiveVibration`: `last_vibration_frequency` = the frequency, power `getRedstoneStrengthForDistance(distance, radius) = max(1, 15 - floor(15 / radius × distance))`, then `activate`; `requiresAdjacentChunksToBeTicking`.
  - `CalibratedSculkSensorBlock`: `getActiveTicks` 10, `getSignal` 0 towards its `facing` (to the block it reads); its user: radius 16, refuses a frequency other than `getBackSignal` (`level.getSignal(pos.relative(facing.opposite), facing.opposite)`) when that is not 0.
  - `SculkSensorBlock.stepOn` (`LivingEntity.aiStep` → `applyEffectsFromBlocks` runs it every tick an entity stands on it, sneaking or not; not a warden): when `inactive` and its user `canReceiveVibration(step, Context.of(state))`, `forceScheduleVibration(step, Context.of(entity), entity.position())`, which skips the tag, sneak, dampening and occlusion checks.
  - `VibrationSignalParticle` (client): lives for the travel ticks; every tick it moves `1 / ticks left` of the way to the listener, so it arrives on its last tick; two quads of size 0.3 turned along its path (`rotationY(rot)·rotateX(-(pitch + π/2))·rotateY(w)` and `rotationY(rot - π)·rotateX(pitch + π/2)·rotateY(w)`, `w = sin((age + partial - 2π) × 0.05) × 2`), block light 15, translucent, sprite `particle/vibration`. `ServerLevel.sendParticles` sends it to the players within 32 blocks.
- **Changed:**
  - New vendored `world/src/level/sculk.rs`: `sculk_can_activate`, `sculk_back_signal`, `sculk_receive` (frequency into the block entity, then `activate` with the original source), the phases on the scheduled ticks with their sounds, `sculk_on_place`, `sculk_after_removal`, `sculk_signal` (calibrated: nothing towards its back) and `sculk_comparator`.
  - `world/src/level/redstone.rs`: `Kind::SculkSensor` (`SculkSensorBlock`, which the calibrated sensor extends) in own signal, signal, direct signal (upwards), analog output, `onPlace`, removal and the scheduled tick.
  - `world/src/level/entity.rs` `step_on`: an item or other level entity on an `inactive` sensor forces a `step` (`Level::force_game_event`); sensors are no longer reported as unsupported (shriekers still are, until W7).
  - `server.rs` `Hearing`: a sensor receives with the checks above and activates through `Level::sculk_receive`; the ripple goes into `EntitySnapshot.vibrations` (`VibrationView { from, to, ticks }`), sent again after a load. Players (alive, not spectators) and mobs (`EntityWorld::grounded`) whose feet are on a sensor's top (`floor(y - 0.2)` is the sensor and the feet within 0.05 of its 0.5 high top) force a `step` every tick.
  - New `crates/minecraft_terrain/src/vibration_particles.rs` (the client particle and its mesh), drawn by the host and friends through `EntityView` (bursts, client ticks, translucent mesh; the sprite joins the atlas in `mesh.rs`). `mob_sender` keeps the ripples with the other one-shot bursts and sends a friend the ones that start within 32 blocks of it in chunks it has.
- **Tests:**
  - `level::sculk::tests` (vendored world, gated on the 26.3 data): `phases_go_active_thirty_cooldown_ten_inactive` (sounds, the tendrils event with its source, nothing new while active), `a_calibrated_sensor_is_active_ten_and_filters_by_its_back_input` (only the back face, no signal back into it), `the_comparator_reads_the_last_frequency_while_active` (and the power lights a lamp), `a_powered_sensor_placed_without_a_tick_loses_its_power`.
  - `server::tests::a_redstone_lamp_lights_when_a_player_walks_near_a_sensor` (through `run_tick`: the ripple, power 8 four blocks away, frequency 1 saved, 30 active and 10 cooling, a second player id six blocks away gives power 4), `a_sensor_does_not_hear_its_own_placing_or_breaking`, `standing_on_a_sensor_activates_it_even_crouched` (power 15; a player above it does not), `an_item_or_a_cow_on_a_sensor_activates_it`, `a_calibrated_sensor_hears_only_the_frequency_coming_in_from_behind` (a back input of 15: steps no, an explosion 12 blocks away yes).
  - `vibration_particles::tests::a_ripple_reaches_its_listener_on_its_last_tick_and_goes`, `a_ripple_turns_along_its_path`.
  - `mob_sender::tests::a_vibration_burst_reaches_friends_near_it`.
- **Decisions:**
  - The server tells a player standing on a sensor from its feet: the block `0.2` under them is a sensor and the feet are within 0.05 of its top. The fork's players send no ground contact to the server, and MW2's hull can rest a hair above a surface.
  - Shriekers (W7) and catalysts still receive nothing.
  - A vibration loaded from a save knows only its source's UUID, so the sensor's tendrils event goes out without a source; nothing reads it before W7's shriekers, which only answer players.
  - A listener loaded with no vibration in flight does not mark its particle for reloading (vanilla's `Data` codec always does, so its first ripple after a load is sent twice, at the same place). The block entities of sensors placed in the fork come from the catalog's default tag, so this also keeps fresh sensors from doubling their first ripple. This changes commit 1's `Listener::from_tag`.
  - Commit 1's `a_player_s_step_reaches_a_sensor_s_listener_through_the_tick` now waits 40 ticks before its last step: from this commit on the first step activates the sensor, which then hears nothing for 40 ticks.
- **Vendored files:** added `world/src/level/sculk.rs`; patched `world/src/level/redstone.rs` and `world/src/level/entity.rs` (and `mod.rs` again, `entities/src/world/game_events.rs` again).
- **Test steps:** with commit 3.

### 3. Players, mobs, blocks and explosions make Minecraft vibrations

After 6dcdeb25 ("Minecraft sculk sensors hear vibrations and power redstone").

- **Vanilla (javap on the 26.3 jar):**
  - `Entity.applyMovementEmissionAndPlaySound` and `vibrationAndSoundEffectsFromBlock`: a step that sounds from the block at the feet posts `step` with that block as the affected state when it is also the supporting block; otherwise the supporting block posts it (no sound); a move over the next step in water that made no step posts `swim`. `doWaterSplashEffect` posts `splash`. `checkFallDamage`: landing with a fall distance above 0 posts `hit_ground` at the position, the affected state the main supporting block.
  - `BlockItem.place` posts `block_place` at the placed block (source the player, affected the placed state); `Block.playerWillDestroy` posts `block_destroy` (affected the old state); blocks an explosion removes post nothing of their own.
  - `Projectile.tick`: on its first tick `projectile_shoot` at the projectile with the owner as source. `Projectile.onHit`: `projectile_land` from the projectile (its owner travels with it as the projectile owner) at the hit location for an entity, or at the centre of the struck block with that block's state for a block.
  - `ServerExplosion.explode` posts `explode` at the centre from the explosion's source entity before anything else.
- **Changed:**
  - Players (host and friends; `render_anim/src/minecraft_players.rs` `movement`): a sounding step posts `step` at the feet with the block it sounds from (a carpet or snow layer on top is that block, so carpets dampen), a move over the next step in water that is no step posts `swim`, and a landing from above posts `hit_ground` with the block landed on. Sneaking comes from the MW2 crouch and prone flags through the tick's controls (commit 1).
  - A placed block posts `block_place` where the survival side places it (`actions.rs`); `ServerSim::player_mines` posts `block_destroy` for each mined block (not for the ones an explosion took) with the acting player.
  - Mobs (`entities/src/movement.rs`): the body keeps the step, swim, splash and landing events of its moves (`Body.game_events`), which `EntityWorld::take_game_events` hands over with the mob's id every tick.
  - Projectiles: thrown items and tridents post `projectile_shoot` when thrown (by a player or a mob) and `projectile_land` where they hit; arrows of the entity world post `projectile_shoot` when spawned and `projectile_land` with their owner where they hit (`EntityWorld::arrow_landed`, `Arrow::impact_state`). A mob or player id behind an entity world event becomes a mob or a player source.
  - Explosions: `Level::explode_from` posts `explode` from its source entity (TNT, creepers, beds, anchors, crystals, MW2 blasts through `weapon_blast`).
- **Tests:**
  - `server::tests::mining_beside_a_sensor_is_heard_but_a_blasted_block_is_no_destroy` (frequency 12), `a_crouched_throw_is_heard_only_where_it_lands` (2; standing, the throw first: 3), `an_mw2_blast_near_a_sensor_is_an_explosion` (15).
  - `world::tests::a_mob_posts_its_landing_and_steps_and_an_arrow_its_shot_and_landing` (vendored entities: a zombie's fall and its steps on stone; a skeleton's arrow's shot and its landing at the centre of the stone it struck).
- **Decisions:**
  - A thrown item's or trident's `projectile_shoot` is posted when it is thrown, at the game time before the tick in which vanilla posts it on the projectile's first tick.
  - A trident that sticks in the ground lands on the block just ahead of where it stopped (the fork's trident keeps no struck block); one that strikes a mob lands at its position.
  - An arrow lands when it plays its hit sound: on a block, or on a mob or player that took the hit. A hit turned away (a creative player) posts no landing, unlike vanilla.
  - Mobs post `hit_ground` too (vanilla does: every landing with a fall above 0), not only the step, swim and splash the brief named.
  - `EMITS_SOUNDS` (D0, not merged into this branch yet): the step and swim game events are pushed by `Body::movement_emission` itself, not by the sounds `play_movement` makes from `Body.emission`, so a family whose movement sounds D0's `Family::emits_sounds` drops still posts them. To check at the D0 merge: every family's body is reached by `EntityWorld::body_mut`, which `take_game_events` drains (today every family with a body is).
- **Vendored files:** patched `entities/src/movement.rs`, `entities/src/projectile.rs`, `entities/src/world.rs`, `entities/src/world/game_events.rs` and `world/src/level/explosion.rs`.
- **Left for later (W3, "every other source"):** the other projectiles (witch potions of the entity world, fireballs, shulker bullets, llama spit, firework rockets), `entity_damage` from a mob hit, and every block, item and entity source of W3. The fork has no player bows, so the brief's "shoot an arrow" step throws a trident instead.
- **Test steps (survival; build on stone, away from other noise; commits 1 to 3):** wait 2 s between actions: a sensor is busy 30 ticks and cools down 10 more, and hears nothing in that time.
  1. Place a sculk sensor with a redstone lamp next to it. Walk around 4 blocks away: a ripple flies from your feet to the sensor, its tendrils click, the lamp lights for a second and a half, then the sensor rests for half a second. **w04_sensor_lamp**
  2. Crouch and walk (MW2 crouch or prone): nothing. Crouch and place a block 3 blocks away: it triggers (placing is never quiet). Stand on the sensor: it fires again every 2 s, crouched too.
  3. Put a comparator out of the sensor into a straight line of 15 redstone dust: the number of lit dust blocks is the frequency. Walking lights 1; jumping in place 4 blocks away lights 2 (the landing); mining a block nearby lights 12; placing one lights 13. **w05_sensor_comparator**
  4. Build a closed shell of white wool around the sensor (all sides): walking outside does nothing. Open one side: it triggers again. Walk on white carpet next to it: nothing. Place or break wool next to it: nothing.
  5. Stand 12 blocks from the sensor, crouch (so the throw itself is not heard) and throw a snowball so it lands 5 blocks from the sensor: it triggers and the line lights 2. Thrown standing up from 5 blocks away, the throw is heard first and the line lights 3. Crouch and throw the trident into the ground 4 blocks from it: it triggers (2).
  6. Put TNT 7 blocks from the sensor with a cobblestone wall between them (stone does not block vibrations, only wool does), light it and step back: the blast 4 s later triggers it and the line lights 15 (lighting it is heard from W3 on). An MW2 grenade near it lights 15 too.
  7. A cow from its egg wandering near the sensor triggers it with its steps (1); one that walks onto it fires it at 15.
  8. Place a calibrated sculk sensor; the face that looked at you as you placed it is its back. Lay 15 redstone dust from a lever so the last dust ends against that back face (that dust carries strength 1) and flip the lever on: only steps (frequency 1) trigger it, walking 12 blocks away still reaches it (radius 16), and mining a block beside it does not (12).
  9. Save and quit while a ripple is in flight, load: the sensor still receives it.
  10. Friend window: the friend's steps trigger the sensor and the host's lamp lights; both see the same ripple. The friend crouching makes no ripple. **w06_friend_sensor**

```
IW4L_MINECRAFT_KIT=minecraft:sculk_sensor*16,minecraft:calibrated_sculk_sensor*4,minecraft:redstone_lamp*8,minecraft:redstone*64,minecraft:comparator*4,minecraft:repeater*4,minecraft:lever*4,minecraft:white_wool*64,minecraft:white_carpet*32,minecraft:snowball*16,minecraft:trident,minecraft:tnt*4,minecraft:flint_and_steel,minecraft:cow_spawn_egg*4,minecraft:stone*64,minecraft:cobblestone*64,minecraft:cooked_beef*16
```

## Group vibrations-more

### 1. Minecraft blocks make vibrations when used

After c89a387d ("Players, mobs, blocks and explosions make Minecraft vibrations").

- **Vanilla (javap on the 26.3 jar):**
  - `Level.gameEvent(entity, event, BlockPos)` posts at the block centre with no affected state; `gameEvent(event, pos, Context.of(state))` and `Context.of(entity, state)` carry one.
  - Doors: `useWithoutItem` and `setOpen` post `block_open`/`block_close` from the player (or the mob); `neighborChanged` posts them with no source when redstone flips the door, before the new state is set; `onExplosionHit` calls `setOpen(null, ...)`. `TrapDoorBlock.playSound(player, ...)` posts them (use: the player; redstone and explosions: none). `FenceGateBlock`: use from the player, redstone with no source, `onExplosionHit` with `Context.of(state)`.
  - `LeverBlock.pull(state, level, pos, player)`: `block_activate` or `block_deactivate` by the new power, from the player (none from an explosion). `ButtonBlock.press(..., player)`: `block_activate`; `tick` → `checkPressed`: `block_deactivate` with the arrow as source (none without arrows). `BasePressurePlateBlock.checkPressed(entity, ...)`: `block_activate` when the signal goes from 0 to more, `block_deactivate` back to 0, from the entity inside (none from the tick).
  - `NoteBlock.playNote(entity, ...)`: `note_block_play` only when it plays (a head instrument or air above), from the player for use and attack, none from redstone.
  - `BellBlock.attemptToRing(entity, ...)`: `block_change`; the three-argument `attemptToRing(level, pos, direction)` (redstone, `onExplosionHit`) passes no entity.
  - `DaylightDetectorBlock.useWithoutItem`: `block_change` with `Context.of(player, state)`. `LecternBlock.resetBookState(entity, ...)`: `block_change`; placing a book passes the player, `LecternBlockEntity` taking it passes none. `SweetBerryBushBlock` and `CaveVines` harvests: `block_change` from the player. `AbstractCandleBlock.extinguish(player, ...)`: `block_change` (none from a wind charge).
  - `ContainerOpenersCounter.incrementOpeners`/`decrementOpeners` (chests, trapped chests, barrels, ender chests) and `ShulkerBoxBlockEntity.startOpen`/`stopOpen`: `container_open` when the first opener comes, `container_close` when the last one goes, from that player; `recheckOpeners` posts with no source.
  - `BucketItem.use` → `fluid_pickup` from the player at the fluid; `playEmptySound` → `fluid_place` (not when the water evaporates); the dispenser's bucket behaviours post the same with no source. `TntBlock.prime(level, pos, entity, stack)`: `prime_fuse` from the igniter (none from redstone or fire). `FlintAndSteelItem` and `FireChargeItem.useOn`: `block_place` for the fire, `block_change` for a lit candle or campfire. Cauldron interactions (`CauldronInteraction.fillBucket`/`emptyBucket`): `fluid_pickup`/`fluid_place` with no source. Axes, hoes, shovels and honeycomb: `block_change` with `Context.of(player, state)`.
  - `PistonBaseBlock.triggerEvent`: `block_activate` (extend) and `block_deactivate` (retract) with `Context.of(state)`, no source; `moveBlocks` posts `block_destroy` for each block it breaks. `DispenserBlock.dispenseFrom`: `block_activate` with `Context.of(state)` only when it is empty (droppers post nothing). `LightningBolt.tick`: `lightning_strike` from the bolt at its position.
  - Wind charges: `ServerExplosion.explode` posts `explode` from the charge, and `onExplosionHit` triggers doors, trapdoors, gates, buttons, levers, bells and candles as above.
  - Item frames: `ItemFrame.interact` (add, rotate), `hurtServer` and `dropItem` (remove) post `block_change`; `HangingEntityItem.useOn` posts `entity_place`; breaking one kills it (`Entity.kill` → `entity_die`).
- **Changed:**
  - Vendored `world/src/level/game_event.rs`: `opened`, `activated`, `Level::block_game_event_from` (a block event with a given source) and `Level::acting_source`.
  - Vendored `redstone.rs`: doors, trapdoors and gates on use (the acting player), on redstone and from wind charges (no source); levers, buttons (and their release, no source), note blocks (`play_note` takes its source: the acting player for use and attack, none for redstone), bells (the acting player when hit on a face, none for redstone and wind), candles put out by wind; `entity.rs`: pressure plates (`plate_check_pressed` takes the entity inside, none from the tick) and lightning; `time.rs` daylight detectors; `lectern.rs` books placed (the acting player) and taken (none); `plants.rs` berry harvests; `dispenser.rs` empty dispensers and both buckets; `explosion.rs` primed TNT (its owner); `fire.rs` fire placed; `piston.rs` extend, retract and blocks a piston breaks.
  - `server.rs`: a wind burst posts `explode` (no source); lecterns and item frame actions run with the acting player; `frame_sound` posts the frame's event (`FrameSound::game_event`).
  - Containers: `OpenerEvent::game_event` (`openers.rs`), posted by the player who opened or closed (`menus.rs` `lid_effects`, `level.rs` `leave`, the closing inventory screen in `render_anim/src/minecraft_world.rs`). This covers chests, trapped chests, barrels, ender chests and shulker boxes, whose counts live on the survival side; the level's `set_chest_openers` (trapped chests only) posts nothing, so nothing is posted twice.
  - Item uses that set a block on the survival side (`actions.rs`, `Change::Set`): `game_events::item_use_event` (a bucket on a cauldron: `fluid_pickup`/`fluid_place`; the powder snow bucket: `block_place`; flint and steel on a candle or campfire, shovels, axes, hoes, honeycomb: `block_change`).
- **Tests:**
  - `level::game_event::tests::doors_gates_trapdoors_levers_buttons_and_note_blocks_post_their_events` (vendored world, gated on the 26.3 data: each id, the acting player as source, `block_close` on closing, the button's release with no source, a note block used and hit, and a door opened by redstone during a player's action with no source).
  - `openers::tests::opening_a_chest_posts_container_open_once_and_closing_the_last_viewer_posts_close` (and a leaving player closes it).
  - `game_events::tests::an_item_used_on_a_block_changes_it_and_a_bucket_on_a_cauldron_moves_fluid`.
  - `server::tests::a_bell_ring_is_frequency_eleven`, `a_lever_beside_a_sensor_is_ten_on_and_nine_off` (through `run_tick`).
  - The full minecraft_terrain run had one failure, `sections::tests::background_full_update_matches_the_synchronous_one` (render sections, untouched here); it passes on its own and failed only while other tracks loaded the PC.
- **Decisions:**
  - The brief's `level::redstone::tests::...` test lives in `level::game_event::tests`, next to the helpers, so the upstream `redstone.rs` keeps a small diff; the brief's `server::tests::opening_a_chest...` lives in `openers::tests`, because the fork counts container openers on the survival side.
  - Container events are posted where the survival side counts openers, not in `Level::set_chest_openers` (which only hears trapped chests). A recheck that closes a container (`recheck_lids`, no player) posts nothing; vanilla's `recheckOpeners` posts it without a source.
  - Events that vanilla posts with no source from a neighbour update (doors, trapdoors, gates, note blocks and bells powered by redstone) pass `None` explicitly, because the fork runs those updates inside the player's command, where `Level.acting_player` is still set.
  - A wind burst's `explode` carries no source or projectile owner (vanilla's source is the charge, whose owner listeners see as the projectile owner): the burst does not know its thrower, and only W4's warden would read it.
  - A bucket on a cauldron posts from the player (vanilla: no source); the survival side has only a player queue for game events.
  - Not in the fork, so not posted: composters, cakes, candles put out by hand, decorated pots, chiseled bookshelves, jukeboxes, tripwire hooks, beehives, and players or mobs on pressure plates (only level entities such as items press plates in the fork).
- **Vendored files:** patched `world/src/level/redstone.rs`, `entity.rs`, `time.rs`, `lectern.rs`, `plants.rs`, `dispenser.rs`, `explosion.rs`, `fire.rs`, `piston.rs` (and `game_event.rs` again).
- **Test steps (survival; a sensor with a comparator out of it into a straight line of 15 redstone dust, so the number of lit dust is the frequency; build 5 blocks from the sensor):** wait 2 s between actions (the sensor is busy 30 ticks and cools down 10 more).
  1. Open an oak door: the line shows 10; close it: 9. The same with an oak trapdoor and an oak fence gate. Flip a lever on (10) and off (9); press an oak button (10), and when it springs back (9). **w07_block_frequencies**
  2. Open a chest and a barrel: 10; close: 9. A second player opening the open chest adds nothing.
  3. Fill the bucket from a water source: 12; empty it on the ground: 13. Ring a bell: 11. Click a note block: 10.
  4. Drop an item (Q) onto an oak pressure plate: 10; pick it up again and the plate springs back: 9.
  5. Put TNT 7 blocks away and light it with flint and steel: 10 (prime fuse), then 15 when it blows. Light a candle with flint and steel: 11.
  6. Put a lever next to a piston and flip it: 10 when it pushes out, 9 when it pulls back.

```
IW4L_MINECRAFT_KIT=minecraft:sculk_sensor*16,minecraft:comparator*4,minecraft:redstone*64,minecraft:oak_door*2,minecraft:oak_trapdoor*2,minecraft:oak_fence_gate*2,minecraft:lever*4,minecraft:oak_button*4,minecraft:oak_pressure_plate*4,minecraft:chest*2,minecraft:barrel*2,minecraft:bucket,minecraft:water_bucket,minecraft:bell,minecraft:note_block*2,minecraft:tnt*4,minecraft:flint_and_steel,minecraft:candle*4,minecraft:piston*2,minecraft:cobblestone*64,minecraft:cooked_beef*16
```

### 2. Minecraft players and mobs make vibrations when they eat, drink, equip, ride, glide, shear, teleport, get hurt and die

After a8407b26 ("Minecraft blocks make vibrations when used").

- **Vanilla (javap on the 26.3 jar):**
  - `Consumable.onConsume`: `drink` or `eat` from the eater at its position; `LivingEntity.completeUsingItem` → `stopUsingItem` then posts `item_interact_finish` (3, in `#ignore_vibrations_sneaking`). Both come from the same spot in the same tick, so a sensor keeps the eating (8).
  - `LivingEntity.onEquipItem`: not on the first tick, not for spectators, only when the item changed; `equip` when the new item is equippable, `unequip` otherwise; a player only for its armour slots (`Player.doesEmitEquipEvent`).
  - `LivingEntity.actuallyHurt` (and `Player.actuallyHurt`): `entity_damage` from the hurt entity when what is left after absorption is not 0. `LivingEntity.die` and `ServerPlayer.die`: `entity_die` first, then the loot and the experience.
  - `LivingEntity.updateFallFlying`: `elytra_glide` every 10 ticks of gliding (`(fallFlyTicks + 1) % 10 == 0`; every second one also wears the elytra).
  - `Entity.startRiding(vehicle, force, true)`: `entity_mount` from the rider at the vehicle's position; `Entity.removeVehicle`: `entity_dismount` from the rider at the vehicle's position.
  - `Sheep`, `MushroomCow` and `SnowGolem` shearing: `shear` from the player at the mob. `Entity.dropAllLeashConnections(player)` also posts `shear`.
  - `SpawnEggItem.spawnMob`: `entity_place` from the user at the spawn block's centre; an egg used on a spawner: `block_change`. `Chicken.aiStep` laying an egg: `entity_place` from the chicken.
  - `Entity.processFlappingMovement` (from `applyMovementEmissionAndPlaySound` when the block under the feet is air): `flap` when `isFlapping`; `Chicken.isFlapping` is `flyDist > nextFlap` and `onFlap` sets `nextFlap = flyDist + flapSpeed / 2` (`flyDist` grows by 0.6 of every move, `nextFlap` starts at 1).
  - `Enderman.teleport(x, y, z)` and `Shulker.teleportSomewhere`: `teleport` from the mob at the spot it left. `TeleportRandomlyConsumeEffect` (chorus fruit) does the same at the eater's old spot.
  - `LivingEntity.checkTotemDeathProtection` and `BoneMealItem.useOn`: `item_interact_finish` from the player at its position.
- **Changed:**
  - Survival side: `actions.rs` posts `game_events::consumed` (`eat` or `drink`, then `item_interact_finish`) when a bite finishes. `Steps::heard(hurts, inventory)` (`player.rs`) returns `entity_damage` when the survival's hurt count grew, `equip`/`unequip` for each armour slot whose item changed (nothing on the first call), and one `elytra_glide` per glide the elytra counted (`Glided.glides`, every 10 gliding ticks); `render_anim/src/minecraft_players.rs` `movement` posts them for the host and each friend. `drop_on_death` takes the player's id and posts `entity_die` at the death spot for every death, kept inventory or not. A popped totem posts `item_interact_finish`.
  - Vendored entities: `DamageState.damaged` (set when a hit takes health) becomes `entity_damage` at the mob in `EntityWorld::take_game_events`; `take_deaths` posts `entity_die` for each death before it hands the deaths (with their experience) back, also with `mob_drops` off; `stop_riding` posts `entity_dismount` from the rider at its vehicle (`rider_id`: a player as `PLAYER_TARGET` plus its id); `Body` gains `fly_dist`, `next_flap` and `flap_speed` (only chickens set it, from their own flap speed after each `ai_step`), and `movement_emission` posts `flap` in the air; a chicken laying an egg posts `entity_place`; endermen (`enderman::fits`) and shulkers post `teleport` at the spot they leave.
  - Terrain: `ServerSim::mount` posts `entity_mount` from the player at the vehicle; shearing a sheep, a mooshroom or a snow golem posts `shear` from the player (`mob_actions.rs`); a hatched spawn egg posts `entity_place` at the spawn block, an egg on a spawner `block_change` (both commands now run with the acting player); bone meal that grows something posts `item_interact_finish` at the acting player's feet.
- **Tests:**
  - `vibrations::tests::eating_eight_blocks_away_powers_a_sensor_to_one` (the eat and the finish from the same spot: the eat wins, power `max(1, 15 - floor(15 / 8 × 8))` = 1).
  - `server::tests::a_dead_cow_posts_entity_die_before_its_experience` (a player's blast kill near a sensor: the orbs are there and the sensor reads 15).
  - `world::tests::a_hurt_cow_posts_entity_damage_its_death_entity_die_and_a_falling_chicken_flaps` (vendored entities: one `entity_damage` per hit that takes health, none for a weaker hit inside the cooldown; `entity_die` at the cow's spot queued by `take_deaths`; a chicken dropped from 5 blocks flaps, only in the air).
  - `player::tests::a_helmet_put_on_is_heard_as_equip_and_taken_off_as_unequip` (also the hurt count and the glides), `elytra::tests::gliding_is_heard_every_ten_ticks_also_in_creative`.
- **Decisions:**
  - A player's hurt is read from the survival's hurt count (`Survival::hurts`, which MW2 damage through `observe` also raises), once per movement update, so several hurts between two updates are one `entity_damage`. A hurt whose damage absorption took entirely still counts when it came through MW2 (`observe` counts it); vanilla posts nothing then.
  - Equipment is compared by item id per armour slot: a durability change is no `equip`. Mob equipment changes post nothing yet.
  - `entity_die` of a player comes from `drop_on_death`, which every death passes through (the drop itself is still skipped with kept inventory).
  - A mob's killing blow can lose its `entity_damage` when the dead mob is gone before the tick's events are taken; the `entity_die` from the same spot (15) outweighs it (7) for every listener anyway.
  - Mounting is posted for players only (the one way the fork mounts at runtime); dismounting is posted for every rider that `stop_riding` takes off. A rider dropped because its vehicle died posts nothing.
  - `EMITS_SOUNDS` (D0): `flap`, `entity_place` of an egg and `teleport` go through `Body.game_events` too, so D0's `Family::emits_sounds` does not drop them. To check at the D0 merge, with steps and swims.
- **Not in the fork, so not posted:** chorus fruit (the fork does not teleport the eater), bats', parrots' and bees' flaps, `bounce` on slime blocks and beds, item entities hurt (`ItemEntity.hurtServer`), `entity_interact`/`entity_action`, and armour stands. **Left for later:** `shear` when shears cut a lead (E2 owns leads this round), mob equipment changes.
- **Vendored files:** patched `entities/src/health.rs`, `movement.rs`, `chicken.rs`, `enderman.rs`, `world.rs`, `world/riding.rs`, `world/shulkers.rs` (and `world/game_events.rs` again).
- **Test steps (survival; the same sensor and comparator line, 5 blocks away; 2 s between actions):**
  1. Eat an apple: the line shows 8. Drink the milk bucket: 8. Crouched, eating is still heard (8).
  2. Put the diamond helmet on from the inventory screen: 5; take it off: 4.
  3. Hatch a chicken from its egg 4 blocks above the sensor's level on a pillar, so it drops off: its flaps light 1 while it falls (place the egg on top of a 4-high cobblestone pillar next to the edge; it walks off). Hatching it lights 14.
  4. Shear a sheep from its egg near the sensor: 6.
  5. Hatch a horse from its egg near the sensor and get on it with an empty hand: 6; crouch to get off (or let it throw you): 5.
  6. Take fall damage (jump from a 5-block pillar next to the sensor): 7. Then build the pillar to 25 blocks and jump: the death next to the sensor lights 15.
  7. Friend window: the friend eats next to the host's sensor: the host's line shows 8 (shown in **w10_friend_vibrations**, commit 4).

```
IW4L_MINECRAFT_KIT=minecraft:sculk_sensor*8,minecraft:comparator*2,minecraft:redstone*32,minecraft:apple*16,minecraft:milk_bucket,minecraft:diamond_helmet,minecraft:chicken_spawn_egg*4,minecraft:sheep_spawn_egg*2,minecraft:shears,minecraft:horse_spawn_egg*2,minecraft:cobblestone*64,minecraft:cobblestone*64,minecraft:cooked_beef*16
```

### 3. Minecraft amethyst resonates sculk vibrations to calibrated sensors

After the commit above (its subject: "Minecraft players and mobs make vibrations when they eat, drink, equip, ride, glide, shear, teleport, get hurt and die").

- **Vanilla (javap on the 26.3 jar, and its data pack):**
  - `SculkSensorBlock.activate`: after the phase, the scheduled tick and `updateNeighbours`, `tryResonateVibration(entity, level, pos, frequency)` runs, then the tendrils event and the clicking sound. It is in `SculkSensorBlock`, so plain and calibrated sensors both resonate.
  - `tryResonateVibration`: for each `Direction.values()` (down, up, north, south, west, east), a neighbour in block `#vibration_resonators` (26.3: only `amethyst_block`; budding amethyst is not in it) posts `VibrationSystem.getResonanceEventByFrequency(frequency)` (`resonate_<frequency>`) at its centre with `Context.of(entity, its state)`, the sensor's own vibration source, and plays `block.amethyst_block.resonate` there with no source entity, volume 1 and pitch `RESONANCE_PITCH_BEND[frequency]` = `NoteBlock.getPitchFromNote` of the notes 0, 0, 2, 4, 6, 7, 9, 10, 12, 14, 15, 18, 19, 21, 22, 24 (`2^((note - 12) / 12)`).
  - `resonate_1` to `resonate_15` are in `#vibrations` with frequency N and radius 16, so every sensor in reach hears them; a calibrated sensor set to N by its back input takes only `resonate_N`.
- **Changed:** vendored `world/src/level/sculk.rs`: `sculk_activate` takes the frequency, and `sculk_resonate` posts the resonance events and sounds from each amethyst neighbour (the tag read from the data pack's block tags at runtime).
- **Tests:**
  - `level::sculk::tests::an_amethyst_block_beside_a_sensor_resonates_its_frequency_to_a_calibrated_sensor` (vendored world, gated on the 26.3 data: amethyst above and east resonate `resonate_8` in vanilla's direction order with the source and the amethyst state, before the tendrils; budding amethyst does not; pitch 1 for frequency 8 and 2 for 15).
  - `server::tests::amethyst_carries_an_eaten_vibration_to_a_calibrated_sensor_out_of_its_reach` (through `run_tick`: a player eats 6 blocks from a sensor with amethyst beside it and 16.4 blocks from a calibrated sensor; the calibrated one reads 8 from the resonance).
- **Decisions:** none beyond vanilla.
- **Vendored files:** patched `world/src/level/sculk.rs` again.
- **Test steps (survival, open stone floor; 2 s between actions):**
  1. Place a sculk sensor with an amethyst block against its side. 14 blocks away on the other side place a calibrated sculk sensor with a comparator and a line of redstone dust out of it. Lay a lever and 8 redstone dust so the last dust ends against the calibrated sensor's back face (the face that looked at you when you placed it): its back input is 8.
  2. Walk to 4 blocks from the first sensor on the side away from the calibrated one (18 blocks from it, out of its 16-block reach) and eat an apple: the first sensor triggers, the amethyst chimes, and a moment later a ripple flies from the amethyst to the calibrated sensor, whose line shows 8. **w08_amethyst_resonance**
  3. Walk there instead (frequency 1): the first sensor triggers and the amethyst chimes, but the calibrated sensor, set to 8, stays dark.

```
IW4L_MINECRAFT_KIT=minecraft:sculk_sensor*4,minecraft:calibrated_sculk_sensor*2,minecraft:amethyst_block*4,minecraft:comparator*2,minecraft:redstone*64,minecraft:lever*2,minecraft:apple*16,minecraft:stone*64,minecraft:cooked_beef*16
```

### 4. MW2 guns make Minecraft vibrations

After the commit above (its subject: "Minecraft amethyst resonates sculk vibrations to calibrated sensors").

- **Vanilla (javap on the 26.3 jar), for the projectile a bullet stands for:** `Projectile.tick` posts `projectile_shoot` on the projectile's first tick with the owner as source (in `#ignore_vibrations_sneaking`, so a crouched shooter's shot is not heard); `Projectile.onHit` posts `projectile_land` from the projectile, whose owner listeners see as the projectile owner, at the centre of the struck block with its state, or where it struck an entity (not in the sneaking tag). A projectile that hits nothing lands nowhere.
- **Changed:**
  - `render_anim/src/minecraft_world.rs`: `gun_vibrations(events, credit)` reads a tick's voxel events before the build filter (a player who may not build still makes noise): one `projectile_shoot` per shooter at the muzzle of its first ray (a shotgun's pellets are one shot); a ray whose end, nudged 0.05 along it, is a block that the same shooter's `Shot` struck lands at that block's centre with the block; of the other rays from a muzzle, as many as that shooter has `MobShot`s from there land where they stopped, shortest first (a ray stops where it meets the mob, while one that met nothing runs to the end of the trace); every other ray lands nowhere. `post_gun_vibrations` hands them to `Entities` with the block's id as the affected state. The host's level and each friend's level (`minecraft_levels.rs`) do this for the shots in them.
  - `game_events::PlayerEvent.projectile` and `Entities::projectile_landed`: a landing comes from the player as projectile owner, not as source (W4's warden reads the owner); the server builds it with no source, so the sneaking check does not apply to it.
  - A mob a bullet hits posts its own `entity_damage` through the entity world (commit 2); grenades, rockets and other MW2 blasts post `explode` through `weapon_blast` (group vibrations).
- **Tests:**
  - `minecraft_world::gun_tests::an_mw2_bullet_posts_its_shot_and_landing_and_a_ray_into_the_air_lands_nowhere` (a block hit lands at its centre with the block; a ray into the air only shoots; another shooter's block is not this bullet's; a shotgun shell is one shot, and of three pellets with one `MobShot` only the short one lands; a ray with no player behind it posts nothing).
  - `server::tests::a_crouched_gun_shot_is_heard_only_where_it_lands` (through `run_tick`: crouched, the sensor reads 2; standing, the nearer shot reads 3; the landing has the player as owner and no source).
- **Decisions:**
  - The brief's `minecraft_world::tests::an_mw2_bullet_posts_its_shot_and_landing_and_a_crouched_shot_only_its_landing` is split: the conversion is tested in `minecraft_world::gun_tests`, the crouch in `server::tests::a_crouched_gun_shot_is_heard_only_where_it_lands`, because the crouch is read where the server hears the event.
  - Bullets are projectiles, as the brief decided: crouched or prone shooting is not heard, the impact always is, and the impact's vibration comes from the bullet with its shooter as owner.
  - A bullet that hits a player (MW2's own hit) posts no landing: there is no block or mob event for it; the hurt player posts its own `entity_damage` (commit 2).
  - A bullet into wool is dampened like an arrow (the affected state is the wool).
- **Test steps (survival; the sensor and comparator line):**
  1. Stand 20 blocks from the sensor and shoot the ground beside it: the line shows 2. **w09_mw2_vibrations**
  2. Stand 6 blocks from it and shoot a wall 30 blocks away on the other side: the shot itself triggers it (3).
  3. Go prone (or crouch) and shoot that wall again: nothing. Shoot the ground beside the sensor: 2.
  4. Shoot into the open sky 4 blocks from it, crouched: nothing (the bullet lands nowhere).
  5. Throw a grenade near it: 15.
  6. Friend window: the friend opens the oak door near the host's sensor (10), eats an apple next to it (8) and, standing 20 blocks away, shoots the ground beside it (2): the host's line shows the same numbers. **w10_friend_vibrations**

Combined kit for the group's main steps (w07 to w10; 27 stacks; the MW2 weapons come from the MW2 loadout). Commit 1's bell, TNT, candle and piston steps keep its own kit:

```
IW4L_MINECRAFT_KIT=minecraft:sculk_sensor*16,minecraft:calibrated_sculk_sensor*4,minecraft:redstone_lamp*8,minecraft:redstone*64,minecraft:comparator*4,minecraft:repeater*4,minecraft:amethyst_block*8,minecraft:oak_door*2,minecraft:oak_trapdoor*2,minecraft:oak_fence_gate*2,minecraft:chest*2,minecraft:barrel*2,minecraft:lever*4,minecraft:oak_button*4,minecraft:oak_pressure_plate*4,minecraft:note_block*2,minecraft:water_bucket,minecraft:bucket,minecraft:apple*16,minecraft:milk_bucket,minecraft:diamond_helmet,minecraft:chicken_spawn_egg*4,minecraft:horse_spawn_egg*2,minecraft:sheep_spawn_egg*2,minecraft:shears,minecraft:white_wool*32,minecraft:cobblestone*64
```

## Group shriekers-catalysts

### 1. Minecraft sculk shriekers warn players and darken them

After 6a9a7594 ("MW2 guns make Minecraft vibrations").

- **Vanilla (javap on the 26.3 jar):**
  - `SculkShriekerBlock`: properties `shrieking`, `waterlogged`, `can_summon` (all false by default); `stepOn` shrieks it for `tryGetPlayer(entity)`, sneaking or not; `tick` sets `shrieking=false` (flags 3) and calls `tryRespond`. `SHAPE_COLLISION` is 8 high, so `TOP_Y` is 0.5. There is no `affectNeighborsAfterRemoval`: the block entity's `preRemoveSideEffects` responds when a shrieking shrieker is removed.
  - `SculkShriekerBlockEntity$VibrationUser`: radius 8, `#shrieker_can_listen`, `canReceiveVibration` only while not shrieking and when `tryGetPlayer(context.sourceEntity)` finds a player, `onReceiveVibration` shrieks for `tryGetPlayer(projectileOwner ?? entity)`, `requiresAdjacentChunksToBeTicking` true. `tryGetPlayer`: a player, a vehicle's controlling passenger that is a player, a projectile's owner player, an item entity's owner player.
  - `tryShriek`: nothing while shrieking; `warningLevel = 0`; when `canRespond` (`can_summon`, not peaceful, game rule `spawn_wardens`) a failed `tryToWarn` returns silently; `shriek`: `shrieking=true` with flags 2, a tick after 90, level event 3007, game event `shriek` at the centre from the player. `tryRespond`: when it can respond and the level is above 0, `trySummonWarden` (level 4 only) or else the reply sound by level at `pos + randomBetweenInclusive(-10, 10)` per axis (block corner, no 0.5), volume 5, pitch 1, HOSTILE; then `Warden.applyDarknessAround(level, centre, null, 40)`.
  - `WardenSpawnTracker`: `tick` (at 12000 ticks since the last warning the level drops one and the count restarts, else the count rises; the cooldown counts down), `tryWarn` (no warden whose box meets the 48 cube around the centre; the alive, non-spectator players within 16 of the centre (`closerThan`, strict) plus the trigger player; nobody on cooldown; the highest level among them rises (level + 1 up to 4, cooldown 200, count 0) and everyone copies it). Codec `ticks_since_last_warning`, `warning_level`, `cooldown_ticks`, each a non-negative int defaulting to 0; a broken one decodes to a new tracker. `ServerPlayer.tick` ticks it every tick, `restoreFrom` keeps it, the player file saves it as `warden_spawn_tracker`.
  - `MobEffectUtil.addEffectToPlayersAround`: survival or adventure players (`GameType.isSurvival`) closer than the radius to the centre, without the effect, with a lower amplifier, or one that `endsWithin(displayLimit - 1)` (not infinite and at most that many ticks left). `applyDarknessAround`: darkness 260 ticks, amplifier 0, display limit 200, ambient false and visible false (so `showIcon` false).
  - `LevelEventHandler` 3007: ten `ShriekParticleOption(i * 5)` at (x + 0.5, y + 0.5, z + 0.5), then unless waterlogged `block.sculk_shrieker.shriek` there at volume 2, pitch `0.6 + random × 0.4`. `ShriekParticle`: waits its delay, then lives 30 ticks rising with `yd` 0.1 and friction 0.98; quad size `0.85 × clamp((age + partial) / 30 × 0.75)`, alpha `1 - clamp((age + partial) / 30)`, block light 15, translucent, two quads turned `rotationX(-1.0472)` and `rotationYXZ(-π, 1.0472, 0)`, sprite `particle/shriek`.
- **Changed:**
  - Vendored `world/src/level/sculk.rs`: `is_sculk_shrieker`, `shrieker_shrieking`, `shrieker_can_respond` (with the new `Level.spawn_wardens`), the block entity's `warning_level`, `sculk_shriek`, the scheduled tick (`sculk_block_tick`, dispatched from `mod.rs` `tick_block`) and `sculk_pre_remove` (from `drops.rs` `pre_remove_side_effects`), which queue responses in `Level.shrieker_responses` with the warning level.
  - New `crates/minecraft_terrain/src/warden_tracker.rs`: `WardenSpawnTracker` (tick, NBT), `try_warn`, the warning box, the reply sounds, `players_around` and `EffectGrant` with `takes` (the rules above). `SHRIEK_DARKNESS` is the shrieker's grant; W5's warden reuses the type.
  - `server.rs`: players (alive, not spectators) standing on a shrieker, and mobs on one with a controlling player, shriek it every tick through `ServerSim::try_shriek`. `Hearing` answers for shriekers: `can_receive` while quiet and with a player behind the source or the projectile owner (`player_behind`, `tryGetPlayer`), `receive` queues the shriek, done after the listeners tick. A sensor's tendrils event now carries the projectile owner as its source when the vibration came from a projectile, so a snowball landing near a sensor makes a shrieker beside it shriek for the thrower. `respond_shriekers` plays the reply (HOSTILE, through `Output.mob_sounds`) and gives darkness to the alive players closer than 40 to the centre through the new `Output.player_effects`. Game rule `spawn_wardens` reaches `Level.spawn_wardens`. A shrieker's listener now also waits for the 3 × 3 chunks around it (`vibrations.rs`).
  - Trackers: `Player.warden_tracker`, saved as `warden_spawn_tracker` in `players/data/<uuid>.dat` (`PlayerData.warden_spawn_tracker`, now an owned key), read on join, kept through death and dimension changes (it lives on the `Player`). It ticks every player tick on the player's 20 Hz hand clock (`minecraft_players::movement`, alive or not). `PlayerView.warden` carries it into `TickInput.warden_trackers`; a warning returns the changed trackers in `Output.warden_trackers`, routed to `PlayerOutputs.warden_tracker` and applied in `minecraft_players::struck` for the host and every friend.
  - `Output.player_effects` reaches `PlayerOutputs.effects`; `struck` applies each with `Survival::take_effect` (survival or adventure, and `EffectGrant::takes`).
  - Client: `level_events.rs` plays the 3007 shriek unless the shrieker is waterlogged; new `crates/minecraft_terrain/src/shriek_particles.rs` (`ShriekRings`), drawn by `EntityView` (host and friends, who get 3007 as a level event) in the translucent mesh; `particle/shriek` joins the atlas (`mesh.rs`).
- **Tests:**
  - `warden_tracker::tests`: `each_shriek_raises_the_level_with_a_ten_second_cooldown`, `players_within_sixteen_rise_together_and_one_at_twenty_does_not` (also: a trigger player out of reach still counts), `a_player_on_cooldown_blocks_everyone`, `a_warden_within_the_box_stops_the_warning`, `the_level_drops_one_every_ten_minutes`, `it_round_trips_in_the_player_file` (partial and broken tags too), `the_replies_grow_closer_and_angrier`, `player_effects_follow_vanilla_s_rules`.
  - Vendored `level::sculk::tests` (gated on the 26.3 data): `a_shriek_lasts_ninety_ticks_and_responds_at_its_end`, `a_shrieker_without_can_summon_shrieks_but_never_warns`, `peaceful_or_spawn_wardens_off_means_no_response`, `a_shrieking_shrieker_broken_still_responds`.
  - `server::tests` (through `run_tick`, with the client's trackers ticked between ticks): `warnings_one_to_three_play_the_nearby_sounds_and_darken_within_forty` (standing on it shrieks it, level event 3007, only the player within 16 warned, reply 90 ticks later within ±10 at volume 5, darkness to the players within 40 and not at 41.5, silent while cooling down, then `nearby_closer` and `nearby_closest`), `the_fourth_warning_plays_the_angry_sound_while_no_warden_can_come` (a player 4.5 blocks away rises with them; with `spawn_wardens` off it shrieks and nothing answers), `a_sensor_s_click_makes_a_shrieker_nearby_shriek` (a step near a sensor two blocks from a shrieker without `can_summon`: it shrieks, never warns).
  - `level_events::tests::a_shriek_screams_over_the_shrieker_unless_it_is_under_water`, `shriek_particles::tests::a_shriek_sends_ten_rings_five_ticks_apart_that_rise_and_fade`, `the_rings_tilt_up_from_both_sides`.
  - `player_nbt::tests::the_warden_spawn_tracker_is_saved_per_player`, `survival::tests::a_shrieker_s_darkness_is_taken_like_vanilla`.
  - `tests/vanilla_world.rs` (`a_saved_world_comes_back_whole_with_vanilla_keys_and_tag_types`) now expects the `warden_spawn_tracker` compound with its three ints in the player file, as vanilla always writes it.
  - The full minecraft_terrain run had one failure, `sections::tests::background_full_update_matches_the_synchronous_one` (untouched render sections); it passes on its own and fails only while other tracks load the PC, as in group vibrations-more.
- **Decisions:**
  - The tracker lives on each `Player` and ticks with the player, as vanilla's `ServerPlayer` does, so it keeps counting in any dimension and survives death. The server reads it from the tick input; after a warning it holds its own copy of each warned tracker for 20 ticks (`WARNED_HOLD`), longer than the round trip of the output, so a second shrieker a tick later sees the cooldown as vanilla's one shared object would.
  - `player_effects_follow_vanilla_s_rules` lives in `warden_tracker::tests` (the rule) and `survival::tests::a_shrieker_s_darkness_is_taken_like_vanilla` (applying it), not in `server::tests`: the game mode is known on the survival side only.
  - The fork's effects carry no visibility flags, so the shrieker's darkness shows its icon (vanilla hides it: `showIcon` false).
  - A shrieker broken or replaced during a command responds at the next server tick (its response is queued), not inside the command.
  - The fork's item entities keep no thrower, so a dropped item never shrieks a shrieker (vanilla: an item a player threw does).
  - A vibration loaded from a save knows only its source's UUID (W2), so a shrieker that receives it after a load stays quiet.
  - A shriek ring rises without block collision (vanilla's particle stops under a ceiling).
  - Until W4 plugs `ServerSim::summon_warden` into `respond_shriekers`, a fourth warning plays `listening_angry`, vanilla's fallback when the summon fails; the "warden nearby" check already finds wardens by entity type (`minecraft:warden`).
- **Vendored files:** patched `world/src/level/sculk.rs`, `world/src/level/mod.rs`, `world/src/level/drops.rs`.
- **Test steps (survival, Normal; flat open ground at night, `/time set midnight`; steps 4 and 5 of the brief change from W4 on):**
  1. `/setblock ~ ~-1 ~3 minecraft:sculk_shrieker[can_summon=true]` (a shrieker placed by hand never summons, as in vanilla). Step on it, then step off: it shrieks with rising soul rings and a scream, your view darkens (W1), and somewhere a warden sound plays. **w23_shriek_warning**
  2. Step on it again within 10 s of the shriek: it stays silent. Wait 10 s and step on it again: the second warning, the warden sound is closer.
  3. Save and quit, load, wait 10 s and step on it: it continues at warning three (the closest sound).
  4. Wait 10 s and step on it a fourth time: it shrieks and the angriest warden sound plays (no warden can come before W4). **w24_warden_summoned** (from W4 on, a warden digs out).
  5. Place a sculk shrieker from the kit (it cannot summon) 2 blocks from a sculk sensor: walking near the sensor makes this shrieker shriek every time the sensor clicks; it never darkens your view or plays warden sounds. Throw a snowball so it lands next to the sensor: the shrieker shrieks too.
  6. `/gamerule spawn_wardens false`, wait 10 s and step on the summoning shrieker: it shrieks, but no warning, darkness or warden sound follows. `/gamerule spawn_wardens true`.
  7. Friend window: with the host 5 blocks away, the friend steps on the summoning shrieker: both warning levels rise together (the host's next shriek continues from it) and both get darkness; the friend sees and hears the shriek. **w26_friend_shrieker**

```
IW4L_MINECRAFT_KIT=minecraft:sculk_shrieker*8,minecraft:sculk_sensor*16,minecraft:deepslate*64,minecraft:cobblestone*64,minecraft:white_wool*32,minecraft:snowball*16,minecraft:milk_bucket,minecraft:milk_bucket,minecraft:cooked_beef*32,minecraft:diamond_chestplate+protection=4,minecraft:diamond_helmet
```

### 2. Minecraft sculk catalysts bloom when mobs die nearby

After 1180b5fb ("Minecraft sculk shriekers warn players and darken them").

- **Vanilla (javap on the 26.3 jar):**
  - `SculkCatalystBlockEntity$CatalystListener`: radius 8, `BY_DISTANCE`; `handleGameEvent` takes `entity_die` from a `LivingEntity` whose experience was not consumed (`wasExperienceConsumed`): when `shouldDropExperience` (not a baby, for animals) and `getExperienceReward(level, killer)` (no player kill and no `mob_drops` needed) is above 0 it adds cursors at `BlockPos.containing(event position + 0.5 up)` (commit 3); then, for every such death, `skipDropExperience` and `bloom`. Every catalyst in reach handles the event in distance order, so the nearest one takes the experience and the others find it consumed.
  - `bloom`: `bloom=true` (`setBlockAndUpdate`), a tick after 8 that sets it back, `sendParticles(SCULK_SOUL, x + 0.5, y + 1.15, z + 0.5, 2, 0.2, 0.0, 0.2, 0.0)` and `block.sculk_catalyst.bloom` at the block, volume 2, pitch `0.6 + random × 0.4`.
  - `LivingEntity.die` posts `entity_die` before `dropAllDeathLoot` and `dropExperience` (which needs a player kill, `shouldDropExperience` and `mob_drops`); `ServerPlayer.die` posts it first too.
  - The sculk soul particle is `EmissiveRisingParticle` in 26.3 (`EmissiveProvider`: glowing, alpha 1): `RisingParticle` (friction 0.96, the base velocity times 0.01 plus the given one, the position nudged `(random - random) × 0.05` per axis, lifetime `8 / (random × 0.8 + 0.2) + 4`), `scale(1.5)`, the 11 `sculk_soul` sprites by age, translucent, block light 15. `sendParticles` places each of its 2 particles at a gaussian offset times the spread (0.2, 0, 0.2).
- **Changed:**
  - Vendored `entities/src/world.rs` `take_deaths`: with `mob_drops` off the deaths still come back, without loot, equipment or charged creeper heads, instead of an empty list. `mob_actions::death_remains` reports every death's spot, also with no experience.
  - `server.rs`: the three places that turned mob deaths into orbs (melee, sweep, `drop_death_loot`) call `ServerSim::award_death`, which asks the catalysts first (`catalyst_takes`: `Vibrations::catalyst_near`, the nearest catalyst listener within 8 blocks by vanilla's block distance, nearest centre first); a catalyst that takes it blooms and no orbs drop. `Command::AwardExperience` gains `died: Option<u64>`; `ServerHandle::award_death_experience` and `Entities::award_death_experience` send a player's death experience marked with the player, and `minecraft_players::drop_on_death` sends it for every death (0 with `keep_inventory`, so a catalyst still blooms).
  - Vendored `world/src/level/sculk.rs`: `sculk_bloom` and the bloom's end in the scheduled tick (`mod.rs` dispatches catalysts too); `Level.catalyst_blooms` collects the blooms, which `finish_batch` turns into `EntitySnapshot.sculk_souls` (`vibrations::soul_burst`, 1.15 over the block).
  - Client: `ambient_particles` `Kind::SculkSoul` with its 11 sprites in the atlas; `EntityView` spawns 2 per soul burst for the host and friends; `mob_sender` keeps the bursts until sent and sends a friend the ones within 32 blocks in chunks it has.
- **Tests:**
  - `server::tests::a_cow_killed_five_blocks_from_a_catalyst_blooms_it_and_drops_no_orbs` (no orbs, bloom, its sound, the soul burst at 1.15 over it, rest after 8 ticks; with `mob_drops` off a death still blooms it and drops nothing), `ten_blocks_away_it_drops_orbs`, `two_catalysts_the_nearer_takes_it`, `a_player_s_death_beside_a_catalyst_leaves_no_orbs` (a death out of reach still drops orbs; plain awarded experience is never taken).
  - `vibrations::tests::the_nearest_catalyst_within_eight_takes_a_death`, vendored `level::sculk::tests::a_catalyst_blooms_for_eight_ticks_with_its_sound`, `ambient_particles::tests::a_sculk_soul_hangs_glowing_and_runs_through_its_eleven_frames`, `mob_sender::tests::a_catalyst_s_souls_reach_friends_near_it`.
  - Full runs of the touched crates pass: minecraft_terrain 1455 (3 ignored), minecraft_survival 766 + 1, minecraftoss-entities 400, minecraftoss-world 113, render_anim 215.
- **Decisions:**
  - The catalyst takes a death where its experience is awarded, not through the game event dispatch: vanilla consumes it inside `die()`, before the orbs, while the fork dispatches game events after the mobs' tick, when the orbs would already be in the level. The `entity_die` game event still reaches the sensors as before.
  - The brief put the raw experience reward (`MobDeath` without the player kill check) in this commit; it moves to commit 3, where it feeds the cursors. Blooming needs only the death's spot.
  - A death with `keep_inventory` sends 0 experience, so the catalyst still blooms (vanilla consumes every player death in reach).
  - The ender dragon's experience is not a death report (vanilla drops it over its death animation in `tickDeath`), so catalysts leave it alone.
- **Vendored files:** patched `world/src/level/sculk.rs` and `world/src/level/mod.rs` again, `entities/src/world.rs`.
- **Test steps (survival, Normal; flat open ground):**
  1. Place a sculk catalyst; hatch cows 2-5 blocks away and kill them with the sword: the catalyst flashes, two soul wisps rise over it with its bloom sound, and no experience orbs drop. (From commit 3 on, sculk also spreads.) **w25_catalyst_bloom**
  2. Kill a cow 10 blocks from the catalyst: orbs drop as usual.
  3. Place a second catalyst 8 blocks from the first and kill a cow between them, closer to one: only the nearer one blooms.
  4. `/gamerule mob_drops false` and kill a cow beside a catalyst: it still blooms. `/gamerule mob_drops true`.
  5. Drink the 16 experience bottles, take the totem out of your hands and `/kill @s` 3 blocks from the catalyst: it blooms and no orbs drop where you died.
  6. Friend window: the friend kills a cow beside the host's catalyst: both see it bloom and the souls rise.

```
IW4L_MINECRAFT_KIT=minecraft:sculk_catalyst*4,minecraft:cow_spawn_egg*16,minecraft:diamond_sword+sharpness=5,minecraft:experience_bottle*16,minecraft:deepslate*64,minecraft:cooked_beef*32
```

### 3. Minecraft sculk spreads from catalysts like vanilla

After 539a40dd ("Minecraft sculk catalysts bloom when mobs die nearby").

- **Vanilla (javap on the 26.3 jar):**
  - `SculkSpreader.createLevelSpreader`: not world generation, `#sculk_replaceable`, growth spawn cost 10, no-growth radius 4, charge decay rate 10, additional decay rate 5 (world generation: `#sculk_replaceable_world_gen`, 50, 1, 5, 10). `MAX_CHARGE` 1000, at most 32 cursors; `addCursors` splits a charge above 1000; `load` reads `cursors` (`sizeLimitedListOf(32)`), `save` writes them. `SculkCatalystBlockEntity.serverTick` calls `updateCursors(level, pos, random, true)` every tick.
  - `ChargeCursor.update` for the level spreader: only while `shouldTickBlocksAt`; a vein spread plays `block.sculk.spread` at the cursor; with nowhere to move a cursor keeps its charge where it is (world generation discharges it and drops it); no 12-block limit on moves (`canMoveToPos` is world generation only). `SculkBlock.attemptUseCharge`: a growth 1 in 11 a shrieker with `can_summon` set to `isWorldGeneration` (false here), else a sensor, its place sound at the cursor. `SculkVeinBlock.attemptPlaceSculk` plays `block.sculk.spread` at the block it turns to sculk.
  - `updateCursors`: a cursor farther than 1024 (chessboard) is dropped; a spent cursor posts level event 3006 with data 0; cursors meeting at one spot merge when the two charges stay within 1000 (level only), otherwise both stay; each spot whose kept cursor has facings posts 3006 with `((int)(log1p(charge) / 2.3) + 1) << 6` plus `MultifaceBlock.pack(facings)`.
  - `ChargeCursor.CODEC`: `pos` (int array), `charge` (0 to 1000, default 0), `decay_delay` (0 to 1, default 1), `update_delay` (default 0), `facings` (a lenient optional list of direction names); defaults are not written.
  - `CatalystListener.handleGameEvent`: `addCursors(BlockPos.containing(position + 0.5 up), reward)` when `shouldDropExperience` and the reward is above 0. `LivingEntity.getExperienceReward` is `getBaseExperienceReward` without the player kill and `mob_drops` checks of `dropExperience`.
  - `LevelEventHandler` 3006: with a charge (`data >> 6`): `block.sculk.charge` with chance `0.3 + charge × 0.1` (volume `0.15 + 0.02 × charge² × random`, pitch `0.4 + 0.3 × charge × random`), then `UniformInt(0, charge)` sculk charge particles on every face (no facings; factor 0.65 up and down, 0.57 on the sides) or on each packed face (factor 0.35), each at a random spot of the face with a drift of ±0.005 along it (`ParticleUtils.spawnParticleOnFace`). Without a charge: `block.sculk.charge` at volume 1, then 40 pops (a full collision block, spread 0.45) or 20 (spread 0.25) at a random offset, moving with it times 0.07. `SculkChargeParticle`: friction 0.96, scale 1.5, no physics, sprites by age, block light 15, translucent, the given speed, lifetime `8 + random(12)`, a roll from its options; `SculkChargePopParticle`: the same at scale 1, lifetime `6 + random(4)`.
- **Changed:**
  - Vendored `generator/src/feature/kinds/sculk.rs`: the spreader is now `SculkSpreader` (public, `new(registries, world_generation)` with both configurations), with public `Cursor`, `add_cursors`, `pack_facings`, `SpreadEffects` (the level's sounds and 3006 events; world generation drops them) and `update_cursors` (vanilla's `updateCursors` with the merging). `SculkPatch` keeps its parameters and holds a world-generation spreader; world generation does exactly what it did.
  - Vendored `world/src/level/sculk.rs`: the catalyst's cursors live in its block entity's `cursors` (vanilla's codec, `cursor_tag` and `cursor_from_tag`); `add_catalyst_cursors`; `tick_catalyst` runs the level spreader on the level with the level random (`with_feature_random`), saves the cursors back and queues the 3006 events and the sounds.
  - Vendored `entities/src/world.rs`: `MobDeath.reward` is the raw reward (`shouldDropExperience`: monsters at any age, other mobs as adults), and `experience` keeps it only for a player's kill with `mob_drops` on. `death_remains` hands both on.
  - `server.rs`: `award_death(position, experience, reward)`; the catalyst that takes a death adds its reward as cursors half a block over the death; a player's death reward is its experience. `ServerSim::tick_catalysts` ticks every catalyst in a ticking chunk on each normal tick, after the level's tick.
  - Client: `level_events.rs` plays 3006 (the charge sounds, the face sparks and the pops), with `ambient_particles` `Kind::SculkCharge` and `Kind::SculkChargePop` and their sprites in the atlas. Friends get 3006 as a level event.
- **Tests:**
  - Vendored `level::sculk::tests::a_catalyst_s_cursors_spread_sculk_and_round_trip` (a charge of 2500 splits into 1000, 1000, 500 and saves with vanilla's keys and defaults; a cursor round trips; 40 charge turns the stone floor to sculk until spent, with 3006 sparks and pops and the spread sound, and no growth within four blocks), `a_runtime_shrieker_cannot_summon`.
  - `server::tests::a_cow_dying_in_lava_near_a_catalyst_feeds_it` (killed by nobody: no orbs, the catalyst blooms and gets the cow's 1 to 3 as charge half a block over it, and sculk spreads there), `a_player_s_death_charge_is_its_experience`.
  - `level_events::tests::a_charge_sparkles_on_the_faces_and_pops_when_it_is_spent`, `ambient_particles::tests::sculk_charges_drift_at_their_given_speed_and_glow`.
  - Full runs of the touched crates: minecraft_terrain 1459 (3 ignored), minecraftoss-entities 400, minecraftoss-generator 30, minecraftoss-world 115, render_anim 215, minecraft_survival 766 + 1. One survival test, `the_end::tests::an_arrival_rebuilds_the_platform_once_its_chunks_are_in` (it waits for streamed chunks with sleeps; untouched here), failed once while other tracks held both build slots and passes on its own.
- **Decisions:**
  - The cursors are read from and written back to the catalyst's block entity on every tick that it holds any: the block entity is the only copy, so a save, a chunk unload or a friend sees the spread where it is. The write sends a block entity change only when the cursors changed.
  - The charge sparks are drawn without their roll (the fork's ambient particles face the camera upright); vanilla turns the sprite half a turn on the up and down faces.
  - The 3006 pops count a block as a full block when it is not air and not a multiface block (veins, lichen), instead of vanilla's collision shape test; the cursor's spot is sculk, a vein or air.
- **Vendored files:** patched `generator/src/feature/kinds/sculk.rs`, `world/src/level/sculk.rs` again, `entities/src/world.rs` again.
- **Test steps (survival, Normal; flat open ground of grass or stone):**
  1. Place a sculk catalyst; hatch cows 2-5 blocks away and kill them with the sword: besides the bloom, a few blocks of sculk spread over the ground at the death spots with soft sparks and a crackle (a cow's 1-3 experience is too little to grow anything). **w25_catalyst_bloom**
  2. Drink the 16 experience bottles (about level 7), take the totem out of your hands and `/kill @s` 3 blocks from the catalyst: no orbs drop, and the bigger charge spreads sculk further, now and then growing a sculk sensor or a shrieker 4 or more blocks from the catalyst. Step on such a shrieker: it shrieks but never warns (it cannot summon).
  3. Save and quit while the sculk still spreads, load: it carries on.
  4. Friend window: the friend sees the sculk spread and its sparks.

Combined kit for the group (steps w23 to w26; 20 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:sculk_shrieker*8,minecraft:sculk_sensor*16,minecraft:sculk_catalyst*4,minecraft:sculk*64,minecraft:cow_spawn_egg*16,minecraft:chicken_spawn_egg*8,minecraft:diamond_sword+sharpness=5,minecraft:shield,minecraft:deepslate*64,minecraft:cobblestone*64,minecraft:white_wool*32,minecraft:milk_bucket,minecraft:milk_bucket,minecraft:cooked_beef*32,minecraft:diamond_chestplate+protection=4,minecraft:diamond_helmet,minecraft:totem_of_undying,minecraft:ender_pearl*16,minecraft:snowball*16,minecraft:experience_bottle*16
```

## Review of segment w1

### 1. Track W's new sculk files pass rustfmt

After 6fb428f4 ("Minecraft sculk spreads from catalysts like vanilla").

- **Cause:** two files this track added failed the new-file rule (`git show :<path> | rustfmt --check` must print nothing): `crates/minecraft_terrain/src/vibrations.rs` (one assert of `the_nearest_catalyst_within_eight_takes_a_death`, from the catalyst commit) and vendored `world/src/level/sculk.rs` (its generator import, `shrieker_can_respond_as`, `catalyst_cursors` and several catalyst and shrieker test lines, from the shriekers-catalysts commits).
- **Changed:** both files formatted with rustfmt (the repo's `rustfmt.toml`); no code changed. Every other new file of the track already passed.
- **Tests:** none added (formatting only); `level::sculk::tests` (12) and `vibrations::tests` (11) pass, `check -p launcher` is green.
- **Test steps:** none (no behaviour changed).

### 2. A player who leaves a level with a container open is heard closing it at once

After e6ddd950 ("Track W's new sculk files pass rustfmt").

- **Cause:** all three ways out of a level (a friend leaving through `Server` in `minecraft_survival/src/server.rs`, `minecraft_players::leave_level` and the host's `leave_level` in `render_anim/src/minecraft_world.rs`) call `Entities::leave`, which drops the player's queue, before the openers post `container_close` through `Entities::game_event` (group vibrations-more, commit 1). `game_event` then made a new queue for the player who had left. Only an actor's queue goes into a tick input, so the close was never heard when the player left. It sat in that queue until the same player came back to the level, maybe hours later, and was then heard at the old chest. Events a player made in the frame before leaving were dropped with the queue as well.
- **Vanilla:** a player who leaves with a container open closes it then (`AbstractContainerMenu.removed` → `stopOpen` → `ContainerOpenersCounter.decrementOpeners`), and that posts `container_close` from that player at once.
- **Changed:** `Entities.left_game_events` (`crates/minecraft_survival/src/entities.rs`) holds the events of a player who has no queue in the level, and `Entities::leave` moves the leaving player's unsent events into it. The level's next tick input takes these events first, whoever ticks it, so no queue is made again for a player who left.
- **Tests:** `entities::tests::a_leaving_player_s_vibrations_go_out_with_the_next_tick_and_leave_no_queue` (gated on the 26.3 data: a step made before leaving and a close made after it both go out with the host's next tick, and the friend gets no queue). Checked to fail with the old routing.
- **Decisions:** when the leaving player was the last one in the level, the level stops ticking and the close goes out with its next tick, when someone comes back. That matches a level that froze at the moment the last player left.
- **EMITS_SOUNDS check (static, against D0 as merged into `minecraft-survival`; D0 is not merged into `track-w` yet):** D0's `play_movement` (`entities/src/world/emissions.rs`) drops only `Body.emission` when `Family::emits_sounds` is false (silverfish and endermites, vanilla's `MovementEmission.EVENTS`). Track W's `step`, `swim`, `splash`, `hit_ground` and `flap` go into `Body.game_events` in `Body::movement_emission` and `Body::move`, so they are posted anyway. D0's `body_mut` and `damage_state_mut` fall back to `families.core_mut`, so `EntityWorld::take_game_events` drains the registry families' bodies and their `entity_damage` as well. To confirm with a silverfish step test at the w2 merge; the warden's own steps come with W4.
- **Test steps (multiplayer, survival; a sculk sensor with a comparator out of it into a straight line of 15 redstone dust, a chest 5 blocks from the sensor):**
  1. Friend window: the friend opens the chest (the line shows 10). Wait 2 s. With the chest still open, the friend quits the world (Esc, then quit): the host's line shows 9 at once (the close). **w27_friend_leaves_chest**
  2. Wait 2 s, then the friend joins again and stays crouched where they appear: the line stays dark. Before this fix, the line showed 9 only now, when the friend came back.

```
IW4L_MINECRAFT_KIT=minecraft:sculk_sensor*4,minecraft:comparator*2,minecraft:redstone*32,minecraft:chest*2,minecraft:stone*64,minecraft:cooked_beef*16
```

## Final verification of segment w1

### 1. Track W's code passes clippy

After f509b23f ("A player who leaves a level with a container open is heard closing it at once").

- **Cause:** clippy on the touched crates (`minecraft_survival`, `minecraft_terrain`, `render_anim`, `render_gpu`, `minecraftoss-entities`, `minecraftoss-generator`, `minecraftoss-world`, `--all-targets`) warned on 11 lines the track added: `unwrap_or_default` (disallowed by the repo's `clippy.toml`) in `game_events.rs` (tag entries), `vibrations.rs` (`Listener::from_tag`'s selector), `warden_tracker.rs` (`try_warn`), `minecraft_survival/src/server.rs` (the saved tracker on join) and three `server::tests` helpers; `extend_with_drain` for the warden trackers and player effects in `run_tick`; `type_complexity` for `mob_actions::death_remains` (its experience gained the reward) and the `gun_tests::heard` helper.
- **Changed:** the fallbacks are written out (`unwrap_or("")`, `map_or_else(<Type>::default, ...)`, an `if let` for the saved tracker, `"no sensor"` in the test helper), `append` replaces `extend(drain(..))`, and the two return types get aliases (`DeathRemains`, `Heard`). No behaviour changed. The warnings left on these crates are older than the track (for example `draw_body_gear`'s nine arguments and the loot `unwrap_or_default` in `death_remains`).
- **Tests:** none added. `minecraft_terrain` 1459 (3 ignored), `minecraft_survival` 767 + 1, `render_anim` 215 pass; `check -p launcher` is green. The track's new files still pass `rustfmt --check`, and its added lines hold no comments.
- **Test steps:** none (no behaviour changed).

## Group warden-live

Segment w2 starts here, from `minecraft-survival` at 1e6e4bdd (D0, D, E1, E2, F, M1's first segment and segment w1 merged; protocol 9).

### 1. Minecraft wardens live in the entity world

After 1e6e4bdd ("Round 10 progress: M1's first segment merged, release 4 building").

- **Vanilla (javap on the 26.3 jar):**
  - `EntityTypes.WARDEN`: MONSTER, `sized(0.9, 2.9)` with no eye height of its own (0.85 of the height, 2.465), `clientTrackingRange(16)`, `fireImmune`, `notInPeaceful`. `Warden.createAttributes`: health 500, speed `0.30000001192092896`, knockback resistance 1, attack knockback 1.5, damage 30, follow range 24; `xpReward` 5; `nextStep` 0.55 past the walked distance; `getSoundVolume` 4; `playStepSound` `entity.warden.step` at 10; `getDefaultDimensions` 1.0 high while digging or emerging; `isInvulnerableTo` everything but `#bypasses_invulnerability` (`out_of_world`, `generic_kill`) then; `isPushable` false then; `removeWhenFarAway` false; `dampensVibrations` true; `canRide` false.
  - Navigation `Warden$1`: a `GroundPathNavigation` whose `PathFinder.distance` is `Node.distanceToXZ` (the only use of `distance` in `findPath` is a step's cost); `setCanFloat(true)`; maluses `UNPASSABLE_RAIL` 0, `DAMAGING` 8, `POWDER_SNOW` 8, `LAVA` 8, `FIRE` 0, `FIRE_IN_NEIGHBOR` 0; `getWalkTargetValue` 0.
  - `AngerManagement`: suspects with anger capped at 150 (`increaseAnger`; a new suspect also takes the anger kept under its UUID, uncapped), sorted angry first (80 or more), then players, then by anger; `highestAnger` is the largest anger met while sorting. `tick` runs when `tickCount % 20 == 0` (**the brief said `tickCount + id`; javap shows `tickCount` alone**), with the warden's `canTargetEntity` as filter: every 2 calls (first after 0 to 2 calls) loaded UUIDs become suspects again, UUID entries lose 1 (gone at 1), and a suspect loses 1 or, at 1, failing the filter or removed, leaves; one unloaded, carried off with a player or changed dimension keeps its anger less 1 under its UUID. Saved as `anger: {suspects: [{uuid, anger}]}` (`suspects` left out when empty, as the catalog's default `anger: {}` shows). `AngerLevel`: calm 0 (`ambient`, `listening`), agitated 40 (`agitated`, `listening_angry`), angry 80 (`angry`, `listening_angry`).
  - `Warden.increaseAngerAt(entity, amount, listening)`: nothing without AI or for an entity it cannot target; the dig cooldown refreshed; anger raised; a player made angry at, while the attack target was no player, clears the attack target; the listening sound by anger level at 10 unless roaring. `getActiveAnger` is the attack target's anger, else the highest. `canTargetEntity`: living, same level, not creative or spectator, not allied, not an armour stand, not a warden, not invulnerable, alive, inside the border.
  - `Warden$VibrationUser`: `EntityPositionSource` at the eye height it had when made (2.465), radius 16, `#warden_can_listen`; `canReceiveVibration`: AI, alive, no `VIBRATION_COOLDOWN`, not digging or emerging, inside the border, and a living source it can target. `onReceiveVibration`: `VIBRATION_COOLDOWN` 40, entity event 61, `entity.warden.tendril_clicks` at 5; with a projectile owner within 30 blocks: `RECENT_PROJECTILE` present means +35 at the owner (and the disturbance at the owner when it can target it), else +10; `RECENT_PROJECTILE` 100 for any owner; without an owner +35 at the source; when not angry, the disturbance is set if there was an owner, no active suspect, or the source is the active suspect. `Warden.tick` ticks the vibration data before the rest of its tick.
  - `WardenAi`: activities by priority EMERGE (`IS_EMERGING`), DIG (no `ROAR_TARGET`, no `DIG_COOLDOWN`), ROAR, FIGHT, INVESTIGATE (`DISTURBANCE_LOCATION`), SNIFF (`IS_SNIFFING`), IDLE; leaving one erases its memory. Core: `Swim(0.8)`, `SetWardenLookTarget` (the roar target's or the disturbance's block), `LookAtTargetSink(45, 90)`, `MoveToTargetSink`. `Emerging(134)` (pose, `entity.warden.emerge` 5; standing again at its end), `ForceUnmount`, `Digging(100)` (on the ground or in a liquid; on the ground the pose and `entity.warden.dig` 5, else `entity.warden.agitated` 5 and it is gone at once; discarded at its end), `GoToTargetLocation(DISTURBANCE_LOCATION, 2, 0.7)` (a block within 1 of it, unless already closer than 2), `TryToSniff` (no `SNIFF_COOLDOWN`, a nearest attackable, **no disturbance** — the brief said no walk target; it erases the walk target, cooldown 100 to 200, pose sniffing), `Sniffing(84)` (`entity.warden.sniff` 5; at its end standing, and the nearest attackable it can target gets +35 within 6 across and 20 up or down, and becomes the disturbance when none is set), IDLE's `RunOne` (no `IS_SNIFFING`: stroll at 0.5 within 10 and 7, weight 2, or nothing for 30 to 60 ticks, weight 1). `setDisturbanceLocation`: refused while it is angry at someone or has an attack target; else the dig cooldown refreshed, `SNIFF_COOLDOWN` 100, look target and disturbance for 100 ticks, the walk target erased. `setDigCooldown` only refreshes a cooldown that is there; `Warden.tick` calls it every tick for a persistent warden. `doPush`: with AI and no `TOUCH_COOLDOWN`, cooldown 20, +35 at the body and the disturbance at its block. `WardenEntitySensor`: of the living within the follow range around its box, the nearest player it can target, else the nearest other. A new brain starts in IDLE.
  - `finalizeSpawn`: `DIG_COOLDOWN` 1200; `TRIGGERED` also pose emerging, `IS_EMERGING` 134 and `entity.warden.agitated` at 5.
- **Changed:**
  - New vendored family `entities/src/world/families/deep_dark/wardens.rs` (key 44, registered in D0's `deep_dark` group file): the state (`Warden`: pose and its age, anger, client anger, brain, navigation, move control, body rotation, walk profile), `Family` (spawn, tick, invulnerable while buried, never far-away despawn, enemy, knockback resistance 1, voice `warden` at 4, its own step), its tick (`family_base_tick`, the AI step in vanilla's order: navigation, brain, anger every 20 ticks, activity, move and look control, jump, travel, its steps at 10 with the 0.55 step, body rotation, fall damage; then `family_after_move` and the touch), `EntityWorld::spawn_warden`, `spawn_emerging_warden`, `warden_can_hear` and `warden_hears` (the vibration user's two rules).
  - `wardens/anger.rs` (`Anger`, `AngerLevel`, `Seen`), `wardens/brain.rs` (`WardenBrain`: memories, activities, the behaviours above, `increase_anger_at`, `set_disturbance`).
  - Vendored `walk_path.rs`: `WalkProfile.distance_xz` makes a step's path cost its horizontal length (`Warden$1$1`).
  - Terrain: `vibrations.rs` keeps entity listeners (`Kind::Warden`, radius 16, `#warden_can_listen`, no neighbour-chunk rule) by entity id at the eye (`track`, `keep_tracked`, `tracked`; `Listener.entity`, `Listener.eye`), dispatched with the block listeners and ticked with them. `ServerSim::track_wardens` follows the entity world's wardens before each dispatch, reading a new warden's saved `listener` from its tag; `Hearing` asks `warden_can_hear` and calls `warden_hears` (a source after a load is found by its UUID), and writes a warden's listener into its tag (`mob_tags`, the `listener` key) when it changes.
  - `game_events::Source::Mob` gains `dampens` (a warden's own events reach no listener); a warden standing on a sculk sensor does not set it off (`SculkSensorBlock.stepOn` leaves wardens out).
  - Terrain `families/deep_dark/wardens.rs`: the warden's `TerrainFamily` (tracking range 16, shadow 0.9, nothing drawn until W6), registered in `families/deep_dark.rs`.
- **Tests:**
  - Vendored entities `wardens::anger::tests`: `anger_levels_match_vanilla`, `anger_rises_by_thirty_five_per_vibration_caps_at_150_and_falls_one_per_second`, `the_angriest_suspect_is_its_target`, `a_suspect_that_leaves_is_kept_by_uuid_and_comes_back_angry`.
  - `wardens::tests`: `a_warden_is_tall_strong_and_slow` (size, eye, health, step, attributes, traits, walk profile, buried size, loot table and 5 experience), `a_vibration_angers_it_by_thirty_five_and_it_listens_for_forty_ticks` (tendril event and sounds, listening sounds by level, creative players are no suspects), `a_far_projectile_angers_only_its_near_owner`, `it_walks_to_a_vibration_and_sniffs`, `sniffing_angers_it_at_the_nearest_within_six`, `a_summoned_warden_emerges_for_134_ticks_and_cannot_be_hurt`, `an_unbothered_warden_digs_away_after_1200_ticks`, `a_persistent_warden_never_digs`, `a_warden_on_peaceful_is_removed_but_never_far_away`, `a_cow_bumping_into_it_angers_it_and_draws_it_there`, `a_silverfish_and_a_warden_post_their_steps_and_swims`.
  - Terrain: `vibrations::tests::a_warden_listens_at_its_eye_sixteen_blocks_out_and_moves_its_ear`; `server::tests::a_warden_hears_a_player_s_step_and_keeps_its_listener_in_its_tag`, `a_cow_walking_by_a_sensor_sets_it_off_but_a_warden_never_does` (through `run_tick`); `game_events::tests::sneaking_spectating_and_dampening_sources_are_not_valid` also checks a dampening mob.
  - Full runs: minecraftoss-entities 546, minecraft_terrain 1639 (3 ignored; the eggs survey, the registry and the spawner cage tests take the warden automatically); `check -p launcher` green.
- **EMITS_SOUNDS check (asked by the w1 review):** `a_silverfish_and_a_warden_post_their_steps_and_swims` walks a silverfish (whose movement sounds D0's `Family::emits_sounds` drops) and a warden over stone into deep water: both post `step` and `swim` through `Body.game_events`.
- **Decisions:**
  - W5 keeps the roar and the fight: `SetRoarTarget`, `Roar`, the FIGHT behaviours, the hurt rule and the darkness pulse are not here, so an angry warden keeps investigating, sniffing and idling until W5. Its memories (`roar_target`, `attack_target`) and the ROAR and FIGHT activities are in place for W5. Sniffing's end anger is part of `Sniffing`, so it is here (with W5's test `sniffing_angers_it_at_the_nearest_within_six`).
  - Draws vanilla takes from the level's random (behaviour lengths, the look's 45 to 90, the 0 to 60 idle wait, the sniff cooldown, `GoToTargetLocation`'s offset, the shuffle of `RunOne`) come from the warden's own random, as the fork's other brains do. A loaded or new anger starts its UUID conversion count at 0 (vanilla: 0 to 2 from a thread-local random).
  - A warden's vibrations are dispatched after the mobs, like every listener (group vibrations), so its brain answers one tick after vanilla, whose warden ticks its listener at the start of its own tick. Its ripple flies to where its eyes were when the vibration was picked (vanilla's particle follows the warden).
  - The listener data stays in `minecraft_terrain` (the entities crate cannot see the selector): `vibrations.rs` keeps it by entity id, and its NBT goes into the `listener` key of the warden's tag in `ServerSim.mob_tags`, written when a vibration is picked or arrives (W2's rule), read when the warden is first tracked.
  - Touches (`doPush`) are found in the family's file from the bodies overlapping its box after its move (players, then mobs; the first one counts, as the 20-tick cooldown stops the rest), not in the shared `pushing.rs`. A creative player still draws it there (no anger: it cannot target them).
  - A warden made to emerge starts in its EMERGE activity, so it climbs out where it appeared. Vanilla's new brain starts in IDLE, whose stroll could set a walk target on the first tick, which `Emerging` waits for.
  - The warden is registered on the terrain side with tracking range 16 already (`TerrainFamily` needs it) and draws nothing until W6 (`warden-render`): in game it is invisible until then.
- **Vendored files:** added `entities/src/world/families/deep_dark/wardens.rs`, `wardens/anger.rs`, `wardens/brain.rs`, `wardens/tests.rs`; patched `entities/src/walk_path.rs` (D0's `deep_dark.rs` group file is the fork's own).
- **Test steps:** with commit 2 (the egg and the summon come there).

### 2. Minecraft wardens are saved, hatch from eggs and dig out when shriekers summon them

After e3495a40 ("Minecraft wardens live in the entity world").

- **Vanilla (javap on the 26.3 jar):**
  - `SculkShriekerBlockEntity.trySummonWarden`: at warning level 4, `SpawnUtil.trySpawnMob(WARDEN, TRIGGERED, level, pos, 20, 5, 6, ON_TOP_OF_COLLIDER, false)`; `tryRespond` plays the reply sound only when that fails, then the darkness within 40.
  - `SpawnUtil.trySpawnMob`: per try `Mth.randomBetweenInclusive(-5, 5)` for x and z from the level's random, the column searched from 6 above down through 13 blocks (`moveToPossibleSpawnPosition`: the strategy is asked of the block below and the one above), then `EntityType.create` (yaw `wrapDegrees(nextFloat × 360)`, then `finalizeSpawn`, which for `TRIGGERED` sets the pose and `IS_EMERGING` and plays `entity.warden.agitated` at 5, then `Mob.finalizeSpawn`'s follow-range triangle and left-handed roll), `checkSpawnRules` and `checkSpawnObstruction`; a mob that fails them is discarded (its growl has already been heard). `ON_TOP_OF_COLLIDER`: the block above has no collision and the block below a full top face (`LEGACY_IRON_GOLEM` is the golem's rule).
  - `Monster.checkMonsterSpawnRules`: not peaceful, `isDarkEnoughToSpawn` unless the reason ignores light (only `TRIAL_SPAWNER`), a valid block below unless a spawner. `Warden.checkSpawnObstruction`: the monster check (no liquid, no entity in the way) and no collision of its full size.
  - `Warden.addAdditionalSaveData`: `anger` and `listener`; the brain saves the warden's `Unit` memories with their time to live (`dig_cooldown`, `is_emerging`, `is_sniffing`, `recent_projectile`, `sniff_cooldown`, `touch_cooldown`, `vibration_cooldown`, and W5's roar and sonic boom ones). The pose is not saved. A spawn egg (`SPAWN_ITEM_USE`) and `/summon` (`COMMAND`) also call `finalizeSpawn`: `DIG_COOLDOWN` 1200, no emerging.
- **Changed:**
  - Vendored `golem_spawn.rs`: `Strategy` (`LegacyIronGolem`, `OnTopOfCollider`), the `SpawnSite` trait (the world and the level's range draws) and `try_spawn_mob(site, start, attempts, xz, y, strategy, create)`; `try_spawn_golem` runs on it with its own checks in `create`, drawing as before.
  - Vendored world: the warden's spawn row in D0's `deep_dark` group file (0.9 by 2.9, no placement restriction, fire immune, monster, follow range 24, `Monster.checkMonsterSpawnRules` as its rule, and a finalize writing `Brain.memories.minecraft:dig_cooldown` with 1200 ticks); `CreatureSpawns::triggered_may_spawn` (the row's rule as a natural spawn, then no liquid, no entity and no block in the box) and `Level::triggered_spawn_ok`, the spawn rules `ServerSim` hands to `try_spawn_mob`.
  - `ServerSim::summon_warden(pos, sounds)`: 20 tries within 5 across and 6 up or down on top of a collider; each try draws the yaw, the follow-range triangle and the left-handed roll from the level's random and asks `triggered_spawn_ok`; a refused try growls (`entity.warden.agitated` at 5, hostile) where it would have stood; the one that passes becomes an emerging warden with a UUID from the level's entity random and the catalog's default tag. `respond_shriekers` calls it at warning level 4 and plays the reply only when it fails.
  - The warden's `TerrainFamily`: reads and writes `anger` (vanilla's codec) and its `Brain` memories, starts its body turned its saved way; `VOLATILE` holds `anger` and `listener` (the server keeps the `listener`, commit 1); `FamilyShow` values 0 to 2 are the pose, the ticks since it began and the client anger (`getActiveAnger` as synced every 20 ticks), which `show_on` puts on the friend's copy (resized while buried).
- **Tests:**
  - Vendored entities `golem_spawn::tests::on_top_of_collider_needs_a_full_top_face`, `a_golem_still_spawns_where_it_did` (the draws in vanilla's order, first try and, blocked once, the second).
  - Vendored world `natural_spawner::families::deep_dark::tests::a_summoned_warden_waits_to_dig_and_a_triggered_one_needs_the_dark` (the row's size, a summon's dig cooldown, dark yes, peaceful no, inside the floor no, daylight never; gated on the 26.3 data).
  - Terrain `families::deep_dark::wardens::tests::a_warden_saves_and_loads_its_anger_and_listener` (UUID suspects back as they came, memories with and without their time to live, the listener kept in the tag, a known suspect saved first), `a_friend_sees_a_warden_emerge` (look tag without `anger`, `listener` and `Brain`; tracking range 16; pose, its age and the anger through `FamilyShow`).
  - Terrain `server::tests`: `a_triggered_warden_needs_darkness_and_room` (noon: nothing and 20 growls; midnight: one within 5 and on the floor, emerging, its UUID in its tag, its own growl; under a low roof: nothing), `a_warden_from_an_egg_or_a_command_stands_at_once_and_waits_a_minute_to_dig` (not on peaceful), `the_fourth_warning_summons_a_warden_in_the_dark_within_five_and_six` (moved here from W7: no reply, darkness still given), `a_summoned_warden_nearby_stops_the_next_warning` (it does not even shriek). W7's `the_fourth_warning_plays_the_angry_sound_while_no_warden_can_come` now runs at noon and counts the 20 growls before the reply.
  - The eggs survey and the registry tests take the warden by themselves (the warden egg hatches a warden).
  - Full runs: minecraftoss-entities 548, minecraftoss-world 132, minecraft_terrain 1645 (3 ignored); `check -p launcher` green.
- **Decisions:**
  - The summon replays vanilla's growl for every warden made and sent away (`finalizeSpawn` plays it inside `EntityType.create`), so a summon that fails in daylight or by torches growls up to 20 times before the angry reply.
  - The spawn rules draw from the level's random (vanilla's `checkSpawnRules` passes the new mob's own random). The liquid and entity checks use the warden's full height (vanilla's use the 1-high box it has while emerging); the block collision check uses its full size, as vanilla's does.
  - The follow-range triangle and the left-handed roll are drawn for each try but not applied: the fork's warden keeps follow range 24 (eggs and `/summon` write the bonus into the tag as for any mob; the family reads 24).
  - Saved memories are the seven the warden uses so far; W5 adds the roar and sonic boom ones. Unknown memories in a loaded `Brain` are dropped on save, as for the fork's other brains.
  - The warden stays invisible until W6, so the steps below are for the segment's release, after `warden-render`.
- **Vendored files:** patched `entities/src/golem_spawn.rs`, `world/src/natural_spawner/spawning.rs`, `world/src/level/spawning.rs` (D0's `deep_dark.rs` row file is the fork's own).
- **Test steps (survival, Normal; flat open ground at dusk or by day unless a step says otherwise; the warden is drawn from W6 on, so run these with the segment's release):**
  1. Use a warden egg on the ground: a warden stands there at once (an egg's warden does not dig out and needs no darkness). **w11_warden_egg**
  2. Stand still 10 blocks away: it wanders and sniffs (head up, a long sniffing sound). Crouch (so the throw is not heard) and throw a snowball beside it: its tendrils click, it walks to where the snowball landed and sniffs. Walk around near it: each of your steps clicks its tendrils and it walks to where you were. Crouch-walk away: your steps make no ripples (it may still sniff you out when you are within 6 blocks). **w12_warden_investigates**
  3. Stay out of its way for a minute (60 s with nothing it can hear): it digs into the floor with a digging sound and is gone, leaving no loot.
  4. Hatch a cow near a warden: the cow's steps make its tendrils click and it walks to the cow; a cow bumping into it angers it at once (it fights from W5 on).
  5. `/summon minecraft:warden ~ ~ ~5`, walk around it until it growls angrily (its ambient sound changes to a deep angry one), save and quit, load: it is still there and still growls angrily.
  6. At midnight on open ground with no torches or lava within 16 blocks, `/setblock ~ ~-1 ~3 minecraft:sculk_shrieker[can_summon=true]` and warn yourself four times (step on it, step off, 10 s apart): on the fourth a warden climbs out of the ground within 5 blocks over about 7 s, growling; no angry reply sound. **w24_warden_summoned** In daylight the same fourth warning plays a burst of growls and the angry reply, and no warden comes.
  7. With that warden still near, wait 10 s and step on the shrieker again: it stays silent.
  8. Friend window: the friend sees the warden stand, walk and sniff in the same place (and climb out of the ground after a summon); the friend's steps make its tendrils click and it walks to the friend. **w13_friend_warden**

```
IW4L_MINECRAFT_KIT=minecraft:warden_spawn_egg*4,minecraft:cow_spawn_egg*8,minecraft:zombie_spawn_egg*4,minecraft:snowball*16,minecraft:white_wool*64,minecraft:torch*64,minecraft:diamond_sword+sharpness=5,minecraft:shield,minecraft:bow+power=5,minecraft:arrow*64,minecraft:milk_bucket,minecraft:cooked_beef*32,minecraft:diamond_chestplate+protection=4,minecraft:diamond_helmet,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:totem_of_undying,minecraft:golden_apple*8,minecraft:deepslate*64,minecraft:cobblestone*64,minecraft:ender_pearl*16
```

## Group warden-fight

### 1. Minecraft wardens roar, sniff and hit hard enough to knock shields down

After f07e78d2 ("Minecraft wardens are saved, hatch from eggs and dig out when shriekers summon them").

- **Vanilla (javap on the 26.3 jar):**
  - `SetRoarTarget` (the first behaviour of IDLE, INVESTIGATE and SNIFF): with no `ROAR_TARGET` and no `ATTACK_TARGET`, `getEntityAngryAt` (the top suspect while the active anger is 80 or more), filtered by `canTargetEntity`, becomes the `ROAR_TARGET`, and `CANT_REACH_WALK_TARGET_SINCE` is erased.
  - `Roar` (a `Behavior` of `ROAR_DURATION` 84 whose `canStillUse` is always true, so it runs from its start tick through its 84th): start: `ROAR_SOUND_DELAY` 25, the walk target erased, `lookAtEntity`, pose `ROARING`, `increaseAngerAt(target, 20, false)`; tick: with neither `ROAR_SOUND_DELAY` nor `ROAR_SOUND_COOLDOWN`, `ROAR_SOUND_COOLDOWN` 84 - 25 and `entity.warden.roar` at 3, pitch 1; stop: standing again if still roaring, `setAttackTarget(ROAR_TARGET)`, `ROAR_TARGET` erased.
  - `Warden.setAttackTarget`: `ROAR_TARGET` erased, `ATTACK_TARGET` set, `CANT_REACH_WALK_TARGET_SINCE` erased, `SonicBoom.setCooldown(200)`.
  - FIGHT (`ATTACK_TARGET` erased on leaving it): `DIG_COOLDOWN_SETTER`; `StopAttackingIfTargetInvalid` (stops when the target cannot be attacked (`canAttack` is `canTargetEntity`), is dead or elsewhere, or the warden is not angry; then `clearAnger` when it cannot target it, and `setDigCooldown`; it never tires of reaching); `SetEntityLookTarget(isTarget, follow range 24)`; `SetWalkTargetFromAttackTargetIfTargetOutOfReach(1.2)`; `SonicBoom` (commit 2); `MeleeAttack(18)` (target seen and within melee reach, no `ATTACK_COOLING_DOWN`: look, swing, `doHurtTarget`, cooling down 18).
  - "Seen" is the sensor's list (the living in its follow-range box, nearest first) filtered by `Sensor.TARGET_CONDITIONS`: within 16 and in line of sight.
  - `Warden.doHurtTarget`: entity event 4, `entity.warden.attack_impact` at 10 with its voice pitch, `SonicBoom.setCooldown(40)`, then `Mob.doHurtTarget`: 30 (scaled for players like any mob hit), and when it hurt, `causeExtraKnockback` (1.5 / 2 along its facing, its own motion ×0.6 across). `getSecondsToDisableBlocking` 5.
  - `Warden.hurtServer`: after the hurt, unless it has no AI or digs or climbs out: `increaseAngerAt(attacker, ANGRY 80 + ON_HURT_ANGER_BOOST 20, false)`; with no `ATTACK_TARGET`, a living attacker becomes it through `setAttackTarget` when the hit was direct (`DamageSource.isDirect`) or the attacker is closer than 5.
  - `MemorySlot.tick`: an expired value (time to live 0 or less) is cleared, else its time to live drops by one. A 25-tick delay set on the roar's first tick therefore ends on its 26th, and an 18-tick cooling down allows the next swing 19 ticks later.
  - Saved memories (`MemoryModuleType` registered with a codec): `roar_sound_cooldown`, `roar_sound_delay` and `sonic_boom_cooldown` join the seven of group warden-live; `attack_cooling_down` has no codec.
- **Changed:**
  - Vendored `wardens/brain.rs`: memories `roar_sound_delay`, `roar_sound_cooldown`, `attack_cooling_down`, `sonic_boom_cooldown` and `nearest_visible` (the sensor's seen list, line of sight checked when the sensor runs); `set_roar_target` in IDLE, INVESTIGATE and SNIFF; `Roar` as a running behaviour (`roar_until`: start, tick, stop); FIGHT (`fight`); `set_attack_target`; `hurt_by` (the hurt rule).
  - Vendored `wardens.rs`: `Warden.hurts` (each hurt since its last tick: the attacker and whether the hit was direct), answered at the start of its next tick before its brain; `strike` (entity event 4, `PlayerHitKind::Melee` with `disable_shield_seconds` 5 and the extra knockback as `throw` through `family_strike`; a mob it hurt also gets the extra knockback; its own motion ×0.6 across); `attack_throw`.
  - Terrain `families/deep_dark/wardens.rs`: the three new saved memories.
  - The listening and ambient sounds by anger level (calm, agitated, angry) came with group warden-live.
- **Tests:**
  - Vendored entities `wardens::tests`: `an_angry_warden_roars_eighty_four_ticks_then_attacks` (85 ticks in the roaring pose, +20 anger, one roar at 3 on the 26th tick, then the attack target with the 200-tick boom cooldown, then FIGHT), `its_melee_hits_thirty_and_disables_a_shield_for_five_seconds` (30 from it, `Melee` with 5 s and the extra knockback, entity event 4, the impact at 10, the boom cooldown set to 40 although 199 were left, the next swing 19 ticks later), `a_hit_angers_it_by_a_hundred_and_only_a_close_or_direct_one_starts_the_fight` (a sword at 2 blocks and a mob's bite from 8 make it fight at once; an arrow from 12 and an MW2 bullet from 20 only anger it by 100, so it roars first; a hit while it climbs out angers nothing), `an_angry_warden_roars_at_a_cow_and_beats_it_to_death`.
  - Terrain `families::deep_dark::wardens::tests::a_warden_keeps_its_roar_and_boom_cooldowns_in_its_brain`.
  - Difficulty scaling of the 30 (16 Easy, 30 Normal, 45 Hard) is the survival side's for every mob hit, covered by `entities::tests::mob_hits_on_the_player_scale_with_difficulty_like_vanilla`; the shield's 100 ticks by D0's `a_typed_hit_skips_armour_and_shields_when_told_and_gives_its_effect` (`offhand::disable_ticks`).
  - Full runs: minecraftoss-entities 552; the terrain warden tests; `check -p launcher` green.
- **Decisions:**
  - Only a mob's melee (`DamageSourceKind::MobAttack`) counts as a direct hit. A player's attacks reach a family as `PlayerAttack` both for a sword and for an MW2 bullet (`Entities::shoot` sends a `PlayerAttack` from the gun), so a player becomes the target at once only within 5 blocks. A sword always is (reach 3); an MW2 bullet from farther counts like an arrow (anger 100, then the roar, then the fight), as the brief asks. Vanilla's player melee is direct at any distance; within reach the result is the same.
  - The hurt rule is answered at the start of the warden's next tick, before its brain (the hurt hook cannot see the other living, which the anger needs), which is when vanilla's brain next acts on it. A hurt is noted whether or not it landed (vanilla angers after `super.hurtServer` whatever it returned), but not while the warden digs or climbs out or has no AI.
  - The melee on a player puts vanilla's extra knockback in `PlayerHitKind::Melee`'s `throw` (0.75 along its facing, lift capped at 0.4), as the ravager does. The survival side's `MeleePush.landing` also throws a shielded player (written for the hoglin's throw), while vanilla's extra knockback needs a hurt, so a raised shield still pushes the player back. `MeleePush` is shared survival code and stays as it is. The warden's own motion slows after every swing at a player, since the entities side does not know whether the player was hurt (also as the ravager does); after a swing at a mob, only when the mob was hurt.
  - A swing at a mob goes through `family_strike` (`mob_hits_mob`: damage type `mob_attack` with the victim's armour, the usual 0.4 knockback), then the extra knockback through `knock_back`.
  - Draws vanilla takes from the level's random for behaviour lengths (`Behavior.tryStart`'s `nextInt(1)` for the roar) are not taken: fixed-length behaviours draw nothing that changes the outcome.
- **Vendored files:** patched (added in group warden-live) `entities/src/world/families/deep_dark/wardens.rs`, `wardens/brain.rs`, `wardens/tests.rs`.
- **Test steps:** with commit 2.

## Combined kit

One kit for the track's main steps (27 stacks): darkness (w01, w03; step 2 needs a cave or torches), sensors (w04 to w06), block, player and gun vibrations (w07 to w10), amethyst (w08), shriekers (w23, w26; the summoning shrieker comes from `/setblock`), catalysts (w25) and the friend leaving with a chest open (w27). The steps that need other items (lava and powder snow in darkness step 5, the trident, trapdoors, gates, plates, the bell, candles, pistons, sheep, shears, horses, the totem) use their commit's own kit line above.

```
IW4L_MINECRAFT_KIT=minecraft:sculk_sensor*16,minecraft:calibrated_sculk_sensor*4,minecraft:sculk_shrieker*8,minecraft:sculk_catalyst*4,minecraft:amethyst_block*4,minecraft:comparator*4,minecraft:redstone*64,minecraft:redstone_lamp*4,minecraft:lever*4,minecraft:white_wool*64,minecraft:white_carpet*16,minecraft:oak_door*2,minecraft:oak_button*4,minecraft:chest*2,minecraft:water_bucket,minecraft:note_block*2,minecraft:snowball*16,minecraft:tnt*4,minecraft:flint_and_steel,minecraft:cow_spawn_egg*16,minecraft:chicken_spawn_egg*4,minecraft:apple*16,minecraft:milk_bucket,minecraft:diamond_helmet,minecraft:diamond_sword+sharpness=5,minecraft:experience_bottle*16,minecraft:cobblestone*64
```

## Protocol

Group darkness changes no `ToClient` or `ToServer` message and no codec: `minecraft_net::PROTOCOL` stays 6.

Group vibrations, commit 1: no message or codec change (`TickInput` and the commands stay on the host).

Group vibrations, commit 2: `EntitySnapshot` (sent to friends inside `ToClient::Bursts` and the state frames) gains `vibrations: Vec<VibrationView>`. The snapshot is serde-encoded, so a friend on an older build cannot read the new one: `minecraft_net::PROTOCOL` must go up at the release (not bumped here).

Group vibrations, commit 3: no message or codec change.

Group vibrations-more, commit 1: no message or codec change.

Group vibrations-more, commit 2: no message or codec change (`Body` and `DamageState` stay on the host).

Group vibrations-more, commit 3: no message or codec change.

Group vibrations-more, commit 4: no message or codec change (`PlayerEvent` and `TickInput` stay on the host).

Group shriekers-catalysts, commit 1: no message or codec change. The shriek reaches friends as the existing `ToClient::LevelEvent` (3007), the reply as a sound, and the darkness in the friend's `PlayerView`; the trackers and `Output.player_effects` stay on the host.

Group shriekers-catalysts, commit 2: `EntitySnapshot` (inside `ToClient::Bursts` and the state frames) gains `sculk_souls: Vec<DVec3>`; it rides in the protocol bump group vibrations already needs at the release. `Command::AwardExperience` stays on the host.

Group shriekers-catalysts, commit 3: no message or codec change (the 3006 events travel as `ToClient::LevelEvent`, the cursors in the catalyst's block entity).

Review of segment w1: no message or codec change.

Final verification of segment w1: no message or codec change.

Group warden-live, commit 1: no message or codec change (the warden's state, `game_events::Source` and the listeners stay on the host; `MobShow` is unchanged).

Group warden-live, commit 2: no message or codec change. The warden's pose, its age and its client anger ride in D0's `MobShow.family` (`FamilyShow` values 0 to 2), which friends already decode; its look tag is an entity tag as before.

Group warden-fight, commit 1: no message or codec change (the roar is the pose already in `FamilyShow`; the hits are host-side `PlayerHit`s).

## State doc changes

- `docs/MULTIPLAYER.md`: a friend's `PlayerView` is also sent again when an effect is given again with a longer duration (`views::lengthened`), and the friend restarts its effect count-down for every view it receives, also one equal to the last (`RemoteWorld.player_views`).
- `docs/MINECRAFT-SURVIVAL.md` (group vibrations-more): sculk sensors and calibrated sensors hear vanilla's game events from blocks used (doors, trapdoors, gates, levers, buttons, plates, note blocks, bells, containers, buckets, TNT, fire, pistons, dispensers, lecterns, item frames, wind charges), from players and mobs (eating, drinking, armour, riding, gliding, shearing, teleports, hurts, deaths, chicken flaps and eggs, spawn eggs) and from MW2 guns (the shot, unless crouched or prone, and where the bullet lands); an amethyst block beside a sensor resonates its frequency. Not yet: composters, cakes, jukeboxes, tripwires, beehives, bounces, bats' flaps, mob equipment, players and mobs on pressure plates.
- `docs/round10/decisions.md` (group vibrations-more): MW2 bullets are projectiles for vibrations: crouched or prone shots are not heard, the impact on a block or a mob always is (from the bullet, with the shooter as projectile owner), and a bullet that hits nothing lands nowhere.
- `docs/MINECRAFT-SURVIVAL.md` (group shriekers-catalysts): sculk shriekers shriek when a player stands on one (or rides a mob onto it) or when a sculk sensor within 8 blocks clicks for a player or a player's projectile. A shrieker that can summon (generated ones in ancient cities, or `can_summon=true`) only shrieks when it can warn: each player's warden warning level (saved in the player file, shared with the players within 16 blocks) rises by one with a 10 s cooldown and drops one every 10 minutes; after the 4.5 s shriek a warden sound answers by level (closer and angrier) and players within 40 blocks get darkness. Peaceful or `spawn_wardens false`: it shrieks, nothing answers. Until the warden groups (W4) a fourth warning only plays the angry sound.
- `docs/MINECRAFT-SURVIVAL.md` (group shriekers-catalysts, commit 2): a sculk catalyst takes the death of any mob or player within 8 blocks (the nearest catalyst; no player kill needed): it blooms with soul wisps and the death drops no experience orbs.
- `docs/MINECRAFT-SURVIVAL.md` (group shriekers-catalysts, commit 3): a catalyst that takes a death turns its experience into charge that spreads sculk and sculk veins over the replaceable ground around it, saved with the catalyst; larger charges grow sculk sensors and shriekers (which cannot summon) at least 4 blocks from it.
- `docs/MULTIPLAYER.md` (review of segment w1): the game events (vibrations) of a player who has left a level, such as the container close of a friend who quits with a chest open, go out with that level's next tick (`Entities.left_game_events`).
- `docs/MINECRAFT-SURVIVAL.md` (group warden-live, commit 1): wardens live in the entity world: they hear vibrations within 16 blocks of their eyes (not their own: a warden makes no vibration and sets off no sensor), grow angry at what made them (35 a vibration, 10 or 35 at a shooter within 30 blocks, 35 a touch, up to 150, one less each second), walk to where they heard something and sniff out players and mobs nearby, dig into the ground after a minute with nothing heard (a persistent one never does), and cannot be hurt while they dig or climb out. They roar and fight from W5 and are drawn from W6.
- `docs/MINECRAFT-SURVIVAL.md` (group warden-live, commit 2): wardens are saved with their anger and the vibration they are hearing, hatch from warden eggs and come from `/summon` standing (no darkness needed; they dig away after a minute with nothing heard). A summoning shrieker's fourth warning makes a warden climb out of the ground within 5 blocks, but only where a monster could spawn (dark enough, room for its 2.9 blocks); otherwise it growls and the angry reply plays. A warden near a summoning shrieker keeps it from warning or shrieking.
- `docs/MINECRAFT-SURVIVAL.md` (group warden-fight, commit 1): a warden angry at a player or a mob (80 or more) roars at it for about 4 s and then fights it: a swing of 30 (16 on Easy, 45 on Hard) with a heavy knockback that knocks a raised shield down for 5 s, one swing a second at most. Whatever hurts a warden makes it angry (100); a hit from within 5 blocks or a mob's bite makes it fight at once, an arrow or an MW2 bullet from farther makes it roar and come.
