# Weapons in Minecraft worlds

This is the design for round 9, track C: the owner's items 11 to 14. MW2 guns, ammo, attachments and grenades become real Minecraft items. They get recipes, two grenade slots, a recipe book and creative tab entries, half damage against players with stronger armour, and a grace period after a totem pops.

The owner can overrule any decision here. Each overrule goes into `docs/round9/decisions.md`.

What was checked, and how:
- Every vanilla item id and item tag below exists in the 26.3 `client.jar` on this PC (`assets/minecraft/items`, `data/minecraft/tags/item`).
- A script compared all 94 fork recipes with every crafting grid in the jar: 827 shaped, 375 shapeless, 33 transmute and 6 dye recipes, with tags expanded and mirrored layouts included. No fork recipe matches a vanilla grid, and no two fork recipes match each other. The special recipes (firework rocket, firework star, banner, shield, book, map, repair, tipped arrow, decorated pot) were checked by hand: none of them accepts a fork grid.
- MW2 facts (fire modes, attachment lists, damage numbers) come from MW2 knowledge and the code (`ui/src/classes/setup.rs`, `frame/src/class_presets.rs`). The owner's game data decides at runtime. Section 18 lists what was not verified.

## 1. What changes for the player

- A gun is a normal Minecraft item. It goes into chests, shulker boxes, bundles, the ender chest and item frames. It drops with Q, falls on death unless `keep_inventory` is on, burns in lava and despawns on the ground. Nothing ever gives it back.
- **MW2 follows the hotbar.** The guns in hotbar slots 1-9 are the guns MW2 holds. A gun in the backpack or in a chest cannot be raised.
- **The class loadout is given once,** on a player's first join to a world: the class guns loaded, three magazines of ammo each, and 32 lethal plus 32 tactical grenades. Respawning gives nothing.
- **Every gun, ammo type, grenade and attachment has exactly one crafting recipe,** and every recipe is in the recipe book from day one.
- **Guns follow Minecraft's progression:** iron makes pistols and SMGs, iron blocks make rifles and shotguns, diamonds make snipers, and the Nether (blaze rods) makes LMGs and launchers. The Barrett and the M240 need netherite.
- **Ammo is an item, one item per round.** A reload takes rounds from the inventory. The loaded magazine is stored on the gun.
- **Attachments are items.** They go onto a gun in a crafting grid. Two identical guns make an akimbo pair. A grindstone strips the attachments.
- **Grenades have two slots of their own,** lethal and tactical, left of the hotbar. Each holds 64. G and Shift+G throw from them.
- **Guns do half damage to players,** and worn armour takes twice vanilla's share of what is left, up to 80 %. LMGs, snipers and FMJ pierce part of that. Damage to mobs does not change.
- **A popped totem also gives 4 seconds of immunity** to bullets and explosions.

## 2. Principles

1. **The inventory is the truth.** A gun, a round of ammo or a grenade exists only as an item stack. MW2's `ps.weapons`, clips, stock and grenade counts follow the items every frame.
   - Today it is the other way round. `sync_weapons` (`minecraft_survival/src/guns.rs:18-55`) rebuilds the inventory from `ps.weapons` every frame and never looks into containers. So every gun moved into a chest is created again: that is the duplication in screenshot 12. This design removes it at its root.
2. **Vanilla first.** Items stack, drop, despawn, burn and move through hoppers like any item. Recipes are vanilla `crafting_shaped` and `crafting_shapeless` JSON made only from items a survival player can get. Every rule that is not vanilla (grenade slots, gun damage to players, armour pierce, the totem's grace, the tactical insertion respawn) says so.
3. **Progression decides the tier.** Each gun sits next to the vanilla tool and armour tier whose materials it needs (section 3).
4. **Balance comes from ammo and armour, not from locking guns.** Gunpowder is the running cost. Armour is the counter to guns, and the late guns and FMJ are the counter to armour. Flash, stun and smoke ignore armour, so a fresh player always has a tool against a geared one.
5. **The host owns everything.** Inventories, crafting, ammo use, grenade counts, damage and the totem grace run on the host for every player, keyed by UUID (players) or MW2 client id (sim).
6. **The world stays a vanilla 26.3 world.** On disk and on the wire, every fork item is a vanilla item with `minecraft:custom_data`. Real vanilla keeps it as a named carrot on a stick or poisonous potato, and our game turns it back into the gun.

## 3. Progression map

| Stage | Vanilla at this stage | New materials | Weapons unlocked |
|---|---|---|---|
| First night | wooden and stone tools | logs, cobblestone, coal, flint, string | none to craft; the first-join loadout carries the player |
| Copper and iron (days 1-3) | copper and iron tools and armour, shield | iron, copper, redstone, gunpowder, leather, paper | gun parts; pistols, machine pistols, SMGs; pistol ammo; red dot, silencer, FMJ, grip, extended mags, rapid fire, tactical knife; frag, semtex, throwing knife, claymore, C4, flash, stun, smoke |
| Mid game | full iron, gold, geodes, enchanting | iron blocks, gold, spyglass, glow ink sacs, pistons, clocks, crying obsidian | assault rifles, shotguns, riot shield, explosive crossbow, ballistic knife; rifle ammo, shells, explosive rounds; ACOG, holographic sight, grenade launcher, underbarrel shotgun; tactical insertion |
| Diamond | diamond tools and armour | diamonds | sniper rifles (not the Barrett); sniper rounds |
| Nether | blaze rods, quartz, glowstone, ancient debris | blaze rods, magma cream, fire charges, netherite scrap | LMGs and launchers; LMG ammo, rockets; thermal scope; M240 (netherite scrap) |
| Late game | netherite, the deep dark, phantoms, strongholds | netherite ingots, sculk sensors, phantom membranes, eyes of ender | Barrett .50cal, Stinger, Javelin; heartbeat sensor |

Cost targets in iron ingots: a pistol about 5 (between an iron sword at 2 and iron leggings at 7), an SMG about 6.5, a rifle or shotgun about 15, a sniper 2 or 3 diamonds and a spyglass, an LMG about 16 and a blaze rod.

## 4. Items

### 4.1 Ids and kinds

Every fork item has its own id in the `iw4l:` namespace. In the game (inventory, menus, recipes, creative, commands) it keeps that id.

| Kind | Id pattern | Max stack | Examples |
|---|---|---|---|
| Gun (riot shield and launchers included) | `iw4l:<MW2 script base>`; T5 guns `iw4l:t5_<base>`; IW5 guns keep their `iw5_` prefix | 1 | `iw4l:m4`, `iw4l:cheytac`, `iw4l:t5_galil`, `iw4l:iw5_ksg` |
| Ammo | `iw4l:<type>` | 64; rockets and explosive rounds 16 | `iw4l:rifle_ammo`, `iw4l:rocket` |
| Grenade | `iw4l:<name>` | 64 | `iw4l:frag_grenade` |
| Attachment | `iw4l:<name>` | 64 | `iw4l:acog_scope` |
| Gun part | `iw4l:gun_barrel`, `iw4l:firing_mechanism` | 64 | |

- Ids use MW2 script names, not session indices. No id and no saved field ever holds a session weapon index. Players see MW2's localized names (`M4A1`, `Intervention`), and creative search and `/give` find the items by name.
- The T5 prefix keeps T5 `spas` apart from IW4 `spas12`.
- An `iw4l:` id gets the safe default in every system keyed by id: it is not fuel, not food, not placeable, not enchantable, not compostable, in no vanilla tag and not wanted by piglins. No vanilla id-keyed rule can fire on a gun by accident.
- **Mobs never pick up fork items.** Zombie loot pickup, allays and foxes refuse them, because mob equipment saves only the id (`server_mobs.rs:728`) and would lose a gun's state.
- The table lives in a new file `crates/minecraft_terrain/src/fork_items.rs` (it must pass `rustfmt --check`). Each row has: id, kind, MW2 weapon key (guns and grenades), lethal or tactical bucket, ammo type, max stack, English fallback name, tier, icon source and creative order.

**The roster is fixed.** It is the 43 IW4 guns (the 42 in the class picker plus the Gold Desert Eagle), the 20 T5 and IW5 guns of the showcase classes, 8 grenades plus the tactical insertion, 7 ammo types, 13 attachments and 2 parts.
- A weapon the content offers outside this table is not an item in Minecraft worlds, so every gun in the game has exactly one recipe.
- If a first-join class names such a gun, the player gets the first table gun of the same MW2 category instead (assault rifle: M4A1, SMG: MP5K, LMG: RPD, sniper: M21 EBR, shotgun: M1014, machine pistol: G18, pistol: USP .45, launcher: AT4-HS). The host logs the name once, so a later round can add it.
- A test asserts that every class-picker and preset weapon has a row.

### 4.2 State on the item

All per-item state lives in `minecraft:custom_data` under the key `iw4l`, in memory, on disk and on the wire alike. Code that copies components keeps it: crafting, shulker boxes, item entities and an anvil rename.

```
gun:  {"minecraft:custom_data": {"iw4l": {"attachments": ["reflex", "fmj"], "clip": 27, "clip_left": 0, "alt_clip": 1}}}
ammo, grenades, attachments, parts: no state, so equal stacks merge
```

- `attachments`: 0 to 2 attachment names as the game's attachment table spells them (`reflex`, `acog`, `silencer`, `rf` for a T5 gun, and so on). `akimbo` counts as one of the two.
- `clip`: rounds in the magazine. `clip_left` is the left gun of an akimbo pair. `alt_clip` is the grenade launcher or underbarrel shotgun.
- **An absent clip means empty.** A crafted gun therefore needs no component on its recipe result and comes out empty, like vanilla's uncharged crossbow. The creative tab, `/give`, the kit and the first-join loadout write a full clip.
- **There is no durability.** Ammo is the running cost. Vanilla durability would bring repair, Mending and Unbreaking questions the owner did not ask for.
- The MW2 weapon is worked out, never stored: the base gun plus its attachments, resolved with `WeaponFamilies::resolve(WeaponSelection::with(family, attachments), LoadoutRules { max_attachments: 2 }, ...)` (`asset_game/src/weapon_families.rs`, made public through a wrapper). It handles IW4 variant defs (`m4_reflex_mp`) and IW5's runtime binding. Results are cached per gun and attachments.
- A session table `GunNames` (item id to weapon index and back) is built once per match where `PreparedWeapons` exists, on the host and on each friend. The relay's weapon digest check already makes both sides agree. `McStack.weapon` (icon and tooltip) comes from this table instead of `weapon_of`.

### 4.3 Saving in the vanilla 26.3 format

The conversion happens only at the save and wire boundary, in two functions `to_disk` and `from_disk` called by `item_nbt::stack_tag` and `stack_of` (`minecraft_terrain/src/item_nbt.rs:74-121`). Both recurse into `minecraft:container` and `minecraft:bundle_contents`.

A gun on disk:
```
{id: "minecraft:carrot_on_a_stick", count: 1, components: {
  "minecraft:custom_data": {iw4l: {id: "iw4l:m4", attachments: ["reflex"], clip: 27}},
  "minecraft:item_name": {translate: "item.iw4l.m4", fallback: "M4A1"}
}}
```
A stackable fork item on disk:
```
{id: "minecraft:poisonous_potato", count: 32, components: {
  "minecraft:custom_data": {iw4l: {id: "iw4l:frag_grenade"}},
  "minecraft:item_name": {translate: "item.iw4l.frag_grenade", fallback: "Frag Grenade"},
  "!minecraft:food": {}, "!minecraft:consumable": {}
}}
```

- **Guns use `minecraft:carrot_on_a_stick`.** It stacks to 1, is in no item tag and appears only as its own recipe's result (checked in the jar). It is damageable, so the disk form never sets `max_stack_size`.
- **Everything else uses `minecraft:poisonous_potato`.** It is in no recipe and no item tag. The `!` removals stop vanilla from eating it. The implementer reads the potato's default components from the catalog, so the removals match what 26.3 gives it. Rockets and explosive rounds also get `"minecraft:max_stack_size": 16`.
- **Netherite guns** (Barrett, M240) also get `"minecraft:damage_resistant": {types: "#minecraft:is_fire"}` on disk and the same rule in the game: like netherite gear, they survive fire and lava on the ground.
- No `item_model`: without a resource pack vanilla would draw the missing-model cube.
- `from_disk` fires only when `custom_data.iw4l.id` is a known fork id. It removes exactly what `to_disk` added (`item_name`, the `!` removals, `iw4l.id`) and keeps everything else, such as an anvil `custom_name` or another pack's `custom_data` keys. An unknown id stays the plain base item, so nothing is destroyed.
- The legacy `iw4:weapon/<index>` ids were never saved and need no migration. Killstreak items (section 5.4) keep that id and are still never saved.
- **Every save path goes through these two functions:** the player file (inventory, equipment, ender chest), containers (`containers.rs`), item frames, ground items (`server_mobs.rs:1619-1690`, which is also the `MobAdd` tag friends receive), piglin inventories, campfires and creative saved hotbars (the gun filter at `minecraft_creative.rs:521-527` goes; killstreak items are still filtered). The implementer greps every `Tag::String(stack.id` and routes it.

**What real vanilla 26.3 does with such a world** (from javap checks, not from running vanilla):
- Guns show as carrots on a stick named "M4A1", and the rest as poisonous potatoes named "Frag Grenade" that cannot be eaten. A carrot on a stick still steers a pig. Nothing else happens.
- Saving in vanilla keeps `custom_data`, so the items become guns again in our game.
- The grenade slots are a fork key that vanilla drops when it saves a player. Grenades still in those slots are lost if the world is saved in vanilla. Put them in a chest first.
- The first-join flag lives in the vanilla `Tags` list, which vanilla keeps, so a vanilla round trip never hands out a second loadout.
- Fork recipe ids are never written to `recipeBook`.

### 4.4 Catalogs, names, commands

- **Stack sizes.** Two small vendored patches add `insert` to `minecraftoss_player::item_catalog::ItemCatalog` and `insert_max_stack` to `minecraftoss_core::item::ItemCatalog`. Without them every fork id defaults to 64, and two guns on the ground would merge. One helper, `fork_items::load`, fills both at the two `Registries::load` sites (host `render_anim/src/minecraft_world.rs:275`, friend `minecraft_remote.rs:54`) and in `minecraft_terrain::crafting::load_recipes`.
- **Names.** `item_icons::item_name` uses `item.<namespace>.<path>`. The fork adds `item.iw4l.*` keys to the language map from MW2's localized names, with the table's English name as the fallback.
- **Commands and kit.** `/give`, `/clear` and `IW4L_MINECRAFT_KIT` accept fork ids (`commands.rs:121`, `minecraft_commands` `registered()`). A given gun comes with a full clip. `/give` also takes the component syntax, e.g. `iw4l:m4[minecraft:custom_data={iw4l:{attachments:["reflex"]}}]`.
- The fork table's hash joins `registry_fingerprint` (`session.rs:701`), so a friend on another build is refused.

### 4.5 Look

- **In the inventory:** a gun shows MW2's kill icon (today's path), with its clip as the vanilla item bar under it: green when full, red when empty.
- **In the hand:** a gun is drawn by MW2 (today's path). A grenade, ammo, attachment or part held in the main hand is drawn flat like a vanilla item and does nothing on right-click. Grenades are thrown only from their slots.
- **On the ground and in item frames:** every fork item is its icon, extruded like vanilla's `item/generated` model. MW2 world models there are later polish.
- **Icons:**
  - Grenades use their MW2 HUD icons.
  - Attachments use MW2's `weapon_attachment_*` materials (`ui/src/classes/icons.rs:237-249`). Rapid fire has none and uses the vanilla repeater sprite.
  - Ammo uses a vanilla sprite tinted at runtime: `iron_nugget` tinted brass (pistol), copper (rifle), grey (LMG), gold (sniper) and red (shells); `fire_charge` tinted green (explosive round); `firework_rocket` (rocket).
  - The barrel uses the vanilla iron bars sprite and the mechanism the tripwire hook sprite.
  - Vanilla sprites are read at runtime from the 26.3 pack and never committed.
- **Tooltip:** the MW2 name, the attachments, "Clip 27/30", "Ammo: Rifle Ammo" and the existing MW2 stat lines (`gun_lines`).

## 5. Guns and MW2

### 5.1 The arsenal seam

`sim/src/voxel.rs` gets a per-client published arsenal next to `set_death_protected` (`voxel.rs:456`), cleared in `deactivate`:

```rust
pub struct GunSpec { pub weapon: u32, pub clip: i32, pub clip_left: i32, pub alt_clip: i32, pub ammo: u8, pub rebind: bool }
pub struct Arsenal { pub guns: Vec<GunSpec>, pub selected: Option<u32>, pub stock: [i32; AMMO_TYPES],
                     pub lethal: Option<(u32, i32)>, pub tactical: Option<(u32, i32)>, pub endless: bool }
pub enum Spent { Rounds { ammo: u8, rounds: i32 }, Grenade { weapon: u32, delta: i32 },
                 Clip { weapon: u32, clip: i32, clip_left: i32, alt_clip: i32 }, Flare { pos: [f32; 3] } }
pub fn set_arsenal(client: u32, arsenal: Arsenal)
pub fn take_spent(client: u32) -> Vec<Spent>
```

`sustain_block_world_arsenal` (`sim/src/item.rs:152`, run at `step.rs:537`) becomes `apply_arsenal`. For each live client on a Minecraft map that has published:

1. **Primaries.** The type-0 weapons in `ps.weapons` become exactly `guns[].weapon`. Missing guns are added with `inventory_add_weapon` without raising them, and others are removed with `take_player_weapon`.
   - Kept: offhands, the alt modes of kept guns, and killstreak weapons.
   - If the held weapon was removed, it switches to `selected`, or to weapon 0 (`none`, which the sim already supports: `item.rs:97`).
2. **Clips.** A gun that is not held gets its clips from its spec. The held gun keeps its live clips, and the sim queues `Clip` when they differ from the spec. With `rebind`, the spec clip is written even into the held gun.
3. **Stock.** The stock of each gun's MW2 ammo index is `min(stock[ammo] - rounds not yet taken, the weapon's maxAmmo)`.
4. **Grenades:** section 9.4.
5. **Endless.** With `endless` set (creative), stock, clips and grenades refill as today and nothing is queued. Clients that never published (bots) keep today's full refill.

The sim queues spending **only at the sites that really spend**, never by comparing numbers:
- A finished reload queues `Rounds` with the rounds moved from stock into the clip. A partial reload queues only the missing rounds.
- A throw or a cook-off (`combat.rs:763/808`) queues `Grenade { delta: -1 }`. Picking a thrown knife up again queues `+1`.
- A planted tactical insertion queues `Flare` (section 9.6).
- Everything else that MW2 does to ammo is overwritten by the published values at the next step: GSC `giveLoadout` at a spawn, Scavenger, One Man Army and ammo pickups. So none of them can create items.

`BLOCK_WORLD_GUNS` (the free Intervention, SPAS-12 and UMP45, `item.rs:146`) is removed, with its `give_weapon_to_ps_akimbo` raise.

**MW2's own weapon drop on death and MW2 weapon pickups are switched off for clients that publish.** Otherwise a dead player's gun would exist twice: once as the Minecraft drop and once as an MW2 pickup.

**The host glue** is one function, `minecraft_survival::guns::settle_arsenal(inventory, mode, spent, names) -> Arsenal`. It runs for the host next to `minecraft_world.rs:1268` and for each friend next to `minecraft_players.rs:1633`:

1. Apply the drained `Spent` list:
   - `Rounds` removes items in vanilla's projectile search order: offhand, main hand, then slots 0-35.
   - `Clip` writes the clips into the bound gun item.
   - `Grenade` changes the grenade slot (a `+1` goes through the pickup rules if the slot is full).
   - `Flare` sets the player's flare respawn.
2. Build the gun list from hotbar slots 0-8 in slot order, each MW2 weapon once.
   - The list stops when `ps.weapons` would overflow: 15 slots, shared with the offhands, alt modes and killstreaks.
   - A gun past that point is inert. Selecting it shows "Too many guns on the hotbar" on the action bar.
3. Count `stock` per ammo type over slots 0-35 and the offhand. Containers and the grenade slots never count. Guns that share an ammo type see the same count.
4. Call `set_arsenal`.

`Follow` (`guns.rs:57-89`) keeps its job. Selecting a hotbar gun raises it through `ui.weapon_request`, and an MW2 switch (alt mode, killstreak, out of ammo) moves the selection. The client's `ps.weapons.contains(target)` check (`net/src/client/runtime.rs:1004-1015`) works unchanged.

`sync_weapons` no longer creates or deletes gun items, except for killstreaks (5.4).

### 5.2 Two copies of the same gun

- `ps.weapons` holds an MW2 weapon only once. With two identical guns on the hotbar, the bound copy is the selected slot if it holds that gun, otherwise the first copy in hotbar order. Only the bound copy's clip is in MW2.
- Selecting the other copy sets `rebind`: the old copy keeps the clip last written to it, and the new copy's clip is loaded without a raise. Two identical guns therefore work as a fast reload. That is MW2's own "switching is faster than reloading", here at the cost of a second gun. A forced raise can come later if it feels cheap.
- A gun put into a chest, dropped or taken off a dead player keeps its loaded rounds, and whoever picks it up gets them.

### 5.3 Moving, dropping, death

- `keep_guns` and the throw arm of `keeps_gun` (`menus.rs:366-424`) go for fork guns. Q drops a gun, and a gun goes into any container cell.
- The offhand ban stays (`offhand.rs:107-116`, the offhand arms of `keeps_gun` and creative `set_slot`): MW2 has no left-hand gun. The riot shield is a gun and follows it. The vanilla shield is unchanged. A grenade may sit in the offhand like any item, but G throws only from the grenade slots.
- **Death:** guns, ammo, attachments and grenades drop like any item, in vanilla order, then the grenade slots after the saddle. `keep_inventory` keeps everything. The death predicate (`minecraft_players.rs:1108-1113`) becomes "killstreak items vanish, everything else follows vanilla".
- **Respawning gives nothing.** GSC `giveLoadout` still runs at every spawn, but `apply_arsenal` overrides it in the same step.
- Dropped guns despawn after 5 minutes. Fire, lava, cactus and the void destroy them, except netherite guns in fire and lava.
- Hoppers, droppers and dispensers move fork items as plain items. A dispenser drops a gun or a grenade; it does not fire it.

### 5.4 Killstreak weapons stay MW2's

Killstreak weapons (`killstreak_*_mp`, `airdrop_*marker*_mp`) are earned per life:
- They stay mirrors with the old `iw4:weapon/<index>` id, minted on the first free hotbar slot by `sync_weapons`, now limited to them.
- They are never saved and vanish when used or on death.
- They cannot go into a container cell, the cursor drop, Q or the offhand. The `keeps_gun` guard is extended to container cells, which closes the same duplication for them.

### 5.5 The class menu after the first join

On Minecraft maps the class menu changes perks and killstreaks only. Its weapon and grenade rows show the note "Guns and grenades are items in Minecraft worlds". One Man Army and Bling change nothing there.

## 6. First-join loadout

- **The flag** is the string `iw4l_loadout_given` in the player file's vanilla `Tags` list. `PlayerData::merged` already carries `Tags`, and the merge owns only that string and keeps any other tags. Vanilla keeps the list, so even a world saved in vanilla does not hand out a second loadout.
- **When:** on the host, for the host and for each friend (the friend tick near `minecraft_players.rs:1625`), on the first frame the player is alive on a Minecraft map with an MW2 class and has no flag. This covers new players, every player file from round 8 and earlier (guns were never saved, so they get them once now) and friends. The flag is per UUID, so it holds across all three dimensions.
- **What is read:** the client's class record `LoadoutSpec` (`sim/src/match_state/loadout.rs:78`): primary, secondary, their attachments, lethal and tactical. Not `ps.weapons`, so the order against `apply_arsenal` does not matter. A variant weapon is split into its base gun and attachments with `WeaponFamilies::describe`.
- **What is given:**
  1. The primary and secondary as gun items, with their attachments and a full clip (both clips for akimbo), into hotbar slots 0 and 1 if free, otherwise the vanilla add order.
  2. Three magazines of reserve per gun as ammo items into the backpack: an M4A1 gets 90 rifle rounds, an Intervention 15 sniper rounds, a Javelin 3 rockets. Guns that share an ammo type add up.
  3. 32 of the class lethal into slot 43 and 32 of the class tactical into slot 44. The fallbacks are frag and flash.
  4. The flag is set. It is saved with the next save, together with the items, so a crash before saving loses both and gives both again.
  - Whatever does not fit drops at the player's feet, like `/give` (`commands::give_stacks`).
- Killstreaks are not part of it. `IW4L_MINECRAFT_KIT` is unchanged and given on its own.
- **Testing:** the track adds vanilla's `/tag <targets> add|remove|list` for players. `/tag @s remove iw4l_loadout_given` and then dying gives the loadout again.

## 7. Guns and their recipes

### 7.1 How the recipes are built

Every gun recipe is a vanilla `crafting_shaped` recipe, `category: "equipment"`, with one `group` per class (`iw4l_pistols`, `iw4l_smgs` and so on) so the recipe book stacks a class behind one button. Shapes are trimmed as vanilla does, so they can sit anywhere in the grid, and mirrored layouts craft too.

Each class has one template. Two cells say what makes the gun different:

- **F, the fire mode, is a redstone part:**
  - `#minecraft:buttons`: semi-auto (one press, one shot)
  - `minecraft:repeater`: full auto
  - `minecraft:clock`: burst
  - `minecraft:lever`: lever or bolt action
  - `minecraft:piston`: pump action
- **X, the signature, is the gun's frame or feature:** `#minecraft:logs` for wooden furniture, `iron_ingot` for steel, `copper_ingot` for light alloy, `gold_ingot` for a heavy round, `leather` for a sling, `bundle` for a big magazine, `glass_pane` for a built-in optic, `redstone_torch` for a built-in red dot, `sugar` for the highest rate of fire, late materials for late guns. T5 and IW5 guns use a dye (their finish) where no material fits.

Within a template no two guns share both F and X, and the templates differ in shape or ingredients, so every grid is unique.

Example, the M4A1:
```json
{"type": "minecraft:crafting_shaped", "category": "equipment", "group": "iw4l_assault_rifles",
 "key": {"B": "iw4l:gun_barrel", "K": "minecraft:iron_block", "F": "minecraft:repeater",
         "M": "iw4l:firing_mechanism", "X": "minecraft:iron_ingot"},
 "pattern": ["BKF", " MX"],
 "result": {"id": "iw4l:m4", "count": 1}}
```

The recipes are our own vanilla-format JSON in `crates/minecraft_terrain/data/fork_recipes.json`, read with `include_str!` and generated from the fork table. No Mojang file is copied.

### 7.2 Gun parts

| Part | Id | Pattern | Key | Yield |
|---|---|---|---|---|
| Gun Barrel | `iw4l:gun_barrel` | `["III"]` | I `minecraft:iron_ingot` | 2 |
| Firing Mechanism | `iw4l:firing_mechanism` | `[" T ", "IRI"]` | T `minecraft:tripwire_hook`, I `minecraft:iron_ingot`, R `minecraft:redstone` | 1 |

A barrel is 1.5 iron. A mechanism is about 2.5 iron (with the hook) and 1 redstone.

### 7.3 Templates

Key in every template: B `iw4l:gun_barrel`, M `iw4l:firing_mechanism`, F fire mode, X signature, I `minecraft:iron_ingot`, K `minecraft:iron_block`, Y `minecraft:spyglass`, D `minecraft:diamond`, Z `minecraft:blaze_rod`, C `minecraft:iron_chain`.

| Template | Used for | Pattern | Cost before F and X |
|---|---|---|---|
| A handgun | pistols, machine pistols | `["BMF", " X "]` | about 4 iron, 1 redstone |
| B SMG | SMGs | `["BMF", " XI"]` | about 5 iron, 1 redstone |
| C rifle | assault rifles | `["BKF", " MX"]` | about 13 iron, 1 redstone |
| D shotgun | shotguns | `["B  ", "BKF", " MX"]` | about 14.5 iron, 1 redstone |
| E sniper | sniper rifles | `[" Y ", "DDX", " MF"]` | 2 diamonds, a spyglass, about 2.5 iron |
| F LMG | LMGs (F is always a repeater) | `["BZK", "CMF", " X "]` | about 14 iron, a blaze rod, an iron chain |
| G launcher | launchers (no F) | `["BBB", "ZMX"]` | about 7 iron, a blaze rod |

### 7.4 Early tier: iron (pistols, machine pistols, SMGs)

| Gun | Id | MW2 weapon | Template | F | X | Ammo |
|---|---|---|---|---|---|---|
| M9 | `iw4l:beretta` | `iw4:weapon/beretta_mp` | A | buttons | `minecraft:copper_ingot` | pistol |
| USP .45 | `iw4l:usp` | `iw4:weapon/usp_mp` | A | buttons | `minecraft:iron_ingot` | pistol |
| .44 Magnum | `iw4l:coltanaconda` | `iw4:weapon/coltanaconda_mp` | A | buttons | `#minecraft:logs` | pistol |
| Desert Eagle | `iw4l:deserteagle` | `iw4:weapon/deserteagle_mp` | A | buttons | `minecraft:gold_ingot` | pistol |
| Gold Desert Eagle | `iw4l:deserteaglegold` | `iw4:weapon/deserteaglegold_mp` | A | buttons | `minecraft:gold_block` | pistol |
| G18 | `iw4l:glock` | `iw4:weapon/glock_mp` | A | repeater | `minecraft:iron_ingot` | pistol |
| PP2000 | `iw4l:pp2000` | `iw4:weapon/pp2000_mp` | A | repeater | `minecraft:copper_ingot` | pistol |
| TMP | `iw4l:tmp` | `iw4:weapon/tmp_mp` | A | repeater | `minecraft:leather` | pistol |
| M93 Raffica | `iw4l:beretta393` | `iw4:weapon/beretta393_mp` | A | clock | `minecraft:iron_ingot` | pistol |
| CZ75 (T5) | `iw4l:t5_cz75` | `t5:weapon/cz75_mp` | A | repeater | `minecraft:brown_dye` | pistol |
| Python (T5) | `iw4l:t5_python` | `t5:weapon/python_mp` | A | buttons | `minecraft:light_gray_dye` | pistol |
| FMG9 (IW5) | `iw4l:iw5_fmg9` | `iw5:weapon/iw5_fmg9_mp` | A | repeater | `minecraft:black_dye` | pistol |
| .44 Magnum (IW5) | `iw4l:iw5_44magnum` | `iw5:weapon/iw5_44magnum_mp` | A | buttons | `minecraft:gray_dye` | pistol |
| MP5K | `iw4l:mp5k` | `iw4:weapon/mp5k_mp` | B | repeater | `minecraft:iron_ingot` | pistol |
| Mini-Uzi | `iw4l:uzi` | `iw4:weapon/uzi_mp` | B | repeater | `minecraft:copper_ingot` | pistol |
| P90 | `iw4l:p90` | `iw4:weapon/p90_mp` | B | repeater | `minecraft:bundle` | pistol |
| Vector | `iw4l:kriss` | `iw4:weapon/kriss_mp` | B | repeater | `minecraft:sugar` | pistol |
| UMP45 | `iw4l:ump45` | `iw4:weapon/ump45_mp` | B | repeater | `minecraft:leather` | pistol |
| Spectre (T5) | `iw4l:t5_spectre` | `t5:weapon/spectre_mp` | B | repeater | `minecraft:black_dye` | pistol |
| AK-74u (T5) | `iw4l:t5_ak74u` | `t5:weapon/ak74u_mp` | B | repeater | `#minecraft:logs` | pistol |
| MP7 (IW5) | `iw4l:iw5_mp7` | `iw5:weapon/iw5_mp7_mp` | B | repeater | `minecraft:gray_dye` | pistol |
| PP90M1 (IW5) | `iw4l:iw5_pp90m1` | `iw5:weapon/iw5_pp90m1_mp` | B | repeater | `minecraft:green_dye` | pistol |

### 7.5 Middle tier: iron blocks (rifles, shotguns, riot shield, T5 specials)

| Gun | Id | MW2 weapon | Template | F | X | Ammo |
|---|---|---|---|---|---|---|
| M4A1 | `iw4l:m4` | `iw4:weapon/m4_mp` | C | repeater | `minecraft:iron_ingot` | rifle |
| FAMAS | `iw4l:famas` | `iw4:weapon/famas_mp` | C | clock | `minecraft:copper_ingot` | rifle |
| SCAR-H | `iw4l:scar` | `iw4:weapon/scar_mp` | C | repeater | `minecraft:gold_ingot` | rifle |
| TAR-21 | `iw4l:tar21` | `iw4:weapon/tar21_mp` | C | repeater | `minecraft:redstone_torch` | rifle |
| FAL | `iw4l:fal` | `iw4:weapon/fal_mp` | C | buttons | `#minecraft:logs` | rifle |
| M16A4 | `iw4l:m16` | `iw4:weapon/m16_mp` | C | clock | `minecraft:iron_ingot` | rifle |
| ACR | `iw4l:masada` | `iw4:weapon/masada_mp` | C | repeater | `minecraft:copper_ingot` | rifle |
| F2000 | `iw4l:fn2000` | `iw4:weapon/fn2000_mp` | C | repeater | `minecraft:glass_pane` | rifle |
| AK-47 | `iw4l:ak47` | `iw4:weapon/ak47_mp` | C | repeater | `#minecraft:logs` | rifle |
| Galil (T5) | `iw4l:t5_galil` | `t5:weapon/galil_mp` | C | repeater | `minecraft:brown_dye` | rifle |
| Commando (T5) | `iw4l:t5_commando` | `t5:weapon/commando_mp` | C | repeater | `minecraft:black_dye` | rifle |
| Type 95 (IW5) | `iw4l:iw5_type95` | `iw5:weapon/iw5_type95_mp` | C | clock | `minecraft:green_dye` | rifle |
| Ranger | `iw4l:ranger` | `iw4:weapon/ranger_mp` | D | buttons | `#minecraft:logs` | shells |
| Model 1887 | `iw4l:model1887` | `iw4:weapon/model1887_mp` | D | lever | `#minecraft:logs` | shells |
| Striker | `iw4l:striker` | `iw4:weapon/striker_mp` | D | buttons | `minecraft:bundle` | shells |
| AA-12 | `iw4l:aa12` | `iw4:weapon/aa12_mp` | D | repeater | `minecraft:iron_ingot` | shells |
| M1014 | `iw4l:m1014` | `iw4:weapon/m1014_mp` | D | buttons | `minecraft:iron_ingot` | shells |
| SPAS-12 | `iw4l:spas12` | `iw4:weapon/spas12_mp` | D | piston | `minecraft:iron_ingot` | shells |
| SPAS-12 (T5) | `iw4l:t5_spas` | `t5:weapon/spas_mp` | D | buttons | `minecraft:black_dye` | shells |
| KSG 12 (IW5) | `iw4l:iw5_ksg` | `iw5:weapon/iw5_ksg_mp` | D | piston | `minecraft:bundle` | shells |

Guns with their own recipe:

| Gun | Id | MW2 weapon | Pattern | Key | Ammo |
|---|---|---|---|---|---|
| Riot Shield | `iw4l:riotshield` | `iw4:weapon/riotshield_mp` | `["IGI", "KSK", " I "]` | I `minecraft:iron_ingot`, G `minecraft:glass_pane` (the viewport), K `minecraft:iron_block`, S `minecraft:shield` | none |
| Explosive Crossbow (T5) | `iw4l:t5_crossbow_explosive` | `t5:weapon/crossbow_explosive_mp` | `["T", "C"]` | T `minecraft:tnt`, C `minecraft:crossbow` | explosive rounds |
| Ballistic Knife (T5) | `iw4l:t5_knife_ballistic` | `t5:weapon/knife_ballistic_mp` | `["S", "P"]` | S `minecraft:iron_sword`, P `minecraft:piston` (the spring) | throwing knives |

The riot shield costs about 23 iron. It stops bullets from the front, so it costs more than a rifle.

### 7.6 Late tier: diamond, Nether, netherite (snipers, LMGs, launchers)

| Gun | Id | MW2 weapon | Template | F | X | Ammo |
|---|---|---|---|---|---|---|
| M21 EBR | `iw4l:m21` | `iw4:weapon/m21_mp` | E | buttons | `minecraft:iron_ingot` | sniper |
| WA2000 | `iw4l:wa2000` | `iw4:weapon/wa2000_mp` | E | buttons | `#minecraft:logs` | sniper |
| Intervention | `iw4l:cheytac` | `iw4:weapon/cheytac_mp` | E | lever | `minecraft:diamond` | sniper |
| Barrett .50cal | `iw4l:barrett` | `iw4:weapon/barrett_mp` | E | buttons | `minecraft:netherite_ingot` | sniper |
| MSR (IW5) | `iw4l:iw5_msr` | `iw5:weapon/iw5_msr_mp` | E | lever | `minecraft:black_dye` | sniper |
| RPD | `iw4l:rpd` | `iw4:weapon/rpd_mp` | F | repeater | `#minecraft:logs` | LMG |
| L86 LSW | `iw4l:sa80` | `iw4:weapon/sa80_mp` | F | repeater | `minecraft:iron_ingot` | LMG |
| MG4 | `iw4l:mg4` | `iw4:weapon/mg4_mp` | F | repeater | `minecraft:copper_ingot` | LMG |
| AUG HBAR | `iw4l:aug` | `iw4:weapon/aug_mp` | F | repeater | `minecraft:glass_pane` | LMG |
| M240 | `iw4l:m240` | `iw4:weapon/m240_mp` | F | repeater | `minecraft:netherite_scrap` | LMG |
| Stoner63 (T5) | `iw4l:t5_stoner63` | `t5:weapon/stoner63_mp` | F | repeater | `minecraft:black_dye` | LMG |
| AT4-HS | `iw4l:at4` | `iw4:weapon/at4_mp` | G | - | `minecraft:iron_ingot` | rocket |
| RPG-7 | `iw4l:rpg` | `iw4:weapon/rpg_mp` | G | - | `#minecraft:logs` | rocket |
| Stinger | `iw4l:stinger` | `iw4:weapon/stinger_mp` | G | - | `minecraft:phantom_membrane` | rocket |
| Javelin | `iw4l:javelin` | `iw4:weapon/javelin_mp` | G | - | `minecraft:ender_eye` | rocket |
| Strela-3 (T5) | `iw4l:t5_strela` | `t5:weapon/strela_mp` | G | - | `minecraft:green_dye` | rocket |
| China Lake (T5) | `iw4l:t5_china_lake` | `t5:weapon/china_lake_mp` | G | - | `minecraft:piston` (pump) | explosive rounds |
| XM25 (IW5) | `iw4l:iw5_xm25` | `iw5:weapon/xm25_mp` | G | - | `minecraft:comparator` (airburst range) | explosive rounds |

- The blaze rod in the LMG and launcher templates is the heat-proof barrel. It gates both behind a Nether fortress.
- The Barrett needs a netherite ingot, the M240 netherite scrap. Both survive fire and lava on the ground.
- The Stinger needs a phantom membrane (it hunts things in the sky) and the Javelin an eye of ender (it seeks its target).

## 8. Ammo

### 8.1 Items and recipes

One item is one round, as one arrow is one arrow. All ammo recipes are shapeless, `category: "equipment"`, group `iw4l_ammo`.

| Ammo | Id | Used by | Recipe (shapeless) | Yield | Stack |
|---|---|---|---|---|---|
| Pistol Ammo | `iw4l:pistol_ammo` | pistols, machine pistols, SMGs | `copper_ingot`, `gunpowder` | 24 | 64 |
| Rifle Ammo | `iw4l:rifle_ammo` | assault rifles | `copper_ingot`, `iron_nugget`, `gunpowder` | 16 | 64 |
| LMG Ammo | `iw4l:lmg_ammo` | LMGs | `copper_ingot`, `copper_ingot`, `iron_nugget`, `gunpowder` | 32 | 64 |
| Shotgun Shells | `iw4l:shotgun_shells` | shotguns, underbarrel shotgun | `paper`, `iron_nugget`, `gunpowder` | 8 | 64 |
| Sniper Rounds | `iw4l:sniper_rounds` | sniper rifles | `iron_ingot`, `copper_ingot`, `gunpowder` | 4 | 64 |
| Explosive Round | `iw4l:explosive_round` | grenade launcher attachment, China Lake, XM25, Explosive Crossbow | `gunpowder`, `gunpowder`, `iron_ingot`, `copper_ingot` | 2 | 16 |
| Rocket | `iw4l:rocket` | AT4-HS, RPG-7, Stinger, Javelin, Strela-3 | `tnt`, `fire_charge`, `iron_ingot` | 2 | 16 |

All ingredients are `minecraft:` items.

- The Ballistic Knife reloads with `iw4l:throwing_knife` items from the offhand and slots 0-35, never from the lethal slot.
- The riot shield uses no ammo.
- Pistols, machine pistols and SMGs share one ammo type, because most of them are 9 mm or .45 in MW2. This keeps the item count low.
- **Why gunpowder.** It comes from creepers, ghasts, witches and chests, and later from creeper farms, so the ammo supply grows with the world. Copper is plentiful and otherwise little used. Brass cases are copper.

**Ammo per kill against mobs** (mob damage is unchanged, section 11.4), for a 20-health mob at close range to the body:
- an M4A1 (30 MW2 points) kills a zombie in 4 shots, so one gunpowder (16 rifle rounds) kills about 4 zombies;
- one gunpowder of pistol ammo (24 rounds) kills about 8;
- a sniper takes 2 rounds, or 1 headshot, at 4 rounds per gunpowder;
- rockets cost TNT and a fire charge each, so they are for players and bosses.

Yields scale with fire rate: SMGs and LMGs get more rounds per gunpowder because they spend them faster.

### 8.2 How ammo is used

- **The clip is on the gun** (`clip`, `clip_left`, `alt_clip`). When a gun joins `ps.weapons`, its item clip goes into MW2. Reloads, firing and switching write it back through `Clip` (section 5.1).
- **The stock is the inventory.** MW2 sees `min(rounds of the gun's ammo type in the offhand and slots 0-35, the weapon's maxAmmo)`. If the inventory holds more, the rest waits. Containers and the grenade slots never count.
- **A reload takes items.** MW2's own reload moves rounds from the stock into the clip, with its own animation and timing. The rounds moved come back as `Rounds`, and the host removes that many items: offhand, main hand, then slots 0-35, as vanilla's projectile search does. Rounds still queued are subtracted from the published stock until they are taken. A partial reload takes only the missing rounds: reloading a 30-round clip at 12 takes 18 items.
- **No ammo:** with an empty clip and no ammo items, MW2 clicks empty as usual. Picking up or crafting ammo makes the stock appear on the next frame.
- **Alt modes:** the grenade launcher uses explosive rounds and the underbarrel shotgun uses shells, each with its own `alt_clip`. They are reached with MW2's attachment-toggle key, as today.
- **Akimbo:** both clips use the same ammo type.
- **Creative:** the stock is always full and reloads take no items, like arrows in creative. The clip still empties and reloads, so the gun feels the same.
- **Bots and MW2 maps** are unchanged. Nothing here runs off Minecraft maps.
- **HUD:** MW2's ammo counter shows `clip / stock`, so both numbers are real items.

## 9. Attachments

### 9.1 Items and recipes

All attachment recipes are shaped, `category: "equipment"`, group `iw4l_attachments`, yield 1, stack 64. All ingredients are `minecraft:` items except the barrel.

| Attachment | Id | MW2 names (T5 and IW5 aliases) | Pattern | Key | Tier |
|---|---|---|---|---|---|
| Red Dot Sight | `iw4l:red_dot_sight` | reflex, elbit | `[" T ", "NGN"]` | T `redstone_torch`, N `iron_nugget`, G `glass_pane` | early |
| Silencer | `iw4l:silencer` | silencer, silencer02, silencer03 | `["IWI"]` | I `iron_ingot`, W `#minecraft:wool` (wool damps vibrations in vanilla) | early |
| FMJ | `iw4l:fmj` | fmj | `[" C ", "CIC", " C "]` | C `copper_ingot` (the jacket), I `iron_ingot` | early |
| Extended Mags | `iw4l:extended_mags` | xmags, extclip, dualclip | `["I", "B", "I"]` | I `iron_ingot`, B `bundle` | early |
| Grip | `iw4l:grip` | grip | `["I", "S", "L"]` | I `iron_ingot`, S `stick`, L `leather` | early |
| Rapid Fire | `iw4l:rapid_fire` | rof, rf, auto | `["SRS"]` | S `sugar`, R `repeater` | early |
| Tactical Knife | `iw4l:tactical_knife` | tactical | `["I", "S"]` | I `iron_ingot`, S `stick` | early |
| Holographic Sight | `iw4l:holographic_sight` | eotech | `[" S ", "NGN"]` | S `glow_ink_sac`, N `iron_nugget`, G `glass_pane` | middle |
| ACOG Scope | `iw4l:acog_scope` | acog, hamrhybrid, hybrid | `["NYN"]` | Y `spyglass`, N `iron_nugget` | middle |
| Grenade Launcher | `iw4l:grenade_launcher` | gl, gp | `["IBI", " T "]` | I `iron_ingot`, B `iw4l:gun_barrel`, T `tripwire_hook` | middle |
| Underbarrel Shotgun | `iw4l:underbarrel_shotgun` | shotgun, mk | `["IBI", " P "]` | I `iron_ingot`, B `iw4l:gun_barrel`, P `piston` | middle |
| Thermal Scope | `iw4l:thermal_scope` | thermal, ir | `["NYN", " M "]` | Y `spyglass`, N `iron_nugget`, M `magma_cream` (heat) | late |
| Heartbeat Sensor | `iw4l:heartbeat_sensor` | heartbeat | `["NSN", "IRI"]` | S `sculk_sensor` (it senses), N `iron_nugget`, I `iron_ingot`, R `redstone` | late |

- Akimbo has no item: two identical guns make an akimbo pair (9.2).
- An attachment name in the content that maps to none of these items cannot be crafted (a T5 flamethrower, for example). A gun can still carry it if it came from the first-join loadout or `/give` with components. A content test lists such names.
- FMJ also pierces armour against players (section 11.3), which is why it is worth a slot in a world without wallbangs.

### 9.2 Putting attachments on, akimbo, taking them off

These are fork special recipes, like vanilla's shield decoration. They work in any crafting grid, the inventory's 2x2 included, and run in `minecraft_survival` where the crafting result is computed. They are not listed in the recipe book. Each attachment's tooltip says "Craft together with a gun to attach it".

- **Attach:** exactly one gun plus 1 or 2 attachment items, in any cells. The result is the same gun with the attachments added and every other component kept: the clip (cut to the new clip size, with the extra rounds given back as ammo items), the custom name and the other attachments.
  - The result exists only if the gun ends with at most 2 attachments and `WeaponFamilies::resolve` accepts the combination. That uses the owner's own attachment and combination tables, so no list is kept by hand. Two sights or two alt modes have no MW2 def and give no result, as vanilla shows nothing for an invalid grid.
  - Two attachments need no Bling: items do not depend on the class, and the crafting cost replaces the perk slot.
- **Akimbo:** two copies of the same gun with the same attachments give one gun with `akimbo` added, if the gun offers akimbo. Its `clip` and `clip_left` come from the two guns.
- **Split an akimbo pair:** an akimbo gun alone in the grid gives one gun without `akimbo`. The second gun is left in the same cell as the crafting remainder, with `clip_left` as its clip. This is vanilla's remainder mechanism (a milk bucket leaves a bucket).
- **Strip at the grindstone:** a gun in the grindstone comes out with no attachments (akimbo excepted), keeping its clip, cut to the bare clip size. The attachments are lost, and the grindstone gives a little XP, as it does for enchantments. Swapping an attachment therefore costs a new one, which keeps attachments a sink like enchanted books.

## 10. Grenades and the grenade slots

### 10.1 Items and recipes

All grenade recipes are shapeless, `category: "equipment"`, groups `iw4l_lethal` and `iw4l_tactical`. All grenades stack to 64. All ingredients are `minecraft:` items.

| Grenade | Id | MW2 weapon | Slot | Recipe (shapeless) | Yield |
|---|---|---|---|---|---|
| Frag Grenade | `iw4l:frag_grenade` | `iw4:weapon/frag_grenade_mp` | lethal | `iron_ingot`, `iron_nugget`, `gunpowder`, `gunpowder` | 2 |
| Semtex | `iw4l:semtex` | `iw4:weapon/semtex_mp` | lethal | `slime_ball` (sticky), `iron_nugget`, `gunpowder`, `gunpowder` | 2 |
| Throwing Knife | `iw4l:throwing_knife` | `iw4:weapon/throwingknife_mp` | lethal | `iron_ingot`, `flint`, `stick` | 2 |
| Claymore | `iw4l:claymore` | `iw4:weapon/claymore_mp` | lethal | `tripwire_hook` (its tripwire), `iron_ingot`, `gunpowder`, `gunpowder` | 2 |
| C4 | `iw4l:c4` | `iw4:weapon/c4_mp` | lethal | `tnt`, `redstone` (the detonator), `slime_ball` | 2 |
| Flash Grenade | `iw4l:flash_grenade` | `iw4:weapon/flash_grenade_mp` | tactical | `gunpowder`, `glowstone_dust` (the flash), `iron_nugget` | 2 |
| Stun Grenade | `iw4l:concussion_grenade` | `iw4:weapon/concussion_grenade_mp` | tactical | `gunpowder`, `fermented_spider_eye` (vanilla's slowness reagent), `iron_nugget` | 2 |
| Smoke Grenade | `iw4l:smoke_grenade` | `iw4:weapon/smoke_grenade_mp` | tactical | `#minecraft:coals`, `paper`, `iron_nugget` | 2 |
| Tactical Insertion | `iw4l:tactical_insertion` | `iw4:weapon/flare_mp` | tactical | `iron_ingot`, `redstone_torch` (the flare), `crying_obsidian` (the respawn link, as in a respawn anchor) | 1 |

Each explosive costs about one gunpowder, a fifth of TNT, which matches a frag's crater (power 3.6 against TNT's 4).

### 10.2 Slots

- **Two new inventory slots: 43 lethal and 44 tactical,** after body 41 and saddle 42. The vendored `Inventory` grows from 43 to 45 slots (`inventory.rs:104` and `add_item`'s `seen`).
- `PLAYER_SLOTS` and `MC_INVENTORY_SLOTS` become 45 in `minecraft_net/messages.rs:392`, `frame/minecraft_ui.rs:10`, `menus.rs:28` and `clear.rs:11`. `minecraft_net::PROTOCOL` goes up by one.
- **Placement** (`guns::fits(slot, stack)`, used by every click path, number keys, drag-spreading and creative `set_slot`):
  - Slot 43 takes only lethal grenades and slot 44 only tactical ones, up to 64. Anything else is refused, as an armour slot refuses a stick.
  - Shift-click on a grenade in slots 0-35 moves it to its slot first, when that slot is empty or holds the same grenade. Shift-click out of 43 or 44 goes to 9-35, then 0-8.
- **Pickup:** a picked-up grenade tops up a grenade slot that holds the same grenade, right after vanilla's main-hand and offhand merge. It also fills an empty grenade slot of its kind, because the slot exists only for grenades. Otherwise it goes to the backpack.
- **Refill:** when throwing empties a grenade slot, it refills from the backpack with the same grenade, as a bow finds arrows anywhere. It never switches to another type on its own.

### 10.3 Layout

- **HUD:** two frames with vanilla's `hud/hotbar_offhand_left` sprite (29x24), left of the offhand frame. Lethal is at x = cx-91-29-29 and tactical at cx-91-29-58, both at y = h-23, with the item at +3, h-19 and its count.
  - They are drawn whenever the hotbar shows, even empty, at fixed places, so they do not jump.
  - They stay on the left even with the main arm on the left, because the MW2 weapon bar starts about 8 px right of the hotbar at 16:9 (screenshot 11).
  - The MW2 weapon bar's grenade icons stay, and their counts now come from the slots.
- **Inventory screen:** slot 43 at (134, 62) and slot 44 at (152, 62), drawn with `container/slot`. That is the empty panel right of the recipe book button at (104, 61) and of the offhand at (77, 62). Empty slots show the tooltip "Lethal" or "Tactical".
- **Creative inventory tab:** about (127, 20) and (145, 20), checked against `tab_inventory.png` in the implementation.

### 10.4 Keys and throwing

- G throws from slot 43 and Shift+G from slot 44. The keys, settings v6 and the client runtime (`net/src/client/runtime.rs:1133-1142`) are unchanged.
- **The slots decide the offhands.** The arsenal's `lethal` and `tactical` (weapon and count) make that weapon the only offhand of its bucket. This reuses the same-bucket replace code factored out of `apply_give_offhand` (`step.rs:1550-1591`): it sets `offhand_primary` or `offhand_secondary` and `meta.loadout.lethal` or `tactical`, so the friend's replicated loadout follows unchanged. The ledger and ps ammo are the count minus throws not yet taken.
- **An empty slot** keeps the last weapon with no ammo, so the MW2 icon stays and placed C4 can still be detonated.
- **A throw or a cook-off spends one,** a frag that blows up in the hand included. Picking a thrown knife up again gives one back. Creative throws spend nothing.
- **Placed claymores and C4** are MW2 entities. They are not saved and vanish when the host quits; the item was already spent.
- Holding a grenade item in the hotbar and pressing G changes nothing: only the slots are thrown.

### 10.5 Start, saving, death

- **A new player starts with 32 + 32** as part of the first-join loadout (section 6). Respawning gives none.
- **Saving:** slots 43 and 44 are saved under a fork top-level key `iw4l_grenades` in the player file, a `slot_list` with `Slot:0b` (lethal) and `Slot:1b` (tactical), in the disk form. It never goes inside `equipment`, because vanilla's equipment codec would refuse an unknown key there. The key joins `OWNED` in `player_nbt.rs`, so emptied slots leave the file.
- **Death:** the grenade slots drop after the saddle, unless `keep_inventory` is on (`death.rs` `EQUIPMENT_DROP_ORDER`).
- `/clear` covers them.

### 10.6 Tactical insertion

The class menu offers it, so it is an item with a recipe. In a Minecraft world it does what it does in MW2, mapped onto Minecraft's respawn:
- Shift+G plants MW2's flare. The sim queues `Flare { pos }`.
- The host stores it as the player's flare respawn (level and position). The next respawn of that player is at the flare, ahead of a bed or respawn anchor. That respawn uses it up, and so does the flare being destroyed or a second flare being planted.
- It is not saved, like the MW2 flare entity. This is a fork rule.

## 11. Damage to players (item 13)

### 11.1 The rule

- Every MW2 weapon hit on a **player** in a Minecraft world is halved, then reduced by Minecraft armour at twice vanilla's armour share. Armour and Protection enchantments together never block more than 80 %, vanilla's own armour ceiling. LMGs, snipers and FMJ pierce part of that protection.
- This covers bullets, grenades, launchers, GSC blasts, killstreaks, the knife and a player's own explosives.
- Damage to mobs is exactly what it is today (`VoxelEvent::MobShot` and `MobBlast` are untouched). Minecraft and mob damage to players is unchanged.
- **Where:** `sim::script::host::players::player_damage`, before GSC `Callback_PlayerDamage`. It is the one point every MW2 player hit passes, and it runs on the host only. It applies when `voxel::active()`, the hit is not a Minecraft hit (`!runtime.minecraft_hit`) and the means is a gun means:
  - bullets: MOD_PISTOL_BULLET, MOD_RIFLE_BULLET, MOD_HEAD_SHOT, MOD_EXPLOSIVE_BULLET;
  - explosions: MOD_GRENADE, MOD_GRENADE_SPLASH, MOD_PROJECTILE, MOD_PROJECTILE_SPLASH, MOD_EXPLOSIVE, MOD_ARTILLERY;
  - melee: MOD_MELEE, MOD_IMPACT.
- The location multiplier for headshots is applied before (`damage.rs:907-915`), so headshots still matter. GSC perks (Stopping Power, Danger Close) multiply the result, and GSC keeps its minimum of 1.
- **Inputs:** each frame, after settle, the host mirrors each player's armour into the sim with `set_gun_guard(client, GunGuard { armor, toughness, epf_bullet, epf_blast, epf_melee, resistance, totem_grace })`, next to `set_death_protected`, for itself (`minecraft_world.rs:1391`) and each friend (`minecraft_players.rs:1638`). The EPF values come from the existing enchantment code with `minecraft:arrow` for bullets (`#is_projectile`), `minecraft:player_explosion` for blasts and `minecraft:player_attack` for the knife.
- **Pierce** comes from a `weapon index -> pierce` table pushed once per match (`sim::voxel::set_pierce_table`), built from the gun's class and its `fmj` attachment.

### 11.2 Formula

Today MW2 damage ignores Minecraft armour entirely (`survival.rs:317-335` applies only absorption). So "twice as much as now" has no literal value. It is read as twice vanilla's armour reduction, with vanilla's 80 % ceiling, so armour never makes a player bulletproof.

```
s       = max_health / 20                                   5 MW2 points per Minecraft health point
d       = D / s * GUN_PLAYER_SCALE                          half damage, in health points
h       = clamp(A - d / (2 + T/4), 0.2*A, 20) / 25          vanilla CombatRules on the halved hit
r_arm   = min(GUN_ARMOR_FACTOR * h, GUN_PROTECTION_CAP)     twice vanilla, capped
r_ench  = min(EPF, 20) * 0.04                               Protection, Projectile or Blast Protection
kept    = max(1 - GUN_PROTECTION_CAP, (1 - r_arm) * (1 - r_ench))
kept    = 1 - (1 - kept) * (1 - pierce)                     pierce cuts the protection
kept   *= 1 - min(0.2 * resistance_level, 1)                vanilla Resistance (level V blocks everything)
D'      = kept == 0 ? 0 : max(1, round(d * kept * s))       back to MW2 points
```

Constants, one place each in the sim: `GUN_PLAYER_SCALE = 0.5`, `GUN_ARMOR_FACTOR = 2.0`, `GUN_PROTECTION_CAP = 0.8`, `PIERCE_LMG = 0.25`, `PIERCE_SNIPER = 0.5`, `PIERCE_FMJ = 0.25` (added, at most 0.75). A retune is a one-line change.

### 11.3 Results

MW2 points per hit (100 = full health = 10 hearts, so 10 points = 1 heart), with the hits to kill from full health in brackets. No enchantments. Copper armour is 10 points (2, 4, 3, 1 in the 26.3 catalog). Gun damage values are MW2 base damage from memory; the real values come from the owner's data (section 18).

| Hit (MW2 points) | Pierce | None | Leather | Copper | Gold | Chain | Iron | Diamond | Netherite |
|---|---|---|---|---|---|---|---|---|---|
| M4A1 body, close (30) | 0 | 15 (7) | 8 (13) | 5 (20) | 4 (25) | 3 (34) | 3 (34) | 3 (34) | 3 (34) |
| M4A1 + FMJ (30) | 0.25 | 15 (7) | 10 (10) | 7 (15) | 6 (17) | 6 (17) | 6 (17) | 6 (17) | 6 (17) |
| USP .45 close (40) | 0 | 20 (5) | 12 (9) | 7 (15) | 6 (17) | 4 (25) | 4 (25) | 4 (25) | 4 (25) |
| Desert Eagle (50) | 0 | 25 (4) | 16 (7) | 10 (10) | 8 (13) | 6 (17) | 5 (20) | 5 (20) | 5 (20) |
| LMG (40) | 0.25 | 20 (5) | 14 (8) | 10 (10) | 9 (12) | 8 (13) | 8 (13) | 8 (13) | 8 (13) |
| LMG + FMJ (40) | 0.5 | 20 (5) | 16 (7) | 14 (8) | 13 (8) | 12 (9) | 12 (9) | 12 (9) | 12 (9) |
| Intervention body (98) | 0.5 | 49 (3) | 45 (3) | 39 (3) | 37 (3) | 35 (3) | 29 (4) | 29 (4) | 29 (4) |
| Intervention + FMJ (98) | 0.75 | 49 (3) | 47 (3) | 44 (3) | 43 (3) | 42 (3) | 39 (3) | 39 (3) | 39 (3) |
| Frag, centre (130) | 0 | 65 (2) | 58 (2) | 47 (3) | 42 (3) | 36 (3) | 21 (5) | 13 (8) | 13 (8) |
| Rocket, direct (150) | 0 | 75 (2) | 67 (2) | 60 (2) | 54 (2) | 48 (3) | 30 (4) | 15 (7) | 15 (7) |

With Protection IV on all four pieces, an M4A1 hit on an unarmoured-equivalent player drops from 15 to 5 and a frag from 65 to 23. Blast Protection IV on four pieces takes every frag to 13.

What this means in play:
- **Unarmoured players** take about twice MW2's hits: 7 M4A1 body shots instead of 4, about half a second. An ambush still works, and a frag no longer kills from full health.
- **Leather, copper and gold help a lot.** From chainmail up, armour reaches the 80 % ceiling against light rounds, so iron, diamond and netherite are equal against an M4A1 (34 hits, more than one magazine, about 2.6 s of continuous hits). Vanilla iron-sword PvP against iron armour takes 4-5 s, so guns stay stronger than swords but are not instant.
- **Gun choice matters against armour.** FMJ halves an M4A1's hits to 17, an LMG needs 13 and a sniper 3-4. The late guns are the late answer.
- **Diamond and netherite pull ahead of iron against explosions** (a frag does 13 against 21) and against Minecraft melee and mobs, where armour is unchanged.
- **Shotguns** still one-shot an unarmoured player up close, as in MW2. Each pellet is armoured on its own, so armour hurts them most.
- **The knife** is halved and armoured like the rest. MW2's lunge (135) takes 2 hits unarmoured.

### 11.4 Not covered

- Guns do not wear armour down: the fork models no armour wear for players.
- Bots and MW2 maps: nothing changes off Minecraft maps.
- With pvp off, gun hits on players stay refused earlier (`damage.rs:903`).

## 12. Totem of undying (item 14)

- When a totem pops, from MW2 or from Minecraft damage, the player is **immune to bullets and explosions for 4 seconds:**
  - MW2 bullet means;
  - MW2 explosion means (grenades, launchers, C4, claymores, killstreaks, their own included);
  - Minecraft `#minecraft:is_explosion` damage (explosion, player_explosion, bad_respawn_point, fireworks): TNT, creepers, end crystals, beds and anchors.
- The pop itself is unchanged: health 1, Regeneration II, Absorption II, Fire Resistance, the rising totem and the particles.
- **Not covered:** the MW2 knife and impacts, Minecraft melee, arrows, fire, falls, the void and `/kill`.
- Explosion knockback still pushes the player, as vanilla pushes an entity whose hurt is refused.
- Refused hits change no health and show no hit marker.
- **Two clocks, one rule:**
  - Sim side: `spend_death_protection(client, tick)` (`voxel.rs:465`) also records `tick + 80` (80 sim ticks of 50 ms, `MATCH_TICK_MS`). `finish_damage` gains the tick and passes it on (callers: `natives/player.rs:485` and the two `height_bands` tests). So the remaining pellets of the same shotgun blast and grenades in the same tick are refused at once.
  - Minecraft side: `Survival::protect` sets `totem_grace_until = tick + 80`, and `hurt` refuses explosions while it runs. It is mirrored to the sim through `GunGuard.totem_grace`. `respawn` clears it.
  - Under `/tick freeze` the Minecraft clock stops while the sim clock runs on. That is accepted.
- This is a fork rule: vanilla has only the hurt cooldown. It goes into `docs/round9/decisions.md`.

## 13. Recipe book and creative

### 13.1 Recipe book

The fork has no recipe book screen yet (`MINECRAFT-SURVIVAL.md:398`), so this track builds vanilla's, for the inventory's 2x2 grid and the crafting table.
- The green book button sits at (104, 61) in the inventory and at vanilla's place in the crafting table screen. The panel opens to the left with the tabs Search, Equipment, Building, Misc and Redstone, 20 recipes a page, the search box and the "Showing craftable" toggle.
- Recipes whose ingredients are missing have the red background. Clicking a recipe places the ingredients, or shows red ghost items for the missing ones. Shift-click places as many as fit. Right-click on a group button opens its alternatives.
- Placing goes through a new `MenuAction::PlaceRecipe { recipe, all }` that the host applies, for the host and for friends.
- **Every fork recipe is always known,** with no unlock toast, so the book shows every gun from the first day (the owner's request). Fork recipe ids are never written to the save.
- **Vanilla recipes** unlock the vanilla way (`auto_unlock_rules`). Their unlocks, `toBeDisplayed` and the per-tab open and filter flags are saved in vanilla's `recipeBook` compound, which today is only carried over.
- **Where fork recipes appear:** all are `category: "equipment"`, so they are in the Equipment tab after the vanilla ones. The groups keep the tab short: `iw4l_parts`, `iw4l_pistols`, `iw4l_machine_pistols`, `iw4l_smgs`, `iw4l_assault_rifles`, `iw4l_shotguns`, `iw4l_specials` (riot shield, crossbow, ballistic knife), `iw4l_snipers`, `iw4l_lmgs`, `iw4l_launchers`, `iw4l_ammo`, `iw4l_lethal`, `iw4l_tactical`, `iw4l_attachments`.
- **Loading:** the vendored `RecipeBook` gets `with_extra(recipes)`, which reuses `parse_recipe` and gives fork recipes display indices after vanilla's. It is called in `load_recipes` before `with_item_catalog`, on the host and on the friend, who needs it only for the display.
- The attach, akimbo and split special recipes are not listed, like vanilla's special recipes.

### 13.2 Creative

- All fork items go at the end of vanilla's **Combat** tab, appended after `creative_tabs(&jar)` (`minecraft_creative.rs:266`). All 14 tab positions are taken and vanilla has no tab paging.
- Order: guns by class in tier order (pistols, machine pistols, SMGs, assault rifles, shotguns, specials, snipers, LMGs, launchers), then parts, attachments, ammo, lethal grenades and tactical grenades. Guns are bare and fully loaded.
- Search finds them all by their MW2 name.

## 14. Multiplayer

- **The host runs everything:** every player's inventory, grenade slots, crafting (special recipes and the grindstone included), ammo use, first-join flag, flare respawn, damage and totem grace. Player state is keyed by UUID, sim state by MW2 client id. The sim's `apply_arsenal` runs for each friend client.
- **Wire form:** slots, menu cells and ground items carry the disk form of section 4.3. The gun special case in `views::slot_nbt` and `view_stack` (`views.rs:150-173`) goes; they just call `stack_tag` and `stack_of`. Ground items (`MobAdd` tags) already come from the same serializer, so friends now see dropped guns. The friend turns the carrier back with the same table, so clips, attachments and names show in the friend's tooltips.
- **Version checks:** `PlayerView` carries 45 slots and `MenuAction::PlaceRecipe` is new. `PROTOCOL` goes up once for both. The fork table and recipe hash joins `registry_fingerprint`, so a friend on another build is refused.
- **A friend's clicks** on slots 43 and 44 pass `valid_slot` (below 45).
- **A friend's MW2 side:** `ps.weapons`, clips, stock and `meta.loadout` reach them by snapshot as today, so their weapon bar, `Follow` and G and Shift+G work unchanged. The friend's prediction takes no items; between snapshots its predicted stock may run ahead after a reload, and the host's snapshot corrects it, as it does today for clips.
- **Damage:** each side's armour and totem protect them against the other's guns, through the same `player_damage` rule.
- **The friend's own data:** icons and names come from the friend's `PreparedWeapons`, the recipe book display from the friend's own jar plus the fork recipes.

## 15. Saving summary

| What | Where | Vanilla 26.3 sees |
|---|---|---|
| Guns, ammo, grenades, attachments and parts in the inventory, ender chest, chests, shulker boxes, bundles, item frames and on the ground | the usual vanilla places, in the disk form | a named carrot on a stick or poisonous potato; kept when vanilla saves |
| Clip and attachments | `custom_data.iw4l` on the gun | kept |
| Grenade slots | top-level `iw4l_grenades` in the player file | ignored; lost if vanilla re-saves that player |
| Loadout flag | `Tags: ["iw4l_loadout_given"]` | kept |
| Vanilla recipe unlocks | `recipeBook` | kept |
| Fork recipe unlocks | not saved (always known) | - |
| Killstreak items, placed claymores and C4, the flare respawn | not saved | - |

## 16. Implementation map

Suggested commit groups for the track brief, in order. Each is one commit or a few small ones.

1. **The fork item table:** catalog stack sizes (two vendored patches), names, `/give`, `/clear`, the kit, icons, `/tag`.
2. **Disk and wire conversion** in `item_nbt` and every save path; friends' views and ground items.
3. **MW2 follows the hotbar:** the `Arsenal` seam, `apply_arsenal`, `BLOCK_WORLD_GUNS` removed, killstreak-only mirror, MW2's weapon drop and pickups off on Minecraft maps.
4. **Guns drop on death and with Q and go into chests like any item.**
5. **Ammo:** the clip on the item, the stock from the inventory, spending at the reload site, creative.
6. **The first-join loadout** with the `Tags` flag.
7. **Recipes:** `with_extra`, parts, guns, ammo, grenades and attachment items.
8. **Attach, akimbo and split** special recipes; the grindstone strip.
9. **Grenade slots:** 45 slots, protocol, sim counts, spending, refill, HUD and screens, saving; the tactical insertion respawn.
10. **The recipe book** screen, `PlaceRecipe` and vanilla `recipeBook` saving.
11. **The creative Combat tab entries.**
12. **Gun damage to players** through armour, enchantments and pierce.
13. **The totem's 4 s grace.**
14. **Docs:** `MINECRAFT-SURVIVAL.md` (lines 59, 134, 350, 374 and 398 change), `MULTIPLAYER.md` sections 10.3, 13 and 20, `docs/round9/decisions.md`, the track log.

This is a large track. If it has to split: C1 is 1-6 (guns as items), C2 is 7-11 (crafting, grenade slots, book, creative), C3 is 12-13 (balance and totem). C3 touches only the damage path, so it can run in parallel with C1.

Crates touched: sim, minecraft_survival, minecraft_terrain, minecraft_net, minecraft_commands, hud, frame, render_anim, asset_game (public resolve and describe), ui (class menu note), and the vendored minecraftoss player and core crates.

**Key tests:**
- **Duplication and the hotbar:** a gun moved into a chest is not given again; only hotbar guns reach MW2, once per weapon; a removed held gun switches to the selected one; no Intervention, SPAS-12 or UMP45 is added; bots keep the refill.
- **Saving:** every fork item round-trips through its carrier in a chest, a shulker box, a bundle, the player file and a friend's view; a renamed gun keeps its name; an unknown `iw4l.id` stays the base item; other packs' `custom_data` is left alone; netherite guns get `damage_resistant`; no disk form puts `max_stack_size` above 1 on a damageable base.
- **Recipes:** no fork grid matches a vanilla 26.3 grid or another fork grid, mirrored or not (needs the jar, like the existing jar tests; the same check as this design's script); every table gun, grenade, ammo type and attachment has exactly one recipe; every ingredient id exists in the jar; every class-picker and preset weapon has a row.
- **Ammo:** a reload takes exactly the missing rounds, offhand first; creative takes none; identical copies swap their clips; spawn refills and Scavenger create no items.
- **Attachments:** attach and strip keep the clip; an impossible combination gives no result; akimbo pairs and splits with the remainder.
- **Grenades:** only a lethal goes in slot 43; shift-click routing; a number key cannot swap dirt into a grenade slot; a throw takes one; refill from the backpack with the same type only; 32 + 32 once; an empty slot keeps the C4 detonator.
- **First join:** given once, the tag kept next to other tags; class attachments become item attachments; round-8 files get it once.
- **Damage:** the table in 11.3 (it has no x.5 cases); pierce; mob damage unchanged; the totem grace boundary at 80 ticks on both clocks; it ends on respawn.
- **Multiplayer:** a friend's slot 44 click is valid; `PLAYER_SLOTS` is 45 in the codec; `PlaceRecipe` round trip.
- **Vendored code:** extra recipes get display indices after vanilla; inserted items have their max stack; two guns on the ground do not merge.

## 17. Owner test

Setup:
- a new world: `IW4L_MINECRAFT_WORLD=r9-weapons` in the `.env`;
- a class with an M4A1 (any attachment), a pistol, Frag and Flash;
- the kit below; `/difficulty normal`;
- for steps 10-12, two windows with `scripts\mp-local.ps1`: host H and friend F.

1. **First spawn.** The hotbar has the class M4A1 and pistol, and no Intervention, SPAS-12 or UMP45. Left of the hotbar are two frames: frag x32 and flash x32. The backpack holds 90 rifle rounds and the pistol's reserve. **Screenshot 1.**
2. **Reloading uses items.** Fire the M4A1 until it reloads. The rifle ammo in the backpack drops by the rounds loaded, the MW2 counter shows `clip / stock`, and the clip bar under the gun icon empties and refills. **Screenshot 2.**
3. **Chests.**
   1. Shift-click the M4A1 into a chest. Hotbar slot 1 stays empty, no new M4A1 appears, and scrolling never raises it. **Screenshot 3.**
   2. Take it back onto the hotbar: it comes up with the same clip.
   3. Put it in the backpack: it cannot be raised.
4. **Dropping and dying.**
   1. Drop the pistol with Q and pick it up again.
   2. `/kill`: the guns, ammo and both grenade stacks drop. After the respawn you get nothing new. **Screenshot 4.**
   3. Pick everything up. With `/gamerule keep_inventory true` and another `/kill`, everything stays.
5. **Save and quit** with a gun in the chest, then load the world. The chest gun, its clip bar and the grenade slots are as you left them, and no second loadout is given. **Screenshot 5.**
6. **Crafting.**
   1. Craft rifle ammo from a copper ingot, an iron nugget and gunpowder: 16 rounds.
   2. Open the recipe book at the crafting table. The Equipment tab shows the gun classes as buttons. **Screenshot 6.**
   3. Craft two gun barrels and a firing mechanism, then click the M4A1 recipe and craft it. It comes out empty (red clip bar) and loads on its first reload.
7. **Attachments.**
   1. Put the M4A1 and the red dot in the crafting grid: the result is the M4A1 with a red dot. Add the FMJ the same way. The tooltip lists both, and aiming shows the red dot. **Screenshot 7.**
   2. Put the two USPs in the grid: one akimbo USP. Put it alone in the grid: one USP comes out and the other stays in the grid.
   3. Put the M4A1 in the grindstone: it comes out bare with its clip.
8. **Grenades.**
   1. G throws a frag (31 left) and Shift+G a flash (31 left).
   2. Press E: the two slots are right of the offhand. An iron block is refused in the lethal slot.
   3. Take the frag stack out of the lethal slot, put one frag back and leave the rest in the backpack. Throw it: the slot refills with frags.
   4. Swap the semtex into the lethal slot and throw it with G. **Screenshot 8.**
9. **Tactical insertion.** Put it in the tactical slot, plant it with Shift+G, walk away and `/kill`. You respawn at the flare once. The next death respawns you at your bed or the world spawn.
10. **Damage between players (H and F).**
    1. F shoots unarmoured H once in the chest with an M4A1: about 1.5 hearts.
    2. H puts on the iron set: an M4A1 hit takes about 0.3 heart. F puts FMJ on the M4A1: about 0.6 heart. **Screenshot 9.**
    3. F's Barrett: a few body hits kill unarmoured H, and in iron about two more.
11. **Totem.**
    1. H holds a totem in the offhand at low health. F holds the trigger and the totem pops.
    2. For about 4 s H's hearts don't drop and F sees no hit markers. **Screenshot 10.**
    3. Repeat with a frag at H's feet just after a pop: no damage, but a push.
    4. A knife hit still hurts during the 4 s.
12. **The friend.**
    1. F's first join gives F their own class guns and 32 + 32 grenades.
    2. F puts a gun in a shared chest. H takes it and fires it, and F can no longer raise it.
    3. F drops a gun with Q. H sees it on the ground and picks it up. **Screenshot 11** (both windows).
13. **Mobs unchanged.** `/summon minecraft:zombie ~ ~ ~5`: it dies in about 4 M4A1 body hits, as before.

Kit (25 stacks):

```
IW4L_MINECRAFT_KIT=iw4l:barrett, iw4l:usp, iw4l:usp, iw4l:sniper_rounds*16, iw4l:semtex*16, iw4l:tactical_insertion*2, iw4l:fmj, iw4l:red_dot_sight, minecraft:crafting_table, minecraft:grindstone, minecraft:chest*2, minecraft:iron_ingot*64, minecraft:iron_block*4, minecraft:copper_ingot*32, minecraft:gunpowder*64, minecraft:iron_nugget*32, minecraft:tripwire_hook*8, minecraft:redstone*16, minecraft:repeater*4, minecraft:iron_helmet, minecraft:iron_chestplate, minecraft:iron_leggings, minecraft:iron_boots, minecraft:totem_of_undying*2, minecraft:white_bed
```

## 18. Decisions and open questions

| # | Question | Chosen answer | Alternative |
|---|---|---|---|
| 1 | Save format | `iw4l:` ids in the game; a vanilla carrier with `custom_data` on disk and on the wire | bare fork ids on disk (vanilla drops them) |
| 2 | Carriers | Guns on `carrot_on_a_stick`, the rest on `poisonous_potato` without food | other unused vanilla items |
| 3 | One item per gun or per MW2 variant | One per base gun; attachments and clips are state | one per `m4_acog_mp` def (hundreds of items) |
| 4 | The roster | Fixed: 63 guns, 9 grenades; content outside it is not an item, and a class naming it gets the category's first gun | spare-list recipes generated at runtime |
| 5 | Which guns MW2 holds | Hotbar guns only, in slot order, up to the free `ps.weapons` slots | the whole inventory (does not fit 15 slots) |
| 6 | Killstreak weapons | MW2 mirrors, never saved, kept out of containers | items |
| 7 | Death and Q | Like vanilla; the offhand ban stays | guns stay on death |
| 8 | Free Intervention, SPAS-12, UMP45 | Removed | given once |
| 9 | Class menu after the first join | Perks and killstreaks only | still gives guns |
| 10 | Loadout reserve | 3 magazines per gun | MW2 start stock; one stack |
| 11 | Ammo | One item per round, 7 types, taken on reload; the clip on the gun | magazine items; endless refill |
| 12 | How spending is counted | At the reload, throw and knife pickup sites only; spawn refills, Scavenger and pickups create nothing | number deltas (a spawn refill would mint items) |
| 13 | A crafted gun | Empty; an absent clip means empty | full |
| 14 | Durability | None; ammo is the running cost | durability and repair |
| 15 | Attachment limit | 2 when MW2 has the def, no Bling | 1, or 2 only with Bling |
| 16 | Attaching | Crafting grid special recipe | anvil with XP |
| 17 | Removing | Grindstone, attachments lost for a little XP; akimbo split in the grid with the second gun as remainder | attachments given back |
| 18 | T5 and IW5 attachments | Through `WeaponFamilies::resolve` and the alias names | none this round |
| 19 | Recipe style | Shared parts, a template per class, fire mode and signature cells | one shape per class with an arbitrary signature item |
| 20 | Recipe unlocks | Fork recipes always known and not saved; vanilla's unlock and save the vanilla way | unlock on picking up an ingredient |
| 21 | Creative place | End of the Combat tab | a tab of their own (needs paging) |
| 22 | Grenade slot place | Left of the offhand on the HUD, right of it in the inventory | right of the hotbar, moving the MW2 weapon bar |
| 23 | Grenade slots on death | Drop like the offhand | stay |
| 24 | Empty grenade slot | Refills from the backpack with the same grenade; pickups fill an empty slot | no refill |
| 25 | Old player files | Get the loadout and 32 + 32 once | nothing |
| 26 | Tactical insertion | An item; the flare is a one-use respawn point ahead of the bed, not saved | not offered in Minecraft worlds |
| 27 | "Twice as much as now" | Twice vanilla's armour share, 80 % ceiling for armour and enchantments together | `(1-h)^2` (diamond almost bulletproof); vanilla once |
| 28 | Protection and Resistance | Count at vanilla rates under the same ceiling; Resistance after it | left out |
| 29 | Armour pierce | LMG 0.25, sniper 0.5, FMJ +0.25 (at most 0.75) | none (iron, diamond and netherite equal against every gun) |
| 30 | Halving scope | All MW2 weapon damage to players, the knife and own grenades included | bullets only (one line) |
| 31 | Totem grace | Bullets and explosions, MW2 and Minecraft, 80 ticks, knockback kept | everything for 4 s |
| 32 | Two copies of one gun | A fast reload by switching, accepted | a forced raise animation |

**Not verified** (the implementer checks these):
- MW2 damage numbers (M4A1 30, USP 40, Desert Eagle 50, LMG 40, Intervention 98, knife 135, rocket 150) are from memory. Only the frag (130) is confirmed by a sim test constant. The tables in 11.3 and the test must be recomputed from the owner's data.
- That MW2's death weapon drop and weapon pickups can be switched off per client on Minecraft maps without GSC errors.
- That an empty primary set (weapon 0) does not upset GSC or the first-person view.
- That `inventory_add_weapon` adds a gun's alt mode by itself for `_gl_` and `_shotgun_` variants.
- Whether the stock setter clamps to `maxAmmo`. The design publishes at most `maxAmmo` either way.
- The site where MW2 retrieves a thrown knife, and where it plants the tactical insertion flare.
- The T5 and IW5 attachment alias names in the content. A content test lists names with no item.
- The default components of `poisonous_potato` in 26.3, which decide the `!` removals.
- The creative tab's free pixels for the grenade slots, against `tab_inventory.png`.
- What real vanilla does when it loads the carrier items. This comes from javap checks, not a vanilla run.
