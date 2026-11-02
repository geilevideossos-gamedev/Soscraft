# Track M2 log: water mobs and buckets on the shared mob registry

Branch `track-m2`, worktree `C:\Dev\Soscraft\mc-wt\track-m2`, target dir `D:\iw4l-wt\track-m2`. Brief: `docs/round10/track-m2.md`. Mob list: `docs/round10/mobs.md` (the `## M2` rows only). Registry: `docs/round10/track-d0.md` (D0-2, D0-3).

The branch starts at `track-d0`'s tip 4c42b0ae: `minecraft-survival` at 9c6efebd plus D0's segment d0b.

A commit can't name its own hash, so each entry names the commit before it. Its own hash is in `git log` under its subject.

## Vendored files

Files under `third_party/minecraftoss` this track patched or added.

- Patched (water-hooks 1): `entities/src/world/families.rs` (the `Breath` hook), `entities/src/world/living.rs` (`breathe_up_to`, `breathe_with_gills`), `entities/src/world/families/water.rs` (the registration of the test probe).
- Added (water-hooks 1): `entities/src/world/families/water/swimming.rs`, `entities/src/world/families/water/swimming/swim_probe.rs` (test only), `entities/src/world/families/water/tests.rs` (test only).

## Hooks

Root hooks this track added (M20, "Owners of shared pieces").

- **"Registered Minecraft mobs breathe like vanilla water animals"** (water-hooks 1): `Family::breath(mob) -> Breath` in `entities/src/world/families.rs`, with `pub enum Breath { Lungs { max, refill }, Gills, Own }`. The default is `Lungs { max: 300, refill: 4 }`, today's `living::breathe`.
  - `Lungs` runs `living::breathe_up_to(body, world, eye_height, breathes_water, max, refill)` where the base tick ran `breathe` before; `breathe` is now `breathe_up_to(.., MAX_AIR, 4)`. A refill of at least `max` fills the air at once (vanilla's dolphin returns `getMaxAirSupply` from `increaseAirSupply`, even from negative air), so the dolphin is `Lungs { max: 4800, refill: 4800 }`.
  - `Gills` is vanilla's `WaterAnimal.handleAirSupply` through `living::breathe_with_gills(body, alive)`, run after the ambient roll, where vanilla runs it after `Mob.baseTick`. The lungs do not run for it.
  - `Own` leaves the air alone: axolotls and nautiluses dry out in their own ticks.

## Protocol

No commit of this track has changed a `ToClient`/`ToServer` message or the codec yet. `minecraft_net::PROTOCOL` is not bumped here.

## State doc changes

None yet: the hooks change nothing a player sees.

## Game events still to add

None yet.

## Left for later

Nothing yet.

## Segment m2a

### Group water-hooks

#### 1. Registered Minecraft mobs breathe like vanilla water animals

After 4c42b0ae.

- **javap (26.3 `client.jar`):**
  - `WaterAnimal.baseTick`: the air is read before `super.baseTick()` (`PathfinderMob`, so `LivingEntity.baseTick` and then `Mob.baseTick`'s ambient roll), then `handleAirSupply(level, air)`. `AgeableWaterCreature` is the same.
  - `WaterAnimal.handleAirSupply(level, air)`: alive and not `isInWater()` (the body, not the eyes): air becomes `air - 1`; then `shouldTakeDrowningDamage()` (air `<= -20`) sets air 0 and `hurtServer(drown, 2)`. Otherwise (in water, or dead) air 300.
  - So a stranded 3-health fish is hurt at tick 320 and dies at tick 340.
  - `Dolphin`: `handleAirSupply` is empty, `getMaxAirSupply` 4800, `increaseAirSupply` returns `getMaxAirSupply()`. It breathes through `LivingEntity.baseTick`'s lungs.
  - `Axolotl` has its own `baseTick`, `handleAirSupply`, `rehydrate` and `getMaxAirSupply`: `Own`.
- **Changed (vendored entities crate):**
  - `world/families.rs`: `Breath`, `Family::breath` (after `breathes_water`), and `family_base_tick`: the lungs through `breathe_up_to` for `Lungs` only, the gills after the ambient roll for `Gills` only, both with the same 2 `minecraft:drown` damage that bypasses armour.
  - `world/living.rs`: `breathe_up_to` beside `breathe`, which calls it, and `breathe_with_gills`.
  - `world/families/water.rs`: `pub(crate) mod swimming;`, `#[cfg(test)] mod tests;` and the probe in `family_group!(water 64..=79 { .. })`.
  - New `world/families/water/swimming.rs` (for now only `#[cfg(test)] pub mod swim_probe;`; `water-swimming` fills it) and `world/families/water/swimming/swim_probe.rs`: the `#[cfg(test)]` family `SwimProbe`, kind `minecraft:swim_probe`, key 79, a cod's size (0.5 by 0.3, eyes at 0.195) and health (3), category `water_ambient`, breathing by its state's `Breath` (gills when spawned).
- **Tests added** (`world::families::water::tests`):
  - `a_gill_breather_keeps_full_air_in_water_and_dies_on_land_after_340_ticks` (air 100 back to 300 in one tick in water and kept there; on land 300 less a tick, hurt to 1 at tick 320 with air 0, dead at tick 340; a dead probe's air goes back to 300).
  - `a_lung_breather_still_drowns_under_water` (the default answer is `Lungs { max: 300, refill: 4 }`; under water from air 5 it is hurt at -20 and set to 0, then gets four back a tick on land up to 300).
  - `an_own_breather_keeps_its_air` (123 under water and on land, never hurt).
  - `a_lung_breather_with_4800_air_refills_at_once_and_still_drowns` (from -5 to 4800 in one breath; 4799 after a tick under water; drowns from -19).
- **Decisions:**
  - The hook tests live in M2's own `world/families/water/tests.rs`, not in D0's `families/tests.rs` (where the brief names them `families::tests::..`): M1 and M3 add tests at the end of that file too, and a test file of M2's own keeps those merges clean.
  - The probe lives at `water/swimming/swim_probe.rs`, the path M2-1's plan gives it, so `water-swimming` only adds to `swimming.rs`.
  - The gills run after the ambient roll and the death clock, where vanilla's `WaterAnimal.baseTick` runs them, not where the lungs run (before the death clock): so a gasp's hurt voice draws after the ambient roll and a gasp that kills starts the death clock on the next tick, as in vanilla.
  - `Breath::Lungs`' refill follows `increaseAirSupply`: `min(air + refill, max)`, but a refill of at least `max` fills to `max` at once, as the dolphin's override does from negative air.
- **Runs:** `test -p minecraftoss-entities --locked --lib families` (108 passed), `check -p launcher --locked` (green; only the old `render_frame` and skate warnings).
- **Test steps:** nothing new to see; registered mobs breathe as before. Release build, survival, Normal, by day. Name each screenshot by its label.
  1. Dig a 1 x 1 hole 3 deep in the ground and use the rabbit egg on its bottom. Pour the water bucket into the hole and put glass on top of it.
  2. After about 16 s the rabbit flashes red, and one second later it dies. **m2-hooks-1**
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:rabbit_spawn_egg*4,minecraft:water_bucket,minecraft:glass*16,minecraft:iron_shovel
```
