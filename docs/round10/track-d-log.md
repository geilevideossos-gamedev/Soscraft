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

### 2. Minecraft fireballs and wither skulls keep their shooter through a save

After bad8dd84.

- **Vanilla (javap, 26.3):**
  - `Projectile.addAdditionalSaveData` writes `Owner` with `EntityReference.store` (the owner's UUID as an int array), `LeftOwner` when set and `HasBeenShot`; `readAdditionalSaveData` reads `Owner` back with `EntityReference.read` as a UUID only.
  - `Projectile.getOwner` resolves that reference when asked (`EntityReference.getEntity(ref, level)`: `Level.getPlayerInAnyDimension` for a player, `Level.getEntityInAnyDimension` otherwise). An owner that is not loaded is `null`, and the projectile flies on without one. `AbstractHurtingProjectile.tick` asks every tick, so the owner is found as soon as it loads.
  - So a wither skull saved in flight still hits for 8 with Wither II and is told as "was shot by a skull from Wither", and a ghast's fireball still belongs to its ghast.
- **Changed:**
  - Vendored `entities/src/fireball.rs`: `Fireball.owner_uuid`, the saved owner still waiting to be found. `Fireball::shoot` leaves it empty, and `deflect` (a player batting a fireball back) clears it with the new owner.
  - Vendored `entities/src/world/fireballs.rs`: `tick_fireball` resolves a waiting owner at the start of every tick, before the flight and the hit, through the new `shooter_of_uuid`, and keeps it once found. A UUID the level knows for a player becomes `Shooter::Player`, a living mob's `Shooter::Mob`. Nothing is resolved at load.
  - `minecraft_terrain/src/server_mobs.rs`: `mob_tags_where` saves `Owner` for fireballs, small fireballs and wither skulls: the shooter's UUID (`uuid_of`, a player's under `PLAYER_TARGET`), or the UUID still waiting. `spawn_saved` loads it into `owner_uuid`.
  - `minecraft_survival/src/entities.rs`: `mob_hit` is `pub(crate)` for the death test.
- **Tests:**
  - `server_mobs::tests::a_wither_skull_saved_in_flight_still_belongs_to_its_wither`: a wither's skull is saved with the wither's UUID as `Owner`; loaded first into a fresh world it has no owner and the UUID waiting; with the wither loaded after it, the skull hits a player for 8 with Wither II and the wither as its source; in a world without the wither it hits for 5 with no source (magic).
  - `server_mobs::tests::a_ghast_fireball_saved_in_flight_still_belongs_to_its_ghast`: the same for a ghast's large fireball (6, the ghast as its source; no source without the ghast).
  - `server_mobs::tests::a_fireball_a_player_batted_back_keeps_that_player_through_a_save`: a fireball batted back by player 1 is saved with that player's UUID; reloaded, it waits while the level does not know the player, finds `Shooter::Player(1)` once it does, and is saved again with the same UUID.
  - `death::tests::a_reloaded_skull_kill_reads_shot_by_a_skull_from_wither`: a skull and its wither saved, loaded in that order, the skull's hit goes through `mob_hit` (8 on Normal, Wither II for 10 s) and its death line reads "%1$s was shot by a skull from Wither".
- **Decisions:**
  - The owner is resolved by `shooter_of_uuid`, not by `id_of_uuid` alone as shulker bullets do: the entity world's UUID map keeps the ids of mobs that died or unloaded, so `id_of_uuid` can name an id that is gone, and a fireball with a gone mob owner is removed (the fork's existing rule). `shooter_of_uuid` takes a map entry only for a player or a mob that is still there, and otherwise looks through the level's living entities. A dead or absent owner leaves the fireball ownerless, as vanilla's `null` owner.
  - A player is found through the UUIDs the level was told (every player present in that level this session). Vanilla looks in every dimension; the fork's levels each have their own entity world, so a mob owner in another dimension stays unknown, and a player owner is found once that player has been in this level.
  - Fireballs keep the fresh `UUID` they had; only `Owner` is new. `HasBeenShot` is not saved (the fork does not track it).
- **Test steps (world `trial10`, survival, Normal):**
  1. Take the armour off for this step (diamond armour cuts a skull's 8 damage to under a heart, so 4 and 2.5 hearts could not be told apart). `/tick rate 2` (the world runs ten times slower). Build a wither (four soul sand in a T, three wither skeleton skulls on top), run 20 blocks away and wait for the blast. When a black skull is in the air towards you, `/tick freeze`. Save and quit, load the world, `/tick rate 20`, `/tick unfreeze`: the skull hits you for about 4 hearts (not 2.5) with Wither II (black hearts) (**302_skull_after_reload**). If it kills you, the chat says "<name> was shot by a skull from Wither". Put the armour back on.
  2. Friend window: the friend joins again after the host has loaded the world (the host's save and quit ends the friend's session), sees the wither's next skulls fly and hears them hit.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:wind_charge*16,minecraft:soul_sand*8,minecraft:wither_skeleton_skull*6,minecraft:netherite_sword,minecraft:shield,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*16,minecraft:milk_bucket*2,minecraft:totem_of_undying,minecraft:cooked_beef*32,minecraft:cobblestone*64
```

## Protocol

No message of this track has changed yet. `minecraft_net::PROTOCOL` stays 6.

## State doc changes

- Release 1's known limits (`docs/round10/release-1-tests.md`, Wind charges) and the round 9 log (group breezes, commit 1, and "Left for later"): drop "A player flying in creative is still pushed by a burst". A player flying in creative was never pushed by a burst: every push goes through the flying check (group d5-review, commit 1).
- Release 1's known limits (Withers) and the round 9 log (group wither-fight, commit 1): drop "A skull loaded from a save has no owner and hurts as magic for 5". `docs/MINECRAFT-SURVIVAL.md`: fireballs, small fireballs and wither skulls keep their shooter through a save (`Owner`), found again when the shooter loads (group d5-review, commit 2).

## Vendored files

Patched in round 10 (none added): `entities/src/fireball.rs` and `entities/src/world/fireballs.rs` (group d5-review, commit 2). Both are already in round 9's patched list (`docs/round9/track-d-log.md`, Vendored files), so the fork's counts do not change.
