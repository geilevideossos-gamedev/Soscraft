# Track M2 log: water mobs and buckets on the shared mob registry

Branch `track-m2`, worktree `C:\Dev\Soscraft\mc-wt\track-m2`, target dir `D:\iw4l-wt\track-m2`. Brief: `docs/round10/track-m2.md`. Mob list: `docs/round10/mobs.md` (the `## M2` rows only). Registry: `docs/round10/track-d0.md` (D0-2, D0-3).

The branch starts at `track-d0`'s tip 4c42b0ae: `minecraft-survival` at 9c6efebd plus D0's segment d0b.

A commit can't name its own hash, so each entry names the commit before it. Its own hash is in `git log` under its subject.

## Vendored files

Files under `third_party/minecraftoss` this track patched or added.

- Patched (water-hooks 1): `entities/src/world/families.rs` (the `Breath` hook), `entities/src/world/living.rs` (`breathe_up_to`, `breathe_with_gills`), `entities/src/world/families/water.rs` (the registration of the test probe).
- Added (water-hooks 1): `entities/src/world/families/water/swimming.rs`, `entities/src/world/families/water/swimming/swim_probe.rs` (test only), `entities/src/world/families/water/tests.rs` (test only).
- Patched (water-hooks 2): `entities/src/world/emissions.rs` (the own step at zero volume and an empty swim sound stay silent). Changed again: `entities/src/world/families.rs` (`swim_sounds`, `quiet_steps`, `movement_sounds`), the probe and the water tests.
- Patched (water-paths 1): `entities/src/walk_path.rs` (the A* loop over the private `NodeSource`, and `#[cfg(test)] mod tests;`).
- Added (water-paths 1): `entities/src/walk_path/tests.rs` (test only).

## Hooks

Root hooks this track added (M20, "Owners of shared pieces").

- **"Registered Minecraft mobs breathe like vanilla water animals"** (water-hooks 1): `Family::breath(mob) -> Breath` in `entities/src/world/families.rs`, with `pub enum Breath { Lungs { max, refill }, Gills, Own }`. The default is `Lungs { max: 300, refill: 4 }`, today's `living::breathe`.
  - `Lungs` runs `living::breathe_up_to(body, world, eye_height, breathes_water, max, refill)` where the base tick ran `breathe` before; `breathe` is now `breathe_up_to(.., MAX_AIR, 4)`. A refill of at least `max` fills the air at once (vanilla's dolphin returns `getMaxAirSupply` from `increaseAirSupply`, even from negative air), so the dolphin is `Lungs { max: 4800, refill: 4800 }`.
  - `Gills` is vanilla's `WaterAnimal.handleAirSupply` through `living::breathe_with_gills(body, alive)`, run after the ambient roll, where vanilla runs it after `Mob.baseTick`. The lungs do not run for it.
  - `Own` leaves the air alone: axolotls and nautiluses dry out in their own ticks.
- **"Registered Minecraft mobs make their own swim and step sounds"** (water-hooks 2): `Family::swim_sounds(kind) -> Option<(&'static str, &'static str)>` (swim, splash; `None` keeps the category's, an empty swim sound means none) and `Family::quiet_steps(kind) -> bool` (false by default), after `emits_sounds` in `entities/src/world/families.rs`. D0's `movement_sounds::<S>` puts the pair into its `MovementSounds` and turns quiet steps into `step: Some("")` at `with_step_volume(0.0)`. In `world/emissions.rs` the own-step arm plays only above zero volume, and the swim arm plays only a sound that is not empty; the step itself (its next-step distance, the amethyst chime and W's `step` game event) and the swim's `swim` game event still happen.

## Protocol

No commit of this track has changed a `ToClient`/`ToServer` message or the codec yet. `minecraft_net::PROTOCOL` is not bumped here.

## State doc changes

None yet: the hooks change nothing a player sees.

## Game events still to add

None yet.

## Left for later

Nothing yet.

## Combined kit

Everything this track's test steps need so far (11 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:rabbit_spawn_egg*4,minecraft:polar_bear_spawn_egg*2,minecraft:silverfish_spawn_egg*2,minecraft:zombie_spawn_egg*2,minecraft:drowned_spawn_egg*2,minecraft:cow_spawn_egg*2,minecraft:water_bucket,minecraft:glass*16,minecraft:stone*64,minecraft:iron_shovel,minecraft:iron_sword
```

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

#### 2. Registered Minecraft mobs make their own swim and step sounds

After 0fa9ec48.

- **javap (26.3 `client.jar`):**
  - `Entity.walkingStepSound`: `playStepSound(pos, state)`, then the amethyst chime when `shouldPlayAmethystStepSound`. So a mob whose `playStepSound` is empty still chimes on amethyst.
  - `AbstractFish` (cod, salmon, tropical fish, pufferfish, tadpole): `playStepSound` is empty, `getSwimSound` is `entity.fish.swim`, the splash is `Entity`'s `entity.generic.splash`.
  - `AbstractNautilus`: `playStepSound` is empty; `ZombieNautilus.getSwimSound` is `entity.zombie_nautilus.swim`.
  - `Dolphin`: `entity.dolphin.swim` and `entity.dolphin.splash`. `Axolotl`: `entity.axolotl.swim` and `entity.axolotl.splash`.
  - `Frog.playStepSound`: `entity.frog.step` at 0.15 and pitch 1 (an own step, `Family::step_sound`).
  - `SulfurCube.playStepSound`: the cube's step only while it holds no block.
  - `Guardian` and `Squid`: `getMovementEmission` is `EVENTS`, so they answer D0's `Family::emits_sounds` false (no step, chime or swim; the splash still plays) and need no empty swim sound.
- **Changed (vendored entities crate):**
  - `world/families.rs`: `Family::swim_sounds` and `Family::quiet_steps` after `emits_sounds`; `movement_sounds::<S>` applies them before `emits_sounds`.
  - `world/emissions.rs`: the own-step arm of `play_movement` plays only when `step_volume > 0.0` (one changed line); the swim arm plays only a sound that is not empty (its two pitch draws still happen).
  - The probe `SwimProbe` gains a second kind, `minecraft:silent_swim_probe` (an empty swim sound and `entity.hostile.splash`, steps as usual), and `minecraft:swim_probe` steps quietly and swims with `entity.fish.swim`. Its tick now moves it by its state's input (`travel_water` in water, else `travel_air`) and plays the movement through `play_movement` with the family's sounds.
- **Tests added** (`world::families::water::tests`):
  - `a_quiet_stepper_flops_without_a_step_sound_but_swims_with_its_own` (two probes walk on stone: the plain one sounds `block.stone.step` at least twice, the quiet one never; in deep water the quiet one swims with `entity.fish.swim`, never the generic swim, and no empty sound is heard).
  - `an_empty_swim_sound_is_silent_but_the_splash_plays` (both probes drop into water: the silent one splashes once with `entity.hostile.splash` and posts at least two `minecraft:swim` game events but plays no swim sound and no empty sound; the other splashes with the generic splash and swims aloud).
- **Decisions:**
  - Two guarded lines in `emissions.rs`, not the brief's one: the own step at zero volume (the brief's line, D0's old `:64`, now the `(Some(own), _)` arm) and the swim sound when empty. An empty swim sound has no other way to stay silent without silencing the steps too (D0's `emits_sounds`), and dropping empty voices later would leave a sound event to send and resolve. No vanilla mob plays steps but no swim sound, so the empty swim only matters to a family that wants it; guardians and squids use `emits_sounds` as vanilla's `EVENTS` does.
  - An empty swim keeps its two pitch draws, as `playSwimSound` evaluates its pitch before playing; a quiet step draws nothing, as in vanilla's empty `playStepSound`.
  - The test name keeps the brief's "flops": the quiet probe walks on stone (fish flopping comes with `water-swimming`); its steps are the moves a flop makes on land.
  - `quiet_steps(kind)` is per kind. The sulfur cube's step depends on the block it holds (per mob), so the sulfur cube groups drop its step voice in their own tick while it holds one, or extend the hook to the mob then.
- **Runs:** `test -p minecraftoss-entities --locked --lib families` (110 passed), `test -p minecraftoss-entities --locked` (the whole crate: the lib tests and every `check_*` binary, exit 0), `check -p launcher --locked` (green; only the old `render_frame` and skate warnings).
- **Test steps:** nothing new to hear; registered mobs sound as before. Release build, survival, Normal, by day. Name each screenshot by its label.
  1. Place a 5 x 5 floor of stone and use the polar bear egg on it: as the bear walks it makes its own heavy steps. **m2-hooks-2**
  2. Use the rabbit egg on grass: its hops sound the grass steps.
  3. Use the silverfish egg on the stone: it crawls without a sound, as before.
  4. Pour the water bucket into a hole and use the rabbit egg beside it; when a rabbit hops in, it splashes.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:polar_bear_spawn_egg*2,minecraft:rabbit_spawn_egg*4,minecraft:silverfish_spawn_egg*2,minecraft:stone*64,minecraft:water_bucket,minecraft:iron_shovel
```

### Group water-paths

#### 1. Minecraft path search takes any node evaluator

After 287abc2f.

- **Why:** vanilla's `PathFinder` takes any `NodeEvaluator`, but here the A* loop (`find_path_to_any`) took the walk evaluator itself. d0b made the flying search a mode of that evaluator (one A* for walking and flying) and did not split the loop, so this commit does, as M2-1 plans: the swim evaluator of the next commit plugs into the same loop, and `Node` and `Heap` stay private to `walk_path.rs`.
- **Changed (vendored entities crate, `walk_path.rs`):** a private trait `NodeSource` (`profile`, `nodes`, `node`, `start`, `neighbors`), implemented by the walk evaluator by handing each call to its own method, so the walk and the flying search run exactly as before. `find_path_to_any` is generic over it. The longest path and the node budget are read from the profile before the start node is made instead of after the targets' nodes (no side effects either way). Nothing public changes.
- **Tests added** (`walk_path::tests`, a new test file beside `walk_path.rs`):
  - `the_walk_search_is_unchanged_behind_the_evaluator_split`: six searches over a stone field with a wall and a pool, pinned node by node to what the search gave before the split (the paths were recorded on 287abc2f's code first): a cow around the wall; a cow to the best of three targets with reach 1; a cow afloat in the pool climbing out; a mob that cannot float stopping before the water; a cow cut short by its 16-block follow range; and a flyer over the wall through `find_fly_path_to_any`.
- **Unchanged:** every walk, flying and goal test, and the `bin/check_*` fixtures (none was touched).
- **Decisions:**
  - The loop keeps its variable names and reaches the nodes through `nodes()`, so the diff of the loop is mechanical; `reconstruct` became a `FnMut` closure for it.
  - The trait stays private: the searches are reached through the public `find_*` functions, as before.
- **Test steps** (regression of paths; nothing new to see). Release build, survival, Normal, by day, flat open ground in the Overworld. Name each screenshot by its label.
  1. Build a stone wall 7 wide and 3 high. Stand behind its middle and use a zombie egg 6 blocks in front of it. The zombie walks round one end of the wall and comes at you. **m2-paths-1**
  2. Dig a pool 5 x 5 and 3 deep, fill it from the water bucket, and use a drowned egg in it: the drowned swims to you when you stand at the edge. Use two cow eggs beside the pool: they wander as before, and a cow pushed into the pool swims out.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:zombie_spawn_egg*2,minecraft:drowned_spawn_egg*2,minecraft:cow_spawn_egg*2,minecraft:stone*64,minecraft:water_bucket,minecraft:iron_shovel,minecraft:iron_sword
```
