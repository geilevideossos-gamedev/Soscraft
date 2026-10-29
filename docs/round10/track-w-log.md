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

## Protocol

Group darkness changes no `ToClient` or `ToServer` message and no codec: `minecraft_net::PROTOCOL` stays 6.

## State doc changes

- `docs/MULTIPLAYER.md`: a friend's `PlayerView` is also sent again when an effect is given again with a longer duration (`views::lengthened`), and the friend restarts its effect count-down for every view it receives, also one equal to the last (`RemoteWorld.player_views`).
