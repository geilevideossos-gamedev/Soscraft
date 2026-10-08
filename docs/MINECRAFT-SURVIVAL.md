# Minecraft survival: state, limitations, testing

State of branch `minecraft-survival` after round 6 (2026-10-08). It is ahead of upstream `main` (f608f85) and 0 behind. Rules and goal: [../CLAUDE.md](../CLAUDE.md).

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
- Water like vanilla: seagrass, kelp, bubble columns and waterlogged blocks hold water, there are no faces between water cells, and water is single-sided apart from the vanilla backward surface.
- XP orbs and the burning overlay with two flames.
- Weather from `IW4L_MINECRAFT_WEATHER`: rain darkens the sky and puts out fire.
- A render error no longer quits the game. Validation and internal GPU errors drop only the broken frame and are written to the game log as `render error ignored`, at most once every 5 s with a count of the ones in between. Running out of GPU memory quits only after 30 times in 10 s, and a lost GPU device still quits; the world is saved on the way out.
- Many explosions at once no longer overflow a render index buffer. Model, spark, effect, decal and glass streams are uploaded every frame, sun shadows of models are drawn only from what the GPU holds, and every MW2 world, model and effect draw in the colour and sun-shadow passes is checked against its bound index buffer before it is sent. A skipped draw is written to the game log as `skipped a ... draw`, naming the pass and stream, for the first 8 times and then every 600th.

### Movement on blocks (MW2 movement with vanilla rules)
- Ground, falls and footsteps. Body size is 0.6 x 1.8 blocks, or 1.5 crouched. Prone keeps MW2's height.
- Swimming: sink, rise, dive, sprint-swim, hop out at ledges. Lava movement.
- Climbing ladders, vines and scaffolding. Sneaking stops at edges.
- Slowdown in berry bushes and cobwebs. Using an item slows you to 0.2.
- A player is never embedded in blocks; anyone inside a block is pushed out.

### Survival and combat
- Hearts, hunger, saturation, exhaustion, regeneration and starvation. MW2 health follows the hearts at 5 per point in every life. MW2's own regeneration is off, and the MW2 scripts can't heal a live player or change its maximum health, so a grenade or bullet costs the same hearts after every respawn.
- Vanilla fall damage, including the blocks that soften falls. Drowning with air bubbles.
- Burning from lava, fire, soul fire, magma and campfires, and putting it out.
- XP levels, the XP bar and vanilla XP from blocks.
- Vanilla attack cooldown, crits, sweeps and knockback. Shield blocking and the totem of undying.
- Vanilla death messages and a Killed By card. A Minecraft death is not counted as an MW2 suicide.
- Dying drops the 36 inventory slots, then the offhand, armour, body and saddle slots, at the death spot like vanilla: from 0.3 below the eye, with vanilla's random spread and a 2 s pickup delay. Crafting grid, bench inputs and the cursor stack go with them. Items with Curse of Vanishing are destroyed (a cursed enchanted book still drops). MW2 guns stay in their hotbar slots. Experience orbs worth 7 points per level, at most 100, drop at the feet, and the XP level starts again at 0. Only a death drops. A match end, MW2 spectating and the suicide MW2 does when a live player picks another team or Spectator in the menu drop nothing, and that team-change suicide keeps the XP level. A player in spectator mode drops nothing and no XP when killed, and keeps the items and XP level, as in vanilla (`ServerPlayer.die` and `restoreFrom`).
- The `keep_inventory` game rule (off by default, saved in `game_rules.dat`) keeps the items, the XP level and its progress through a death, as vanilla does. Hunger and health still reset.
- The world difficulty (Normal for a new world, changed with `/difficulty` and saved in `level.dat`) works like vanilla: the server thread gets it for mob spawning, despawning and behaviour (Peaceful removes hostile mobs), hunger and starvation follow it (Peaceful regenerates and never starves, Easy starves down to 10 health, Normal to 1, Hard to death), and mob hits on the player scale with it: none on Peaceful, half plus one (at most the full hit) on Easy, the full hit on Normal and x1.5 on Hard.
- `/kill` kills like vanilla `genericKill`: the health goes to 0 through armour, Resistance and absorption, a totem of undying doesn't save the player, and the death message is `<name> was killed`.
- When the `show_death_messages` game rule is on (the default), the death message also goes to the chat.
- Game modes: survival, creative, adventure and spectator, with vanilla's abilities for each (`GameType.updatePlayerAbilities`). `/gamemode` switches it, and each player's mode is saved in their player file. A new player starts in the world's default `GameType` from `level.dat` (survival for a new world); `IW4L_MINECRAFT_GAMEMODE` switches to another mode on load for testing. Creative and spectator players are invulnerable like vanilla: no MW2 bullets, blasts, knife or killstreak damage, no mob hits, falls, lava, fire, cactus or berry bushes, drowning, starvation, poison or harming, and they never get hungry. Lava sets them alight for one tick only, so the flames show only while they stand in it. `/kill` still kills them. Mobs don't target or attack them, and spectators pick up no items or XP. In creative and spectator the hearts, armour, hunger, air and XP bar are hidden; the hotbar stays. Creative players can eat on a full stomach from either hand without using up the food.
- Building by game mode, like vanilla (`MultiPlayerGameMode`, `ServerPlayerGameMode.destroyBlock`, `Player.blockActionRestricted`):
  - Creative players break the targeted block at once on a click, and again every 6 ticks while the button is held. Bedrock and other unbreakable blocks break too; swords, tridents and maces break nothing. Creative reach is 5 blocks for blocks and mobs (4.5 and 3 otherwise), and a click into the air has no miss delay.
  - A block a creative player breaks drops no items and no experience, wears no tool and leaves no water where ice was. A filled shulker box still drops with its contents, and chest and furnace contents still spill. MW2 bullets still mine by hardness in creative, but what they break drops nothing either. Explosions still drop like in survival.
  - Creative players use up nothing: placed blocks, doors, scaffolding, planted berries, thrown snowballs, eggs and ender pearls, spawn eggs, food put on a campfire, fire charges and bone meal stay in the hotbar, and flint and steel doesn't wear. The ender pearl cooldown stays.
  - Adventure and spectator players can't build: no hand mining, no placing or planting, and no buckets, flint and steel, fire charges, bone meal or spawn eggs on blocks. While the player is in adventure or spectator, MW2 bullets mine nothing; explosions still blow craters. Adventure players still open doors, chests and buttons, eat, drink, block with a shield, throw and fight. Spectators can't use or hit anything with a Minecraft item.
- MW2 explosives (frag, semtex, C4, claymore, RPG, grenade launcher, killstreak blasts) hurt and push mobs, and a player's kills drop loot and XP. Tactical grenades (concussion, flash and smoke) leave no crater. Every other MW2 blast does, including killstreak bombs (airstrike, stealth bomber) and any other script blast meant as an explosion, splash or artillery hit. A crater is a vanilla explosion on the Minecraft server with power radius / 72 blocks, capped at 8 (a frag about 3.6, TNT 4): blocks break by vanilla blast resistance, so obsidian, water and lava hold and bedrock stays. Each blasted block drops its empty-hand loot with a chance of one in the power (stone gives cobblestone, ores their items), merged into stacks of up to 16, and gives no block XP. TNT a crater reaches is primed, and dropped items in it are hurt and pushed. Craters are saved with the world.
- A stun (concussion) grenade that hurts a mob slows it like a stunned MW2 player: Slowness IV (0.4x walk speed) for 2 s at the 512-unit edge up to 6 s at point blank. Flash grenades don't slow mobs, as they don't slow players.

### Mining and placing
- [upstream] Shooting and knifing breaks blocks by hardness, broken blocks drop items, and grenades blow craters.
- The hand ray follows the MW2 crosshair in every stance. Items that are not blocks are never placed.
- Torches (also soul, copper and redstone) hang on walls when a side face is clicked, and pop off when their support goes.
- Vanilla placement states: beds, banners, signs, heads, bells, lecterns, pots, shulker boxes, ladders, vines (one face per use, added to a vine already there), levers, buttons, lanterns, tripwire hooks, shelf mushrooms and glow lichen (the first looking direction with a support), lily pads and frogspawn (on the water source the crosshair meets). Chests join into double chests.
- Blocks are only placed where vanilla `canSurvive` allows, on the client and again on the server: sugar cane needs water, a waterlogged block or frosted ice beside its ground, cactus a free side, flowers and saplings soil, torches a floor that holds their centre, carpets and cakes something below, rails a rigid top. A refused block stays in the hotbar; one the server refuses comes back as its item.
- Doors, trapdoors, fence gates, levers, buttons and note blocks.
- Scaffolding. Blocks that lose support break and drop. Sand and gravel fall. Sugar cane and cactus columns break one block a tick, corals, cakes and shelf mushrooms go with their support, stalagmites break and stalactites fall.
- Ice broken by hand, bullet or knife over a block in `#blocks_motion` or a liquid turns into a water source, except where water evaporates; blasts and Silk Touch leave none. Packed and blue ice never melt.
- Hand mining with Silk Touch or Fortune applies the enchantment to the drops, and Silk Touch gives no block experience.

### Crafting, inventory, containers
- 2x2 and 3x3 crafting from the 26.3 recipe data, with remainders and real stack sizes.
- The vanilla inventory, hotbar, status bars and station screens, drawn from the runtime packs.
- Vanilla clicks: drag-split, double-click, shift-click, number keys, the drop key (Q) with or without Ctrl, the swap-hands key (F) for the offhand, click outside to drop. Controller slot focus.
- Chests, barrels, furnace, smoker and blast furnace (they cook while closed), brewing stand, stonecutter, smithing table, grindstone, anvil and enchanting table.
- Structure loot tables, shulker boxes that keep their contents, a shared ender chest, and the offhand slot.
- Tooltips with MW2 gun stats and Minecraft tool stats.
- Enchanted items shimmer with the vanilla glint in the hotbar, the inventory, every container and station screen, on the cursor, in the first-person hand (main hand and offhand) and as dropped items on the ground, every copy of a dropped stack included. The world glint is added after the light, so it still shows in the dark, and fog covers it. Placed blocks never glint. As in vanilla, so do enchanted books, enchanted golden apples, bottles o' enchanting, written books, nether stars, end crystals, lodestone compasses and items with a glint override, but not stacks that only hold stored enchantments.

### Items
- Eating, drinking and potions, with the vanilla first-person eat and drink animation, crumbs, sounds and burp. Food only counts when finished; letting go cancels. Glass bottles, buckets (water, lava, powder snow), cauldrons and waterlogging.
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
- The Minecraft keys are their own binding layer beside the MW2 one, so a key can hold one of each. The vanilla defaults are E inventory, Q drop, F swap hands, T chat, / (the physical slash key) command and F5 perspective. On the Minecraft map a Minecraft key takes the key from its MW2 binding, except while a use hint shows and the key is bound to MW2 use (+activate or +usereload) with no Minecraft screen open: then only the MW2 use fires, so E picks up a gun instead of opening the inventory. The swap-hands key keeps its MW2 binding (F: the lethal grenade, frag, semtex, C4 or claymore) while a gun is in hand, and swaps hands like vanilla only with a Minecraft item or an empty hand selected, or a Minecraft screen open. Off the Minecraft map the Minecraft keys do nothing. The minimap key M, the hotbar number keys and the controller D-pad stay fixed.
- The perspective key (F5) cycles the view like vanilla: first person, third person from behind, third person from the front, and back to first person. It works in the world but not while a Minecraft screen is open, and the view is kept for the session, through deaths and matches, but not saved. The third-person camera sits 4 blocks from the eye, behind it or in front of it looking back at the face, and the vanilla eight corner rays pull it in where a block is in the way, so it never looks inside a block. The MW2 character model is drawn at the predicted player position; the crosshair, the MW2 gun and hands, the first-person Minecraft item and the fire overlay are hidden, and the world is culled from the real camera. The block outline, mining and placing still follow the eyes, as in vanilla. Skating, the MW2 death camera, the killcam and the predator missile take the camera over as before.
- The chat key (T) opens the vanilla chat with an empty input line, and the command key (/) opens it with `/` already typed; while the command key has a binding, a typed `/` from any other key, such as Shift+7 on a German keyboard, also opens it with `/`. A pressed chat or command key wins over the typed `/`, so a chat key moved to / opens the chat empty. The key that opens it is never typed into it. Enter sends the line and closes the chat, Esc closes it without sending (and never opens the MW2 pause menu), Up and Down walk the last 100 sent lines, Page Up, Page Down and the mouse wheel (7 lines a notch, 1 with Shift) scroll, and Backspace, Delete, the arrows, Home and End edit the line, with Ctrl for whole words. The line holds at most 256 characters and loses `§` and control characters; a sent line is trimmed and its runs of spaces are collapsed like vanilla. While the chat is open the mouse is free and no MW2 binding, Minecraft key (inventory, drop, swap hands, perspective), hotbar number, mouse wheel slot change, minimap or skate toggle, console key, mining or item use fires. Messages show bottom left above the hotbar at the vanilla size: 320 pixels wide, word-wrapped, at most 100 lines kept, 10 lines shown and 20 while the chat is open, each on a half-black background. A line stays for 9 s and fades out over the last second, as in vanilla. Chat lines go through a small command server (`crates/minecraft_commands`), which stamps the sender and sends `<name> message` back. The chat is cleared when a Minecraft match starts or ends, and its sent-line history is kept for the session.
- Commands typed in the chat with `/` run on the command server with vanilla brigadier parsing, permission levels and messages. The host is always the owner (level 4), with or without an `ops.json` entry, like a singleplayer world with cheats on. A command the player's level doesn't reach answers `Unknown or incomplete command`, exactly as an unknown one does. Parse errors are red, followed by a grey context line with the last 10 characters before the error, the rest red and underlined, and a red `<--[HERE]`. A command that parses but fails, such as `/difficulty` set to the current difficulty, answers with one red line. Players are named by name, `@s`, `@p`, `@a` or `@r`; `@e`, `@n` and selector options like `[limit=1]` are refused.
  - `/help [<command>]` (level 0) lists the commands the player can use, with vanilla usages such as `/gamerule <rule> [<value>]`.
  - `/seed` (level 0) shows the seed as a green `[number]`.
  - `/op <targets>` and `/deop <targets>` (level 3) add or remove `ops.json` entries in the world folder (written at once, vanilla format, level 4). `/deop` on the host removes its entry, but the host stays the owner.
  - `/difficulty [peaceful|easy|normal|hard]` (level 2) shows or sets the difficulty.
  - `/gamerule <rule> [<value>]` (level 2) shows or sets one of the 58 game rules, by its 26.3 id with or without `minecraft:` (`keep_inventory`, not `keepInventory`). Integers are checked against the rule's bounds.
  - `/time set <time>|day|noon|night|midnight`, `/time add <time>`, `/time pause`, `/time resume`, `/time query time` and `/time query gametime` (level 2) use the 26.3 clock syntax. A time takes `d` (24000 ticks), `s` (20) or `t`. A time marker moves the clock forward to the next one, so the day count keeps going. The game time (`/time query gametime` and `Time` in `level.dat`) keeps counting while the clock is paused or `advance_time` is off, and `/time set` and `/time add` don't change it.
  - `/weather clear|rain|thunder [<duration>]` (level 2) sets the server's weather. Without a duration it lasts vanilla's random time: 12000-180000 ticks clear, 12000-24000 rain and 3600-15600 thunder.
  - `/kill [<targets>]` (level 2) kills players; without a target it kills the one who typed it.
  - `/gamemode <gamemode> [<target>]` (level 2) sets `survival`, `creative`, `adventure` or `spectator` for the player who typed it or for the targets. The player who typed it sees `Set own game mode to Creative Mode`; for another player the typer sees `Set <name>'s game mode to ...` and the target gets `Your game mode has been updated to ...` (only while `send_command_feedback` is on). A player already in that mode is left alone without a message. The change is written to the player file at once.
  - Success messages follow the `send_command_feedback` rule, and other operators see them grey and in italics when `log_admin_commands` is on.
- Guns left in a crafting grid are not duplicated. Esc closes a Minecraft screen. The Skate toggle is blocked while a screen is open.
- Options: the Minecraft minimap (M), Show Blood, and an Advanced Video page that replaces the Video page. Options > Controls has a Minecraft row that opens a page with the six Minecraft keys under their vanilla names; click a row and press a key to rebind it. The mouse wheel can't take a Minecraft key, so scrolling while a row listens is ignored. The bindings are saved in `settings.cfg` (now `// IW4L user settings v3`, with lines like `bind F5 mc_perspective`), and `unbind <key>` clears both layers of a key. A v1 or v2 `settings.cfg` from an older build gets the vanilla Minecraft defaults on its first load; after that a Minecraft key left unbound stays unbound.
- F1 to F12 can be bound in the controls menu or with `bind`, and keep their binding in `settings.cfg` after a restart. Keys outside the bind key list, such as the numpad, Insert or the backquote key, still lose their binding on a restart.
- The loading screen stays up like vanilla's until the 7x7 spawn chunks have collision and the camera's section is drawn (0.5 s close delay, 30 s timeout). Every MW2 spawn and respawn lands on the Minecraft spawn, or on the nearest free dry column within 10 blocks when someone stands there. A player in that area waits in place until the ground under it has collision.

## Multiplayer and saving today

Everything here comes from reading the code. No multiplayer session was run.

### MW2 netcode
The MW2 netcode is host-authoritative and goes through the `iw4l-master` QUIC relay. It needs a self-hosted master; there is no direct IP connect and no LAN discovery. Player movement, MW2 models, gun switching and PvP are synced.

### The Minecraft world is single player
- It runs only on the host. `render_anim/src/minecraft_world.rs` returns early without a local `AuthorityWorld`, so a joining client most likely never finishes loading the world.
- The seed is stored with the world (see below).
- Nothing Minecraft goes over the network: blocks, mobs, items, inventories, containers, hunger, time of day and weather are all host-only.
- Chat goes through serde message types made for the network (`Serverbound` chat and command lines without a sender, `Clientbound` system and player messages with translatable text that each client resolves in its own language), but they only travel inside the host for now.
- The server thread knows one player (`PLAYER = 0` in `minecraft_entities.rs`). Chunks stream around the host only.
- Survival state (inventory, containers, hunger) lives in the render crate (`render_anim`), not in the sim or the server thread.
- Commands run on the host only, behind a `CommandWorld` trait that the render crate implements for now. Players are known by the vanilla offline UUID of their name, so `ops.json` already works for friends once they can join: a friend has level 0 until `/op` makes them an operator.

### Saving (one player, overworld)
- The world lives in `iw4l-artifacts\saves\<name>` next to the exe, in the vanilla 26.3 layout: `level.dat`, `session.lock`, `data/minecraft/world_gen_settings.dat` (the seed), `world_clocks.dat` (time of day) and `game_rules.dat` (the 58 vanilla 26.3 game rules, such as `minecraft:keep_inventory`), and `dimensions/minecraft/overworld/{region,entities,poi,data}`. `level.dat`, `world_clocks.dat` and `game_rules.dat` keep a `_old` copy that is read when the file is missing or broken.
- Game rules start at the vanilla defaults. `max_minecart_speed` is left out, because it needs the experimental minecart flag. Rules from other versions or mods in `game_rules.dat` are kept as they are, and values outside a rule's bounds are clamped. `/gamerule` changes them in game, and `game_rules.dat` is written at the same times as `level.dat`. Rules with an effect: `keep_inventory`, `show_death_messages`, `send_command_feedback`, `log_admin_commands`, `advance_time`, `advance_weather`, `spawn_mobs`, `mob_griefing`, `mob_drops`, `block_drops` (also for blocks the player mines or shoots, and their XP), `entity_drops`, `random_tick_speed`, `fire_spread_radius_around_player`, `max_snow_accumulation_height`, `water_source_conversion`, `lava_source_conversion`, `tnt_explodes`, `tnt_explosion_drop_decay`, `fall_damage`, `fire_damage`, `drowning_damage` and `natural_health_regeneration`. The others are stored and saved only. A pre-26 `GameRules` compound in an old `level.dat` is not carried over. A temporary world keeps its rules in memory only.
- `level.dat` holds the difficulty (`difficulty_settings`), `allowCommands=1`, the world's default `GameType` (read and kept, 0 for a new world, not changed by `/gamemode`) and `singleplayer_uuid`, the host's UUID as four ints like vanilla. Worlds from older builds, which wrote `allowCommands=0`, get 1 on their next save. `ops.json` sits in the world folder in the vanilla format (`uuid`, `name`, `level`, `bypassesPlayerLimit`). A temporary world keeps its operators in memory only.
- Each player has `players/data/<uuid>.dat`, named by the vanilla offline UUID of the MW2 player name (`OfflinePlayer:<name>`, UUID v3). It holds `playerGameType`, `previousPlayerGameType` (-1 for none), the vanilla `abilities` compound (`invulnerable`, `flying`, `mayfly`, `instabuild`, `mayBuild`, `flySpeed`, `walkSpeed`) and `DataVersion`. It is written on a background thread at the same times as `level.dat` and right after a game mode change, by merging these keys into the file, so keys it doesn't know survive. It keeps a `_old` copy that is read when the file is missing or broken. On load the abilities follow the saved mode again, keeping the saved `flying`. Renaming the MW2 player starts a new player file, as on a vanilla offline server. A temporary world keeps the mode in memory only.
- `IW4L_MINECRAFT_WORLD` names it (default `world`, letters, digits, `_` and `-` only). `temp` plays a throw-away world as before. A world that another copy of the game holds falls back to a temporary world with a warning.
- `IW4L_MINECRAFT_SEED` only seeds a new world; an existing world keeps its seed and the log warns that the variable is ignored.
- Changed chunks, mobs, dropped items and XP orbs are saved every 5 minutes (6000 ticks) on background threads, when a match ends and when the game quits.
- Not saved yet: containers, player inventory, survival stats, position and weather. The player file holds only the game mode and abilities so far. The kit is given on every load.

## Limitations

### Persistence and multiplayer
- No Minecraft multiplayer, and only the player's game mode and abilities are saved so far (see above).
- Overworld only. There is no Nether or End travel yet (see the roadmap below).
- The chat is local: there are no other players to talk to.
- Commands so far: `/help`, `/seed`, `/op`, `/deop`, `/difficulty`, `/gamerule`, `/time`, `/weather` and `/gamemode`, and `/kill` for players only. There are no mob targets, no `/time rate`, timelines or `/time of`, no `/defaultgamemode`, `/execute`, functions or command blocks, and no tab completion yet. `/time pause` is not saved, so the clock runs again after a restart.

## Roadmap: dimensions

The order is the one in [../CLAUDE.md](../CLAUDE.md): persistent world, crate split, dimension switch and Nether travel, multiplayer, then the End. Everything saved or networked must handle all three dimensions. The 26.3 folder layout is `dimensions/minecraft/<overworld|the_nether|the_end>/`; 26.3 no longer uses `DIM-1` or `DIM1`.

### Nether travel (about 3.5-4.5 days after the crate split)
- **Already there:** portals light, shimmer and break. The Nether terrain generator exists.
- **Runtime dimension switch**, in this order:
  1. save the old level and drop it;
  2. load the new level behind the loading screen;
  3. hold the player until the arrival chunk has collision;
  4. teleport.
  It also needs the one-time loading split from the per-level parts.
- **Portal travel:**
  - the 80-tick timer in the portal, with a cooldown;
  - x8 / ÷8 coordinate scaling and the world-border clamp;
  - the vanilla portal search (POI radius 16 in the Nether, 128 in the Overworld), building a new portal when none is found, and the vanilla exit position and turn.
- **Presentation:** the Nether sky and fog (no clouds), the portal trigger and travel sounds and the purple overlay. A bed explodes in the Nether, and respawning there sends you to the Overworld spawn.
- **Nether mobs:** ghast, piglin, zombified piglin, hoglin, magma cube, blaze, wither skeleton, strider.

### The End (about 4-6 weeks after the dimension switch, mostly the dragon)
- **Already generated by MinecraftOSS:**
  - End terrain and outer islands;
  - obsidian spikes with iron-bar cages and end crystals;
  - small islands and chorus plants;
  - end cities with loot, shulkers and elytra item frames;
  - strongholds (ring placement, a portal room with 12 frames that each get an eye 10% of the time, a silverfish spawner).
- **Missing:**
  - eyes of ender: throwing one toward the nearest stronghold, inserting it into a frame, and filling the portal once all 12 frames have eyes;
  - end portal travel, the End spawn platform at 100, 50, 0, the End sky, and the return through the exit portal;
  - the Ender Dragon fight: the phase machine, multipart hitboxes for MW2 bullets, crystal healing, the boss bar, the exit podium, the egg and the XP;
  - the 20 end gateways at radius 96 and gateway teleports;
  - simulated end crystals, shulkers and their bullets;
  - elytra flight on MW2 movement;
  - chorus fruit teleport.

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
- Third person: the body holds the last MW2 gun while a Minecraft block or item is selected, and no worn armour or burning flames are drawn on it. While the inventory is open the body leaves the world for the inventory's character box. The camera stops on block collision shapes, so glass, fences and snow layers stop it where vanilla's visual shapes would let it through or closer. Underwater fog follows the eyes, not the camera, and MW2 scope overlays are not checked in third person yet.
- An ignored render error loses that frame, so an error that repeats every frame would flicker the screen. bevy also prints `Caught rendering error` to the console, not to the game log.
- A model whose index stream has not been uploaded yet casts no sun shadow for that one frame, and an effect mesh past its 16384-vertex or 24576-index buffer is not drawn that frame.
- Chat gaps: no clickable or hoverable text, underline or per-word italics, no chat settings (opacity, width, scale, line spacing), no clipboard paste or text selection, no message signing or narration, and the sent-line history is not saved. The chat is hidden while dead, like the rest of the Minecraft HUD. The input line scrolls to keep the cursor in view without vanilla's remembered scroll position.
- GUI gaps: no 3D enchanting book, smithing armour stand, recipe book, effects panel or rarity colours. Characters missing from the font show as gaps.
- The enchantment glint is drawn on GUI icons, held items and dropped items only. Worn armour shows no glint, because worn armour itself is not drawn on the MW2 model, and mob armour, items held by mobs, items on campfires and thrown bottles o' enchanting have none either, because they reach the renderer as bare item ids. The glint streaks in the world are sized for a 512-pixel atlas, so their density can differ a little from vanilla. A block item gets one flat glint over its icon instead of one per face, and the glint speed and strength are fixed at the vanilla defaults (0.5 and 0.75).

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
- Death drops are spawned by the render crate on the host for the one local player, not by the server thread. Anvil, enchanting and other bench inputs go back to the inventory when the screen closes and then drop with it. Items that fall into lava or fire burn like any dropped item. A match end still resets the XP level to 0.
- Shield: no knockback to the attacker, axes don't disable it, and MW2 bullets ignore it.
- The totem plays no animation. Every item has the same use speed.
- Witch potions don't reach the player.
- Game modes: creative players don't fly yet, and spectators still collide, touch blocks, see their hotbar and can shoot. In creative, buckets and bottles still empty and fill like in survival (vanilla keeps the full bucket). Adventure ignores the `can_break` item component, honours `can_place_on` only for plain blocks, and a water bottle can still turn dirt into mud. The MW2 bullet rules follow the local player's mode, whoever fired: while the host is in adventure no bullet mines, and while the host is in creative no bullet break drops, bots' bullets included. MW2 effects that scripts apply without damage, such as a flashbang's blinding, may still reach creative players, and MW2 bots still shoot at them (harmlessly).
- MW2 blast damage on mobs is MW2 damage x 0.8 (bullets x 0.2), not vanilla's explosion formula, so a frag kills a 20-health mob it can see anywhere in its reach. MW2 craters are full spheres (the claymore's cone limits only its damage) and appear a server tick after the MW2 explosion. Mobs can't set off claymores. Kills by bot explosives give no XP.
- TNT explosions don't hurt mobs or players; only creeper explosions do.
- MW2 perks and scripts that heal or resize a live player's health (Last Stand and Final Stand revives, the low-health overlay's max reset) do nothing on Minecraft maps; only Minecraft heals. Since MW2's maximum health stays at its spawn value, MW2's blood overlay and pain vision stay on while the hearts are low.

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
| `IW4L_MINECRAFT_WORLD` | Save folder name under `iw4l-artifacts\saves` (default `world`, `temp` for a throw-away world) |
| `IW4L_MINECRAFT_SEED` | Seed of a new world |
| `IW4L_MINECRAFT_TIME` | Start time in ticks |
| `IW4L_MINECRAFT_WEATHER` | `clear`, `rain` or `thunder` |
| `IW4L_MINECRAFT_GAMEMODE` | Game mode to switch to on world load, over the saved one: `survival`, `creative`, `adventure` or `spectator` (or vanilla's ids 0 to 3) |
| `MINECRAFTOSS_DEBUG_SERVER=1` | Logs server changes and chest placement |
| `MINECRAFTOSS_MESH_CHECK=1` | Checks baked sections against unbaked ones |
| `MINECRAFTOSS_GEN_THREADS`, `MINECRAFTOSS_SECTION_THREADS` | Thread counts for world generation and section building |

### Vanilla reference
To check vanilla behaviour, run `javap -c -p -cp <client.jar> <class>` (Mojang names) on `iw4l-artifacts\minecraft-26.3\client.jar`. Never commit anything from the jar.
