# C5: six open limitations (design brief)

Track C, item c5. Paths and lines are as of e2313625. Vanilla facts were checked with javap on the 26.3 `client.jar` (no code copied); data and asset facts come from the same jar. Game state stays per `Player` (UUID) and per `Level`; every place that still runs for the local player only is marked **[E]** for the multiplayer track.

## 1. The owner's request

From the round 8 brief: fix the open limitations named in `docs/MINECRAFT-SURVIVAL.md`, each as its own commit:

- powder snow: sinking, freezing, leather boots walk on it, the frozen overlay and damage, buckets;
- lectern books: place a written or writable book, open the book screen, turn pages, comparator output by page, a redstone pulse on a page turn;
- the trapped chest signal: power = number of viewers up to 15, and comparators;
- the totem animation: the floating totem on screen and the particles when a totem pops;
- the ladder ground jump;
- falling rain and snow streaks, as vanilla's `WeatherEffectRenderer` draws them around the camera.

The state doc's exact wording today:

- `:306` "Shallow-lava details differ from vanilla. Powder snow neither slows nor freezes."
- `:307` "No ground jump at a ladder, no climbing in fluids, and an open trapdoor above a ladder doesn't count as part of it." (Only the ground jump is in scope; the other two stay.)
- `:316` "The totem plays no animation."
- `:326` "Books can't be placed on a lectern."
- `:328` "Trapped chests give no redstone signal."
- `:296` "Not drawn: ... the falling rain and snow streaks (rain darkens the sky, splashes on the ground and sounds, but nothing falls through the air)."
- Also stale once this lands: `:265` lists "lectern books" under block entities that are not simulated.

## 2. What exists today

| Area | State | Where |
|---|---|---|
| Powder snow collision | The state's collision boxes come from data with an empty context, so powder snow has no boxes and no shape flags: `shape_id` gives it id 0 and the player falls straight through at full speed. | `render_anim/src/minecraft_world.rs:2501-2540` (`shape_id`) |
| Voxel flags | One `u16` per shape id: `0x0f` amount, `0x10` water, `0x20` lava, `0x40` climbable, `0x80` scaffolding, `0x100` scaffolding bottom, `0x200` berry bush, `0x400` cobweb. Track D claims `0x7800` for rail shapes, so only `0x8000` is free. | `sim/src/voxel.rs:546-557` |
| Mover-dependent shapes | `Mover { feet, descending }` (`Mover::of(ps, cmd)`) switches scaffolding between full, slab and empty in `shape_for`. | `voxel.rs:559-569`, `:786-803` |
| Stuck blocks | `stuck_in` ORs the flags of every cell the body touches; cobweb `[0.25, 0.25, 0.05]`, berry bush `[0.8, 0.8, 0.75]` (map x, map y, vertical). `stuck_move` runs a vanilla-tick walk with those multipliers and climbs when `Stuck.climb` is set. | `voxel.rs:1247-1267`; `movement_iw4/src/stuck.rs:32-125`; dispatch `movement_iw4/src/single.rs:307-310` |
| Climbing | `ClimbSample { on, scaffolding }` from the feet cell. `climb_move` on the ground only walks (friction, `walk_push`); jump just makes `rise_after_move` set `CLIMB_RISE` (0.2 a tick, less gravity) after the move. `mantle_buttons` strips JUMP while climbing. There is no ground jump. | `movement_iw4/src/climb.rs:26-29`, `:44-51`, `:72-153`; `voxel.rs:805-808` |
| Inside-block effects | Vendored `block_effects`: powder snow only extinguishes. `ClearFreeze` is a no-op; there is no `Freeze` kind and no `Hazard::Freeze`. `APPLY_ORDER` is ClearFreeze, FireIgnite, LavaIgnite, Extinguish. | `third_party/minecraftoss/entities/src/inside_blocks.rs:30-73`, `:76-108`, `:445-478` |
| Player hazards | `Survival::hazards` runs per tick from `render_anim` for the local player only **[E]**; `BlockEffect::ClearFreeze => {}`. No ticks frozen, no `freeze_damage` rule, no `TicksFrozen` in the player file. | `minecraft_survival/src/survival.rs:470-567`; call `render_anim/src/minecraft_world.rs:950-985`; owned keys `minecraft_survival/src/player_nbt.rs:122-142` |
| Falls on powder snow | Already no damage (`breaks_fall` lists it); no landing sound. | `survival.rs:983-1001`, `:1011-1021`; landing code `minecraft_world.rs:876-905` |
| Powder snow buckets | Done: fill, place (pitch 0.8), cauldron, dispenser. Nothing to add beyond the test steps. | `minecraft_terrain/src/bucket.rs:14-33`, `:258-282`, `:443-458` |
| Hearts | Poisoned and withered sprites; no frozen sprite. The gui atlas already loads all of `hud/heart/` (frozen included). | `hud/src/minecraft_vitals.rs:63-85`; `render_anim/src/minecraft_gui.rs:38-52`, `:220-225` |
| Screen overlays | The portal overlay is the model: a full-screen textured quad with an alpha. | `hud/src/minecraft_portal.rs:1-60`; `frame/src/minecraft_ui.rs:245-249` |
| Lava fog | `lava_fog::in_lava` replaces the sky state while the eye is in lava; nothing for powder snow. | `minecraft_terrain/src/lava_fog.rs`; call `minecraft_world.rs:2251-2253` |
| Totem | `player::hold_off_death` spends the totem and gives the effects; the `render_anim` wrapper only plays `item.totem.use`. MW2 damage pops arrive through `sim::voxel::take_totem_pops`. No item activation, no particles. | `minecraft_survival/src/survival.rs:872-891`, `player.rs:296`; `render_anim/src/minecraft_world.rs:792`, `:1863`, `:2575-2588` |
| Held item view | A view-space mesh drawn with a fixed 70 degree projection (`hand_clip`), built only while an item is held and not in third person. `item_display_transform(packs, id, context)` gives any display context. | `minecraft_world.rs:2114-2160`; `minecraft_terrain/src/model.rs:191-218`; `minecraft_survival/src/entities.rs:493` |
| Particles with alpha | `AmbientParticles` has per-particle alpha, gravity, friction, sprites by age and routes translucent kinds to the translucent entity mesh. | `minecraft_terrain/src/ambient_particles.rs:28-60`, `:240-262`, `:732-800`; owner `minecraft_survival/src/entities.rs:70`, drawn at `:1283` |
| Trapped chests | Openers are counted per `Level` and per player key on the main thread and drive lids and sounds; the server thread never hears the count. The vendored redstone treats `ChestBlock` (trapped included) as a plain container: analog output = fill, own signal 0. | `minecraft_terrain/src/openers.rs:37-104`; `minecraft_survival/src/level.rs:245-310`; `render_anim/src/minecraft_world.rs:2022-2046`; `third_party/minecraftoss/world/src/level/redstone.rs:81-129`, `:242-290`, `:337-372`, `:449-497` |
| Lecterns | Placement states and the book model on `has_book=true` exist. No use: right-click with a book does nothing. The block entity keeps `Book` and `Page` when loaded and saved; removal with a `Book` is logged as unsupported. No lectern redstone. | `third_party/minecraftoss/player/src/placement.rs:93`; `minecraft_terrain/src/special_models.rs:344-360`; `third_party/minecraftoss/core/src/block_entity.rs:251-256`; `world/src/level/drops.rs:123-135` |
| Server-owned block entities | Every block entity the containers don't own is the server thread's: changes come back in `Output.block_entities` and are saved. | `minecraft_terrain/src/server.rs:2151-2169`; `minecraft_survival/src/level.rs:176-183` |
| Server commands | `Command::UseBlock { pos, facing }` and `ItemOnBlock { pos, action }` (no player id); `throw_eye(PLAYER, ...)` uses `const PLAYER: u64 = 0` **[E]**. | `server.rs:31-39`, `:2189-2238`; `minecraft_survival/src/entities.rs:36`, `:574-600` |
| Right-click dispatch | Tables, benches, stations, campfires, respawn blocks, then `entities.use_block`. | `render_anim/src/minecraft_world.rs:1195-1270` |
| Books | No book screen, no book editing, no `writable_book_content`/`written_book_content` support. `/give` accepts only enchantment components; the kit splits on every comma. | `minecraft_commands/src/args.rs:51-56`, `:337-394`; `minecraft_commands/src/kit.rs:36-56` |
| GUI | Sheets are `gui/container/*`; sprites are allow-listed (no `widget/`); text keys are allow-listed by prefix (no `book.`, `lectern.`, `gui.done`). Track B4 adds `widget/button` and `widget/button_highlighted`. | `render_anim/src/minecraft_gui.rs:18-68`, `:206-240` |
| Rain | Splashes, smoke and the rain sound (`rain_tick`, radius 10, `rains_at` = has precipitation and temperature >= 0.15). No streaks; no snow at all. | `minecraft_terrain/src/rain.rs:10-16`, `:59-131`, `:148-171`; `minecraft_survival/src/entities.rs:713-714` |
| World renderer | Sky, sections, particles, mobs, translucent sections, cracks, hand, clouds; the frame is copied from `MinecraftWorldView` in `render/src/extract.rs`. | `render_gpu/src/drawsurf/minecraft_world.rs:64-101`, `:723-860`, `:1306-1520`; `render_anim/src/minecraft_world.rs:33-73`; `render/src/extract.rs:981-1120` |

## 3. Vanilla 26.3 behaviour (javap)

### Powder snow (`PowderSnowBlock`, `Entity`, `LivingEntity`, `InsideBlockEffectType`)

- **Collision** (`getCollisionShape`): for an entity context only (not placement):
  - fall distance > 2.5 gives `FALLING_COLLISION_SHAPE`, a box 0.9 high;
  - otherwise, if the entity can walk on powder snow (`canEntityWalkOnPowderSnow`: in `#powder_snow_walkable_mobs` (rabbit, endermite, silverfish, fox) or **leather boots in the feet slot**), the bottom of its box is above the block top (`isAbove(Shapes.block(), pos, false)`) and it is not descending (sneaking), it is a full block;
  - else empty. The visual shape is always empty.
- **Inside** (`entityInside`, `getEntityInsideCollisionShape` is the full block):
  - a living entity is made stuck (`makeStuckInBlock`, multiplier x 0.9, y 1.5, z 0.9) only when its feet block (`getInBlockState`) is powder snow; stuck also resets the fall distance, and `Entity.move` scales that move and zeroes the motion;
  - server side, when it moved horizontally and a coin flip passes: one `snowflake` particle at (x, block y + 1, z) with motion (±1/12 · r, 0.05, ±1/12 · r);
  - then, for every overlapped powder snow block: before EXTINGUISH runs, a burning entity (a player, or any mob with `mob_griefing`) that may interact there destroys the block without drops; then FREEZE; then EXTINGUISH.
- **Effect order** (`InsideBlockEffectType` enum order): FREEZE, CLEAR_FREEZE, FIRE_IGNITE, LAVA_IGNITE, EXTINGUISH.
  - FREEZE: `isInPowderSnow = true`; if `canFreeze`, ticks frozen = min(140, ticks + 1).
  - CLEAR_FREEZE: ticks frozen = 0.
- **Ticks** (`Entity.baseTick`, `LivingEntity.aiStep` "freezing", server only): each tick `wasInPowderSnow = isInPowderSnow`, then false; after the move, if not (in powder snow and `canFreeze`), ticks frozen = max(0, ticks - 2). `getTicksRequiredToFreeze` = 140; `getPercentFrozen` = min(ticks, 140) / 140; `isFullyFrozen` = ticks >= 140.
  - Every `tickCount % 40 == 0` while fully frozen and `canFreeze`: hurt by `freeze`, 1.0.
  - `canFreeze` (`LivingEntity`): false for spectators and when any armour slot holds an item in `#freeze_immune_wearables` (leather helmet, chestplate, leggings, boots, horse armour); else true unless `#freeze_immune_entity_types`.
  - `freeze` damage type: exhaustion 0, effects `freezing` (`entity.player.hurt_freeze`), in `#bypasses_armor` and `#no_knockback`, message `death.attack.freeze` ("%1$s froze to death"). Five times the damage for `#freeze_hurts_extra_types` (not players). `Player.hurt` refuses it when the `freeze_damage` game rule is off.
  - Frost slowdown (`tryAddFrost` / `removeFrost`): while ticks > 0 and the block under (`getBlockStateOnLegacy`, 0.2 below) is not air, movement speed gets an ADD_VALUE modifier of -0.05 x percent frozen. On the player's base 0.1 that is a factor of (1 - 0.5 x percent) on walking and sprinting.
  - `Entity.checkInsideBlocks` plays `entity.generic.extinguish_fire` (0.7, 1.6 + (r1 - r2) x 0.4) when the entity stopped burning, or stopped freezing, during the effects.
  - `TicksFrozen` (int) is saved only when > 0 and loaded with default 0. Respawn sets 0.
- **Climbing out** (`LivingEntity.handleRelativeFrictionAndCalculateMovement`): after the move, if (horizontal collision or jumping) and (on a climbable, or `wasInPowderSnow` and can walk on powder snow), y motion = 0.2. Only real climbables get the ±0.15 clamp (`handleOnClimbable`).
- **Landing** (`fallOn`): no damage (it does not call the base fall); a living entity falling >= 4 plays its small fall sound, >= 7 the big one, at 1.0, 1.0.
- **Overlay** (`Hud.extractCameraOverlays`, all camera modes): while ticks frozen > 0, `textures/misc/powder_snow_outline.png` blitted over the whole screen in white with alpha = percent frozen.
- **Hearts** (`Hud$HeartType.forPlayer`): poisoned, else withered, else **frozen** when fully frozen, else normal. Absorbing hearts stay absorbing (withered only replaces them). Sprites `hud/heart/frozen_full`, `frozen_half` and the blinking and hardcore variants.
- **Fog** (`PowderedSnowFogEnvironment`, camera block is powder snow): colour `#9FBECC`; environmental start 0, end 2 (spectator: -8 to half the render distance); sky and cloud ends = environmental end.
- **Snowflake particle**: gravity 0.225, friction 1.0, motion = given + (r x 2 - 1) x 0.05 on each axis, size 0.1 x (1 + r x r), lifetime (int)(16 / (r x 0.8 + 0.2)) + 2, sprites `generic_7` to `generic_0` by age, opaque; each tick motion x 0.95, y x 0.9, z x 0.95 after the base move.

### Ladder ground jump (`LivingEntity.aiStep`, `jumpFromGround`, `handleRelativeFrictionAndCalculateMovement`)

- While jumping on the ground (not in a deep fluid) with `noJumpDelay == 0`: `jumpFromGround` sets y = max(jump power, y), jump power 0.42 x block jump factor (+0.1 per Jump Boost level), and when sprinting adds 0.2 along the facing; `noJumpDelay = 10` (reset to 0 whenever jump is released).
- Then on a climbable `handleOnClimbable` clamps x and z to ±0.15 and y to >= -0.15 (y 0 when sneaking, except scaffolding); the 0.42 passes. After the move, jumping on a climbable sets y = 0.2, then gravity (0.2 - 0.08) x 0.98.
- So the first tick of a jump at the foot of a ladder rises 0.42 instead of about 0.12, and a single tap lifts the player about 0.58 blocks (0.42 + 0.118 + 0.037) before sliding down.

### Totem pop (`ClientPacketListener.handleEntityEvent` 35, `ItemActivation`, `ScreenEffectRenderer`, `TrackingEmitter`, `TotemParticle`)

- Event 35 on any entity: a tracking emitter of `totem_of_undying` for 30 ticks, and `item.totem.use` at the entity (1.0, 1.0). If it is the local player: `displayItemActivation(findTotem(player))` (the hand stack with `death_protection`, else a new totem).
- `ItemActivation`: 40 ticks; random `offX`, `offY` in [-1, 1); ticks down once per client tick.
- Drawn by `ScreenEffectRenderer` after the block, water and fire overlays, in every camera mode, unless the HUD is hidden (F1):
  - t = (40 - ticks + partial) / 40;
  - c = 10.25t⁵ - 24.95t⁴ + 25.5t³ - 13.8t² + 4t; a = c · π;
  - aspect = window width / height;
  - translate (offX · 0.3 · aspect · |sin 2a|, offY · 0.3 · |sin 2a|, -10 + 9 sin a);
  - scale 0.8; rotate Y by 900 · |sin a| degrees, X by 6 cos(8t) and Z by 6 cos(8t);
  - the item in the `FIXED` display context (`LevelExtractor`), full bright (`15728880`), no overlay, with the hand's projection.
- `TrackingEmitter`: emits once on creation, then each tick, until life >= 30. Each emission makes 16 tries of (x, y, z) uniform in [-1, 1); tries outside the unit sphere are skipped; a kept try adds a particle at (entity x + width · x/4, entity y + height · (0.5 + y/4), entity z + width · z/4) with motion (x, y + 0.2, z).
- `TotemParticle` (a `SimpleAnimatedParticle`):
  - gravity 1.25, friction 0.6, size x 0.75, lifetime 60 + `nextInt(12)`;
  - sprites `glitter_7` to `glitter_0` by age;
  - colour: one in four (r = 0.6 + r·0.2, g = 0.6 + r·0.3, b = r·0.2), else (0.1 + r·0.2, 0.4 + r·0.3, r·0.2);
  - translucent, full bright; after half its life, alpha = 1 - (age - lifetime/2) / lifetime.

### Trapped chests (`TrappedChestBlock`, `TrappedChestBlockEntity`, `ContainerOpenersCounter`)

- `isSignalSource` true. Own signal = clamp(the chest's opener count, 0, 15) to every side; direct (strong) signal only when the direction is UP (it strongly powers the block below).
- Opener count is per chest block entity (each half of a double chest counts its viewers; opening a double chest counts on both). On a count change it updates neighbours at its position and below. Spectators don't count; the counter re-checks every 5 ticks.
- Comparator output is the chest's (container fill), as for a plain chest. The count is not saved.

### Lecterns (`LecternBlock`, `LecternBlockEntity`, `LecternMenu`, `LecternScreen`, `BookViewScreen`, `PageButton`)

- Books: `#lectern_books` = `written_book`, `writable_book`. `writable_book` has a default `writable_book_content` (empty); `written_book` has none by default (`Items`).
- `useItemOn`: on `has_book=true` try the empty hand; with a lectern book, `tryPlaceBook`; otherwise pass. Placing:
  - the block entity takes one book (`consumeAndReturn(1)`, so creative keeps it), page 0, page count = pages of `written_book_content` or `writable_book_content`;
  - the state becomes `has_book=true, powered=false`, neighbours below are updated;
  - `item.book.put` at the block (blocks, 1.0, 1.0).
- `useWithoutItem` with a book opens the menu (stat `interact_with_lectern`).
- Menu buttons (`clickMenuButton`): 1 previous page, 2 next page, 3 take the book (only if the player `mayBuild`; it goes to the inventory or drops), 100 + n go to page n. Page = clamp(n, 0, count - 1); only a real change calls `signalPageChange`.
- `signalPageChange`: `powered=true`, update below, block tick in 2 ticks, level event 1043 (`book.page_turn` at the block, 1.0, 0.9 + r · 0.1). The tick sets `powered=false` and updates below.
- Signals: signal source; own signal 15 while powered, direct signal 15 only towards UP.
- Comparator (`getRedstoneSignal`): f = page / (count - 1) if count > 1, else 1.0; output = floor(f · 14) + (1 if the book has content, else 0). Without a book 0. So a 3-page book reads 1, 8, 15, and an empty book and quill reads 15.
- Removing the book (take or break) resets page and count, `has_book=false`, `powered=false`, updates below. Breaking drops the book as an item at (x + 0.5 + 0.25 · facing x, y + 1, z + 0.5 + 0.25 · facing z) with the default pickup delay.
- `Book` (item) and `Page` (int) are saved; `Page` is clamped on load.
- **Screen** (`BookViewScreen`, `LecternScreen`):
  - `textures/gui/book.png`, 192 x 192 from a 256 sheet, at left = (width - 192) / 2, top 2;
  - page text from left + 36, top + 30, 114 wide, up to 128 / 9 = 14 lines of 9 pixels, black (`0xFF000000`) without shadow;
  - page indicator `book.pageIndicator` ("Page %1$s of %2$s") right-aligned at (left + 148, top + 16);
  - page buttons 23 x 13 at (left + 116, top + 157) forward and (left + 43, top + 157) back, sprites `widget/page_forward`, `widget/page_backward` and `_highlighted`; back shows only after page 0 and forward only before the last page; a click plays `book.page_turn` as a UI sound; Page Up and Page Down press them;
  - controls at y = top + 192 + 2 = 196: if the player may build, "Done" (`gui.done`) at (width/2 - 100, 196) and "Take Book" (`lectern.take_book`) at (width/2 + 2, 196), each 98 wide; otherwise one "Done", 200 wide, centred;
  - the screen does not pause; written pages are text components, writable pages plain strings.

### Weather streaks (`WeatherEffectRenderer`, `ClientLevel.getPrecipitationAt`, `RenderPipelines.WEATHER`, `LevelRenderer`)

- Every frame with rain level (interpolated) > 0, radius r = `weatherRadius` option (3 to 10, default 10): for each column within r of the camera block:
  - h = MOTION_BLOCKING height; bottom = max(camY - r, h), top = max(camY + r, h); skip if equal;
  - precipitation = the biome at (x, camY, z) (`getPrecipitationAt`: NONE without precipitation, SNOW when too cold to rain, i.e. temperature < 0.15, else RAIN); skip NONE;
  - seed a legacy random with (x² · 3121 + x · 45238971) ^ (z² · 418711 + z · 13761);
  - light at (x, max(camY, h), z).
- Rain column: offset = (x² · 3121 + x · 45238971 + z² · 418711 + z · 13761) & 255; speed = 3 + r.nextFloat(); v offset = (-(((gameTime & 131071) + offset + partial) / 32) · speed) mod 32; u offset 0.
- Snow column: t = (gameTime & 131071) + partial; u = r.nextDouble() + t · 0.01 · r.nextGaussian(); v jitter = r.nextDouble() + t · r.nextGaussian() · 0.001; v = -((gameTime & 511) + partial) / 512 + v jitter; light per channel (l · 3 + 15) / 4.
- Quads: centre (x + 0.5, z + 0.5); half widths from a 32 x 32 table: for dx = column - camera column, dz likewise, len = √(dx² + dz²), size x = -dz / len / 2, size z = dx / len / 2 (a 1-block quad facing the camera across the ground).
  - Corners (x - sx, top, z - sz), (x + sx, top, z + sz), (x + sx, bottom, z + sz), (x - sx, bottom, z - sz);
  - u 0 and 1 plus the u offset; v = bottom · 0.25 + v offset on the top edge and top · 0.25 + v offset on the bottom edge.
  - Alpha = lerp(min(dist² / r², 1), base, 0.5) · rain level with base 1.0 for rain and 0.8 for snow (1.0 or 0.8 at the camera, 0.5 at the edge).
- Textures `textures/environment/rain.png` and `snow.png`, repeating; particle shader (texture x colour, lightmap, fog); translucent blend, no culling, depth test, **no depth write**.
- Drawn after translucent features, translucent terrain and particles, and after the clouds, before the world border (`LevelRenderer.addMainPass`).

## 4. Plan (commit groups, in order)

Rules for every group: no comments, no reformatting of existing files, new files pass `rustfmt --check`, `minecraft_terrain` stays edition 2021 (no let-chains). Run `check -p`/`test -p` on the touched crates and `check -p launcher` before each commit.

### Group 1. "Minecraft ladders take a ground jump like vanilla"

Files: `crates/movement_iw4/src/climb.rs`.

- In `climb_move`, when `pml.walking != 0`, run the MW2 jump gate the way `stuck_move` does (`stuck.rs:43-55`: clear a stale JUMPING, `jump::check` with `JumpCheckContext`, stamp `ps.jump_time`).
- While within one tick (50 ms) of the launch, the vertical velocity is the jump power 0.42 a tick (`stuck.rs` `JUMP_POWER`), and a sprinting player gets the 0.2 push along the facing. Then `hold` clamps the horizontal speed as today.
- `rise_after_move` then sets `CLIMB_RISE` while jump stays held, as now. Exhaustion is already counted from the JUMPING flag.
- Tests:
  - `a_ground_jump_at_a_ladder_rises_the_vanilla_first_tick`: one tick from the ground rises 0.42 blocks ±1 %, then 0.1176 a tick while held, at 125 and 333 fps;
  - `a_tap_at_a_ladder_lifts_about_half_a_block`: peak 0.575 ±0.02;
  - `the_jump_waits_for_the_gate`: holding jump does not relaunch within the MW2 gate.

### Group 2. "Minecraft powder snow holds, sinks and slows players, and leather boots walk on it"

Files: `crates/sim/src/voxel.rs`, `crates/sim/src/step.rs`, `crates/sim/src/frame.rs`, `crates/sim/src/carrier.rs`, `crates/playerstate_iw4/src/playerstate.rs`, `crates/movement_iw4/src/climb.rs`, `crates/movement_iw4/src/stuck.rs`, `crates/movement_iw4/src/single.rs`, `crates/render_anim/src/minecraft_world.rs`, `third_party/minecraftoss/entities/src/inside_blocks.rs`, `crates/minecraft_terrain/src/ambient_particles.rs`, `crates/minecraft_survival/src/entities.rs`.

- **Voxel flags** (`voxel.rs`):
  - `pub const POWDER_SNOW: u16 = 0x8000` (the last free bit; D takes `0x7800`). `shape_id` (`minecraft_world.rs:2513-2520`) sets it for `minecraft:powder_snow`.
  - `Mover` gains `powder_walker: bool` and `falling: bool`. `Mover::of` reads the walker from a new pm_flags bit `MC_POWDER_WALKER`; take an unused upper bit, checked with grep across `crates/` (`0x0080_0000` is a candidate; `0x0100_0000` is B3's `MC_RIDING`).
  - `falling` is true when -velocity z > √(2 · `ps.gravity` · 2.5 · `BLOCK`): the speed of a 2.5-block free fall from rest under the pmove's gravity, standing in for vanilla's fall distance > 2.5.
  - `shape_for` with a mover on a POWDER_SNOW cell: `falling` gives `[0, 0, 0, 1, 0.9, 1]`; else a full block when `powder_walker`, the feet are above `y + 1 - ABOVE_EPSILON` and not `descending`; else nothing. Without a mover it stays empty (bullets and projectiles pass, as vanilla's non-entity contexts).
  - `stuck_in`: cobweb, then berry bush, then powder snow when the **feet cell** (floor of `at`) has POWDER_SNOW, giving `[0.9, 0.9, 1.5]`.
- **Climbing out:**
  - `ClimbSample` gains `powder: bool`. `voxel::climb` takes `Option<Mover>` and sets `on` and `powder` when the feet cell is powder snow and the mover is a walker; keep a mover-less wrapper for the footstep code at `minecraft_world.rs:840`. `ClipBackend::climb` passes `self.mover` (`step.rs:1898-1904`).
  - `hold` is skipped when `powder` (vanilla clamps only real climbables).
  - In `stuck_move`, `climb` set by a walker in powder snow rises 0.2 on jump or a blocked move; `rise_after_move` already does this.
- **Walker bit:** `SimWorld::set_powder_snow_walker(id, bool)` (`frame.rs`, `carrier.rs`, like `set_may_glide` at `frame.rs:427-437`). `render_anim` sets it each frame from boots slot 36 being `minecraft:leather_boots` **[E: per player from that player's inventory]**.
- **Inside effects** (`inside_blocks.rs`): `inside_block` gets the block position and the body's feet block. Powder snow gives `BlockEffect::Stuck((0.9, 1.5, 0.9))` only when the feet block is that block, then the extinguish as now. `Survival::hazards` already resets the fall on `Stuck` (`minecraft_world.rs:972-974`).
- **Landing sound:** in the landing branch (`minecraft_world.rs:876-905`), a fall of >= 4 onto powder snow (the cell at the feet) plays `entity.player.small_fall`, >= 7 `entity.player.big_fall`, at 1.0, 1.0, with no damage.
- **Snowflakes:** `ambient_particles::Kind::Snowflake` with the section 3 numbers. `Entities::powder_snow_flake(feet_x, block_y, feet_z)` adds one with motion (±1/12 · r, 0.05, ±1/12 · r). `render_anim` calls it on a coin flip each tick that the feet block is powder snow and the player moved horizontally **[E: vanilla sends these from the server to everyone near]**.
- Tests:
  - voxel: `powder_snow_catches_a_fall_at_nine_tenths`, `leather_boots_stand_on_powder_snow_until_sneaking`, `powder_snow_holds_only_through_the_feet_cell`, `powder_snow_needs_no_collision_without_a_mover`;
  - movement: `powder_snow_sinks_at_the_vanilla_rate` (0.1176 a tick ±2 %, horizontal walk x 0.9), `a_walker_climbs_out_of_powder_snow_at_two_tenths`;
  - inside_blocks: `powder_snow_sticks_only_by_the_feet`;
  - ambient: `snowflakes_live_and_slow_like_vanilla`.

### Group 3. "Minecraft powder snow freezes players like vanilla: frost, damage and melting"

Files: `third_party/minecraftoss/entities/src/inside_blocks.rs`, `third_party/minecraftoss/entities/src/world/hazards.rs` (exhaustive matches only), `crates/minecraft_survival/src/survival.rs`, `crates/minecraft_survival/src/death.rs`, `crates/minecraft_survival/src/player_nbt.rs`, `crates/render_anim/src/minecraft_world.rs`, `crates/sim/src/step.rs`, `crates/sim/src/frame.rs`, `crates/sim/src/carrier.rs`, `crates/movement_iw4/src/stuck.rs`.

- **Vendored inside blocks:**
  - `Kind::Freeze` first in `APPLY_ORDER` and `BlockEffect::Freeze`.
  - `BlockEffect::MeltIfBurning(Pos)` runs before EXTINGUISH: add a `before` list to `Collector`, flushed ahead of its kind.
  - Powder snow applies MeltIfBurning, Freeze and Extinguish for every overlapped powder snow block.
  - `Hazard::Freeze`: damage type `minecraft:freeze`, `bypasses_armor` true, not fire, no panic.
- **`Survival`:**
  - fields `ticks_frozen: i32` and `in_powder_snow: bool`; `SurvivalRules.freeze_damage` from the `freeze_damage` rule (`game_rules.rs:128`); `Motion.freeze_immune` (any leather armour piece in slots 36-39, or spectator), filled in `render_anim`;
  - in `hazards`: Freeze sets `in_powder_snow` and adds 1 (max 140) unless immune; ClearFreeze sets 0; after the effects, -2 (min 0) unless in powder snow and not immune;
  - every `tick % 40 == 0` while >= 140 and not immune: `hurt_aloud(Hazard::Freeze, 1.0)` when the rule allows, with `entity.player.hurt_freeze`;
  - the extinguish sound also plays when ticks frozen went from > 0 to 0 within the effects;
  - `MeltIfBurning(pos)` while `fire_ticks > 0` returns the position in a new `Survival::take_melted()`; `render_anim` writes air there with `write_blocks` like Frost Walker (`minecraft_world.rs:968-971`), with the block's break particles and sound;
  - `respawn` resets ticks (default); `restore` takes it; `Vitals` gains `frozen: f32` (percent) and `fully_frozen: bool`.
- **Death and save:** `death.rs` adds `Hazard::Freeze` → `minecraft:freeze` and message id `freeze` ("%1$s froze to death"; `.player`: "%1$s was frozen to death by %2$s"). `player_nbt.rs` owns `TicksFrozen` (int, written only when > 0).
- **Frost slowdown:**
  - `SimWorld::set_minecraft_move_scale(id, f32)` keeps a per-client factor (default 1) that `step.rs:2164` feeds into `WalkMoveContext.weapon_move_scale`, which is always 1.0 today;
  - `stuck_move` multiplies its walk speeds by `context.weapon_move_scale`;
  - `render_anim` sets 1 - 0.5 x percent frozen while ticks > 0 and the block 0.2 below the feet is not air, else 1 **[E: not replicated; joined clients will mispredict until E sends it]**.
  - This deliberately leaves `ps.move_speed_scale_multiplier` alone, because MW2's `_weapons.gsc` writes it for weapon classes.
- Tests:
  - survival: `seven_seconds_in_powder_snow_freeze_a_player`, `frozen_players_take_one_every_two_seconds`, `thawing_takes_two_ticks_per_tick`, `leather_armour_keeps_a_player_from_freezing`, `the_freeze_damage_rule_stops_the_damage`, `fire_clears_the_frost_with_the_extinguish_sound`, `a_burning_player_melts_the_powder_snow_it_enters`;
  - death: `freezing_to_death_has_the_vanilla_message`;
  - player_nbt: `ticks_frozen_round_trips_and_is_left_out_at_zero`;
  - inside_blocks: `powder_snow_freezes_before_it_extinguishes`.

### Group 4. "Frozen Minecraft players see the powder snow outline, frozen hearts and powder snow fog"

Files: `crates/frame/src/minecraft_ui.rs`, `crates/hud/src/minecraft_vitals.rs`, `crates/hud/src/minecraft_freeze.rs` (new), `crates/hud/src/lib.rs` (or `plugin.rs`, wherever `minecraft_portal` is registered), `crates/hud/src/gpu_list.rs` (a `minecraft_freeze` job beside `minecraft_portal`), `crates/render_anim/src/minecraft_world.rs`, `crates/minecraft_terrain/src/lava_fog.rs`.

- `McVitals` gains `frozen: f32` and `fully_frozen: bool` (`mc_vitals`, `minecraft_world.rs:2651-2673`).
- `heart_sprite` picks the `frozen_*` sprites for normal hearts when fully frozen and neither poisoned nor withered; absorbing hearts unchanged.
- `MinecraftUi.freeze_outline: Option<Handle<Image>>`, loaded once from `minecraft:misc/powder_snow_outline` the way the fire and portal strips are loaded.
- `hud/src/minecraft_freeze.rs` draws it full-screen in white with alpha = `vitals.frozen` while > 0, in every camera mode, under the HUD hide rule the portal overlay uses.
- `lava_fog::in_powder_snow(sky, spectator, render_distance)`: colour `0x9F_BE_CC`, start 0 and end 2 (spectator -8 and half the render distance), sky and cloud ends = end, cloud alpha 0, no skybox, as `in_lava`. Called at `minecraft_world.rs:2251` when the eye block is `minecraft:powder_snow` (after the lava check).
- Tests:
  - vitals: `fully_frozen_players_get_frozen_hearts_but_poison_wins`;
  - freeze overlay: `the_outline_fades_with_the_frost` (pure quad builder like `minecraft_portal::overlay`);
  - fog: `powder_snow_fog_is_two_blocks_of_pale_blue`.

### Group 5. "Minecraft totems pop with vanilla's floating totem and glitter"

Files: `crates/render_anim/src/minecraft_hand.rs`, `crates/render_anim/src/minecraft_world.rs`, `crates/minecraft_terrain/src/ambient_particles.rs`, `crates/minecraft_survival/src/entities.rs`.

- `minecraft_hand::ItemActivation { ticks: u32, off: [f32; 2] }` with `activate(random)` (40 ticks, offsets 2r - 1), `tick()` and a pure `activation_pose(ticks, partial, off, aspect) -> Option<Mat4>` with the section 3 curve.
- `render_anim`'s `hold_off_death` wrapper (`minecraft_world.rs:2575-2588`), on a pop:
  - sets the local hand's activation (client-local, as vanilla's `LocalPlayer.itemActivation`);
  - calls `Entities::totem_pop(feet, width 0.6, height)` **[E: the host must tell every client that sees this player, like vanilla's entity event 35]**.
- Hand view:
  - the activation is drawn whenever it is active and the HUD is shown, also with an empty hand, an MW2 gun or in third person;
  - build the `posed` list and `view.hand` when an activation is active even if nothing is held (`minecraft_world.rs:2119`), adding `(activation_pose · item_display_transform(packs, "minecraft:totem_of_undying", "fixed"), id, false)`;
  - set that item's vertices to full sky and block light after `held_item_mesh`;
  - in third person draw only the activation.
- `ambient_particles`:
  - `Kind::Totem` (translucent, full bright, sprites `glitter_7`..`glitter_0` by age, the section 3 gravity, friction, size, lifetime, colours and fade; add the glitter textures to `textures()`);
  - an emitter list `(entity key, life)` whose `tick` takes the tracked body's current feet, width and height and makes the 16 sphere tries a tick for 30 ticks;
  - the local player's key is its UUID **[E: remote players' bodies]**.
- Tests:
  - hand: `the_activation_rises_spins_and_returns_like_vanilla` (pose at t = 0, 0.25, 0.5, 1: z = -10 at both ends, the x and y offsets 0 at t = 0 and t = 1 (|sin 2a| = 0), spin 900 · |sin a|);
  - particles: `a_totem_emitter_runs_thirty_ticks_inside_the_body` (count and every start point inside the body box), `totem_glitter_is_mostly_green_and_fades_after_half_life`.

### Group 6. "Minecraft trapped chests power redstone by their number of viewers"

Files: `third_party/minecraftoss/world/src/level/mod.rs`, `third_party/minecraftoss/world/src/level/redstone.rs`, `crates/minecraft_terrain/src/server.rs`, `crates/minecraft_survival/src/entities.rs`, `crates/render_anim/src/minecraft_world.rs`.

- Vendored `Level` gains `chest_openers: HashMap<(i32, i32, i32), i32>` (not saved) and `pub fn set_chest_openers(pos, count)`:
  - stores or removes the count;
  - when it changed and the block is a `TrappedChestBlock`, updates neighbours at `pos` and at `pos.below()` with the chest block as source;
  - drop the entry when the block at `pos` changes (`set_block` removal path, `mod.rs:395-410`).
- `redstone.rs`:
  - `own_signal` gives clamp(count, 0, 15) for `TrappedChestBlock`;
  - `state_direct_signal` gives the same for direction `Up` only;
  - `is_signal_source` is true for it (check the registry flag first and add an explicit case only if the extracted data lacks it);
  - the analog output stays the container fill.
- `server.rs`: `Command::ChestOpeners { pos, count }` → `level.set_chest_openers`. `Entities::chest_openers(pos, count)` sends it.
- `render_anim` (`:2035-2046`): for each `OpenerEvent` on a `minecraft:trapped_chest`, send its count. Recheck and `leave` events go through the same loop (they already reach `opener_outcome`). The count is per `Level` and per player key already **[E: friends' viewers add up with no change]**.
- Tests:
  - vendored: `a_trapped_chest_powers_by_viewers_and_strongly_below` (counts 1, 3, 20 → 1, 3, 15; direct only up; neighbours updated at the chest and below);
  - server: `opening_a_trapped_chest_lights_a_lamp_beside_it` (place chest, lamp, `ChestOpeners` 1 then 0);
  - level: `a_double_trapped_chest_counts_on_both_halves`.

### Group 7. "/give and the kit take writable and written book contents"

Files: `crates/minecraft_commands/src/args.rs`, `crates/minecraft_commands/src/kit.rs`, `crates/minecraft_survival/src/commands.rs`, `crates/minecraft_terrain/src/crafting.rs` (the kit parser the game actually uses, and `kit_potion_stack`'s signature or a wrapper next to it).

- `ItemArg` gains `components: Option<serde_json::Value>`. The item component list (`args.rs:337-394`) accepts:
  - `writable_book_content={pages:[...]}` (strings, or `{raw:...}` filterable strings);
  - `written_book_content={title:...,author:...,pages:[...]}` (pages are text components: a string or a JSON-like compound).
- Read the SNBT value with balanced braces and quotes, parse it with `minecraftoss_core::snbt::parse_compound`, check the keys (unknown or wrong types give `arguments.item.component.malformed` like the enchantment path), and store it as the stack's `minecraft:...` component JSON in the shape the saved item NBT already uses.
- Other components keep `arguments.item.component.unknown`.
- The kit line: the game does not use `minecraft_commands::kit::kit_entries` (that is only the `World` trait's default, `minecraft_commands/src/server.rs:167`, used by the command tests). The starting kit (`crafting::give_kit`, called from `player.rs:287` and `commands.rs:1966`) and `/kit` (`minecraft_survival/src/commands.rs:690`) both parse with `minecraft_terrain::crafting::kit` (`crafting.rs:402`), then `kit_enchantments` (splits on `+`) and `kit_potion` (splits on `@`). So:
  - `crafting::kit` splits on commas outside `[]`, `{}` and quotes and takes the `*count` only after the closing `]`, and `kit_enchantments` and `kit_potion` only look at the part before a `[` component list, so page text with `,`, `*`, `+` or `@` survives;
  - `give_kit` and `commands.rs:690` carry the components onto the stack;
  - `minecraft_commands::kit::kit_entries` gets the same comma rule so the test world matches.
- `give` copies `components` onto the stack.
- Tests:
  - args: `a_written_book_takes_title_author_and_pages`, `writable_pages_take_plain_and_raw_strings`, `bad_book_content_is_malformed`;
  - kit: `kit_entries_keep_commas_inside_components` (in `minecraft_commands`) and `give_kit_puts_book_pages_on_the_stack` (in `crafting.rs`, a written book whose page holds `,`, `+` and `@`);
  - commands: `give_puts_book_pages_on_the_stack`.

### Group 8. "Minecraft lecterns hold books, pulse on page turns and feed comparators"

Files: `third_party/minecraftoss/world/src/level/lectern.rs` (new vendored module, counted in the fork's added files), `third_party/minecraftoss/world/src/level/mod.rs`, `third_party/minecraftoss/world/src/level/redstone.rs`, `third_party/minecraftoss/world/src/level/drops.rs`, `crates/minecraft_terrain/src/lectern.rs` (new), `crates/minecraft_terrain/src/lib.rs`, `crates/minecraft_terrain/src/server.rs`, `crates/minecraft_survival/src/entities.rs`, `crates/render_anim/src/minecraft_world.rs`.

- `minecraft_terrain::lectern` (pure, fork code):
  - `page_count(book: &ItemStack-like id + components JSON)`; `has_content(...)` (writable books always; written only with `written_book_content`);
  - `comparator_signal(page, count, has_content)` with the section 3 formula;
  - `pages_text(...) -> Vec<String>`: written pages flattened to plain text (`text` plus `extra`, `translate` falling back to its key); writable pages as is; `filtered` ignored;
  - `drop_point(pos, facing)`.
- Vendored `level/lectern.rs` (all on the server thread's `Level`; the block entity tag holds `Book` and `Page` as saved today):
  - `lectern_place_book(pos, book: Tag) -> bool` (false if `has_book`);
  - `lectern_set_page(pos, page) -> bool` (clamped; on a change sets `powered=true`, updates below, schedules a block tick in 2 and pushes level event 1043);
  - `lectern_take_book(pos) -> Option<Tag>`;
  - `lectern_view(pos) -> Option<(Tag, i32)>`.
  - `tick_block` routes `LecternBlock` (via a new `redstone::Kind::Lectern`) to clear `powered` and update below.
  - `redstone.rs`: own signal 15 while powered, direct 15 only `Up`, signal source; `analog_output` reads the block entity for the comparator value (it can't follow the state alone).
  - `drops.rs:123-135`: a lectern with `Book` drops it at the drop point as one item entity with the default pickup delay, instead of logging it as unsupported.
- `server.rs`:
  - `ItemOnBlock::PlaceBook { book: String }` (the stack as SNBT or JSON, count 1), answered through `item_uses` like the bucket path, with `item.book.put` at the block;
  - `Command::Lectern { player: u64, pos, action: LecternAction }` with `Open`, `Button(i32)` and `Close`;
  - the server keeps `lectern_viewers: HashMap<u64, BlockPos>` and answers in `Output.lecterns: Vec<LecternView { player, pos, book: Option<String>, page: i32, taken: Option<String>, closed: bool }>`;
  - Open answers the view; `Button(1 | 2 | 100 + n)` sets the page and answers every viewer of that lectern; `Button(3)` takes the book (`taken` goes to that player's inventory or drops, on the main thread) and closes every viewer; a removed lectern or book closes its viewers (vanilla `stillValid`).
  - `player` is the server's player id from `PlayerUuid { player, uuid }`; today the local player is `PLAYER` (`entities.rs:36`) **[E: one id per joined player]**.
- `render_anim` right-click (`minecraft_world.rs:1195-1270`), before `use_block`:
  - target a lectern without a book while holding `written_book` or `writable_book` (main or offhand, not crouching with an item) → `item_on_block(pos, PlaceBook, sent)`, consuming one unless creative;
  - target a lectern with a book → `Lectern Open`, swing the hand.
  - The level event 1043 already flows through `Output.level_events`; add its sound (`book.page_turn`, 1.0, 0.9 + r · 0.1) to the client's level event handling.
- Tests:
  - lectern (pure): `the_comparator_reads_page_steps_like_vanilla` (3 pages: 1, 8, 15; one page: 15; empty book and quill: 15; written book without content: page 0 → 0), `written_pages_flatten_to_text`;
  - vendored: `a_page_turn_pulses_two_ticks`, `placing_a_book_sets_has_book_and_page_zero`, `breaking_a_lectern_drops_its_book_above`, `page_is_clamped_on_load`;
  - server: `a_book_placed_by_a_player_reaches_the_lectern_and_its_comparator`, `take_book_closes_every_viewer`.

### Group 9. "Minecraft lecterns open vanilla's book screen with page turning and Take Book"

Files: `crates/frame/src/minecraft_ui.rs`, `crates/hud/src/minecraft_book.rs` (new), `crates/hud/src/minecraft_screens.rs`, `crates/hud/src/minecraft_inventory.rs` (input), `crates/hud/src/lib.rs`, `crates/render_anim/src/minecraft_gui.rs`, `crates/render_anim/src/minecraft_inventory.rs`, `crates/render_anim/src/minecraft_world.rs`.

- `McScreen::Lectern` with `MinecraftUi.book: Option<McBook { pages: Vec<String>, page: usize, take: bool }>`.
- It opens like a block screen (`inventory_open`, cursor, Esc and E close, no slots, does not pause). Clicks reuse vanilla's menu buttons as `McClick::Choose(1 | 2 | 3)`; Done is `McClick::Close`.
- `render_anim` turns them into `Lectern Button` / `Close` and fills `ui.book` from `LecternView` answers. `take` = the player's mode may build. `taken` stacks go into the inventory with the pickup rules (`give_stacks`), else drop at the feet.
- Drawing (`minecraft_book.rs`):
  - the `book` sheet at the section 3 positions;
  - page text wrapped to 114 pixels with the gui font (word wrap as vanilla `Font.split`), at most 14 lines, black, no shadow;
  - the page indicator right-aligned at left + 148, top + 16;
  - the page buttons with highlight on hover and the visibility rules; Page Up and Page Down;
  - the two 98-wide buttons (Done, Take Book) or one 200-wide Done.
  - The page-button click plays `book.page_turn` as a UI sound.
- `minecraft_gui.rs`:
  - load `textures/gui/book.png` as a sheet;
  - add `widget/page_forward`, `widget/page_forward_highlighted`, `widget/page_backward`, `widget/page_backward_highlighted` to `LOOSE_SPRITES`, plus `widget/button` and `widget/button_highlighted` unless B4 merged them first;
  - `TEXT_PREFIXES` gains `book.`, `lectern.` and the `gui.done` key.
- Tests:
  - hud: `book_layout_matches_vanilla` (button and text rectangles at gui size 427 x 240), `page_buttons_hide_at_the_ends`, `pages_wrap_at_114_pixels_and_14_lines`;
  - gui: `the_book_sheet_and_page_sprites_load`.

### Group 10. "Rain and snow fall around the camera like vanilla's weather renderer"

Files: `crates/minecraft_terrain/src/weather.rs` (new), `crates/minecraft_terrain/src/rain.rs`, `crates/minecraft_terrain/src/lib.rs`, `crates/render_anim/src/minecraft_world.rs`, `crates/render/src/extract.rs`, `crates/render_gpu/src/drawsurf/minecraft_world.rs`.

- `rain.rs`: `pub enum Precipitation { None, Rain, Snow }` and `precipitation_at(scene, temperature, pos)` (no precipitation → None; temperature < 0.15 → Snow; else Rain). `rains_at` becomes `== Rain`, so splashes and the sound are unchanged.
- `weather.rs` (pure):
  - `pub struct WeatherVertex { position: [f32; 3], uv: [f32; 2], alpha: f32, sky: f32, block: f32, snow: f32 }` (position in block space, as the sections);
  - `pub fn columns(camera: DVec3, radius: i32, game_time: i64, partial: f32, height: impl Fn(i32, i32) -> i32, precipitation: impl Fn(BlockPos) -> Precipitation, light: impl Fn(BlockPos) -> (u8, u8)) -> Vec<Column>` with the section 3 seeds, offsets and snow light;
  - `pub fn mesh(columns, camera, radius, rain_level) -> (Vec<WeatherVertex>, Vec<u32>)` with the quad table, uv and alpha rules; rain columns first, then snow (two index ranges).
- `render_anim`, each frame with `entities.weather.rain_level > 0` in a level whose biomes have precipitation:
  - build the mesh with radius 10 (`rain::WEATHER_RADIUS`), height `motion_blocking_height`, precipitation `precipitation_at` and light from `SkyLight`;
  - use the level's game time if the client has it, else `entities.ticks()`;
  - put it in `MinecraftWorldView.weather` with the two range lengths, and load `environment/rain.png` and `environment/snow.png` once into `weather_textures`.
  - **[E: per client camera and its level's weather; nothing server-side]**
- `render_gpu`:
  - `MinecraftWorldFrame.weather`, `weather_ranges` and `weather_textures`;
  - a `weather` pipeline (the 13th entry of `pipelines()`): no culling, depth test with no depth write, alpha blend;
  - the fragment samples rain or snow by the vertex flag with a repeating sampler, multiplies alpha, applies `lightmap(sky, block)` and the terrain fog, and discards alpha < 0.1;
  - drawn after the cracks and before the hand (`minecraft_world.rs:836-842`), after the below-camera clouds as vanilla;
  - two textures need two new bindings in `layout()`, or one 2-layer array; pick whichever keeps the bind group simplest.
- `extract.rs` copies the new fields (re-converting textures only when the `Arc` changes, like the clouds).
- Tests:
  - weather: `rain_columns_scroll_at_their_seeded_speed` (offset and v for known x, z, time), `snow_columns_drift_by_gaussians`, `quads_face_the_camera_and_fade_to_half_at_the_edge`, `roofed_columns_draw_nothing`, `only_cold_biomes_snow`;
  - render_gpu: `terrain_shader_validates` (existing) and `weather_vertex_stride_matches_the_frame`.

### Group 11. "The Minecraft state doc covers powder snow, lecterns, trapped chests, totems, ladder jumps and weather streaks"

Files: `docs/MINECRAFT-SURVIVAL.md`.

- Add the features under their sections.
- Rewrite the limitation lines:
  - `:296` keeps only the copper golem statue and the chest lighting;
  - `:306` keeps the shallow-lava details;
  - `:307` drops the ground jump;
  - remove `:316`, `:326` and `:328`.
- Add the new limitations below.
- Drop "lectern books" from `:265`.
- Add `freeze_damage` to the game rules with an effect (`:249`).

New limitations to record:

- mobs don't freeze, sink or get stuck in powder snow (server-side, vendored `movement.rs:707` treats it as plain);
- the frost slowdown and powder snow walker are not replicated to joined clients yet;
- remote players' totem pops and snowflakes wait for track E;
- books can't be written or signed in game;
- written book pages show plain text (no colours, click or hover events);
- the falling shape uses the fall speed, not the fall distance;
- the weather radius is fixed at the vanilla default of 10.

## 5. The owner's test steps

Start `iw4l-survival.exe map minecraft:overworld` in survival with this kit (20 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:powder_snow_bucket,minecraft:powder_snow_bucket,minecraft:powder_snow_bucket,minecraft:bucket,minecraft:leather_boots,minecraft:leather_helmet,minecraft:flint_and_steel,minecraft:netherrack*16,minecraft:lectern*2,minecraft:writable_book,minecraft:comparator*4,minecraft:redstone*64,minecraft:redstone_lamp*8,minecraft:trapped_chest*4,minecraft:totem_of_undying,minecraft:totem_of_undying,minecraft:ladder*32,minecraft:oak_planks*64,minecraft:cobblestone*64,minecraft:torch*64
```

**Ladder (group 1)**

1. Put a ladder on a wall 6 blocks high. Stand at its foot and tap jump once: you hop up about half a block and slide back down. Hold jump: the first step up is a quick hop, then the steady climb. Before this round a tap only lifted you a hand's width. **Screenshot 1** (mid-hop).

**Powder snow (groups 2 to 4)**

2. Dig a pit 1 x 1 and 3 deep and fill it with the three powder snow buckets (one per block). Scoop the top block back up with an empty bucket (fill sound, the bucket turns into a powder snow bucket) and pour it back in.
3. Walk into the pit: you sink slowly, about 2 blocks a second, and moving is a little slower. Small white flakes rise when you move inside. Inside, the screen turns pale blue fog about 2 blocks deep. **Screenshot 2** (inside, looking out).
4. Stay inside with your head in the snow: over 7 seconds a frosty border fades in over the whole screen; when it is full, the hearts turn frozen (blue-white) and you take half a heart every 2 seconds with a crackling hurt sound. **Screenshot 3** (full frost, hearts visible).
5. Jump out without boots: you can't climb out of 3 deep. Get out by scooping the three blocks back up with your empty buckets (powder snow broken by hand drops nothing), pillaring up with cobblestone, and pouring them back in. Outside the frost fades over about 3.5 seconds, and while it lasts, walking is slower (half speed at full frost).
6. Stay inside until the frost kills you (the `freeze_damage` rule is on by default): the chat says "<name> froze to death". Respawn, then `/gamerule freeze_damage false` and go back in: the frost still builds, but there is no damage.
7. Put on the leather boots: walk over the powder snow as if it were solid. Crouch: you sink in. Inside with boots, hold jump or walk into the pit wall: you climb out at a steady pace. **Screenshot 4** (standing on the snow).
8. With the leather helmet on (boots off), stay inside: no frost border at all.
9. `/tp ~ ~12 ~` above the pit: you land on the snow surface without damage (a fall sound), then sink in.
10. Set yourself on fire (place netherrack, light it, step in) and jump into the powder snow: the fire goes out with a hiss and the snow block you are in melts away.

**Totem (group 5)**

11. Put a totem in the offhand and `/tp ~ ~40 ~`. On landing the totem pops: a big totem flies up from the middle of the screen, spins and drops back over 2 seconds, and green and yellow glitter bursts around your body for about 1.5 seconds. **Screenshot 5** (totem mid-spin).
12. Repeat with an MW2 gun selected and the second totem in the offhand; then once more in third person (F5) with the HUD shown: the floating totem shows each time and the glitter surrounds the body. **Screenshot 6** (F5).

**Trapped chest (group 6)**

13. Place a trapped chest, a redstone lamp right beside it and a line of 15 redstone dust from its other side. Open the chest: the lamp lights and exactly one dust next to the chest is lit. Close it: both go off. **Screenshot 7** (open).
14. Put a lamp under a second trapped chest: opening the chest lights it (strong power down).
15. A comparator facing out of a trapped chest reads its fill like a normal chest: put one stack in, the comparator's dust lights 1.

**Books and lecterns (groups 7 to 9)**

16. Give yourself a written book (one line in chat):

```
/give @s minecraft:written_book[written_book_content={title:"Test",author:"Owner",pages:["Page one text.","Page two text.","Page three text."]}]
```

17. Place a lectern, run a comparator out of its side into a line of 15 dust, and put a redstone lamp right beside the lectern. Right-click the lectern with the written book: it lies on the lectern with the book sound; the dust shows 1 lit.
18. Right-click the lectern: the book screen opens with "Page one text." and "Page 1 of 3", the forward arrow and the Done and Take Book buttons. **Screenshot 8.**
19. Click the forward arrow: page 2, a page sound, the lamp flashes, and the dust shows 8 lit. Forward again: page 3, the forward arrow is gone, 15 lit. Page Up goes back. **Screenshot 9** (page 2 with the dust).
20. Click Take Book: the screen closes, the book is in your inventory, the lectern is empty and the dust is dark.
21. Put the book and quill on the second lectern: the screen shows "Page 1 of 1" with an empty page, and its comparator gives 15 (vanilla does the same).
22. Put the written book back and break the lectern: the book drops on top of where it stood. Leave the world and come back with a book on a lectern: the book and its page are still there.

**Weather (group 10)**

23. `/weather rain`: rain streaks fall around you up to about 10 blocks, fading out with distance; under a roof none fall inside; at night they are darker, near a torch brighter. **Screenshot 10.**
24. `/locate biome minecraft:snowy_plains`, `/tp` there: slow snowflakes drift down instead of rain, with no splashes. **Screenshot 11.**
25. In the Nether and the End nothing falls.

## 6. Risks, shared files and decisions

**Risks**

- **Frost slowdown:** it goes through a new per-client sim value fed into `weapon_move_scale`. Check that nothing else sets that field and that the MW2 walk honours it everywhere (walk, climb, stuck).
- **Powder snow fall catch:** it uses the pmove's fall speed. A player who jumps up first and then falls 2.5 blocks reaches the same speed, so it matches, but a fall slowed by water or webs just before won't be caught.
- **Hand view in third person:** the view model currently builds only in first person with an item. Drawing the activation in third person must not draw the held item.
- **Lectern viewers:** these are server state keyed by the server player id. Until E, there is one id.
- **Book wrapping:** it must use the gui font widths. A mismatch shows as different line breaks from vanilla; compare one page with a vanilla screenshot if the owner has one.
- **Weather textures:** check the pack's `rain.png` size (vanilla 64 x 256). The v scaling of 0.25 a block repeats the texture every 4 blocks of column.
- **Vendored files:** `world/src/level/mod.rs`, `redstone.rs`, `drops.rs` and `entities/src/inside_blocks.rs` are patched, and `level/lectern.rs` is added. Update the patched and added counts in CLAUDE.md's vendored-code note at the end of the track.

**Shared files with other round 8 tracks**

- **A (fixes):** `world/src/level/mod.rs` (a1, a4, a5), `minecraft_gui.rs` allow-lists, `docs/MINECRAFT-SURVIVAL.md`, `render_gpu/.../minecraft_world.rs` shader (a1 gateway).
- **B:** `pm_flags` (B3's `MC_RIDING 0x0100_0000`: don't reuse it), `player_nbt.rs` (B3, B4), `minecraft_ui.rs` and `minecraft_gui.rs` (B4 adds `widget/button*` and a button helper; group 9 reuses it if B4 lands first, else adds the sprites itself and B4 drops its copy), `args.rs` (B4, A5).
- **C:**
  - C1 adds `EffectKind`s and edits `survival.rs` lists: group 3 touches `Survival` fields and `hazards`, not those lists;
  - C2 touches `redstone.rs` near the comparator input (`:478`): groups 6 and 8 touch `own_signal`, `state_direct_signal`, `analog_output` and `tick_block`;
  - C3 group 1 changes `render_gpu/.../minecraft_world.rs` (sky, celestial atlas, `end_portal` uniform), `render/src/extract.rs` and adds `SkyState.end_flash`: group 10 adds a pipeline and frame fields to the same files, so run it after c3; group 4's `in_powder_snow` (before c3 in the track) gets c3's field when c3 lands;
  - C1 groups 1-3 change the same movement, sim and survival files as groups 1-3 here (`single.rs`, `stuck.rs`, `climb.rs`, `sim/src/frame.rs`, `carrier.rs`, `playerstate.rs`, `survival.rs`, `death.rs`, `player_nbt.rs`, the fall block of `minecraft_world.rs`); they run just before groups 1-4 here. C1 adds the `mc_float` field and no `pm_flags` bit, so `MC_POWDER_WALKER` is free to take;
  - C4 reads `pm_flags` but adds no bit.
  - All of c1, c3 and c5 add `ambient_particles` kinds: add each at the end of the enum.
  - Docs: earlier docs commits in the track move the `MINECRAFT-SURVIVAL.md` lines, so find them by text.
- **D (skating):** `sim/src/voxel.rs` shape flags. D takes `0x7800` for rails, this brief takes `0x8000` for powder snow, and both extend the `shape_ids` key. Also `movement_iw4/src/single.rs`.
- **E (multiplayer):** every **[E]** mark above:
  - hazards and freeze ticked in `render_anim` for the local player;
  - the powder walker bit and move scale per player's ps;
  - totem pop and snowflake broadcasts;
  - lectern `player` ids;
  - trapped chest openers are already per player key.

**Decisions (vanilla-like option picked)**

- Books for testing come through `/give` components (and kit lines that can now carry them), not a book-editing screen. Writing books stays a limitation.
- Lectern state lives on the server thread's level (the block entity it already saves), not in the main-thread containers. Comparators, pulses and drops then need no second copy.
- The trapped chest count goes to the server per chest position on every opener change. It is not saved, as in vanilla.
- The totem activation is client-local (vanilla `LocalPlayer`); the glitter is level-wide and tracks the body.
- Weather streaks are built on the CPU each frame (at most 441 quads), as vanilla does, with no instancing.
- Snowflakes and the burning-melt follow vanilla; the melt goes through the same block-write path as Frost Walker.
