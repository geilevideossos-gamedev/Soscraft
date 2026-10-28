# Track D log: trial chambers, vaults, lodestones and breezes (round 10)

Branch `track-d`, worktree `C:\Dev\mc-wt\track-d`, target dir `D:\iw4l-wt\track-d`. Brief: `docs/round10/track-d.md`. Round 9's history: `docs/round9/track-d.md` and `docs/round9/track-d-log.md` (which stays as it is).

A commit can't name its own hash, so each entry names the commit before it. Its own hash is in `git log` under its subject.

Screenshots of this track are numbered from 301.

## Group d5-review

Brief D11: the review of round 9's partial d5 run (keyframes 81f6f353, withers d4f932de..ef9ad9de, wind charges f142f9c1).

### 1. Track D pins that a wind charge does not push a player flying in creative

After 382bf959.

- **Vanilla (javap, 26.3):** `SimpleExplosionDamageCalculator.getKnockbackMultiplier` returns 0 for a `Player` whose `getAbilities().flying` is set, and the calculator's own multiplier (1.22 for a player's wind charge, the default 1.0 for a breeze's) otherwise. Wind charges and breeze charges both explode with that calculator. Gliding with an elytra is not `abilities.flying`, so a gliding player is pushed.
- **Checked (every push path to a player, at 382bf959):**
  - Wind charges: `minecraft_terrain/src/wind.rs` `burst` returns `(player, push)` for every player in reach; `ServerSim::wind_burst` (`server.rs:1889-1896`) adds them to `players_pushed`; `run_tick` moves them to `Output.players_pushed` (`server.rs:4282`).
  - The other producers of `Output.players_pushed`: the entity world's player pushes (`server.rs:4283`, `EntityWorld::take_player_pushes`: ravagers and the other mob pushes), shield blocks (`server.rs:4133`, `Command::ShieldBlock`) and the dragon (`server.rs:2642`, `events.player_pushes`).
  - `minecraft_survival/src/entities.rs:301-304` routes every `players_pushed` entry to that player's `PlayerQueue.out.pushes`; the blast pushes of mob hits (creepers, fireballs, TNT) are added at `entities.rs:329` (`explosion_push`). Nothing else writes `out.pushes`.
  - `render_anim/src/minecraft_players.rs` `struck` applies `out.pushes` and the knockback of hits (`landed`) only through the one `pushed` check, for the host (`minecraft_world.rs:1245`) and for every friend (`step_friend`, `minecraft_players.rs:1661`: the host runs every player's step).
  - The level's own `wind_burst` (vendored `world/src/level/explosion.rs:349`) pushes only level entities (items, falling blocks), never players. `Body::push` (`actions.rs:112`) is only the riptide launch.
  - `MC_FLYING` is set only by creative and spectator flight (`movement_iw4/src/fly.rs:60-79`, `sim/src/frame.rs:529-539`); elytra gliding is `MC_GLIDING`. So the check matches vanilla's `abilities.flying`.
  - Result: no path bypasses the check. Release 1's known limit "A player flying in creative is still pushed by a burst" was wrong: the check is in the per-player step since cb4251e9, before f142f9c1.
- **Changed:** `render_anim/src/minecraft_players.rs`: the check moved unchanged into the small function `takes_pushes(alive, mode, pm_flags)`, so it can be tested.
- **Tests:** `render_anim` `minecraft_players::tests::a_wind_charge_does_not_push_a_player_flying_in_creative`: a player's burst one block away pushes the player out and up; `takes_pushes` keeps the push for a walking player, a creative player on the ground and a player gliding with an elytra, and drops it for a creative player flying, a spectator and a dead player.
- **Decisions:** none beyond the check; no code path changed.
- **Test steps (new world `IW4L_MINECRAFT_WORLD=trial10`, survival, Normal, by day, flat open ground):**
  1. `/gamemode creative`, double-tap jump to fly 5 blocks up, and throw a wind charge at a block right below you: the burst does not move you (**301_wind_flying**).
  2. Stop flying (double-tap jump), land, and throw one at your feet: it launches you. `/gamemode survival`.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:wind_charge*16,minecraft:cobblestone*64
```

## Protocol

No message of this track has changed yet. `minecraft_net::PROTOCOL` stays 6.

## State doc changes

- Release 1's known limits (`docs/round10/release-1-tests.md`, Wind charges) and the round 9 log (group breezes, commit 1, and "Left for later"): drop "A player flying in creative is still pushed by a burst". A player flying in creative was never pushed by a burst: every push goes through the flying check (group d5-review, commit 1).

## Vendored files

None patched or added yet in round 10.
