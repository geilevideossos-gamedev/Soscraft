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

## Protocol

Group darkness changes no `ToClient` or `ToServer` message and no codec: `minecraft_net::PROTOCOL` stays 6.

## State doc changes

- `docs/MULTIPLAYER.md`: a friend's `PlayerView` is also sent again when an effect is given again with a longer duration (`views::lengthened`), and the friend restarts its effect count-down for every view it receives, also one equal to the last (`RemoteWorld.player_views`).
