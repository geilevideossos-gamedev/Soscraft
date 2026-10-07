# Minecraft survival: state, limitations, testing

State of branch `minecraft-survival` at 622f915 (2026-10-07). It is 90 commits ahead of upstream `main` (f608f85) and 0 behind. Rules and goal: [../CLAUDE.md](../CLAUDE.md).

## Implemented

Most of this was added by the fork. Items marked [upstream] were already in upstream's README.

### World and rendering
- [upstream] Minecraft 26.3 world generation from MinecraftOSS, with day and night, sky, clouds, lighting, block, mob and footstep sounds, and a minimap.
- Back-face culling like vanilla, so cross plants no longer flicker.
- Faces are culled by vanilla occlusion shapes (`face_culling.rs`). Cactus, spawners, ice and beacons no longer leave holes in their neighbours.
- Vanilla `BlockColors` tinting. Faces are tinted only where vanilla has a tint source: grass, foliage per leaf type, stems, lily pad, leaf litter, water cauldron. The stonecutter saw stays grey.
- A translucent pass for stained glass and panes, ice, honey, slime, tinted glass and the nether portal. Fluids are only hidden by faces that actually cover them, and clouds are ordered against blended terrain.
- Animated textures from `.mcmeta` (fire, lava, water, portal, sea lantern, prismarine, sculk, stonecutter and more), in the world and in icons.
- The break-crack overlay follows the block's real model.
- Block-entity models: chests (double, trapped, ender, copper), shulker boxes, banners, heads, bell, conduit, decorated pot and books. They are used in the world, as icons, as dropped items and as held items.
- Falling blocks and primed TNT are meshed like terrain, with no flicker.
- XP orbs and the burning overlay with two flames.
- Weather from `IW4L_MINECRAFT_WEATHER`: rain darkens the sky and puts out fire.

### Movement on blocks (MW2 movement with vanilla rules)
- Ground, falls and footsteps. Body size is 0.6 x 1.8 blocks, or 1.5 crouched. Prone keeps MW2's height.
- Swimming: sink, rise, dive, sprint-swim, hop out at ledges. Lava movement.
- Climbing ladders, vines and scaffolding. Sneaking stops at edges.
- Slowdown in berry bushes and cobwebs. Using an item slows you to 0.2.
- A player is never embedded in blocks; anyone inside a block is pushed out.

### Survival and combat
- Hearts, hunger, saturation, exhaustion, regeneration and starvation. MW2 health follows the hearts at 5 per point, and MW2's own regeneration is off.
- Vanilla fall damage, including the blocks that soften falls. Drowning with air bubbles.
- Burning from lava, fire, soul fire, magma and campfires, and putting it out.
- XP levels, the XP bar and vanilla XP from blocks.
- Vanilla attack cooldown, crits, sweeps and knockback. Shield blocking and the totem of undying.
- Vanilla death messages and a Killed By card. A Minecraft death is not counted as an MW2 suicide.
- MW2 explosives (frag, semtex, C4, claymore, RPG, grenade launcher, killstreak blasts) hurt and push mobs, and a player's kills drop loot and XP. Concussion and flash grenades leave no crater. Crater size follows the blast radius.
- A stun (concussion) grenade that hurts a mob slows it like a stunned MW2 player: Slowness IV (0.4x walk speed) for 2 s at the 512-unit edge up to 6 s at point blank. Flash grenades don't slow mobs, as they don't slow players.

### Mining and placing
- [upstream] Shooting and knifing breaks blocks by hardness, broken blocks drop items, and grenades blow craters.
- The hand ray follows the MW2 crosshair in every stance. Items that are not blocks are never placed.
- Vanilla placement states: beds, banners, signs, heads, bells, lecterns, pots, shulker boxes, ladders, vines (one face per use, added to a vine already there), levers, buttons, lanterns, tripwire hooks, shelf mushrooms and glow lichen (the first looking direction with a support), lily pads and frogspawn (on the water source the crosshair meets). Chests join into double chests.
- Blocks are only placed where vanilla `canSurvive` allows, on the client and again on the server: sugar cane needs water, a waterlogged block or frosted ice beside its ground, cactus a free side, flowers and saplings soil, torches a floor that holds their centre, carpets and cakes something below, rails a rigid top. A refused block stays in the hotbar; one the server refuses comes back as its item.
- Doors, trapdoors, fence gates, levers, buttons and note blocks.
- Scaffolding. Blocks that lose support break and drop. Sand and gravel fall. Sugar cane and cactus columns break one block a tick, corals, cakes and shelf mushrooms go with their support, stalagmites break and stalactites fall.
- Ice broken by hand, bullet or knife over a block in `#blocks_motion` or a liquid turns into a water source, except where water evaporates; blasts and Silk Touch leave none. Packed and blue ice never melt.
- Hand mining with Silk Touch or Fortune applies the enchantment to the drops, and Silk Touch gives no block experience.

### Crafting, inventory, containers
- 2x2 and 3x3 crafting from the 26.3 recipe data, with remainders and real stack sizes.
- The vanilla inventory, hotbar, status bars and station screens, drawn from the runtime packs.
- Vanilla clicks: drag-split, double-click, shift-click, number keys, Q and Ctrl+Q, F for offhand, click outside to drop. Controller slot focus.
- Chests, barrels, furnace, smoker and blast furnace (they cook while closed), brewing stand, stonecutter, smithing table, grindstone, anvil and enchanting table.
- Structure loot tables, shulker boxes that keep their contents, a shared ender chest, and the offhand slot.
- Tooltips with MW2 gun stats and Minecraft tool stats.

### Items
- Eating, drinking and potions. Glass bottles, buckets (water, lava, powder snow), cauldrons and waterlogging.
- Flint and steel and fire charges light fire, campfires and candles, and prime TNT.
- Snowballs, eggs and ender pearls. Sweet and glow berries.
- Campfire cooking: 4 slots, 30 s per item, nothing cooks while unlit.
- Spawn eggs for every simulated mob.
- Dropped items burn up in fire, soul fire and lava, and lava sizzles. Netherite gear and ancient debris never burn and float on lava. Lit campfires leave items alone, as in vanilla.

### World simulation (integrated server thread)
- Fluids flow, and lava meeting water hardens. Fire spreads. TNT explodes.
- Nether portals: a 2-21 x 3-21 obsidian frame lights, shimmers with particles and hum, and collapses when the frame breaks.
- Simulated mobs: cow, mooshroom, horse, donkey, pig, chicken, sheep, zombie, husk, zombie villager, skeleton, stray, bogged, parched, creeper, spider, slime, enderman, witch, iron golem, wolf, bat and villager. Other kinds stay dormant.

### MW2 integration and UI
- [upstream] Guns sit in the hotbar with unlimited ammo. Skate 3 works on block collision.
- Guns left in a crafting grid are not duplicated. Esc closes a Minecraft screen. The Skate toggle is blocked while a screen is open.
- Options: the Minecraft minimap (M), Show Blood, and an Advanced Video page that replaces the Video page.
- The loading screen stays up like vanilla's until the 7x7 spawn chunks have collision and the camera's section is drawn (0.5 s close delay, 30 s timeout). Every MW2 spawn and respawn lands on the Minecraft spawn, or on the nearest free dry column within 10 blocks when someone stands there. A player in that area waits in place until the ground under it has collision.

## Multiplayer and saving today

Everything here comes from reading the code. No multiplayer session was run.

### MW2 netcode
The MW2 netcode is host-authoritative and goes through the `iw4l-master` QUIC relay. It needs a self-hosted master; there is no direct IP connect and no LAN discovery. Player movement, MW2 models, gun switching and PvP are synced.

### The Minecraft world is single player
- It runs only on the host. `render_anim/src/minecraft_world.rs` returns early without a local `AuthorityWorld`, so a joining client most likely never finishes loading the world.
- The seed is random per process unless `IW4L_MINECRAFT_SEED` is set.
- Nothing Minecraft goes over the network: blocks, mobs, items, inventories, containers, hunger, time of day and weather are all host-only.
- The server thread knows one player (`PLAYER = 0` in `minecraft_entities.rs`). Chunks stream around the host only.
- Survival state (inventory, containers, hunger) lives in the render crate (`render_anim`), not in the sim or the server thread.

### Saving does not happen yet
- The world is opened without a world folder, so chunks go to a temp `minecraftoss-session-*` folder that is deleted on exit.
- Entity storage is never set, so the 5-minute entity autosave writes nothing.
- These are not serialized at all: containers, player inventory, survival stats, position, time and weather.
- The storage layer itself exists and writes vanilla Anvil regions (DataVersion 5023).

## Limitations

### Persistence and multiplayer
- No persistent world and no Minecraft multiplayer (see above).
- Overworld only. There is no Nether travel. It would need:
  - a shared world folder and a runtime dimension switch;
  - the 80-tick portal timer, x8 coordinate scaling and linking to a return portal;
  - the Nether sky and fog;
  - Nether mobs.

### Rendering
- Translucent faces are not sorted by camera distance within a 16-block section. Inside one block, smaller faces are drawn first.
- Blockstates rotated about X (horizontal logs, upside-down stairs, ceiling vines) are drawn rotated, but still cull with the old full-cube rule, so they may draw a few hidden faces.
- `uvlock` only applies to faces it turns half round. Faces it would turn a quarter (some stair tops, fence and wall caps) keep the model's texture direction, and the face `rotation` key is not read.
- Block entities never animate: no chest lids, bell swing or banner wave. There are no banner or shield patterns and no sign text. Heads use the default skin, and the conduit always shows its inactive shell.
- Not drawn: the copper golem statue and the held trident model. Each half of a double chest is lit separately.
- No lava fog and no rain particles.
- Missing particles: bone meal, smoke, fizz, item break, TNT and campfire smoke.
- Missing sounds: ambient fire, lava and campfire.
- The display tick only animates nether portals.
- The animation clock keeps running while paused.
- GUI gaps: no 3D enchanting book, smithing armour stand, recipe book, effects panel, enchant glint or rarity colours. Characters missing from the font show as gaps.

### Movement
- The bush and cobweb slowdown has no momentum carry, and the Weaving effect is ignored.
- Shallow-lava details differ from vanilla. Powder snow neither slows nor freezes.
- No ground jump at a ladder, no climbing in fluids, and an open trapdoor above a ladder doesn't count as part of it.
- Sneak edge: no back-off while in the air, and it checks blocks only, not entities.
- A respawn shows no loading screen; the player only waits for its own chunk's collision, so terrain may still fade in. Respawns are not spread at random over vanilla's radius of 10.

### Survival and combat
- Hand mining counts as a diamond pickaxe with the held item's enchantments, so a bare hand or a wrong tool still drops items.
- Not modelled: Fire Protection, Frost Walker and the burn-time attribute.
- Lava and fire deaths have no death sound.
- Shield: no knockback to the attacker, axes don't disable it, and MW2 bullets ignore it.
- The totem plays no animation. Every item has the same use speed.
- Difficulty is fixed at Normal. Witch potions don't reach the player.
- MW2 blast damage on mobs is MW2 damage x 0.4 (bullets x 0.2), not vanilla's explosion formula. Killstreak blasts leave no crater. Mobs can't set off claymores. Kills by bot explosives give no XP.
- TNT explosions don't hurt mobs or players; only creeper explosions do.

### Blocks and items
- Buckets and flint and steel wait for the server, with no client prediction.
- No Unbreaking on flint and steel, and lava doesn't set blocks on fire.
- Books can't be placed on a lectern.
- A shulker box or bundle that burns as an item loses its contents instead of spilling them. Experience orbs don't burn. A burning item keeps burning in a water cauldron.
- Trapped chests give no redstone signal.
- Pointed dripstone always points up, so it is refused under a ceiling where vanilla hangs it. Farmland and dirt paths go under solid blocks where vanilla places dirt instead. The client judges mushroom and crop light from its own light; where the server disagrees it refuses the block and drops it as an item. Falling stalactites hurt nobody.
- Redstone covers only levers, buttons and the powered sounds of doors, trapdoors and gates.
- Spawn eggs for unsimulated mobs (fox, rabbit, goat and others) spawn nothing, and the egg is kept. Clicking a spawner with an egg doesn't change its mob.
- The fire charge uses flint and steel's block-update flags.
- No campfire smoke, and no dousing with a shovel.
- The enchanting slot takes tools, armour and books only. The anvil name field only edits at the end.
- Ice whose support breaks in the same frame leaves no water, and melted ice shows air for about a server tick before the water appears.
- To check in game: whether breaking one bed half removes the other, and whether a ladder drops when its wall is removed. The update rules for both exist now.

### Mobs
- Only the kinds listed above are simulated. There are no Nether mobs: ghast, piglin, zombified piglin, hoglin, magma cube, blaze, wither skeleton and strider are missing.

## Testing workflow

### Environment (PowerShell)
```
$env:Path="$env:USERPROFILE\.cargo\bin;$env:Path"; $env:CARGO_TARGET_DIR="D:\iw4l-target"; $env:CARGO_INCREMENTAL="0"; $env:CARGO_PROFILE_DEV_DEBUG="0"; $env:MINECRAFTOSS_ROOT="C:\Games\Mashup\2010-Rust-Rewrite-Mashup\iw4l-artifacts\minecraft-26.3"
```
Run every cargo command with `-j 8`, one at a time.

### Per change
1. **Unit tests** for each touched crate: `cargo test --locked -j 8 -p <crate>`.
   - `third_party/minecraftoss` is outside the workspace. Its packages are `minecraftoss-core`, `minecraftoss-world`, `minecraftoss-generator`, `minecraftoss-player` and `minecraftoss-entities`.
   - `minecraftoss-entities` refuses `--locked`. Run it with `--offline` and delete the `Cargo.lock` that this creates.
   - `minecraftoss-player` tests need two empty stand-in files: `third_party/scenarios/survival/difficulty-reference.csv` and `difficulty-damage-reference.csv`. Create them, run the tests, then delete `third_party/scenarios`.
   - Tests that need the real game data are gated on `MINECRAFTOSS_ROOT`. Every other test uses synthetic fixtures.
   - Known flaky tests: `poof_particles::a_puff_rises_and_fades_within_its_lifetime` and `sections::tests::background_full_update_matches_the_synchronous_one`.
2. **Clippy** on the touched crates: `cargo clippy --locked -j 8 -p <crates> --all-targets -- -A clippy::approx_constant`.
   - It must report no warnings on lines the change touched.
   - The workspace denies `unwrap_or_default`.
3. **rustfmt on new files only**, checking the staged content: `git show :<file> | rustfmt --check --edition <ed>`. Any "Diff in" output is a failure. Checkouts are CRLF and `rustfmt.toml` wants LF.
   - Edition 2021: `minecraft_terrain`, `minecraftoss-entities`, `minecraftoss-player`.
   - Edition 2024: everything else.
   - `minecraft_terrain` is edition 2021, so it can't use let-chains.
4. **Whole game:** `cargo check --locked -j 8 -p launcher`.
5. **Commit:** stage only the change's files and end the message with the Co-Authored-By line.

### Play test
1. Build with `cargo build --release --locked -j 8 -p launcher`. It takes about 25 minutes and produces `D:\iw4l-target\release\iw4l.exe`.
2. Copy it to `C:\Games\Mashup\2010-Rust-Rewrite-Mashup\iw4l-survival.exe`. Older builds are kept as `iw4l-survival-<hash>.exe`.
3. The `.env` next to the exe holds `IW4L_MINECRAFT_KIT=`.
   - Format: a comma list of `id` or `id*count`; the `minecraft:` prefix is optional. `+enchantment` or `+enchantment=level` after the id enchants the item, e.g. `diamond_pickaxe+silk_touch`; a count goes last, as in `iron_sword+sharpness=5*2`.
   - It fills only the 27 backpack slots, so give at most 27 stacks. The log prints how many stacks were given.
4. Start with `iw4l-survival.exe map minecraft:overworld`.
5. The owner plays and sends numbered screenshots in a folder named after the build, e.g. `screenshots-<hash>/` or `ac0c791/`. These folders are git-ignored.
6. Logs: `C:\Games\Mashup\2010-Rust-Rewrite-Mashup\iw4l-artifacts\logs\latest.log`.

Other switches:

| Variable | Effect |
|---|---|
| `IW4L_MINECRAFT_SEED` | Fixed world seed |
| `IW4L_MINECRAFT_TIME` | Start time in ticks |
| `IW4L_MINECRAFT_WEATHER` | `clear`, `rain` or `thunder` |
| `MINECRAFTOSS_DEBUG_SERVER=1` | Logs server changes and chest placement |
| `MINECRAFTOSS_MESH_CHECK=1` | Checks baked sections against unbaked ones |
| `MINECRAFTOSS_GEN_THREADS`, `MINECRAFTOSS_SECTION_THREADS` | Thread counts for world generation and section building |

### Vanilla reference
To check vanilla behaviour, run `javap -c -p -cp <client.jar> <class>` (Mojang names) on `iw4l-artifacts\minecraft-26.3\client.jar`. Never commit anything from the jar.
