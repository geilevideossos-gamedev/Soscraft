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

## Protocol

Group darkness changes no `ToClient` or `ToServer` message and no codec: `minecraft_net::PROTOCOL` stays 6.

Group vibrations, commit 1: no message or codec change (`TickInput` and the commands stay on the host).

Group vibrations, commit 2: `EntitySnapshot` (sent to friends inside `ToClient::Bursts` and the state frames) gains `vibrations: Vec<VibrationView>`. The snapshot is serde-encoded, so a friend on an older build cannot read the new one: `minecraft_net::PROTOCOL` must go up at the release (not bumped here).

## State doc changes

- `docs/MULTIPLAYER.md`: a friend's `PlayerView` is also sent again when an effect is given again with a longer duration (`views::lengthened`), and the friend restarts its effect count-down for every view it receives, also one equal to the last (`RemoteWorld.player_views`).
