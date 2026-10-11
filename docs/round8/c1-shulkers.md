# Track C, item c1: shulkers and levitation (design brief)

Paths and lines are as of e2313625. Vanilla facts were checked with javap on the obfuscation-free 26.3 `client.jar` and with the 26.3 data and assets under `iw4l-artifacts/minecraft-26.3`; no Mojang code was copied. Shulkers and their bullets live in the `EntityWorld` of their `Level`. Levitation and slow falling live on each `Player` (per UUID: `Player.survival.status`) and reach that player's MW2 `PlayerState`. Nothing here adds global or "player 0" state; the places that still go through `PLAYER = 0` are named in section 6.

## 1. The owner's request

> Shulkers and levitation: the vanilla Shulker (attaches to block faces, peeks/opens its shell, shoots ShulkerBullet homing projectiles that give Levitation 10 s, teleports when hurt/when its block is gone, armour 20 when closed, colours, drops shulker shell 50% + looting), the ShulkerBullet movement, the Levitation effect on players (MW2 movement must float up at (0.05*(amp+1) - vy)*0.2 per tick), slow falling interplay, and renderers (shulker model with lid animation, bullet). Note: shulkers spawn in end cities (track C2), coordinate.

This is the shulker and levitation part of X6 in `docs/MINECRAFT-SURVIVAL.md:282` ("shulkers don't peek, shoot homing bullets or teleport, there is no levitation").

## 2. What exists today

| Area | State |
|---|---|
| Shulker entity | None in the entity world. `shulker` is not in `SIMULATED_KINDS` (`minecraft_terrain/src/server_mobs.rs:389-420`), so a saved, generated, summoned or egg-spawned shulker waits dormant: saved unchanged, never ticked, drawn or hit (`docs/MINECRAFT-SURVIVAL.md:341`). |
| Dormant rules | Round 7 fixed the deletion bug: shulkers are never removed far away and stay in peaceful (`minecraft_terrain/src/dormant.rs:4-19`, tests `:78-82`, `server.rs:5650-5680`). |
| Live despawn rules | `EntityWorld::check_despawn` takes a `(distance, removeWhenFarAway, allowedInPeaceful)` rule per category (`entities/src/world.rs:4700-4709`). A shulker needs `(128.0, false, true)` although its category is Monster. |
| Category, fire | `MobCategory::of_type` says Monster (`world/src/natural_spawner/spawning.rs:101`). `fire_immune` lists shulker (`entities/src/world/hazards.rs:27-40`). |
| End city sentries | "Sentry" markers add a default-tag shulker at the block's bottom centre, yaw 0 (`generator/src/structure/kinds/end_city.rs:300-306`, `feature/entities.rs:202-207`). No AttachFace, Peek or Color is written, so the vanilla defaults (down, 0, 16) apply. |
| Shulker shell geometry | The shulker box block entity draws the same lid and base cuboids (`minecraft_terrain/src/special_models.rs:834-842`), with all 17 `entity/shulker/shulker*` textures already in the atlas through `special_models::textures()` (`special_models.rs:241-252`, used at `mesh.rs:483-485`). Its lid motion is the block entity's, not the mob's. |
| Mob kind wiring (template: ghast, blaze) | State `entities/src/ghast.rs`, world module `entities/src/world/ghasts.rs` (`spawn_ghast` `:135`, `tick_ghast` `:166`), `EntityKey` (`world.rs:1877`), `MobHit` (`world.rs:1929-1958`), storage (`world.rs:2278`), `entity_type` (`:3932`), deaths and XP (`:3910-3913`), voices (`:4153`), despawn (`:4747`), tick dispatch (`:5380`). Goals: `ghast_goals` (`monster_ai.rs:1204`), `blaze_goals` (`:1320`). Server: `spawn_saved` arm (`server_mobs.rs:784`), save (`:1114`). Client: `server_mobs()` (`client_mobs.rs:1024`, ghast push `:1100`), `shadow_casters` (`:1121`, `:1140`). Hit boxes, models and bullet keys moved to `minecraft_survival/src/entities.rs`: `boxes()` `:1084-1152`, `shot_boxes()` `:1154-1168`, `meshes()` `:1179-1300`, `encode`/`decode` `:1350-1398`, keys 0-20 are taken (`FIREBALL_KEY` 17, `CRYSTAL_KEY` 19, `DRAGON_KEY` 20 at `:1398-1401`). |
| Projectile template | Fireballs: `entities/src/fireball.rs` (`margin` `:92`, `tick` with entity and block hits `:106-150`) and `entities/src/world/fireballs.rs` (spawn, targets from mob bodies and `PlayerCandidate`s `:88-123`, `PlayerHit` on a player `:211-223`, deflect and shoot-down helpers `:44-91`). Saved and loaded as entities (`server_mobs.rs:815-823`, `:1291`). |
| Hits on the player | `PlayerHit { player_id, damage, kind, source }` and `PlayerHitKind` (`entities/src/world.rs:558-584`). `PlayerHitKind::Arrow` carries `effect: Option<(EffectKind, u32)>` (`:578`), but `Entities::tick` drops it when it turns hits into `(damage, from, toward, cause)` tuples, filtered to `PLAYER` (`minecraft_survival/src/entities.rs:36`, `:993-1018`). `render_anim/src/minecraft_world.rs:1803-1823` applies the tuples with the shield. So tipped-arrow effects never reach the player today either. |
| Death causes | `MobAttack` has Melee, Arrow, Explosion, Fireball, SmallFireball (`minecraft_survival/src/death.rs:62-68`); damage types at `:144-150`. There is no `mob_projectile`. |
| Player effects | `EffectKind` has 10 kinds, no levitation or slow falling (`third_party/minecraftoss/player/src/survival.rs:36-49`), stored in `[Option<EffectInstance>; 10]` (`:168`, `:187`) and ticked from a list (`:357-370`). Lists of all kinds are repeated in `minecraft_survival/src/commands.rs:150-162`, `:470-479`, `minecraft_survival/src/survival.rs:893-905` and `player_nbt.rs:19-28`. |
| Potions | `slow_falling` and `long_slow_falling` potions exist with their colours (`minecraft_terrain/src/potions.rs:62-63`, `:95`, `:98`), but `player_effect` maps neither (`:230-242`) and `apply_to_player` skips what it can't map (`:407-416`). **Drinking a slow falling potion does nothing today.** `mob_effect` (`:214`) maps only the 10 `MobEffect`s (`entities/src/effects.rs:17-60`). |
| Player movement | MW2 pmove. Creative flight and elytra gliding return early (`movement_iw4/src/single.rs:209-228`); then water, lava, climbing, air or walk (`:300-324`). `air_move` uses MW2 gravity `ps.gravity` (800, `sim/src/world.rs:4085`) through `step_slide_move` (`movement_iw4/src/air.rs:14-64`). Gliding uses vanilla's 0.08 gravity (`glide.rs:10`, `:126`). |
| Per-player movement state | Precedent: `mc_rocket_time` in `PlayerState` (`playerstate_iw4/src/playerstate.rs:122`, `:349`), its netfield (`net/src/transport/netfields.rs:367`), the sim setters `set_may_glide`/`launch_rocket` (`sim/src/frame.rs:427-442`, `sim/src/carrier.rs:294-300`), fed each frame by `render_anim/src/minecraft_elytra.rs:11-30`, called at `minecraft_world.rs:705`. |
| Fall damage | `steps.air_peak` in `render_anim/src/minecraft_world.rs:878-906`: landing hurts from the peak; creative flight resets the peak each frame (`:900`). |
| Teleport particles | `minecraft_terrain::dragon_egg::teleport_particles(from, to, random)` (`dragon_egg.rs:47`), already emitted through the portal particles (`minecraft_survival/src/entities.rs:928-932`). Vanilla uses the same effect for shulkers (level event 2016). |
| Effect HUD | None: no effect icons anywhere. Out of scope. |

## 3. Vanilla 26.3 behaviour (javap)

### 3.1 Levitation and slow falling (`LivingEntity`, `Player`)

- `LivingEntity.travel`: fluids (`travelInFluid`) first, then `isFallFlying` (`travelFallFlying`), else `travelInAir`.
- `LivingEntity.travelInAir`, after `handleRelativeFrictionAndCalculateMovement` (the move uses the current velocity):
  - with `LEVITATION` at amplifier `a`: `vy += (0.05 * (a + 1) - vy) * 0.2`;
  - otherwise `vy -= getEffectiveGravity()`;
  - then `vy *= 0.98` (`computeModifiedFriction(0.98F, air_drag_modifier = 1)`), and horizontal by `friction * 0.91`.
  - Per tick: `v' = 0.98 * (0.8 * v + 0.01 * L)` with `L = a + 1`. Steady rise `0.0098 L / 0.216 = 0.04537 L` blocks per tick (0.907 L blocks per second, 32.67 L MW2 units per second). From rest, 200 ticks rise `0.04537 * (200 - 1 / 0.216) = 8.864` blocks at `L = 1`.
- `LivingEntity.getEffectiveGravity`: when `vy <= 0` and `SLOW_FALLING`, `min(gravity, 0.01)`; otherwise the gravity (0.08 for players). Rising is unaffected. Slow fall: `v' = 0.98 * (v - 0.01)`, terminal −0.49 blocks per tick (9.8 blocks per second, 352.8 MW2 units per second); from rest, 20 ticks fall 1.656 blocks.
- Levitation wins: the `if` checks levitation first, so slow falling does nothing while levitating.
- `LivingEntity.aiStep`: with `SLOW_FALLING` or `LEVITATION`, `resetFallDistance()` every tick. A fall starts counting only when both are gone.
- `LivingEntity.canGlide`: false with `LEVITATION`, so levitation stops elytra flight. `travelFallFlying` uses `getEffectiveGravity`, so slow falling lowers the glide's gravity to 0.01 while sinking; on a climbable it calls `travelInAir` and stops gliding.
- `Player.travel` while `abilities.flying`: keeps the old `vy`, runs the travel, then sets `vy = old * 0.6`. Levitation does nothing in creative flight.
- In water and lava (`travelInFluid`) levitation does nothing. On a climbable, `travelInAir` still runs (after `handleOnClimbable` clamps the fall to 0.15), so levitation lifts.
- `ShulkerBullet` gives `new MobEffectInstance(LEVITATION, 200)`: amplifier 0, 10 s.

### 3.2 Shulker (`monster.Shulker` and its inner classes, `EntityTypes`)

- **Type:** Monster category, `fireImmune`, `canSpawnFarFromPlayer`, 1x1, eye height 0.5, tracking range 10. Extends `AbstractGolem` (`removeWhenFarAway` false). 30 max health (`createAttributes`), `xpReward` 5. Movement emission none, sound source hostile.
- **Saved keys:** `AttachFace` (byte, `Direction.LEGACY_ID_CODEC`: 0 down, 1 up, 2 north, 3 south, 4 west, 5 east; default down), `Peek` (byte, default 0), `Color` (byte, 16 = none, 0-15 dye ids). The covered armour lives in the saved `attributes` as an `armor` modifier `minecraft:covered`, 20, `add_value`.
- **No motion:** `getDeltaMovement` is always zero, `setDeltaMovement` and `push` do nothing, no body rotation (`ShulkerBodyRotationControl.clientTick` empty), `setPos` snaps to the block's bottom centre and resets `Peek` to 0 when the block changes. `move(SHULKER_BOX, ..)` teleports instead.
- **Peek:** raw peek 0-100. `updatePeekAmount` each tick: `current` moves 0.05 toward `raw * 0.01`. `getPhysicalPeek(p) = 0.5 - sin((0.5 + p) * π) * 0.5` (0 at 0, 0.206 at 0.3, 1.0 at 1). When `current` grows, `onPeekAmountChange` moves every non-shulker, non-noPhysics, non-spectator entity in the newly covered slab by the growth along the opening direction (`MoverType.SHULKER`).
- **Bounding box:** `getProgressAabb(scale, attachFace.opposite, physicalPeek, pos)`: the unit cube on the block, stretched toward the opening side by the physical peek (up to two blocks long). `getDefaultDimensions` for a floor shulker is height `1 + current`.
- **`setRawPeekAmount(n)`** (server): removes the `covered` modifier; at `n == 0` adds it back (+20 armour) and plays `entity.shulker.close`, else plays `entity.shulker.open` (both volume 1, pitch 1). Quirk kept: a fresh shulker has no covered modifier until it first closes.
- **Staying put** (`tick`, server): if `!canStayAt(blockPos, attachFace)`, `findNewAttachment`: the first of DOWN, UP, NORTH, SOUTH, WEST, EAST that passes `canStayAt`, else `teleportSomewhere`.
- **`canStayAt(pos, face)`:** the shulker's own cell is air (or its own moving piston), the neighbour on `face` has a sturdy face toward it (`loadedAndEntityCanStandOnFace`), and the fully open box (`getProgressAabb(scale, face.opposite, 1.0, bottomCentre(pos))` deflated by 1e-6) has no block or entity collision. Water in its cell or a block in front of its lid makes it leave.
- **`teleportSomewhere`** (not with NoAI, only alive): 5 tries at `pos + (randomBetweenInclusive(-8, 8) for x, y, z)`; needs `y > minY`, an empty block, inside the world border, no collision in the unit cube, an attachable face, and the attach block not in `#shulker_does_not_teleport_to` (`bedrock`). On success: dismount, set the face, `entity.shulker.teleport` (1, 1), move, game event, level event 2016 (`singleBlockTeleportationParticles` from old to new), raw peek 0, target cleared. The client lerps the drawing from the old block over 6 ticks: offset `-(new - old) * ((t - partial) / 6)^2`.
- **`hurtServer`:** while closed (raw peek 0), a hit whose direct entity is an `AbstractArrow` is refused (the arrow bounces). After a hit lands: if `health < 15` and `nextInt(4) == 0`, `teleportSomewhere`; else if the source is a projectile from a `ShulkerBullet`, `hitByShulkerBullet`.
- **`hitByShulkerBullet`:** only when open and `teleportSomewhere` succeeds: count live shulkers in the old box inflated by 8 (`n`, itself included); if `nextFloat() < (n - 1) / 5` nothing more, else a new shulker with the same colour appears at the old position.
- **Sounds:** ambient `entity.shulker.ambient` only when open; hurt `entity.shulker.hurt_closed` when closed, else `entity.shulker.hurt`; death `entity.shulker.death`; shoot `entity.shulker.shoot` at volume 2, pitch `(r1 - r2) * 0.2 + 1`.
- **Goals:** 1 `LookAtPlayerGoal(Player, 8, 0.02, onlyHorizontal = true)`, 4 `ShulkerAttackGoal`, 7 `ShulkerPeekGoal`, 8 `RandomLookAroundGoal`. Targets: 1 `HurtByTargetGoal` (no alerting), 2 `ShulkerNearestAttackGoal` (players, must see; not in peaceful), 3 `ShulkerDefenseAttackGoal` (`Enemy` mobs, only when the shulker is on a team: never in this fork).
- **Target search area:** the box inflated by 4 along the attach face's axis and by the follow range (16) on the other two. A floor shulker only *acquires* players within 4 blocks above or below it. `getTargetSearchArea` is used only by `NearestAttackableTargetGoal.canUse`; a target it already has is kept by `TargetGoal.canContinueToUse` (alive, within the follow range 16, not unseen for longer than the unseen memory) and by the attack goal's 20-block rule, so climbing 5 blocks above a shulker that is already firing does not stop it.
- **`ShulkerAttackGoal`** (every tick, not in peaceful): `start` sets `attackTime = 20` and raw peek 100; `stop` raw peek 0. Each tick: `attackTime--`, look at the target (180, 180); if the target is within 20 blocks (`distSqr < 400`) and `attackTime <= 0`: `attackTime = 20 + nextInt(10) * 20 / 2` (20-110 ticks), spawn a bullet toward it along the attach axis rule, play the shoot sound. Beyond 20 blocks it drops the target.
- **`ShulkerPeekGoal`** (no target, every other tick): `canUse` when `nextInt(reducedTickDelay(40)) == 0` (1 in 20) and `canStayAt`; `start`: `peekTime = adjustedTickDelay(20 * (1 + nextInt(3)))`, raw peek 30; `stop` (no target): raw peek 0.
- **Look:** `ShulkerLookControl` never clamps the head to the body, pitch is always 0, and the head yaw is measured in the plane of the attach face: `atan2(-d·right, d·forward)` with `forward` = south rotated by `attachFace.opposite.getRotation()` and `right = forward × up`. Max head yaw and pitch 180.
- **Loot** (`loot_table/entities/shulker.json`): one `shulker_shell` under `random_chance_with_enchanted_bonus`: 0.5 unenchanted, `0.5625 + 0.0625 * (level - 1)` with Looting.
- **Colour:** `Color` 0-15 picks `entity/shulker/shulker_<dye>`, anything else `entity/shulker/shulker`.

### 3.3 Shulker bullet (`projectile.ShulkerBullet`, `Projectile`)

- **Type:** Misc, 0.3125 x 0.3125, no loot, tracking range 8, `noPhysics = true` (it doesn't collide by moving; block hits come from its ray), gravity 0.04 (`getDefaultGravity`), always pickable, never on fire, full bright.
- **Spawn:** at the shulker's bounding box centre, owner = shulker, target = its target, `currentMoveDirection = UP`, then `selectNextMoveDirection(attachFace axis, target)`.
- **`selectNextMoveDirection(axis, target)`:**
  - The aim point is the block containing the target's mid-height (`target.y + 0.5 * height`), at `(x + 0.5, y + 0.5 * height, z + 0.5)`; with no target, the block below the bullet with a 0.5 offset.
  - If the bullet is within 2 blocks of that block's centre, aim at the point itself (direction kept null).
  - Otherwise, for each axis other than `axis`, add the direction that brings the bullet's block toward the aim block if the next block that way is empty (east before west, up before down, south before north). With none, try up to 6 random directions until one leads into an empty block; else pick one of the candidates at random. The aim becomes `position + direction`.
  - `targetDelta = (aim - position).normalize() * 0.15` (zero if at the aim); `flightSteps = 10 + nextInt(5) * 10`.
- **`tick` (server):**
  - The target is resolved by reference; a dead, missing or spectator target, or none, means it falls with gravity 0.04 instead of homing.
  - Homing: `targetDelta = clamp(targetDelta * 1.025, -1, 1)` per axis, `velocity += (targetDelta - velocity) * 0.2`.
  - Ray `getHitResultOnMoveVector` (blocks with the collider shape, entities that `canHitEntity`), then `position += velocity`, then a hit is handled.
  - Steering: `flightSteps--` (when > 0); at 0, `selectNextMoveDirection(current axis, target)`. If the block ahead in the current direction can be stood on, re-select on that axis; else, once the bullet's block coordinate on the moving axis equals the target's, re-select on that axis.
  - `checkDespawn`: removed in peaceful.
- **`canHitEntity`:** pickable living things (not `noPhysics`); the owner is not excluded, but a ray starting inside a box (`AABB.clip`) misses it, so a fresh bullet never hits its own shulker.
- **Hit an entity:** `mobProjectile(bullet, owner)` damage 4 (`hurtOrSimulate`). If it lands and the target is living, `LEVITATION` 200 ticks, amplifier 0. Then the bullet is destroyed. Damage type `mob_projectile`: message `mob` ("%1$s was slain by %2$s"), scales with difficulty, in `#is_projectile`.
- **Hit a block:** 2 `explosion` particles (spread 0.2), `entity.shulker_bullet.hit` (1, 1), destroyed.
- **Hurt:** any damage: `entity.shulker_bullet.hurt` (1, 1), 15 `crit` particles (spread 0.2), destroyed.
- **Shield:** `LivingEntity.hurtServer` returns `!blocked || amount > 0`, so a fully blocked bullet gives no levitation.
- **Saved keys:** `Target` (UUID), `Dir` (byte, may be absent), `Steps` (int), `TXD`, `TYD`, `TZD` (doubles), plus `Projectile`'s `Owner` and `LeftOwner`.
- **Client trail:** each tick one `end_rod` particle at `position - velocity + (0, 0.15, 0)` with no motion. `EndRodParticle`: lifetime `60 + nextInt(12)`, gravity 0.0125, friction 0.91, size × 0.75, fades toward `#F2DEC9` and out over its second half, sprites `particle/glitter_7` to `glitter_0` by age, full bright.

### 3.4 Renderers

- **`ShulkerModel`** (64x64): `lid` box (-8, -16, -8) size 16x12x16 at UV (0, 0) and `base` box (-8, -8, -8) size 16x8x16 at UV (0, 28), both at offset (0, 24, 0); `head` box (-3, 0, -3) 6x6x6 at UV (0, 52), offset (0, 12, 0). `setupAnim`: `g = (0.5 + peek) * π`, `h = -1 + sin(g)`; wobble `sin(age * 0.1) * 0.7` when `g > π`; `lid.y = 16 + sin(g) * 8 + wobble`; `lid.yRot = h^4 * π * 0.125` when `peek > 0.3`, else 0; `head.xRot = xRot` (always 0); `head.yRot = (yHeadRot - 180 - yBodyRot)` in radians.
- **`ShulkerRenderer`:** `MobRenderer` with no shadow (radius 0); rotations: the normal `yBodyRot + 180`, then a rotation of `attachFace.opposite.getRotation()` around (0, 0.5, 0); `peek = lerp(partial, currentO, current)`; the render offset from the teleport lerp.
- **`ShulkerBulletRenderer`:** translate (0, 0.15, 0); rotate Y `sin(age * 0.1) * 180°`, X `cos(age * 0.1) * 180°`, Z `sin(age * 0.15) * 360°`; scale (-0.5, -0.5, 0.5); draw `ShulkerBulletModel` (64x32: (-4, -4, -1) 8x8x2 at UV (0, 0); (-1, -4, -4) 2x8x8 at (0, 10); (-4, -1, -4) 8x2x8 at (20, 0)) translucent with `entity/shulker/spark`; then scale 1.5 and draw again with colour `0x26FFFFFF` (alpha 0.15). Full bright.

## 4. Plan (one commit per group, in order)

`minecraft_terrain` and the vendored crates are edition 2021 (no let-chains). New files pass `rustfmt --check`; edits keep the surrounding style; no comments anywhere. Every number below comes from section 3.

### Group 1. "Minecraft players and mobs can have levitation and slow falling"

- `third_party/minecraftoss/player/src/survival.rs`: `EffectKind::Levitation` and `EffectKind::SlowFalling` (append after `Wither` so saved indices don't shift), the effect array grows to 12 (`:168`, `:187`), both in the tick list (`:357-370`) with no per-tick work.
- Every list of all kinds: `minecraft_survival/src/commands.rs:150-162` (`effect_kind`), `:470-479`, `minecraft_survival/src/survival.rs:893-905` (`clear_effects`, so milk and the totem clear them), `player_nbt.rs:19-28` (`minecraft:levitation`, `minecraft:slow_falling`).
- `minecraft_terrain/src/potions.rs:230-242`: `player_effect` maps both, so slow falling potions work.
- `third_party/minecraftoss/entities/src/effects.rs`: `MobEffect::Levitation` and `MobEffect::SlowFalling` with their ids, in `ALL`; no per-tick work. `potions.rs:214` `mob_effect` maps them, and mob `active_effects` round-trip through the existing `effect_of`/`effect_tag` (`server_mobs.rs:986-1021`).
- Tests: `/effect give @s minecraft:levitation 10 1` gives amplifier 1 for 200 ticks and `/effect clear` removes it; drinking `long_slow_falling` gives 4800 ticks; `active_effects` round-trip both ids on a player and on a mob; a totem clears levitation.

### Group 2. "Minecraft levitation lifts players and slow falling slows their fall"

- `playerstate_iw4/src/playerstate.rs`: a new `mc_float: i32` (default 0): bits 0-15 the levitation level `amplifier + 1` (0 = none), bit 16 slow falling. Its netfield after `mc_rocket_time` (`net/src/transport/netfields.rs:367`, replicated, exact).
- `sim/src/frame.rs` and `sim/src/carrier.rs`: `set_float(id, levitation: Option<u8>, slow_falling: bool)`, like `set_may_glide`.
- `movement_iw4/src/float.rs` (new, pure, exported from `lib.rs`):
  - `rise(v, level, ticks)`: the closed form of `v' = a v + b` over fractional ticks, `a = 0.8 * 0.98F`, `b = 0.98F * 0.01 * level`: `v_inf + (v - v_inf) * a^ticks`, `v_inf = b / (1 - a)`, in blocks per tick (convert with `WATER_BLOCK / 0.05` like `glide.rs:9`).
  - `slow_fall(v, ticks)`: the closed form of `v' = 0.98F (v - 0.01)` while `v <= 0`.
- `movement_iw4/src/single.rs:300-324` and `air.rs`:
  - With a levitation level, the walk branch is replaced by the air branch and `ground_entity_num` is cleared before the move. Without this the player is re-grounded at the start: the first ticks rise slower (7 units per second after one tick) than Q3's ground-trace kick-off of 10.
  - The air move then sets the vertical velocity from `rise` and slides with no gravity. Horizontal stays MW2 air control.
  - The climbable branch (`:318`) also uses `rise` for the vertical velocity.
  - With slow falling and no levitation, while `velocity[2] <= 0`, the air move uses `slow_fall` instead of MW2 gravity. Rising keeps MW2 gravity.
  - Water, lava, creative flight and noclip ignore both.
- `movement_iw4/src/glide.rs:10`, `:126`: the glide gravity is 0.01 while slow falling and `motion[2] <= 0`.
- `render_anim/src/minecraft_elytra.rs:11-30` (per player, from `player.survival.status`): call `set_float`; `may` glide is false while levitating (`canGlide`).
- `render_anim/src/minecraft_world.rs:899-901`: while either effect is on, the fall peak restarts at the feet each frame (`resetFallDistance`), like creative flight; `minecraft_survival/src/elytra.rs:58` `fall_peak` likewise.
- Tests (`movement_iw4`):
  - from rest, 200 ticks of 50 ms frames rise 8.864 ± 0.01 blocks at level 1 and 44.32 ± 0.05 at level 5; 16 ms frames land within 0.1 block of that;
  - steady rise 32.67 units per second per level;
  - a player standing on the ground leaves it on the first frame;
  - a ceiling stops the rise;
  - slow falling reaches 352.8 units per second (±1) and falls 1.656 ± 0.01 blocks in the first 20 ticks;
  - slow falling leaves a jump's rise untouched;
  - levitation wins over slow falling;
  - flight, water and gliding ignore levitation;
  - a glide sinks with gravity 0.01;
  - `set_float` round-trips through the PS netfield.
- Tests (`minecraft_survival`): the fall peak resets while levitating, and a fall from the levitation's end hurts from there.

### Group 3. "Minecraft mobs float with levitation and fall slowly with slow falling"

- `entities/src/movement.rs`: `Body.levitation: Option<u8>` and `Body.slow_falling: bool`. In `travel_air_jumping` (`:479`), the vertical update becomes the 3.1 formula, and `fall_distance` resets while either is set.
- `entities/src/world.rs`: set both from each mob's effects right after its effects tick, where every kind already handles `EffectWork` (`:4836` and the matching arms).
- Tests: a zombie with levitation rises 8.864 blocks in 200 ticks; slow falling caps its fall at 0.49 per tick; neither takes fall damage while on.
- If this outgrows one commit, it can move after group 7: bullets that hit mobs then only damage them, which is noted in the track log.

### Group 4. "Minecraft shulkers are live mobs that cling to blocks, peek and teleport"

This group is two commits, in this order, because the wiring alone touches about 25 places in `world.rs`:
- **4a "Minecraft shulkers are live mobs that cling to blocks, peek and teleport":** the state, the pure functions, `world/shulkers.rs`, the goals and look, the `world.rs` wiring without the damage rules (shulkers take plain damage), despawn, server load and save, the client push and the hit boxes, with their tests.
- **4b "Minecraft shulkers hide in their shell: armour, arrow bounces, hurt teleports, loot and sounds":** the covered armour, the arrow refusal, the half-health teleport, the death table and XP, the voices, with their tests.

- **`entities/src/shulker.rs` (new):**
  - state: `body`, `health` (30), `attach_face`, `raw_peek`, `peek`/`peek_o`, `color: Option<u8>`, `covered: bool`, `teleport_from: Option<Pos>` with a 6-tick counter, `persistence_required`, `damage`, `voices`, `random`;
  - pure functions: `physical_peek`, `progress_box(face, peek, pos)` (the vanilla delta box), `can_stay_at`, `find_attachable_surface`, `teleport_somewhere` over the `World` trait, using the `shulker_does_not_teleport_to` tag;
  - constants: `EXPERIENCE` 5, `COVERED_ARMOR` 20.
- **`entities/src/world/shulkers.rs` (new):** `spawn_shulker`, `shulkers()` and `tick_shulker`:
  - the attachment check, then `findNewAttachment` or a teleport;
  - the peek step, with the push of other mobs' bodies (players are not pushed, see section 6);
  - the goals;
  - level event 2016 as a teleport report `(from, to)`, which feeds `dragon_egg::teleport_particles` like `egg_teleports` (`minecraft_survival/src/entities.rs:928-932`).
- **Goals (`monster_ai.rs`, next to `blaze_goals`):** look at player (8, 0.02, horizontal only), peek (1 in 20 on its every-other-tick cadence, 20/40/60 ticks halved by `adjustedTickDelay`, raw 30), random look, hurt-by target, nearest player with the axis-shaped search box, must see. The attack goal comes in group 6.
- **`entities/src/look.rs` or the shulker module:** the shulker look rule (3.2 "Look"); body yaw fixed at 0.
- **`entities/src/world.rs` wiring like the ghast** (`EntityKey`, `MobHit::Shulker`, storage, `entity_type`, `mob_body`, `body_mut`, damage):
  - damage through `damage_after_armor(d, if covered { 20.0 } else { 0.0 }, 0.0)`;
  - closed shulkers refuse arrows, and the arrow takes its existing "not accepted" path;
  - after a hit, the half-health teleport (`health < 15`, 1 in 4);
  - deaths with table `minecraft:entities/shulker` and XP 5; voices with the open-only ambient and the closed hurt sound;
  - despawn rule `(128.0, false, true)`;
  - no knockback and no velocity, ever;
  - wherever a box is built from a body (fireball targets `world/fireballs.rs:88-123`, `mob_body` in `world/combat.rs:133`), shulkers give `progress_box`.
- **Server** (`minecraft_terrain/src/server_mobs.rs`): `"minecraft:shulker"` in `SIMULATED_KINDS`; a `spawn_saved` arm reading `AttachFace`, `Peek`, `Color` and the `minecraft:covered` armour modifier from `attributes`; a `mob_tags` block writing them back (byte keys), with the modifier present only while `covered`.
- **Client:** a `server_mobs()` push (`client_mobs.rs:1024`) with the head yaw, and no shadow (`MobRenderer` radius 0, so not in `shadow_casters`).
- **Hit boxes** (`minecraft_survival/src/entities.rs:1084`): shulkers use `progress_box`, not the body's width and height, so wall and ceiling shulkers are hit where they are drawn. Key 26 for `MobHit::Shulker` in `encode`/`decode` (see section 6).
- **Tests (`entities`):**
  - `physical_peek` at 0, 0.3 and 1;
  - the box for each face at peek 0 and 1;
  - a floor shulker whose support is removed clings to a wall, and teleports with none;
  - water in its cell or a block in front of its lid makes it leave;
  - teleports stay within ±8, land on air with a sturdy face, never next to bedrock, reset the peek and the target, and report from and to;
  - raw peek 30 is reached in 6 ticks (0.05 per tick);
  - armour 20 only after the first close; a closed shulker refuses an arrow and an open one takes it;
  - below half health one hit in four teleports it (seeded);
  - it never moves when hit or blasted;
  - it never despawns far away and stays in peaceful;
  - the death drops one shell about half the time and more often with Looting III (seeded, through the real loot table, gated on `MINECRAFTOSS_ROOT`);
  - XP 5;
  - a floor shulker only acquires a player within 4 blocks vertically, and keeps a player it already targets who then climbs higher (within 16 blocks, in sight).
- **Tests (`minecraft_terrain`):** a save round-trip of `AttachFace`, `Peek`, `Color` and the covered modifier; an end city sentry tag (default tag) becomes a live shulker facing down.

### Group 5. "Minecraft shulkers are drawn with their shell, lid and head"

- `minecraft_terrain/src/shulker_render.rs` (new): the `ShulkerModel` cuboids and `setupAnim` from 3.4.
  - The pose: block bottom centre, plus the teleport offset, `yBodyRot + 180`, then the attach rotation around (0, 0.5, 0).
  - Peek interpolated with `partial`, head yaw from the client mob pose.
  - The texture per `Color`; the cuboid layout can share `special_models.rs:838-839`'s numbers.
- `minecraft_survival/src/entities.rs:1179`: call it in `meshes()`.
- Tests: the vertex count (3 cuboids); `lid.y` is 24, 20.70 and 8 at peek 0, 0.3 and 1; the lid turn is 0 at 0.3 and `h^4 π / 8` above; the head yaw follows `yHeadRot`; a ceiling shulker's mesh is the floor mesh turned upside down about the block centre; colour 14 picks `shulker_red`, 16 and out-of-range values the plain sheet; the 6-tick teleport offset at t = 6, 3 and 0.

### Group 6a. "Minecraft tipped arrows give players their effect"

Split out of group 6 so the bullet commit stays reviewable, and because it fixes a bug on its own.
- **`minecraft_survival/src/entities.rs:993-1018`:** hits become `(damage, from, toward, cause, effect: Option<(EffectKind, u32)>)`, filled from `PlayerHitKind::Arrow`'s `effect`.
- **`render_anim/src/minecraft_world.rs:1803-1823`:** after `survival.hurt` returns true (not fully blocked by the shield, not invulnerable), add the effect.
- **Tests (`minecraft_survival`):** a tipped arrow of poison gives the player poison; a fully shield-blocked one gives none; creative gives none.

### Group 6. "Minecraft shulkers shoot homing bullets that make players levitate"

- **`entities/src/shulker_bullet.rs` (new):** state (`position`, `velocity`, `direction: Option<Dir>`, `steps`, `target_delta`, `owner`, `target: Option<u64>`, `alive`, `tick_count`), `select_next_move_direction`, `tick` with the ray (reuse `fireball.rs`'s margin and hit search), gravity 0.04 without a target.
- **`entities/src/world/shulker_bullets.rs` (new), like `world/fireballs.rs`:**
  - spawn, tick, and targets: living mobs from bodies and shulker boxes, plus players.
  - A player hit pushes `PlayerHit { damage: 4.0, kind: PlayerHitKind::ShulkerBullet { velocity, effect: (EffectKind::Levitation, 200) }, source: Some(shulker) }`.
  - A mob hit applies 4 damage, credited to the shulker, and `MobEffect::Levitation` for 200 ticks when it lands. If the victim is a shulker, `hitByShulkerBullet` (duplication with `(n - 1) / 5`).
  - Block hits report 2 explosion puffs and the hit sound. Hurt by anything: the hurt sound, 15 crit puffs, gone.
  - Peaceful removes them.
- **Attack goal:** 20 then `20 + nextInt(10) * 10` ticks, raw peek 100, drops targets beyond 20 blocks, shoot sound at volume 2, the bullet from the box centre, not in peaceful.
- **`entities/src/world.rs:569-584`:** the new `PlayerHitKind::ShulkerBullet` variant, and `bin/check_zombie_player_hits.rs:158`.
- **`minecraft_survival/src/death.rs`:** `MobAttack::MobProjectile` (damage type `minecraft:mob_projectile`, message `mob`).
- **`minecraft_survival/src/entities.rs:993-1018`:** the bullet's arm in the hit conversion (toward from its velocity, `MobAttack::MobProjectile`, its effect into the tuple from group 6a).
- **Shooting bullets down:** `shot_boxes()` adds bullets under key 27 (the 0.3125 box); a hit or melee hurts them (the fireball shoot-down path at `entities.rs:1428` is the model).
- **Saving** (`server_mobs.rs`, next to the fireball arms `:815`, `:1291`): `minecraft:shulker_bullet` with `Steps`, `Dir`, `TXD`/`TYD`/`TZD`, `Owner`, and `Target` kept as read. A target that doesn't resolve (players have no UUID in the entity world yet) leaves the bullet falling.
- **Tests (`entities`):**
  - the direction choice: the axis excluded, the order, the 2-block aim, the random fallback with an empty list;
  - `flightSteps` 10-50;
  - homing converges on a still target and hits it (seeded);
  - re-selection when the block ahead is solid and when the axis coordinate matches;
  - with no target it falls with 0.04 and pops on the ground;
  - a fresh bullet doesn't hit its own shulker;
  - the attack timing 20 then 20-110 and the 20-block drop;
  - none in peaceful;
  - a mob hit gives levitation 200; a bullet hitting an open shulker teleports it and, with `n = 1`, always duplicates it with its colour (seeded);
  - any damage destroys a bullet;
  - the save round-trip.
- **Tests (`minecraft_survival`):** a bullet hit on the player gives 4 (normal), 3 (easy), 6 (hard) and levitation 200 amplifier 0; a fully shield-blocked one gives no levitation; creative gives none; the death line "was slain by Shulker".

### Group 7. "Minecraft shulker bullets glow and leave an end rod trail"

- `minecraft_terrain/src/shulker_render.rs`: the bullet model and its two passes from 3.4; the outer pass goes to `out.translucent` with alpha 0.15, both full bright. `mesh.rs:480-503`: add `minecraft:entity/shulker/spark` to the atlas.
- End rod particles in a small module beside `portal_particles.rs`: one per bullet per client tick, with the `EndRodParticle` numbers and the `glitter_*` sprites. Explosion puffs on block hits and crit puffs on hurt through the existing particle paths.
- `minecraft_survival/src/entities.rs:1179-1270`: draw bullets interpolated by `partial`.
- Tests: the bullet vertex count (2 × 3 cuboids); the spin angles at age 0 and 10; the outer pass alpha 0.15; end rod lifetime 60-71, gravity and fade; one trail particle per tick per bullet.

### Group 8. "Docs: Minecraft shulkers and levitation"

Line numbers are as of e2313625; earlier docs commits in this track move them, so find each line by its text. `docs/MINECRAFT-SURVIVAL.md`: the X6 line (`:282`) loses shulkers and levitation; the simulated mobs line (`:341`) names shulkers; a short section on levitation, slow falling and shulkers; the limitations from section 6. Also the shulker row in `docs/NETHER-END.md:251`.

## 5. Owner test

Any world in survival, `/difficulty normal`, daytime, on an open end stone floor about 15 x 15. `/tp ~ ~ ~` prints the coordinates. One screenshot per bold number. Take the armour off for steps 6 and 7 so the fall damage is clean.

```
IW4L_MINECRAFT_KIT=minecraft:shulker_spawn_egg*16,minecraft:diamond_sword+looting=3,minecraft:iron_sword,minecraft:bow+power=5,minecraft:arrow*64,minecraft:shield,minecraft:end_stone*64,minecraft:purpur_block*64,minecraft:diamond_pickaxe+efficiency=5,minecraft:water_bucket,minecraft:milk_bucket,minecraft:potion@slow_falling,minecraft:potion@long_slow_falling,minecraft:splash_potion@slow_falling,minecraft:elytra,minecraft:firework_rocket*64,minecraft:ladder*16,minecraft:zombie_spawn_egg*4,minecraft:cooked_beef*32,minecraft:totem_of_undying,minecraft:torch*64
```

(21 stacks.)

1. Use a shulker egg on the floor 10 blocks away. A purple shulker sits closed on the block. Every few seconds it lifts its lid a little and closes it again, with the open and close sounds (**1**).
2. `/summon minecraft:shulker ~3 ~ ~ {Color:14b}` and `/summon minecraft:shulker ~5 ~ ~ {Color:0b}`: a red and a white shulker (**2**). Use eggs on a wall face and under a ceiling you build: they cling to the wall and hang from the ceiling, lid away from the block (**3**).
3. Stand within 16 blocks of a floor shulker, at its height, in sight. It opens fully, turns its head to you and fires every 1 to 5.5 seconds. Bullets move in straight steps that turn at right angles, glow, spin and leave a white sparkle trail (**4**). Walk more than 16 blocks away until it closes, then come back on a pillar 5 or more blocks above it: it stays closed and ignores you (it only picks targets within 4 blocks of its height; one it already fires at keeps being shot until it is more than 16 blocks away or out of sight for a few seconds).
4. Let a bullet hit you: 2 hearts, and you float up for 10 seconds. You can strafe while floating, and you end about 9 blocks up (**5**). Then you fall and lose about 3 hearts (**6**).
5. Get hit again, then drink the milk while floating: you fall at once.
6. Build a 20-block purpur tower and drink the slow falling potion. Jump off: you sink gently (about 10 blocks per second at most) and take no damage (**7**). Get hit by a bullet while slow falling: you still rise (levitation wins).
7. Equip the elytra, jump off the tower and glide. A bullet hit stops the glide and you rise (**8**). With slow falling, a glide sinks more slowly.
8. Raise the shield toward a bullet: it blocks and you don't float (**9**). Shoot a bullet with the MW2 gun or punch it: it pops with sparks.
9. Stand in 2-deep water when hit: you don't rise. Climb the ladder when hit: you float up the ladder.
10. Shoot arrows at a closed shulker: they bounce off (**10**). The MW2 gun does little to it closed and full damage open. Use a shulker that has opened and closed at least once: a fresh one has no shell armour until its first close (vanilla quirk).
11. Hurt one below half health: now and then it teleports away with purple particles and a sound (**11**). Mine the block under a shulker on a 1-block pillar: it moves to another face or teleports. Pour water on one: it teleports.
12. Kill 8 with the iron sword: about half drop a shulker shell, plus XP orbs. Kill 8 with the looting sword: more shells (**12**).
13. Place two shulkers 3 blocks apart and stand so the far one's bullets pass the near one while it is open. A bullet that hits it teleports it, and sometimes a new shulker of its colour appears where it sat (**13**).
14. Place a zombie between you and a shulker: a bullet that hits it lifts it up (**14**). Throw the splash slow falling potion at zombies on a tower edge and push them off: they sink gently.
15. `/effect give @s minecraft:levitation 5 4`: you rise fast, about 4.5 blocks per second (**15**). `/effect clear @s`.
16. `/difficulty peaceful`: shulkers stay, close and stop firing; bullets in flight vanish. `/difficulty normal`.
17. Quit with a bullet in flight and shulkers of three colours on a floor, a wall and a ceiling. Restart: all three keep their colour and face (**16**).
18. Once track C2 is in: fly to an end city. Its shulkers attack (**17**).

## 6. Risks, shared files and decisions

**Risks**
- **The movement hook is the delicate part.** If levitation doesn't clear the ground first, the player stays stuck on the ground (Q3's ground trace kicks off only above 10 units per second). The closed form must stay frame-rate independent; group 2's 16 ms test guards that.
- **Size of group 4.** The template wiring touches about 25 places in `world.rs`, so it is planned as 4a "cling, peek and teleport" and 4b "hurt, armour, loot and sounds", in that order.
- **Box shape.** Shulker boxes aren't body boxes. Any spot that builds a box from `width`/`height` (fireball targets, arrow targets, splash and blast reach) must use `progress_box`, or wall and ceiling shulkers are hit in the wrong place.
- **Arrow refusal.** A closed shulker refusing arrows needs the arrow's "not accepted" path. Check that it bounces and isn't dropped.
- **`PlayerState` grows a field.** Host and joining friends must run the same build; the netfield mask grows by one.

**Shared files with other round 8 tracks**
- **E (multiplayer):**
  - `minecraft_survival/src/entities.rs:36`, `:993-1018` (the `PLAYER = 0` filter that the bullet's player hits and their effect go through);
  - `render_anim/src/minecraft_world.rs:1803-1823` (the hit loop, per local player) and `minecraft_elytra.rs` (the per-player movement adapter): both are on E's list to leave `render_anim`;
  - `playerstate.rs` and `netfields.rs` (the new `mc_float`);
  - `sim/src/frame.rs` and `carrier.rs`;
  - the bullet's `Target` can't name a player until players have a UUID in the entity world.
- **D (skating):** `movement_iw4/src/single.rs`, `air.rs`, `playerstate.rs` and `netfields.rs`. Group 2 adds no `pm_flags` bit (everything is in `mc_float`), so D can take free bits. Merge order matters only for `single.rs:300-324`.
- **B (Nether and Overworld mobs):**
  - `entities/src/world.rs` (`EntityKey`, `MobHit`, storage, despawn and tick dispatch);
  - `monster_ai.rs`, `server_mobs.rs` (`SIMULATED_KINDS`, `spawn_saved`, `mob_tags`), `client_mobs.rs`;
  - `minecraft_survival/src/entities.rs` (`boxes`, `meshes`, `encode`/`decode`), `mesh.rs` (atlas list);
  - `entities/src/effects.rs` and `movement.rs`;
  - piglin crossbow bolts go through `PlayerHitKind::Arrow`, whose effect group 6 starts delivering.
  - **Hit key numbers:** 0-20 are taken (20 is `DRAGON_KEY`); c1 takes 26 (shulker) and 27 (shulker bullet), and c2's item frames take 28, leaving 21-25 for B's piglin, brute, hoglin, zoglin and strider. The b1 brief asks for 20, which is the dragon's key: B must move it to 21 or above. Whoever merges second checks for clashes.
- **C2 (end cities):** `end_city.rs:300-306` stays as it is; c1 owns the shulker entity, so C2 must not add shulker handling of its own. The sentry tag is a default tag, and group 4's `spawn_saved` gives it the vanilla defaults. C2's item frame and elytra work shares `server_mobs.rs`, `client_mobs.rs` and `entities.rs`.
- **C5 (limitations), same track:** groups 1-3 of c5 edit the same movement and survival files as groups 1-2 here (`movement_iw4/src/single.rs`, `stuck.rs`, `climb.rs`, `sim/src/frame.rs`, `carrier.rs`, `playerstate.rs`, `minecraft_survival/src/survival.rs`, `death.rs`, `player_nbt.rs`, the fall-peak block in `minecraft_world.rs`). Run them right after groups 1-3 here, so each builds on the other's committed state. c5 takes a `pm_flags` bit; this item takes the new `mc_float` field, so they don't collide.
- **C3, same track:** `ambient_particles.rs` gains c3's `DragonBreath` and c5's `Snowflake` and `Totem` kinds; the end rod module of group 7 sits beside them.
- **A (fixes):**
  - a4 (the mob inside spawners) and a1 (pearls through gateways) touch `minecraft_survival/src/entities.rs`;
  - a5 (overload and `/tick`) touches the server tick loop that ticks shulkers and bullets: bullets must step with the same tick count as every other entity.

**Decisions (the vanilla-like choice each time)**
- **Levitation movement:** the vertical speed follows vanilla's per-tick formula exactly, including the 0.98 drag. Horizontal stays MW2 air control, as the owner wants MW2 movement. Slow falling uses vanilla's falling formula; jumps and other rising keep MW2 gravity.
- **Where levitation applies:** in the air, on the ground and on climbables, not in fluids, creative flight or noclip. It stops elytra gliding. Slow falling also lowers the glide's gravity, as in vanilla.
- **Fall damage:** while either effect is on, the fall peak restarts every frame, as vanilla resets the fall distance.
- **Covered armour:** follows vanilla's modifier, including the quirk that a fresh shulker has none until it first closes, and saves as the vanilla `minecraft:covered` modifier.
- **MW2 bullets** are not arrows: they hit closed shulkers, cut by armour 20. Minecraft arrows bounce off closed shulkers.
- **Shooting bullets down:** shulker bullets can be shot down by MW2 guns and melee (vanilla: any damage destroys one).
- **Levitation only lands with damage:** it lands only when the bullet's damage lands (not fully shield-blocked, not on an invulnerable player).
- **Solidity:** shulkers are not solid to the MW2 player, and an opening shulker doesn't push players, matching today's mob collision and the shulker box limitation (`MINECRAFT-SURVIVAL.md:295`). Opening shulkers do push mob bodies.
- **The defence goal** (shulkers on a team) is left out, since the fork has no teams.
- **Effect icons:** no HUD effect icons. This is a known limitation, and the steps above test levitation by behaviour.
- **Saved bullet targets:** a saved bullet whose target is a player falls on reload until E gives players a UUID in the entity world.
