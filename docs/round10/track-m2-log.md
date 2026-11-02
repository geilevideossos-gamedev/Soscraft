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
- Patched (water-paths 2): `entities/src/navigation.rs` (`WaterNav`, `plan_water_path`, `can_update_path`, `tick_water`, `#[cfg(test)] mod tests;`), `entities/src/stroll.rs` (`water_bound_random_position`, `#[cfg(test)] mod tests;`). Changed again: `entities/src/walk_path.rs` (the amphibious mode, `SwimEvaluator`) and its test file.
- Added (water-paths 2): `entities/src/navigation/tests.rs`, `entities/src/stroll/tests.rs` (both test only).

## Hooks

Root hooks this track added (M20, "Owners of shared pieces"). The shared path pieces of `water-paths` (M20's "Shared files outside the registry") are not registry hooks; they are under "Group water-paths" below, with how a family uses them.

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

- **Water paths (water-paths 2):** the 26.3 shelf mushroom (`ShelfMushroomBlock`) refuses every path, water ones too, but its block ids are not known here, so `refuses_water_paths` leaves it out.

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

#### 2. Minecraft mobs find paths through water like vanilla swimmers and amphibians

After d0c0cd7b.

- **javap (26.3 `client.jar`):**
  - `SwimNodeEvaluator(allowBreaching)`:
    - `getStart` is `getNode(floor(minX), floor(minY + 0.5), floor(minZ))` of the bounding box (no path type, no cost); `getTarget` the floored point.
    - `getNeighbors`: `findAcceptedNode` for the six `Direction.values()` (down, up, north, south, west, east), each kept when not closed; then for north, east, south and west with their clockwise neighbour, the diagonal at the same height when both sides are nodes with a cost of at least 0.
    - `findAcceptedNode`: only `WATER`, or `BREACH` when breaching, with a malus of at least 0; the cost becomes `max(cost, malus)`, then 8 more where the block holds no fluid. The 8 is added again each time the node is looked at.
    - `getPathTypeOfMob` over the box cells, x then y then z: an air cell over a block that is `isPathfindable(WATER)` is `BREACH`; a cell whose fluid is not water is `BLOCKED`; after the loop the last cell is `WATER` when it is `isPathfindable(WATER)`, else `BLOCKED`. The types are cached per search.
  - `AmphibiousNodeEvaluator(prefersShallowSwimming)` extends `WalkNodeEvaluator`:
    - `prepare` sets the mob's `WATER` malus to 0 for good, and `WALKABLE` 6 and `WATER_BORDER` 4 for the search (`done` puts those two back).
    - `getStart`: in water (`isInWater`) the swim start through `getStartNode`, else the walk start. `getTarget` floors `y + 0.5`, which a block target leaves as it is.
    - `getNeighbors`: the walk ones, then up (`jump - 1`, at least 0) and down (`jump`) through the walk `findAcceptedNode` with the node's floor height, each kept only when valid and `WATER`, and down not from a `TRAPDOOR` node; `jump` is `floor(max(1, maxUpStep))`, or 0 when the type above has a negative malus or the node is sticky honey. With the shallow preference every `WATER` neighbour below `seaLevel - 10` costs 1 more.
    - `getPathType`: a `WATER` block with a `BLOCKED` block on any of its six faces (`getPathTypeFromState`) is `WATER_BORDER`; other blocks as the walk evaluator.
    - `WalkNodeEvaluator.isAmphibious` is read in `getFloorLevel` (`y + 0.5` in floatable water for a mob that floats or is amphibious) and `findAcceptedNode` (a `WATER` node returns at once, like `WALKABLE`; the search below the water is for non-amphibious walkers only).
  - `WaterBoundPathNavigation`: `SwimNodeEvaluator(mob is DOLPHIN)`, no doors; `canUpdatePath` is breaching or `isInLiquid`; `getTempMobPos` `(x, y + 0.5 * height, z)`; `getGroundY` the waypoint's own y; `canMoveDirectly` `isClearForMovementBetween(.., false)` (collider clip, no fluids); `isStableDestination` is `!isSolidRender` of the block itself; `getMaxVerticalDistanceToWaypoint` 0.5.
  - `AmphibiousPathNavigation`: `AmphibiousNodeEvaluator(false)`; `canUpdatePath` always; the same temp position and ground y; `canMoveDirectly` only in a liquid; `isStableDestination` is the block below not being air; the vertical distance stays `PathNavigation`'s 1.0.
  - `PathNavigation`: `createPath` gives up on no targets, below the world or when it cannot update its path, and keeps a path under way to one of the targets; `tick` follows the path when it can update it, otherwise advances only when the temp position is above the next node, off the ground and in its column; `followThePath` reaches a node within `maxDistanceToWaypoint` across and the vertical distance up or down, and cuts to the next one through `shouldTargetNextNodeInDirection` (within 2 blocks, and a direct move or the angle rule).
  - `isPathfindable(WATER)`: `BlockBehaviour` answers whether the state's fluid is water, `LiquidBlock` whether it is not lava, `PowderSnowBlock` always. `DoorBlock` and `FenceGateBlock` answer false for water and `TrapDoorBlock` whether it is waterlogged, which is the fluid rule for them. These classes answer false for every type, and those that can hold water matter here: stairs, fences, walls, glass panes and bars (`CrossCollisionBlock`), chains, lanterns, chests and ender chests, campfires, sculk sensors, sea pickles, conduits, decorated pots, heavy cores, lightning rods (`RodBlock`), pointed dripstone and sulfur spikes (`SpeleothemBlock`), wall hanging signs and dried ghasts.
  - `DefaultRandomPos.getPos` with a home: `GoalUtils.mobRestricted` (home set and its centre within radius + horizontal + 1 of the mob), `RandomPos.generateRandomPosTowardDirection` (with a home and a horizontal range above 1, x and z move by `nextDouble * horizontal / 2` towards the home, x first), then not outside the build height, not outside the home (`isWithinHome`: `distSqr < radius²`), stable and without a malus. `Mob.hasHome` is a radius other than -1.
- **Changed (vendored entities crate):**
  - `walk_path.rs`:
    - `WalkProfile.amphibious: Option<bool>` (the shallow preference; `None` for every existing mob) and `WalkProfile.sea_level` (63, as `EntityWorld` starts).
    - The amphibious branches of the walk evaluator: the swim start in water, the vertical neighbours and the shallow cost (a new `amphibious_neighbors` called at the end of `neighbors`), the water floor in `floor_level`, the early `WATER` return in `find_accepted_node`, and `WATER_BORDER` through `static_type`, which every box type reads. `find_walk_path_to_any` searches an amphibious profile with the prepared maluses (`WATER` 0, `WALKABLE` 6, `WATER_BORDER` 4).
    - `SwimEvaluator`, a second `NodeSource` beside the walk evaluator, with the walk evaluator's node store, box size and type cache inside it, and `find_swim_path_to_any(.., breaching)`.
    - `pathfindable_in_water(world, pos)`, public for the swimming goals of `water-swimming` (`RandomSwimmingGoal` retries until the spot is `isPathfindable(WATER)`).
  - `navigation.rs`:
    - `WaterNav { Ground, Bound { breaching }, Amphibious { shallow } }` and `GroundNavigation.water` (after the climber's fields; `Ground` by default).
    - `plan_walk_path` and `plan_walk_path_to_any` hand a water navigation to a new `plan_water_path` (after the flying branch): no surface search, the navigation's own `canUpdatePath`, the swim search or the amphibious walk search, and the path adopted as d0b's flyers do.
    - `can_update_path(body, fluid)` for the three navigations; `recompute` asks it (one changed line).
    - `tick_water(world, body, fluid, speed)` after `tick_amphibious`: the temp position half the height up, half a block of vertical reach for a water-bound mob, the direct-move cut by a collider clip (an amphibious mob only in a liquid), the stuck check from the temp position, the advance rule when it cannot update, and the next node's own height as the wanted height.
  - `stroll.rs`: `water_bound_random_position(world, profile, position, horizontal, vertical, home, random)` after the amphibious positions, with `home: Option<(BlockPos, i32)>` (centre, radius) and `within_home`.
  - New test files `walk_path/tests.rs` (grown), `navigation/tests.rs` and `stroll/tests.rs`, with `#[cfg(test)] mod tests;` at the end of `navigation.rs` and `stroll.rs`.
- **How a family uses it** (M2's swimmers and M1's turtles):
  - Set `navigation.water` on the mob's own `GroundNavigation` right after building it in `spawn` (a loaded mob goes through `spawn` too): `WaterNav::Bound { breaching: false }` for fish, tadpoles, guardians and nautiluses, `Bound { breaching: true }` for dolphins, `Amphibious { shallow: false }` for axolotls and `Amphibious { shallow: true }` for frogs and turtles. Every `plan_walk_path` and `plan_walk_path_to_any` (the shared goals, `navigate_walk_to`, `MoveSink`, `recompute`) then plans the water way.
  - Give the profile vanilla's water malus 0 (`WaterAnimal`, `AgeableWaterCreature` and the amphibious evaluator's `prepare` all leave it there); `water-swimming`'s `water_walk` does it.
  - A shallow family copies `EntityWorld.sea_level` into its profile's `sea_level` each tick, as the drowned's goals copy it.
  - The family's step ticks the path with `navigation.tick_water(..)` instead of `tick_in`, after `delayed_recompute` as before.
  - Wandering: `water_bound_random_position(.., home)` for water-bound mobs (the elder guardian and the tame nautilus pass their home), `amphibious_random_position` for amphibious ones.
- **Tests added:**
  - `walk_path::tests::a_swim_path_goes_around_a_glass_wall_through_water_only` (every node is water, the wall is passed only through its opening, steps are faces or level diagonals and some are diagonal; a sealed wall or one of waterlogged fences stops the fish; waterlogged slabs and trapdoors and powder snow are pathfindable in water, air is not).
  - `walk_path::tests::a_swim_path_breaches_only_for_a_dolphin` (a spot in the air above a channel is reached by the breacher only, through water until the last node; the box types `BREACH`, `BLOCKED` over glass and in the air, `WATER`; an air node costs its malus 4 plus 8, and 8 more on the next look; a water node 0; an open-water node's ten neighbours in vanilla's order, the six faces and then the four level diagonals).
  - `walk_path::tests::a_two_block_swimmer_does_not_fit_a_one_block_tunnel` (a fish swims the 1 x 1 tunnel; a mob 1.2 high or 1.2 wide stays in the first pool).
  - `walk_path::tests::the_swim_start_is_half_a_block_above_the_feet` (the box's low corner, `floor(minY + 0.5)`).
  - `walk_path::tests::an_amphibious_path_swims_across_a_pond_and_walks_out` (the prepared maluses 6, 4 and 0 while the profile keeps its own; the swim start; the path stays in water until it climbs out onto land; a mob that cannot float starts at the pond's floor instead; the water floor `y + 0.5`; `WATER_BORDER` at the floor and the sides; up and down neighbours only into `WATER`; the shallow +1 below `sea_level - 10` and none above).
  - `navigation::tests::a_water_bound_mob_reaches_a_node_within_half_a_block_up_or_down` (and within 1.0 for an amphibious mob; the wanted height is the node's own, where the ground navigation would take the floor).
  - `navigation::tests::a_stranded_swimmer_makes_no_path_unless_it_breaches` (no path and no move for a stranded fish, a path for a breacher, a path in water; the advance rule off the ground above the node's column only).
  - `navigation::tests::it_cuts_a_corner_only_with_a_clear_line` (stone or glass in the line stops the cut; an amphibious mob cuts only in a liquid).
  - `navigation::tests::a_water_navigation_follows_its_own_rules_to_update_its_path` (the ground navigation refuses to plan in the air, the amphibious one plans anywhere; a fish's path is re-planned after 20 ticks in water and waits out of it, a breacher's is not).
  - `stroll::tests::a_water_bound_spot_needs_no_floor_but_no_solid_block` (mid-water spots with no floor are taken, solid blocks never, even with no malus for them; spots above the surface are taken too; the ground rule finds none in open water).
  - `stroll::tests::a_water_bound_spot_stays_inside_its_home` (inside the radius when the home is near; when it is far the spots lean towards it, beyond the plain range, and without a home they stay within it).
- **Unchanged:** the walk and flying paths (the pinned `the_walk_search_is_unchanged_behind_the_evaluator_split` still passes), the drowned's measured amphibious search (`navigate_amphibious_to_with_accuracy`, `tick_amphibious`), every existing navigation and stroll caller.
- **Decisions:**
  - **`WaterNav` is the switch**, kept in the navigation as vanilla's `createNavigation` is; `WalkProfile.amphibious` is the evaluator's mode, which `plan_water_path` sets on a clone of the profile. A profile that sets it itself gets the same search (the prepared maluses are applied whenever it is set).
  - **The sea level for the shallow preference** travels in `WalkProfile.sea_level`, which a shallow family refreshes from `EntityWorld.sea_level` (M1's brief, decision 4: no `World::sea_level`). Until it does, 63 holds, the overworld's.
  - **The swim evaluator reuses the walk evaluator's store** (nodes, hash, box size, type cache) instead of a copy of them; its own path types go into the same per-search cache, which the walk rules never read in a swim search.
  - **The walked distance** is written before the neighbour is checked, as vanilla's `PathFinder.findPath` does (javap: the `putfield walkedDistance` comes ahead of the `maxPathLength` test), so a node looked at again from a worse side keeps the longer distance. With the 16-block follow range a fish then gives up on detours that look short: a first test layout (a wall 5 blocks out with its opening 4 to the side, the target 10 away, a 13.3-block path) was not reached, as a Python model of the same loop confirmed, while moving the length check after the update reached it. The loop is the shared one and stays vanilla's; the tests use layouts within that reach.
  - **The +8 for a node without fluid** is added on every look, as vanilla's `findAcceptedNode` does, so a much-visited air spot grows dearer.
  - **`isPathfindable(WATER)`** is the water fluid (`floatable_fluid`, which the server answers from the catalog, kelp and seagrass included) less the block classes that refuse every path and can hold water (listed by id endings and names), plus powder snow. The 26.3 shelf mushroom (`ShelfMushroomBlock`) refuses too, but its ids are not known here; it is left out.
  - **The amphibious start** in water uses the box's low corner, as vanilla's `getStart`, not the block position the walk start uses.
  - **`tick_water` does not run the delayed recomputation** itself: as with the ground navigation, the step calls `delayed_recompute` first.
  - **Corners:** as d0b's flyers, the navigation keeps no node types, so `canCutCorner` holds at every node.
  - **The home test** (`a_water_bound_spot_stays_inside_its_home`) is in `water-swimming`'s list in the brief; the home is built here, so its test came with it.
  - **Test files** sit beside the three shared files (`walk_path/tests.rs`, `navigation/tests.rs`, `stroll/tests.rs`), so the shared files only gain one module line each.
- **Protocol:** no message or codec change.
- **Game events still to add:** none (paths post none).
- **What changes in the game:** nothing yet. No mob uses a water navigation until `water-swimming` and the fish (segment m2b); every ground, flying and drowned path is as before.
- **Runs:** `test -p minecraftoss-entities --locked` (586 lib tests passed, among them the pinned walk split test and `flying::tests`; every `check_*` binary builds), `check -p launcher --locked` (green; only the old render_frame and skate warnings). The three new test files pass `rustfmt --check --edition 2021` with empty output on their staged content, and no added line holds a comment.
- **Test steps** (regression; nothing new to see). Release build, survival, Normal, by day. Name each screenshot by its label.
  1. Dig a pool 6 x 6 and 3 deep, fill it from the water bucket, and use a drowned egg in it. Stand at the edge: the drowned swims to you and climbs out after you. **m2-paths-2**
  2. Use two cow eggs beside the pool and push one in: it swims to the edge and climbs out. The other wanders on the grass.
  3. Use a zombie egg on the far side of the pool: it comes to you, as before.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:drowned_spawn_egg*2,minecraft:cow_spawn_egg*2,minecraft:zombie_spawn_egg*2,minecraft:water_bucket,minecraft:iron_shovel,minecraft:iron_sword
```
