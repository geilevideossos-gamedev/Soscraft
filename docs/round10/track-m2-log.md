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
- Patched (water-buckets 1): `world/src/level/mod.rs` (`pub mod mob_bucket;` after `pub mod dispenser;`).
- Added (water-buckets 1): `world/src/level/mob_bucket.rs` (the seven bucket rows, `entity_data`, `Level::empty_mob_bucket`), `world/src/level/mob_bucket/tests.rs` (test only).
- Changed again (water-buckets 2): `world/src/level/mob_bucket.rs` (`bucket_of`, `bucket_entity_data`).
- Patched (water-buckets 3): `world/src/level/dispenser.rs` (the arm for the six water mob buckets after `"water_bucket" | "lava_bucket"`). Changed again: `world/src/level/mob_bucket.rs` (`dispense_mob_bucket`) and its test file.
- Patched (squids-live 1): `entities/src/world/families.rs` (`Family::OWN_DYING_TRAVEL` and its test in `family_base_tick`). Changed again: the probe and the water tests.
- Patched (squids-live 2): `entities/src/movement.rs` (`Body::move_self` split out of `travel_air_flying`). Changed again: `entities/src/world/families/water.rs` (the squids' registration).
- Added (squids-live 2): `entities/src/world/families/water/squids.rs`, `entities/src/world/families/water/squids/tests.rs` (test only).
- Patched (squids-live 3): `world/src/natural_spawner.rs` (the `InWater` line in `check_spawn_obstruction`). Changed again: `world/src/natural_spawner/families/water.rs` (the squid rows and their rules).
- Added (squids-live 3): `world/src/natural_spawner/families/water/tests.rs` (test only).

## Hooks

Root hooks this track added (M20, "Owners of shared pieces"). The shared path pieces of `water-paths` (M20's "Shared files outside the registry") are not registry hooks; they are under "Group water-paths" below, with how a family uses them.

- **"Registered Minecraft mobs breathe like vanilla water animals"** (water-hooks 1): `Family::breath(mob) -> Breath` in `entities/src/world/families.rs`, with `pub enum Breath { Lungs { max, refill }, Gills, Own }`. The default is `Lungs { max: 300, refill: 4 }`, today's `living::breathe`.
  - `Lungs` runs `living::breathe_up_to(body, world, eye_height, breathes_water, max, refill)` where the base tick ran `breathe` before; `breathe` is now `breathe_up_to(.., MAX_AIR, 4)`. A refill of at least `max` fills the air at once (vanilla's dolphin returns `getMaxAirSupply` from `increaseAirSupply`, even from negative air), so the dolphin is `Lungs { max: 4800, refill: 4800 }`.
  - `Gills` is vanilla's `WaterAnimal.handleAirSupply` through `living::breathe_with_gills(body, alive)`, run after the ambient roll, where vanilla runs it after `Mob.baseTick`. The lungs do not run for it.
  - `Own` leaves the air alone: axolotls and nautiluses dry out in their own ticks.
- **"Registered Minecraft mobs make their own swim and step sounds"** (water-hooks 2): `Family::swim_sounds(kind) -> Option<(&'static str, &'static str)>` (swim, splash; `None` keeps the category's, an empty swim sound means none) and `Family::quiet_steps(kind) -> bool` (false by default), after `emits_sounds` in `entities/src/world/families.rs`. D0's `movement_sounds::<S>` puts the pair into its `MovementSounds` and turns quiet steps into `step: Some("")` at `with_step_volume(0.0)`. In `world/emissions.rs` the own-step arm plays only above zero volume, and the swim arm plays only a sound that is not empty; the step itself (its next-step distance, the amethyst chime and W's `step` game event) and the swim's `swim` game event still happen.
- **"Registered Minecraft mobs can move by their own rules while dying"** (squids-live 1): `const OWN_DYING_TRAVEL: bool = false` after `MOUNT` in `Family` (`entities/src/world/families.rs`). `family_base_tick` runs the shared `living::dying_travel` (no input, falling or sinking with the plain drag, the swim and step sounds) only for a dead mob with AI whose family leaves it false. A family that sets it moves its dead mobs in its own tick, as vanilla's `LivingEntity.aiStep` calls the class's own `travel` while it dies. Squids set it (`Squid.travel` is a bare `move`), and dolphins and axolotls will.
- **"Registered Minecraft mobs can have a tracking range per kind"** (squids-drawn 1): `fn tracking_range(kind: &'static str) -> i32 { Self::TRACKING_RANGE }` after `CAGE_MARGIN` in `TerrainFamily` (`crates/minecraft_terrain/src/families.rs`). `terrain_group!`'s `tracking_range` finds the family's own `KINDS` entry for the asked kind (with or without `minecraft:`, the new private `named` beside `names`) and asks the hook with it, so `mob_tracking::tracking_range` answers per kind. Squids answer 10 for the glow squid (squids-drawn 2), and guardians will (the elder guardian).

## Protocol

No commit of this track has changed a `ToClient`/`ToServer` message or the codec yet. `minecraft_net::PROTOCOL` is not bumped here. The mob buckets (water-buckets 1) add `Change::MobBucket` and `Command::EmptyMobBucket`, which stay in-process and are not serialized: a friend's click runs on the host.

## State doc changes

- `docs/MINECRAFT-SURVIVAL.md` (water-buckets 1): "Cod, salmon, tropical fish, pufferfish, axolotl and tadpole buckets pour their water with their own splash (a hiss in the Nether; in creative the mob bucket stays in the hand); the sulfur cube bucket pours nothing. Each lets out its mob with the bucket's data and `FromBucket`; until the mob's family is live it is kept in the world but not shown."
- `docs/MINECRAFT-SURVIVAL.md` (squids-live 3): "Squids spawn near the surface of oceans and rivers (up to 13 blocks below sea level) and glow squids in unlit water 33 or more blocks below it; both wait in the world, not yet shown, until `squids-drawn`."
- `docs/MINECRAFT-SURVIVAL.md` (squids-drawn 2), replacing squids-live 3's line: "Squids and glow squids are live: they spawn in oceans and rivers (glow squids in unlit water 33 blocks below sea level), hatch from their eggs, swim in pulses, squirt and flee when hit, dry out on land, grow up (a golden dandelion locks a baby) and are saved; a glow squid glows at full brightness and goes dark for five seconds after it squirts. Friends see the same strokes, tilts and rolls."
- `docs/MINECRAFT-SURVIVAL.md` (squids-drawn 3), after squids-drawn 2's line: "A hit squid leaves a black ink cloud (a glow squid a teal one) that drifts, fades and sinks slowly in air; a glow squid sheds tiny sparkles. Friends see the ink in the same place."

## Game events still to add

None yet. The mob bucket posts `entity_place` at the block the mob comes out in (`Level::empty_mob_bucket`, water-buckets 1); its `fluid_place` comes from `empty_bucket_into`, as for every water bucket.

## Left for later

- **Water paths (water-paths 2):** the 26.3 shelf mushroom (`ShelfMushroomBlock`) refuses every path, water ones too, but its block ids are not known here, so `refuses_water_paths` leaves it out.
- **Mob buckets (water-buckets 1):**
  - `MobBucketItem.spawn` plays the new mob's ambient sound; a dormant kind has none, so each family plays it when it goes live (`fish-buckets`, `axolotls-buckets`, the tadpoles, the sulfur cube's items group).
  - The landing height uses the kind's spawn-row width; until a family adds its row (`squids-live`, `fish-spawning`, ..) it is the spawn eggs' 0.6.
  - The family components (salmon size, tropical pattern, axolotl variant, sulfur cube content) start empty in the rows (`MobBucket.components`); each family group fills its own.
- **Mob buckets in dispensers (water-buckets 3):** 26.3 dispenses the sulfur cube bucket with the same behaviour as the fish buckets; the sulfur cube groups add `"sulfur_cube_bucket"` to the dispenser arm (the level side already lets a cube out without water). A dispensed water mob bucket plays no hiss where water evaporates, as the fork's plain water bucket plays none from a dispenser.
- **Squids (squids-live 2, squids-drawn):** Impaling's extra damage (`#sensitive_to_impaling`) waits until the fork has Impaling for any mob. The golden dandelion's pause and reset particles are dropped, as for rabbits and polar bears.
- **Mob bucket pickup (water-buckets 2):** vanilla copies the mob's custom name onto the bucket (`saveDefaultDataToBucketTag`), but `MobCore` holds no name (only the server's tag for the mob keeps it), so a picked-up mob loses its name. A name on the bucket item itself still names the poured mob.

## Combined kit

Everything this track's test steps need so far (27 stacks; water-buckets 1 counted 24 where its line held 23). The kit is full, so squids-live 2's regression steps with the chicken and spider eggs keep their own kit line; squids-live 3's glass platform takes the glass up to 64. The squids' steps (squids-drawn 2 and 3) keep their own kit lines too; for a test of the whole segment m2a, run them after the steps of this line.

```
IW4L_MINECRAFT_KIT=minecraft:rabbit_spawn_egg*4,minecraft:polar_bear_spawn_egg*2,minecraft:silverfish_spawn_egg*2,minecraft:zombie_spawn_egg*2,minecraft:drowned_spawn_egg*2,minecraft:cow_spawn_egg*2,minecraft:water_bucket,minecraft:glass*64,minecraft:stone*64,minecraft:iron_shovel,minecraft:iron_sword,minecraft:cod_bucket,minecraft:salmon_bucket,minecraft:tropical_fish_bucket,minecraft:pufferfish_bucket,minecraft:axolotl_bucket,minecraft:tadpole_bucket,minecraft:sulfur_cube_bucket,minecraft:sulfur_cube_bucket,minecraft:lava_bucket,minecraft:bucket*16,minecraft:cauldron,minecraft:campfire,minecraft:obsidian*10,minecraft:flint_and_steel,minecraft:dispenser*2,minecraft:lever*2
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

### Group water-buckets

#### 1. Minecraft mob buckets pour their water and let out the mob they hold

After 166f11a8.

- **javap (26.3 `client.jar`):**
  - `Items`: the cod, salmon, tropical fish and pufferfish buckets are `MobBucketItem(type, WATER, BUCKET_EMPTY_FISH)`; the axolotl's and tadpole's use `BUCKET_EMPTY_AXOLOTL` and `BUCKET_EMPTY_TADPOLE`; `sulfur_cube_bucket` is `MobBucketItem(SULFUR_CUBE, EMPTY, BUCKET_EMPTY_SULFUR_CUBE)`. All stack to 1 and start with an empty `bucket_entity_data`. `SoundEvents`: `item.bucket.empty_{fish,axolotl,tadpole,sulfur_cube}` and `item.bucket.fill_{fish,axolotl,tadpole,sulfur_cube}`.
  - `BucketItem.use`: the ray-cast uses `getFluidContext()` (`NONE` for mob buckets). `posToPlace` is the clicked block when it is a `LiquidBlockContainer` and the content is `WATER`, else the block against the clicked face. Then `emptyContents(player, level, posToPlace, hit)` (which may retry against the face), `checkExtraContent(player, level, stack, posToPlace)` with the first `posToPlace`, and `createFilledResult(stack, player, getEmptySuccessItem)`: an empty bucket, or the same stack for a player with infinite materials.
  - `MobBucketItem.emptyContents`: an `EMPTY` content only plays the empty sound and succeeds; otherwise `BucketItem.emptyContents`, whose evaporation branch hisses and never calls `playEmptySound`. `playEmptySound` plays the item's own sound (`NEUTRAL`, 1, 1) instead of `item.bucket.empty`.
  - `MobBucketItem.checkExtraContent`: on the server `spawn`, then `gameEvent(player, ENTITY_PLACE, posToPlace)`. `spawn`: `EntityType.create(level, createDefaultStackConfig(level, stack, null), posToPlace, BUCKET, true, false)`, then for a `Bucketable` `loadFromBucketTag(bucket_entity_data)` and `setFromBucket(true)`, then `addFreshEntityWithPassengers` and `playAmbientSound`.
  - `EntityType.create(.., true, false)`: the entity at `(x + 0.5, y + 1, z + 0.5)`, `getYOffset` (`1 + Shapes.collide(Y, box, collisions of AABB(pos), -1)`), `snapTo(x + 0.5, y + offset, z + 0.5, wrapDegrees(level.random.nextFloat() * 360), 0)`, head and body yaw as the yaw, `finalizeSpawn(.., BUCKET, null)`, then the stack's components and custom name (`appendDefaultStackConfig`).
  - `Bucketable.loadDefaultDataFromBucketTag`: `NoAI`, `Silent`, `NoGravity`, `Glowing` and `Invulnerable` from `getBoolean` when present, `PersistenceRequired` only when true, `Health` from `getFloat`. The axolotl also reads `Age`, `AgeLocked` and `HuntingCooldown`, the tadpole `Age` and `AgeLocked`, the sulfur cube `age` and `age_locked` (left to its group's converters).
- **Changed:**
  - Vendored world crate, new `level/mob_bucket.rs`:
    - `MobBucket` rows for the seven buckets in `MOB_BUCKETS` (item, kind, water, empty and fill sounds, the row's bucket keys, and a component converter that starts as `no_components`), with `mob_bucket(item)`.
    - `entity_data(row, components)`: the bucket's `bucket_entity_data` read as vanilla's `loadDefaultDataFromBucketTag` (flags as bytes, `PersistenceRequired` only when true, `Health` as a float), the row's keys, `CustomName` from `minecraft:custom_name`, the row's converter, and `FromBucket: 1b`. Anything else in the data (`Pos`, `UUID`, unknown keys) is ignored.
    - `Level::empty_mob_bucket(pos, at, item, components)`: `empty_bucket_into(pos, true)` for a water row (false when it cannot pour), then the mob at `at`: vanilla's landing height from the collision shapes there, a yaw from the level random, `summon_mob(kind, .., None, yaw)` (the creature finalize; an unsimulated kind gets its fresh tag), the bucket data merged in, the tag pushed to `Level.spawned`, and `entity_place` posted.
  - `world/src/level/mod.rs`: `pub mod mob_bucket;` after `pub mod dispenser;`.
  - Terrain:
    - `families.rs`: `mod water;` becomes `pub mod water;`. `families/water.rs` gains `pub mod buckets;`.
    - New `families/water/buckets.rs`: `MobBucketUse { pos, at, item, components }` and `bucket_use`. It finds the row by item and aims as a water bucket (`pour`, now `pub(crate)` with `liquid_container`). `at` is vanilla's first `posToPlace`. The empty sound becomes the row's (douse sounds and the Nether hiss stay). The sulfur cube goes against the clicked face with only its sound. The item's components travel as a tag, and the result is an empty bucket.
    - `bucket.rs`: `Change::MobBucket(MobBucketUse)` after `Server`; the first lines of `bucket_use` hand a mob bucket to `buckets::bucket_use`; `evaporated` also answers a mob bucket (the smoke); the test helper's arm.
    - `campfire.rs`: `doused` also answers a mob bucket (the douse puff).
    - `server.rs`: `Command::EmptyMobBucket { player, bucket }` after `ItemOnBlock`; `ServerHandle::empty_mob_bucket` after `item_on_block`; its dispatch after `ItemOnBlock`'s (into `out.item_uses`, acting as the player); `ServerSim::empty_mob_bucket` after `item_on_block_by`, which hands the new mob to the entity world at once (`take_spawned`; an unsimulated kind waits dormant).
  - Survival: `Entities::empty_mob_bucket(player, bucket, sent, infinite)` after `item_on_block` (in creative the answered use keeps the mob bucket), and its `actions.rs` arm after `Change::Server`.
- **Tests added:**
  - `level::mob_bucket::tests::a_cod_bucket_pours_water_and_summons_a_cod_from_the_bucket`:
    - the water is placed, and the cod stands at the block's centre on its floor;
    - `FromBucket`, `Health` 1.5 as a float, `NoAI`, `PersistenceRequired` and `Silent` are kept; `Pos`, an unknown key and another row's key are ignored;
    - its yaw is within ±180, and `fluid_place` and `entity_place` are posted;
    - the cod from a bucket without data is still `FromBucket`, and `PersistenceRequired: 0b` leaves the default.
  - `level::mob_bucket::tests::a_bucketed_mob_lands_on_what_the_water_went_into` (into a bottom slab: waterlogged, the axolotl on top at +0.5 with its `Age` and `AgeLocked`; into stone nothing pours and nothing comes out; a plain water bucket is no mob bucket).
  - `level::mob_bucket::tests::a_sulfur_cube_bucket_pours_nothing` (no water, the cube with `FromBucket`, only `entity_place`).
  - `level::mob_bucket::tests::a_fish_bucket_in_the_nether_hisses_and_still_leaves_its_fish` (no water in the Nether, the salmon still comes out; the hiss is the client's and is checked in the terrain test below).
  - `level::mob_bucket::tests::the_bucket_data_keeps_vanillas_keys_with_their_types` (number types become bytes and a float, `UUID` is dropped, `CustomName` comes from the name component; the seven rows).
  - `families::water::buckets::tests::a_fish_bucket_aims_like_water_with_its_own_splash` (through `bucket::bucket_use`: the empty bucket and the fish splash; each row's own splash; into a waterloggable block; the cube against the face; sneaking pours on top while the fish comes out where the bucket aimed).
  - `families::water::buckets::tests::a_fish_bucket_hisses_where_water_evaporates_and_douses_a_campfire` (only the hiss, the smoke and the fish; the douse sound before the fish splash).
  - `families::water::buckets::tests::a_mob_bucket_sends_its_components_and_other_items_pass`.
  - `server::tests::an_emptied_mob_bucket_spawns_its_tag_with_the_bucket_data` (the water; the cod handed over at once and dormant at the block's centre with its data; the data's `Motion` ignored; into stone nothing).
  - `entities::tests::a_friend_empties_a_fish_bucket_through_the_host` (the friend's use is answered only to the friend, with the empty bucket; the water is there for the friend's next bucket; in creative the answer keeps the fish bucket).
- **Decisions:**
  - **The seven rows live in the world crate** (`level/mob_bucket.rs`), not in terrain's `buckets.rs`. Dispensers (world crate, commit 3) empty the same buckets and cannot see terrain, so one table serves players, dispensers and the pickup. Terrain's `buckets.rs` holds the player's side (`bucket_use`, and `Bucketable` and `pickup` next).
  - **`Change::MobBucket(MobBucketUse { pos, at, item, components })`** instead of the brief's `{ pos, water, kind, data }`:
    - the item names the row, and with it the water and the kind;
    - `at` is vanilla's first `posToPlace`, where the mob comes out, which differs from the water's block when sneaking against a waterloggable block;
    - the item's components travel whole, so the level reads the bucket data, the name and later the family components the same way for players and dispensers.
  - **The yaw comes from the level random on the server** (vanilla `EntityType.create`), not from the client's view random as `hatch_egg` draws it: dispensers need the server's draw anyway.
  - **The landing height is computed on the server** with vanilla's `getYOffset` (`block_collisions` and `collide_shapes`), with the kind's spawn-row width or the spawn eggs' 0.6.
  - **A player's mob joins at once:** `ServerSim::empty_mob_bucket` takes `Level.spawned` right away, as vanilla's `addFreshEntity` does; a dispensed one waits for the tick's `take_spawned`.
  - **Creative keeps the mob bucket** (vanilla `getEmptySuccessItem`): `Entities::empty_mob_bucket` drops the answer's result for a player with infinite materials. The plain water bucket still empties in creative in this fork; that is not changed here.
  - **`pub mod water`**, not `pub(crate)`, in terrain's `families.rs`, folded into this commit as its first need: `crate::bucket` and `crate::server` call into it, and a public path keeps the hooks only later families use (`Bucketable`, `pickup`) free of dead-code warnings.
  - **The custom name** (`minecraft:custom_name`) names the mob, as vanilla's `appendDefaultStackConfig` does.
  - **The client-side tests** sit in M2's `families/water/buckets/tests.rs`, with a small world and material rules of their own, rather than in the shared `bucket.rs` test module the brief names.
- **Protocol:** no message or codec change (`Change` and `Command` are in-process).
- **Runs:**
  - `test -p minecraftoss-world --locked --lib mob_bucket`: 5 passed, also with the game data required.
  - `test -p minecraft_terrain --locked --lib -- bucket an_emptied campfire`: 56 passed.
  - `test -p minecraft_survival --locked --lib -- a_friend_empties a_friends_picks item_use`: 2 passed, the friend test also with its world required.
  - `check -p launcher --locked`: green, with only the old render_frame and skate warnings.
  - The four new files pass `rustfmt --check` with empty output.
- **Test steps.** Release build, survival, Normal, by day, flat ground in a normal world. No mob is shown yet: the fish, axolotls, tadpoles and sulfur cubes come with later segments, and until then they are kept in the world. Name each screenshot by its label.
  1. Pour the water bucket and take the water back with an empty bucket. Pour the lava bucket and take it back. Use the water bucket on the cauldron. Everything works as before.
  2. Place stone 3 blocks away and use the cod bucket on it: a water source appears, you hear the fish-bucket splash (not the plain pour), and you hold an empty bucket. **m2-buckets-1**
  3. Use the axolotl bucket and the tadpole bucket on the stone too: each pours water with its own splash. Use the sulfur cube bucket on the side of a block: no water, its own sound, and you hold an empty bucket.
  4. Place the campfire. `/gamemode creative`, then use the salmon bucket on the ground: water appears and you keep the salmon bucket. Use it on the lit campfire: the fire goes out with a hiss and a puff of smoke. `/gamemode survival`.
  5. Build a Nether portal from the obsidian and light it with the flint and steel. In the Nether, use the tropical fish bucket on the ground: it hisses with a little smoke and leaves no water, and you hold an empty bucket. **m2-buckets-2**
  6. Friend window (`scripts\mp-local.ps1`, the friend with the same kit): the friend uses the cod bucket on the ground. Both windows show the water, the friend hears the splash and holds an empty bucket. **m2-buckets-3**
- **Kit** (14 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:cod_bucket,minecraft:salmon_bucket,minecraft:tropical_fish_bucket,minecraft:axolotl_bucket,minecraft:tadpole_bucket,minecraft:sulfur_cube_bucket,minecraft:water_bucket,minecraft:lava_bucket,minecraft:bucket*16,minecraft:cauldron,minecraft:campfire,minecraft:obsidian*10,minecraft:flint_and_steel,minecraft:stone*64
```

#### 2. Minecraft fish buckets carry the mob they caught like vanilla

After 1322d5fb.

- **javap (26.3 `client.jar`):**
  - `Bucketable.bucketMobPickup(player, hand, mob)`:
    - it acts only when `canBePickedUpWithBucket(held)` (by default the held item is `WATER_BUCKET`; `SulfurCube` answers `BUCKET`) and the mob `isAlive`;
    - then `playSound(getPickupSound(), 1, 1)` at the mob in its own sound source, `getBucketItemStack()`, `saveToBucketTag(stack)`, `ItemUtils.createFilledResult(held, player, stack, false)`, `setItemInHand`, the `FILLED_BUCKET` trigger, `Leashable.dropLeash()` and `discard()`.
  - `Bucketable.saveDefaultDataToBucketTag`: `CUSTOM_NAME` is copied onto the stack. In `bucket_entity_data`, `NoAI`, `Silent`, `NoGravity`, `Glowing`, `Invulnerable` and `PersistenceRequired` are put only when true, and `Health` always, as a float.
  - What each family's `saveToBucketTag` adds:
    - `AbstractFish` saves only the default data, and its pickup sound is `item.bucket.fill_fish`;
    - the axolotl adds its variant component, `Age`, `AgeLocked` and `HuntingCooldown`;
    - the tadpole adds `Age` and `AgeLocked`;
    - the sulfur cube adds its `SULFUR_CUBE_CONTENT` component, `age` and `age_locked`.
  - `ItemUtils.createFilledResult(held, player, filled, false)`:
    - one held item is consumed (`consume` does nothing for a player with infinite materials);
    - an emptied hand takes the filled stack; otherwise the filled stack goes into the inventory, or is dropped when there is no room.
    - So creative keeps the water bucket and gains a bucket on every pickup.
- **Changed:**
  - Terrain `families/water/buckets.rs`:
    - `pub trait Bucketable: TerrainFamily`: `PICKUP_ITEM` (default `minecraft:water_bucket`), `bucket(kind)` (by default the world row of that kind) and `bucket_components(mob)` (the family's item components, none by default).
    - `pub fn pickup::<S>(world, id, actor, outcome) -> bool`, which runs as vanilla's `bucketMobPickup`:
      - only with `S::PICKUP_ITEM` in hand, on a mob that is not removed and has health;
      - the row's fill sound at the mob, at volume 1 and pitch 1, in the family's sound category;
      - the filled bucket with `bucket_entity_data` and the family's components;
      - `filled_result` in survival, or one more bucket in creative (the overflow drops where the mob was);
      - `drop_leash` with its lead drops, then `core.removed`, with no death, loot or experience.
  - World `level/mob_bucket.rs`: `bucket_of(kind)`, `NO_AI`, and `bucket_entity_data(row, no_ai, persistent, health, fields)`. It saves `NoAI` and `PersistenceRequired` only when true, `Health` as a float, and the row's keys from the family's saved fields.
- **How a family uses it** (the fish, axolotls, tadpoles and sulfur cubes):
  - `impl Bucketable for <Family> {}`. The sulfur cube sets `const PICKUP_ITEM: &'static str = "minecraft:bucket"`, and families with item components override `bucket_components`.
  - The family's `TerrainFamily::interact` calls `buckets::pickup::<Self>(world, id, actor, outcome)` first and returns true when it picked the mob up.
  - The row's keys come from the family's `TerrainFamily::fields`. When the bucket is poured, the row's `components` converter (world crate) reads the family's components back.
- **Tests added** (`families::water::buckets::tests`):
  - `a_picked_up_mob_keeps_its_health_and_vanishes_without_dying`:
    - the bucket holds `Health` 5, `NoAI` and `PersistenceRequired` plus the family's component, and the fill sound plays at the mob in its category;
    - the mob is removed with no death, and a second pickup of it fails;
    - pouring the bucket gives back the same health and flags with `FromBucket`;
    - a plain mob saves only its health.
  - `only_the_family_s_pickup_item_picks_a_mob_up`: empty, lava and cod buckets and a bare hand do nothing to the silverfish; the endermite's test impl takes only an empty bucket and fills with its own sound; a dead mob cannot be scooped.
  - `creative_keeps_the_water_bucket_and_adds_one_fish_bucket`: two pickups keep the water bucket and add two buckets, and with a full inventory the bucket drops where the mob was.
- **Decisions:**
  - **Test impls.** No water family is registered in terrain yet, so the tests give D0's silverfish (water bucket) and endermite (`PICKUP_ITEM` = empty bucket) `#[cfg(test)]` `Bucketable` impls in M2's test file. Their test-only rows (`minecraft:silverfish_bucket`, `minecraft:endermite_bucket`) are statics there, returned by the impls' `bucket`.
  - **A family's item components come from `Bucketable::bucket_components`**, not a row converter: the family's state knows them, and the pickup only runs in terrain. Pouring keeps the row's `components` converter in the world crate, where dispensers reach it.
  - **What the bucket saves.** Registered mobs keep no `Silent`, `NoGravity`, `Glowing` or `Invulnerable`, so the bucket saves `NoAI` and `PersistenceRequired` (each only when true) and always `Health`, as vanilla does for those three.
  - **The custom name is not copied onto the bucket:** `MobCore` holds no name (only the server's tag for the mob keeps it). See Left for later.
  - **With no room** the filled bucket drops where the mob was. Vanilla drops it at the player, but a mob action knows no player position.
- **Protocol:** none.
- **Runs:**
  - `test -p minecraft_terrain --locked --lib families::water::buckets`: 6 passed.
  - `check -p launcher --locked`: green, with only the old render_frame and skate warnings.
  - The new files pass `rustfmt --check` with empty output.
- **Test steps** (nothing new to see; the first family to use the pickup is `fish-buckets`, segment m2c). Release build, survival, Normal, by day.
  1. Place stone and use the silverfish and cow eggs on it. Use the empty bucket on the cow: you get milk, as before.
  2. Use the water bucket on the silverfish: it is not picked up. **m2-buckets-4**
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:water_bucket,minecraft:bucket,minecraft:cow_spawn_egg*2,minecraft:silverfish_spawn_egg*2,minecraft:stone*64
```

#### 3. Dispensers empty fish buckets like vanilla

After 6c0c1ea0.

- **javap (26.3 `client.jar`):**
  - `DispenseItemBehavior.bootStrap` registers one `DefaultDispenseItemBehavior` (`DispenseItemBehavior$3`) for the lava, water, powder snow, salmon, cod, pufferfish, tropical fish, axolotl, sulfur cube and tadpole buckets.
  - Its `execute`:
    - `emptyContents(null, level, front, null)` on the block in front;
    - on success, `checkExtraContent(null, level, stack, front)` and `consumeWithRemainder(source, stack, new ItemStack(BUCKET))`;
    - otherwise the default behaviour, which throws one item out.
  - So a dispensed mob bucket plays its own empty sound at the front block (except where the water evaporates) and lets its mob out there, with no player as the game event's source.
- **Changed (vendored world crate):**
  - `level/dispenser.rs`: one arm after `"water_bucket" | "lava_bucket"` for the six water mob buckets (cod, salmon, tropical fish, pufferfish, axolotl, tadpole). It mirrors `dispense_filled_bucket`: the front block, then `consume_with_remainder` with an empty bucket, or `default_dispense`. `OTHER_BEHAVIOURS` is untouched; the earlier arm wins.
  - `level/mob_bucket.rs`: `Level::dispense_mob_bucket(front, stack)`. It runs `empty_mob_bucket(front, front, ..)` with the stack's components, and pushes the row's empty sound as a `LevelSound` at the front block's centre, unless the water evaporates there.
- **Tests added** (`level::mob_bucket::tests`):
  - `a_dispenser_empties_a_cod_bucket_and_keeps_the_empty_bucket`:
    - the water flows out in front and the dispenser keeps one empty bucket;
    - the cod comes out with `FromBucket` and its `Health`, standing on the front block;
    - the fish splash plays at the front block, and `entity_place` is posted with no source;
    - a second pulse takes the water back into a water bucket;
    - a tadpole bucket facing stone is thrown out whole, with no mob and no sound.
  - `a_sulfur_cube_bucket_is_still_not_dispensed`: it stays in the dispenser, no mob comes out, and the level notes the unsupported behaviour.
- **Decisions:**
  - **The sulfur cube bucket stays unported in dispensers.** 26.3 dispenses it with the same behaviour (javap above), but M2-1 gives its dispenser behaviour to the sulfur cube groups. Its whole port is adding `"sulfur_cube_bucket"` to this arm: the level side already lets a cube out without water. Recorded under "Left for later".
  - **The arm is a block in `dispenser.rs`**, mirroring `dispense_filled_bucket`, so the dispenser's private helpers (`dispenser_facing`, `consume_with_remainder`, `default_dispense`) keep their visibility and no existing line changes. The mob-bucket part lives in `mob_bucket.rs`.
  - **No hiss from a dispenser** where water evaporates, as for the plain water bucket, which plays no sound from a dispenser in this fork. The mob bucket's splash plays everywhere else.
- **Protocol:** none.
- **Runs:**
  - `test -p minecraftoss-world --locked --lib`: 142 passed, among them the 7 `mob_bucket` tests.
  - `test -p minecraft_terrain --locked --lib -- dispens bucket`: 33 passed.
  - `check -p launcher --locked`: green, with only the old render_frame and skate warnings.
  - The changed new files pass `rustfmt --check` with empty output.
- **Test steps.** Release build, survival, Normal, by day, flat ground. Name each screenshot by its label.
  1. Place a dispenser on the ground facing open ground, put the pufferfish bucket in it and place a lever on the dispenser. Flip the lever: water flows out in front with the fish-bucket splash, and the dispenser now holds an empty bucket. **m2-buckets-5**
  2. Flip the lever off and on again: the empty bucket takes the water back, and the dispenser holds a water bucket.
  3. Put the sulfur cube bucket in the second dispenser with its own lever and flip it: nothing comes out, and the bucket stays in the dispenser.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:pufferfish_bucket,minecraft:sulfur_cube_bucket,minecraft:dispenser*2,minecraft:lever*2,minecraft:stone*64
```

### Group squids-live

#### 1. Registered Minecraft mobs can move by their own rules while dying

After 48f5c4bb.

- **javap (26.3 `client.jar`):**
  - `LivingEntity.aiStep`: a dying mob (`isImmobile`) runs no `serverAiStep`, but `travel(input)` still runs whenever `canSimulateMovement` and `isEffectiveAi` hold, dead or alive; the fall-distance reset under Slow Falling or Levitation comes just before it.
  - `Squid.travel` is only `move(SELF, getDeltaMovement())`, and `Squid.aiStep`'s own tail runs after it while the squid dies. So a dying squid keeps its last pulse and neither sinks nor slows by the shared water drag.
- **Changed (vendored entities crate):**
  - `world/families.rs`: `Family::OWN_DYING_TRAVEL` (false) after `MOUNT`, and `family_base_tick` runs `dying_travel` only when it is false (one changed line).
  - The test probe `SwimProbe` sets it (its tick never moves a dead probe), so the hook has a user before any squid exists.
- **Tests added** (`world::families::water::tests`):
  - `a_family_that_moves_its_own_way_while_dying_skips_the_shared_dying_travel`: a dead swim probe in water and in the air keeps its place and its push for a tick; D0's dead probe slides on, slows and sinks in water and falls in the air, as before.
- **Decisions:**
  - A const, not a method: whether a family moves its own way while dying is a class fact (`travel` overridden), the same for every mob of the family.
  - The hook test lives in M2's `water/tests.rs` (water-hooks 1's decision), not in D0's `families/tests.rs`.
- **Protocol:** none.
- **Test steps** (regression; nothing new to see). Release build, survival, Normal, by day. Name each screenshot by its label.
  1. Dig a hole 3 x 3 and 3 deep, pour the water bucket into it and use a rabbit egg on the water. Kill the rabbit with the sword: it flashes red and sinks slowly, as before. **m2-squids-1**
  2. Use the polar bear egg on the grass and kill the bear: it tips over where it stands, as before.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:rabbit_spawn_egg*2,minecraft:polar_bear_spawn_egg*2,minecraft:water_bucket,minecraft:iron_shovel,minecraft:iron_sword
```

#### 2. Minecraft squids swim in pulses, squirt ink and flee in the entity world

After 2c0d74bf.

- **javap (26.3 `client.jar`):**
  - `Squid` extends `AgeableWaterCreature`: health 10 (`createAttributes`), gravity 0.08, sound volume 0.4, `canBeLeashed` true, movement emission `EVENTS`. Baby dimensions `scalable(0.5, 0.5).withEyeHeight(0.37)`. The constructor draws `tentacleSpeed = 1 / (nextFloat + 1) * 0.2`.
  - `AgeableWaterCreature`: ambient interval 120, experience `1 + nextInt(3)`, the gills' `handleAirSupply` (water-hooks 1), `removeWhenFarAway` left at `Mob`'s true.
  - `Squid.aiStep`, after `super` (`LivingEntity.aiStep`'s travel, then `AgeableMob`'s growth):
    - the old values; `tentacleMovement += tentacleSpeed`; past `6.283185307179586` it loses `6.2831855f`, draws a new speed one time in ten (`nextInt(10)`) and sends entity event 19;
    - in water (`wasTouchingWater`): below `π` the angle is `Mth.sin(f²π)·π·0.25` with `f = tm/π`, and above `f = 0.75` the motion becomes `movementVector` with `rotateSpeed` 1, else `rotateSpeed ×0.8`; from `π` on the angle is 0, the motion `×0.9` and `rotateSpeed ×0.99`; then `yBodyRot += (−atan2(x, z)·57.295776 − yBodyRot)·0.1`, `setYRot(yBodyRot)`, `zBodyRot += π·rotateSpeed·1.5` and `xBodyRot += (−atan2(horizontal, y)·57.295776 − xBodyRot)·0.1`;
    - out of water: the angle `|Mth.sin(tm)|·π·0.25`, the motion `(0, y′·0.98, 0)` with `y′` `0.05·(amplifier + 1)` under Levitation or `y − 0.08`, and `xBodyRot += (−90 − xBodyRot)·0.02`.
  - `Squid.travel` is `move(SELF, getDeltaMovement())`.
  - `Squid.hurtServer`: when `super` succeeds and `getLastHurtByMob()` is set, `spawnInk` and true; otherwise **false**. `spawnInk`: `makeSound(squirt)` (0.4, the voice pitch), then 30 particles, each with `nextFloat` twice for the sideways spread and once for the reach (`0.3`, a baby `0.1`, plus `nextFloat·2`), rotated by `xBodyRotO` and `−yBodyRotO`, from `rotateVector(0, −1, 0) + pos` half a block up, sent with count 0 and speed 0.1.
  - `GlowSquid.hurtServer` sets `DarkTicksRemaining` to 100 only when `Squid.hurtServer` returned true, so only on a hit that squirts. `GlowSquid.aiStep` counts it down, then `addParticle(GLOW, getRandomX(0.6), getRandomY(), getRandomZ(0.6), 0, 0, 0)`: three `nextDouble` draws from the squid's random, on the server too.
  - `SquidRandomMovementGoal` (priority 0, no flags, ticks on full selector ticks): over 100 idle ticks the vector is `ZERO`; otherwise `nextInt(reducedTickDelay(50) = 25) == 0`, being out of water or `lengthSqr ≤ 1.0e-5f` draws `a = nextFloat·6.2831855f` and `(Mth.cos(a)·0.2, −0.1 + nextFloat·0.2, Mth.sin(a)·0.2)`.
  - `SquidFleeGoal` (priority 1, `requiresUpdateEveryTick`): usable in water with a last attacker under 10 blocks (`distanceToSqr < 100`); `d = squid − attacker`, used only when the block at `squid + d` holds water or is air; `d.normalize()` is discarded; `d ×= 3 − max(0, (|d| − 5)/5)` when that is above 0; no vertical part toward air; `movementVector = d/20`. Its bubbles are a server `addParticle` with no draws.
  - `LivingEntity.aiStep` resets the fall distance under Slow Falling or Levitation just before `travel`.
  - `Mob.checkDespawn` returns early for a persistent mob and resets `noActionTime` only for a player within 32 blocks.
  - `ClientboundLevelParticlesPacket` (26.3) carries float spreads and float speeds; with count 0 the client's velocity is their float product, widened.
- **Changed (vendored entities crate):**
  - `movement.rs`: `Body::move_self(world) -> Option<f64>`, vanilla's `Entity.move`, split out of `travel_air_flying`: the stuck multiplier, the collision, the ground and supporting block, the water check, the fall (its landing distance returned), the restitution and the movement emission. `travel_air_flying` calls it; its climbing push and drag moved above the call unchanged, so every walker, flyer and swimmer moves as before.
  - New `world/families/water/squids.rs`, the family `Squid` (key 64; `minecraft:squid` and `minecraft:glow_squid` in one store, `glow` set by kind), with the state the brief lists: `age`, the body rotations and their old values, the tentacle movement and angle with their old values, the tentacle and rotate speeds, `movement_vector`, `fleeing`, `ink_spurts` and `dark_ticks`.
    - Hooks: 0.8 by 0.8, eyes 0.4, health 10, ambient interval 120; categories `water_creature` and `underground_water_creature` (D0's default despawn: 128, removable, allowed on peaceful); experience 1-3 for an adult; `breathes_water`, `leashable`; `Breath::Gills`; `emits_sounds` false; voice `(squid | glow_squid, 0.4, neutral_volume)`; `OWN_DYING_TRAVEL`.
    - `hurt`: `hurt_plain`, then on an applied hit with `hurt.attacker` or `core.hurt_by` set the squirt: its voice, the ink's 90 float draws (`ink`), `ink_spurts + 1`, and for a glow squid `dark_ticks = 100`.
    - The tick, in vanilla's order: the first tick draws the tentacle speed; `family_base_tick`; the velocity trim; for a living squid with AI the idle clock and both goals; for any squid with AI the fall-distance reset under Levitation or Slow Falling, `move_self`, its fall damage and its splash (`play_movement` with the silent sounds); `family_after_move`; growth (a baby resizes at the boundary); the stroke; the glow squid's countdown and three draws.
    - Public helpers for `squids-drawn`: `set_baby`, `rotate_vector`, `ink` (the 30 specks' origins and velocities from a random) and `tentacle_speed`.
  - `world/families/water.rs`: `pub mod squids;` and `squids: squids::Squid,` in `family_group!(water 64..=79 { .. })`.
- **Tests added** (`world::families::water::squids::tests`):
  - `a_squid_is_0_8_with_ten_health_and_its_kind`: both kinds and a baby (size, eyes, health, key, census and `mob_candidates`, categories, traits and voices, gills, silent movement, the despawn rule); adults leave their own table and 1-3 experience, a baby nothing.
  - `a_baby_squid_is_half_a_block_and_grows_up_to_full_size` (and a locked baby stays small).
  - `a_squid_pulses_only_in_the_last_quarter_of_its_tentacle_stroke`: with a speed of 0.1, 23 ticks of waiting, 8 pulses at `movementVector` with `rotateSpeed` 1 and 31 glides at `×0.9`; the angle `sin(f²π)·π/4`, then 0.
  - `a_squid_picks_a_new_heading_on_full_ticks_and_stops_after_a_hundred_idle_ticks`: 0.2 sideways and −0.1 to 0.1 up; only on full ticks; on every full tick out of water; `ZERO` after 100 idle ticks until a hit wakes it.
  - `a_squid_turns_tilts_and_rolls_with_its_motion`: yaw −90 moving east and +90 moving west, the head with it; tilt −90 sideways and 0 rising; the roll; the old values; on land it tips toward −90 by 2% a tick and keeps its yaw.
  - `a_hit_squid_squirts_and_flees_up_to_a_block_a_tick`: the hurt voice, then the squirt at 0.4; the vector at 1, 5, 9.5 and 9.99 blocks; none at 10 or on land; no rise toward air; no way into stone.
  - `a_squid_hurt_again_within_five_seconds_squirts_even_without_an_attacker`: a hit with nobody to blame does not ink; within 100 ticks of the player's hit one does, after them not; a mob's hit inks.
  - `a_squid_on_land_falls_curls_and_dries_out`: gravity and the 0.98 drag; Levitation lifts it; a fall of 6.5 blocks hurts 3 with the small fall sound and no ink; dry, it is hurt at ticks 320, 340, 360 and 380 and dies at 400; it lies flat.
  - `a_glow_squid_goes_dark_for_a_hundred_ticks_when_it_squirts`: not on a hit with nobody to blame; 100 counted down to 0; a plain squid never; the glow squid's three draws a tick.
  - `a_dying_squid_drifts_on_its_last_pulse_and_does_not_sink`.
  - `squids_are_silent_when_they_swim_and_call_every_120_ticks_or_so`: one splash falling in; no swim or step sound but `minecraft:swim` game events; ambient calls at 0.4, more than 120 ticks apart.
  - `a_leashed_squid_is_pulled_after_its_player`: pulled to within 7 blocks; the lead snaps past 12.
- **Decisions:**
  - **The yaw is the body yaw.** `Squid.aiStep` sets `yRot` from `yBodyRot` every tick in water, so the state keeps only `y_body_rot_o` (the yaw at the stroke's start) and `core.yaw` is `yBodyRot`. The head yaw follows the body: a squid has no head to turn.
  - **The tentacle speed is drawn on the first tick**, as the first draw of the squid's own random, instead of in `spawn`: the loader seeds `core.random` from the UUID after `spawn`, which would give every squid the same first speed.
  - **The glow squid goes dark only when it squirts** (javap above), where the brief had any applied hit; the test name says so.
  - **The hit's result stays applied** where vanilla's `Squid.hurtServer` returns false (a hit with no last attacker). That return only matters to an arrow without an owner, which would bounce off; arrows, knockback and the hurt flash act as for every mob.
  - **The server keeps the random draws:** the squirt's 90 float draws and the glow squid's three doubles a tick are drawn and dropped, so the squid's random runs as vanilla's. Each view draws its own ink from `ink_spurts` (`squids-drawn`, the brief's decision 4).
  - **Not ported:** entity event 19 (friends get the tentacle values through `FamilyShow` in `squids-drawn`, decision 3) and the flee bubbles (a server `addParticle`, shown to nobody).
  - **The idle reset near players** is D0's `check_despawn`, which runs before the mobs tick, as for every registered mob. It also resets a persistent squid near a player, where 26.3 would let it go still.
  - **Leads:** `leash_walk` stays D0's no-op (a squid's navigation never moves it); the elastic pull moves it through its velocity.
  - **Game events:** a squid's moves post `minecraft:swim` through `move_self`'s emission (`EVENTS`), drained by `take_game_events`, so no line was needed (M20, W's game events).
- **Left for later:** Impaling's extra damage to squids (`#sensitive_to_impaling`) is not in the fork for any mob yet.
- **Protocol:** none.
- **Terrain:** no terrain registration yet (`squids-drawn`), so squids in the game stay dormant. D0's terrain test `server_mobs::tests::every_registered_kind_is_simulated_and_summonable`, which compares the two registries, fails from this commit until `squids-drawn` 2 registers `Squid` in terrain; segment m2a merges only with both groups in, as M1's rabbits did.
- **Runs:** `test -p minecraftoss-entities --locked --lib`: 598 passed and one failed, `squids_are_silent_..`, whose ambient count filtered the calls by position while the squid had swum past the filter's line; the test now runs the diving squid and the calling squid in two worlds, and passes. `check -p launcher --locked`: green, with only the old render_frame and skate warnings. The new files pass `rustfmt --check --edition 2021` with empty output.
- **Test steps** (regression; squids stay dormant until `squids-drawn`, and every other mob now moves through `move_self`). Release build, survival, Normal, by day, flat ground. Name each screenshot by its label.
  1. Build a stone tower 5 high. Use a zombie egg on its top and walk away: the zombie follows you down off the edge, falls and is hurt by the fall, as before. **m2-squids-2**
  2. Use a chicken egg on the tower's top and push the chicken off: it flaps and floats down slowly.
  3. Use a spider egg beside the tower: it climbs the tower's side.
  4. Dig a pool 3 x 3 and 2 deep, pour the water bucket into it and use a cow egg in it: the cow swims and climbs out.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:zombie_spawn_egg*2,minecraft:chicken_spawn_egg*2,minecraft:spider_spawn_egg*2,minecraft:cow_spawn_egg*2,minecraft:stone*64,minecraft:water_bucket,minecraft:iron_shovel,minecraft:iron_sword
```

#### 3. Minecraft squids spawn in oceans and rivers, and glow squids in dark water underground

After ddb173f4.

- **javap (26.3 `client.jar`):**
  - `SpawnPlacements`: `SQUID` and `GLOW_SQUID` are `IN_WATER` with `MOTION_BLOCKING_NO_LEAVES`; the squid's predicate is `AgeableWaterCreature.checkSurfaceAgeableWaterCreatureSpawnRules`, the glow squid's `GlowSquid.checkGlowSquidSpawnRules`.
  - `checkSurfaceAgeableWaterCreatureSpawnRules`: `seaLevel − 13 ≤ y ≤ seaLevel`, the fluid below in `#water`, and the block above `Blocks.WATER`. It reads no reason, so spawners use it too; `WaterAnimal.checkSurfaceWaterAnimalSpawnRules` is the same test.
  - `checkGlowSquidSpawnRules`: `y ≤ seaLevel − 33`, `getRawBrightness(pos, 0) == 0` and the block `Blocks.WATER`, with no random draw.
  - `AgeableWaterCreature.checkSpawnObstruction` (and `WaterAnimal`'s, `Axolotl`'s, `Guardian`'s, `Drowned`'s) is `isUnobstructed` alone: water in the box never blocks a spawn of these types.
  - `Squid.finalizeSpawn`: `AgeableMobGroupData(0.05)`. `PathfinderMob.getWalkTargetValue` 0, `getMaxSpawnClusterSize` 4, `removeWhenFarAway` true; the follow range is `Mob`'s 16.
- **Changed (vendored world crate):**
  - `natural_spawner/families/water.rs`: the rows of `minecraft:squid` (`WaterCreature`, the rule `surface_water_animal`) and `minecraft:glow_squid` (`UndergroundWaterCreature`, the rule `dark_deep_water`), both 0.8 by 0.8, `InWater`, `MotionBlockingNoLeaves`, `Rules::Family { creature: true }`, `Walk::Neutral`, removed far away, clusters of 4, follow range 16, baby chance 0.05, no finalize of their own. `surface_water_animal` is the shared rule for the fish and dolphins to come (M2-3, M2-4); both rules read the level's sea level.
  - `natural_spawner.rs` `check_spawn_obstruction`: `|| info.placement == Placement::InWater` beside `Rules::Strider | Rules::Drowned` (one changed line), so every water row and the built-in axolotl skip the fluid test.
- **What changes in the game:** squids now spawn in their oceans and rivers and glow squids in unlit water below y 30, and `/summon` finalizes both as creatures. They wait dormant (counted toward their caps, saved, despawned when far) until `squids-drawn` registers them in terrain. The glow squid's rule was already ported, but the fluid in its box refused every spawn and its finalize fell into the monster catch-all with a note; the row fixes both. The built-in axolotl row keeps `Rules::Other`, so axolotls still do not spawn until their family's row (M2-7).
- **Tests added** (`natural_spawner::families::water::tests`, a new test file beside the rows):
  - `surface_water_animals_spawn_from_thirteen_below_sea_level_up_to_it`: the squid's row; y 50 to 63 with water above and below, by the level's sea level; not with air above or stone below; spawners the same.
  - `squids_spawn_in_the_top_thirteen_blocks_of_open_water`: natural spawning in a lukewarm ocean from starts at y 50, 56 and 62, none from 35, 49 or 63; nothing unsupported noted.
  - `glow_squids_spawn_only_in_unlit_water_thirty_three_below_sea_level`: y 30 and not 31, not in stone, not in light 1; natural spawning at y 20 and none at 40.
  - `a_squid_group_starts_with_an_adult_and_has_babies_one_time_in_twenty`: 300 natural calls in a warm ocean; each call's first squid is an adult, at most four a call, babies (`Age` −24000) under one in ten.
  - `water_spawns_ignore_the_water_in_their_box`: the squid, glow squid, axolotl and drowned rows pass `check_spawn_obstruction` in water; a cow does not, and does above the sea.
  - `summoning_a_squid_notes_nothing_unsupported`: both kinds summon as adults with no note.
- **Decisions:**
  - **One `InWater` test** in `check_spawn_obstruction` rather than a flag per row (brief decision 6): javap shows `isUnobstructed` for every `IN_WATER` type.
  - **The built-in glow squid lines stay** (`type_info`, `Rules::GlowSquid`, `MobCategory::of_type`): rows are read first, so they are never reached, and leaving them keeps the merge small.
  - **The tests' level** is a column of water over stone (`Ocean`) with its own light and sea level, in M2's own test file.
- **Protocol:** none.
- **Game events still to add:** none.
- **Runs:** `test -p minecraftoss-world --locked --lib`: 148 passed, among them the 6 new tests, which also passed in a run that fails when the game data is missing (so none skipped). `check -p launcher --locked`: green, with only the old render_frame and skate warnings. The new test file passes `rustfmt --check` with empty output.
- **Test steps** (nothing new to see: the squids spawn but wait dormant until `squids-drawn`). Release build, survival, Normal, by day. Name each screenshot by its label.
  1. `/locate biome minecraft:lukewarm_ocean`, `/tp` there and build a 5 x 5 glass platform on the sea surface. Stand on it for a minute: the game runs as before and no message about squids appears. **m2-squids-3**
  2. `/summon minecraft:squid ~ ~-2 ~4` and `/summon minecraft:glow_squid ~ ~-3 ~4`: both commands are answered as summoned, and nothing is drawn yet.
  3. Save, quit and load the world: it loads as before.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:glass*64
```

### Group squids-drawn

#### 1. Registered Minecraft mobs can have a tracking range per kind

After efd113a9.

- **javap (26.3 `client.jar`):** `EntityType.SQUID` is built with `clientTrackingRange(8)` and `GLOW_SQUID` with `clientTrackingRange(10)`, so one family needs two ranges; `ELDER_GUARDIAN` (10) beside `GUARDIAN` (8) is the next family that does.
- **Changed (terrain crate):**
  - `families.rs`: `TerrainFamily::tracking_range(kind) -> i32`, by default the family's `TRACKING_RANGE`, after `CAGE_MARGIN`; `terrain_group!`'s `tracking_range` asks it with the family's own `KINDS` entry (a new private `named` beside `names` finds it with or without `minecraft:`). Every family answers as before.
  - `families/water.rs`: `#[cfg(test)] mod tests;`.
- **Added (terrain crate):** `families/water/tests.rs` (test only).
- **Tests added** (`families::water::tests`):
  - `a_family_s_tracking_range_by_kind_defaults_to_its_constant`: silverfish and endermites at 8 and breezes at 10 through the hook, with or without `minecraft:`; every registered kind has a range and `mob_tracking::tracking_range` gives it; an unregistered kind has none from the families and keeps its own (a zombie 8).
- **Decisions:**
  - **The hook takes the `KINDS` entry**, a `&'static str` with `minecraft:`, so a family compares it with its own constants; the macro, not the family, accepts the short name.
  - **The hook test lives in M2's terrain `families/water/tests.rs`**, not in D0's `families/tests.rs` where the brief put it (the same choice as water-hooks 1 for the entity hooks), so D0's file stays as merged. It covers every registered family of this branch (silverfish, endermites, breezes, rabbits, polar bears and Margit), not only D0's two.
- **Protocol:** none.
- **Test steps** (regression; nothing new to see). Two windows (`scripts\mp-local.ps1`), survival, Normal, by day. Name each screenshot by its label.
  1. The host uses a rabbit egg and a silverfish egg beside the friend: the friend sees both, as before. **m2-squids-4**
  2. The host walks 100 blocks away and back: the friend still sees both, as before.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:rabbit_spawn_egg*2,minecraft:silverfish_spawn_egg*2
```

#### 2. Minecraft squids and glow squids are saved, hatch from eggs and are drawn like vanilla

After d11758b3 (its subject starts with a byte-order mark that PowerShell put in front of the message; the text is right, and the commit was not amended).

- **javap (26.3 `client.jar`):**
  - `SquidRenderer extends AgeableMobRenderer(SquidModel, BabySquidModel, shadow 0.7)`; textures `entity/squid/{squid,squid_baby}` and, in `GlowSquidRenderer`, `entity/squid/{glow_squid,glow_squid_baby}`, picked by `isBaby`.
  - `extractRenderState` lerps `oldTentacleAngle`/`tentacleAngle`, `xBodyRotO`/`xBodyRot` and `zBodyRotO`/`zBodyRot` by the partial tick; the body yaw is `LivingEntityRenderer`'s `rotLerp(yBodyRotO, yBodyRot)`.
  - `setupRotations` replaces the living one (no death flip): `translate(0, 0.5 | baby 0.25, 0)`, `YP(180 − bodyRot)`, `XP(xBodyRot)`, `YP(zBodyRot)`, `translate(0, −1.2 | baby −0.6, 0)`; `LivingEntityRenderer.submit` then scales `(−1, −1, 1)` and moves `−1.501`. `state.scale` is the entity's scale attribute (1): the baby is its own model, not a scaled adult.
  - `LayerDefinitions`: `SQUID`/`GLOW_SQUID` bake `SquidModel.createBodyLayer` and `SQUID_BABY`/`GLOW_SQUID_BABY` `BabySquidModel.createBodyLayer`.
  - `SquidModel` (64 x 32): `body` `texOffs(0,0)` box `(−6,−8,−6) 12×16×12` grown by 0.02, at `(0,8,0)`; eight `tentacle<i>` `texOffs(48,0)` box `(−1,0,−1) 2×18×2` at `(cos a·5, 15, sin a·5)` with `a = i·π·2/8` in double, turned `yRot = (float)(i·π·−2/8 + π/2)`; `setupAnim` sets every tentacle's `xRot` to `tentacleAngle`.
  - `BabySquidModel` (32 x 32): `body` box `(−4,−5,−4) 8×10×8` at `(0,13,0)`, no growth; tentacles `texOffs(0,18)` box `(−1,−0.5,−1) 2×6×2` at `(cos a·3, 18.5, sin a·3)`.
  - `GlowSquidRenderer.getBlockLightLevel`: `i = (int) Mth.clampedLerp(1 − darkTicks/10, 0, 15)`; 15 when `i` is 15, else `max(i, the normal block light)`. The sky light is the normal one.
  - `AgeableMob.mobInteract`: the golden dandelion on a baby not in `#cannot_be_age_locked` (zombie and skeleton horses, villagers) toggles the lock, resets the age to the baby start and waits 40 ticks; `Squid` has no `mobInteract` of its own and no food.
  - `AgeableMob.addAdditionalSaveData`: `Age`, `ForcedAge`, `AgeLocked`; `GlowSquid` adds `DarkTicksRemaining`.
  - The squid and glow squid spawn eggs and `/summon` go through the generic `spawn_saved`; the spawner cage shows the display mob unticked (`SpawnerRenderer`: scale 0.53125, 30° tilt), so its tentacles hang below the cage.
- **Changed (terrain crate):**
  - New `families/water/squids.rs`, `impl TerrainFamily for Squid`:
    - `TRACKING_RANGE` 8, with `tracking_range` 10 for the glow squid (squids-drawn 1); `SHADOW_RADIUS` 0.7; the 4 textures; `CAGE_MARGIN` 0.5 (the hanging tentacles); `VOLATILE` `AgeLocked` and `DarkTicksRemaining` (`Age` is in the base list).
    - `read`: `Age` (the look tag's `Baby` gives the baby start), `ForcedAge`, `AgeLocked`, the glow squid's `DarkTicksRemaining`; `y_body_rot_o` takes the saved yaw; a baby gets its 0.5 box. `fields` writes the same keys.
    - `show`: `FamilyShow { flags 0, values [tentacle angle, x body rot, z body rot, body yaw (f32 bits); ink spurts; dark ticks; 0; 0] }`. `show_on` shifts each angle into its old field, then stores the new ones, so the host and a friend lerp the same pairs.
    - `draw`: the models into `out.culled`.
    - `interact`: a golden dandelion on a living baby locks or unlocks its age through `animal::interact` (no food), with the lock sound at the squid; the generic lead arm handles leads.
  - New `families/water/squids/render.rs`: the adult and baby models, `stance` (the four lerped values), `model_root` (the spawner cage's turn, then vanilla's `setupRotations`, then the living flip), the glow squid's `block_light`, the red hurt and death overlay without a death flip.
  - `families/water.rs`: `mod squids;` and `squids::Squid` in `terrain_group!(water { .. })`.
- **Added (terrain crate):** `families/water/squids.rs`, `families/water/squids/render.rs`, and their test files `families/water/squids/tests.rs` and `families/water/squids/render/tests.rs`.
- **What changes in the game:** squids and glow squids are live: they spawn in their water, hatch from their eggs, answer `/summon`, swim, ink and flee as the entity world runs them, are saved, are seen by friends, and show in a spawner. The ink cloud and the glow squid's sparkles follow in commit 3. D0's terrain test `server_mobs::tests::every_registered_kind_is_simulated_and_summonable` passes again from this commit.
- **Tests added:**
  - `families::water::squids::tests`:
    - `a_squid_saves_and_loads_with_its_age_and_a_glow_squid_with_its_dark_ticks`
    - `a_friend_sees_a_squid_from_its_look_tag`
    - `a_squid_tracks_at_eight_chunks_and_a_glow_squid_at_ten` (a squid 150 blocks away is not tracked, a glow squid is)
    - `a_friend_sees_a_squid_pulse_tilt_and_roll_in_the_same_pose` (two ticks of shows; the same eight values and the same mesh at partials 0, 0.5 and 1)
    - `a_golden_dandelion_locks_a_baby_squid_s_age` (lock, the 40-tick wait, unlock; not an adult, not food, not a dying baby)
  - `families::water::squids::render::tests`:
    - `an_adult_squid_has_a_body_and_eight_tentacles_on_64_by_32_and_a_baby_on_32_by_32`
    - `every_squid_picks_its_texture_by_kind_and_age`
    - `a_dying_squid_flashes_red_but_does_not_tip_over`
    - `a_glow_squid_is_full_bright_and_dims_in_its_last_ten_dark_ticks`
- **Decisions:**
  - **The host computes the tentacles, tilt and roll** and sends them in `FamilyShow` (brief decision 3); vanilla's client runs its own `aiStep` with its own random speed, which two windows cannot match. The values travel as f32 bits, so both windows lerp identical pairs.
  - **The drawn yaw is the squid's own pair** `y_body_rot_o`/`core.yaw` (brief decision 5), not `MobPose.body_rot`, which eases a still mob toward its head.
  - **The old values are shifted in `show_on`**, as eight values hold no room for four more; a friend that misses a tick lerps from the last values it got.
  - **The cage margin is 0.5**: vanilla draws the unticked display squid with its tentacles below the spawner, as here.
  - **The golden dandelion** goes through the shared `animal::interact` with no food and a dummy love clock, as rabbits and polar bears do; its lock particles are dropped like theirs.
- **Protocol:** none (`FamilyShow` is D0's field).
- **Runs:** `test -p minecraft_terrain --locked --lib`: 1723 passed (3 ignored, as before), among them the 9 new tests, D0's `every_registered_kind_is_simulated_and_summonable` (green again), `every_registered_kind_is_shown_inside_its_cage` with both squids and the eggs survey `every_spawn_egg_of_a_mob_the_game_has_hatches_that_mob` with the game data present. `check -p launcher --locked`: green, with only the old skate warnings. The new files pass `rustfmt --check --edition 2021` with empty output.
- **State doc changes:** `docs/MINECRAFT-SURVIVAL.md`: "Squids and glow squids are live: they spawn in oceans and rivers (glow squids in unlit water 33 blocks below sea level), hatch from their eggs, swim in pulses, squirt and flee when hit, dry out on land, grow up (a golden dandelion locks a baby) and are saved; a glow squid glows at full brightness and goes dark for five seconds after it squirts. Friends see the same strokes, tilts and rolls."
- **Test steps.** Release build, survival, Normal, by day. `/locate biome minecraft:lukewarm_ocean`, `/tp` there, and stand on a 5 x 5 glass platform on the sea surface. Name each screenshot by its label.
  1. Use the squid egg on the water 6 blocks away three times. Dark blue squids swim in pulses: the eight tentacles snap shut, the squid shoots forward, then it glides, slows and slowly rolls, tilted toward where it goes. **squid1_swim**
  2. Hit one with the sword: it darts away fast with a squirt sound (the ink cloud comes in the next commit). Shoot one with the bow and one with the M4A1 from 8 blocks: each flees. **squid2_flee**
  3. Kill one: it flashes red without tipping over, vanishes in a poof, and leaves 1-3 ink sacs and an orb.
  4. Use an egg on the glass. The squid lies there with its tentacles beating and slowly tips flat. At about 16 s it flashes red, then once a second, and it dies at about 20 s. **squid4_land**
  5. `/summon minecraft:squid ~ ~-2 ~4 {Age:-24000}` gives a small, round baby with short tentacles. Use a golden dandelion on it and listen for the lock sound. Kill a second baby: no drops and no orb. **squid5_baby**
  6. Put a lead on a squid and walk along the platform: it is pulled after you. At 13 blocks the lead snaps.
  7. `/time set midnight`. Use the glow squid egg on the water: a teal squid glows at full brightness. Hit it: it goes as dark as the water around it for 5 s, then brightens again within half a second. **squid7_glow** `/time set day`.
  8. Leave two squids, the locked baby and a glow squid alive. Save, quit and load: all four are there and swimming, and the baby is still small.
  9. `/gamemode creative`, place the spawner and use a squid egg on it: a small squid turns in the cage, its tentacles hanging below it. `/gamemode survival`. **squid9_cage**
  10. Two windows (`scripts\mp-local.ps1`). The host hatches two squids and a glow squid. The friend sees the same pulses, tilts, rolls and tentacle beats at the same moments, the glow and the glow squid going dark when the friend hits it. **squid10-host**, **squid10-friend**
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:squid_spawn_egg*16,minecraft:glow_squid_spawn_egg*16,minecraft:iron_sword,minecraft:bow,minecraft:arrow*64,minecraft:lead*4,minecraft:golden_dandelion*4,minecraft:glass*64,minecraft:spawner,minecraft:cooked_beef*32
```

#### 3. Minecraft squids leave ink clouds and glow squids sparkle, for friends too

After 93f77e9b.

- **javap (26.3 `client.jar`):**
  - `SquidInkParticle extends SimpleAnimatedParticle`: no velocity jitter (the 4-argument `Particle` constructor), the sent velocity as is, friction 0.92, `quadSize` 0.5, no physics, gravity 0; colour `0xFF000000` for `squid_ink` and `ARGB.colorFromFloat(1, 0.2, 0.8, 0.6)` for `glow_squid_ink`; lifetime `(int)(quadSize·12 / (nextFloat·0.8 + 0.2))`, 6 to 30 ticks; sprites `generic_7` down to `generic_0` by age (`particles/squid_ink.json`, `glow_squid_ink.json`).
  - Its `tick`: `SimpleAnimatedParticle.tick` (`Particle.tick`, the sprite, and after half its life `alpha = 1 − (age − life/2)/life`), then, unless removed, `yd −= 0.0074` (the double `0.007400000002235174`) when the block at its position `isAir`.
  - `SimpleAnimatedParticle`: the `TRANSLUCENT` layer and `getLightCoords` 15728880, full bright.
  - `GlowParticle$GlowSquidProvider`: `GlowParticle(x, y, z, 0.5 − nextDouble, yd, 0.5 − nextDouble)` (the jittered `Particle` constructor), colour `(0.6, 1, 0.8)` or `(0.08, 0.4, 0.4)` by `nextBoolean`, `yd ×0.2`, and `xd`, `zd ×0.1` when the sent `xd` and `zd` are 0; lifetime `(int)(8 / (nextDouble·0.8 + 0.2))`, 8 to 40 ticks. `GlowParticle`: friction 0.96, `speedUpWhenYMotionIsBlocked`, `quadSize ×0.75`, no physics, the `OPAQUE` layer, sprite `glow`, and `getLightCoords` through `LightCoordsUtil.addSmoothBlockEmission(age + partial)/lifetime`, the flame's rule.
  - `GlowSquid.aiStep` adds one `GLOW` at `getRandomX(0.6)`, `getRandomY()`, `getRandomZ(0.6)` with no velocity every tick, on the client, dark or not and while it dies.
  - `Squid.spawnInk` (squids-live 2) sends 30 `squid_ink` or `glow_squid_ink` with count 0, so the client adds each at the origin with `speed · spread` as its velocity.
- **Changed (terrain crate):**
  - `ambient_particles.rs` (each line new and of its own, after the flame group, which track D's small and soul flames joined after `Flame` since the brief):
    - `Kind::SquidInk`, `Kind::GlowSquidInk` and `Kind::Glow` after `SoulFireFlame`;
    - their constructors `squid_ink` and `glow_sparkle` after `flame`;
    - their arms after the flame arms in `add`, `sprite` (the generic sheet backwards for the ink, `glow`), `size_at` (their own quad size) and the light (full bright for the ink, the flame's `glow()` on the cell's block light for the glow);
    - the tick: the ink's arm after `Kind::Flame | Kind::SmallFlame | Kind::SoulFireFlame => particle.rise(),` runs `coast()`, `fade_after_half_life()` and the sink in air; the glow's arm runs `coast()` and the speed-up when its height did not change. Nothing inside `coast`'s own match;
    - the two inks on new lines of `translucent`, and `glow` chained into `textures()`.
  - `families/water/squids.rs`: `client_effects`. Each view keeps the last ink counter it saw per squid in `ClientFx.seen`; on first sight it only records it; each new squirt (up to 4 at once) draws 30 specks from `squids::ink` with the view's own random, at the squid's place and turn at the start of the tick (`previous_position`, `x_body_rot_o`, `y_body_rot_o`), as `Squid.spawnInk` used them. A glow squid adds one `Glow` a tick in its box.
- **Tests added:**
  - `ambient_particles::tests::squid_ink_is_black_glow_ink_teal_and_sinks_slowly_in_air`
  - `ambient_particles::tests::a_glow_sparkle_drifts_brightens_and_lives_eight_to_forty_ticks`
  - `families::water::squids::tests::a_squirt_draws_thirty_specks_behind_the_squid_once_and_none_on_first_sight`
  - `families::water::squids::tests::a_friend_sees_a_squid_squirt_and_a_glow_squid_go_dark_at_the_same_tick`
- **Decisions:**
  - **Ink by counter** (brief decision 4): the host sends the squirt count in `FamilyShow`, each view draws its own 30 specks. They start in the same place in both windows and spread with each view's random.
  - **The ink's origin is the squid's place at the start of the tick** (`previous_position` with the old turn) rather than where it is after the tick's move, so it matches the turn `spawnInk` used and is the same for the host and a friend.
  - **Up to 4 squirts are drawn at once** for one squid, a bound for a view that missed many ticks.
  - **The anchors moved:** the brief put M2's kinds after `Flame`; D's merged `SmallFlame` and `SoulFireFlame` now follow it and share its arms, so M2's kinds and arms go after that group. The friend test sits in the squids' own tests, not in D0's `mob_tracking.rs`, which a family track does not edit.
  - **`glow` joins `textures()` on a chained line of its own** after the name list, so that long line, which other tracks extend, stays as merged.
- **Protocol:** none.
- **Runs:** `test -p minecraft_terrain --locked --lib`: 1727 passed (3 ignored, as before), among them the 4 new tests and the particle tests of the other kinds. `check -p launcher --locked`: green, with only the old render_frame and skate warnings. The squid files pass `rustfmt --check --edition 2021` with empty output; `ambient_particles.rs` keeps its own style (not reformatted).
- **State doc changes:** `docs/MINECRAFT-SURVIVAL.md`: "A hit squid leaves a black ink cloud (a glow squid a teal one) that drifts, fades and sinks slowly in air; a glow squid sheds tiny sparkles. Friends see the ink in the same place."
- **Test steps.** Release build, survival, Normal, by day. `/locate biome minecraft:lukewarm_ocean`, `/tp` there, and stand on a 5 x 5 glass platform on the sea surface. Name each screenshot by its label.
  1. Use the squid egg on the water 6 blocks away twice. Hit one with the sword: a black ink cloud bursts from its rear with a squirt sound, and it darts away. Shoot the other with the bow and then with the M4A1 from 8 blocks: it inks at each hit and flees. **squid11_ink**
  2. Use a squid egg on the glass and hit it: the ink puffs out behind it, drifts, fades and sinks slowly through the air. **squid12_ink_air**
  3. `/time set midnight`. Use the glow squid egg on the water: tiny teal and pale green sparkles drift off it and brighten before they vanish. **squid13_sparkles** Hit it: a teal ink cloud, and it goes dark for 5 s while the sparkles go on. `/time set day`.
  4. Two windows (`scripts\mp-local.ps1`). The host hatches a squid and a glow squid.
     - The host hits the squid: the friend sees the ink cloud in the same place at the same moment and hears the squirt.
     - The friend hits the glow squid: teal ink and the glow squid going dark in both windows; it flees from the friend. Both see its sparkles.
     - The friend kills the squid with the M4A1: the ink sacs drop in both windows.
     - **squid14-host**, **squid14-friend**
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:squid_spawn_egg*8,minecraft:glow_squid_spawn_egg*4,minecraft:iron_sword,minecraft:bow,minecraft:arrow*32,minecraft:glass*64
```

### Review of segment m2a

The adversarial review of the five m2a groups (r10-m2a-base..8809844c). Each fix is its own commit below.

#### 1. The swim probe's dying test keeps rustfmt's layout

After 8809844c.

- **Defect:** `third_party/minecraftoss/entities/src/world/families/water/tests.rs` is a new file of this track, but `git show :<path> | rustfmt --check --edition 2021`, run in the worktree with the repo's `rustfmt.toml` (style edition 2024), printed a diff. Squids-live 1 (2c0d74bf) had added `assert!(!Probe::OWN_DYING_TRAVEL, "the shared dying travel by default");`, whose arguments are wider than rustfmt's call width. The earlier checks of this file ran without the repo's config, where that line passes.
- **Changed:** only that assertion, now split over four lines as rustfmt writes it. Nothing else changes.
- **Checked:** every new `.rs` file of the track passes the check with empty output, both from the worktree and without the repo's config.
- **Also found, not fixable here:** d11758b3's subject starts with a byte-order mark (noted under squids-drawn 2). Removing it means rewriting that commit, which a track run must not do; it is left for the owner to decide at the merge.
- **Runs:** `test -p minecraftoss-entities --locked --lib world::families::water`: 19 passed. The staged file passes `rustfmt --check --edition 2021` with empty output.
- **Test steps:** none in game (a test file only). No kit is needed.
