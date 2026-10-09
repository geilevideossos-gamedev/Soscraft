# B2: shulker box lid animation

Owner, screenshot 6 (`6_shulker_ohne_animation.png`): "Shulker boxes have no open and close animation; the lid slides up and turns like vanilla. Chests and ender chests are fine."

## 1. What exists today

- **There is no chest lid mechanism to reuse.** No block entity animates (`docs/MINECRAFT-SURVIVAL.md:187`). `closed_chest_model` (`model.rs:470-472`) is a "closed pose until opening animation is represented in the world pass". We take the owner's "fine" to mean no complaint. The mechanism below is shared, so chests can follow (commit 5).
- **How shulker boxes are drawn.**
  - `block_model` (`special_models.rs:282-293`) poses the box like `ShulkerBoxRenderer.createModelTransform`.
  - `push_shulker` (`:784-790`) emits the lid cube (uv 0,0, origin -8,-16,-8, size 16x12x16) and the base cube (uv 0,28) under one pose, `part([0,24,0])`. The item model (`:360-367`) shares it.
  - The model is baked into sections: `resolve_block_variants` (`model.rs:416`), then `ModelCache` (`terrain.rs:584`), then `build_section_blocks` (`terrain.rs:701`).
- **Opening and closing.**
  - `Station::of` (`containers.rs:58`) recognises the box, and `lid_obstructed` (`containers.rs:134`) is checked at `minecraft_world.rs:1077-1082`.
  - The transition loop at `minecraft_world.rs:1769-1792` plays `block.shulker_box.open`/`close` at volume 0.5 and pitch 0.9-1.0, as vanilla does.
- **Precedent for drawing a block every frame.**
  - `FallingBlocks` (`falling_render.rs`) keeps a landed block drawn until `cell_settled` (`terrain.rs:1380`) reports its section rebuilt.
  - `ItemVisuals::append_moving_blocks` (`mesh.rs:1065`) runs `append_block` (`mesh.rs:2206`) through the one-block `MovingBlockScene` (`mesh.rs:809`).
  - It is called from `Entities::meshes` (`minecraft_entities.rs:901-903`) with `partial`. The client 20 Hz tick is `Entities::tick` (`:597-610`).
  - `TerrainStream::mark_edited` (`terrain.rs:1323`) also relights. Compile jobs capture a `Neighborhood` at `terrain.rs:1488-1505`.
- **The barrel works.** The loop at `minecraft_world.rs:1788-1791` writes `open=true/false` through `write_blocks` (`:2237`), and `barrel.json` maps that to `barrel_open`. It differs from vanilla in small ways:
  - The sound plays at the block centre.
  - There is no openers counter.
  - Each toggle relights and sends `PlayerEdit::Place`.
  - Closing the game window while the barrel screen is open saves `open=true`. Escape can't cause this, because it closes the inventory first (`hud/src/minecraft_inventory.rs:217`).

## 2. Vanilla 26.3, checked with javap

- **`ShulkerBoxBlockEntity`**
  - Constants: `OPENING_TICK_LENGTH=10`, `MAX_LID_HEIGHT=0.5`, `MAX_LID_ROTATION=270`.
  - `AnimationStatus`: CLOSED, OPENING, OPENED, CLOSING.
  - `updateAnimation` sets `progressOld=progress`, then:
    - OPENING adds 0.1; at 1 or more the status is OPENED with progress 1.
    - CLOSING subtracts 0.1; at 0 or less the status is CLOSED with progress 0.
    - While opening it calls `moveCollidedEntities`.
  - It ticks on the client and the server.
  - `triggerEvent(1, count)`: a count of 0 means CLOSING, 1 means OPENING, and other counts change nothing.
  - `startOpen`/`stopOpen` skip spectators. They send `blockEvent(pos, block, 1, openCount)`, and the sound plays only on 0→1 and 1→0.
  - `getProgress(partial)` is `lerp(partial, progressOld, progress)`.
- **`ShulkerBoxRenderer$ShulkerBoxModel.setupAnim(p)`:** `lid.setPos(0, 24 - p*0.5*16, 0)` and `lid.yRot = 270°*p`. The base never moves. `ShulkerModel.createShellMesh` gives both parts the offset (0,24,0).
- **`ShulkerBoxBlock`**
  - `canOpen` checks the lid space only while CLOSED.
  - `getShape` is the full block grown 0.5·p toward `facing` (`Shulker.getProgressAabb`).
  - `moveCollidedEntities` moves entities in the delta box with `MoverType.SHULKER_BOX`.
  - Its properties include `noOcclusion`.
- **`ChestLidController`:** openness moves ±0.1 per tick, clamped. `ChestRenderer` uses `1-(1-o)^3`, and `ChestModel.setupAnim` sets the lid and lock `xRot = -o·π/2` about the pivot (0,9,1).
- **`BarrelBlockEntity`:** `ContainerOpenersCounter` calls `onOpen`/`onClose`. Its sound plays at `pos+0.5+facing/2` (the front face), and `recheckOpeners` runs every 5 ticks.

## 3. Commits

1. **"Minecraft shulker box model takes a lid opening progress"**
   - `push_shulker(elements, sheet, pose, progress)` poses the lid with `part([0, 24-8p, 0], [0, (270p).to_radians(), 0])`. `part` already composes translation·Rz·Ry·Rx like `ModelPart`.
   - Extract `pub(crate) fn shulker_box_model(block, progress)` from `:282-293`. `block_model` and `item_model` pass 0.
2. **"Minecraft block entity lids tick open and shut like vanilla"**
   - New `crates/minecraft_terrain/src/lids.rs`, which must pass `rustfmt --check`.
   - `ShulkerLid { status, progress, old }` with `set_open_count` (the `triggerEvent` rules), `tick` (`updateAnimation`) and `progress(partial)`.
   - `ChestLid { open, openness, old }`.
   - `Lids`, keyed by `BlockPos`, with these methods:
     - `set_open_count(pos, kind, count)`.
     - `tick(settled) -> Vec<BlockPos>`: the boxes that just closed. A new entry does not advance until `settled(pos)`, so the static lid never shows inside a rising one.
     - `posed(scene, partial)`.
     - `settle(scene, cell_settled)`: drops entries whose block changed, and closed entries once their section has been rebuilt.
3. **"Minecraft terrain sections leave animating block entities to the frame pass"**
   - `TerrainStream::draw_apart(pos, bool)` keeps an `apart` set. It dirties only that cell's section (through `player_sections`), with no relight and no `record_edits`.
   - `CompileJob` carries the apart cells of its section.
   - `build_section_blocks` skips appending those cells, on both the baked and the check path.
   - Neighbours are unaffected, because the box is `noOcclusion`.
4. **"Minecraft shulker boxes open and close their lids like vanilla"**
   - `Entities` gets `lids: Lids`. The loop at `minecraft_world.rs:1769-1792` calls `lids.set_open_count(cell, Shulker, open as u32)` and, on open, `world.stream.draw_apart(cell, true)`.
   - `Entities::tick` returns the boxes that closed, for `draw_apart(pos, false)`.
   - `Entities::meshes` calls a new `ItemVisuals::append_open_lids` (`mesh.rs`, next to `append_moving_blocks`). It runs `append_block` on `shulker_box_model(block, progress)` at the real cell, so light and shading match the static mesh.
   - `minecraft_world.rs:1079-1082` checks the lid space only while the lid is Closed.
   - Update `MINECRAFT-SURVIVAL.md:187`.
5. **"Minecraft chests and ender chests open their lids like vanilla"** (owner decision)
   - `chest_open_model(block, openness)` rebuilds the `chest_model` cubes with `push_cube`; the test at `special_models.rs:1135` proves they map the same.
   - Lid and lock rotate `-eased·π/2` about (0,9,1).
   - The two halves of a double chest are `Opened.cells`.
6. **"Minecraft barrels sound from their opening face like vanilla":** for barrels, the sound position at `minecraft_world.rs:1783` becomes `pos+0.5+facing/2`.
7. **Deferred: "Minecraft opening shulker boxes push players and grow their collision like vanilla."**
   - This needs a per-cell voxel shape override through `sim::voxel::set_block_shape`. Shapes 1.5 high already work (`FENCE`), but whether a shape can reach below the cell (a box facing down) is unverified.
   - It also needs a player push along the delta box, and server-side lids for mobs.

## 4. Unit tests

- **`special_models.rs`, next to `upward_shulker_box_base_and_lid_fill_the_block` (`:1058`):**
  - Progress 0 is unchanged from today.
  - At progress 1, the lid of an upward box spans y 0.75..1.5 (times 0.9995), a downward box spans -0.5..0.25, and a north-facing box moves to -z.
  - At 0.5 the lid is turned 135°, and the base is fixed throughout.
- **`lids.rs`:**
  - It opens in exactly 10 ticks and closes in 10.
  - A count of 2 changes nothing.
  - `progress(0.5)` is the midpoint.
  - A box does not advance before it is settled.
  - A closed box is reported once.
  - Chest openness clamps and eases.
- **`terrain.rs`, modelled on `:1871`:** an apart shulker box has no faces at its cell, and every other cell is unchanged, on both build paths.
- **`mesh.rs`, modelled on `a_moving_block_is_the_placed_block_...` (`:3773`):** `append_open_lids` at progress 0 gives the same vertices as the section path.

## 5. In-game test steps

```
IW4L_MINECRAFT_KIT=shulker_box,red_shulker_box*4,chest*2,ender_chest,barrel*2,cobblestone*64,oak_slab*16,stone*32
```

1. Place a red box on the floor. Open it: the lid rises half a block and turns three-quarters. Close it: the lid comes back down in about half a second. Screenshot 1, taken mid-animation.
2. Do the same with a box on a wall and a box on a ceiling. Screenshots 2 and 3.
3. Open a box, press Escape, and reopen it at once. The lid should reverse smoothly with no jump. Screenshot 4.
4. A bottom slab on top of a box keeps it shut, and a top slab lets it open. Screenshot 5.
5. Open a box, close it and break it at once. There must be no floating lid. Screenshot 6.
6. Watch from 10 blocks away while a box opens. There should be no flicker and no second, closed lid. Screenshot 7.
7. Open and close a barrel. Then close the game window while a barrel is open, restart, and report whether it is still open. Screenshot 8.
8. If commit 5 ships, a chest, a double chest and an ender chest swing their lids. Screenshot 9.

## 6. Risks and overlaps

- **Crate split:**
  - The openers count and the lid status move to `crates/minecraft_survival` together with `containers.rs`, as a per-level `Openers` holding a set of player UUIDs per cell, without spectators.
  - The client `Lids` is then driven by a `minecraft_net` `BlockEvent { pos, id: 1, param: count }`.
  - The sound loop at `:1769-1792` becomes server-side.
- **Dimensions:** `Lids` and `apart` are keyed by `BlockPos`, so there should be one per level.
- **Saving (track D):** vanilla does not save lid progress. The barrel's `open=true` on a hard quit is an existing gap.
- **Merge conflicts:** other track B work touches `Entities::meshes`, `ItemVisuals` and `special_models.rs`.
- **Still static:** the crack overlay, the selection outline and the collision stay static while a box is open.
- **What the owner may notice next:** the enchanting table book does not open (`special_models.rs:303-307`), the bell does not swing, and banners do not wave.
