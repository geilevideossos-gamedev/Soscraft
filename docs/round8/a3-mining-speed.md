# Track A, item a3: vanilla mining speed (design brief)

Paths and lines are as of e2313625. Vanilla facts were checked with javap on the obfuscation-free 26.3 `client.jar` (classes named in section 3) and the jar's `data/minecraft` files. Mojang code is described, never copied. The dig state is per `Player` (UUID), the mining data is world-wide read-only `GameData`, and nothing here adds global or "player 0" state. Section 6 names the places that touch the per-player and server tick boundary.

## 1. The owner's request

> Broken (screenshot): ... 3 Efficiency V diamond pickaxe does not insta-mine netherrack

Screenshot `scrennshots/screenshots-e231362/screenshots-e231362/3_effizienz_netherrack.png` ("efficiency netherrack"): a Nether cave, survival HUD, the enchanted diamond pickaxe selected and drawn at the lower right with its glint, a netherrack wall in front. In vanilla that pickaxe breaks netherrack the moment the button goes down, and holding the button while sweeping along a wall tunnels through one block per tick.

Scope from the orchestrator: check the vanilla mining speed formula and implement all of it:
- tool speed from the tool component rules;
- Efficiency (level squared plus 1, as the `mining_efficiency` attribute);
- Haste and Conduit Power, Mining Fatigue;
- the underwater penalty (`submerged_mining_speed`, Aqua Affinity) and the off-ground penalty;
- the hardness divisor 30 or 100 and the correct-tool check;
- creative;
- instant breaks (progress >= 1 on the first tick: no crack, no 5-tick delay) against the 5-tick delay after a normal break;
- the client-side break progress, sounds and particles.

## 2. What exists today

**In short: survival hand mining ignores the held item completely.** Every survival block breaks in `30 x hardness` ticks, whatever the tool, enchantments, effects, water or footing. Netherrack (hardness 0.4) always takes 12 ticks, so the owner's pickaxe can't break it instantly. Wrong tools also still drop items, and mining never wears a tool.

| Area | Where | State |
|---|---|---|
| Survival hand mining | `render_anim/src/minecraft_world.rs:1133-1149` | Each hand tick while the button is held pushes a fake bullet, `VoxelEvent::Shot { damage: 160.0 / 30.0 }`, at the target within a fixed 4.5 blocks. There is no item, effect, water, ground or correct-tool input. |
| Bullet progress | `render_anim/src/minecraft_mining.rs:79-94`, `:326-334`; `sim/src/voxel.rs:1046` | Progress grows by `damage / 160 / hardness` per shot. With the hand's fake damage that is `1 / (30 x hardness)` per tick. Hardness is `registries.blocks.state(state).destroy_speed`; -1 means unbreakable. |
| Progress memory | `minecraft_mining.rs:22-23`, `:124-125` | Kept per block for 6 s after the last hit. Letting go and coming back continues the crack; vanilla starts again from zero. |
| Crack | `minecraft_mining.rs:220-247`, `minecraft_world.rs:2212` | Stage `(progress x 10).clamp(0, 9)` on the block's real model. It only comes from the bullet progress map. |
| Hit sound | `minecraft_world.rs:1560-1572` | Each `Shot` plays the hit sound at `(volume + 1) / 4`, pitch x 0.5, so hand mining clicks every tick. Vanilla plays it on the start tick and then every 4 ticks, at `(volume + 1) / 8`. |
| Hit particles | none | Vanilla adds one small terrain particle on the hit face each tick. The fork has only the break burst (`minecraft_mining.rs:84-91`, `BlockParticles::spawn` at `minecraft_terrain/src/block_particles.rs:53-90`). |
| Break sound and burst | `minecraft_world.rs:1597-1608` | Correct (vanilla level event 2001: `(volume + 1) / 2`, pitch x 0.8). It stays. |
| Dragon egg | `minecraft_world.rs:1537-1547` | A `Shot` on the egg calls `entities.attack_block` (teleport) instead of mining. The new dig loop must keep this as its "attack on start". |
| Creative breaking | `minecraft_world.rs:1118-1132`, `minecraft_terrain/src/game_mode.rs:253-287` | Correct: a click breaks at once, then every 6 ticks while held (`CREATIVE_BREAK_DELAY = 5`). Swords, tridents and maces break nothing. It stays as is. |
| Click and miss | `minecraft_world.rs:1050-1083` | Correct: the attack click, `miss_time` 10 and `busy` (eating, blocking, charging, MW2 offhand). |
| Hand clock | `minecraft_world.rs:923-934` | `hand_ticks` per frame at 20 Hz. `attack_ticks` resets when the held item id changes. |
| Per-player hand state | `minecraft_survival/src/player.rs:48-63` | `Hand` holds swing, cooldowns, `miss_time`, `creative_delay` and so on. It has no dig state. |
| Effects | `third_party/minecraftoss/player/src/survival.rs:36-49`, `:168`, `:187`, `:220-228`, `:357-370` | `EffectKind` has 10 kinds, stored in `effects: [Option<EffectInstance>; 10]`. Haste, Mining Fatigue and Conduit Power are missing, so `/effect give @s haste` answers "Unable to apply this effect" (`docs/MINECRAFT-SURVIVAL.md:127`). |
| Effect lists | `minecraft_survival/src/player_nbt.rs:18-29` (ids), `:31-60` (attribute modifiers); `minecraft_survival/src/commands.rs:150-163`, `:470-482`; `minecraft_survival/src/survival.rs:893-907` (`clear_effects`); `minecraft_terrain/src/potions.rs:230-242` | Five hand-written lists of the same 10 kinds. |
| Attack speed | `minecraft_world.rs:1062-1064`, `third_party/minecraftoss/player/src/crafting.rs:420` | `attack_attributes` gives the item's attack speed. Effects don't change it; vanilla Haste and Mining Fatigue do. |
| Drops | `minecraft_survival/src/entities.rs:134`, `:338-391`, `:1337-1341` | Hand breaks roll the loot with `LOOT_TOOL = minecraft:diamond_pickaxe` plus the held item's enchantments, so a bare hand or a wrong tool still drops. This is a known limitation (`docs/MINECRAFT-SURVIVAL.md:312`). |
| Exhaustion | `minecraft_world.rs:1609` | 0.005 for every block not blasted, bullets included. |
| Tool wear | `third_party/minecraftoss/player/src/inventory.rs:357-368` | `wear_tool_after_mining` exists (it uses `damage_per_block` from the item catalog) but nothing calls it. Mining never wears a tool. Unbreaking's spare chance is applied for elytra and tridents (`minecraft_survival/src/elytra.rs:105-115`, `minecraft_terrain/src/trident.rs:84-93`). |
| Block data | `third_party/minecraftoss/core/src/block.rs:146` | The state catalog has `destroy_speed` and the sound type (`hit`, volume, pitch), but **not** `requiresCorrectToolForDrops`. The item catalog (`iw4l-artifacts/minecraft-26.3/artifacts/item-catalog/26.3.json`) has `damage_per_block` but no tool rules. |
| Block tags | `third_party/minecraftoss/core/src/tags.rs:49-80`, `registries.rs:144`; `minecraft_terrain/src/transform.rs:28-47` (`BlockFacts::in_tag`) | Every jar block tag is loaded, including `mineable/*`, `incorrect_for_*_tool`, `sword_*` and `shears_*`. |
| Jar class reading | `minecraft_terrain/src/class_file.rs:93-106`, `:142`, `:316-360`, `:405`; for example `creative_variants.rs:29-50` | A class-file reader with the constant pool, bootstrap methods and instructions, already used at runtime for creative tabs and enchantment names. |
| Enchantment data | `minecraft_terrain/src/protection.rs:250-340` | `ProtectionBook` reads `data/minecraft/enchantment/*.json` from the jar, including `minecraft:attributes` effects (only `burning_time`) and `slots`. That is the pattern to follow. |
| World data | `minecraft_survival/src/data.rs:9-18`, `:55-58` | `GameData` (world-wide, `Arc`) loads the books once. |
| Vendored mining | `third_party/minecraftoss/player/src/mining.rs` | An unused MinecraftOSS module with a hand-written table of 16 blocks, a water check by block id and no effects. **Don't use it or extend it.** |
| Kit and commands | `docs/MINECRAFT-SURVIVAL.md:125-127`, `:374` | The kit takes `diamond_pickaxe+efficiency=5`. `/effect give` works for known kinds. `/gamemode` exists. |

## 3. Vanilla 26.3 behaviour (javap)

### 3.1 Progress per tick

`BlockBehaviour.getDestroyProgress(state, player, level, pos)` (no block overrides it; only `BlockBehaviour`, `BlockStateBase`, `MultiPlayerGameMode` and `ServerPlayerGameMode` reference it):
- `hardness = state.getDestroySpeed()`. If it is -1, the result is 0 and the block never breaks.
- `divisor = player.hasCorrectToolForDrops(state) ? 30 : 100`.
- The result is `player.getDestroySpeed(state) / hardness / divisor`, all in `f32` and in that order.
  - Hardness 0 gives +infinity, so the block always breaks instantly.
  - Golden pickaxe on netherrack is `12 / 0.4 / 30`, which is exactly `1.0` in `f32`, so it is instant. Unit-test it as a guard on the operation order.

`Player.hasCorrectToolForDrops(state)` is `!state.requiresCorrectToolForDrops() || mainHand.isCorrectToolForDrops(state)`.

`Player.getDestroySpeed(state)` with `f` as an `f32`, in this order:
1. `f = mainHand.getDestroySpeed(state)` (the tool speed, section 3.2).
2. If `f > 1.0`: `f += attribute(mining_efficiency)`. Efficiency does nothing on a block the tool isn't fast on.
3. If `MobEffectUtil.hasDigSpeed` (Haste or Conduit Power): `f *= 1 + (amp + 1) x 0.2`, where `amp = max(haste amplifier, conduit amplifier)` (`getDigSpeedAmplification`; a missing effect counts as 0).
4. If Mining Fatigue: `f *= (float) pow(0.3, amp + 1)`. In 26.3 this is a power, no longer a table: amplifier 0 gives 0.3, 1 gives 0.09, 2 gives 0.027 and 3 gives 0.0081.
5. `f *= attribute(block_break_speed)` (default 1.0; nothing in vanilla survival changes it).
6. If `isEyeInFluid(#water)`: `f *= attribute(submerged_mining_speed)` (default 0.2).
7. If not `onGround()`: `f /= 5`.

### 3.2 Tool rules (the `minecraft:tool` component)

- `Item.getDestroySpeed(stack, state)`: the stack's `Tool` component's `getMiningSpeed(state)`, or 1.0 without one. No `Item` subclass overrides it; `DebugStickItem` only matters in creative.
- `Item.isCorrectToolForDrops(stack, state)`: `Tool.isCorrectForDrops(state)`, or false without one.
- `Tool(rules, defaultMiningSpeed, damagePerBlock, canDestroyBlocksInCreative)`:
  - `getMiningSpeed` returns the speed of the first rule that has a speed and whose block set holds the state, else `defaultMiningSpeed`.
  - `isCorrectForDrops` returns the `correctForDrops` of the first rule that has one and matches, else false.
  - `Rule.minesAndDrops(set, speed)` has both a speed and correct = true; `deniesDrops(set)` has only correct = false; `overrideSpeed(set, speed)` has only a speed.
- **Diggers** (`Item$Properties.pickaxe/axe/shovel/hoe` → `ToolMaterial.applyToolProperties`): `Tool([deniesDrops(#incorrect_for_<material>_tool), minesAndDrops(#mineable/<kind>, materialSpeed)], 1.0, 1, true)`.
  - The deny rule has no speed, so a wooden pickaxe on obsidian still mines at speed 2, just with the divisor 100 and no drop.

  | Material (item prefix) | Speed | Incorrect tag |
  |---|---|---|
  | wood (`wooden_`) | 2 | `incorrect_for_wooden_tool` |
  | stone (`stone_`) | 4 | `incorrect_for_stone_tool` |
  | copper (`copper_`) | 5 | `incorrect_for_copper_tool` |
  | iron (`iron_`) | 6 | `incorrect_for_iron_tool` |
  | diamond (`diamond_`) | 8 | `incorrect_for_diamond_tool` |
  | gold (`golden_`) | 12 | `incorrect_for_gold_tool` |
  | netherite (`netherite_`) | 9 | `incorrect_for_netherite_tool` |

- **Swords** (`ToolMaterial.applySwordProperties`): `Tool([minesAndDrops({cobweb}, 15), overrideSpeed(#sword_instantly_mines, Float.MAX_VALUE), overrideSpeed(#sword_efficient, 1.5)], 1.0, 2, false)`. `#sword_instantly_mines` is bamboo and bamboo sapling. `#sword_efficient` is leaves, vines, glow lichen, pumpkins, melon, cocoa, big dripleaf and chorus.
- **Shears** (`ShearsItem.createToolProperties`): `Tool([minesAndDrops({cobweb}, 15), overrideSpeed(#shears_extreme_breaking_speed = #leaves, 15), overrideSpeed(#shears_major_breaking_speed = wool, wool slabs and stairs, 5), overrideSpeed(#shears_minor_breaking_speed = glow lichen and vine, 2)], 1.0, 1, true)`.
- **Mace and trident** (`MaceItem/TridentItem.createToolProperties`): `Tool([], 1.0, 2, false)`, so they have speed 1, are never correct, and wear by 2 per block.
- Spears and every other item have no tool component.
- `requiresCorrectToolForDrops` is a `BlockBehaviour.Properties` flag set in `Blocks.<clinit>`. **It is in no data file and not in the fork's catalogs.**
  - In 26.3 some blocks dropped it. From the bytecode: `IRON_DOOR`, `BELL`, `LANTERN`, `ENDER_CHEST`, `BREWING_STAND`, `COPPER_DOOR`, `STONE_PRESSURE_PLATE`, `STONE_BUTTON`, `CRAFTER`, `HEAVY_CORE`, `CONDUIT` and `PISTON` don't set it.
  - Still set: stone and its variants (slabs, stairs and walls copy it), all ores, netherrack, obsidian, cobweb, snow layer and snow block, `IRON_CHAIN`, `IRON_TRAPDOOR`, furnace, anvil, hopper, cauldrons, terracotta, concrete (and the new concrete stairs and slabs), copper blocks, cut copper and their stairs and slabs, and so on.
  - A javap prototype found 294 single blocks plus 5 dye-colour families and 12 copper families, about 470 ids.

### 3.3 Enchantments as attributes

- `data/minecraft/enchantment/efficiency.json`: `minecraft:attributes`, attribute `minecraft:mining_efficiency`, `add_value`, amount `{type: levels_squared, added: 1}`, slots `["mainhand"]`. `LevelBasedValue$LevelsSquared.calculate(level)` is `level² + added`: Efficiency I gives 2, II gives 5, III gives 10, IV gives 17 and V gives 26.
- `aqua_affinity.json`: attribute `minecraft:submerged_mining_speed`, `add_multiplied_total`, amount `{type: linear, base: 4, per_level_above_first: 4}`, slots `["head"]`. That gives `0.2 x (1 + 4) = 1.0`: no underwater penalty.
- Attribute defaults (`Attributes.<clinit>`):
  - `block_break_speed`: 1.0, range 0 to 1024;
  - `mining_efficiency`: 0.0, range 0 to 1024;
  - `submerged_mining_speed`: 0.2, range 0 to 20.
- `AttributeInstance.calculateValue`: `((base + Σadd_value) x (1 + Σadd_multiplied_base)) x Π(1 + add_multiplied_total)`, then clamped to the range.
- `unbreaking.json` (for wear): for non-armour, each damage point is spared with chance `level / (level + 1)`, as the elytra and trident code already does.

### 3.4 Effects

`MobEffects.<clinit>`:
- `haste` (colour 14270531) adds `minecraft:effect.haste` +0.1 per level to `attack_speed` (`add_multiplied_total`).
- `mining_fatigue` (4866583) adds `minecraft:effect.mining_fatigue` -0.1 per level to `attack_speed`.
- `conduit_power` (1950417) has no attribute modifier. Its water breathing and underwater vision are out of scope.
- `MobEffect` scales a modifier by `amplifier + 1`, so Haste II gives attack speed x 1.2.

### 3.5 The dig loop (client, `MultiPlayerGameMode` and `Minecraft.continueAttack`)

**Each client tick while attack is held and the hit result is a block that isn't air:** `continueAttack` returns early while `missTime > 0` or while an item is in use, and does nothing for an item with `PIERCING_WEAPON` (a spear). It calls `continueDestroyBlock(pos, face)`, then `player.swing`. If the hit result is not a block (air, or an entity in front), it calls `stopDestroyBlock`.

**`continueDestroyBlock(pos, face)`:**
1. If `destroyDelay > 0`: decrement it and return. The swing still plays.
2. Creative: `destroyDelay = 5` and destroy now. The fork already does this.
3. If `sameDestroyTarget(pos)` (same pos, and the main hand `isSameItemSameComponents` as `destroyingItem`; the count doesn't matter):
   1. If the block is now air: `isDestroying = false`, return.
   2. Otherwise `destroyProgress += getDestroyProgress(...)` (an `f32` sum of each tick's value, so jumping mid-dig slows only those ticks) and `destroyTicks += 1`.
   3. If `destroyProgress >= 1`: destroy, `isDestroying = false`, progress and ticks back to 0, and **`destroyDelay = 5`**.
   4. Send the crack stage `getDestroyStage()`: `(int)(progress x 10)` while progress > 0, else -1 (no crack).
4. Otherwise, `startDestroyBlock(pos, face)`. No progress is added on this tick.

**`startDestroyBlock(pos, face)` in survival:**
1. If a different dig was running, abort it (`ABORT_DESTROY_BLOCK`).
2. If the block isn't air and the progress is 0, call `state.attack(...)` (for example the dragon egg teleports).
3. If `getDestroyProgress(...) >= 1`: **destroy now**. There is no crack, no start sound or particle, and **`destroyDelay` is not set**. So while the button is held, the next block, on the next tick, is started and broken the same way: one block per tick.
4. Otherwise: `isDestroying = true`, remember the pos, face and `destroyingItem = mainHand`, set progress and ticks to 0, send the stage (-1), and `addBreakingBlockEffects(pos, face, true)`: the hit sound and one particle.

**The click tick:** `startAttack` calls `startDestroyBlock` and `continueAttack` runs in the same tick, so the click tick already adds one tick of progress. A held button moving onto a new block only starts on that tick. Dirt by hand (1/15 a tick) breaks on the 15th tick counting the click.

**`stopDestroyBlock()`** (button released, or target no longer a block): if digging, abort, `isDestroying = false`, progress 0, crack -1, and `resetAttackStrengthTicker()`.

**Hit effects** come from the server's `ServerPlayerGameMode.tick()` while `isDestroyingBlock`:
- each tick it sends level event 2019 (particle only), or 2020 (particle and sound) when `(gameTicks - destroyProgressStart) % 4 == 0`;
- relative to the start, that is the 4th, 8th, 12th... tick;
- `LevelEventHandler` turns both into `ClientLevel.addBreakingBlockEffects(pos, face, sound)`.

`addBreakingBlockEffects` (not for air):
- **Sound** (`playBreakingSound`): the sound type's `hit` sound, BLOCKS, volume `(volume + 1) / 8`, pitch `pitch x 0.5`, at the block.
- **Particle** (`addBreakingParticles`), only when the render shape isn't invisible and `shouldSpawnTerrainParticles()`:
  - Take the outline shape's bounds `b`. A random point is `pos + rand x (b.max - b.min - 0.2) + 0.1 + b.min` on each axis.
  - On the hit face's axis, the point is pushed to `b.min - 0.1` or `b.max + 0.1`.
  - One `TerrainParticle` with zero speed, `setPower(0.2)` (x and z speed x 0.2, y speed `(y - 0.1) x 0.2 + 0.1`) and `scale(0.6)`.

**Server tolerance** (`ServerPlayerGameMode.handleBlockBreakAction`):
- On START it runs the enchantment `onHitBlock` hook and `state.attack`, and destroys at once if `getDestroyProgress >= 1`.
- On STOP it destroys when `getDestroyProgress x (ticks since start + 1) >= 0.7`. Otherwise it keeps a delayed destroy that `tick()` finishes once the progress reaches 1.

**`destroyBlock` on the server** (`ServerPlayerGameMode.destroyBlock`):
1. `canDestroyBlock` (creative and sword rule), then `playerWillDestroy` and `removeBlock`.
2. If not creative: `correct = hasCorrectToolForDrops(state)` and `mainHand.mineBlock(...)`.
   - `Item.mineBlock` wears the tool by `damagePerBlock` when the block's hardness isn't 0 and the item has a tool component. **This happens even with the wrong tool.**
3. Then, only if removed and correct: `Block.playerDestroy`. That gives the mined stat, `causeFoodExhaustion(0.005)` and `dropResources` (loot with the real tool, and block XP through `spawnAfterBreak`). **A wrong tool gives no drops, no XP and no exhaustion.**

## 4. Plan (one commit per group, in order)

New files are `minecraft_terrain/src/mining_speed.rs` and `minecraft_survival/src/digging.rs`. They must pass `rustfmt --check` on their staged content. Edits to existing files change only the lines they need, in the surrounding style, with no comments. All arithmetic in the speed and progress code is `f32` in vanilla's order.

### Group 1. "Minecraft haste, mining fatigue and conduit power are real effects"

**Files:**
- `third_party/minecraftoss/player/src/survival.rs` (vendored, already patched; note it in the round log);
- `minecraft_survival/src/{player_nbt.rs, commands.rs, survival.rs}`;
- `minecraft_terrain/src/potions.rs`;
- `render_anim/src/minecraft_world.rs:1062-1064`.

**Changes:**
- Add `Haste`, `MiningFatigue` and `ConduitPower` to `EffectKind`, and `pub const ALL: [EffectKind; 13]`. Size `effects` from it (`:168`, `:187`). Iterate `EffectKind::ALL` in `tick_effects` (`:357-370`), `Survival::clear_effects` (`minecraft_survival/src/survival.rs:893-907`) and the `/effect clear` list (`commands.rs:470-482`), so the next track that adds a kind (C's levitation) touches one place.
- `effect_kind` (`commands.rs:150-163`): `haste`, `mining_fatigue` and `conduit_power`.
- `player_nbt::EFFECTS`: the three ids.
- `player_nbt::MODIFIERS`:
  - `minecraft:effect.haste` on `minecraft:attack_speed`, base 4.0, amount `0.10000000149011612`, `add_multiplied_total`;
  - `minecraft:effect.mining_fatigue`, the same with amount `-0.10000000149011612`.
- `PotionEffect::player_effect` (`potions.rs:230-242`): the three ids, for custom potion contents.
- `SurvivalStatus::attack_speed_multiplier()`, next to `attack_damage_modifier` (`:226`): `(1 + 0.1 (haste + 1)) x (1 - 0.1 (fatigue + 1))`, with 1.0 for a missing effect.
  - In `minecraft_world.rs:1062`, multiply the item's attack speed by it before `attack_strength`.

**Tests:**
- `EffectKind::ALL` covers each kind once.
- `/effect give @s minecraft:haste 60 1` applies amplifier 1 for 1200 ticks; `/effect clear @s minecraft:mining_fatigue` removes only that one; milk clears all three.
- A player file round-trips the three effects. It writes the haste attack-speed modifier with amount 0.2 for amplifier 1, and drops it once the effect is gone.
- `attack_speed_multiplier` is 1.1 for Haste I (amplifier 0), 1.2 for Haste II, 0.8 for Mining Fatigue II, and 1.1 x 0.9 for Haste I with Mining Fatigue I.

### Group 2. "Minecraft reads which blocks need the right tool from the game jar"

**Files:**
- `minecraft_terrain/src/mining_speed.rs` (new) and `minecraft_terrain/src/lib.rs`;
- `minecraft_survival/src/data.rs` (`pub mining: Option<Arc<MiningBook>>`, loaded like `protection` at `:55-58`, with a warning on failure).

**`MiningBook::from_jar(jar)`** scans `net/minecraft/world/level/block/Blocks.class` and `net/minecraft/references/BlockItemIds.class` with `class_file`. The `requires_tool: HashSet<String>` it builds holds full ids such as `minecraft:stone`.

**Algorithm** (the javap prototype in section 3.2 got the right answer for every block checked):
1. **Ids.** In `BlockItemIds.<clinit>`, map each static field to its ids:
   - `ldc "x"; invokestatic BlockItemId.create; putstatic F` gives `F → [x]`.
   - `ldc "base"; invokestatic createSimpleColored; putstatic F` gives the 16 `<dye>_base` ids, in `DyeColor` order: white, orange, magenta, light_blue, yellow, lime, pink, gray, light_gray, cyan, purple, blue, brown, green, red, black.
   - `ldc "base"; invokestatic createSimpleCopper; putstatic F` gives `base`, `exposed_base`, `weathered_base` and `oxidized_base`, plus `waxed_` before each.
   - `COPPER_BLOCK` uses the special names `copper_block`, `exposed_copper`, `weathered_copper` and `oxidized_copper`, plus `waxed_` before each.
   - Any other shape is logged and skipped.
2. **Segments.** Split `Blocks.<clinit>` into segments, each ending at a `putstatic` of a `Block`, `ColorCollection` or `WeatheringCopperCollection` field. The id comes from the segment's first `getstatic BlockItemIds.F`.
3. **Requires flag.** A segment, or any method it reaches, sets the flag directly if it calls `BlockBehaviour$Properties.requiresCorrectToolForDrops`. The methods it reaches are:
   - lambdas through `invokedynamic`, resolved with the bootstrap method's `REF_invokeStatic Blocks.lambda$...` argument;
   - `Blocks` helpers returning `Properties`, such as `logProperties` and `pistonProperties`.

   Follow these recursively, skipping any method already visited.
4. **Copies.** It inherits the flag from a source field when it calls `ofFullCopy`, `ofLegacyCopy`, `registerStair`, `registerLegacyStair`, `registerSlab` or `registerWall`. The source field is the nearest `getstatic Blocks.<field>` of type `Block` or a collection before the call, in the same method body. Resolve inheritance after the whole scan; chains and collection-to-collection copies (cut copper copies the copper block, concrete stairs copy concrete) work that way.
5. **Result.** Every id of a flagged field goes into `requires_tool`. A failed scan gives an empty set, logged once: everything then mines and drops as if no tool were needed, as today's drops already do.

**Tests:**
- Pure: the dye and copper id expansion, and the copper special names.
- With `MINECRAFTOSS_ROOT` (skip without it, as `protection.rs:665-672` does):
  - Every id in the set exists in the block registry, and there are more than 400.
  - True for: `stone`, `netherrack`, `obsidian`, `cobweb`, `snow`, `snow_block`, `iron_ore`, `deepslate_iron_ore`, `cobbled_deepslate`, `stone_slab`, `stone_stairs`, `cobblestone_wall`, `iron_chain`, `iron_trapdoor`, `furnace`, `anvil`, `ancient_debris`, `end_stone`, `basalt`, `white_concrete`, `black_concrete_stairs`, `red_terracotta`, `white_glazed_terracotta`, `copper_block`, `waxed_exposed_copper`, `cut_copper`, `waxed_oxidized_cut_copper_stairs`.
  - False for: `dirt`, `ice`, `packed_ice`, `oak_log`, `glass`, `white_wool`, `iron_door`, `bell`, `lantern`, `ender_chest`, `brewing_stand`, `copper_door`, `stone_button`, `stone_pressure_plate`, `piston`, `conduit`, `crafter`, `heavy_core`, `powder_snow`, `bedrock`.

### Group 3. "Minecraft tools mine at their vanilla speeds with Efficiency, Haste, Mining Fatigue and the water and air penalties"

**Files:** `minecraft_terrain/src/mining_speed.rs`, plus `minecraft_survival/src/data.rs` if `MiningBook` gains the enchantment data.

**Code:**
- **`Tool` and `ToolRule`:**
  - `RuleBlocks::Tag(String) | Blocks(Vec<String>)`, `speed: Option<f32>`, `correct: Option<bool>`, `default_speed`, `damage_per_block` and `destroys_in_creative`.
  - `tool_of(stack)` builds section 3.2's table from the item id: `<prefix>_pickaxe|_axe|_shovel|_hoe` with the material table, `*_sword`, `shears`, `mace` and `trident`.
  - If the stack has a `minecraft:tool` component, it is parsed instead: `rules[{blocks, speed?, correct_for_drops?}]`, `default_mining_speed` (default 1.0), `damage_per_block` (default 1) and `can_destroy_blocks_in_creative` (default true).
  - `Tool::speed(facts, block)` and `Tool::correct(facts, block)` take the first matching rule, with block sets checked through `transform::BlockFacts::in_tag` (tags) or the id (direct sets).
- **Enchantment attributes** in `MiningBook`, read from `data/minecraft/enchantment/*.json`:
  - each enchantment's `minecraft:attributes` entries for `mining_efficiency`, `submerged_mining_speed` and `block_break_speed`;
  - amounts of type constant, `linear`, `levels_squared` and `clamped`; operations `add_value`, `add_multiplied_base` and `add_multiplied_total`;
  - the `slots` groups.
- **`equipped_attribute(attribute, base, min, max, equipment)`:** `equipment` is `(slot name, enchantments)` for the main hand, the offhand and the four armour slots (`39` head, as in `protection.rs:25`). It counts an enchantment only when its `slots` group fits the slot (`mainhand`, `offhand`, `hand`, `head`, `armor`, `any` and so on), then applies the attribute formula of section 3.3.
- **`pub struct Digger`:** `held: Option<&ItemStack>`, `equipment`, `haste: Option<u8>`, `conduit: Option<u8>`, `fatigue: Option<u8>`, `eye_in_water: bool`, `on_ground: bool`.
- **`pub fn correct_for_drops(book, facts, block, held) -> bool`** and **`pub fn destroy_speed(book, facts, block, digger) -> f32`**: section 3.1, steps 1 to 7.
- **`pub fn destroy_progress(book, facts, block, hardness, digger) -> f32`:** 0 for hardness -1, else `speed / hardness / (30 or 100)`.
- **`pub fn ticks_to_break(progress) -> Option<u32>`** (test helper, `f32` accumulation): `None` for 0.

**Tests** (a fake `BlockFacts` with the needed tags, as `transform.rs:396` does). The expected tick counts include the click tick:

| Case | Expected |
|---|---|
| Netherrack, diamond pickaxe, Efficiency V | progress about 2.83, instant |
| Netherrack, diamond pickaxe, Efficiency II | instant |
| Netherrack, diamond pickaxe, Efficiency I | 2 ticks |
| Netherrack, plain diamond pickaxe | 2 ticks |
| Netherrack, golden pickaxe | exactly 1.0, instant |
| Netherrack, wooden pickaxe | 6 ticks |
| Netherrack, bare hand | 41 ticks (the `f32` sum of 0.025 is 0.9999996 after 40), not correct |
| Stone, bare hand | 151 ticks (`f32` sum), not correct |
| Stone, plain diamond pickaxe | 6 ticks |
| Stone, Efficiency V | 2 ticks |
| Stone, Efficiency V and Haste II | instant |
| Stone, Efficiency V and Conduit Power I | 2 ticks |
| Stone, Efficiency V and Haste I with Conduit Power II | uses amplifier 1, instant |
| Stone, Efficiency V and Mining Fatigue I | 5 ticks |
| Stone, Efficiency V and Mining Fatigue III | 50 ticks (progress 0.0204) |
| Stone, Efficiency V, eye in water, on ground | 7 ticks |
| Stone, Efficiency V, eye in water, not on ground | 34 ticks |
| Stone, Efficiency V, eye in water, not on ground, Aqua Affinity helmet | 7 ticks |
| Stone, Efficiency V, dry, not on ground | 7 ticks |
| Obsidian, diamond pickaxe, Efficiency V | 45 ticks |
| Obsidian, iron pickaxe | divisor 100, not correct |
| Iron ore, wooden pickaxe | 151 ticks (`f32` sum), not correct |
| Iron ore, stone pickaxe | 23 ticks |
| Cobweb, diamond sword | 8 ticks, correct |
| Cobweb, shears | 8 ticks, correct |
| Cobweb, bare hand | 401 ticks (`f32` sum) |
| Bamboo, sword | instant (and finite, not NaN) |
| White wool, shears | 5 ticks |
| Oak log, iron axe | 10 ticks (`f32` sum of 0.1) |
| Dirt, diamond pickaxe, Efficiency V | 15 ticks: Efficiency is ignored at speed 1 |
| Torch (hardness 0), Mining Fatigue III | instant |
| Bedrock | progress 0 |

Two more tests:
- Efficiency on an offhand pickaxe or on a helmet adds nothing.
- With `MINECRAFTOSS_ROOT`: the jar's Efficiency V gives `mining_efficiency` 26, and Aqua Affinity I gives `submerged_mining_speed` 1.0.

### Group 4. "Minecraft hand mining follows vanilla's dig loop: instant breaks, the 5-tick delay, cracks, hit sounds and particles"

**Files:**
- `minecraft_survival/src/digging.rs` (new) and `lib.rs`;
- `minecraft_survival/src/player.rs` (`Hand.dig: crate::digging::Dig`);
- `render_anim/src/minecraft_world.rs` (`:1133-1149`, `:1537-1547`, `:1597-1609`, `:2212`);
- `render_anim/src/minecraft_mining.rs` (`crack_mesh` also takes the local dig's `(pos, stage)`);
- `minecraft_terrain/src/block_particles.rs` (a new `spawn_hit(packs, scene, pos, face, block, bounds, atlas)` next to `spawn`, minimal lines).

**`Dig` (per player)** holds `destroying: Option<(Pos, Face, Option<ItemStack>)>`, `progress: f32`, `ticks: u32`, `delay: u32` and `start_tick`. Two calls:
- `Dig::tick(&mut self, input: DigInput) -> Vec<DigEvent>`. `input` is the target `(pos, face, &Block)` or `None`, the held stack, `clicked` (this tick is the click tick), `held_down`, and a `progress: &dyn Fn(&Block) -> f32` closure.
- `Dig::stop(&mut self) -> bool`, true when a dig was running, so the adapter resets `attack_ticks`.

It follows section 3.5 step by step:
- delay first;
- same target, by pos and `ItemStack::same_item`;
- start, giving `Attacked` and then `Broke { instant: true }` or `Started`;
- continue, giving `Hit { particle: true, sound: ticks % 4 == 0 }` and `Broke { instant: false }`, which sets delay 5.

`Dig::crack()` returns `(pos, stage)` only while progress > 0.

**Adapter changes in `minecraft_world.rs`:**
- **Replace** the fake `VoxelEvent::Shot` at `:1133-1149` with a `hand.dig.tick(...)` per hand tick.
  - Target: `player.target(Outlined, game_mode.block_reach())`, except that a mob closer than the block (`entities.mob_distance`, as the creative path does at `:1122`) counts as no target.
  - The click tick is the existing `creative_press` case (`:1077-1081`); rename it to `block_press`.
  - The `progress` closure calls `mining_speed::destroy_progress` with `globals.data.mining`, `terrain.stream.states` (as `BlockFacts`), the state's `destroy_speed`, and a `Digger` built from:
    - `inventory.slots[*selected]`, the offhand and armour slots;
    - `survival.status.effect(...)` for the three effects;
    - `water.eye` (`:833`) and `on_ground` (`:831`).
- **Events:**
  - `Attacked`: call `entities.attack_block` for the dragon egg (moved from `:1541-1544`, which then only filters bullets).
  - `Broke`: push to `hand_mined` and to a `hand_broken` list that goes through `mining.break_now` with `creative_broken` (`:1598`). It gets the same break burst, break sound and server path.
  - `Started` and `Hit`: the hit sound at `(volume + 1) / 8`, pitch x 0.5, when `sound` is set, and `mining.hit_particle(...)` (a new wrapper over `BlockParticles::spawn_hit`).
- **Stop:** when the button is up, the item is not a Minecraft item, `busy`, or the target is lost, call `hand.dig.stop()`; if that returns true, reset `hand.attack_ticks`. A gun in hand stops the dig.
- **Swing:** keep the existing swing restart on each dig tick, delay ticks included.
- **Crack:** `view.cracks` (`:2212`) adds `hand.dig.crack()` to the bullet cracks.
- **Bullets** keep `Mining::apply` with `DAMAGE_PER_HARDNESS` unchanged; they are MW2 rules. Only the hand stops using it.
- **Creative** is unchanged.

**Tests (pure `Dig`):**
- Dirt at 1/15 a tick, click and hold: `Started` on the click tick and progress that same tick; breaks on tick 15; crack stages 0..9 in between, none on the start tick.
- Held on, a second dirt: 5 idle delay ticks, a start tick without progress, then 15 ticks, so it breaks on tick 36.
- Instant chain: progress 2.8 gives `Broke { instant: true }` on every tick for successive targets, with no `Started`, no `Hit`, no crack and no delay.
- Hits: on start, then a sound on ticks 4, 8 and 12 and a particle every tick.
- Releasing resets progress and crack, `stop()` returns true, and a new click starts from 0.
- A changed item (other components) restarts; a changed count does not.
- A target that turned to air stops the dig without breaking.
- A target that moved, at the same item, restarts with an `Attacked` start.

### Group 5. "Minecraft blocks mined with the wrong tool drop nothing, and mining wears the tool, like vanilla"

**Files:**
- `minecraft_survival/src/entities.rs` (`drop_blocks`, `award_block_experience`, `:338-391`);
- `minecraft_survival/src/digging.rs` (a `wear_after_break` helper);
- `render_anim/src/minecraft_world.rs:1609-1626`.

**Changes:**
- For each `hand_broken` block in survival, compute `correct = mining_speed::correct_for_drops(...)` with the held stack **before** the break.
  - Only correct blocks are passed on to `drop_blocks` and `award_block_experience`, and only they add `MINE_EXHAUSTION`.
  - Bullet and blast breaks keep today's rules.
- Hand loot rolls with the **real held stack** (`None` for an empty hand) instead of `LOOT_TOOL`. Shears on leaves or short grass then drop the block, and a sword on cobweb drops string. `LOOT_TOOL` stays for bullets.
- **Wear:** after every survival hand break (creative excluded, correct or not), use the stack's `damage_per_block` (from `tool_of` or `recipes.mining_tool_wear`) when the block's hardness isn't 0.
  - Each point is spared with chance `unbreaking / (unbreaking + 1)`, as `elytra.rs:105-115` does, then the rest goes through `inventory.wear_tool`.
  - A tool that breaks plays the existing tool break burst and sound (`minecraft_world.rs:1833`).

**Tests:**
- A bare hand on stone gives no drops, no XP and no exhaustion.
- A wooden pickaxe on iron ore gives no drop; a stone pickaxe drops raw iron.
- A sword on cobweb drops string; shears on cobweb drop cobweb.
- A pickaxe loses 1 per stone; a sword loses 2 per block; a torch (hardness 0) costs nothing; creative costs nothing.
- Unbreaking III with a seeded random spares about 3 in 4.
- A pickaxe with 1 durability left breaks with the burst event.

### Group 6. "Docs: Minecraft mining speed"

**File:** `docs/MINECRAFT-SURVIVAL.md`.
- Under "Mining and placing", describe the dig loop, tool speeds, Efficiency, the effects, the water and air penalties, wrong-tool drops and wear.
- Remove the limitation at `:312`.
- Add the limitations:
  - `block_break_speed` from item attribute modifiers is not read;
  - other players' cracks wait for multiplayer;
  - Conduit Power only speeds mining.
- Note in the round log that `third_party/minecraftoss/player/src/survival.rs` was patched.

## 5. Owner test steps

`.env`: any survival world; the Nether for step 1. The kit has 22 stacks:

```
IW4L_MINECRAFT_KIT=minecraft:diamond_pickaxe+efficiency=5,minecraft:diamond_pickaxe,minecraft:golden_pickaxe,minecraft:wooden_pickaxe,minecraft:stone_pickaxe,minecraft:netherrack*64,minecraft:stone*64,minecraft:obsidian*16,minecraft:iron_ore*8,minecraft:diamond_shovel+efficiency=5,minecraft:dirt*64,minecraft:shears,minecraft:white_wool*16,minecraft:cobweb*8,minecraft:diamond_sword,minecraft:bamboo*16,minecraft:diamond_helmet+aqua_affinity,minecraft:water_bucket,minecraft:ladder*8,minecraft:milk_bucket,minecraft:oak_log*16,minecraft:iron_axe
```

1. **Efficiency V on netherrack** (the owner's case). In the Nether (or on placed netherrack), select the Efficiency V diamond pickaxe and click a netherrack block.
   - It breaks the instant the button goes down, with no crack.
   - Hold the button and sweep the crosshair along a wall: one block breaks every tick, about 20 a second, with no pause between blocks.
   - The pickaxe's durability bar appears after the first block.
   - **Screenshot 1:** a long tunnel dug while holding.
2. **The plain diamond pickaxe** on netherrack: a short crack flash, 2 ticks per block. Holding it, each next block waits a short beat (5 ticks) before it starts.
   - The golden pickaxe also breaks netherrack instantly.
   - The wooden pickaxe takes about 0.3 s per block.
3. **A bare hand** (an empty hotbar slot) on netherrack: about 2 s, and **nothing drops**. A bare hand on stone takes 7.5 s and drops nothing.
   - The crack climbs through 10 stages, the hit sound ticks about 5 times a second, and small chips fly off the face you hit.
   - Let go halfway and click again: the crack starts over from nothing.
   - **Screenshot 2:** a half-cracked stone block.
4. **Stone with the Efficiency V pickaxe:** 2 ticks per block.
   - Type `/effect give @s minecraft:haste 60 1`: stone now breaks instantly, like the netherrack in step 1.
   - Type `/effect give @s minecraft:mining_fatigue 60 2`: stone now takes about 2.5 s.
   - Drink the milk bucket: back to 2 ticks.
   - `/effect give @s minecraft:conduit_power 60 0` makes it 2 ticks (no visible change).
5. **Water.** Put the water bucket into a 2-deep stone hole, or find a lake, and dive.
   - Standing on the bottom with the Efficiency V pickaxe, a stone block takes about 0.35 s.
   - Swimming without touching the bottom, about 1.7 s.
   - Put on the Aqua Affinity helmet: swimming, it is back to about 0.35 s.
6. **Off the ground:** place a ladder column against a wall, climb it, and mine stone beside it. It takes about 0.35 s instead of 2 ticks.
7. **Obsidian** with the Efficiency V pickaxe: about 2.3 s, and it drops. With the plain diamond pickaxe it takes about 9.4 s.
8. **Iron ore:** with the wooden pickaxe, about 7.5 s and **nothing drops**. With the stone pickaxe, about 1.2 s and raw iron drops.
9. **Other tools.**
   - Cobweb: by hand about 20 s, no drop; with the sword 0.4 s and string drops; with shears 0.4 s and the cobweb itself drops.
   - Bamboo with the sword: instant.
   - Wool: shears 0.25 s, hand 1.2 s.
   - Oak log: iron axe 0.5 s, hand 3 s.
   - Dirt: the Efficiency V shovel is instant (tunnel through dirt like step 1); the Efficiency V pickaxe takes 0.75 s, the same as a bare hand.
10. **Creative is unchanged:** `/gamemode creative`; a click breaks any block at once, and holding breaks one every 6 ticks. The sword breaks nothing. `/gamemode survival` afterwards.
11. **26.3 drop changes**, so they aren't reported as bugs: in 26.3 iron doors, bells, lanterns, ender chests, brewing stands and copper doors drop even when broken by hand (slowly). Stone pressure plates and buttons do too.

## 6. Risks, shared files and decisions

**Risks**
- **The `Blocks.class` scan is the fragile part.** The group 2 truth table and the "every id exists" test catch a changed bytecode shape. Without the jar (the stand-in suite), the set is empty: blocks then mine at /30 and drop, which is today's behaviour, never a crash.
- **`f32` order.** Golden pickaxe on netherrack is exactly 1.0 only when computed as `speed / hardness / 30` in `f32`. The oak log case only reaches 10 ticks with an `f32` running sum, and the bare-hand cases need one tick more than the exact fraction (41, 151, 401) for the same reason. All have tests; the counts in group 3 were recomputed with `f32` accumulation.
- **The eye-in-water and ground inputs come from MW2 movement** (`sim::voxel::water(..).eye`, `ps.ground_entity_num`), not from Minecraft physics. A ladder or vine in MW2 has no ground entity, so it counts as off the ground, as vanilla does. Prone and crouch don't matter.
- **Owner perception.** Mining by hand gets much slower for stone-like blocks (7.5 s instead of 2.25 s for stone) and gives no drops. That is vanilla, and section 5 step 3 states it.
- **Tools now wear while mining.** Long tunnels cost durability, as in vanilla.
- **Mining Fatigue only comes from `/effect`:** there are no elder guardians. Haste only comes from `/effect` too: there are no beacons yet.

**Shared with other round 8 tracks**
- **`render_anim/src/minecraft_world.rs:1050-1149` and `:1590-1626` (E).** E moves the player-action block into `minecraft_survival::actions` (`docs/CRATE-SPLIT.md` section 6). All the logic here lives in `minecraft_survival::digging` and `minecraft_terrain::mining_speed`, so E only moves the `Dig::tick` call and the `Digger` construction. Merge A before E's action move, or E rebases the 30 changed adapter lines.
- **`third_party/minecraftoss/player/src/survival.rs`, `player_nbt.rs`, `commands.rs`, `potions.rs`, `survival.rs::clear_effects` (C: levitation for shulkers; B: nothing expected).** Group 1's `EffectKind::ALL` gives C a single place to add a kind. Whichever merges second rebases the enum and the array size.
- **`minecraft_survival/src/entities.rs` `drop_blocks` (B and C add mob loot, E splits `Entities`).** Only the hand-tool argument and the correct-tool filter change.
- `minecraft_survival/src/player.rs` `Hand` (E): one new field.
- `minecraft_survival/src/data.rs` (other A items may add books): one field.
- `minecraft_terrain/src/block_particles.rs` (C's visual leftovers): one new function.
- `docs/MINECRAFT-SURVIVAL.md` (everyone).

**The per-player and server tick boundary (for track E)**
- **Per player:** `Hand.dig` is on `Player` (UUID). Its inputs (held stack, equipment, effects, eye in water, on ground) are all per player. `Dig::tick` is pure and has no render or `sim` types, so the server can run it per player when E moves actions.
- **World-wide:** `MiningBook` is read-only, in `GameData` (`Arc`, one per world).
- **Still client-authoritative:**
  - The local adapter decides the break and sends `player_mines`, as today.
  - When E moves actions to the server, use vanilla's tolerance: START with progress >= 1 breaks at once; STOP breaks when `progress x (ticks + 1) >= 0.7`, else a delayed destroy.
  - For now the server never sees progress.
- **Other players' view:**
  - Vanilla broadcasts the crack (`destroyBlockProgress`, stage per breaker id) and the 2019/2020 hit events to everyone nearby. Here only the local player sees their own crack, chips and hit sound.
  - E needs a `minecraft_net` message `(player, pos, stage)` and the hit event; `Dig::crack()` and the `Hit` events are its source.
- **No new "player 0".** The dragon egg attack keeps the existing `entities.attack_block` path, which E maps to the sender.

**Decisions taken (vanilla-like)**
1. The full formula of section 3.1, with tool rules from section 3.2. Efficiency and Aqua Affinity are read from the jar's enchantment JSON as attribute effects, not hard-coded numbers. Tool rules are written per item id, from javap, since the item catalog lacks them; a stack's own `minecraft:tool` component wins.
2. `requiresCorrectToolForDrops` is read from the jar's `Blocks` bytecode at runtime. A committed list would be Mojang data, and a tag guess is wrong for ice, iron doors, lanterns and more.
3. Hand mining becomes vanilla's dig loop:
   - progress resets on release, item change and target change;
   - no delay after an instant break; 5 ticks after a normal one;
   - the click tick counts;
   - the crack shows only while progress > 0;
   - the hit sound on start and every 4 ticks at `(volume + 1) / 8`, one chip particle a tick on the hit face.
4. Bullets and blasts keep their MW2 rules: hardness-based progress, `LOOT_TOOL` drops and their own hit sound. Only the hand changes.
5. Wrong-tool hand breaks drop nothing, give no XP and no exhaustion. Hand loot uses the real held item. Mining wears the held tool by its `damage_per_block`, with Unbreaking, in survival only.
6. Haste, Mining Fatigue and Conduit Power become real effects: saved, cleared by milk, and settable by `/effect`. Haste and fatigue also change attack speed, as in vanilla. Conduit Power's water breathing and night vision are not part of this item.
7. Creative breaking stays as it is; it already matches vanilla.
8. The unused vendored `minecraftoss_player::mining` module is left alone.
