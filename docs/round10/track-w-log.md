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
  7. Friend window: the friend eats next to the host's sensor: the host's line shows 8. **w08_friend_eats**

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

## Protocol

Group darkness changes no `ToClient` or `ToServer` message and no codec: `minecraft_net::PROTOCOL` stays 6.

Group vibrations, commit 1: no message or codec change (`TickInput` and the commands stay on the host).

Group vibrations, commit 2: `EntitySnapshot` (sent to friends inside `ToClient::Bursts` and the state frames) gains `vibrations: Vec<VibrationView>`. The snapshot is serde-encoded, so a friend on an older build cannot read the new one: `minecraft_net::PROTOCOL` must go up at the release (not bumped here).

Group vibrations, commit 3: no message or codec change.

Group vibrations-more, commit 1: no message or codec change.

Group vibrations-more, commit 2: no message or codec change (`Body` and `DamageState` stay on the host).

Group vibrations-more, commit 3: no message or codec change.

## State doc changes

- `docs/MULTIPLAYER.md`: a friend's `PlayerView` is also sent again when an effect is given again with a longer duration (`views::lengthened`), and the friend restarts its effect count-down for every view it receives, also one equal to the last (`RemoteWorld.player_views`).
