# C2: End cities and item frames (design brief)

Track C (End and visual leftovers), item c2. Paths and lines are as of e2313625. Vanilla facts were checked with javap on the obfuscation-free 26.3 `client.jar` and with the 26.3 data pack (`iw4l-artifacts/minecraft-26.3/datapacks/local/minecraft-26.3/data/minecraft/`). No Mojang code is copied; this brief gives behaviour and numbers only.

## 1. The owner's request

"End cities with item frames (and the elytra in its frame on end ships)": the end city pieces built from the 26.3 structure templates, placed on the outer islands, with their `end_city_treasure` chests, the shulkers from the "Sentry" markers, the ship's item frame holding an elytra, and the dragon head at the bow. Item frames in general: the entity, hanging on block faces, turning the item, knocking the item out, glow item frames, drawing the item in the frame, and saving.

Scope boundaries:
- **Getting to the outer islands** is track A1 (gateway travel). This brief only needs `/tp` and `/locate` for its tests.
- **Shulker behaviour** (peeking, bullets, teleporting, levitation, drawing) is track C's shulker item. This brief only makes sure the Sentry shulkers are placed and saved as vanilla places them.

## 2. What exists today

| Area | State | Where |
|---|---|---|
| End city generation | Ported but never checked in game or in a test: start rule (lowest corner of a rotated 5x5 at chunk offset 7 must be at Y 60 or more), house towers, towers, bridges, fat towers, one ship per city. | `third_party/minecraftoss/generator/src/structure/kinds/end_city.rs:22-47`, `:94-265` |
| Data markers | "Chest" sets `chests/end_city_treasure` with a seed on the container below; "Sentry" adds a shulker; "Elytra" adds an `item_frame` with an elytra. The vanilla `Level.isInSpawnableBounds` check on Sentry and Elytra markers is missing. | `end_city.rs:288-312`, `:317-335` |
| Templates | Read at run time from `data/minecraft/structure/<name>.nbt` (the 26.3 pack has all 20 `end_city/*.nbt`). Data markers and jigsaws are handled by the template piece. | `generator/src/feature/template/mod.rs:584-598`, `structure/kinds/template_piece.rs:41-69`, registration `structure/kinds/mod.rs:38` |
| Placement | `random_spread` with `triangular` spread is supported; `end_cities` is spacing 20, separation 11, salt 10387313, biomes `#has_structure/end_city` (end_highlands, end_midlands). | `generator/src/structure/placement.rs:32-39`, `:148` |
| Outer End terrain | `end_outer_islands` density, the End biome source (the_end within 64 chunks, then highlands, midlands, small islands, barrens by erosion). Only the main island was ever generated in a test. | `generator/src/density/ir.rs:308`, `generator/src/terrain.rs:256`; test `minecraft_terrain/src/terrain.rs:2356` |
| `/locate` | `nearest_structure` works per level, so `/locate structure minecraft:end_city` works in the End. | `minecraft_terrain/src/locate.rs:140` |
| Structure chest loot | Rolled when the chest is first opened, on the client side of the adapter. | `minecraft_terrain/src/containers.rs:667-709`, caller `render_anim/src/minecraft_world.rs:1960-1966` |
| **Bug: enchanted loot aborts the roll** | The core loot engine knows only `set_count`, `explosion_decay`, `apply_bonus`, `limit_count`, `copy_components`, `copy_state`. Any other function returns an error, and the error aborts the whole table (`emit` uses `?`), so `structure_loot` returns `None` and the chest stays empty. `end_city_treasure` puts `enchant_with_levels` on 15 of the 25 entries of its first pool (weight 45 of 89; 26.3 writes the function under the entry's `modifier` key) and rolls 2-6 times, so about nine end city chests in ten come out empty. The dungeon test passes only because its seed misses the enchanted book. | `third_party/minecraftoss/core/src/loot.rs:206-212`, `:580`, `:659`; test `containers.rs:1723-1746` |
| Enchanting engine | `EnchantmentHelper.selectEnchantment` is already ported (`Enchantments::select`) and used by trades; `enchant_randomly` and `enchant_with_levels` exist there for trades only. One `Enchantments` per world sits in `GameData`. | `third_party/minecraftoss/entities/src/enchant.rs:215`, `entities/src/trading.rs:470-500`, `minecraft_survival/src/data.rs:15`, `:65` |
| Item frames | Missing everywhere. Generated frames (the elytra) go dormant: saved unchanged, never drawn, never hit. Nothing places, draws or uses a frame. | dormant path `minecraft_terrain/src/server.rs:529-549` |
| Non-mob entity pattern | End crystals are the model to copy: a list in `ServerSim` loaded from tags, saved per chunk, placed through `ItemOnBlock`, hit through `EndTarget`, sent in `EntitySnapshot`, drawn by the client copy. | `server.rs:31-39`, `:83`, `:540`, `:560-565`, `:1438-1468`, `:1848-1890`, `:209-224`, `:2837`; `end_crystal.rs:140-185`, `:206-304`; `minecraft_survival/src/entities.rs:97`, `:554-564`, `:908`, `:1147-1160`, `:1232` |
| Hit keys | MW2 hit boxes are keyed `kind << 56 | id`; 17 fireball, 19 crystal, 20 dragon. | `minecraft_survival/src/entities.rs:1399-1430` |
| Left click | `Entities::punch` tries End targets, fireballs and mobs at `game_mode.entity_reach()` (3.0, creative 5.0). It never compares with the block under the crosshair. | `entities.rs:513-545`; `render_anim/src/minecraft_world.rs:1056-1083`; `minecraft_terrain/src/game_mode.rs:255-256` |
| Right click | No right-click on any entity exists (no mob interaction is wired). Item uses on blocks run as a chain of `*_use` functions, and server-decided ones go through `Change::Server(pos, ItemOnBlock)` with the client waiting on `item_uses`. | `render_anim/src/minecraft_world.rs:1197-1214`, `:1316-1331`; `minecraft_survival/src/entities.rs:582-585`; `server.rs:1429-1446`, `:2394`, `:2701` |
| Fixed-display items | Campfire food is drawn as `HeldItem { display: HeldDisplay::Fixed }`; every item visual already resolves its `fixed` transform. | `minecraft_terrain/src/campfire.rs:212-228`; `mesh.rs:903-906`, `:1154-1172`, `:1258-1277` |
| Block models under a pose | `append_moving_blocks` and `append_open_lids` resolve block-state variants and mesh them. All `textures/block/` sprites are preloaded, so `block/item_frame` and `block/glow_item_frame` are in the atlas. | `mesh.rs:1094-1150`, `:650-660` |
| Server sounds | Positional sounds go out as `MobSound { event, position, volume, pitch, category }`. | `entities/src/world.rs:623-629`; `server.rs:90`, `:1569` |
| Explosions | The level reports each blast as `ExplosionEffect { center, radius, blocks, interacts }`. | `world/src/level/explosion.rs:36-41` |
| Block flags | `flags::LEGACY_SOLID` (vanilla `BlockState.isSolid`); block collision shapes per state. | `core/src/block.rs:36`; `world/src/level/physics.rs:325` |
| Dragon head | `dragon_head`/`dragon_wall_head` already draw (static jaw). | `minecraft_terrain/src/special_models.rs:132-133` |
| Known gaps that show in cities | Banner patterns are not drawn (tower tops get plain magenta banners); end rods have no particles. | `docs/MINECRAFT-SURVIVAL.md:295`, `:297` |

What the templates hold (read from the 26.3 NBTs):
- `ship` (13x24x29): two chests at (5,4,7) and (7,4,7) with "Chest" markers above them. The "Elytra" marker is at (6,5,7) between them. "Sentry" markers are at (6,4,8), (8,6,27) and (4,11,27). There is a `dragon_wall_head` (facing north) at (6,8,0) and a brewing stand at (6,7,25).
- Sentry markers in `base_floor` (2), `second_floor_2` (1), `third_floor_2` (2), `fat_tower_middle` (4) and `tower_top` (1).
- Chests: `fat_tower_top` (2) and `third_floor_2` (1, beside an unmarked ender chest).
- `tower_top`: eight magenta wall banners with black `triangle_top` and `triangle_bottom` patterns.
- No template carries an item frame entity. The elytra frame is the only frame worldgen makes.

## 3. Vanilla 26.3 behaviour (javap)

### End city
- **`EndCityStructure.findGenerationPoint`:**
  1. The point is block (7, 7) of the chunk. Generation fails unless `couldValidBiomeExistInTerrainColumn` holds there.
  2. A random `Rotation` is drawn.
  3. `getLowestYIn5by5Box` takes the lowest of the four corner heights: offsets (5, 5) for none, (-5, 5) for clockwise 90, (-5, -5) for 180, and (5, -5) for counter-clockwise 90.
  4. If that height is below 60, there is no city. The port matches this.
- **`EndCityPieces$EndCityPiece.handleDataMarker`:**
  - **"Chest…"**: when `pos.below()` is inside the box, `RandomizableContainer.setBlockEntityLootTable(level, random, below, END_CITY_TREASURE)` sets the table with `random.nextLong()`, only on a randomizable container.
  - **Other markers** need the marker inside the box and `Level.isInSpawnableBounds(pos)`:
    - **"Sentry…"**: a `Shulker` is created with `EntitySpawnReason.STRUCTURE`, `setPos(x + 0.5, y, z + 0.5)`, then `addFreshEntity`. There is no finalize call.
    - **"Elytra…"**: `new ItemFrame(level, pos, rotation.rotate(SOUTH))`, then `setItem(new ItemStack(ELYTRA), false)` (no sound, no neighbour update), then `addFreshEntity`.
- **Piece settings**: `OW` (overwrite) picks the `STRUCTURE_BLOCK` ignore processor, and entities in templates are ignored. The port matches.
- **`end_city_treasure`** (data pack):
  - **Pool 1** rolls `uniform(2, 6)` times:
    - diamond 2-7 (weight 5), iron ingot 4-8 (10), gold ingot 2-7 (15), emerald 2-6 (2), beetroot seeds 1-10 (5);
    - saddle (3), and copper, iron, golden and diamond horse armour (1 each);
    - weight 3 each, all with `enchant_with_levels` levels `uniform(20, 39)` and options `#minecraft:on_random_loot`: diamond sword, spear, boots, chestplate, leggings, helmet, pickaxe and shovel, and iron sword, boots, chestplate, leggings, helmet, pickaxe and shovel.
  - **Pool 2** rolls once: empty (14) or `spire_armor_trim_smithing_template` (1).
- **`EnchantWithLevelsFunction.run`**: `EnchantmentHelper.enchantItem(random, stack, levels.getInt(ctx), registryAccess, options)`.
  - It runs `selectEnchantment(random, stack, level, options)`, and a book becomes an `enchanted_book`. Each result goes on with `ItemStack.enchant`.
  - `include_additional_cost_component` matters only for trades.
- **`EnchantRandomlyFunction`** (used by 50 other chest entries and by piglin bartering): one random enchantment from the options that `canEnchant` the item (any for a book, unless `only_compatible` is false), at a level from 1 to max, and a book becomes `enchanted_book`. The trade port at `trading.rs:470-485` has the same behaviour.

### Item frames (`ItemFrame`, `GlowItemFrame`, `HangingEntity`, `BlockAttachedEntity`, `ItemFrameItem`, `HangingEntityItem`, `ItemFrameRenderer`)
- **Type** (`EntityTypes`): `item_frame` and `glow_item_frame`, category MISC, no loot table, nominal size 0.5x0.5, eye height 0, client tracking range 10 chunks, no update interval.
- **Constants**: `NUM_ROTATIONS` 8, `DEPTH` 0.0625, `WIDTH` and `HEIGHT` 0.75, default rotation 0, `dropChance` 1.0, invisible false, fixed false.
- **Position**:
  - `pos` is the block the frame occupies, the air cell in front of the face it hangs on. It hangs on `pos.relative(direction.opposite())`.
  - `createBoundingBox(pos, dir, map)`: the centre is `atCenterOf(pos)` moved 0.46875 against the facing. The box is 0.0625 on the facing's axis and 0.75 (1.0 with a map) on the other two. The entity position is the box centre.
  - `setDirection`:
    - horizontal: xRot 0 and yRot `2D value × 90` (S 0, W 1, N 2, E 3);
    - vertical: xRot `-90 × step` (up -90, down 90) and yRot 0.
  - `BlockAttachedEntity.setPos` snaps `pos` to the block containing the position.
- **Saving** (vanilla keys):
  - entity keys plus `block_pos` (int array), `Facing` (byte, 3D data value D0 U1 N2 S3 W4 E5), `Item` (item stack codec, only when not empty), `ItemRotation` (byte), `ItemDropChance` (float), `Invisible` and `Fixed`.
  - Defaults on load: `Facing` DOWN, `ItemRotation` 0, `ItemDropChance` 1.0, `Invisible` and `Fixed` false.
  - A `block_pos` more than 16 blocks from the entity position is logged and ignored.
- **Survival check** (`BlockAttachedEntity.tick`, server only):
  - `checkBelowWorld` runs every tick.
  - A counter is post-incremented and compared with 100, so the check runs every 101 ticks. The counter is not saved.
  - It runs `survives()`. A frame that fails is discarded and runs `dropItem(level, null)`: the break sound, then both drops.
- **`ItemFrame.survives`**: a fixed frame always survives. Otherwise all of these must hold:
  - the 0.75 box (`getPopBox`) touches no block collision and is inside the world border;
  - the support block `pos.relative(dir.opposite())` is `isSolid()`, or a diode (repeater or comparator) when the facing is horizontal;
  - `canCoexist(true)`: no other `HangingEntity` with the same facing has a box that intersects this pop box.
- **Placing** (`HangingEntityItem.useOn` with `ItemFrameItem.mayPlace`):
  - The frame goes at `clicked.relative(face)` facing `face`, on any of the six faces.
  - The position must be inside the build height, and `player.mayUseItemAt(pos, face, stack)` must hold (no adventure without `can_place_on`).
  - Custom data from the stack is applied (`createDefaultStackConfig`).
  - If `survives()`: the place sound, the `ENTITY_PLACE` game event, `addFreshEntity`, and `shrink(1)` (creative keeps the stack through the usual use-on restore). Otherwise CONSUME with nothing placed.
- **Use** (`interact`, right click):
  - A fixed frame passes.
  - On the server:
    - **empty frame, holding something:** `setItem(copyWithCount(1))`, which plays the add-item sound and updates the neighbour output, then `consume(1, player)` (creative keeps it), then SUCCESS;
    - **empty frame, empty hand:** PASS;
    - **framed item:** the rotate-item sound, rotation +1 modulo 8, SUCCESS.
  - Off hand: vanilla tries the main hand, then the off hand.
- **Hurt** (`hurtServer`):
  - **Fixed:** only a damage type in `#bypasses_invulnerability` or a creative player gets through, and then it behaves as a `HangingEntity` hit.
  - **Has an item and the damage is not an explosion:** `dropItem(level, attacker, false)` takes only the item out, with the remove-item sound. The frame stays.
  - **Otherwise** (`BlockAttachedEntity.hurtServer`):
    - no effect if a `Mob` is the attacker and `mob_griefing` is off;
    - else `kill`, then `dropItem(level, attacker)`: the break sound, then the frame item and the item.
  - A player's attack never runs the normal attack: `skipAttackInteraction` sends a 0-damage `playerAttack` through `hurtOrSimulate`, so there is no cooldown use, sweep or knockback.
- **`dropItem(level, entity, withFrame)`**:
  1. A fixed frame drops nothing.
  2. The item is cleared.
  3. If `entity_drops` is off, or the cause is a player with infinite materials, nothing drops.
  4. Otherwise the frame item drops when `withFrame` is set: `item_frame` or `glow_item_frame`, keeping the frame's custom name.
  5. Then the item drops if `random.nextFloat() < dropChance`.
  - **Where the drops spawn** (`HangingEntity.spawnAtLocation`): at x + stepX × 0.15, y, z + stepZ × 0.15. Each is a normal new item entity with the default pickup delay of 10 ticks.
- **Explosions**:
  - `ServerExplosion.hurtEntities` hurts every entity within `2 × radius` with damage of at least 1, even at 0 exposure, so any frame in reach breaks with both drops.
  - `BlockAttachedEntity.ignoreExplosion` skips the frame when the explosion's direct source is in water, or when `shouldAffectBlocklikeEntities` is false. That is the case for wind charges, and with `mob_griefing` off for block interactions that keep blocks (`ExplosionEffect.interacts`).
  - `move` or `push` with any non-zero motion kills the frame with drops (pistons; nothing in the fork pushes frames yet).
- **Comparator** (`ComparatorBlock.getInputSignal`):
  - The input normally comes from the block behind.
  - When that is below 15 and the block behind is a redstone conductor, it looks two blocks behind. If exactly one `ItemFrame` there has the comparator's facing, its `getAnalogOutput` is used: `rotation % 8 + 1` with an item, or 0 when empty. The larger of that and the block's analog output wins.
  - `setItem` and `setRotation` call `updateNeighbourForOutputSignal(pos)`.
- **Sounds** (`SoundEvents`; all at volume 1 and pitch 1 through `Entity.playSound`, source NEUTRAL):
  - `entity.item_frame.place`, `.add_item`, `.rotate_item`, `.remove_item` and `.break`;
  - the `entity.glow_item_frame.*` set of the same five for glow frames.
- **Drawing** (`ItemFrameRenderer.submit`), in the entity's frame:
  1. Undo the render offset (stepX × 0.3, -0.25, stepZ × 0.3).
  2. Translate by step × 0.46875, back to the block centre.
  3. Rotate X by 0 for a horizontal facing or `-90 × axisStep` for a vertical one.
  4. Rotate Y by `180 - dir.toYRot()` (toYRot: S 0, W 90, N 180, E 270) for a horizontal facing, or 180 for a vertical one.
  5. **Frame model:** unless invisible, the block model `item_frame[map=false]` (or `glow_item_frame`, or `*_map` with a map) at `translate(-0.5, -0.5, -0.5)`, drawn with a small z offset.
     - The models are `block/template_item_frame`: a 10x10 back of `#back` at z 15.5-16 and a 12x12 rim of 1x1 `#wood` sticks at z 15-16.
     - The textures are birch planks plus `block/item_frame` or `block/glow_item_frame`.
  6. **Item:** translate (0, 0, 0.4375) (0.5 when invisible), rotate Z by `rotation × 45°`, scale 0.5. It is drawn with `ItemDisplayContext.FIXED` (generated items rotate 180 about Y; block items scale 0.5 and the frame halves that).
  7. **Light:**
     - the frame uses the light at the entity's block, with a glow frame's block light raised to at least 5 (`GLOW_FRAME_BRIGHTNESS`);
     - a glow frame's item is full bright (`0xF000F0`), a normal frame's item uses the entity's light.
  8. The name tag shows only for a crosshair-picked frame whose item has a custom name.
- **Pick** (`Minecraft.pick`): the crosshair picks the nearest entity box within the entity interaction range (`entity_interaction_range` 3.0) only if it is closer than the block hit. Frames have pick radius 0.
- **Glow item frame recipe**: shapeless `item_frame` + `glow_ink_sac` (data pack). `item_frame` is shaped from sticks and leather. Both come from the data pack already.

## 4. The plan (commit groups, in order)

All new files are rustfmt-clean and have no comments. `minecraft_terrain` and the vendored `entities` and `player` crates are edition 2021 (no let-chains); vendored `core`, `generator` and `world` are 2024. Every MinecraftOSS patch is listed in the track log.

### Group A: End cities that generate and fill their chests

**1. "Minecraft End cities generate on the outer islands"**
- `minecraft_terrain/src/terrain.rs` (tests): a headless End generation test next to `end_chunks_generate_the_main_island_around_the_origin`.
  - Use seed 77 (or the first seed with a ship within the search).
  - `locate::nearest_structure` finds the nearest `minecraft:end_city` from (1000, 0, 0).
  - Generate the chunks around it with `load_now`. Grow the area until no new city blocks appear, about 7x7; cities reach about 50 blocks from the start.
  - Assert:
    - the start column is above Y 60;
    - there is purpur, end stone bricks, end rods and magenta stained glass;
    - every "Chest" container has `LootTable` = `minecraft:chests/end_city_treasure` and a non-zero `LootTableSeed`;
    - each Sentry marker gives a `minecraft:shulker` entity tag at a block centre with feet on a whole Y;
    - no `minecraft:structure_block` is left in the chunks.
  - Then walk more cities in order of distance until one has a ship, and assert:
    - one `minecraft:item_frame` tag with `Item.id` = `minecraft:elytra` and `count` 1;
    - a `Facing` that matches the city rotation (`rotation.rotate(SOUTH)`, 3D value);
    - `block_pos` equal to the marker, and `Pos` at the marker centre moved 0.46875 against the facing;
    - a `dragon_wall_head` block with a `minecraft:skull` block entity, and two `end_city_treasure` chests one block either side of the frame.
  - A second test: chunks at x 1100-1300 of the same seed hold `end_highlands` or `end_midlands` columns and end stone (the outer islands generate at all).
- Vendored `generator/src/structure/kinds/end_city.rs:300`: add vanilla's `isInSpawnableBounds` check (Y within ±20,000,000 and x/z within ±30,000,000) to the Sentry and Elytra branches. Fix anything the test finds, and log each fix as a MinecraftOSS patch.
- Unit test (generator): `end_city_markers_follow_the_piece_rotation`. For all four rotations, the elytra frame's `Facing`, yaw (`2D × 90`) and position match `ItemFrame(level, pos, rotation.rotate(SOUTH))`.

**2. "Minecraft structure chests roll enchanted gear and books"**
- Vendored `core/src/loot.rs`:
  - A trait `Enchanting` with `enchant_with_levels(random, item, cost, options) -> Vec<(String, i32)>` and `enchant_randomly(random, item, options, only_compatible) -> Option<(String, i32)>`.
  - `LootParams` gets `enchanting: Option<Arc<dyn Enchanting + Send + Sync>>`. It is `None` by default, so every existing caller compiles unchanged.
  - `apply_function` gets the `enchant_with_levels` and `enchant_randomly` arms. Both turn `minecraft:book` into `minecraft:enchanted_book` and write `minecraft:enchantments` (or `minecraft:stored_enchantments` on the book) in the fork's component JSON.
  - Without `enchanting`, both arms leave the stack unchanged instead of failing, so one missing engine never empties a whole chest.
- Vendored `entities/src/enchant.rs`: `impl Enchanting for Enchantments`, reusing `select` and the `trading.rs:470-500` logic. Adapt the core `RandomSource` to `LootRandom`. Move the shared part out of `trading.rs` so the trade code and the loot code share it.
- `minecraft_terrain/src/containers.rs:667`: `structure_loot` takes `enchanting: Option<Arc<Enchantments>>`. `render_anim/src/minecraft_world.rs:1964` passes `globals.data.enchantments.clone()`.
- Tests:
  - `containers`: `an_end_city_chest_always_rolls_items` (seeds 1-200 all give a non-empty chest, and at least one has an enchanted diamond or iron piece whose levels come from cost 20-39);
  - `a_dungeon_book_becomes_an_enchanted_book`, with a seed that picks the book;
  - `the_enchanting_functions_pass_through_without_an_engine`.
  - Core loot: `unknown_functions_still_fail` keeps the error for the other unported functions.

### Group B: item frames on the server

**3. "Minecraft item frames load, save and survive like vanilla"**
- New `minecraft_terrain/src/item_frame.rs` (pure, no ServerSim):
  - Constants from §3 and `Facing` (the six directions with their 2D and 3D values and steps).
  - `ItemFrame { id, uuid, glow, pos, facing, item: Option<ItemStack>, rotation, drop_chance, invisible, fixed, check_ticks, tag }`, where `tag` keeps the unknown keys of the loaded tag, as `mob_tags` does.
  - `from_tag` and `to_tag` with the vanilla keys and defaults of §3.
  - `bounding_box()`, `pop_box()`, `position()` and `yaw_pitch()`.
  - `survives(&impl FrameWorld)`, where `FrameWorld` answers `legacy_solid(pos)`, `is_diode(pos)`, `collides(aabb)` and `inside_border(aabb)`.
  - `Frames { list, next_id }` with `add(tag)`, `tags_where`, `remove_where`, `views()`, `coexists(frame)`, and `tick(world) -> Vec<Popped>`. `tick` runs the 101-tick check and `checkBelowWorld` (below `min_y - 64`).
- `minecraft_terrain/src/server.rs`:
  - a `frames` field;
  - `add_saved_entity` (`:529`) hands `item_frame` and `glow_item_frame` tags to `Frames::add` before the dormant fallback;
  - `save_chunk_entities` (`:552-570`) saves and unloads them;
  - `ServerSim::tick` (`:2043`) ticks them.
  - A popped frame plays the break sound as a `MobSound` (category `neutral`) and drops its frame item and its item through `spawn_item` (`:2058`). Use the item-entity random motion and the 10-tick pickup delay. Both drops follow `entity_drops`, and the item follows `drop_chance`.
  - `EntitySnapshot` (`:209-224`) gets `frames: Vec<FrameView>` (id, glow, pos, facing, position, item id, item glint flag, rotation, invisible). It is filled at `:2837` as crystals are.
- `/summon minecraft:item_frame` and `minecraft:glow_item_frame` with NBT (e.g. `{Facing:2b,Fixed:1b,Item:{id:"minecraft:diamond",count:1}}`) go through `Frames::add` (`server.rs:749`), so the summon feedback works. Vanilla does not check `survives` on summon, and the frame pops at the next check if it has no support.
- Tests (`item_frame`):
  - `a_frame_round_trips_its_vanilla_tag`
  - `the_box_sits_against_the_support_for_every_facing`
  - `defaults_fill_missing_keys`
  - `a_far_block_pos_is_ignored`
  - `frames_check_every_101_ticks`
  - `support_needs_a_solid_block_or_a_diode_on_a_wall`
  - `a_block_inside_the_frame_pops_it`
  - `a_fixed_frame_never_pops`
- Tests (`server`):
  - `the_end_city_elytra_frame_is_live_not_dormant`: the chunk from commit 1, or a hand-made tag, gives `dormant_count()` 0 and one frame in the snapshot;
  - `a_frame_whose_wall_is_removed_drops_itself_and_its_item_within_101_ticks`, which also checks `entity_drops` off;
  - `frames_are_saved_with_their_chunk_and_come_back`.

**4. "Minecraft item frames are drawn with their item"**
- `minecraft_terrain/src/item_frame.rs`:
  - `frame_pose(view) -> Mat4`, the §3 transform chain;
  - `item_pose(view) -> Mat4`, which adds `translate(0, 0, 0.4375 or 0.5)`, `rot_z(rotation × 45°)` and `scale(0.5)`;
  - `light(view, block_light) -> (sky, block)`: the light at the frame's own block, a glow frame's block light at least 5, and a glow frame's item at full bright.
- `minecraft_terrain/src/mesh.rs`: `ItemVisuals::append_frames(mesh, frames, packs, atlas, sky_light)`.
  - It resolves the variant models of the `item_frame`/`glow_item_frame` blockstate (`map=false`) once, as `append_moving_blocks` does at `:1094`. It meshes their elements under `frame_pose`, with entity shading, at the frame's light. Invisible frames have no frame model.
  - The item goes through the `append_held_items` path with `HeldDisplay::Fixed` and a new `HeldItem.light_floor: Option<(f32, f32)>` for the glow case. Existing callers pass `None`.
  - An enchanted item pushes its vertex range to `mesh.glint`, as `held_item_mesh` does (`minecraft_survival/src/entities.rs:500-508`).
- `minecraft_survival/src/entities.rs`:
  - a `frames: Vec<FrameView>` copy taken from the snapshot (`:908`);
  - `append_frames` called after `append_open_lids` (`:1276`);
  - frames drawn while within 10 chunks of the camera (vanilla tracking range).
- Tests:
  - `frame_pose_puts_the_back_against_the_wall_for_all_six_facings`: the back face's centre lies 0.5 from the support's face centre plus `DEPTH` / 2 inward;
  - `the_item_turns_45_degrees_per_rotation_about_the_facing`;
  - `an_invisible_frame_moves_the_item_to_the_wall`;
  - `a_glow_frame_raises_its_block_light_to_five_and_lights_its_item_fully`;
  - `frames_mesh_the_pack_model_and_their_item`: with the pack under `MINECRAFTOSS_ROOT`, the frame's vertices use the `block/item_frame` and birch sprites, and an elytra adds item vertices.

**5. "Minecraft item frames hang on block faces"**
- `minecraft_terrain/src/item_frame.rs`: `frame_use(player, world, held) -> Option<ItemUse>` for `minecraft:item_frame` and `minecraft:glow_item_frame`.
  - It targets the block at block reach and returns `Change::Server(hit.pos, ItemOnBlock::HangFrame { face, glow })` with `consumed: true`.
  - It is gated, as other uses are, by `game_mode.may_build()` and the build height of `pos.relative(face)`.
- `server.rs`:
  - `ItemOnBlock::HangFrame { face: &'static str, glow: bool }` (`:31-39`);
  - `item_on_block` (`:1429`) builds the frame at `pos.relative(face)`. It places it only if `survives` and `Frames::coexists`, plays the place sound, and answers through `item_uses` (`:2701`).
  - The client keeps the item when the answer is false, through the existing `item_use_waiting` and `finish_item_use` path. Creative keeps the stack through `game_mode.consumes_items()`.
- `render_anim/src/minecraft_world.rs:1330`: one more `.or_else(|| minecraft_terrain::item_frame::frame_use(&player, &outlined, stack))`.
- Tests:
  - `item_frame` `frames_hang_on_all_six_faces_with_vanilla_rotations`;
  - `server`:
    - `a_frame_is_refused_without_support_on_a_taken_face_or_inside_a_block`
    - `a_repeater_holds_a_wall_frame_but_not_a_floor_frame`
    - `two_frames_on_one_face_never_overlap`
    - `a_frame_and_a_floor_frame_share_a_cell`
    - `the_place_sound_is_the_glow_one_for_glow_frames`

**6. "Minecraft item frames take, turn and give back items"**
- `item_frame.rs`, all pure:
  - `FrameAction`:
    - `Use { held: Option<ItemStack>, infinite }`;
    - `Attack { infinite }`;
    - `Shot`, an MW2 bullet, counted as a projectile: not an explosion, so it takes the item out first and then breaks the frame;
    - `Blast`.
  - `FrameOutcome { consumed, sounds, drops, removed }`.
  - The rules are §3's `interact`, `hurtServer` and `dropItem`, including fixed frames and creative.
- `server.rs`:
  - `Command::FrameAction { id, player, action }` and `ServerHandle::frame_action`. The `player` id goes through the command, so track E can pass real players. `PLAYER` stays the only caller for now.
  - A use that took an item answers on `item_uses`, so the client spends one item.
  - Explosions: after each level tick, every new `ExplosionEffect` with `interacts` breaks the frames whose position lies within `2 × radius` of its centre, with both drops.
- `minecraft_survival/src/entities.rs`:
  - `frame_on_ray(eye, look, reach)` returns the nearest frame by its box, which is also its pick box.
  - `punch` (`:513`) tries frames with the End targets and mobs, but takes a frame only if it is nearer than the block the player is looking at. The caller passes the block hit distance; mobs keep their current behaviour.
  - New `use_frame(id, inventory, selected, infinite)`.
  - `shot_boxes` (`:1154`) adds frame boxes under the new `FRAME_KEY = 28 << 56` (0-20 are taken, B's mobs take 21-25 and c1's shulker and bullet 26 and 27), and `shoot` and `blast` (`:751`, `:781`) route that key to `Shot` and `Blast`.
- `render_anim/src/minecraft_world.rs`:
  - In the use branch (`:1197`, before table, station and the block uses), if `frame_on_ray` at `game_mode.entity_reach()` is nearer than the block target, right-click goes to `use_frame` with the main hand, then the off hand if the main hand passed. The hand swings and the use is consumed.
  - The left-click path at `:1078` passes the block hit distance into `punch`.
- Tests (`item_frame`):
  - `an_empty_frame_takes_one_item_and_a_full_frame_turns_it`
  - `eight_turns_bring_the_item_back`
  - `a_fixed_frame_ignores_use_and_survival_hits_but_not_creative`
  - `a_hit_takes_the_item_out_and_a_second_hit_breaks_the_frame`
  - `creative_hits_drop_nothing`
  - `entity_drops_off_drops_nothing`
  - `drop_chance_zero_keeps_the_item_from_dropping`
  - `a_blast_breaks_a_full_frame_with_both_drops`
  - `drops_appear_a_step_out_from_the_wall`
  - `glow_frames_use_their_own_sounds`
- Tests (`server`): `a_bullet_pops_the_elytra_out_of_the_ship_frame`, and `tnt_beside_a_frame_breaks_it`, which also checks that `mob_griefing` off with a creeper keeps it.
- Tests (`entities`): `the_frame_key_round_trips` and `a_frame_behind_a_wall_is_not_hit`.

### Group C: the leftovers that touch frames

**7. "Minecraft comparators read item frames"** (optional: skip it and log why if the redstone hook gets large)
- `ServerSim` keeps `Level.frame_outputs: HashMap<(BlockPos, facing), i32>` up to date whenever a frame is added, removed, filled, emptied or turned, and calls the level's `update_neighbour_for_output_signal(pos)` as `setItem` and `setRotation` do.
- The vendored comparator input (`world/src/level/redstone.rs`, around `:478`) reads it two blocks behind when the block behind is a conductor and below 15. It uses the frame only when exactly one frame has the comparator's facing.
- Test: `a_comparator_reads_the_frame_rotation_through_a_block`, giving outputs 1 to 8, then 0 when empty.

**8. "Minecraft thrown items and tridents knock items out of frames"** (optional, the same rule)
- `thrown.rs` and `trident.rs` hits test frame boxes. A snowball, egg, pearl, potion or trident hit is a non-explosion hurt: it takes the item out, or breaks an empty frame.
- Test: `a_snowball_pops_a_framed_item`.

**9. "Docs: Minecraft End cities and item frames"**
- Line numbers are as of e2313625; c1's docs commit runs first in this track and moves them, so find each line by its text.
- `docs/MINECRAFT-SURVIVAL.md`:
  - End cities, chests, the ship and frames go in the End section (`:174-193`);
  - frames go in the entity list;
  - X6 (`:282`) drops "item frames don't show or drop their item";
  - add the limits: banner patterns on tower tops, end rod particles, maps in frames (no maps), frame name tags, middle-click pick, and the comparator or projectiles if commits 7 or 8 were skipped.
- `docs/NETHER-END.md:123-128`, `:134` and `:262`: the end city and the frame are no longer dormant.
- The `enchant_*` fix and the loot functions still missing (`set_damage`, `exploration_map`, `filtered`, `set_potion`, `set_name`, `set_stew_effect`, `set_ominous_bottle_amplifier`, `set_instrument`, `set_enchantments`) go into the limitations, since any of them still empties its chest.

## 5. The owner's test steps

Setup: `.env` with `IW4L_MINECRAFT_WORLD=c2test`, `IW4L_MINECRAFT_DIMENSION=the_end`, and the kit below. Start `iw4l-survival.exe map minecraft:overworld`.

```
IW4L_MINECRAFT_KIT=minecraft:item_frame*16,minecraft:glow_item_frame*16,minecraft:diamond_sword+sharpness=5,minecraft:compass,minecraft:end_stone*64,minecraft:oak_planks*64,minecraft:torch*16,minecraft:repeater*2,minecraft:comparator*2,minecraft:redstone*16,minecraft:tnt*4,minecraft:flint_and_steel,minecraft:diamond_pickaxe+efficiency=5,minecraft:firework_rocket*64,minecraft:golden_carrot*64,minecraft:water_bucket,minecraft:snowball*16
```

1. You start on the End platform. Type `/locate structure minecraft:end_city`. The chat names a spot more than 1000 blocks out. Type `/gamemode creative`, then `/tp <x> 120 <z>` with its x and z, and look down. A purple city of towers and bridges stands on an outer island. The banners on the tower tops are plain magenta (known). **Screenshot 1.**
2. Fly around the city. If no ship floats beside it (a long purple boat with a black dragon head on its bow), `/locate` again from about 300 blocks further out and go there. **Screenshot 2:** the ship with the dragon head.
3. Inside the ship's lower deck, between two chests, an item frame on the wall holds an elytra. **Screenshot 3.**
4. Type `/gamemode survival`. Right-click the frame once: the elytra turns 45 degrees with a click. Left-click it: the elytra pops out with a sound and lands in front of the frame, and the empty frame stays. Pick it up. **Screenshot 4:** the empty frame and the elytra in the hotbar.
5. Open both chests. They hold loot: diamonds, iron or gold, often an enchanted diamond or iron tool or armour piece (hover to see its enchantments), sometimes a spire armour trim. **Screenshot 5:** a chest with its tooltip.
6. Put the compass into the ship frame. Quit through the menu and start again. The compass is still in the frame, turned the way you left it, and the opened chests keep what you left in them.
7. Back on an island, place end stone walls and hang frames on all four sides of a block, on its top and under its bottom. Each frame lies flat against its face. Put the sword in one: it shows the enchantment glint. Right-click it 8 times: it turns in 45-degree steps and comes back. **Screenshot 6.**
8. Build a small roofed box of planks and hang a normal frame and a glow frame inside, each with a carrot. The glow frame's carrot stays bright in the dark, the normal one is dark. **Screenshot 7.**
9. Hang a frame on a plank, put an item in, and mine the plank. Within about 5 seconds the frame falls off and drops itself and the item, with a break sound.
10. Hang a frame on the side of a repeater: it holds. A frame on top of a repeater cannot be placed.
11. Fill a frame and shoot it once with the MW2 gun: the item pops out. Shoot again: the frame breaks and drops.
12. Hang three filled frames near a TNT block and light it. All three break and drop the frame and the item.
13. Type `/gamemode creative` and left-click a filled frame: the item vanishes and nothing drops. Hit again: the frame vanishes with no drop.
14. (Commit 7) Put a comparator facing away from a plank with a filled frame on the far side of the plank, and redstone after it. Turning the item lengthens the powered redstone by one block per turn.
15. (Commit 8) Throw a snowball at a filled frame: the item pops out.
16. Optional, with vanilla 26.3 Java: open `c2test` in vanilla. From the platform, `/locate structure minecraft:end_city` gives the same coordinates as step 1, the city looks the same, and your frames hang with the same items and turns.

Kit: 17 stacks.

## 6. Risks, shared files and decisions

**Risks**
- **Loot engine.**
  - The `enchant_*` fix is the first change to `core/src/loot.rs` in round 8. Track B's piglin bartering needs `enchant_randomly` and `set_potion` from the same file, so whichever track lands second rebases onto the `Enchanting` hook.
  - The other unported functions still empty any chest that rolls them. That includes desert pyramids and shipwrecks (`set_damage`), and ocean ruins and shipwrecks (`exploration_map`). Flag this to track A as a separate fix; it is not in this brief's scope.
- **Generation parity.** The end city port has never run. The piece order, the `gen_depth` collision rule and the ship roll (`nextInt(10 - depth)`) decide what appears. A mismatch only shows against vanilla, so test step 16 is the real check.
- **No entity interaction existed.** The right-click branch is the first one, and it sits in `render_anim/src/minecraft_world.rs`, which track E is moving out of the adapter (CRATE-SPLIT.md section 6). Keep the rules in `item_frame.rs` and `entities.rs` so the adapter glue is about 20 lines and moves with the rest.
- **Server tick boundary.** Frames live per `Level` in `ServerSim`, with no global or player-0 state. Actions carry a player id in `Command::FrameAction`. Drops are shared item entities, and the sounds are positional `MobSound`s. Only `Entities` still calls with `PLAYER`, like every other action today. Track E replaces that in one place.
- **Snapshot size.** Every snapshot carries all of the level's frames, as for crystals. Frames are few. If a big build shows a cost, send them only when a frame changes, using a version counter. Decide at review, not up front.
- **Shulkers.** c1 runs before this item in track C, so the Sentry shulkers load as live shulkers through c1's `spawn_saved` arm. This brief adds no shulker handling. If c2 is ever run without c1, the shulkers stay dormant and a city looks empty, which is still fine for its tests.

**Files shared with other round 8 tracks**
- `minecraft_terrain/src/server.rs`: A (gateways, spawners, `/tick`), B (mobs), C (shulkers), E (players).
- `minecraft_survival/src/entities.rs`: B, C and E.
- `render_anim/src/minecraft_world.rs`: everyone, and E moves the player actions out of it.
- `minecraft_terrain/src/mesh.rs`: C visual leftovers.
- `third_party/minecraftoss/core/src/loot.rs`: B.
- `entities/src/enchant.rs` and `trading.rs`: B, for bartering.
- `generator/src/structure/kinds/end_city.rs`: C, if the shulker item touches the Sentry spawn.
- `world/src/level/redstone.rs` (commit 7): A, for spawners, and c5 in this track (trapped chest and lectern signals in `own_signal`, `state_direct_signal`, `analog_output`, `tick_block`). Commit 7 only touches the comparator input near `:478`.
- `minecraft_terrain/src/server.rs` `EntitySnapshot` (`:211-224`) and its literal at `:2837`: c3 in this track adds `dragon_breath` and `dragon_explosions`. Append fields at the end of both.
- Hit keys in `minecraft_survival/src/entities.rs`: c1 takes 26 and 27, this item 28.
- Docs `MINECRAFT-SURVIVAL.md` and `NETHER-END.md`: every track.

**Decisions** (the vanilla-like option each time)
- **Entity model.** Frames are their own list in `ServerSim`, like end crystals, not mobs in the vendored `EntityWorld`. Vanilla frames are `Entity`, not `LivingEntity`.
- **Placement.** The server decides placement, using `survives` and `coexists`, and the client waits on `item_uses` as for end crystals. The client never predicts a frame.
- **Crosshair pick.** Frames use the entity reach (3.0, creative 5.0) and must be nearer than the block hit, as `Minecraft.pick` does. Mobs keep the fork's current punch rule, which is unchanged here.
- **MW2 bullets.** They count as projectile hurts: the item comes out first, then the frame breaks. MW2 grenades and all level explosions count as explosions and break the frame with both drops.
- **Survival check.** It runs every 101 ticks, the exact post-increment rule, not "every 5 seconds".
- **Glow light.** A glow frame's block light is at least 5, and its item is full bright. A normal frame uses the light of its own cell, not the support's.
- **Not ported:**
  - maps in frames (the fork has no maps);
  - frame name tags;
  - creative middle-click pick on frames;
  - frames pushed by pistons (nothing pushes entities);
  - the `isInSpawnableBounds` corner cases beyond the plain range check.
