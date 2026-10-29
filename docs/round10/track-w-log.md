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

## Protocol

Group darkness changes no `ToClient` or `ToServer` message and no codec: `minecraft_net::PROTOCOL` stays 6.

## State doc changes

- `docs/MULTIPLAYER.md`: a friend's `PlayerView` is also sent again when an effect is given again with a longer duration (`views::lengthened`), and the friend restarts its effect count-down for every view it receives, also one equal to the last (`RemoteWorld.player_views`).
