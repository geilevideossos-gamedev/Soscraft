# Track C log: shulkers, end cities, end sky and dragon effects, elytra bodies, open limitations

Branch `track-c`, worktree `C:\Dev\mc-wt\track-c`. Briefs: `c1-shulkers.md`, `c5-limitations.md`, `c2-end-cities.md`, `c3-end-sky-dragon-fx.md`, `c4-elytra-body.md`. Order: c1 groups 1-3, c5 groups 1-4, c1 groups 4-8, c2, c3, c5 groups 5-11, c4. Base: 0bcbfb17.

A commit can't name its own hash, so each entry names the previous commit's hash; the last entry's hash is in `git log` under its subject.

## Group c1-levitation: levitation and slow falling (c1 groups 1-3)

### 1. Minecraft players and mobs can have levitation and slow falling

Previous commit: 0bcbfb17

- **Changed:**
  - `minecraftoss-player` `survival.rs`: `EffectKind::Levitation` and `EffectKind::SlowFalling`, appended after `Wither` so the indices of the older kinds don't move; the effect array holds 12. Both count down in `tick_effects` with no work of their own.
  - `minecraft_survival`: `/effect give` and `/effect clear` know `minecraft:levitation` and `minecraft:slow_falling` (`commands.rs`), `clear_effects` removes them (so milk and the totem clear them, `survival.rs`), and the player file saves and reads them under their vanilla ids (`player_nbt.rs`).
  - `minecraft_terrain` `potions.rs`: `player_effect` maps `levitation` and `slow_falling`, so drinking or being splashed by a slow falling potion now gives the effect (before, it did nothing).
  - `minecraftoss-entities` `effects.rs`: `MobEffect::Levitation` and `MobEffect::SlowFalling` with their ids, in `ALL`, with no per-tick work. Splash and lingering potions of slow falling now give mobs the effect, and a mob's `active_effects` keep both through a save. `potion.rs` gives both their vanilla colours (`levitation` 13565951, `slow_falling` 15978425) for the witch potion match.
- **Tests:**
  - `commands`: `levitation_and_slow_falling_are_given_and_cleared_by_command` (`/effect give @s minecraft:levitation 10 1` gives amplifier 1 for 200 ticks, slow falling 30 s gives 600 ticks, `/effect clear @s` removes both).
  - `survival`: `slow_falling_potions_last_like_vanilla_and_a_totem_clears_levitation` (long slow falling 4800 ticks, a shorter one doesn't cut it, the totem clears levitation and slow falling, levitation counts down and ends).
  - `player_nbt`: `levitation_and_slow_falling_are_saved_and_read_back`.
  - `minecraft_terrain` `server`: `levitation_and_slow_falling_on_a_mob_round_trip_through_its_save`.
- **Decisions:**
  - The new kinds are appended at the end of both enums. Tracks A and B also append kinds (haste, mining fatigue, conduit power, nausea); whoever merges second keeps every kind and grows the arrays to match.
  - Nothing moves yet: the effects only exist, count down, save and clear. The movement comes in the next two commits.
- **Test steps:** (any world, survival, open ground)
  1. `/effect give @s minecraft:levitation 10 1`, then quit to the menu and load the world again within 10 seconds, then `/effect clear @s`: no error, the command is accepted each time.
  2. Drink the slow falling potion, quit and load: the world loads normally (the effect is in the player file; you can't see it yet, there are no effect icons).
  3. Throw the splash slow falling potion at a zombie: it is accepted like any splash potion.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:potion@slow_falling,minecraft:potion@long_slow_falling,minecraft:splash_potion@slow_falling,minecraft:zombie_spawn_egg*2
```
