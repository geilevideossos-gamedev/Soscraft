# Track C: guns and grenades as real items

The owner tested round 8's release 2 (03e0c9be). Screenshots are in `scrennshots/screenshots-03e0c9b/screenshots-03e0c9b/` (untracked, in `C:\Dev\mashup-crafting`). Screenshot N is the file whose name starts with N. The owner's words for this track (round 9 brief, `docs/PROGRESS.md`):

> Track C, guns and grenades as real items (design in `docs/WEAPONS.md` first): 11 guns are normal items, class loadout once on first join, recipes for every gun, ammo, attachments, recipe book and creative tabs; 12 lethal and tactical grenade slots next to the hotbar, 64 per stack, 32 of each at start, recipes; 13 guns do half damage to players and armour reduces gun damage twice as much; 14 a totem also gives 4 s immunity to bullets and explosions.

**The design is already written: `docs/WEAPONS.md`.** This brief does not redo it. It says where each part of the design lands in the code, in which order, with which tests and owner steps. When this brief and WEAPONS.md disagree on a design point, WEAPONS.md wins. When the code or the owner's data proves a WEAPONS.md fact wrong (a damage number, a component of `poisonous_potato`, a free pixel in the creative tab), follow the code or data, update WEAPONS.md in the same commit and record it in the track log.

Track C has four items:

- **C11:** guns, ammo, attachments and gun parts are real items: the item table, saving, MW2 following the hotbar, drops, the first-join loadout, recipes, attachments, the recipe book, creative and ammo. This is most of the track.
- **C12:** lethal and tactical grenade slots next to the hotbar.
- **C13:** guns do half damage to players, and armour takes twice vanilla's share.
- **C14:** a popped totem gives 4 s of immunity to bullets and explosions.

Track C runs beside tracks A (multiplayer fixes), B (items and the screen) and D (finishing survival). It is merged after tracks A and B, before the round's last release build.

Paths and line numbers are as of 53260221 (f1dddf78 plus docs), so re-check them before editing. Vanilla facts were checked with javap on the 26.3 `client.jar`. No Mojang code or data is copied: the fork recipes are our own vanilla-format JSON.

Every section has the same parts: the owner's finding, the cause, what vanilla does, the plan, the tests, and the owner's in-game steps with a kit line. The commit subjects are the ones listed in each plan.

**Multiplayer rule for every part:** the host owns every player's inventory, grenade slots, crafting, ammo use, first-join flag, flare respawn, damage and totem grace. Player state is keyed by UUID, sim state by MW2 client id. Nothing keys on player 0 or on the local client. Every host-side step that runs for the host (`render_anim/src/minecraft_world.rs`, near `:1268` and `:1391`) also runs for each friend (`render_anim/src/minecraft_players.rs`, near `:1625-1638`), through one shared `minecraft_survival` function.

**MW2 maps and bots do not change.** Every new rule is gated on a Minecraft map (`sim::voxel::active()` in the sim, the Minecraft world in render_anim). A client that never publishes an arsenal (a bot) keeps today's full refill.

---

## C11. Guns, ammo and attachments are real items

### The owner's finding
Screenshot 12 (`12_waffen_in_kiste.png`): the chest holds 15 guns over its three rows, and the hotbar still shows 5 guns in slots 1-5. Every gun the owner put into the chest came back on the hotbar. The owner's request: guns become normal items, the class loadout is given once on the first join, every gun, ammo type and attachment gets a recipe, and they show in the recipe book and the creative tabs.

### Cause
A gun is not an item but a mirror of MW2's `ps.weapons`, rebuilt every frame. Containers are invisible to it.

1. **What a gun is.** `ItemStack { id: "iw4:weapon/<n>", count 1, max 1, components None }` (`crates/minecraft_survival/src/guns.rs:5-16`). `<n>` is the match's session weapon index, not a name. Attachments are part of the index (`m4_reflex_mp` and `m4_acog_mp` are different indices; `sim/src/step.rs:1702-1755` `validate_class_content`).
2. **The mirror.** `sync_weapons(inventory, owned)` (`guns.rs:18-55`) runs every frame for the host (`render_anim/src/minecraft_world.rs:1268`, `owned` from `:1246`) and for each friend (`render_anim/src/minecraft_players.rs:1633`). `owned` is `owned_weapons` (`minecraft_players.rs:1119-1130`): every type-0 weapon in `ps.weapons`.
   - It deletes unowned gun items, but only in the player's slots, crafting grids and cursor (`guns.rs:19-35`).
   - It mints a new item for every owned weapon that has none in those places (`:36-54`).
   - Chest, barrel, shulker and ender chest cells are never checked. `keeps_gun` (`crates/minecraft_survival/src/menus.rs:403-424`, guard at `:678`) and `keep_guns` (`:366-381`) block only the offhand and throwing, not a container cell. So a gun shift-clicked into a chest leaves `ps.weapons` unchanged, and the next frame mints a copy: screenshot 12.
3. **The free guns.** `sustain_block_world_arsenal` (`crates/sim/src/item.rs:146-205`, run every host step at `sim/src/step.rs:537`) adds `BLOCK_WORLD_GUNS` (Intervention, SPAS-12, UMP45) to every live player on a Minecraft map with `give_weapon_to_ps_akimbo` (`sim/src/world.rs:4039`), and refills every clip, stock and the grenade ledger to full on every step.
4. **The loadout every spawn.** MW2's GSC `giveLoadout` fills `ps.weapons` at every spawn from the class. Death keeps guns (`minecraft_players.rs:1099-1117`, `death_drops` with "guns stay", `minecraft_survival/src/death.rs:618-647`).
5. **Never saved.** `item_nbt::stack_tag` drops every stack outside the `minecraft:` namespace (`crates/minecraft_terrain/src/item_nbt.rs:74-82`, `vanilla()` at `:72`). The test `guns_and_empty_stacks_are_not_saved` (`minecraft_survival/src/player_nbt.rs:726`) pins it. So the chest copies vanish on reload.
6. **The friend.** `views::slot_nbt` and `view_stack` (`minecraft_survival/src/views.rs:150-173`) send a gun as `{id, count}` only and rebuild `weapon_item(n)`. A gun on the ground never reaches the friend, because its `MobAdd` tag comes from `stack_tag`.
7. **No item table for non-vanilla items.** Both item catalogs default to stack 64 (`third_party/minecraftoss/player/src/item_catalog.rs:283`, `third_party/minecraftoss/core/src/item.rs:38`), so two guns on the ground would merge. `RecipeBook::from_jar` (`third_party/minecraftoss/player/src/crafting.rs:145`) reads only the jar, and `parse_result` (`:984-989`) drops `result.components`. `/give` and the kit accept only `assets/minecraft/items` ids (`minecraft_survival/src/commands.rs:121`, `minecraft_commands/src/kit.rs:79` `registered()`). `item_icons::item_name` hard-codes the `minecraft` namespace (`crates/minecraft_terrain/src/item_icons.rs:92-99`). The creative tabs come from the jar (`render_anim/src/minecraft_creative.rs:267`). There is no recipe book screen (`docs/MINECRAFT-SURVIVAL.md:398`), and `recipeBook` is only carried over.
8. **Ammo.** It lives in `ps.ammoclip` and `ps.ammo`, per player, never on an item, and the sustain refills it every step, so a clip never empties for good.

### Vanilla 26.3
- An item is one `ItemStack` with an id and data components. Moving it into a chest moves it; nothing ever gives it back. Containers, the player file, item frames and item entities save `id`, `count` and `components`.
- `minecraft:custom_data` holds a free compound and survives every codec. A component patch removes a default component with a `!` key.
- An item id that is not in the registry fails the item codec, and vanilla drops that stack (this is why the disk form is a vanilla carrier).
- `ItemStack.validateComponents` rejects "damageable and stackable", so a carrot-on-a-stick carrier must never get `max_stack_size` above 1.
- Recipe results are `ItemStackTemplate(item, count, DataComponentPatch)`; ingredients are id or tag sets with no component test. Unknown recipe ids in `recipeBook` are logged and skipped.
- Ranged weapons keep their state on the item (the crossbow's `charged_projectiles`) and find ammo with `Player.getProjectile`: offhand, main hand, then the inventory. Creative uses no ammo.
- Death drops the whole inventory unless `keep_inventory` is on. There is no class loadout.
- The recipe book is `RecipeBookComponent`: tabs, 20 recipes a page, a search box, the "Showing craftable" toggle, ghost ingredients and the server-side place-recipe packet. Unlocks are saved in `recipeBook`.

### Plan
The design is WEAPONS.md sections 1-9 and 13-15. The parts land in this order. Each numbered part is one commit-group of the track runner.

#### C11.1 The fork item table (WEAPONS.md 4.1, 4.4, 4.5)
1. **"Fork items have their own table: ids, kinds, stack sizes and names"**
   - New file `crates/minecraft_terrain/src/fork_items.rs` (must pass `rustfmt --check`, edition 2021). One row per item of WEAPONS.md 7.2-7.6, 8.1, 9.1 and 10.1: id, kind (gun, ammo, lethal, tactical, attachment, part), MW2 weapon key (`iw4:weapon/m4_mp`, `t5:weapon/galil_mp`, ...), bucket, ammo type, max stack, English fallback name, tier, icon source, creative order, and the fire-proof flag (Barrett, M240). `is_fork(id)`, `get(id)`, `rows()`.
   - Vendored patches: `insert` on `minecraftoss_player::item_catalog::ItemCatalog` and `insert_max_stack` on `minecraftoss_core::item::ItemCatalog`. One helper `fork_items::load` fills both. Call it in `minecraft_terrain::crafting::load_recipes` (`crafting.rs:59`, also when the catalog file is missing) and right after `Registries::load` at `render_anim/src/minecraft_world.rs:275` (host) and `render_anim/src/minecraft_remote.rs:54` (friend).
   - `item_icons::item_name` uses `item.<namespace>.<path>`; `item_icons::language` gains the `item.iw4l.*` keys (MW2 localized name where the session has one, else the table's English name).
   - The table hash joins `registry_fingerprint` (`minecraft_survival/src/session.rs:701`).
   - Mobs never pick up fork items: refuse `fork_items::is_fork` ids in the zombie/allay/fox pickup paths (additive arm; those files are shared with track D).
2. **"Fork items work in /give, /clear and the kit, and players can be tagged with /tag"**
   - `minecraft_survival::commands::registries` (`commands.rs:121`) appends the fork ids, so `/give`, `/clear` and `IW4L_MINECRAFT_KIT` accept them. A given gun comes with a full clip (`custom_data.iw4l.clip`); `/give` takes the component syntax `iw4l:m4[minecraft:custom_data={iw4l:{attachments:["reflex"]}}]`.
   - Vanilla's `/tag <targets> add|remove|list` for players (new file in `crates/minecraft_commands/src/`, rustfmt-clean). It edits a per-player `tags: Vec<String>` that the player file saves under the vanilla `Tags` list. `PlayerData::merged` today carries `Tags` as an unknown key: the merge now owns the list (keeps every other tag).
3. **"Fork items show MW2 icons, names and tooltips in the inventory"**
   - A session table `GunNames` (fork id plus attachments to MW2 weapon index and back), built once per match where `PreparedWeapons` exists, on the host and on each friend. The MW2 weapon is resolved with `WeaponFamilies::resolve(WeaponSelection::with(family, attachments), LoadoutRules { max_attachments: 2 }, ...)` (`asset_game/src/weapon_families.rs:654`, `pub(crate)` today: add a public wrapper) and cached.
   - `McStack.weapon` (`render_anim/src/minecraft_inventory.rs:342`) comes from `GunNames` instead of `weapon_of`. The atlas skip at `minecraft_inventory.rs:488` becomes `fork_items::is_fork`. Grenades use their MW2 HUD icons, attachments `weapon_attachment_*` (`ui/src/classes/icons.rs:237-249`), ammo and parts the tinted vanilla sprites of WEAPONS.md 4.5, read at runtime.
   - Tooltip: MW2 name, attachments, "Clip 27/30", "Ammo: Rifle Ammo", then `gun_lines` (`hud/src/minecraft_inventory.rs:756`).
   - Track B9 adds `McStack.icon` and keys the atlas by it. Keep the change local: a fork stack's `icon` is its own id.

#### C11.2 Disk and wire form (WEAPONS.md 4.2, 4.3, 14)
1. **"Fork items are saved as a vanilla carrier with custom_data and come back when the world loads"**
   - `to_disk(id, components)` and `from_disk(id, components)` in `item_nbt.rs`, called by `stack_tag` and `stack_of`, recursing into `minecraft:container` and `minecraft:bundle_contents`. Guns on `minecraft:carrot_on_a_stick` (never `max_stack_size`), everything else on `minecraft:poisonous_potato` with `!` removals of its 26.3 default food components (read them from the catalog), rockets and explosive rounds with `max_stack_size` 16, netherite guns with `damage_resistant`. `from_disk` fires only for a known `custom_data.iw4l.id` and removes exactly what `to_disk` added; an unknown id stays the base item. `iw4:weapon/` ids (killstreaks) are still never saved.
   - Every caller that already uses `stack_tag`/`slot_list` now saves fork items: the player file, the ender chest, containers (`containers.rs:395`, `:520`, `:780`).
   - Replace `guns_and_empty_stacks_are_not_saved` (`player_nbt.rs:726`).
2. **"Ground items, item frames, piglin inventories, campfires and creative hotbars save fork items the same way"**
   - Grep every `Tag::String(stack.id` and route it: `server_mobs.rs:1619-1690` (`level_entity_tags`, `load_level_entity`), `item_frame.rs:186-196` and `:258`, `piglin_save.rs:39-55`, `hotbars.rs:89-117`, campfire. Remove the gun filter at `minecraft_creative.rs:521-527` except for `iw4:weapon/` killstreaks.
   - Mob equipment saves only the id (`server_mobs.rs:728`); fork items never get there because mobs refuse them (C11.1).
3. **"Friends see fork items through the same vanilla form in slots, menus and on the ground"**
   - `views::slot_nbt`/`view_stack` lose the gun special case and call `stack_tag`/`stack_of` (with the killstreak `iw4:weapon/` id kept as today). Menu cells (`menus.rs:968-975`, `:1278-1283`) follow. Ground items already use `level_entity_tags`, so friends now see dropped guns.

#### C11.3 MW2 follows the hotbar (WEAPONS.md 5.1, 5.2, 5.4)
1. **"MW2 holds only the guns on the Minecraft hotbar, so a gun put into a chest is not given again"**
   - `sim/src/voxel.rs`: `GunSpec`, `Arsenal`, `Spent`, `set_arsenal`, `take_spent` exactly as WEAPONS.md 5.1, next to `DEATH_PROTECTED` (`voxel.rs:453`), cleared in `deactivate` (`:656`).
   - `sim/src/item.rs`: `sustain_block_world_arsenal` becomes `apply_arsenal`: primaries become exactly the published guns (offhands, alt modes of kept guns and killstreaks stay), missing guns added without a raise, others removed with `take_player_weapon`, a removed held gun switches to `selected` or weapon 0; clips from the spec for guns not held, `Clip` queued when the held gun's live clip differs, `rebind` writes the spec clip. Remove `BLOCK_WORLD_GUNS` (`item.rs:146`) and its raise. **Until C11.9 lands, stock stays topped up for the published guns only. Until C12 lands, grenades keep today's refill.** Clients that never published keep the full refill.
   - Switch off MW2's own weapon drop on death and MW2 weapon pickups for clients that publish (find the GSC/native sites; record them in the log).
   - Host glue `minecraft_survival::guns::settle_arsenal(inventory, mode, spent, names) -> Arsenal`: apply `Clip` to the bound gun item, build the gun list from hotbar slots 0-8 in slot order, each MW2 weapon once, stop when `ps.weapons` would overflow (15 slots shared with offhands, alt modes and killstreaks); a gun past that is inert and selecting it shows "Too many guns on the hotbar" on the action bar. Two copies of one gun: WEAPONS.md 5.2. Call it next to `minecraft_world.rs:1268` and `minecraft_players.rs:1633`, then `set_arsenal`.
   - `sync_weapons` mints and deletes only killstreak weapons (`killstreak_*_mp`, `airdrop_*marker*_mp`) from now on. `Follow` (`guns.rs:57-89`) stays.
2. **"Killstreak weapons stay MW2 mirrors and cannot be put into containers"**
   - Extend `keeps_gun` (`menus.rs:403-424`) so an `iw4:weapon/` stack cannot go into a container cell, the cursor drop, Q or the offhand. Death removes them (they vanish, they do not drop).

#### C11.4 Drops, look on the ground, first-join loadout (WEAPONS.md 4.5, 5.3, 6)
1. **"Guns drop on death and with Q and go into chests like any item"**
   - `keep_guns` and the throw arm of `keeps_gun` go for fork guns. The offhand ban stays (`offhand.rs:106-116`, the offhand arms of `keeps_gun`, creative `set_slot`). The death predicate (`minecraft_players.rs:1108-1113`) becomes "killstreak items vanish, everything else follows vanilla". Netherite guns survive fire and lava on the ground.
2. **"Dropped fork items and fork items in item frames are drawn as their icon"**
   - Where the ground-item and item-frame renderers resolve an item model, a fork id resolves to its icon extruded like vanilla's `item/generated`. Do it at the model-resolve level so held non-gun fork items get the same look without editing `render_anim/src/anim/minecraft_gear.rs` (track A owns that file this round).
3. **"A player gets the class guns and three magazines once, on their first join"**
   - On the host, for the host and each friend, on the first frame the player is alive on a Minecraft map with an MW2 class and without the `iw4l_loadout_given` tag. Read the class from `LoadoutSpec` (`sim/src/match_state/loadout.rs:78`), not `ps.weapons`; split a variant weapon with `WeaponFamilies::describe` (`weapon_families.rs:650`). A gun outside the table becomes the category's first table gun (WEAPONS.md 4.1), logged once.
   - Give: the primary and secondary with attachments and full clips into hotbar slots 0 and 1 if free, else the vanilla add order; three magazines of reserve per gun as ammo items; overflow drops at the feet like `commands::give_stacks`. Set the tag. C12 adds the grenades to this same function.
   - Round-8 player files have no tag, so they get the loadout once.

#### C11.5 Recipes (WEAPONS.md 7, 8.1, 9.1, 10.1, 13.1 loading)
1. **"Recipe results keep their components, so crafted suspicious stew has its effect"**: `parse_result` (`crafting.rs:984`) keeps `result.components`.
2. **"Guns, ammo, grenades, attachments and gun parts have crafting recipes"**
   - `RecipeBook::with_extra(recipes)` in the vendored crate reuses `parse_recipe` and gives fork recipes display indices after vanilla's. Called in `load_recipes` before `with_item_catalog`, on the host and the friend.
   - The recipes are our own JSON in a new `crates/minecraft_terrain/data/fork_recipes.json` (`include_str!`), written from WEAPONS.md 7.2-7.6, 8.1, 9.1 and 10.1. Ingredients name `iw4l:` ids directly (`iw4l:gun_barrel`).
   - Every fork recipe is always known, never written to `recipeBook`.

#### C11.6 Attachments, akimbo, the grindstone (WEAPONS.md 9.2)
1. **"Attachments go onto a gun in the crafting grid, and two identical guns make an akimbo pair"**: fork special recipes in `minecraft_survival`, where the crafting result is computed, for every grid including the 2x2. Attach (1 or 2 items, at most 2 total, only if `WeaponFamilies::resolve` accepts it; extra rounds from a smaller clip come back as ammo items), akimbo (two equal guns, if the gun offers akimbo), split (the second gun is the cell's remainder). Not listed in the recipe book.
2. **"A grindstone strips a gun's attachments"**: the grindstone result is the bare gun (akimbo kept), clip cut to the bare size, a little XP.

#### C11.7 The recipe book (WEAPONS.md 13.1)
1. **"Minecraft recipe book in the inventory and the crafting table"**: the green button at (104, 61) in the inventory and vanilla's place in the crafting table; the panel to the left with Search, Equipment, Building, Misc and Redstone, 20 a page, search, "Showing craftable", red backgrounds, group buttons with the right-click alternatives. Sprites from the 26.3 pack at runtime.
2. **"The recipe book places ingredients for the host and friends"**: `MenuAction::PlaceRecipe { recipe, all }` applied on the host for the player who sent it; red ghost items for missing ingredients.
3. **"Vanilla recipe unlocks and the book's settings are saved in the player file"**: vanilla unlocks the vanilla way (`auto_unlock_rules`), saved in `recipeBook` (`recipes`, `toBeDisplayed`, the per-tab open and filter flags).

#### C11.8 Creative (WEAPONS.md 13.2)
1. **"Guns, ammo, grenades and attachments are at the end of the creative Combat tab"**: appended after `creative_tabs(&jar)` (`minecraft_creative.rs:267`), in the WEAPONS.md order, guns bare and fully loaded; search finds them by MW2 name. Track A6 also edits `minecraft_creative.rs` (friend creative): keep the edit to the loader thread.

#### C11.9 Ammo (WEAPONS.md 8.2, 5.1 stock)
1. **"Reloading a gun in a Minecraft world takes ammo items from the inventory"**
   - `Arsenal.stock` per ammo type is counted over slots 0-35 and the offhand (never containers or the grenade slots). `apply_arsenal` sets each gun's MW2 stock to `min(stock - rounds not yet taken, maxAmmo)`.
   - The sim queues `Spent::Rounds` at the finished-reload site only (partial reloads queue only the moved rounds). The host removes items: offhand, main hand, then slots 0-35. Spawn refills, Scavenger, One Man Army and ammo pickups create nothing, because they are overwritten next step.
   - Creative (`endless`): stock full, no items taken; the clip still empties and reloads. The Ballistic Knife reloads from `iw4l:throwing_knife` items outside the lethal slot.
2. **"A gun's clip shows as the item bar under its icon"**: green when full, red when empty, on the host's and the friend's screens.

### Tests
- **fork_items:** every class-picker gun (`ui/src/classes/setup.rs:541-597`) and every showcase preset weapon (`frame/src/class_presets.rs`) has a row; ids unique; stack sizes as WEAPONS.md; `every_fork_item_s_disk_form_is_valid_in_vanilla` (base id in the 26.3 item list, no `max_stack_size` above 1 on a damageable base, sizes within 1..99; data-gated on `MINECRAFTOSS_ROOT`).
- **Vendored catalogs:** `inserted_items_have_their_max_stack` (player and core); a world test that two identical guns on the ground do not merge.
- **commands/kit:** `the_kit_and_give_accept_fork_items` (a given gun has a full clip; the component syntax works); `/tag` add, remove, list, and the merge keeps other tags.
- **item_nbt:** a gun round-trips through `minecraft:carrot_on_a_stick` with clip and attachments; a grenade stack through `poisonous_potato`; a renamed gun keeps `custom_name`; an unknown `iw4l.id` stays the base item; another pack's `custom_data` keys stay; guns inside a shulker box and a bundle convert both ways; netherite guns get `damage_resistant`; `iw4:weapon/` is still not saved.
- **player_nbt, containers, server_mobs:** a gun in the inventory, the ender chest, a chest and on the ground survives save and load; `minecraft_survival/tests/vanilla_world.rs` extends the chest round trip and still passes the vanilla tag-type checks.
- **views:** `a_friend_sees_a_fork_item_through_slot_nbt` (clip and attachments survive).
- **guns.rs:** `a_gun_moved_into_a_chest_is_not_given_again` (through `menus::apply` with a chest menu, then `settle_arsenal`: one gun item total, not in the list); `only_hotbar_guns_reach_mw2_once_each`; `a_gun_past_the_free_weapon_slots_is_inert`; `two_copies_bind_the_selected_one`; `killstreak_items_cannot_go_into_a_chest`.
- **sim:** `the_published_guns_become_the_primaries` (offhand, alt mode of a kept gun and killstreak kept; removed held gun switches to `selected`); `no_intervention_spas_or_ump_is_added`; `a_client_that_never_published_keeps_the_full_refill`; `a_held_gun_queues_its_clip`.
- **death.rs:** update `death_drops_follow_vanilla_order_keep_guns_and_destroy_vanishing_items`: fork guns drop, killstreak items vanish.
- **Loadout:** given once with the tag set; a second call gives nothing; class attachments become item attachments; overflow drops; the tag sits beside other tags.
- **Recipes (jar-gated):** `a_recipe_result_keeps_its_components` (suspicious stew); every table row has exactly one recipe; every ingredient id exists in the jar or the table; no fork grid matches a vanilla 26.3 grid or another fork grid, mirrored or not (the same check WEAPONS.md's script did); fork display indices come after vanilla's.
- **Attachments:** attach keeps clip, name and other attachments; an impossible pair gives no result; a third attachment gives none; akimbo pairs and splits with the remainder; the grindstone strips and keeps the clip.
- **Recipe book:** page and filter logic (pure); `PlaceRecipe` codec round trip; placing fills the grid from the inventory, `all` places as many as fit; `recipeBook` round-trips through the player file.
- **Creative:** the Combat tab ends with the fork items in table order; search finds "M4A1".
- **Ammo:** a reload takes exactly the missing rounds, offhand first; creative takes none; rounds not yet taken are subtracted from the stock; a spawn refill creates no items.

### Owner's steps (idea)
New world: `IW4L_MINECRAFT_WORLD=r9-guns` in the `.env`, a class with an M4A1 and a pistol, survival.
1. First spawn: the class M4A1 and pistol on hotbar slots 1 and 2, no Intervention, SPAS-12 or UMP45, rifle and pistol ammo in the backpack. Screenshot 1.
2. Shift-click the M4A1 into a chest: slot 1 stays empty, no copy appears, scrolling never raises it (the screenshot 12 case). Take it back: it comes up with the same clip. Put it in the backpack: it cannot be raised. Screenshot 2.
3. Put a gun in a shulker box, a bundle, an item frame and the ender chest. Drop one with Q. Save and quit, load again: all still there, tooltips unchanged, no second loadout. Screenshot 3.
4. `/kill`: guns and ammo drop and lie on the ground as their icons. Respawn gives nothing. `/gamerule keep_inventory true`, `/kill` again: everything stays.
5. Fire the M4A1 until it reloads: the rifle ammo in the backpack drops by the rounds loaded, and the clip bar under the icon empties and refills. Screenshot 4.
6. Crafting table: open the recipe book, Equipment tab, find the M4A1 group; craft barrels, a mechanism and the M4A1. It comes out empty. Screenshot 5.
7. Grid: M4A1 plus red dot gives the M4A1 with a red dot; two USPs give an akimbo USP; the akimbo USP alone splits; the grindstone strips the M4A1. Screenshot 6.
8. `/gamemode creative`: the Combat tab ends with the guns, parts, attachments, ammo and grenades; search "M4". `/gamemode survival`.
9. Two windows: the friend's first join gives their own class guns; the friend puts a gun in a shared chest, the host takes and fires it, the friend can no longer raise it; the friend drops a gun and the host sees it on the ground. Screenshot 7 (both windows).
10. `/tag @s remove iw4l_loadout_given`, `/kill`: the loadout comes once more.

```
IW4L_MINECRAFT_KIT=iw4l:m4,iw4l:ak47,iw4l:usp,iw4l:usp,iw4l:barrett,iw4l:rifle_ammo*64,iw4l:pistol_ammo*64,iw4l:red_dot_sight,iw4l:fmj,minecraft:crafting_table,minecraft:grindstone,minecraft:chest*4,minecraft:shulker_box,minecraft:bundle,minecraft:ender_chest,minecraft:item_frame*4,minecraft:iron_ingot*64,minecraft:iron_block*4,minecraft:copper_ingot*32,minecraft:gunpowder*64,minecraft:iron_nugget*32,minecraft:tripwire_hook*8,minecraft:redstone*16,minecraft:repeater*4,minecraft:lava_bucket,minecraft:cooked_beef*32
```

---

## C12. Lethal and tactical grenade slots

### The owner's finding
The owner's words: lethal and tactical grenade slots next to the hotbar, 64 per stack, 32 of each at the start, with recipes. G and Shift+G stay as round 8 track F set them. No screenshot shows grenades. Screenshot 11 (`11_rotes_overlay.png`, a track B item) shows the MW2 weapon bar starting about 8 px right of the hotbar, with the frag and flash icons under "192": that is why the slots go to the left.

### Cause
This is a new feature. Today grenades are MW2 offhand weapons only, not items, and they never run out.
- The class picks the lethal and tactical (`sim/src/match_state/loadout.rs:78-89`); GSC `giveLoadout` puts them in `ps.weapons` and sets `offhand_primary`/`offhand_secondary`; `meta.loadout.lethal`/`tactical` are replicated (`net/src/transport/meta_wire.rs:1897-1931`).
- `sustain_block_world_arsenal` refills every grenade's ps ammo and the match ledger every step (`sim/src/item.rs:192-196`).
- G and Shift+G put `loadout.lethal` or `loadout.tactical` into `cmd.off_hand_index` (`net/src/client/runtime.rs:1133-1142`). `weapon_check_for_offhand` (`weapon_iw4/src/offhand.rs:397-459`) picks the first offhand of the class with ammo (`:78-96`); with none, only a detonator row (C4) works. Offhand rows come from `ps.weapons` plus `meta.ammo_for` (`sim/src/combat.rs:437-466`). A throw spends one at `combat.rs:761` (OffhandUsed) and a cook-off at `:808` (OffhandCookedOff).
- The inventory has 43 slots (`third_party/minecraftoss/player/src/inventory.rs:104`, `:399`, `seen` at `:961`); the view sends 41 (`minecraft_net/src/messages.rs:392`, `frame/src/minecraft_ui.rs:10`, `menus.rs:28`, `minecraft_commands/src/clear.rs:11`).
- The hotbar draws slots 0-8 and the offhand only (`hud/src/minecraft_hotbar.rs:79-118`).
- Offered types (`asset_game/src/weapon_catalog.rs:445-449`): lethal frag, semtex (class 1), throwing knife (4), claymore and C4 (5); tactical flash (3), concussion and smoke (2). The class menu also offers the tactical insertion (`flare_mp`).

### Vanilla 26.3
- No grenade slot exists. The closest model is the offhand: slot 40, saved under `equipment.offhand`, drawn at (77, 62) in `inventory.png` and (35, 20) in the creative inventory tab. Extra slots use the 18x18 `container/slot` sprite.
- HUD: `Hud.extractItemHotbar` draws `hud/hotbar_offhand_left` (29x24) at (cx-91-29, h-23) only when the offhand holds something.
- Equipment slots refuse items that do not belong (`Inventory::click` `:463-489`, `number_swap` `:665-672`); shift-click routes equipment to its slot (`quick_move` `:686-707`).
- Death drops the offhand unless `keep_inventory`. `/clear` covers equipment. A pickup merges into the offhand only when the same item is there.
- An unknown key inside `equipment` breaks vanilla's equipment codec, so the grenade slots must be a separate top-level key. Unknown top-level keys are ignored.
- A bow finds arrows anywhere in the inventory (`Player.getProjectile`). Creative throwables do not use up the stack.

### Plan
WEAPONS.md section 10. Slots 43 lethal and 44 tactical. **Not vanilla.**

#### C12.1 The slots
1. **"Lethal and tactical grenade slots in the player inventory, saved with the player, for the host and friends"**
   - Vendored `Inventory` grows to 45 (`inventory.rs:104`, `:399`, `seen` `:961`); `add_item` merges a pickup into a matching grenade slot right after the main-hand and offhand merge, and fills an empty grenade slot of its kind.
   - `PLAYER_SLOTS`/`MC_INVENTORY_SLOTS` become 45 in `minecraft_net/src/messages.rs:392`, `frame/src/minecraft_ui.rs:10`, `menus.rs:28` and `clear.rs:11`. Bump `minecraft_net::PROTOCOL` once for the track here unless C11.7's `PlaceRecipe` commit already did.
   - `guns::fits(slot, stack)` used by every click path, number keys, drag-spread and creative `set_slot`: 43 takes only lethal grenades, 44 only tactical, up to 64. Shift-click of a grenade from 0-35 goes to its slot first if empty or the same grenade; shift-click out of 43 or 44 goes to 9-35, then 0-8. `valid_slot` (`menus.rs:434`) accepts below 45.
   - Saving: the fork top-level key `iw4l_grenades`, a `slot_list` with `Slot:0b` (lethal) and `Slot:1b` (tactical) in the disk form, in `OWNED` in `player_nbt.rs` so emptied slots leave the file. Never inside `equipment`.
   - Death: add 43 and 44 to `EQUIPMENT_DROP_ORDER` (`death.rs:19`) after the saddle. `/clear` covers them.

#### C12.2 Throwing from the slots
1. **"The grenade slots decide what G and Shift+G throw, and each throw uses one up"**
   - `Arsenal.lethal`/`tactical` (weapon index and count) from slots 43 and 44. `apply_arsenal` makes that weapon the only offhand of its bucket, through a helper factored out of `apply_give_offhand` (`sim/src/step.rs:1526-1591`): it sets `offhand_primary`/`offhand_secondary` and `meta.loadout.lethal`/`tactical`, so the client runtime and the friend's replicated loadout follow unchanged. Ledger and ps ammo are the count minus throws not yet taken. Add a public `offhand_class(weapon)` on `SimWorld` (`equipment_facts_for`, `world.rs:1277`, is `pub(crate)`).
   - `combat.rs:761` and `:808` queue `Spent::Grenade { delta: -1 }` on a Minecraft map; picking a thrown knife up queues `+1` (find the site; record it).
   - An empty slot keeps the last weapon with ammo 0, so the MW2 icon stays and placed C4 can still be detonated.
   - `settle_arsenal` applies `Grenade` deltas; a `+1` with a full slot goes through the pickup rules. When a throw empties a slot, it refills from the backpack with the same grenade only. Creative throws spend nothing.
   - Check how the ps stock setter behaves with 32 or more against a small `maxAmmo`; the ledger stays the truth.
2. **"The hotbar and the inventory show the lethal and tactical grenade slots"**
   - HUD (`hud/src/minecraft_hotbar.rs`): two `hud/hotbar_offhand_left` frames left of the offhand frame, lethal at x = cx-91-29-29, tactical at cx-91-29-58, y = h-23, item at +3, h-19 with its count; always drawn when the hotbar shows. The MW2 weapon bar's grenade icons stay.
   - Inventory screen (`hud/src/minecraft_screens.rs`): slot 43 at (134, 62), 44 at (152, 62), drawn with `container/slot`; empty slots show the tooltip "Lethal" or "Tactical". Creative inventory tab (`hud/src/minecraft_creative.rs`): about (127, 20) and (145, 20), checked against `tab_inventory.png`.
3. **"New players start with 32 lethal and 32 tactical grenades"**: the C11.4 loadout also puts 32 of the class lethal into 43 and 32 of the class tactical into 44, falling back to frag and flash.

#### C12.3 Tactical insertion and the class menu
1. **"A planted tactical insertion is a one-use respawn point"** (WEAPONS.md 10.6, **not vanilla**): Shift+G plants MW2's flare, the sim queues `Spent::Flare { pos }` (find the plant site), the host stores the player's flare respawn (level and position, per UUID, not saved). The next respawn uses it ahead of a bed or anchor and clears it; a second flare or a destroyed flare clears it too. `minecraft_survival/src/respawn.rs` is also edited by track A7 (`level_spawn`): add a separate check before it, do not restructure it.
2. **"On Minecraft maps the class menu changes perks and killstreaks only"** (WEAPONS.md 5.5): the weapon and grenade rows show "Guns and grenades are items in Minecraft worlds"; One Man Army and Bling change nothing there.

### Tests
- **Vendored inventory:** 45 slots; a pickup tops up a matching grenade slot and fills an empty one of its kind; a non-grenade never lands in 43 or 44.
- **menus:** `only_a_lethal_grenade_goes_in_the_lethal_slot` (dirt, flash and a gun refused; semtex accepted); `only_a_tactical_goes_in_the_tactical_slot`; `shift_click_moves_a_grenade_to_its_slot_and_back_out`; `a_number_key_cannot_swap_dirt_into_a_grenade_slot`; `a_friends_click_on_slot_44_is_valid`; drag-spread obeys `fits`.
- **player_nbt:** `grenade_slots_are_saved_under_their_own_key_and_read_back`; `emptied_grenade_slots_leave_the_file`; `tests/vanilla_world.rs` fills both slots and checks `iw4l_grenades` is a list and `equipment` has no new key.
- **death.rs:** grenade slots drop after the saddle; `keep_inventory` keeps them.
- **minecraft_net:** `PLAYER_SLOTS` is 45 (`messages.rs:657`) and slots 43 and 44 round-trip.
- **clear:** `clear_empties_the_grenade_slots`.
- **sim:** `a_published_grenade_count_limits_throws` (frag 2: two throws, the third does nothing, two `Grenade` entries queued); `pending_throws_are_subtracted_until_taken`; `the_lethal_item_replaces_the_class_lethal` (semtex replaces frag; `offhand_primary` and `loadout.lethal` follow; also throwing knife and claymore); `an_empty_slot_keeps_the_c4_detonator`; `a_client_that_never_published_keeps_unlimited_grenades`.
- **guns.rs:** `a_throw_takes_one_from_its_slot`; `an_emptied_slot_refills_with_the_same_grenade_only`; `creative_keeps_the_stack`; `a_new_player_gets_32_of_each_class_grenade_once`.
- **hud:** pure position helpers: hotbar frames at cx-149 and cx-178 (item origin), inventory slots at (134, 62) and (152, 62), next to `the_inventory_tab_places_armor_offhand_and_trash_like_vanilla`.
- **Flare:** a stored flare respawn wins over a bed once and is then cleared; a second plant replaces it.

### Owner's steps (idea)
New world `IW4L_MINECRAFT_WORLD=r9-grenades`, a class with Frag and Flash.
1. Spawn: two frames left of the offhand frame: frag x32 and flash x32. Screenshot 1.
2. G throws a frag (31), Shift+G a flash (31), whatever is held. Cook a frag until it blows in the hand: still one less. Screenshot 2.
3. E: the two slots right of the offhand. Cobblestone and a concussion grenade are refused in the lethal slot. Shift-click the semtex stack: it goes into the lethal slot if that is empty. Screenshot 3.
4. Swap through throwing knife, claymore and C4 in the lethal slot and stun and smoke in the tactical slot; throw each. Empty the lethal slot with C4 placed: G still detonates.
5. Put one frag in the lethal slot and the rest in the backpack; throw it: the slot refills with frags.
6. Drop grenades with Q and walk over them: they go back into their slot.
7. Save and quit, load: slots and counts unchanged, no new 32. Screenshot 4.
8. `/gamemode creative`: throws do not lower the count. `/gamemode survival`.
9. `/kill`: the grenade stacks drop after the rest; respawn with empty slots.
10. Tactical insertion: put it in the tactical slot, plant it with Shift+G, walk away, `/kill`: respawn at the flare once; the next death respawns at the bed.
11. Two windows: the friend's first join gives 32 + 32 of their class grenades; their throws lower their count and the host sees the grenade; the host's counts are unaffected. Screenshot 5 (friend window).

```
IW4L_MINECRAFT_KIT=iw4l:frag_grenade*16,iw4l:semtex*16,iw4l:throwing_knife*8,iw4l:claymore*4,iw4l:c4*4,iw4l:concussion_grenade*16,iw4l:smoke_grenade*8,iw4l:tactical_insertion*2,minecraft:chest*2,minecraft:cobblestone*64,minecraft:white_bed,minecraft:torch*16,minecraft:cooked_beef*32
```

---

## C13. Guns do half damage to players, and armour takes twice vanilla's share

### The owner's finding
The owner's words: guns do half damage to players, and armour reduces gun damage twice as much. Damage to mobs does not change. No screenshot.

### Cause
Today armour reduces gun damage by exactly 0 %, so "twice as much as now" has no literal value. WEAPONS.md 11.2 reads it as twice vanilla's armour share with vanilla's 80 % ceiling.
- Every MW2 hit on a player runs on the host: bullets, projectiles and grenade splash through `sim::damage::apply_damage_attempt` (`sim/src/damage.rs:882-920`; creative refused at `:899`, pvp off at `:903`, hit location at `:907-915`), GSC blasts through `apply_script_blast` (`:389-498`), GSC hits through `apply_script_hit` (`:522-577`).
- All end in `sim::script::host::players::player_damage` (`players.rs:26`), which runs GSC `Callback_PlayerDamage`, then `finishplayerdamage` (`natives/player.rs:427-502`), then `script_player::finish_damage` (`script_player.rs:266-302`): `ps.health -= amount`.
- On the next frame `Survival::observe` (`minecraft_survival/src/survival.rs:317`) turns the MW2 loss into hearts at `max_health/20`, absorption first. No armour, Protection or Resistance anywhere. `docs/MULTIPLAYER.md` section 13 says so.
- Halving in `observe` cannot work: MW2 already killed at `ps.health <= 0` in `finish_damage`. The rule must run in the sim before GSC.

### Vanilla 26.3
- `CombatRules.getDamageAfterAbsorb`: f = 2 + toughness/4, g = clamp(armor - damage/f, 0.2*armor, 20), damage*(1 - g/25): armour takes at most 80 %. Then Resistance, then `getDamageAfterMagicAbsorb` (Protection EPF, capped at 20, 4 % a point), then absorption. Vendored as `SurvivalStatus::actually_hurt` (`third_party/minecraftoss/player/src/survival.rs:331-355`).
- `#is_projectile` covers arrows and the like (Projectile Protection), `#is_explosion` explosions (Blast Protection).
- Armour wears by damage/4 per hit; the fork models no armour wear for players.

### Plan
WEAPONS.md section 11. **Not vanilla** (a fork balance rule).
1. **"Guns do half damage to players in Minecraft worlds and worn armour takes twice vanilla's share of it"**
   - Pure rules in the sim (a new `crates/sim/src/gun_damage.rs`, rustfmt-clean, edition 2024): `GunHit { Bullet, Explosion, Melee }`, `gun_hit(means)` (the means lists of WEAPONS.md 11.1; `MINECRAFT_MEANS`, falls, suicide, crush and unknown are None), `gun_damage_to_player(amount, max_health, guard, pierce, kind)` with the WEAPONS.md 11.2 formula and the named constants `GUN_PLAYER_SCALE`, `GUN_ARMOR_FACTOR`, `GUN_PROTECTION_CAP`.
   - `sim/src/voxel.rs`: `GunGuard { armor, toughness, epf_bullet, epf_blast, epf_melee, resistance, totem_grace }`, `set_gun_guard(client, guard)`, cleared in `deactivate`.
   - `player_damage` (`players.rs:26`), before GSC: when `voxel::active()`, `!runtime.minecraft_hit` and `gun_hit(means)` is Some, build a scaled copy of the hit (amount and `commit.amount`) and pass that copy to GSC and `current_hit`. `max_health` from the victim's `ps.max_health` (100 if absent).
   - The mirror: after settle, `set_gun_guard` for the host (next to `minecraft_world.rs:1391`) and each friend (next to `minecraft_players.rs:1638`), through one `minecraft_survival` helper that reads `survival::armor(inventory)` (`survival.rs:1182`), the EPF with `minecraft:arrow`, `minecraft:player_explosion` and `minecraft:player_attack` from the existing enchantment code, and the Resistance level.
2. **"LMGs, snipers and FMJ pierce part of a player's armour"**: `sim::voxel::set_pierce_table(weapon index -> pierce)`, pushed once per match from the gun class and the `fmj` attachment (LMG 0.25, sniper 0.5, FMJ +0.25, at most 0.75).
- Recompute the WEAPONS.md 11.3 table from the owner's real weapon damage values if they differ from memory (only the frag 130 is confirmed), and update it.

### Tests
- `gun_hits_are_bullets_explosions_or_melee` (every means in WEAPONS.md 11.1; `script_player::MINECRAFT_MEANS`, MOD_FALLING, MOD_SUICIDE, MOD_UNKNOWN are None).
- `guns_do_half_damage_to_an_unarmoured_player`: 30 → 15, 98 → 49, 130 → 65, 1 → 1.
- `armour_takes_twice_vanilla_s_share_up_to_four_fifths`: the WEAPONS.md 11.3 rows without pierce (no x.5 cases).
- `pierce_cuts_the_protection`: the LMG, sniper and FMJ rows.
- `protection_and_resistance_count_under_the_same_ceiling`: Protection IV on four pieces takes an unarmoured-equivalent M4A1 hit 15 → 5; Resistance V blocks everything.
- `mob_damage_is_unchanged`: a `MobShot` path test, or an assertion that `player_damage` is the only call site of the rule.
- `mw2_maps_pass_the_amount_through`: with `voxel` inactive, the amount is unchanged.
- Check by grep and a run that no existing sim test with `voxel` active and GSC running expects full player damage.

### Owner's steps (idea)
Two windows (`scripts\mp-local.ps1`), survival, pvp on, `/difficulty normal`. Host H, friend F.
1. H unarmoured, full health. F shoots H once in the chest with an M4A1 from 5 blocks: about 1.5 hearts. Screenshot 1.
2. H in leather: about 0.8 heart; in iron: a sliver (0.3 heart). F puts FMJ on the M4A1: about 0.6 heart. Screenshot 2.
3. F's Barrett: a few body hits kill unarmoured H; in iron a few more.
4. H in diamond throws a frag at their feet: about 1.3 hearts at most; unarmoured it no longer kills from full health. Screenshot 3.
5. H in diamond with Protection IV on the chestplate: hits are smaller still.
6. `/summon minecraft:zombie ~ ~ ~5`: it dies in about 4 M4A1 body hits, as before. Screenshot 4.
7. Swap roles: F's own screen shows the same small losses.

```
IW4L_MINECRAFT_KIT=iw4l:m4,iw4l:m4,iw4l:rifle_ammo*64,iw4l:rifle_ammo*64,iw4l:fmj,iw4l:barrett,iw4l:sniper_rounds*16,iw4l:frag_grenade*16,minecraft:crafting_table,minecraft:leather_helmet,minecraft:leather_chestplate,minecraft:leather_leggings,minecraft:leather_boots,minecraft:iron_helmet,minecraft:iron_chestplate,minecraft:iron_leggings,minecraft:iron_boots,minecraft:diamond_helmet,minecraft:diamond_chestplate+protection=4,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*16,minecraft:zombie_spawn_egg*2,minecraft:cooked_beef*32
```

---

## C14. A popped totem gives 4 s of immunity to bullets and explosions

### The owner's finding
The owner's words: a totem also gives 4 s of immunity to bullets and explosions. No screenshot.

### Cause
- The totem pops in two places. MW2 damage: `finish_damage` (`sim/src/script_player.rs:289`) calls `voxel::spend_death_protection` (`voxel.rs:465`), which sets `ps.health = 1` and queues `TOTEM_POPS`; render_anim drains it (`minecraft_world.rs:704`) and `Survival::protect` (`survival.rs:964-983`) takes the totem and applies the effects. Minecraft damage: the per-tick `hold_off_death` in `minecraft_players.rs:596-609`.
- `set_death_protected` is mirrored into the sim each frame (`minecraft_world.rs:1391`, `minecraft_players.rs:1638`).
- Nothing protects the player after the pop. The remaining shotgun pellets of the same tick hit a player on 1 health.

### Vanilla 26.3
`LivingEntity.checkTotemDeathProtection` sets health 1, clears effects and gives Regeneration II 900, Absorption II 100 and Fire Resistance 800. There is no immunity window, only the 20-tick hurt cooldown. Explosion knockback still pushes an entity whose hurt is refused. The 4 s grace is a fork rule.

### Plan
WEAPONS.md section 12. **Not vanilla.**
1. **"A popped totem of undying turns bullets and explosions away for four seconds"**
   - Sim: `spend_death_protection(client, tick)` also records `tick + TOTEM_GRACE_TICKS` (80 sim ticks of 50 ms). `finish_damage` gains `tick` (callers `natives/player.rs:485`, `damage/height_bands.rs:332` and `:354`). `player_damage` refuses Bullet and Explosion hits while the sim grace runs or `GunGuard.totem_grace` is set: no GSC call, no health change, no hit marker. Melee is not covered.
   - Minecraft side: `Survival` gets `totem_grace_until`; `protect` sets it to `tick + 80`; `totem_grace()`; `hurt` (`survival.rs:704`) refuses `DamageCause::is_explosion()` (new on `death.rs` `DamageCause`: explosion, player_explosion, bad_respawn_point, fireworks) while it runs, so TNT, creepers, crystals and beds are covered through `struck`. Knockback still applies. `respawn` clears it. `/kill` and the void are unaffected.
   - `GunGuard.totem_grace` comes from `survival.totem_grace()` in the C13 mirror.
   - Under `/tick freeze` the Minecraft clock stops while the sim clock runs: accepted, recorded in the log.

### Tests
- sim: `a_spent_totem_turns_guns_away_for_four_seconds` (immune at ticks 100 and 179, not at 180; `totem_grace: true` in the guard makes it immune at 500); update `a_totem_saves_one_death_and_reports_it` for the new signature; the height-band tests pass `Tick(..)` with unchanged assertions; `a_knife_still_hurts_during_the_grace`.
- minecraft_survival: `a_popped_totem_turns_explosions_away_for_four_seconds` (explosion and bad respawn point refused, mob melee and a fall still hurt, an explosion lands again after 80 ticks); `the_totem_grace_ends_on_respawn`; `explosions_are_the_vanilla_is_explosion_types`.

### Owner's steps (idea)
Two windows, survival, pvp on.
1. H unarmoured with a totem in the offhand at 2-3 hearts. F holds the trigger on H. The totem pops (rising totem, particles, sound), and for about 4 s H's hearts do not drop and F sees no hit markers. Then absorption goes first. Screenshot 1 (during the 4 s, F visibly firing).
2. H holds a new totem at low health next to lit TNT; F shoots H down just before it explodes: H is pushed but loses no hearts. Screenshot 2.
3. Right after a pop, F knifes H: H takes damage (knife not covered).
4. Swap roles: F's totem protects F the same way. Screenshot 3 (F's window).

```
IW4L_MINECRAFT_KIT=iw4l:m4,iw4l:rifle_ammo*64,iw4l:frag_grenade*16,minecraft:totem_of_undying*4,minecraft:tnt*8,minecraft:flint_and_steel,minecraft:golden_apple*16,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:cooked_beef*32
```

---

## Shared points for the track
- **Order.** Items and saving first (C11.1-C11.4), then recipes, attachments, the recipe book, creative and ammo (C11.5-C11.9), then the grenade slots (C12), then balance and the totem (C13, C14). C13 and C14 touch only the damage path and could run earlier if the orchestrator splits the track.
- **The protocol.** `minecraft_net::PROTOCOL` goes up once in this track, in the first commit that changes a message shape (C11.7's `PlaceRecipe` or C12.1's 45 slots, whichever lands first). Tracks A and B each bump it too: whoever merges later takes the next number and renumbers the new message indices in the codec tests. Record the final number in the log.
- **Shared files.** Keep every edit additive (new modules, fields, match arms) and small:
  - `minecraft_survival/src/menus.rs`: track A6 (friend creative) and track D3 (merchant screen) edit it.
  - `minecraft_survival/src/player_nbt.rs`: track D adds effects, the warden tracker and the enchanting seed.
  - `minecraft_survival/src/death.rs`: track D adds death messages.
  - `minecraft_survival/src/respawn.rs`: track A7 changes `level_spawn`.
  - `sim/src/step.rs`: track A7 edits `run_players_system` (`:381-422`); this track edits `:537` and `apply_give_offhand` (`:1526`).
  - `sim/src/voxel.rs`: track A reads it only; add new statics next to `DEATH_PROTECTED`.
  - `minecraft_terrain/src/server_mobs.rs`, `item_frame.rs`, `mesh.rs`: tracks B and D edit them.
  - `render_anim/src/minecraft_inventory.rs`, `frame/src/minecraft_ui.rs` (`McStack`): track B adds `icon` and `lore`. Whoever merges second keeps every field.
  - `render_anim/src/minecraft_creative.rs`: track A6 splits `apply` and `hotbar_key`; this track only appends tab entries in the loader.
  - `render_anim/src/minecraft_players.rs`, `minecraft_world.rs`, `minecraft_remote.rs`, `minecraft_net/src/messages.rs`, `hud/src/minecraft_inventory.rs`, `hud/src/minecraft_gui.rs`: every track edits them.
- **Vendored patches.** Count the minecraftoss files patched and added by this track in the log (catalogs, `crafting.rs`, `inventory.rs`, any world pickup refusal), for the CLAUDE.md count at the end of the round.
- **Design doc.** `docs/WEAPONS.md` belongs to this track. Update it in the same commit when the code or the owner's data proves a fact wrong.
- **State docs.** Do not edit `CLAUDE.md`, `docs/MINECRAFT-SURVIVAL.md`, `docs/MULTIPLAYER.md`, `docs/PROGRESS.md` or `docs/round9/decisions.md`. Write their new wording in the track log under "State doc changes": `MINECRAFT-SURVIVAL.md` lines 55 (totem), 59 (guns stay on death), 134 (guns in the hotbar with unlimited ammo), 350 (guns not saved), 374 (MW2 damage through armour deferred), 398 (no recipe book); `MULTIPLAYER.md` sections 10.3, 13 and 20; every "Not vanilla" decision for `decisions.md`.
- **Combined kit line** for the track (25 stacks):

```
IW4L_MINECRAFT_KIT=iw4l:barrett,iw4l:usp,iw4l:usp,iw4l:sniper_rounds*16,iw4l:semtex*16,iw4l:tactical_insertion*2,iw4l:fmj,iw4l:red_dot_sight,minecraft:crafting_table,minecraft:grindstone,minecraft:chest*2,minecraft:iron_ingot*64,minecraft:iron_block*4,minecraft:copper_ingot*32,minecraft:gunpowder*64,minecraft:iron_nugget*32,minecraft:tripwire_hook*8,minecraft:redstone*16,minecraft:repeater*4,minecraft:iron_helmet,minecraft:iron_chestplate,minecraft:iron_leggings,minecraft:iron_boots,minecraft:totem_of_undying*2,minecraft:white_bed
```

## Rules
One commit per fix or feature, with tests. Run clippy, and rustfmt on new files. No code comments. Use gentle cargo mode. Owner steps and the kit line for each commit go in `docs/round9/track-c-log.md`.
