# Track C log: guns and grenades as real items

Branch `track-c`, worktree `C:\Dev\mc-wt\track-c`, target dir `D:\iw4l-wt\track-c`. Briefs: `docs/round9/track-c.md` and the design `docs/WEAPONS.md`. Base: `24dfdf71`.

A commit can't name its own hash, so each entry is written with its commit and names the commit before it; the last entry's hash is in `git log` under its subject.

## C11.1 The fork item table

### 1. Fork items have their own table: ids, kinds, stack sizes and names

Commit: the one after `24dfdf71`.

- **Changed:**
  - `minecraft_terrain` `fork_items.rs` (new, edition 2021, rustfmt-clean): one row per fork item of WEAPONS.md 7.2-7.6, 8.1, 9.1 and 10.1, 94 rows: 63 guns (riot shield and launchers included), 2 gun parts, 13 attachments, 7 ammo types, 5 lethal grenades and 4 tacticals (the tactical insertion included). Each row has the id (`iw4l:...`), the kind (gun, ammo, lethal, tactical, attachment, part), the MW2 weapon key (`iw4:weapon/m4_mp`, `t5:weapon/galil_mp`, `iw5:weapon/iw5_ksg_mp`, `iw5:weapon/xm25_mp`), the bucket (the gun class for guns, lethal or tactical for grenades, `Other` for the rest), the ammo item id, the max stack (guns 1, rockets and explosive rounds 16, else 64), the English fallback name, the tier, the icon source (MW2 kill icon, MW2 HUD icon, an MW2 material, or a vanilla sprite with a tint), the fire-proof flag (Barrett and M240) and, for attachments, the MW2 attachment names (aliases). The row order is the creative order (`creative_order()`). Functions: `rows`, `get`, `is_fork`, `by_weapon`, `by_alias`, `ammo_index`, `load` (fills a catalog through the `Catalog` trait), `add_names` (the `item.iw4l.*` keys) and `table_hash`.
  - Vendored `minecraftoss_player::item_catalog`: `ItemCatalog` derives `Default` and gains `insert`; `ItemProperties::plain(max_stack)` gives the absent-item defaults (no fuel, speed 1.0, no remainder, no armour, no durability).
  - Vendored `minecraftoss_core::item`: `ItemCatalog::insert_max_stack`.
  - `fork_items::load` runs on the player catalog in `minecraft_terrain::crafting::load_recipes` and on the core catalog right after `Registries::load` on the host (`render_anim/src/minecraft_world.rs` `load`) and on the friend (`render_anim/src/minecraft_remote.rs` `prepare`).
  - `item_icons::item_name` looks up `item.<namespace>.<path>` and `block.<namespace>.<path>`, then the table's English name; `item_icons::language` adds the `item.iw4l.*` keys (English fallback; MW2 localized names come in commit 3), also when the pack has no `en_us.json`.
  - `minecraft_survival::session::registry_fingerprint` also hashes the fork table, so a friend on a build with another table is refused.
  - Vendored `minecraftoss_entities` `world/piglins.rs`: `piglin_wants` refuses every `iw4l:` id (an additive arm right after the griefing check).
- **Tests:**
  - `minecraft_terrain` `fork_items::tests`: `every_class_picker_and_preset_weapon_has_a_row` (reads `ui/src/classes/setup.rs` `cac_roster` primary to tactical rows, 51 weapons, and every `*:weapon/*` of `frame/src/class_presets.rs` with `include_str!`, so a new class gun fails the test), `ids_and_weapons_are_unique_and_in_the_fork_namespace`, `the_roster_has_the_designed_size`, `stack_sizes_follow_the_design`, `guns_have_ammo_and_only_netherite_guns_resist_fire`, `the_creative_order_puts_guns_by_class_then_parts_attachments_ammo_and_grenades`, `names_fill_the_language_map_without_replacing_existing_keys`, `both_catalogs_take_the_fork_stack_sizes`, and, with the 26.3 data, `two_identical_guns_on_the_ground_do_not_merge` (a real `Level` with the fork-filled core catalog: two sticks merge, two M4A1s stay two, two frags merge).
  - Vendored: `minecraftoss_core` `item::tests::inserted_items_have_their_max_stack`, `minecraftoss_player` `item_catalog::tests::inserted_items_have_their_max_stack`, `minecraftoss_entities` `world::piglins::tests::piglins_never_want_fork_items` (a bare-handed piglin wants a stick and gold, never a gun, a grenade or ammo).
- **Decisions:**
  - **No zombie, allay or fox pickup exists in the fork** (searched every `items_in` caller): only piglins and villagers pick items up. A bare-handed piglin wants any item (`can_replace` with an empty hand), so the refusal arm went into `piglin_wants`. Villagers want only their food and farming items, so they never take a fork item and got no arm. The arm tests the `iw4l:` namespace because the vendored entities crate cannot see `fork_items`; every `iw4l:` id is a fork item. WEAPONS.md 4.1 updated.
  - **When `artifacts/item-catalog/26.3.json` is missing** the recipe book keeps no catalog, as before: attaching a catalog holding only the fork rows would switch off the book's built-in fuel table and the lava bucket remainder, so furnaces would stop working in that mode. In that degraded mode (already warned about: "stacks of 64") guns stack like every item in the inventory; on the ground they still never merge, because the core catalog is always filled. WEAPONS.md 4.4 updated.
  - The ground merge test lives in `minecraft_terrain`, not in the vendored world crate: the world crate has no `Level` test harness, and the terrain test exercises the real `fork_items::load` on the real core catalog.
  - Tiers follow WEAPONS.md section 3: pistols, machine pistols, SMGs, parts, early attachments and the grenades are `Early`; rifles, shotguns, the specials, rifle ammo, shells, explosive rounds, middle attachments and the tactical insertion `Middle`; snipers and sniper rounds `Diamond`; LMGs, the AT4, RPG, Strela, China Lake, XM25, LMG ammo, rockets and the thermal scope `Nether` (the thermal scope needs magma cream, so it sits with the Nether stage of section 3 although 9.1 calls it "late"); the Barrett, Stinger, Javelin and heartbeat sensor `Late`.
  - The CZ75 (T5) is a pistol and the FMG9 (IW5) a machine pistol in the bucket; both share pistol ammo as WEAPONS.md says.
  - English fallback names drop the "(T5)" and "(IW5)" of the design tables (they are annotations), so two rows are called ".44 Magnum" and two "SPAS-12". MW2's localized names replace them in commit 3 where the session has them.
  - Ammo tints (brass, copper, grey, gold, red, green) are picked RGB values in the table; the sprites themselves are read at runtime (commit 3).
- **Owner's test steps:** nothing new to see yet; fork items can be given from commit 2 on.
  1. Start the world as host: it loads as before (no new warning in the log).
  2. Two windows (`scripts\mp-local.ps1`): the friend on the same build joins as before.
- **Kit:** none needed.

### 2. Fork items work in /give, /clear and the kit, and players can be tagged with /tag

Commit: the one after `cc8f967b`.

- **Changed:**
  - `minecraft_survival::commands::registries` appends every fork id to the command item list (only when the pack listed items, so an empty list still means "accept all"), so `/give`, `/clear`, `/kit` and the `/give` suggestions take `iw4l:` ids.
  - `minecraft_commands` `args.rs`: the item argument takes the `minecraft:custom_data` component as SNBT (`iw4l:m4[minecraft:custom_data={iw4l:{attachments:["reflex"]}}]`, short name `custom_data` too); a malformed compound is `arguments.item.component.malformed` at the brace, like the book components. The crate now depends on `minecraftoss-core` for the SNBT parser (`Cargo.toml`, one line in `Cargo.lock`).
  - `minecraft_terrain` `crafting::kit_components` also reads `custom_data`, so `IW4L_MINECRAFT_KIT` and `/kit` take the same syntax.
  - `fork_items::with_full_clip(id, components)`: a gun with ammo gets `custom_data.iw4l.clip = -1` unless the clip is named, plus `clip_left = -1` with `akimbo` and `alt_clip = -1` with a grenade launcher or underbarrel shotgun attachment; every other component and `custom_data` key is kept. `/give` (`Host::give`, which `/kit` uses) and the start kit (`crafting::give_kit`) call it. The riot shield, ammo, grenades, attachments and parts stay without components, so equal stacks merge.
  - Vanilla `/tag <targets> add|remove|list` for players: `minecraft_commands` `tag.rs` (new, edition 2024, rustfmt-clean), registered next to `/tick`, permission level 2. `ArgKind::Word` (brigadier's `word()`). The vanilla responses: `commands.tag.add.success.single|multiple`, `commands.tag.add.failed` when no target changed (already tagged or 1024 tags), the same for remove, `commands.tag.list.single|multiple.empty|success` with the tags sorted and green, joined by ", " (`ComponentUtils.formatList`, checked with javap). `CommandWorld` gains `tags`, `add_tag` and `remove_tag` with empty defaults.
  - `Player.tags` (per player, per UUID, on the host) is read from the player file's vanilla `Tags` list and written back by `PlayerData::merged`; the merge replaces `Tags` only when the data carries a list, and the list holds every tag read from the file, so other packs' tags stay. An empty list removes `Tags`, as vanilla writes it only when non-empty.
- **Tests:**
  - `minecraft_survival` `commands::tests::the_kit_and_give_accept_fork_items` (the registries list fork ids next to vanilla's; `/kit` gives a loaded M4A1 that stacks to 1, 70 rifle rounds as 64 + 6 without components, an akimbo USP with `clip` and `clip_left` -1, a riot shield without a clip, and refuses `iw4l:nothing`; `/give ... iw4l:m4[minecraft:custom_data={iw4l:{attachments:["reflex","gl"]}}] 2` gives two separate guns with `clip` and `alt_clip` -1; 40 frags stack; `/clear @s iw4l:m4` removes the three guns), `tag_adds_and_removes_tags_on_the_player`.
  - `minecraft_survival` `player_nbt::tests::tags_round_trip_through_the_vanilla_tags_list`, `the_merge_keeps_foreign_tags_and_owns_the_list_only_when_it_has_one`.
  - `minecraft_commands` `tag::tests` (`add_and_remove_report_like_vanilla`, `list_sorts_the_tags_of_every_target`, `tags_are_unique_and_capped`, `only_game_masters_may_tag`), `args::tests::custom_data_is_parsed_as_snbt`; the `/help`, root suggestion and `/t` completion lists now include `tag`.
  - `minecraft_terrain` `fork_items::tests::given_guns_get_full_clips_and_other_items_stay_plain`, `the_start_kit_gives_loaded_guns_with_their_attachments`.
- **Decisions:**
  - **A given gun's full clip is written as `-1`.** The commands and the kit run in `minecraft_survival` and `minecraft_terrain`, which never see `PreparedWeapons`, so the real clip size (which also depends on the attachments, e.g. extended mags) is unknown there. `-1` means "full": the settle of group arsenal (C11.3) writes the real size when it first binds the gun, and the tooltip (commit 3) shows it as full. No session weapon index is stored. WEAPONS.md 4.2 updated.
  - `/tag` takes players only (`ArgKind::Players`), since mobs have no tags in the fork yet; vanilla takes any entity.
  - `custom_data` is the only new `/give` component; vanilla accepts every component, the fork keeps its list short.
- **Owner's test steps** (survival, any world, kit below; fork items have no icon yet and are not saved yet, both come in later commits):
  1. Join: the backpack holds the kit. Hover each fork item: the tooltip names it ("M4A1", "Rifle Ammo", "Frag Grenade"). **Screenshot 1.**
  2. `/give @s iw4l:m4 2`: two separate M4A1 items, not one stack of 2.
  3. `/give @s iw4l:frag_grenade 70`: one stack of 64 and one of 6. `/give @s iw4l:rocket 20`: 16 and 4.
  4. `/give @s iw4l:m4[minecraft:custom_data={iw4l:{attachments:["reflex"]}}]`: accepted (the attachment shows in the tooltip from commit 3 on).
  5. `/clear @s iw4l:m4`: "Removed 3 item(s) from player ...".
  6. `/tag @s add iw4l_test`, `/tag @s list` ("... has 1 tag(s): iw4l_test"), save and quit, load again, `/tag @s list`: still there. `/tag @s remove iw4l_test`. **Screenshot 2.**
- **Kit:**

```
IW4L_MINECRAFT_KIT=iw4l:m4,iw4l:ak47,iw4l:usp[minecraft:custom_data={iw4l:{attachments:["akimbo"]}}],iw4l:barrett,iw4l:rifle_ammo*70,iw4l:sniper_rounds*16,iw4l:frag_grenade*40,iw4l:flash_grenade*8,iw4l:rocket*20,iw4l:red_dot_sight,iw4l:gun_barrel*4,iw4l:firing_mechanism
```

### 3. Fork items show MW2 icons, names and tooltips in the inventory

Commit: the one after `e6c6666d`.

- **Changed:**
  - `minecraft_survival` `gun_names.rs` (new, edition 2024, rustfmt-clean): `GunNames`, the session table fork id + attachments to MW2 weapon index and back, cached both ways, plus clip sizes. It works through the `GunContent` trait (`configuration`, `selection`, `clip_size`), so `minecraft_survival` stays free of `asset_game` and the later settle and loadout can use the same table. Grenades map with no attachments. The reverse map accepts both spellings of IW5 keys (`iw5:weapon/xm25_mp` and `iw5:weapon/iw5_xm25_mp`). Also `attachments(stack)`, `clip(stack, key)`, `attachment_name` and the pure `tooltip_lines`.
  - `render_anim` `minecraft_fork_items.rs` (new, edition 2024, rustfmt-clean): `RegistryGuns`, the `GunContent` of the session's `WeaponRegistry`: `FamilyKey::parse` + `WeaponFamilies::find`, then the existing public `WeaponRegistry::resolve_configuration` with `LoadoutRules { max_attachments: 2 }` (the public wrapper the brief asked for already existed, so `asset_game` is unchanged), `resolve_index` for weapons without a family, `describe_configuration` or `namespaced_key_of` back, `facts_of(..).clip_size`. `ForkLook` holds one `GunNames` per `PreparedWeapons` (a new registry, i.e. a new match, starts a new table), fills the language map with MW2's localized names (`display_name_key_of` + the localized strings) once per registry, makes the tooltip lines and the non-gun icons.
  - `InventoryUi::set_weapons` runs before every `publish` on the host (`minecraft_world.rs`, from the `update` system's existing `PreparedWeapons` and strings) and on the friend (`minecraft_remote.rs` `friend`, two `Option<Res<..>>` added to an existing parameter tuple). When MW2 names arrive, the cached `ui.names` of fork ids are dropped so they are looked up again.
  - `McStack.weapon` is `weapon_of` (killstreak mirrors) or, for a fork gun, its `GunNames` index, so the HUD draws its MW2 kill icon and MW2 name exactly as for the old mirrors. McStack got no new field: the gun lines (attachments, "Clip 27/30", "Ammo: Rifle Ammo") go first into `McStack.enchantments`, which the HUD draws in grey right under the name and before `gun_lines`, the order the brief asks for. Track B's `McStack.icon` therefore merges without touching these lines.
  - The item atlas skips every fork id whose icon is the kill icon (all guns) and builds the other fork icons itself: ammo and parts from the vanilla sprite read at runtime from the 26.3 pack, multiplied by the row's tint (iron nugget brass, copper, grey, gold and red; fire charge green; firework rocket, the barrel's iron bars and the mechanism's tripwire hook untinted); rapid fire the vanilla repeater; the other attachments and every grenade the MW2 picture decoded from the owner's IWDs on a worker thread (`asset_material::decode_ui_image`, the class menu's way), fitted into the square cell and centred.
  - `hud` `minecraft_inventory.rs` `stack_name`: a renamed gun shows its anvil name instead of the MW2 name (one guard).
  - `fork_items`: grenade rows carry their picture stem (`Icon::Material`); the unused `Icon::HudIcon` is gone.
- **Tests:**
  - `minecraft_survival` `gun_names::tests::gun_names_go_both_ways_and_cache` (a fake content table: bare, one and two attachments, a refused pair, a grenade ignoring attachments, the IW5 key spellings, non-fork ids and killstreak mirrors refused, every index round-trips, an unknown index is None), `the_item_state_is_read_from_custom_data`, `tooltip_lines_list_attachments_clip_and_ammo` (names of attachments and akimbo, "Clip 27/30", an absent clip is 0, `-1` is full, akimbo shows both clips, an unknown size, the riot shield has no lines).
  - `render_anim` `minecraft_fork_items::tests::a_tint_multiplies_the_sprite_and_keeps_alpha`, `a_wide_image_is_fitted_into_the_square_and_centred`.
- **Decisions:**
  - **Grenade icons are the class menu's equipment pictures**, not the weapon defs' `hudIcon`: the HUD icons are zone materials that only the MW2 draw path has, while the item atlas needs pixels. The pictures come from the same IWDs the class menu decodes. WEAPONS.md 4.5 updated.
  - Grenades keep `McStack.weapon` empty, so they show their picture and their MW2 name (from the language map) without the gun stat lines.
  - A gun the session cannot resolve (no MW2 content loaded yet, or a combination the content lacks) has no icon and keeps its English or MW2 name; nothing else breaks.
  - Attachment names stay the table's English names; MW2's localized attachment strings are not looked up.
  - The alternate clip (`alt_clip` of a grenade launcher or underbarrel shotgun) is not shown yet; it joins the tooltip with the ammo group.
- **Owner's test steps** (survival, a new world, kit below; fork items are still not saved, that comes with the save group):
  1. Open the inventory: every gun shows its MW2 kill icon; the frag, semtex, flash and tactical insertion show their MW2 class-menu pictures; the red dot, ACOG and heartbeat sensor their attachment pictures; rapid fire a repeater; the ammo shows coloured nuggets (rifle copper, pistol brass, LMG grey, shells red, sniper gold), the explosive round a green fire charge, the rocket a firework rocket; the barrel iron bars and the mechanism a tripwire hook. **Screenshot 1.**
  2. Hover the M4A1: "M4A1" (MW2's name), "Clip 30/30", "Ammo: Rifle Ammo", then MW2's stat lines. **Screenshot 2.**
  3. Hover the akimbo USP: "Akimbo", "Clip 12/12 + 12/12" (MW2's clip size), "Ammo: Pistol Ammo".
  4. `/give @s iw4l:m4[minecraft:custom_data={iw4l:{attachments:["reflex","fmj"]}}]`: the tooltip lists "Red Dot Sight" and "FMJ". **Screenshot 3.**
  5. Hover the frag: MW2's name for it, no stat lines. The riot shield has no clip or ammo line.
  6. `/xp add @s 5 levels`, rename the M4A1 in the anvil: the tooltip shows the new name in italics.
  7. Two windows (`scripts\mp-local.ps1`): the friend's inventory shows the same icons, names and tooltips. **Screenshot 4** (friend window).
- **Kit:**

```
IW4L_MINECRAFT_KIT=iw4l:m4,iw4l:t5_galil,iw4l:iw5_ksg,iw4l:usp[minecraft:custom_data={iw4l:{attachments:["akimbo"]}}],iw4l:riotshield,iw4l:rifle_ammo*64,iw4l:pistol_ammo*32,iw4l:lmg_ammo*32,iw4l:shotgun_shells*16,iw4l:sniper_rounds*16,iw4l:explosive_round*8,iw4l:rocket*8,iw4l:frag_grenade*8,iw4l:semtex*4,iw4l:flash_grenade*4,iw4l:tactical_insertion,iw4l:red_dot_sight,iw4l:acog_scope,iw4l:rapid_fire,iw4l:heartbeat_sensor,iw4l:gun_barrel*4,iw4l:firing_mechanism,minecraft:anvil
```

## Combined kit for group fork-items

```
IW4L_MINECRAFT_KIT=iw4l:m4,iw4l:ak47,iw4l:t5_galil,iw4l:iw5_ksg,iw4l:barrett,iw4l:usp[minecraft:custom_data={iw4l:{attachments:["akimbo"]}}],iw4l:riotshield,iw4l:rifle_ammo*70,iw4l:pistol_ammo*32,iw4l:lmg_ammo*32,iw4l:shotgun_shells*16,iw4l:sniper_rounds*16,iw4l:explosive_round*8,iw4l:rocket*20,iw4l:frag_grenade*40,iw4l:semtex*4,iw4l:flash_grenade*8,iw4l:tactical_insertion,iw4l:red_dot_sight,iw4l:acog_scope,iw4l:rapid_fire,iw4l:gun_barrel*4,minecraft:anvil
```

## C11.2 Disk and wire form

### 1. Fork items are saved as a vanilla carrier with custom_data and come back when the world loads

Commit: the one after `86cd3918`.

- **Changed:**
  - `minecraft_terrain` `item_nbt.rs`: `to_disk(id, components)` and `from_disk(id, components)` on NBT components, called by `stack_tag` and `stack_of`. Both recurse into the items of `minecraft:container` (`{slot, item}` entries) and `minecraft:bundle_contents`.
    - `to_disk` of a fork item: a gun becomes `minecraft:carrot_on_a_stick` (never `max_stack_size`), everything else `minecraft:poisonous_potato` with `"!minecraft:food": {}` and `"!minecraft:consumable": {}`, plus `"minecraft:max_stack_size": 16` for rockets and explosive rounds (any row whose stack is not 64). Every fork item gets `minecraft:item_name` `{translate: "item.iw4l.<path>", fallback: "<English name>"}` and `custom_data.iw4l.id`; the Barrett and the M240 get `minecraft:damage_resistant` `{types: "#minecraft:is_fire"}`. Components the stack already has are kept (an added component never replaces one already there).
    - `from_disk` fires only when `custom_data.iw4l.id` is a known fork id: the id becomes the fork id, `iw4l.id` goes (and `iw4l`, then `custom_data`, when that leaves them empty), and each component `to_disk` adds is removed only when it still holds exactly the added value. An anvil `custom_name`, another pack's `custom_data` keys or an `item_name` a pack changed stay. An unknown `iw4l.id` stays the plain base item with all its components.
    - `stack_tag` saves vanilla and fork ids (`item_nbt::saved`); `iw4:weapon/` killstreak mirrors and every other namespace are still never saved, also inside a shulker box or bundle (dropped on save only; loading keeps every nested id it finds).
  - With that the player file (inventory, equipment, ender chest) and every container that saves through `slot_list` (`containers.rs` chests and the like, ovens, the brewing stand and the other block entities) save fork items, and `stack_of`/`slots` bring them back.
  - `docs/WEAPONS.md` 4.3: the potato sentence now records what was checked (below).
- **Tests:**
  - `minecraft_terrain` `item_nbt::tests`: `a_gun_is_saved_on_a_carrot_on_a_stick_and_comes_back_with_its_clip_and_attachments` (the exact disk form, and a bare gun gets only `iw4l.id`), `a_grenade_stack_is_saved_on_a_poisonous_potato_that_cannot_be_eaten` (the exact disk form; rockets 16, rifle ammo no size), `netherite_guns_resist_fire_on_disk_and_other_guns_do_not`, `a_renamed_gun_keeps_its_name_and_other_packs_keep_their_custom_data` (also an `item_name` changed by a pack stays), `an_unknown_fork_id_or_a_plain_carrier_stays_the_base_item`, `killstreak_mirrors_and_other_namespaces_are_still_never_saved`, `fork_items_in_shulker_boxes_and_bundles_are_saved_and_loaded_both_ways` (a gun, grenades, a diamond, a killstreak mirror and a bundle holding rockets in a shulker box; loading keeps a foreign nested id), `every_fork_item_comes_back_from_its_disk_form` (all 94 rows), and with the 26.3 data `every_fork_item_s_disk_form_is_valid_in_vanilla` (the base id has an item definition in the jar, guns and only guns sit on a damageable base, no `max_stack_size` on a damageable base, sizes 1..99, the stack size vanilla gives the carrier equals the table's).
  - `minecraft_survival` `player_nbt::tests::fork_items_are_saved_in_the_inventory_and_ender_chest_and_killstreak_ids_are_not` replaces `guns_and_empty_stacks_are_not_saved`.
  - `minecraft_survival` `tests/vanilla_world.rs`: the player holds a loaded M4A1 with a red dot (slot 1) and 16 frags (slot 2), and a second gun is clicked into the chest. The player file and the chest's block entity carry the carrier with vanilla tag types (`custom_data` and `item_name` compounds, `iw4l.attachments` a list, `clip` an int, `id` a string; the potato's `!` removals compounds), vanilla's chest load-and-save keeps them, and after reopening the world both guns come back equal to what was saved.
- **Decisions:**
  - **The potato's default components were checked with javap, not read from the catalog.** `artifacts/item-catalog/26.3.json` lists only 588 items with non-default properties and has no row for `minecraft:poisonous_potato`. `Items.<clinit>` registers it with `new Item.Properties().food(Foods.POISONOUS_POTATO, Consumables.POISONOUS_POTATO)`, and `Item$Properties.food(FoodProperties, Consumable)` sets exactly `DataComponents.FOOD` and `DataComponents.CONSUMABLE` (no `compostable` call, unlike the baked potato). So the removals are `!minecraft:food` and `!minecraft:consumable`, as WEAPONS.md said. The carrot on a stick is `new Item.Properties().durability(25)`, so it carries `max_damage` 25 (the catalog agrees) and its disk form never sets `max_stack_size`.
  - A removal is matched by value on load, so a carrier that vanilla or a pack changed keeps the changed component rather than losing it.
  - A nested item whose id would not be saved (a killstreak mirror, another mod's item) is dropped when a shulker box or bundle is saved, the same rule as a top-level stack; loading drops nothing.
  - Grenades in the inventory are saved like any item for now; the grenade slots and `iw4l_grenades` come with the grenade group.
- **Owner's test steps** (survival, a new world, kit below):
  1. Hover the M4A1 (red dot, FMJ) and write down its clip. Put the Barrett and 8 frags into a chest, the AK-47 and 16 rockets into the ender chest, and the USP and 8 rocket rounds into the shulker box; break the shulker box and pick it up. **Screenshot 1** (the inventory with the shulker box).
  2. Save and quit to the title, load the world again.
  3. The M4A1 is in the same slot with the same attachments and clip in its tooltip; the frags still stack. **Screenshot 2.**
  4. Open the chest: the Barrett and the frags are there. Open the ender chest: the AK-47 and the rockets (a stack of 16). Place the shulker box and open it: the USP and the rockets. **Screenshot 3** (the shulker box open).
  5. `/give @s iw4l:rocket 20`: 16 and 4, so the saved rockets still stack to 16.
- **Kit:**

```
IW4L_MINECRAFT_KIT=iw4l:m4[minecraft:custom_data={iw4l:{attachments:["reflex","fmj"]}}],iw4l:barrett,iw4l:ak47,iw4l:usp,iw4l:frag_grenade*16,iw4l:rocket*24,minecraft:chest,minecraft:ender_chest,minecraft:shulker_box,minecraft:diamond_pickaxe
```

### 2. Ground items, item frames, piglin inventories, campfires and creative hotbars save fork items the same way

Commit: the one after `06e61fa9`.

- **Changed:**
  - `server_mobs.rs` `level_entity_tags` writes a ground item through `item_nbt::to_disk` and `load_level_entity` reads it through `from_disk` (three lines each). A ground stack with an `iw4:weapon/` id is skipped (never saved); every other id is saved as before.
  - `item_frame.rs` `stack_of_tag` (frames load) goes through `from_disk`; the frame's save already used `item_nbt::stack_tag`.
  - `piglin_save.rs` `stack_of` (inventory and equipment load) goes through `from_disk`; the save already used `stack_tag`. Piglins never take fork items (C11.1), so this only keeps a hand-edited world consistent; the same tags are what a friend's `MobAdd` carries.
  - `hotbars.rs` (creative saved hotbars, `hotbar.nbt`) writes through `to_disk` and reads through `from_disk`.
  - `item_nbt.rs`: `KILLSTREAK_PREFIX` (`iw4:weapon/`). Changed from commit 1: a nested shulker-box or bundle item is now dropped on save only when it is a killstreak mirror (another mod's nested item is kept, as before this track), and `to_disk`/`from_disk` hand non-compound components (older JSON text) back unchanged.
- **Grep results (`Tag::String(stack.id` and every own item serializer):**
  - Campfires save and load through `item_nbt::slot_list`/`slots` (`campfire.rs:151`), so they were done by commit 1.
  - `render_anim/src/minecraft_creative.rs:521-527` needs no edit: its filter is `guns::weapon_of(stack).is_none()`, and `weapon_of` only matches `iw4:weapon/<index>` ids, so since commit 1 of C11.1 it drops killstreak mirrors only and fork guns are already stored in saved hotbars. Loading them back passes `creative::apply`, because the command item list (the `known` check) has every fork id since `e6c6666d`.
  - Not routed, because fork items cannot get there: mob equipment saves (`server_mobs.rs:728`, the id only), the villager's held trade item and inventory (`server_mobs.rs:1394`, `:1501`; villagers want only their food and farming items), lecterns and books (`server.rs:258`, `lectern.rs`), tridents (`trident.rs`), the splash potion item (`server_mobs.rs:1609`), the eye of ender.
  - Friends do not see ground items through `MobAdd`: they come in the `EntitySnapshot` as `ServerItem { item, components }` with the in-memory id (`iw4l:m4`), which the friend's own table reads. So ground items needed no wire change (commit 3 covers slots and menus).
- **Tests:**
  - `minecraft_terrain` `server::tests::a_dropped_gun_is_saved_as_a_vanilla_item_entity_and_comes_back` (a loaded M4A1, 5 frags and a killstreak mirror on the ground: the saved tags hold a carrot on a stick with `iw4l.id` and `clip` and a poisonous potato, no mirror; after loading, the M4A1 has its exact components and the frags are 5 again).
  - `minecraft_terrain` `item_frame::tests::a_gun_in_a_frame_is_saved_as_its_carrier_and_comes_back`.
  - `minecraft_terrain` `hotbars::tests::a_saved_hotbar_keeps_a_gun_and_rockets_through_their_carriers`.
  - `item_nbt::tests::fork_items_in_shulker_boxes_and_bundles_are_saved_and_loaded_both_ways` also checks that another mod's nested item is saved and that JSON-text components pass through unchanged.
- **Decisions:**
  - A ground item of another namespace is still saved (as before this track); only killstreak mirrors are skipped. Inventories keep their older rule (only vanilla and fork ids are saved).
- **Owner's test steps** (survival, a new world, kit below):
  1. Press Q on the M4A1 (red dot) and on the frag stack so both lie on the ground. Within a minute, save and quit to the title and load the world again: both still lie there. Pick them up: the M4A1 tooltip still lists the red dot and the same clip, the frags are 8. **Screenshot 1** (the M4A1 tooltip after the reload).
  2. Place an item frame and put the Barrett into it. Save and quit, load again: the frame still holds it (it may look empty or wrong until the look group draws fork items on the ground and in frames). Hit the frame: the Barrett drops, and its tooltip says Barrett .50cal. **Screenshot 2.**
  3. `/gamemode creative`, keep the M4A1 and the rockets in the hotbar, hold C and press 1 ("Saved toolbar"). `/clear`, then hold X and press 1: the M4A1 (with its red dot) and the rockets come back. Quit the game completely, start it again, load the world, X+1 again: they come back. **Screenshot 3.**
- **Kit:**

```
IW4L_MINECRAFT_KIT=iw4l:m4[minecraft:custom_data={iw4l:{attachments:["reflex"]}}],iw4l:barrett,iw4l:frag_grenade*8,iw4l:rocket*16,minecraft:item_frame*2,minecraft:oak_planks*16
```

### 3. Friends see fork items through the same vanilla form in slots, menus and on the ground

Commit: the one after `44c94ac7`.

- **Changed:**
  - No code change was needed in `views.rs` or `menus.rs`. `views::slot_nbt` already called `item_nbt::stack_tag` for everything except `weapon_of` stacks, and `weapon_of` only matches `iw4:weapon/<index>`: since the C11.1 table the "gun special case" is exactly the killstreak pair the brief keeps. Fork guns used to be dropped there because `stack_tag` refused every non-vanilla id; since commit 1 of this group they travel in the disk form (carrot on a stick or poisonous potato with `custom_data.iw4l`), and `view_stack` turns them back through `stack_of`/`from_disk` with the friend's own recipe book (whose catalog has the fork stack sizes from `fork_items::load`). Menu cells (`menus.rs` `sent` at :968-975 and `stacks` at :1281) and the friend's inventory mirror (`render_anim/src/minecraft_remote.rs:1040`) call the same two functions, so clips, attachments, custom names and stack sizes reach the friend's slots, menus and tooltips.
  - Ground items reach a friend in the `EntitySnapshot` as `ServerItem { item, components }` with the in-memory id (see commit 2), so a dropped gun arrives as `iw4l:m4` with its components; how it is drawn on the ground comes with the look group (C11.4).
  - No `PROTOCOL` bump: no message changed shape.
- **Tests:**
  - `minecraft_survival` `views::tests::the_view_carries_items_and_guns_and_they_come_back` now also carries an M4A1 with a red dot and a grenade launcher (`clip` 27, `alt_clip` 1) and 9 frags: they come back as `iw4l:m4` and `iw4l:frag_grenade` with the exact components; the killstreak mirror still comes back as `iw4:weapon/12`.
  - `minecraft_survival` `views::tests::a_friend_sees_a_fork_item_through_slot_nbt` (a renamed Barrett with a thermal scope goes on the wire as a carrot on a stick and comes back whole; menu cells through `menus::stacks` show the Barrett, an empty cell, 16 rockets and a killstreak mirror).
- **Owner's test steps** (two windows with `scripts\mp-local.ps1`, survival, kit below in the `.env` both games read):
  1. The friend opens their inventory: the M4A1, the Barrett, the frags and the rockets show their MW2 icons and names (not a carrot on a stick or a potato). Hover the M4A1: "Red Dot Sight", "Grenade Launcher" and its clip. **Screenshot 1** (friend window).
  2. The friend puts the M4A1 and 8 frags into a chest; the host opens the chest: the same M4A1 (same tooltip) and frags. The host takes the M4A1 out; the friend's chest view updates. **Screenshot 2** (host window with the chest).
  3. The friend renames the Barrett in an anvil (`/xp add @s 5 levels` first): the friend's tooltip shows the new name in italics, and the host sees it when the friend puts it in the chest.
  4. The friend presses Q on the frags: they drop and the host can pick them up as frags.
  5. The friend leaves and joins again: their inventory still holds the fork items (saved in their player file on the host). **Screenshot 3** (friend window after the rejoin).
- **Kit:**

```
IW4L_MINECRAFT_KIT=iw4l:m4[minecraft:custom_data={iw4l:{attachments:["reflex","gl"]}}],iw4l:barrett,iw4l:frag_grenade*16,iw4l:rocket*16,minecraft:chest,minecraft:anvil
```

## Combined kit for group disk-wire

```
IW4L_MINECRAFT_KIT=iw4l:m4[minecraft:custom_data={iw4l:{attachments:["reflex","gl"]}}],iw4l:barrett,iw4l:ak47,iw4l:usp,iw4l:frag_grenade*16,iw4l:rocket*24,minecraft:chest,minecraft:ender_chest,minecraft:shulker_box,minecraft:item_frame*2,minecraft:anvil,minecraft:diamond_pickaxe,minecraft:oak_planks*16
```

## C11.3 MW2 follows the hotbar

### 1. MW2 holds only the guns on the Minecraft hotbar, so a gun put into a chest is not given again

Commit: the one after `781ec027`. This is the fix for the owner's screenshot 12 (guns duplicated by a chest).

- **Changed:**
  - `sim/src/voxel.rs`: `GunSpec`, `Arsenal`, `Spent` (`Rounds`, `Grenade`, `Clip`, `Flare`), `set_arsenal(client, Arsenal)` and `take_spent(client)` as WEAPONS.md 5.1, plus `forget_arsenal`, `has_arsenal`, `is_killstreak_weapon` (`killstreak_*`, `airdrop_*marker*`) and `kept_by_mw2`. Per MW2 client id, in statics next to `DEATH_PROTECTED`, cleared in `deactivate`. A `rebind` the sim has not used yet survives the next `set_arsenal`.
  - `sim/src/item.rs`: `sustain_block_world_arsenal` is now `apply_arsenal` (still run at `step.rs:537`). For each live client that published:
    - Type-0 weapons not in the list are taken (`take_player_weapon`), and missing guns are added with `inventory_add_weapon` (no raise).
    - A removed held gun switches to `selected` (no raise) or to weapon 0.
    - Kept: offhands, alt modes, killstreak weapons and `kept_by_mw2`.
    - Clips: a gun not held gets its spec clips (`-1` means full). The held gun keeps its live clips and queues `Spent::Clip` once when they differ from the spec. `rebind`, a switch to another gun, and creative write the spec into the held gun.
    - Stock stays topped up for the published guns and their alt modes. Grenades and every other kept weapon keep today's full refill. Both are transitional (see Decisions).
    - A client that never published (bots) keeps today's full refill.
    - `BLOCK_WORLD_GUNS` (the free Intervention, SPAS-12 and UMP45) and its `give_weapon_to_ps_akimbo` raise are gone.
  - MW2's own weapon drop and pickups are off for publishing clients:
    - `item::drop_weapon` (the `dropItem` native) returns nothing, and the `anyammoforweaponmodes` native answers false (`sim/src/script/host/natives/player.rs`).
    - `item::try_touch_one` (walking over a dropped weapon) and `item::selected_item` (the use-key swap) skip dropped weapons for them. Scavenger bags still work.
  - `minecraft_survival::guns::settle_arsenal(inventory, mode, spent, (names, content), Hotbar { selected, room, binding }) -> Settled`:
    - It writes `Spent::Clip` into the bound gun item's `custom_data.iw4l`: `clip`, `clip_left` only on akimbo, `alt_clip` only with a grenade launcher or underbarrel shotgun.
    - It lists hotbar slots 0-8 in slot order, each MW2 weapon once. The selected copy is bound, else the first; moving or swapping the bound copy sets `rebind`. The list stops at `room`.
    - A `-1` clip becomes the real size when the gun is bound.
    - It counts ammo items over slots 0-35 and the offhand into `stock`, and sets `endless` in creative.
    - A gun past `room` is inert; selecting it gives `too_many` once.
    - `Binding` (in `Player.arsenal`, per UUID on the host) keeps the bound copies and the hotbar slots MW2 holds.
    - Also new: `is_fork_gun`, `is_gun`, and `Follow::request_with` (the hotbar gun map; a gun arriving in the selected slot is asked for).
  - `render_anim/src/minecraft_fork_items.rs`:
    - `ForkLook::settle` uses the session's `GunNames` and `RegistryGuns`, drains `take_spent` and calls `set_arsenal`. `gun_room` is 15 minus the non-gun weapons in `ps.weapons`.
    - The host calls it next to `sync_weapons` in `minecraft_world.rs`. Each friend calls it in `minecraft_players.rs` `step_friend` through `FriendWorld.guns`: the host's own `ForkLook`, split out of `InventoryUi` with `language_and_fork`, so friends and host share one table.
    - "Too many guns on the hotbar" goes to the host's action bar, and to a friend's as an overlay `System` line.
    - `forget_arsenal` runs when a friend leaves.
  - `owned_weapons` keeps only killstreak weapons, so `sync_weapons` mints and deletes only killstreak mirrors.
  - "A gun is in hand" now means a killstreak mirror or a fork gun MW2 holds (`Binding::holds_gun`). This applies to the shared `movement` (`holding_item`, so the trigger fires), to `tick_actor`'s main hand, and to `gear::body_gear` (any gun). The friend's own window builds the same hotbar map from its `ForkLook` and its `ps.weapons` for `Follow` and `holding_item` (`minecraft_remote.rs`).
  - `fork_items::has_alt_mode` (split out of `with_full_clip`).
  - `docs/WEAPONS.md` 5.1: the real `settle_arsenal` signature, `kept_by_mw2`, the GSC drop and pickup sites, and how rare "too many guns" is.
- **GSC and native sites** (read from the owner's `common_mp.ff`):
  - `dropItem` has one caller, `dropWeaponForDeath` in `maps/mp/gametypes/_weapons.gsc`, called inline from `PlayerKilled_internal` in `_damage.gsc`.
    - For any gun but the riot shield it returns early when `AnyAmmoForWeaponModes` is false (its only caller), and it never checks `dropItem`'s result.
    - For the riot shield it checks `isDefined( item )`.
    - The two natives above take both paths without a runtime error.
  - The pickups are native only (`phase_touch_items` and the use-key swap in `sim/src/item.rs`); no GSC change.
- **Tests:**
  - `sim` `item::arsenal_tests` (new file `crates/sim/src/item/arsenal_tests.rs`, edition 2024, rustfmt-clean):
    - `the_published_guns_become_the_primaries`: an offhand, a grenade launcher alt mode, a killstreak weapon and the AC-130's 105mm stay. The removed held gun switches to the selected one; with nothing selected it goes to weapon 0.
    - `no_intervention_spas_or_ump_is_added`.
    - `a_client_that_never_published_keeps_the_full_refill`.
    - `a_held_gun_queues_its_clip`: a full clip of unknown size is reported with its real size. The held gun keeps its live clip and queues once, and a gun not held gets its spec clip. `rebind` loads the other copy's clip, and creative keeps clips full.
    - `test -p sim`: 121 passed, the MW2-map tests included.
  - `minecraft_survival` `guns::tests`:
    - `a_gun_moved_into_a_chest_is_not_given_again`: a shift-click through `menus::apply` into an open chest, then `settle_arsenal`. No gun is in the list, and there is one M4A1 in all, in the chest.
    - `only_hotbar_guns_reach_mw2_once_each`: hotbar order, a gun in the backpack ignored, both copies mapped, stock counted over the backpack and offhand, creative is endless.
    - `a_gun_past_the_free_weapon_slots_is_inert`, `two_copies_bind_the_selected_one`.
    - `a_clip_spent_is_written_to_the_bound_item`: akimbo and alt clips, and `-1` replaced when bound.
- **Decisions:**
  - **Transitional, as planned:**
    - Stock stays full for the published guns until the ammo group (C11.9), and grenades keep today's refill until the grenade-throws group (C12).
    - MW2's own drop and pickups are off from now on.
    - Death drops and Q for fork guns are unchanged until group drops-loadout.
  - **`kept_by_mw2`:** these weapons are kept like killstreak weapons but not minted as items.
    - The AC-130 gives its gunner `ac130_105mm_mp`, `ac130_40mm_mp` and `ac130_25mm_mp`, and S&D gives `briefcase_bomb_mp` while planting.
    - They are type-0 weapons, so a rule that kept only `killstreak_*` would take them away mid-use.
  - **`settle_arsenal` takes more than WEAPONS.md listed.** WEAPONS.md updated.
    - The session table needs its `GunContent` next to `GunNames`.
    - The binding of copies needs the selected slot and the per-player `Binding`.
    - The room depends on `ps.weapons`, which `minecraft_survival` cannot see.
    - It returns `Settled`: the arsenal, the hotbar gun map and `too_many`.
    - `minecraft_survival` has its own `Arsenal`, `Gun` and `Spent` because it does not depend on `sim`; `render_anim` converts them.
  - **A gun bound for the first time sets `rebind`,** so the item's clip goes into MW2 even when the gun is already held. After that, only a moved, swapped or changed copy sets it.
  - **"Too many guns on the hotbar"** needs at least 7 non-gun weapons in `ps.weapons` with 9 hotbar guns, so it is rare in play; a unit test covers it.
  - No sim test covers the drop and pickup switches: they read the process-wide arsenal statics, which parallel tests would share.
- **Owner's test steps** (survival, a new world, kit below; the kit fills the backpack, not the hotbar):
  1. Join. You hold no MW2 gun, and the mouse wheel and number keys raise none: the free Intervention, SPAS-12 and UMP45 are gone, and the class guns are not given. **Screenshot 1.**
  2. Open the inventory and move the M4A1 to hotbar slot 1 and the USP to slot 2. Close it and press 1: the M4A1 is raised. Press 2: the USP. Press 1 again.
  3. Fire 10 rounds with the M4A1 and stop. Hover it in the inventory: "Clip 20/30". MW2 still refills the reserve for now.
  4. Place the chest, open it and shift-click the M4A1 into it. Close the chest. You no longer hold the M4A1, and pressing 1 raises nothing. The only M4A1 is the one in the chest; the inventory has no second one. **Screenshot 2** (the chest and the inventory open together).
  5. Take the M4A1 back to slot 1 and press 1: it comes up with 20 rounds in the clip.
  6. Put the second M4A1 into slot 4. Press 4: it has a full clip (30). Press 1: 20 again. **Screenshot 3** (the ammo counter after pressing 1).
  7. Die (e.g. `/kill`). No MW2 gun model lies on the ground next to the Minecraft drops, and the log has no "script runtime error" line.
  8. `/gamemode creative` and fire the M4A1: the clip stays full.
  9. Two windows (`scripts\mp-local.ps1`): the friend repeats steps 2 to 6 in their window, and the host sees the friend holding only hotbar guns. **Screenshot 4** (friend window, step 4).
- **Kit:**

```
IW4L_MINECRAFT_KIT=iw4l:m4,iw4l:m4,iw4l:usp,iw4l:ak47,iw4l:rifle_ammo*64,iw4l:pistol_ammo*32,minecraft:chest
```

### 2. Killstreak weapons stay MW2 mirrors and cannot be put into containers

Commit: the one after `18a2bd59`.

- **Changed:**
  - `minecraft_survival` `menus.rs` `keeps_gun` (the guard every menu click passes, host and friends alike, in `apply`) now also refuses an `iw4:weapon/` killstreak item:
    - placed from the cursor into a container cell (`Slot` on `Container`, chests and the bench cells of furnaces, anvils and the like);
    - shift-clicked from the inventory while a block container or a bench is open;
    - swapped into a container cell with a number key (`Swap` on `Container`);
    - spread over cells that include a container cell (`Spread`);
    - dropped from the cursor outside the window (`Outside`).
  - The offhand arms and the in-menu Q (`Throw`) were already there. Q outside menus already put a killstreak item back (`keep_guns`, and the friend's `friend_hand`).
  - Killstreak items are still minted on the first free hotbar slot by `sync_weapons` and never saved. Death is unchanged until group drops-loadout, which makes them vanish.
- **Tests:** `minecraft_survival` `menus::tests::killstreak_items_cannot_go_into_a_chest`: with a chest open, a shift-click, a number-key swap into a cell, a cursor click on a cell, a spread over two cells and a cursor drop outside all leave the killstreak item with the player. Dirt still shift-clicks into the chest, and the chest ends up holding only the dirt.
- **Decisions:** the 2x2 and crafting-table grids are not container cells. An item left there comes back through `close` and `keep_guns`, and `sync_weapons` already counts those cells, so no copy can be made there.
- **Owner's test steps** (survival, kit below; first earn a killstreak, e.g. a UAV after 3 kills on bots or on the friend with the M4A1 moved to the hotbar, since killstreak items cannot be given):
  1. With the UAV item on the hotbar, open a chest. Shift-click the UAV item: it stays in the hotbar. Pick it up with the cursor and click a chest cell: it stays on the cursor. Click outside the window: it stays on the cursor. Put it back. **Screenshot 1** (the chest open, the UAV item still in the hotbar).
  2. Hover a chest cell and press the UAV's hotbar number: nothing moves.
  3. Shift-click dirt into the same chest: it moves as usual.
  4. Two windows: the friend repeats step 1 with their own killstreak item. **Screenshot 2** (friend window).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:chest,minecraft:dirt*16,iw4l:m4,iw4l:rifle_ammo*64
```

## Combined kit for group arsenal

```
IW4L_MINECRAFT_KIT=iw4l:m4,iw4l:m4,iw4l:usp,iw4l:ak47,iw4l:rifle_ammo*64,iw4l:pistol_ammo*32,minecraft:chest,minecraft:dirt*16
```

## C11.4 Drops, look on the ground, first-join loadout

### 1. Guns drop on death and with Q and go into chests like any item

Commit: the one after `188c2f61`.

- **Changed:**
  - `minecraft_survival` `death.rs`: the predicate of `death_drops` now names the items that are destroyed (like Curse of Vanishing), not the ones that stay. New `dropped_on_death(death, inventory)`: nothing with `keep_inventory`, else `death_drops` with "killstreak mirrors (`iw4:weapon/`) vanish". Fork guns, ammo, attachments and grenades drop in vanilla order with their components. `render_anim` `drop_on_death` (the one function the host and every friend use) calls it.
  - `keep_guns` and the `Throw`, container and cursor arms of `keeps_gun` (`menus.rs`) were already limited to killstreak mirrors (`weapon_of` matches only `iw4:weapon/`), so Q, the in-menu Q, a cursor drop and every container cell take fork guns like any item. No change was needed there.
  - **The offhand ban now covers fork guns** (the riot shield too; it is a gun): the three offhand arms of `keeps_gun` use `guns::is_gun`, and a new arm refuses a drag-spread of a gun that includes the offhand; `offhand::swap_hands` (F, host and friends) uses `is_gun`; creative `set_slot` refuses a gun in menu slot 45 (the offhand) with `BadSlot`; the host's drag targets (`render_anim` `minecraft_inventory.rs` `droppable`) leave the offhand out for a fork gun. A grenade, ammo or an attachment may sit in the offhand.
  - **Netherite guns survive fire and lava on the ground.** Vendored `minecraftoss_world` `level/burning.rs` (a fork-added file) gains `add_fire_resistant_items(ids)`, a process-wide list read after its `FIRE_RESISTANT_ITEMS`; `ServerSim::new` registers `fork_items::fire_proof_ids()` (the Barrett and the M240). A `minecraft:damage_resistant` or `!minecraft:damage_resistant` component still decides first, as in vanilla.
- **Tests:**
  - `minecraft_survival` `death::tests::death_drops_follow_vanilla_order_drop_guns_and_destroy_killstreaks_and_vanishing_items` (renamed from `..._keep_guns_...`): an M4A1, rifle ammo and frags drop in slot order; the killstreak mirrors in the hotbar and the crafting table vanish; the inventory ends empty.
  - `death::tests::keep_inventory_keeps_guns_and_killstreaks_and_a_normal_death_drops_only_the_guns`: with `keep_inventory` nothing drops or changes; without it the M4A1 drops with its clip and the killstreak mirror is gone.
  - `menus::tests::fork_guns_drop_with_q_go_into_chests_and_stay_out_of_the_offhand`: a fork gun on the cursor is refused by the offhand cell and by a spread over it, F swaps of the riot shield are refused both ways, Q (`Throw`) drops the M4A1 with its clip, a shift-click puts the USP into a chest and a cursor drop outside throws a gun.
  - `offhand::tests::swapping_hands_trades_the_selected_slot_with_the_offhand_but_never_a_gun` also covers the M4A1 and the riot shield both ways and lets a frag swap.
  - `creative::tests::a_gun_never_goes_into_the_creative_offhand`.
  - `minecraft_terrain` `server::tests::a_netherite_gun_survives_lava_and_other_guns_burn` (26.3 data): the Barrett floats in lava with its clip, the M4A1 burns up.
  - Vendored `minecraftoss_world` `level::burning::tests::added_items_resist_fire_like_netherite`.
- **Decisions:**
  - **Killstreak items are destroyed on death, not kept.** Before this commit they stayed in their slot. MW2 still owns the weapon while dead, so `sync_weapons` mints the mirror again on the first free hotbar slot if MW2 keeps it after the respawn (MW2's own rule for earned streaks); nothing of it ever drops.
  - **Netherite fire resistance is a hook in the vendored burning code,** not a component added to the in-memory gun: items reach the level from many places (Q, death, broken chests, hoppers), and the hook covers them all without changing the item.
- **Owner's test steps** (survival, a new world, kit below; the kit fills the backpack):
  1. Move the M4A1 to hotbar slot 1 and fire a few rounds. Press Q: the M4A1 drops (no MW2 gun is held any more). Pick it up again: its tooltip shows the same clip. **Screenshot 1** (the tooltip after pickup).
  2. Open the inventory, pick up the riot shield and click the offhand slot: it stays on the cursor. Put the USP in hotbar slot 2, select it and press F: nothing swaps. Put a frag in hotbar slot 3 and press F: the frag goes into the offhand. **Screenshot 2.**
  3. Place a chest and shift-click the AK-47 and the USP into it: both go in. Take them out again.
  4. Dig a hole two blocks wide, pour the lava bucket into it, and press Q on the Barrett and then the M4A1 while looking into the lava: the M4A1 burns up, the Barrett floats on the lava and does not burn. **Screenshot 3** (the Barrett on the lava).
  5. `/kill`: the guns, the ammo and the frags drop and can be picked up (they may still look like plain items until the next commit). Respawn: no gun is given back.
  6. `/gamerule keep_inventory true`, `/kill`: everything stays in its slot. `/gamerule keep_inventory false`.
  7. `/gamemode creative`, open the creative inventory tab, pick up the M4A1 and click the offhand slot: it stays on the cursor. `/gamemode survival`.
  8. Two windows: the friend repeats steps 1, 2 and 5 in their window, and the host sees the friend's dropped gun lying on the ground. **Screenshot 4** (friend window).
- **Kit:**

```
IW4L_MINECRAFT_KIT=iw4l:m4,iw4l:ak47,iw4l:usp,iw4l:barrett,iw4l:riotshield,iw4l:rifle_ammo*64,iw4l:pistol_ammo*32,iw4l:frag_grenade*8,minecraft:chest,minecraft:lava_bucket,minecraft:cobblestone*16
```

### 2. Dropped fork items and fork items in item frames are drawn as their icon

Commit: the one after `34a1697f`.

- **Changed:**
  - `minecraft_terrain` `pack.rs`: `PackStack` gains an in-memory overlay (`put(path, bytes)`), read before the packs and kept across a pack reload. Fork models and icons go there at runtime; nothing is written to disk.
  - `minecraft_terrain` `fork_look.rs` (new, edition 2021, rustfmt-clean), all at the model-resolve level, so every renderer that resolves an item model through the pack (ground items and item frames through `ItemVisuals`, the held non-gun item in the hand) draws fork items without a renderer change:
    - `reserve(packs)`: ammo, gun parts and rapid fire get an item definition that reuses the vanilla item's model (`iron_nugget`, `fire_charge`, `firework_rocket`, `iron_bars`, `tripwire_hook`, `repeater`) with the table's tint as a `minecraft:constant` tint, so they are extruded and tinted like the inventory icon. Guns, grenades and the other attachments get a blank 16x16 texture `iw4l:item/<path>` so the world atlas reserves a cell for them.
    - `install(packs, id, sprite)`: the icon as `iw4l:item/<path>` (PNG), the model `{"parent": "minecraft:item/generated", "textures": {"layer0": ...}}` and the item definition. From then on `resolve_item_model` gives vanilla's extruded `item/generated` geometry with the `ground` and `fixed` display transforms of `item/generated`.
    - `sprite(icon, flip)`: the MW2 picture fitted into 16x16, centred, alpha cut at 128 (vanilla item sprites are 16 px and cut out).
    - `patch(atlas, id, sprite)`: the `AtlasPatch` (64 px tile and its 4 mip levels) that writes the icon into its reserved atlas cell on the GPU.
  - `mesh.rs` (one loop, 3 lines): the world atlas reserves `fork_look::textures()` next to the other preloaded sprites.
  - `render_anim` `minecraft_fork_items.rs` `ForkLook::dress_world`: once the MW2 pictures are decoded (the class-menu pictures it already decodes for the inventory, plus the guns' kill icon images, `kill_icon_image_of` or `kill_icon_of`, decoded from the owner's IWDs on a worker thread, mirrored when the weapon's kill icon is flipped), it installs every icon into the pack overlay and queues the atlas patches; it installs again when a new world look (a new `PackStack`) appears and patches again when the atlas or the view generation changes or the view becomes active. A picture that cannot be decoded uses a stand-in (`crossbow_standby` for a gun, `firework_star` for the rest), logged once by id.
  - Host: `load_look` calls `fork_look::reserve` before the atlas is built (the host and the friend both use it); `dress_world` runs next to `set_weapons` in `minecraft_world.rs` and in the friend's frame in `minecraft_remote.rs` (one call each).
  - No edit to `render_anim/src/anim/minecraft_gear.rs` (track A's file) or to `item_frame.rs`.
- **Tests** (`minecraft_terrain` `fork_look::tests`, with stand-in model JSON written by the test, no Mojang file):
  - `a_fork_id_resolves_to_a_generated_model_with_its_icon_sprite`: before `install` the M4A1 has no model; after it the model is the extruded sprite (1 front-and-back element and 12 edge elements for a 4x2 block of pixels), every face on `iw4l:item/m4`; ammo and non-fork ids are refused.
  - `ammo_and_parts_use_their_vanilla_sprite_with_the_tint`: rifle ammo resolves to the iron nugget with the copper `constant` tint on tint index 0; the barrel has no tint.
  - `guns_grenades_and_attachments_have_icon_textures_and_sprites_do_not`, `a_sprite_is_fitted_flipped_and_cut_out`, `a_patch_covers_the_reserved_atlas_tile_with_every_mip_level` (origin on the tile corner, 64/32/16/8/4 levels, no missing texture after `reserve`).
- **Decisions:**
  - **The icons live in an in-memory pack overlay, not in a renderer special case,** so ground items, item frames and the held non-gun item all take the same vanilla path (`item_definition` -> `item/generated`), and a later resource pack could even replace them by giving `iw4l` models.
  - **The GPU atlas is patched instead of rebuilt.** The world atlas is built when the world loads, before MW2's weapons and pictures may be ready; a rebuild would take seconds. The cells are reserved up front and written with the existing `atlas_patches` path (the one animated textures use). The CPU copy of the atlas keeps the blank cell; nothing reads fork pixels from it.
  - **A fork item is not drawn on the ground until its pictures are decoded** (a second or two after MW2's weapons load, as before this commit, when it was never drawn); ammo, parts and rapid fire are drawn at once.
  - **Stand-ins** for a picture the owner's files lack, so no item is ever invisible on the ground.
- **Owner's test steps** (survival, a new world, kit below):
  1. Press Q on the M4A1, the Intervention, the frags, the red dot sight, the rifle ammo and the gun barrel, one by one, so they lie in a row. Each lies on the ground as its inventory picture, a few pixels thick and spinning like a dropped vanilla item: the guns as their MW2 kill icon, the frag as its class-menu picture, the ammo as a copper nugget, the barrel as iron bars. **Screenshot 1.**
  2. Place two item frames and put the M4A1 and the frags into them: they show the same pictures flat on the wall. **Screenshot 2** (both frames).
  3. Hold the frags or the rifle ammo in the main hand: they are drawn in the hand like a vanilla item (no MW2 gun is raised). Hold the M4A1: MW2 draws the gun as before.
  4. Save and quit, load again: the dropped items and the framed items look the same.
  5. Two windows: the friend sees the dropped items and the framed items the same way, and drops their own gun, which the host sees as its kill icon. **Screenshot 3** (friend window).
  6. If the log has "Minecraft fork items with a stand-in look on the ground: ...", send that line: those items lie as a crossbow or a firework star.
- **Kit:**

```
IW4L_MINECRAFT_KIT=iw4l:m4,iw4l:cheytac,iw4l:usp,iw4l:frag_grenade*8,iw4l:flash_grenade*4,iw4l:red_dot_sight,iw4l:rifle_ammo*32,iw4l:gun_barrel*2,iw4l:rapid_fire,minecraft:item_frame*4,minecraft:oak_planks*16
```

### 3. A player gets the class guns and three magazines once, on their first join

Commit: the one after `5aa90e2b`.

- **Changed:**
  - `minecraft_survival` `loadout.rs` (new, edition 2024, rustfmt-clean): `give_first_loadout((inventory, selected, tags), class, (names, content))`, the one function the host runs for itself and for every friend.
    - It gives nothing without a class (no `LoadoutSpec`, or neither a primary nor a secondary) or when the vanilla `Tags` list already holds `iw4l_loadout_given` (`GIVEN_TAG`). Otherwise it adds the tag through `minecraft_commands::tag::add`, so other tags stay (and a player with 1024 tags gets nothing instead of a loadout every frame).
    - The primary and the secondary come from the class's weapon indices through `GunNames::item`, which splits a variant or configured weapon with `WeaponFamilies::describe` (`describe_configuration`) into the fork id and its attachments. Each gun goes into hotbar slot 0 (primary) or 1 (secondary) when free, otherwise through vanilla's `add_item`. It carries its attachments in `custom_data.iw4l.attachments` and full clips (`with_full_clip`: `clip` -1, `clip_left` -1 with akimbo, `alt_clip` -1 with a grenade launcher or underbarrel shotgun; the settle writes the real sizes when it binds the gun).
    - Three magazines of reserve per gun (`MAGAZINES`, from the clip size of the class's configuration, so an extended mag counts). The rounds of guns that share an ammo type add up. They go into the backpack first (merging into the same ammo, then slots 9-35, then the hotbar), in stacks of the table's size.
    - What does not fit is returned and dropped at the player's feet with `Entities::drop_given`, the way `/give` does it.
    - A class gun outside the fork table becomes the first table gun of its category (`stand_in`: M4A1, MP5K, RPD, M21 EBR, M1014, G18, USP .45, AT4-HS), and its MW2 name is returned in `stood_in`. A weapon whose category is unknown becomes the M4A1 (primary) or the USP .45 (secondary).
    - `Class` already carries `lethal` and `tactical`: the grenade group (C12) adds the 32 + 32 grenades in this same function, after the ammo.
  - `GunContent` gains `bucket(index)` (default `None`). `render_anim`'s `RegistryGuns` maps MW2's `weapClass` (rifle, sniper, MG, SMG, spread, pistol, rocket launcher) to the fork bucket.
  - `render_anim` `minecraft_fork_items.rs`: `class_of(authority, client)` reads the client's `LoadoutSpec` (`client_meta(..).loadout`, not `ps.weapons`). `ForkLook::first_loadout` runs `give_first_loadout` with the session's `GunNames` once MW2's weapons are loaded and the player is alive, and logs a stood-in gun as a warning ("Minecraft first-join loadout: no item for ...").
  - Host: called in `minecraft_world.rs` right before `sync_weapons` and the settle, so the guns are bound in the same frame. Each friend: in `minecraft_players.rs` `step_friend` right before the same two calls, with the friend's own inventory, tags and feet.
- **Tests** (`minecraft_survival` `loadout::tests`, with a stand-in `GunContent`):
  - `the_class_guns_and_three_magazines_are_given_once_and_the_tag_is_set`: the M4A1 in slot 0 (stack size 1, full clip) and the USP .45 in slot 1, 90 rifle rounds (64 + 26) and 36 pistol rounds in the backpack, the rest of the hotbar empty, the tag added after a foreign tag that stays. A second call gives nothing and changes nothing.
  - `no_class_or_an_empty_class_gives_nothing_and_sets_no_tag`.
  - `class_attachments_become_item_attachments_and_akimbo_fills_both_clips` (red dot and silencer on the M4A1, an akimbo USP with `clip` and `clip_left`).
  - `shared_ammo_adds_up_and_a_taken_slot_uses_the_vanilla_add_order` (MP5K and USP: 126 pistol rounds as 64 + 62; slot 0 held dirt, so the guns go to slots 1 and 2).
  - `what_does_not_fit_is_dropped` (a full inventory: both guns and both ammo stacks come back to be dropped, and the tag is still set).
  - `a_gun_outside_the_table_becomes_the_first_gun_of_its_category` (an unknown shotgun becomes the M1014 with 12 shells, an unknown-category secondary the USP; every stand-in is a gun of its bucket).
- **Decisions:**
  - **The ammo goes into the backpack first** (WEAPONS.md 6 says "into the backpack"): vanilla's add order would put it on hotbar slots 2-8 next to the guns. Only when the backpack is full does it use the hotbar, and after that it drops.
  - **An akimbo pair gets three magazines of one gun,** like any other gun: "three magazines per gun" counts the item, not the two hands.
  - **The tag is added before the items** (same frame, same save), so a full tag list can never make the loadout repeat. Removing the tag with `/tag` while alive gives the loadout again at once (the rule is "alive, with a class, without the tag"), not only after the next death.
  - **Unknown category:** a class weapon that is neither in the table nor of a known MW2 weapon class becomes the slot's default gun (M4A1 or USP .45), so the player always gets two guns.
  - The stood-in name is logged each time a loadout is given, which is once per player.
- **Owner's test steps** (a new world `IW4L_MINECRAFT_WORLD=r9-loadout`, survival, an MW2 class with an M4A1 with a red dot sight and a USP .45; no kit):
  1. Join: hotbar slot 1 holds the M4A1 and slot 2 the USP .45; the backpack holds 90 rifle ammo (64 + 26) and 36 pistol ammo. No Intervention, SPAS-12 or UMP45. Hover the M4A1: "Red Dot Sight", "Clip 30/30". **Screenshot 1.**
  2. Press 1 and fire: the M4A1 shoots. `/tag @s list` lists `iw4l_loadout_given`.
  3. Save and quit, load again: no second loadout. **Screenshot 2.**
  4. `/kill`: the guns and the ammo drop; the respawn gives nothing.
  5. `/tag @s remove iw4l_loadout_given`: the loadout comes once more, right away.
  6. Load a world played in round 8 (its player file has no tag): the class guns and ammo arrive once, next to what was there.
  7. Pick a class with an akimbo pistol and start another new world: the pistol shows "Clip 12/12 + 12/12".
  8. Two windows (`scripts\mp-local.ps1`): the friend's first join gives the friend's own class guns and ammo in the friend's slots, and the host's inventory is unchanged. The friend leaves and joins again: nothing new. **Screenshot 3** (friend window).
- **Kit:** none (the loadout is the test).

## Combined kit for group drops-loadout

```
IW4L_MINECRAFT_KIT=iw4l:m4,iw4l:ak47,iw4l:usp,iw4l:barrett,iw4l:cheytac,iw4l:riotshield,iw4l:rifle_ammo*64,iw4l:pistol_ammo*32,iw4l:frag_grenade*8,iw4l:flash_grenade*4,iw4l:red_dot_sight,iw4l:gun_barrel*2,iw4l:rapid_fire,minecraft:chest,minecraft:lava_bucket,minecraft:cobblestone*16,minecraft:item_frame*4,minecraft:oak_planks*16
```

## C11.5 Recipes

### 1. Recipe results keep their components, so crafted suspicious stew has its effect

Commit: the one after `639ea393`.

- **Changed:**
  - Vendored `minecraftoss_player::crafting::parse_result` keeps `result.components` on the result stack (an empty object counts as none). Before, every crafting and cooking result was a bare id and count, so the 17 vanilla suspicious stew recipes (the only 26.3 crafting results with components) gave a stew without `minecraft:suspicious_stew_effects`. Now a crafted stew carries the same component a mooshroom's stew does, and it is saved and sent to friends with it.
  - Fork recipes (next commit) need the same path: a recipe result is cloned whole, so whatever its JSON says stays on the crafted item.
- **Tests:** vendored `crafting::tests::a_recipe_result_keeps_its_components`: a parsed shapeless result keeps its components and an empty object is none; with the 26.3 jar (`MINECRAFTOSS_ROOT`) bowl, red and brown mushroom and allium in the 3x3 grid give `minecraft:suspicious_stew` with fire resistance for 60 ticks, an oxeye daisy gives regeneration, and oak planks still have no components.
- **Decisions:**
  - **Eating a suspicious stew still applies no effect.** The fork's eating code (`survival.rs` `apply_consumed_food_effects`) has no suspicious stew arm, for mooshroom stews either; that is an open limitation from before this track and outside the recipes group. The stew now carries the effect, so adding the arm later needs no data change.
- **Owner's test steps:** nothing new to see in the game: the stew's tooltip shows no effects in the fork, and eating it applies none yet (see above). Craft one to check nothing broke:
  1. Crafting table: a bowl, a red mushroom, a brown mushroom and an allium make a suspicious stew. Eat it: hunger fills as before. **Screenshot 1** (the stew in the result slot).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:crafting_table,minecraft:bowl*4,minecraft:red_mushroom*4,minecraft:brown_mushroom*4,minecraft:allium*2,minecraft:oxeye_daisy*2
```

### 2. Guns, ammo, grenades, attachments and gun parts have crafting recipes

Commit: the one after `47a945a3`.

- **Changed:**
  - `minecraft_terrain/data/fork_recipes.json` (new): our own vanilla-format recipes, one object keyed by recipe id (the result's `iw4l:` id), one recipe per line, 94 in all: every row of WEAPONS.md 7.2-7.6 (the two parts, templates A-G with each gun's F and X cells, the riot shield, the explosive crossbow and the ballistic knife with their own shape), 8.1 (ammo, shapeless), 9.1 (attachments, shaped) and 10.1 (grenades and the tactical insertion, shapeless). All `category: "equipment"`, groups as WEAPONS.md 13.1 (`iw4l_parts`, `iw4l_pistols`, `iw4l_machine_pistols`, `iw4l_smgs`, `iw4l_assault_rifles`, `iw4l_shotguns`, `iw4l_specials`, `iw4l_snipers`, `iw4l_lmgs`, `iw4l_launchers`, `iw4l_ammo`, `iw4l_lethal`, `iw4l_tactical`, `iw4l_attachments`). Ingredients name `iw4l:gun_barrel` and `iw4l:firing_mechanism` directly. No result has components, so a crafted gun has no clip (empty). Written by a scratch script from the WEAPONS.md tables; nothing is copied from the jar.
  - `minecraft_terrain` `fork_recipes.rs` (new, edition 2021, rustfmt-clean): `JSON` (`include_str!`), `recipes()` (id and recipe in fork table order, whatever the map order), `group(row)` (the 13.1 group of a row).
  - Vendored `minecraftoss_player::crafting`: `RecipeBook::with_extra(recipes)` parses each with `parse_recipe`, skips a malformed recipe or an id the book already has, appends it after vanilla's (so `matching` tries vanilla first) and gives it the next display index: after the highest index of a loaded display catalog, or, without one (the game loads none), after vanilla's crafting and cooking recipe count. `parse_recipe` now reads `category` and `group` itself (`from_jar` still sets them the same way).
  - `crafting::load_recipes` calls `with_extra(fork_recipes::recipes())` right after `from_jar`, before `with_item_catalog`, so the results get the fork stack sizes (guns 1, rockets and explosive rounds 16). `GameData::load` uses it on the host (`minecraft_world.rs`) and on the friend (`minecraft_remote.rs`), so both books hold the fork recipes; the host computes every grid result.
  - `fork_items::table_hash` also hashes the recipe JSON, so a friend whose build has other recipes is refused by `registry_fingerprint`.
  - Fork recipes are never unlocked and never saved: auto-unlocks come only from vanilla's recipe advancements, which never name an `iw4l:` id, and `recipeBook` is still only carried over. The recipe book group (C11.7) shows them as always known.
  - `docs/WEAPONS.md` 7.1 (the file's form and its tests) and 13.1 (the display index rule, the hash).
- **Tests:**
  - `minecraft_terrain` `fork_recipes::tests`:
    - `every_fork_item_has_exactly_one_recipe_of_its_kind_and_group`: 94 recipe lines and 94 recipes in table order, one per row, keyed by the result id, `equipment`, the row's group, no result components, shaped for guns, parts and attachments, shapeless for ammo and grenades, ingredients only `minecraft:`, `#minecraft:` or a gun part.
    - `yields_follow_the_design` (barrel 2, ammo 24/16/32/8/4/2/2, grenades 2, everything else 1), `the_m4a1_recipe_is_the_design_example` (WEAPONS.md 7.1 exactly).
    - With the 26.3 jar (`MINECRAFTOSS_ROOT`): `every_ingredient_exists_in_the_jar_or_the_table` (item definitions under `assets/minecraft/items`, item tags, or a fork id; `minecraft:iron_chain` exists); `no_fork_grid_matches_a_vanilla_grid_or_another_fork_grid` (reads the jar's 827 shaped, 375 shapeless, 33 transmute and 6 dye recipes and the tipped arrow's imbue ring, expands tags, compares trimmed shaped grids both ways round, shapeless lists by a perfect matching and shaped against shapeless by their cells, every fork recipe against every vanilla one and every other fork one; a mirrored shears grid must collide, to prove the check works); `fork_display_indices_come_after_vanilla` (the 94 indices start at vanilla's crafting plus cooking count and run on in table order, no vanilla index is above them, the book holds 94 fork recipes, and picking up iron, gunpowder or a barrel unlocks vanilla recipes but no fork one); `crafting_an_m4a1_gives_an_empty_gun_that_stacks_to_one` (three iron make 2 barrels that stack to 64; the M4A1 grid mirrored in the 3x3 gives `iw4l:m4`, count 1, max 1, no components, and taking it empties the grid; rifle ammo and rockets craft in the 2x2 with max 64 and 16).
    - I made the jar assertions fail on purpose once (827 to 828, max 1 to 2, the first index plus 1) to see that they really run; all three failed, then were put back.
  - Vendored `crafting::tests::extra_recipes_get_display_indices_after_vanilla` (indices after the highest catalog index, a known id and a malformed recipe skipped, category and group kept, an extra recipe matches mirrored, a book with no catalog starts at its recipe count, and a changed item unlocks no extra recipe).
- **Decisions:**
  - **No WEAPONS.md recipe collides and every ingredient exists in 26.3.** The test reproduces the design script's numbers (827, 375, 33, 6) and finds no clash, so no recipe changed. The special recipes other than the tipped arrow (firework rocket and star, banner duplicate, shield decoration, book cloning, map extending, repair, decorated pot) have no grid in their JSON; WEAPONS.md's hand check of them stands (none takes a gun part, and none accepts the ingredient mixes used here).
  - **The JSON is one object keyed by recipe id,** not one file per recipe as in a data pack: one `include_str!`, and the ids read like vanilla's (`iw4l:m4` next to `minecraft:iron_sword`). Order comes from the fork table, so the display indices follow the creative order.
  - **Display indices without a catalog:** the game loads no recipe display catalog, so vanilla's recipes have no index; fork recipes start after vanilla's crafting and cooking count so they can never meet a vanilla index if a catalog is added later (stonecutting and smithing are not in this book).
  - **With no item catalog** (the warned, degraded mode) fork recipes still load, and their results stack to 64 like every item there (WEAPONS.md 4.4).
  - **Crafting statistics** (`minecraft:crafted` for `iw4l:` ids) are queued like any item's and dropped like every other stat event today (`take_stat_events` is discarded), so no fork id reaches a stats file.
- **Owner's test steps** (survival, any world, kit below; the recipe book screen comes with C11.7, so place the items by hand; "row 1" is the top row of the crafting table):
  1. Crafting table, row 1: three iron ingots: 2 Gun Barrels. Craft once more (4 barrels). **Screenshot 1** (the result slot).
  2. Row 1 middle: a tripwire hook; row 2: iron ingot, redstone, iron ingot: 1 Firing Mechanism. Make 3.
  3. Row 1: gun barrel, iron block, repeater; row 2: (empty), firing mechanism, iron ingot: an M4A1. Take it and hover it: "Clip 0/30". **Screenshot 2** (the tooltip). Put it on the hotbar and press its number: it is raised empty; reloading fills it (the reserve still refills until the ammo items group).
  4. The same M4A1 layout mirrored left to right (repeater, iron block, barrel; iron ingot, mechanism, empty) also makes an M4A1.
  5. Row 1: barrel, mechanism, stone button; row 2 middle: iron ingot: a USP .45. With a lever instead of the button: nothing (no gun has that grid).
  6. Row 1: barrel, blaze rod, iron block; row 2: iron chain, mechanism, repeater; row 3 middle: an oak log: an RPD. **Screenshot 3.**
  7. Inventory 2x2 grid: copper ingot, iron nugget and gunpowder in any cells: 16 Rifle Ammo. TNT, fire charge and iron ingot: 2 Rockets (a stack of 16 at most). Iron ingot, iron nugget and two gunpowder: 2 Frag Grenades. Gunpowder, glowstone dust and iron nugget: 2 Flash Grenades. **Screenshot 4** (the 2x2 with the rockets).
  8. Crafting table: row 1 middle a redstone torch, row 2 iron nugget, glass pane, iron nugget: a Red Dot Sight. Iron ingot, redstone torch and crying obsidian (any cells): 1 Tactical Insertion.
  9. Vanilla still works: two iron ingots in a row make a heavy weighted pressure plate, an oak log makes planks.
  10. Two windows (`scripts\mp-local.ps1`, both with the kit): the friend crafts gun barrels and an M4A1 the same way. **Screenshot 5** (friend window with the M4A1 tooltip).
- **Kit** (26 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:crafting_table,minecraft:iron_ingot*64,minecraft:iron_block*4,minecraft:copper_ingot*32,minecraft:gunpowder*64,minecraft:iron_nugget*32,minecraft:tripwire_hook*8,minecraft:redstone*16,minecraft:repeater*4,minecraft:stone_button*4,minecraft:lever*2,minecraft:tnt*4,minecraft:fire_charge*4,minecraft:glowstone_dust*4,minecraft:redstone_torch*4,minecraft:glass_pane*8,minecraft:crying_obsidian*2,minecraft:blaze_rod*2,minecraft:iron_chain*2,minecraft:oak_log*8,minecraft:paper*8,minecraft:slime_ball*4,minecraft:spyglass*2,minecraft:diamond*4,minecraft:stick*8,minecraft:leather*4
```

## Combined kit for group recipes

```
IW4L_MINECRAFT_KIT=minecraft:crafting_table,minecraft:iron_ingot*64,minecraft:iron_block*4,minecraft:copper_ingot*32,minecraft:gunpowder*64,minecraft:iron_nugget*32,minecraft:tripwire_hook*8,minecraft:redstone*16,minecraft:repeater*4,minecraft:stone_button*4,minecraft:lever*2,minecraft:tnt*4,minecraft:fire_charge*4,minecraft:glowstone_dust*4,minecraft:redstone_torch*4,minecraft:glass_pane*8,minecraft:crying_obsidian*2,minecraft:blaze_rod*2,minecraft:iron_chain*2,minecraft:oak_log*8,minecraft:bowl*2,minecraft:red_mushroom*2,minecraft:brown_mushroom*2,minecraft:allium*2,minecraft:spyglass*2,minecraft:diamond*4
```

## C11.6 Attachments, akimbo, the grindstone

### 1. Attachments go onto a gun in the crafting grid, and two identical guns make an akimbo pair

Commit: the one after `afafe0c5`.

- **Changed:**
  - `minecraft_terrain` `gun_mods.rs` (new, edition 2021, rustfmt-clean): the trait `GunRules` (`accepts(id, attachments)`, `clip_size(id, attachments)`), the item state helpers `is_gun`, `attachments`, `clip`, `clips`, `with_state` (rewrites `custom_data.iw4l` and keeps every other component; an empty state leaves no empty `custom_data`) and `refit` (new attachments; a full clip `-1` becomes the old clip size when it is known; a clip above the new size is cut and the extra rounds are counted; `clip_left` is kept only with `akimbo`, `alt_clip` only while the gun keeps its alt mode).
  - `minecraft_survival` `gun_crafting.rs` (new, edition 2024, rustfmt-clean): `Rules(&impl GunContent)` (the MW2 check through `WeaponFamilies::resolve` with at most 2 attachments, the way `GunNames` uses it), `craft(cells, rules)`, `result(inventory, grid, rules)` and `take(inventory, grid, rules, place)`:
    - **Attach:** exactly one gun plus 1 or 2 attachment items in any cells, nothing else. Each item tries its MW2 names in table order (`reflex`/`elbit`, `silencer`/`silencer02`/`silencer03`, `xmags`/`extclip`/`dualclip`, `rof`/`rf`/`auto`, `acog`/`hamrhybrid`/`hybrid`, `gl`/`gp`, `shotgun`/`mk`, `thermal`/`ir`) and the first combination MW2 resolves wins. No result for a third attachment, an attachment the gun already has, the same item twice, a combination MW2 has no def for (two sights) or any other item in the grid. The gun keeps its name, its clip (cut to the new size) and its other attachments; one item is taken from each attachment cell.
    - **Akimbo:** two copies of the same gun with the same attachments (in any order), nothing else, at most one attachment so far, and MW2 resolves the gun with `akimbo` added: one gun with `akimbo`, `clip` from the first gun in grid order and `clip_left` from the second.
    - **Split:** an akimbo gun alone in the grid gives the gun without `akimbo` with its `clip`; the second gun stays in the same cell with `clip_left` as its clip.
    - Rounds above a smaller clip become the gun's ammo items, added to the inventory in vanilla's add order; what does not fit drops at the feet.
    - The result goes onto the empty cursor (click), into the inventory from the right end of the hotbar like vanilla's shift-click, into an empty hotbar slot (number key; never the offhand) or onto the ground (Q). A full cursor takes nothing.
  - `menus.rs`: `Books` gains `guns: Option<&dyn GunRules>` (`Books::of` sets none, `with_guns` adds it). `grid_action` tries the special recipe first for a click, a shift-click, a number key and Q on the result slot, and falls back to vanilla's crafting when the grid holds no special recipe. The host passes the rules for its own clicks (`minecraft_world.rs`, also when it leaves a level) and for every friend's clicks (`minecraft_players.rs` `friend_menus`), so both go through the same function.
  - `render_anim` `minecraft_fork_items.rs`: `ForkRules` (the registry's rules), `ForkLook::rules` and `ForkLook::special`; `minecraft_inventory.rs` shows the special result in the 2x2 and the crafting table before vanilla's result, on the host's and the friend's screen (each from its own MW2 content). Attachment tooltips say "Craft together with a gun to attach it". When MW2 content arrives, the game logs once the content's attachment names that have no item.
  - `minecraft_terrain` `crafting.rs`: `result_order` and `move_into` are public, so the shift-click of a special result uses vanilla's order.
  - `docs/WEAPONS.md` 9.2: where the special recipes run and how the MW2 check reaches them.
- **Tests:**
  - `minecraft_survival` `gun_crafting::tests`: `attaching_keeps_the_clip_the_name_and_the_other_attachments`, `a_smaller_clip_gives_the_extra_rounds_back_as_ammo`, `two_sights_a_third_attachment_or_the_same_one_twice_give_nothing` (also three items, dirt in the grid and a gun alone), `a_t5_gun_takes_the_attachment_under_its_own_name` (the Galil gets `elbit`), `two_identical_guns_make_an_akimbo_pair_with_both_clips` (full `-1` clips; different attachments, an M4A1 pair and a mixed pair give none), `an_akimbo_gun_alone_splits_and_the_second_gun_stays_in_its_cell`, `the_result_goes_to_the_hotbar_the_inventory_or_the_ground_and_never_onto_a_full_cursor` (the offhand refused), `a_click_on_the_result_crafts_through_the_menu` (through `menus::apply`, and no special result without rules), `content_attachment_names_without_an_item_are_listed` (reads `frame/src/class_presets.rs`, prints the names with no item and fails only if a mapped name is missing; today it lists `speed`).
  - `minecraft_terrain` `gun_mods::tests`: `a_smaller_clip_gives_the_extra_rounds_back_and_keeps_every_other_component`, `a_bare_state_leaves_no_empty_custom_data`.
- **Decisions:**
  - **The content test reads the class presets, not the MW2 zones.** No unit test in the workspace loads a weapon registry from the owner's content (that needs the full session load), so the test lists the preset classes' attachment names, and the game logs the content's own unmapped names once when the registry arrives (`MW2 attachments with no Minecraft item`).
  - **The first gun in grid order is the right-hand gun** of an akimbo pair: its clip is `clip`, and its components (custom name) are kept. The second gun's own name is lost; a split gives both guns the pair's name.
  - **Shift-click crafts once:** a special recipe uses up the gun, so it cannot repeat like vanilla's shift-click.
  - **A new alt attachment comes empty** (`alt_clip` absent), like a crafted gun; the grindstone drops its rounds with it.
  - **No MW2 content, no special result** (the grid shows nothing, as for any invalid grid), because the combination and the clip size can only come from MW2.
  - No MW2 attachment makes a clip smaller, so in the game the "rounds back" path matters only for content that does; the grindstone (next commit) is where clips really shrink.
- **Owner's test steps** (survival, any world, kit below; the kit gives every gun a full clip):
  1. Open the inventory. Put an M4A1 and a Red Dot Sight anywhere in the 2x2 grid: the result is the M4A1. Hover it: "Red Dot Sight", "Clip 30/30". Take it. **Screenshot 1** (the tooltip).
  2. Crafting table: the second M4A1, a Red Dot Sight and an ACOG Scope: no result (two sights). Take the ACOG out: the result is the M4A1 with a red dot.
  3. A red-dot M4A1 and an FMJ: the M4A1 now lists "Red Dot Sight" and "FMJ". That M4A1 and a Silencer: no result (a third attachment). **Screenshot 2** (the empty result slot).
  4. Rename the other M4A1 on an anvil, then put a Silencer on it: the name stays.
  5. Two USP .45 in the 2x2: one USP .45 with "Akimbo" and "Clip 12/12 + 12/12". Take it. Two M4A1: no result. **Screenshot 3.**
  6. Put the akimbo USP .45 alone in the grid: the result is one USP .45. Take it: the other USP .45 stays in the cell. **Screenshot 4.**
  7. Hover a Red Dot Sight in the inventory: "Craft together with a gun to attach it".
  8. The Galil and a Red Dot Sight make a Galil with a red dot sight.
  9. With a gun and an attachment in the grid: press Q over the result (the gun drops), a number key over the result (the gun goes into that empty hotbar slot) and shift-click the result (it goes to the right end of the hotbar).
  10. Two windows (`scripts\mp-local.ps1`, both with the kit): the friend makes the akimbo USP .45 and splits it again. **Screenshot 5** (friend window).
- **Kit** (14 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:crafting_table,minecraft:anvil,minecraft:experience_bottle*16,iw4l:m4,iw4l:m4,iw4l:usp,iw4l:usp,iw4l:t5_galil,iw4l:red_dot_sight*4,iw4l:acog_scope*2,iw4l:fmj*2,iw4l:silencer*2,iw4l:extended_mags*2,iw4l:grip*2
```

### 2. A grindstone strips a gun's attachments

Commit: the one after `8563f752`.

- **Changed:**
  - `minecraft_terrain` `gun_mods.rs`: `strip(gun, rules)` gives the gun with only `akimbo` left (if it had it) through `refit`, so the clip and `clip_left` are cut to the bare size, and the number of attachments removed. No result for a gun with nothing to strip or for a stack that is not one gun.
  - `minecraft_terrain` `stations.rs`: `Context` gains `guns: Option<&dyn GunRules>`. The grindstone accepts fork guns in both input slots. One gun alone (in either slot) gives the stripped gun; a gun with any second item gives nothing. Taking it gives XP by vanilla's disenchanting formula (half the cost, plus a random part up to half), each removed attachment counted as cost 5 (`ATTACHMENT_COST`): 3-5 XP for one attachment, 5-9 for two. Enchantment XP is unchanged.
  - `menus.rs` fills the context's `guns` from `Books.guns`, so the host's and every friend's grindstone use the MW2 content; `render_anim` `minecraft_inventory.rs` fills it from `ForkLook::rules` for the result shown on the host's and the friend's screen.
  - `docs/WEAPONS.md` 9.2: the lost rounds and the XP numbers.
- **Tests:**
  - `minecraft_survival` `gun_crafting::tests::the_grindstone_strips_a_gun_keeps_akimbo_and_cuts_the_clip` (through `menus::apply` on an open grindstone and the output slot): an M4A1 with extended mags and 40 rounds comes out bare with 30 and 3-5 XP; an akimbo USP .45 with FMJ keeps `akimbo` and both clips; a bare gun, two guns and a gun without MW2 content give nothing.
  - `minecraft_terrain` `gun_mods::tests::stripping_keeps_akimbo_and_needs_something_to_strip`.
- **Decisions:**
  - **The rounds above the bare clip are lost** with the attachments: the grindstone has one output slot and vanilla's grindstone gives nothing back but XP.
  - **XP:** an attachment counts like a level-1 enchantment with Unbreaking I's minimum cost (5), the middle of vanilla's level-1 costs, so stripping gives a little XP, less than disenchanting a good tool.
  - **The grindstone needs MW2 content** for the bare clip size; without it a gun can be put in, but no result shows.
- **Owner's test steps** (survival, kit below; the kit gives every gun a full clip):
  1. Put an M4A1 and Extended Mags into the inventory grid: the M4A1 shows "Extended Mags" and "Clip 30/45" (the kit's 30 rounds stay 30). Put it on the hotbar, select it and press R: "Clip 45/45".
  2. Put that M4A1 into the grindstone (either slot): the result is the M4A1 without "Extended Mags", "Clip 30/30". Take it: a few XP orbs' worth of XP is added (3-5). **Screenshot 1** (the grindstone with the result).
  3. Two USP .45 make an akimbo pair; put an FMJ on it; put it in the grindstone: the result keeps "Akimbo" and both clips and loses "FMJ".
  4. A bare M4A1 in the grindstone: no result. An M4A1 with a red dot together with an iron sword: no result.
  5. Two windows (`scripts\mp-local.ps1`, both with the kit): the friend strips a red-dot M4A1 at a grindstone. **Screenshot 2** (friend window).
- **Kit** (9 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:grindstone,minecraft:iron_sword,iw4l:m4,iw4l:m4,iw4l:usp,iw4l:usp,iw4l:extended_mags*2,iw4l:fmj*2,iw4l:red_dot_sight*2
```

## Combined kit for group attachments

```
IW4L_MINECRAFT_KIT=minecraft:crafting_table,minecraft:grindstone,minecraft:anvil,minecraft:experience_bottle*16,minecraft:iron_sword,iw4l:m4,iw4l:m4,iw4l:m4,iw4l:usp,iw4l:usp,iw4l:t5_galil,iw4l:red_dot_sight*4,iw4l:acog_scope*2,iw4l:fmj*2,iw4l:silencer*2,iw4l:extended_mags*2,iw4l:grip*2
```

## C11.7 The recipe book

### 1. Minecraft recipe book in the inventory and the crafting table

Commit: the one after `10e0370b`.

- **Changed:**
  - `minecraft_terrain` `recipe_book.rs` (new, edition 2021, rustfmt-clean): the book's pure logic, written from `RecipeBookComponent`, `RecipeBookPage`, `RecipeButton`, `RecipeCollection`, `ClientRecipeBook` and `CraftingRecipeBookComponent` in the 26.3 client (javap):
    - `Tab` (Search, Equipment, Building, Misc, Redstone). A recipe's `category` picks the tab: `building` Building, `redstone` Redstone, `equipment` Equipment, everything else Misc. The Search tab lists Equipment, Building, Misc and Redstone in that order. The tab icons are vanilla's: compass; iron axe and golden sword; bricks; lava bucket and apple; redstone.
    - `known(book, unlocked)`: every unlocked vanilla crafting recipe plus every fork recipe (`fork_items::is_fork`, always known), in the book's order (vanilla's by file name, then the fork recipes in table order, so they come after vanilla's in the Equipment tab), each with its 3x3 and, if it fits, 2x2 placement (`crafting_grid_candidates`, vanilla's centring).
    - `collections`: `ClientRecipeBook.categorizeAndGroupRecipes`. A recipe without a group is its own button; a group (per tab) is one button at its first member's place.
    - `counts` and `craftable`: the player's slots 0-35 and the open grid (vanilla's `fillStackedContents` and `fillCraftSlotsStackedContents`), stacks without components only (the rule the vendored placement already uses). A recipe is craftable when every cell gets its own item (a small max-flow, so 5 oak and 3 birch planks make a chest and 5 and 2 do not).
    - `matches`: the search box, through the creative search's `name_matches` on the result's name (`ns:path` queries too).
    - `View`: the buttons of a tab (only recipes that fit the grid; then the search; then, with "Showing craftable", only buttons with a craftable recipe), tab visibility (a tab shows only when it has a recipe that fits; Search always), `pages` (20 a page) and `page_of` (a page past the end goes back to the first, like `RecipeBookPage.updateCollections`).
  - `frame` `minecraft_recipe_book.rs` (new, edition 2024, rustfmt-clean): `McRecipeBook` in `MinecraftUi.recipe_book` (one added field). The HUD owns the open flag, "Showing craftable", tab, page, search text, focus, the clicked recipe (ghost) and the open alternatives. The world side fills the visible tabs, tab icons, page count, the page's buttons (`McRecipeButton`: each fitting recipe of the button with its result and craftable flag) and the ghost cells. `McRecipeButton` has vanilla's `selected` (all, or only craftable while filtering), `alternatives` (craftable first, then the rest unless filtering), the shown result cycling every 30 ticks, and `same_result` for the stacked look.
  - `render_anim` `minecraft_recipe_book.rs` (new, edition 2024, rustfmt-clean): `RecipeBookUi` builds the page from the player's `Inventory` (its recipe book and `unlocked_recipes`) only while the inventory or crafting table is open with the book open or a ghost shown, and only when something it depends on changed (the unlocked count, tab, page, search, filter, ghost, alternatives or the item counts). `InventoryUi::publish` converts it with the same stack conversion as every slot, so icons, MW2 names and tooltips match the inventory. This runs for the host and for the friend (the friend's book is the friend's own jar plus the fork recipes).
  - `render_anim` `minecraft_gui.rs`: loads `textures/gui/recipe_book.png` (the panel), the `gui/sprites/recipe_book/` folder, `widget/text_field` and `widget/text_field_highlighted`, and the `gui.recipebook.*` texts, all from the 26.3 pack at runtime.
  - `hud` `minecraft_recipe_book.rs` (new, edition 2024, rustfmt-clean): geometry, clicks, typing and drawing.
    - The green book button (20x18, `recipe_book/button`) at (104, 61) in the inventory and (5, 34) in the crafting table (vanilla's `height/2 - 22` and `height/2 - 49`).
    - Opening shifts the container to `177 + (width - 176 - 200) / 2` and draws the 147x166 panel at `(width - 147) / 2 - 86`. Under 379 GUI pixels wide the panel sits in the middle over a hidden container, like vanilla's narrow screen (a pick then closes the book).
    - Tabs 35x27 down the left edge (`x - 30`, `y + 3 + 27 n`, only visible ones, the selected one 2 pixels further out); 20 recipe buttons 25x25 in five columns from (11, 31); the page arrows at (38, 137) and (93, 137) with "2/3" centred at (73, 141); the search box (25, 13, 81 x 14, hint "Search..." in grey italics, 50 characters, the chat key focuses it, typing blocks E, Q and the number keys); the filter toggle at (110, 12) with "Showing Craftable" or "Showing All".
    - Button looks `slot_craftable`, `slot_uncraftable` and their `slot_many_` forms (red when no recipe of the button can be made). A group whose recipes all make the same item draws a second item behind (the stacked look). A hovered button shows the result's tooltip and "Right Click for More".
    - Right-click on a button with more than one recipe opens the alternatives overlay (`OverlayRecipeComponent`: 4 columns up to 16, else 5, kept near the middle of the book with vanilla's float maths, `crafting_overlay` or `_disabled`, the ingredients drawn at 6 pixels). Any click closes it.
    - A left click on a recipe (or an alternative) shows its ghost: every ingredient over its empty grid cell with vanilla's red background and white cover, the result over the result slot (24x24 red in the crafting table, 16x16 in the inventory) with its count. The candidates cycle every 30 ticks and show their tooltip. A click on a grid or result cell clears the ghost (vanilla's `slotClicked`).
    - Clicks on the panel and the tabs never throw the carried stack.
  - `hud` `minecraft_inventory.rs`: the shifted origin, the book's clicks before the slots', the search box in `typing`, the ghost cleared by grid clicks, the book drawn over the slots and its tooltips. `minecraft_gui.rs`: `Painter::item_sized` (an item at any size; `item_tinted` calls it at 16).
- **Tests:**
  - `minecraft_terrain` `recipe_book::tests`: `only_unlocked_vanilla_recipes_and_every_fork_recipe_are_known`, `recipes_of_one_group_and_tab_share_one_collection_at_the_first_member` (and the Search tab's order), `a_recipe_is_craftable_only_when_every_cell_gets_its_own_item` (planks spread over two kinds; a named stack does not count), `the_shown_list_keeps_fitting_matching_and_craftable_collections` (2x2 against 3x3, filtering, search by name and by `iw4l:`), `twenty_recipes_fill_a_page_and_a_page_past_the_end_goes_back_to_the_first`, `a_shaped_recipe_is_centred_in_the_table_only_when_it_is_narrow`.
  - `frame` `minecraft_recipe_book::tests`: `a_button_shows_its_craftable_recipes_first_and_hides_the_rest_while_filtering` (selection, alternatives order, 30-tick cycle, same result), `turning_pages_stays_inside_the_book_and_a_new_tab_starts_at_its_first_page` (reopening keeps only the open and filter flags).
  - `render_anim` `minecraft_recipe_book::tests`: `the_book_lists_fork_recipes_always_and_vanilla_ones_once_unlocked` (no rebuild without a change, filtering, a hidden tab falls back to Search, search, alternatives cells only for the open overlay), `a_clicked_recipe_shows_its_ghost_in_the_open_grid` (2x2 and 3x3 cells, the result count). `minecraft_gui::tests::the_recipe_book_sheet_sprites_and_texts_load` (with the 26.3 pack: the sheet, every sprite size, the nine-slice overlay and text field, the page text); I made it fail once on purpose ("2/4") to see that it runs.
  - `hud` `minecraft_recipe_book::tests`: `the_book_opens_left_of_the_screen_and_shifts_it_like_vanilla` (button, panel and container positions, narrow screens), `tabs_stack_down_the_left_edge_and_recipes_fill_five_columns`, `the_alternatives_overlay_stays_near_the_middle_of_the_book`, `clicks_open_tabs_turn_pages_pick_ghosts_and_open_alternatives`, `the_book_button_toggles_the_book_and_typing_needs_the_search_box` (and which clicks clear the ghost).
- **Decisions:**
  - **Vanilla's order inside a tab is the jar's file order.** The client orders known recipes by the server's display ids in a Java `HashMap`; the fork loads no display catalog, so those ids do not exist. File order keeps related recipes together and puts every fork recipe after vanilla's.
  - **Transmute, dye and special crafting recipes are not in the book**: the vendored book reads only shaped and shapeless recipes. The fork's attach, akimbo and split recipes are not listed either, as planned.
  - **Clicking a recipe only shows its ghost** in this commit, also when the ingredients are there; placing comes with `PlaceRecipe` in the next group. The ghost is drawn only over empty cells, because nothing clears the grid first yet.
  - **The friend's book shows only the fork recipes** until the next commit sends the friend its unlocked vanilla recipes from the host.
  - **The book is mouse only.** Vanilla's keyboard and pad navigation of the book, the bounce of newly unlocked recipes and the furnace books are not built.
  - The "Showing craftable" flag and the open flag live on the screen side for now; the next commit saves them per player on the host.
- **Owner's test steps** (survival, kit below; vanilla recipes unlock from items that change in your inventory this session, so step 2 picks items up; they are saved from the next commit on):
  1. Open the inventory (E). Below the 2x2 grid, right of the shield slot, is the green book button. Click it: the book opens on the left and the inventory moves right. **Screenshot 1.**
  2. Close the inventory, drop the oak logs, iron ingots and gunpowder with Q and pick them up again. Open the inventory: the tabs on the book's left edge (compass, iron axe and golden sword, bricks, lava bucket and apple) and on the Search tab planks, the gun parts, ammo and more.
  3. Click the iron axe tab: vanilla's equipment recipes first, then the fork's: the gun parts, the pistols as one button, and so on. Buttons you can make now have the normal background, the rest are red. **Screenshot 2.**
  4. Hover a fork button: the item's MW2 name and "Right Click for More". Right-click it: the alternatives open, the craftable ones first. Click one: its ingredients show as red ghosts in the 2x2 grid. **Screenshot 3.**
  5. Put any item into the 2x2 grid: the ghosts disappear.
  6. Click the button top right of the book: "Showing Craftable", only makeable buttons stay. Click it again: "Showing All".
  7. Type `rifle` into the search box: Rifle Ammo. While typing, E and the number keys type instead of closing or swapping. Type `m4`: the M4A1 (by its MW2 name). Clear the box.
  8. With more than 20 buttons (Search tab) the page arrows and "1/2" appear under the buttons; they turn the page.
  9. Crafting table: the book button is left of the 3x3 grid. Open the book: the guns are listed now (they need the 3x3). Click the M4A1: its ghost fills the grid and the result slot. **Screenshot 4.**
  10. Two windows (`scripts\mp-local.ps1`, both with the kit): the friend opens the book and sees the fork recipes (vanilla ones come with the next commit) and their ghosts. **Screenshot 5** (friend window).
- **Kit** (12 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:crafting_table,minecraft:oak_log*16,minecraft:iron_ingot*32,minecraft:gunpowder*32,minecraft:copper_ingot*16,minecraft:iron_nugget*16,minecraft:tripwire_hook*4,minecraft:redstone*8,minecraft:repeater*2,minecraft:iron_block*2,minecraft:stick*8,minecraft:cobblestone*16
```

### 2. Vanilla recipe unlocks and the book's settings are saved in the player file

Commit: the one after `cac751f5`.

- **Changed:**
  - `minecraft_survival` `recipe_book.rs` (new, edition 2024, rustfmt-clean): `Book`, each player's recipe book on the host, in `Player.recipe_book` (per UUID, saved with the player):
    - `restore` (on join, from the file): every saved recipe id the 26.3 data knows is unlocked again, an unknown id is dropped (vanilla's "Tried to load unrecognized recipe: ... removed now."), fork ids are skipped (they are always known); `toBeDisplayed` likewise; the eight vanilla settings flags come back as they were.
    - `notice_carried`: right after the restore every stack the player carries counts as a changed item, like vanilla's first inventory sync on login (`ServerPlayer.initInventoryMenu`, every slot fires `inventory_changed`), so a new player's kit or a round-8 player file unlocks its recipes on the first join.
    - Unlocks themselves stay the vendored `auto_unlocks_for_change` path (vanilla's recipe advancements, `auto_unlock_rules`) that every inventory change already runs, per player. `update` adds every newly unlocked recipe to `toBeDisplayed` (vanilla's `addHighlight` in `ServerRecipeBook.addRecipes`); `seen` removes the recipes the book has shown (vanilla's `ServerboundRecipeBookSeenRecipePacket`).
    - `saved` / `view`: the unlocked recipes without the fork ids, sorted, `toBeDisplayed`, the settings.
    - `follow_screen` (the host's own screen: the saved open and "Showing craftable" flags go to the screen once, then the screen's flags come back to the book) and `follow_friend` (a friend's messages), so the host and every friend go through the same `Book`.
  - `minecraft_survival` `player_nbt.rs`: `SavedBook` and `PlayerData.recipe_book`, read from and written to vanilla's `recipeBook` compound: `recipes` and `toBeDisplayed` string lists (always written), and `isGuiOpen`, `isFilteringCraftable`, `isFurnaceGuiOpen`, `isFurnaceFilteringCraftable`, `isBlastingFurnaceGuiOpen`, `isBlastingFurnaceFilteringCraftable`, `isSmokerGuiOpen`, `isSmokerFilteringCraftable` as bytes written only when true (vanilla's `optionalFieldOf(..., false)`; checked with javap on `RecipeBookSettings$TypeSettings` and `ServerRecipeBook$Packed`). The furnace flags are kept as read. The compound replaces the old one when the player is saved; it was only carried over before.
  - `minecraft_survival` `player.rs` and `server.rs`: the field, `data()` saves it, `join_from` restores it before `notice_carried`.
  - Vendored `minecraftoss_player::crafting`: `RecipeBook` keeps every recipe id of the jar (`data/minecraft/recipe/*.json`, stonecutting, smithing and special recipes too) and `has_recipe(id)` answers for them, the parsed crafting and cooking recipes and the extra (fork) ones. Without it the restore would drop every stonecutting or smithing unlock a vanilla 26.3 player file has, and re-saving would lose them.
  - Friends:
    - `minecraft_net` `messages.rs` (appended): `ToClient::RecipeBook(RecipeBookView { recipes, highlight, open, filtering })`, `ToServer::RecipeBookSettings { open, filtering }` and `ToServer::RecipeSeen(ids)`. `codec.rs` tests cover the three.
    - `minecraft_survival` `session.rs`: `Session::send_recipe_book` sends a friend its book whenever the unlocked count, the highlight count or a setting changes (from `send_views`, so every host frame); `book_settings` and `seen_recipes` hold what the friend sent.
    - `render_anim` `minecraft_host.rs`: the two friend messages go into the friend's session (at most 256 seen ids a step), and every step `follow_friend` applies them to the friend's `Book` before the friend's step.
    - `render_anim` `minecraft_recipe_book.rs`: `FriendBook`: the first book from the host sets the friend's open and filter flags; afterwards the friend's own changes go to the host as `RecipeBookSettings`, and recipes the book showed that were waiting to be displayed go as `RecipeSeen`. The unlocked ids reach the book through `InventoryUi::set_unlocked` (the friend rebuilds its `Inventory` every frame, so the set is kept once instead of copied).
    - `render_anim` `minecraft_remote.rs`: the new message is taken from the effects queue (`show_effects`), so `minecraft_survival/src/remote.rs` is untouched; `show_player` sends the outgoing book messages.
  - `render_anim` `minecraft_inventory.rs`: `InventoryUi` keeps the ids the book showed (`take_seen`) and the friend's unlocked set; `minecraft_world.rs`: one `follow_screen` call for the host.
  - `hud` `minecraft_recipe_book.rs`: a test builds its `MinecraftUi` with struct update syntax (clippy's `field_reassign_with_default` on the previous commit's test).
  - `minecraft_survival/tests/vanilla_world.rs`: the player file now has `recipeBook` (compound) with `recipes` and `toBeDisplayed` lists, like a vanilla player file.
- **Tests:**
  - `minecraft_survival` `recipe_book::tests`: `a_saved_book_comes_back_without_unknown_or_fork_recipes`, `new_unlocks_wait_to_be_displayed_until_the_book_shows_them`, `the_host_screen_takes_the_saved_settings_once_then_writes_them_back` (and a friend's settings), `items_carried_on_joining_unlock_their_recipes` (with the 26.3 jar: an oak log in the backpack unlocks the planks; I made it fail once on purpose to see that it runs).
  - `minecraft_survival` `player_nbt::tests`: `the_recipe_book_is_saved_in_vanilla_s_recipe_book_compound` (round trip, true flags only, a file without our book keeps its compound, missing lists read as empty), `a_player_saves_its_unlocked_recipes_but_never_a_fork_recipe` (an unknown id in the file is dropped on load like vanilla, a fork id in the file or in the unlocked set is never written, the filter flag survives).
  - `minecraft_survival` `session::tests::a_friends_recipe_book_is_sent_once_for_each_change`.
  - `render_anim` `minecraft_recipe_book::tests::a_friend_takes_the_hosts_settings_once_and_reports_its_own_changes_and_seen_recipes`.
  - Vendored `crafting::tests::every_recipe_of_the_jar_and_every_extra_recipe_is_known` (with the 26.3 jar: a crafting, a smelting and a stonecutting id are known, made-up ids are not).
  - `minecraft_net` `codec::tests`: the round trips cover `RecipeBookSettings`, `RecipeSeen` and `RecipeBook`.
- **Decisions:**
  - **The message shapes changed without the protocol bump**, as the brief says: `PROTOCOL` goes up once in the next group's `PlaceRecipe` commit, which also covers these three messages. Until then a friend on this build and a host on an older one decode each other wrongly; nobody plays between the two commits.
  - **The recipe list goes to a friend as its own message, not in `PlayerView`**: `PlayerView` is resent on every vitals change, the list is hundreds of ids and changes rarely. It travels through the friend's effects queue, so Track D's `remote.rs` needs no edit.
  - **The bounce of newly unlocked recipes is still not drawn**, but `toBeDisplayed` behaves like vanilla: a recipe leaves it once the book has shown it on a page, so a vanilla client loading the save bounces only what this game never showed.
  - **A friend's settings from the host are taken once** (the first book after joining); after that the friend's screen is the source and the host only stores them. The host never changes them on its own, so nothing is lost.
  - Recipes unlocked on join by carried items are also highlighted, as in vanilla (they are new to the book).
  - **WEAPONS.md 13.1 and 14 corrected**: vanilla's flags are per book type (crafting, furnace, blast furnace, smoker), not per tab; the inventory and the crafting table share the crafting pair. Section 14 lists the three new messages under the one protocol bump.
- **Owner's test steps** (a new world, `IW4L_MINECRAFT_WORLD=r9-book`, kit below):
  1. Join. Open the inventory and the book: planks, sticks, the crafting table and more are already there (the kit's items unlock their recipes on joining). **Screenshot 1.**
  2. Click "Showing craftable" so it reads "Showing Craftable", leave the book open, close the inventory. Break a stone block and pick up the cobblestone: the furnace and stone tools join the book.
  3. Save and quit, start the world again, open the inventory: the book is open by itself, still "Showing Craftable", and the furnace is still there. **Screenshot 2.**
  4. Close the book with its button, save and quit, start again: the inventory opens without the book.
  5. Two windows (`scripts\mp-local.ps1`, both with the kit): the friend opens the book: the friend's vanilla recipes (planks and so on) are there now, next to the fork ones. The friend sets "Showing Craftable" and leaves; the friend joins again: the setting is kept. **Screenshot 3** (friend window).
  6. Optional, with vanilla Minecraft 26.3: open a copy of the world folder: the recipe book shows the same unlocked recipes and settings.
- **Kit** (8 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:oak_log*16,minecraft:wooden_pickaxe,minecraft:iron_ingot*16,minecraft:gunpowder*16,minecraft:copper_ingot*8,minecraft:iron_nugget*8,minecraft:redstone*8,minecraft:crafting_table
```

## Combined kit for group recipe-book

```
IW4L_MINECRAFT_KIT=minecraft:crafting_table,minecraft:oak_log*16,minecraft:wooden_pickaxe,minecraft:iron_ingot*32,minecraft:gunpowder*32,minecraft:copper_ingot*16,minecraft:iron_nugget*16,minecraft:tripwire_hook*4,minecraft:redstone*8,minecraft:repeater*2,minecraft:iron_block*2,minecraft:stick*8,minecraft:cobblestone*16
```

## C11.7 and C11.8 Placing from the book, creative

### 1. The recipe book places ingredients for the host and friends

Commit: the one after `7c7633a2`. This is the brief's C11.7 item 2.

- **Changed:**
  - `minecraft_net` `messages.rs`: `MenuAction::PlaceRecipe { recipe, all }`, appended at the end of `MenuAction`. `PROTOCOL` goes from 3 to 4: the track's one bump, which also covers the recipe-book group's `ToClient::RecipeBook`, `ToServer::RecipeBookSettings` and `ToServer::RecipeSeen`.
  - Vendored `minecraftoss_player::inventory`: `place_crafting_recipe` keeps its unlock check and hands the rest to the new `place_known_crafting_recipe` (the same body, unchanged), so the fork can place recipes that are always known without unlocking them. The placement itself is the vendored one, written from vanilla's `ServerPlaceRecipe`: the grid goes back into the inventory first (nothing happens when it does not fit), ingredients come only from slots 0-35 and only from stacks without components, a click on the recipe already in the grid adds one more set, shift places as many sets as the inventory and the stack sizes allow (at most 64), and a recipe the inventory cannot make leaves the grid empty and answers `Ghost`.
  - `minecraft_survival` `recipe_book.rs`: `place(inventory, table, recipe, all)`: a fork recipe (always known) or an unlocked vanilla recipe is placed, anything else is refused.
  - `minecraft_survival` `menus.rs` (additive): `apply` places the recipe in the open grid, the 2x2 with the inventory open and the 3x3 with a crafting table open; with any other screen open it does nothing. `valid` refuses a `PlaceRecipe` from a friend when no crafting screen is open or the id is longer than `RECIPE_ID_LIMIT` (256), and the menu is sent again. The host's own clicks (`apply`) and a friend's clicks (`click`) go through the same code, per player.
  - `frame`: `McClick::PlaceRecipe { recipe, all }` (appended); `McRecipeBook.last_placed`, `place` and `clear_ghost`.
  - `hud` `minecraft_recipe_book.rs`: a left click on a recipe button or on an alternative sends `PlaceRecipe` (shift held: `all`). `BookFrame` carries the shift key; the click works on a `Pick` and pushes into `ui.clicks`, so the request goes in order with the slot clicks. A grid click clears the ghost and the last placed recipe (`clear_ghost`), like vanilla's `slotClicked`. In a narrow window an alternative pick closes the book too, like a button pick.
  - `render_anim` `minecraft_inventory.rs` `action_of`: `McClick::PlaceRecipe` becomes `MenuAction::PlaceRecipe`, which the host applies to its own player and a friend's window sends as a `Click` on its open menu (`minecraft_remote.rs` unchanged).
- **Tests:**
  - `minecraft_survival` `recipe_book::tests` (fork recipes from `fork_recipes::recipes()` plus a planks recipe, no jar needed):
    - `placing_a_recipe_clears_the_grid_then_fills_it_and_another_click_adds_a_set`: with a crafting table open, dirt in the grid goes back to the inventory, the gun barrel's three iron ingots land in the middle row, a second click makes them 2 each, and the grid gives 2 gun barrels.
    - `shift_places_as_many_sets_as_the_inventory_holds`: 5 copper ingots give 5 sets of rifle ammo ingredients, and 256 of each give 64 a cell.
    - `a_missing_ingredient_places_nothing_and_asks_for_the_ghost`: a frag grenade with one plain gunpowder (a named stack does not count) answers `Ghost` with the grid emptied into the inventory; a vanilla recipe is refused until it is unlocked, then placed; a recipe that does not fit the 2x2 and an unknown id are refused.
    - `a_friend_places_through_its_click_and_only_in_a_crafting_screen`: through `menus::click`, refused (and resent) with a chest open and for an overlong id, placed in the friend's own 2x2 with the inventory open.
  - `frame` `minecraft_recipe_book::tests::placing_a_recipe_asks_the_host_and_shows_a_ghost_only_for_missing_ingredients`; `reopen` clears `last_placed`.
  - `hud` `minecraft_recipe_book::tests::clicks_open_tabs_turn_pages_pick_ghosts_and_open_alternatives` now checks the sent `PlaceRecipe` requests: a missing recipe once (a second click on it sends nothing), a shift pick of an alternative with `all`.
  - `render_anim` `minecraft_inventory::tests::screen_clicks_become_menu_actions_and_the_rest_stay_on_the_screen` covers `PlaceRecipe`.
  - `minecraft_net` `codec::tests::every_message_to_the_server_round_trips` carries a `Click` with `PlaceRecipe`.
- **Decisions:**
  - **The ghost is decided on the screen, not sent back.** Vanilla's server answers an unplaceable recipe with `ClientboundPlaceGhostRecipePacket`. Here every screen already knows whether the recipe can be made (the button's craftable flag, counted from the same slots 0-35 and grid, components-free stacks only), and the host answers `Ghost` exactly when that flag is false. So the screen shows the ghost itself when it asks for a recipe it cannot make, and the host empties the grid; no extra message, for the host and for friends alike.
  - **Vanilla's repeat rule kept** (`RecipeBookComponent.tryPlaceRecipe`, checked with javap): a click on the same recipe that still cannot be made does nothing; a recipe that can be made is asked for again, which adds a set.
  - **A full inventory blocks the place,** as vanilla's survival `testClearGrid`; vanilla's creative mode would drop the grid items instead, which the fork does not do.
  - **A friend's `PlaceRecipe` with no crafting screen open is refused** and the menu resent, like an invalid slot.
- **Owner's test steps** (survival, a new world, kit below; "row 2" is the middle row of the crafting table):
  1. Open the inventory and the recipe book (green button). Iron axe tab, find Rifle Ammo and click it: a copper ingot, an iron nugget and gunpowder jump into the 2x2 grid and the result shows 16 Rifle Ammo. **Screenshot 1.**
  2. Click Rifle Ammo again: each cell holds 2. Shift-click it: each cell holds 8 (all the copper).
  3. Take the grid items back out, put 4 dirt into the grid, click Rifle Ammo: the dirt goes back into the inventory and the ingredients come in.
  4. Place the crafting table and open it, open the book: click Gun Barrel: three iron ingots in row 2. Take the 2 barrels. Click Firing Mechanism, take it.
  5. Click the M4A1 (no iron block in the kit): the grid empties, red ghost items show every ingredient (the iron block too, since the grid holds nothing) and a red M4A1 over the result. Click it again: nothing changes. Put any item into the grid: the ghost goes. **Screenshot 2.**
  6. Click Oak Planks (unlocked by the logs): a log lands in the grid.
  7. Two windows (`scripts\mp-local.ps1`, both on this build, both with the kit): the friend places Rifle Ammo in their 2x2 and shift-clicks Gun Barrel in a crafting table; only the friend's grid fills, and the friend takes the results. **Screenshot 3** (friend window).
  8. A friend on an older build is refused when joining with the outdated-build message (protocol 4).
- **Kit** (10 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:crafting_table,minecraft:oak_log*8,minecraft:iron_ingot*16,minecraft:copper_ingot*8,minecraft:iron_nugget*16,minecraft:gunpowder*16,minecraft:tripwire_hook*2,minecraft:redstone*4,minecraft:repeater*2,minecraft:dirt*8
```

### 2. Guns, ammo, grenades and attachments are at the end of the creative Combat tab

Commit: the one after `095da2fb`. This is the brief's C11.8.

- **Changed:**
  - `minecraft_terrain` `fork_items.rs`: `creative_entries()` (one `TabEntry` per fork item in the table's creative order, `in_tab` and `in_search`), `add_to_creative(tabs)` (appends them to `minecraft:combat`, after vanilla's entries; false when there is no Combat tab), `loaded_gun()` (the full-clip components, `custom_data.iw4l.clip = -1`), `CREATIVE_TAB`.
  - `minecraft_terrain` `creative_tabs.rs`: `Variants::FullClip` (appended). `creative_variants.rs` `expand`: `FullClip` gives one variant, `loaded_gun()`. Every gun with ammo gets this variant, so the creative item is the bare gun with a full clip (the same components `/give` writes), while the riot shield, parts, attachments, ammo and grenades stay plain, so their stacks keep merging.
  - `render_anim` `minecraft_creative.rs`: only the loader thread, right after `creative_tabs(&jar)`: `add_to_creative`, with a log line if the jar has no Combat tab. Track A's `apply` and `hotbar_key` are untouched.
  - Search needs no change: the Search tab lists every category entry that is `in_search`, and its lines are the item names from the language map, where the fork names are (the table's English names from the start, MW2's localized names once the weapon registry is in).
- **Tests:**
  - `minecraft_terrain` `fork_items::tests::the_combat_tab_ends_with_every_fork_item_in_table_order_and_guns_come_loaded` (on the fallback tabs: a vanilla entry stays first, then all 94 fork ids in table order; guns with ammo carry `FullClip` and the riot shield does not; the expanded components equal `with_full_clip` for every such gun; the Search tab holds all 94; no Combat tab, nothing added).
  - `minecraft_terrain` `fork_items::tests::the_real_combat_tab_keeps_vanilla_first_and_ends_with_the_fork_items` (with the 26.3 jar: vanilla's Combat entries stay first and are all `minecraft:`, the fork items follow in table order, no other tab gets one). I made it fail once on purpose to see that it runs.
  - `render_anim` `minecraft_fork_items::tests::the_creative_combat_tab_and_search_list_the_fork_items_by_their_mw2_names` (through the creative screen's own `entry_stacks` and `search_lines`: the Combat tab shows the fork items in table order, the M4A1 loaded and stacking to 1, the riot shield without a clip, rockets plain and stacking to 16; the search finds "M4A1" and "m4a1", a name from the language map ("Intervention (MW2)" standing in for MW2's string) and "frag").
- **Decisions:**
  - **"Fully loaded" is a creative variant**, the way vanilla's creative tabs carry potions or fireworks with components. `TabEntry` has no components field, and the brief keeps the edit in `minecraft_creative.rs` to the loader. If `VariantData` cannot be read from the jar (already logged as "creative variants unavailable"), the guns show bare with no clip, like the other variant items fall back to their plain stack.
  - **The order is the fork table's order**, which a test already ties to WEAPONS.md 13.2's (guns by class, pistols to launchers, then parts, attachments, ammo, lethal, tactical). Inside the launchers the table keeps the IW4 launchers (AT4, RPG, Stinger, Javelin) before the T5 and IW5 ones, as the design tables list them.
  - **The search list is built when the Search tab is first opened**, after the match has loaded the MW2 names; a search list built earlier keeps the table's English names (the same as MW2's for most guns) until the creative screen loads again.
  - The friend's creative screen comes with track A (A6); it uses the same loader, so the fork items appear there too after that merge.
- **Owner's test steps** (any world, `/gamemode creative`):
  1. Open the inventory (E) and the Combat tab. Scroll to the end: after vanilla's combat items come the pistols, machine pistols, SMGs, assault rifles, shotguns, the riot shield, crossbow and ballistic knife, the snipers, LMGs and launchers, then Gun Barrel and Firing Mechanism, the attachments, the seven ammo types, the lethal grenades and the tactical grenades (Tactical Insertion last). **Screenshot 1.**
  2. Hover the M4A1 in the tab: "Clip 30/30". Take it to the hotbar, close the inventory and press its number: it comes up with a full magazine. **Screenshot 2** (the tooltip in the tab).
  3. Hover the Riot Shield: no clip line. Shift-click Rockets into the inventory: a stack of 16.
  4. Search tab: type `M4A1`: the M4A1. Type `intervention`: the Intervention. Type `frag`: the Frag Grenade. Hovering a result names the Combat tab. **Screenshot 3** (the search for `M4A1`).
- **Kit:** none needed (creative).

## Combined kit for group book-place-creative

```
IW4L_MINECRAFT_KIT=minecraft:crafting_table,minecraft:oak_log*8,minecraft:iron_ingot*16,minecraft:copper_ingot*8,minecraft:iron_nugget*16,minecraft:gunpowder*16,minecraft:tripwire_hook*2,minecraft:redstone*4,minecraft:repeater*2,minecraft:dirt*8
```

## Review of the whole track

Fixes from the adversarial review of `24dfdf71..820dc0ab`, one commit each.

### 1. The fork item look file passes rustfmt

Commit: the one after `820dc0ab`.

- **Defect:** `crates/render_anim/src/minecraft_fork_items.rs` is a new file of this track, but its committed content did not pass `rustfmt --check --edition 2024` (an import out of order and five over-long lines). The project rule says every new file must pass rustfmt.
- **Changed:** rustfmt (edition 2024) run on that one file. Formatting only, no code change.
- **Tests:** none needed; `git show :crates/render_anim/src/minecraft_fork_items.rs | rustfmt --check --edition 2024` prints nothing, and `check -p render_anim` passes.
- **Owner's steps:** none (no behaviour change).

### 2. Fork item code drops unwrap_or_default and an indexed hotbar loop, as clippy asks

Commit: the one after `9a085715`.

- **Defect:** clippy on the touched crates (`24dfdf71..9a085715`) warned on six lines this track added: the workspace's `clippy.toml` disallows `Option::unwrap_or_default` (it hides which fallback a site takes), used in `minecraft_commands` `tag.rs` (`/tag list`), `minecraft_survival` `server.rs` (the saved `Tags`), `gun_names.rs` and `minecraft_terrain` `gun_mods.rs` and `fork_items.rs` (a gun's attachment list), and `guns.rs` indexed `guns[slot]` in a `0..HOTBAR` loop (`needless_range_loop`).
- **Changed:** the five fallbacks read the `Option` with `into_iter().flatten()` (no tags or no attachment list gives an empty list, as before); the hotbar loop walks `guns.iter_mut().enumerate()`. No behaviour change. The older `unwrap_or_default` in `item_icons.rs` `language` (from before this track, on a line it only reshaped) stays.
- **Tests:** none new; the existing `/tag`, player file, attachment and hotbar binding tests cover these lines and pass (`test -p minecraft_commands -p minecraft_survival -p minecraft_terrain`). The three new files still pass `rustfmt --check`.
- **Owner's steps:** none (no behaviour change).

## Combined kit for the whole track

27 stacks. It covers the inventory, saving, hotbar, drop, item frame, crafting, attachment, grindstone and recipe book steps; the first-join loadout (no kit) and the creative tab (creative) need none. For the gun recipes' full list of ingredients use the group recipes kit above.

```
IW4L_MINECRAFT_KIT=minecraft:crafting_table,minecraft:grindstone,minecraft:anvil,minecraft:chest,minecraft:item_frame*4,minecraft:oak_log*16,minecraft:iron_ingot*32,minecraft:copper_ingot*16,minecraft:iron_nugget*16,minecraft:gunpowder*32,minecraft:tripwire_hook*4,minecraft:redstone*8,minecraft:repeater*2,minecraft:experience_bottle*16,iw4l:m4,iw4l:m4,iw4l:usp,iw4l:usp,iw4l:barrett,iw4l:riotshield,iw4l:rifle_ammo*64,iw4l:pistol_ammo*32,iw4l:frag_grenade*8,iw4l:rocket*16,iw4l:red_dot_sight*2,iw4l:extended_mags*2,iw4l:fmj*2
```

## C11.9 Ammo

### 1. Reloading a gun in a Minecraft world takes ammo items from the inventory

Commit: the one after `c27f9bc9` (the merge of tracks A and B into track C).

- **Changed:**
  - `weapon_iw4`: `WeaponHandState` has a new `loaded` counter. `reload::reload_clip`, the one place a reload moves rounds from the stock into the clip (normal, per-shell and bolt reloads alike), adds the rounds it moved. Nothing reads it on MW2 maps.
  - `sim/src/combat.rs` `advance_weapon_command`: after `weapon_hands`, the rounds loaded by both hands (akimbo) are queued as `Spent::Rounds` for the held weapon through `voxel::spend_rounds`, only on the host (`publishes_snapshot`) and only in Minecraft worlds (`voxel::active()`).
  - `sim/src/voxel.rs`:
    - `AMMO_TYPES` is 8: the 7 ammo items, then the throwing knife for the Ballistic Knife.
    - `GunSpec.alt_ammo`: the ammo type of a gun's alt mode.
    - `spend_rounds(client, weapon, rounds, alternate)`: finds the published gun (or the gun whose alt mode it is) and queues `Rounds` with its `ammo` or `alt_ammo`. It queues nothing for a client that has not published, in creative, or for a gun with no ammo type.
    - `queued_spent()`, and `script_set_clips(client)`, which marks a publishing client's `Published.scripted`.
  - `sim/src/script_player.rs`: `give_weapon` (when it seeds a new weapon's ammo) and `set_ammo_clip` (also behind `giveStartAmmo` and an offhand's `giveMaxAmmo`) call `script_set_clips`.
  - `sim/src/item.rs` `apply_arsenal`:
    - Each published gun's MW2 stock is `min(stock[ammo] - rounds still queued for this client and type, full stock)`, and its alt mode's the same with `alt_ammo`. Full stock is what MW2 refilled to before (`max(maxAmmo, startAmmo)`); the stock setter does not clamp, so the clamp is here. Creative and a gun with no ammo type (the riot shield) get the full stock.
    - Creative no longer forces full clips: clips empty, reload and are written back to the item as in survival.
    - After a script marked the client, the held gun's live clips are capped at the last clips the sim knew (other guns get their item clips anyway). So GSC `giveLoadout` at a spawn, and One Man Army, cannot grow a clip.
    - The transitional stock top-up of group arsenal is gone. Grenades keep today's refill until group grenade-throws.
  - `minecraft_survival/src/guns.rs` `settle_arsenal`:
    - `Spent::Rounds` removes that many ammo items in vanilla's projectile order: offhand, main hand, then slots 0-35. In creative it removes nothing. If fewer items are left than rounds queued, it removes what is there.
    - `stock_index` (the 7 ammo items, then `iw4l:throwing_knife`) counts the stock over slots 0-35 and the offhand, so the Ballistic Knife reloads from throwing knives anywhere but the lethal slot.
    - `Gun.alt_ammo` from the gun's attachments: explosive rounds for the grenade launcher, shotgun shells for the underbarrel shotgun.
  - `minecraft_terrain/src/fork_items.rs`: `alt_ammo(attachments)`; `has_alt_mode` uses it.
  - `render_anim/src/minecraft_fork_items.rs`: passes `alt_ammo` to the sim. The host and each friend go through the same `ForkLook::settle`, so each friend's stock comes from the friend's own inventory and their reloads take the friend's items.
  - `docs/WEAPONS.md` 5.1 and 8.2: `GunSpec.alt_ammo`, 8 stock types, the full-stock cap, creative clips, the reload site and the script clip cap.
- **Tests:**
  - `weapon_iw4` `reload::tests::a_finished_reload_counts_only_the_rounds_it_moved`: 12/30 with 100 in stock loads and counts 18; a short stock loads 5; no stock loads nothing; a shotgun counts one shell per insert.
  - `sim` `item::arsenal_tests`:
    - `the_stock_is_the_counted_ammo_less_the_rounds_not_yet_taken`: 50 rifle rounds with 18 queued publish 32; another client's or another type's queue is not subtracted; the grenade launcher counts explosive rounds; 500 rounds publish the full stock; creative publishes the full stock with no items.
    - `no_ammo_items_leave_the_gun_to_click_empty`: no items and an empty clip give stock 0, and MW2 does not allow the reload.
    - `a_spawn_refill_creates_no_items`: `takeAllWeapons`, `giveWeapon` and `giveMaxAmmo` refill the held M4 to 30; the next step puts back its 5 rounds and the counted stock, with nothing queued; rounds fired before a script call stay spent.
    - `script_clip_changes_mark_only_a_publishing_client`, `a_reload_queues_rounds_of_the_gun_s_ammo_type` (rifle, the alt mode's explosive rounds, nothing for a gun without an ammo type or not published, nothing in creative).
    - `a_held_gun_queues_its_clip` now checks that creative keeps the live clip (4) and reports it, with the full stock.
  - `minecraft_survival` `guns::tests`: `a_reload_takes_exactly_the_missing_rounds_offhand_first` (offhand, main hand, then slot order; a partial reload of 3; more rounds than items empties the type only), `creative_takes_no_rounds_and_has_endless_stock`, `the_ballistic_knife_reloads_from_throwing_knives`, `each_player_s_arsenal_counts_their_own_inventory` (the friend's rounds come out of the friend's inventory only); `a_clip_spent_is_written_to_the_bound_item` checks `alt_ammo`.
  - `minecraft_terrain` `fork_items::tests::guns_have_ammo_and_only_netherite_guns_resist_fire` checks `alt_ammo`.
  - `test -p weapon_iw4 -p sim -p minecraft_survival -p minecraft_terrain` green (sim 135, the MW2-map tests included); `check -p launcher` green.
- **Decisions:**
  - **The reload site is a counter on the hand, not the `ReloadAmmoAdded` event.** `finish_weapon_tick` returns one event per hand, and a rechamber or a state change in the same tick hides the shells event, so a count on the event would lose rounds.
  - **Akimbo:** the sim's MW2 code gives both hands the smaller of their two stocks after a tick, so if both hands load in the same tick it takes fewer rounds from MW2's stock than it loads (an existing difference to IW4, unchanged for MW2 maps). The queue counts the rounds both hands loaded, so the items taken always match the rounds in the clips.
  - **The stock cap** is the full stock MW2 refilled to before (`max(maxAmmo, startAmmo)`), not maxAmmo alone, so a def with a small maxAmmo (an alt mode) still reloads as it did. WEAPONS.md updated.
  - **The throwing knife is stock type 8** (index 7) in `minecraft_survival` and the sim; `fork_items::AMMO_TYPES` stays the 7 ammo items, so recipes and the creative tab are unchanged.
  - **Creative clips:** WEAPONS.md 5.1 said creative refills clips, 8.2 and this group's brief say the clip still empties and reloads. The brief wins; 5.1 updated.
  - **Script clip cap (not in the brief):** without it, the spawn refill would have grown the held gun's live clip, which the sim keeps and writes back to the item. Capping at the last known clips (instead of writing them) keeps rounds fired in the same step spent.
  - Rounds still queued when a friend leaves are dropped with the friend's arsenal; at most one frame's reload.
  - The riot shield, the only gun without an ammo type, keeps the full stock (it has no clip).
- **Owner's test steps** (survival, a new world or one without the kit items, kit below; the kit fills the backpack):
  1. Open the inventory. Move the M4A1 to hotbar slot 1 and the USP .45 to slot 2. Put the stack of 10 rifle ammo into the offhand slot. Close it and press 1. MW2's ammo counter shows 30 and, to the right, the rifle rounds you carry (74, or MW2's maximum for the M4A1 if smaller).
  2. Fire 12 rounds and press R. When the reload finishes, the offhand stack is gone and the backpack's 64 rifle ammo became 62; the counter shows 30 and 62. **Screenshot 1** (the inventory open).
  3. Press 2 and fire the USP until it reloads by itself: the pistol ammo drops by the rounds loaded; the rifle ammo stays at 62.
  4. Place the chest and put all rifle ammo into it. Press 1, fire the M4A1 empty: it clicks, the counter shows 0 and 0, R does nothing. Take 5 rifle ammo back: the right number shows 5 at once; R loads 5 and the 5 items are gone. **Screenshot 2.**
  5. Take the rifle ammo back. `/gamerule keep_inventory true`. Fire the M4A1 down to about 10 rounds, then `/kill` and respawn. Press 1: the clip still has those 10 rounds (hover the M4A1: "Clip 10/30"), and the rifle ammo count is what it was before `/kill`. **Screenshot 3.** `/gamerule keep_inventory false`.
  6. `/give @s iw4l:m4[minecraft:custom_data={iw4l:{attachments:["gl"]}}]`, put it on slot 3 and press 3. Switch to the grenade launcher with MW2's attachment key and fire it, then reload it: the 8 explosive rounds become 7.
  7. Put the Ballistic Knife on slot 4 and press 4. Fire it and reload: the 8 throwing knives in the backpack become 7.
  8. `/gamemode creative`. Fire the M4A1 empty and reload: the clip empties and reloads, the counter's right number is full, and no ammo item is used. `/gamemode survival`.
  9. Two windows (`scripts\mp-local.ps1`), the friend with the same kit: the friend repeats steps 1 and 2 in their window; the friend's rifle ammo drops, the host's does not. **Screenshot 4** (both windows).
- **Kit** (9 stacks):

```
IW4L_MINECRAFT_KIT=iw4l:m4,iw4l:usp,iw4l:rifle_ammo*64,iw4l:rifle_ammo*10,iw4l:pistol_ammo*64,iw4l:explosive_round*8,iw4l:t5_knife_ballistic,iw4l:throwing_knife*8,minecraft:chest
```

### 2. A gun's clip shows as the item bar under its icon

Commit: the one after `268d78de`.

- **Changed:**
  - `frame` `McStack.clip: Option<(u32, u32)>`: the rounds loaded and the clip capacity (added at the end of the struct; every literal gets `clip: None`).
  - `minecraft_survival` `gun_names::clip_bar(stack, size)`: for a fork gun with an ammo type and a known clip size, the rounds in `clip` (and `clip_left` for akimbo) and the capacity (the size, twice for akimbo). `-1` counts as full, a missing clip as empty. The riot shield, other items and a gun whose MW2 weapon is unknown get no bar.
  - `render_anim`: `ForkLook::clip_bar` takes the size from the session's `GunNames`, and `InventoryUi::publish` fills `McStack.clip`. The host and the friend's window both build their slots there, each from its own MW2 content, so both screens show the bar.
  - `hud` `minecraft_gui`: `decorations` draws the bar like vanilla's item bar (a black 13x2 strip at (2, 13) under the icon, then the coloured 1-pixel row), so it shows in the inventory, every menu, the creative inventory, the recipe book and the hotbar. `clip_bar(loaded, capacity)` gives its width and colour.
- **Tests:** `hud` `minecraft_gui::tests::a_gun_s_clip_bar_is_green_when_full_and_red_when_empty` (full 13 px green, empty 1 px red, half 7 px, akimbo 17 of 24, more than the capacity full, a zero capacity safe); `minecraft_survival` `gun_names::tests::the_clip_bar_counts_the_loaded_rounds_of_both_akimbo_clips` (a full `-1` clip, no clip, an over-full clip clamped, unknown size, akimbo, riot shield, ammo). `test -p hud -p frame -p minecraft_survival -p render_anim` and `check -p launcher` green.
- **Decisions:**
  - **Width like vanilla's bundle bar, colour like the durability bar.** Vanilla shows a durability bar only on damaged items, and its width reaches 0 when broken, so an empty clip would show nothing. The bundle bar (`BundleItem.getBarWidth`: `min(1 + mulAndTruncate(fullness, 12), 13)`, checked with javap) is vanilla's bar for "how full" and keeps one pixel at empty, so the red shows. The colour is the durability hue by the fraction loaded: green full, yellow half, red empty.
  - **The bar is always shown on a gun,** full ones included, as the design asks ("green when full").
  - **Akimbo** shows both clips as one bar over twice the clip size.
- **Owner's test steps** (survival, kit below):
  1. Put the M4A1 on hotbar slot 1 and press 1. Under its icon in the hotbar a full green bar shows. **Screenshot 1** (hotbar).
  2. Put the rifle ammo into the chest, so the M4A1 cannot reload. Fire about half the clip: the bar is half as wide and yellow. Fire until the clip is empty: one red pixel. Open the inventory: the same bar under the M4A1 there. **Screenshot 2** (inventory open, empty M4A1).
  3. Take the ammo back and reload: the bar is full and green again.
  4. Put the two USP .45 into the crafting grid to make an akimbo pair, put it on slot 2, press 2 and fire 12 rounds: the bar is about half (12 of 24 rounds left).
  5. Two windows: the friend does steps 1 and 2 in their window; the friend's bar follows the friend's gun, and the host's bars are unchanged. **Screenshot 3** (friend window).
- **Kit:**

```
IW4L_MINECRAFT_KIT=iw4l:m4,iw4l:usp,iw4l:usp,iw4l:rifle_ammo*64,iw4l:pistol_ammo*64,minecraft:chest
```

## Combined kit for group ammo

```
IW4L_MINECRAFT_KIT=iw4l:m4,iw4l:usp,iw4l:usp,iw4l:rifle_ammo*64,iw4l:rifle_ammo*10,iw4l:pistol_ammo*64,iw4l:explosive_round*8,iw4l:t5_knife_ballistic,iw4l:throwing_knife*8,minecraft:chest
```

## C12.1 The grenade slots

### 1. Lethal and tactical grenade slots in the player inventory, saved with the player, for the host and friends

Commit: the one after `2dffcdda`.

- **Changed:**
  - Vendored `minecraftoss_player::inventory`: the inventory has 45 slots (`Default`, `demo_hotbar`, `add_item`'s `seen`); 43 is the lethal slot and 44 the tactical slot. `add_item` asks the recipe book for the stack's reserved slot and, right after the main-hand and offhand merge, tops up that slot when it holds the same item or fills it when it is empty; only then the backpack.
  - Vendored `minecraftoss_player::crafting`: `RecipeBook::with_reserved_slots(fn(&str) -> Option<usize>)` and `reserved_slot(id)`; a book without one (every `RecipeBook::default()`) reserves nothing, so vanilla behaviour and the vendored tests are unchanged.
  - Vendored `minecraftoss_player::chest` (test only): the drag test's chest cells start at 45 instead of 43, because `distribute` puts external cells after the player slots.
  - `minecraft_terrain` `fork_items`: `LETHAL_SLOT` (43), `TACTICAL_SLOT` (44) and `grenade_slot(id)` (lethal rows to 43, tactical rows to 44, the tactical insertion included). `crafting::load_recipes` hands it to the book with `with_reserved_slots`, so the host's and every friend's inventory route grenades. `Grid::Inventory.player_slots()` is 45, so a drag in the inventory screen reaches the grenade slots (the table still only reaches 0-35).
  - `minecraft_survival` `guns`: `fits(slot, stack)`: slot 43 takes only lethal grenades, 44 only tactical ones, at most 64; body 41 and saddle 42 take nothing; every other slot anything. `grenade_quick_move(inventory, index)`: a shift-click on a grenade in 0-35 moves as much as fits into its slot when that slot is empty or holds the same grenade with room.
  - `minecraft_survival` `menus`: `PLAYER_SLOTS` is 45, so `valid_slot` accepts a friend's click on 43 and 44. `apply` refuses a click that would put the cursor into a slot where it does not fit and a number-key or F swap that would put either stack where it does not fit (`misfits`, next to `keeps_gun`), and drops the slots that do not fit from a drag before any spread path runs (`fitted_spread`; vanilla's quick-craft leaves out slots whose `mayPlace` fails). In the player's own inventory screen a shift-click goes through `grenade_quick_move` first; out of 43 or 44 the vendored `quick_move` already goes to 9-35, then 0-8. In a chest or the crafting table a shift-click stays vanilla (to the chest, or between backpack and hotbar), as vanilla's `ChestMenu` and `CraftingMenu` never route to equipment slots.
  - `minecraft_survival` `creative`: `MENU_SLOTS` is 48; menu index 46 is slot 43 and 47 is slot 44 (`menu_place`, `menu_index`); `set_slot` refuses a stack that does not `fit` with `BadSlot`, so a creative friend cannot set dirt into a grenade slot either. The trash's shift-click empties both.
  - `minecraft_survival` `player_nbt`: `SLOTS` is 45. Slots 43 and 44 are saved under the fork's top-level key `iw4l_grenades`, a `slot_list` in the disk form (`Slot:0b` lethal, `Slot:1b` tactical, each grenade as the uneatable `minecraft:poisonous_potato` carrier), written only when one of them holds something. The key is in `OWNED`, so emptied slots leave the file. It never goes into `equipment`.
  - `minecraft_survival` `death`: `EQUIPMENT_DROP_ORDER` ends with 43 and 44, after the saddle; `keep_inventory` keeps them as before.
  - `minecraft_commands` `clear`: `PLAYER_SLOTS` is 45, so `/clear` covers the grenade slots (and body and saddle, see the decisions).
  - `minecraft_net` `PLAYER_SLOTS` and `frame` `MC_INVENTORY_SLOTS` are 45: `views::player_view` sends a friend all 45 slots, and the host's and friend's `McUi.slots` hold them for the next commit's drawing.
  - `docs/WEAPONS.md` 10.2: where the pickup predicate lives, body and saddle, the creative menu indices and `/clear`.
- **Tests:**
  - Vendored `inventory::tests`: `the_inventory_has_two_grenade_slots_after_the_saddle`, `a_pickup_fills_an_empty_grenade_slot_of_its_kind` (a frag pickup goes into the empty lethal slot although 10 frags sit in the backpack; a flash into the tactical slot), `a_pickup_tops_up_a_matching_grenade_slot_after_the_hands` (main hand first, then the lethal slot to 64, then the backpack; a semtex does not merge into a slot holding frags), `other_items_never_land_in_a_grenade_slot` (dirt with a full backpack stays out; a book without the predicate puts a frag in the backpack). `chest::tests::dragging_across_chest_and_player_slots_conserves_stack` uses the new cell indices.
  - `minecraft_terrain`: `fork_items::tests::lethal_and_tactical_grenades_have_their_own_slot`, `crafting::tests::the_loaded_book_sends_picked_up_grenades_to_their_slots` (a frag and a smoke in the grid go to 43 and 44 when the screen closes, a log to the hotbar).
  - `minecraft_survival` `menus::tests`: `only_a_lethal_grenade_goes_in_the_lethal_slot` (dirt, flash, an M4A1 and a killstreak mirror refused; semtex, throwing knife, claymore and C4 accepted; 65 frags do not fit), `only_a_tactical_goes_in_the_tactical_slot` (dirt, frag, rifle ammo refused; flash, stun, smoke and tactical insertion accepted; body and saddle refuse a saddle), `shift_click_moves_a_grenade_to_its_slot_and_back_out` (40 + 24 semtex into the lethal slot, the rest of a full slot to the hotbar, a frag with semtex in the slot to the hotbar, out of 43 to slot 9, out of 44 to the hotbar when 9-35 are full, in a chest to the chest), `a_number_key_cannot_swap_dirt_into_a_grenade_slot` (dirt refused both ways, semtex swaps with frag, an empty hotbar slot takes the grenade, F swaps a smoke from the offhand into 44), `a_drag_spreads_only_into_grenade_slots_that_fit` (dirt only to slot 20; 10 frags over 43, 44 and 20 give 5 and 5), `a_friends_click_on_slot_44_is_valid` (the friend's flash lands in 44 with no resend; slot 45 is refused and resent).
  - `minecraft_survival` `creative::tests::creative_puts_only_grenades_of_the_right_kind_in_the_grenade_slots`; `menu_slots_map_onto_the_inventory_like_the_inventory_menu` covers 46 and 47; `the_trash_deletes_the_carried_stack_and_shift_empties_the_inventory` empties 43 and 44.
  - `minecraft_survival` `player_nbt::tests::grenade_slots_are_saved_under_their_own_key_and_read_back` (two poisonous potato carriers with `Slot` 0b and 1b, no `equipment`, an empty `Inventory`, both read back into 43 and 44; a tactical alone is `Slot:1b`), `emptied_grenade_slots_leave_the_file`.
  - `minecraft_survival` `death::tests::grenade_slots_drop_after_the_saddle_and_keep_inventory_keeps_them`.
  - `minecraft_survival` `views::tests::the_view_carries_items_and_guns_and_they_come_back` sends 45 slots and gets 43 and 44 back.
  - `minecraft_survival` `tests/vanilla_world.rs` (26.3 data): the saved player has semtex and flashes in 43 and 44; `iw4l_grenades` is a list of two slot items (`Slot` byte, `count` int, `id` string, `components` compound) with `Slot` 0b and 1b; `equipment` still has only chest, feet, head, legs and offhand; the slots come back after loading.
  - `minecraft_commands` `clear::tests::clear_empties_the_grenade_slots`; the count tests now include the saddle slot (112 instead of 103 stone, 123 instead of 114 items).
  - `minecraft_net`: `messages::tests::a_new_player_view_has_every_slot` checks 45; `codec::tests::a_player_view_carries_both_grenade_slots`, and the round-trip `player_view` fills 43 and 44.
  - `test -p minecraftoss-player` (with the stand-in scenario files) `-p minecraft_terrain -p minecraft_survival -p minecraft_commands -p minecraft_net -p frame` green; `check -p launcher` green.
- **Decisions:**
  - **Not vanilla:** two extra player slots, 43 lethal and 44 tactical, saved under the top-level `iw4l_grenades` (vanilla 26.3 ignores the key and drops it if it re-saves the player). For `decisions.md`.
  - **The pickup predicate lives on the recipe book,** not on `Inventory` and not as a global: every player inventory on the host and the friend's mirror already get the book from `load_recipes`, so the rule reaches all of them without touching the places that build inventories, and `RecipeBook::default()` keeps the vendored crate's own behaviour. When the 26.3 jar is missing there is no book and no routing; grenades then go to the backpack, but `fits` still guards the slots.
  - **An empty grenade slot is filled before the backpack is topped up,** as WEAPONS.md 10.2 orders it ("otherwise it goes to the backpack"), so a pickup fills the slot even when the backpack holds the same grenade.
  - **Body 41 and saddle 42 are refused by `fits`.** Raising `PLAYER_SLOTS` to 45 also makes them valid click targets and sends them to friends; nothing may be placed there, as no screen shows them.
  - **`/clear` now also clears body and saddle.** `PLAYER_SLOTS` was 41 in `clear.rs`, which skipped them; vanilla's `Inventory.clearOrCountMatchingItems` walks the whole 43-slot container (`getContainerSize` = items + `EQUIPMENT_SLOT_MAPPING`, which maps 41 to BODY and 42 to SADDLE, checked with javap). The old test asserted the saddle stayed; it now matches vanilla.
  - **Creative menu indices 46 and 47** for the two slots (vanilla's inventory menu ends at 45); the creative tab will draw them in C12.2.
  - **Shift-click routing only in the player's own inventory screen,** like vanilla's armour routing in `InventoryMenu.quickMoveStack`.
  - **No protocol bump:** `PlayerView.slots` is a list, so 45 slots ride on protocol 5 (see Protocol).
  - Transitional until C12.2: the slots are not drawn yet, and G and Shift+G still throw the class grenades with unlimited ammo.
- **Owner's test steps** (survival, a new world `IW4L_MINECRAFT_WORLD=r9-grenade-slots`, kit below; the two slots are drawn only from the next commit, so this commit is checked with `/clear ... 0`, which counts without removing):
  1. Join and open the inventory: the kit's frags, semtex, flashes and smoke grenades are in the backpack. **Screenshot 1.**
  2. Shift-click the frag stack: it vanishes from the backpack. `/clear @s iw4l:frag_grenade 0`: "Found 16 matching item(s)": the frags are in the hidden lethal slot. Shift-click the semtex stack: it stays in the inventory (it moves to the hotbar), because the lethal slot holds frags.
  3. Shift-click the flash stack: it vanishes too (tactical slot). `/clear @s iw4l:flash_grenade 0` finds 8.
  4. `/clear @s iw4l:flash_grenade` empties the tactical slot. Move the smoke grenades to the hotbar, hold them, press Q once and walk over the dropped one: it vanishes from view (it filled the empty tactical slot), and `/clear @s iw4l:smoke_grenade 0` still finds 4. Drop one semtex with Q and walk over it: it goes back onto its hotbar stack, because the lethal slot holds frags.
  5. Save and quit, load the world: `/clear @s iw4l:frag_grenade 0` still finds 16 and `/clear @s iw4l:smoke_grenade 0` the same as before. **Screenshot 2** (the chat lines).
  6. `/gamerule keep_inventory false`, stand on flat ground and `/kill`: the frags and the smoke grenade drop with the rest; pick them up again after respawning and check with `/clear ... 0` that the frags are back in the lethal slot (16 counted, none visible in the backpack). **Screenshot 3.**
  7. Two windows (`scripts\mp-local.ps1`), the friend with the same kit: the friend repeats steps 2 and 3 in their window and gets the same counts; the host's counts are unchanged. **Screenshot 4** (friend window).
- **Kit** (5 stacks):

```
IW4L_MINECRAFT_KIT=iw4l:frag_grenade*16,iw4l:semtex*16,iw4l:flash_grenade*8,iw4l:smoke_grenade*4,minecraft:cobblestone*64
```

## Combined kit for group grenade-slots

```
IW4L_MINECRAFT_KIT=iw4l:frag_grenade*16,iw4l:semtex*16,iw4l:flash_grenade*8,iw4l:smoke_grenade*4,minecraft:cobblestone*64
```

## C12.2 Throwing from the slots

### 1. The grenade slots decide what G and Shift+G throw, and each throw uses one up

Commit: the one after `3b6a9ed3`.

- **Changed:**
  - `sim/src/step.rs`: `offhand_slot(class)` (1, 4, 5 lethal; 2, 3 tactical), `only_offhand_of_its_slot` (removes the other offhands of the same slot, adds the weapon, sets `offhand_primary` or `offhand_secondary`) and `set_loadout_offhand` (`loadout.lethal` or `tactical`), factored out of `apply_give_offhand`, which now calls them and behaves as before.
  - `sim/src/world.rs`: public `offhand_class(weapon)` on the sim state (0 when the weapon is no offhand).
  - `sim/src/voxel.rs`: `Published.offhands` (the last lethal and tactical weapon per client) and `spend_grenade(client, weapon, delta)`. A `-1` is queued only for a publishing client in survival and only for its published (or last) lethal or tactical weapon, so an airdrop marker or a bot queues nothing; a `+1` for any weapon of a publishing client in survival.
  - `sim/src/combat.rs`: the two identical spend blocks of `OffhandUsed` and `OffhandCookedOff` are one function, `use_offhand_round(world, id, weapon, minecraft)`, which also queues `Grenade { delta: -1 }` when `minecraft` (the host, `publishes_snapshot`, in a Minecraft world, `voxel::active()`). A frag cooked until it blows in the hand spends one like a throw.
  - `sim/src/item.rs`:
    - `apply_arsenal` (`follow_grenades`): the published lethal and tactical (weapon, count) make that weapon the only offhand of its slot (killstreak and AC-130 weapons are kept), with `loadout.lethal`/`tactical` following, so the client runtime and the friend's replicated loadout send the right `off_hand_index` unchanged. The ledger and `ps.ammoclip` hold the count plus the `Grenade` deltas still queued for that weapon. A weapon whose class does not fit the slot is ignored.
    - An empty slot keeps the last weapon of that slot with 0, so the MW2 icon stays and a placed C4 can still be detonated; with no last weapon, the class offhands of that slot get 0.
    - The transitional grenade refill is gone for publishing clients: offhands of classes 1-5 are no longer refilled. Killstreak weapons still are, and clients that never published (bots, MW2 maps) keep the full refill.
    - `grab_projectile` (walking over a thrown knife, the only pickup site of thrown projectiles) queues `+1`. For a publishing client a thrown knife or a Ballistic Knife blade is picked up even when MW2's clip has no room, because the item count is the truth.
  - `minecraft_survival/src/guns.rs`:
    - `Arsenal.lethal` and `tactical` from slots 43 and 44 through `GunNames` (`weapon_of` already covers lethal and tactical rows), with the stack count.
    - `spend_grenades(inventory, id, delta, selected)`: a `-1` takes one from the grenade's slot, or, if the slot no longer holds that grenade, from slots 0-35 and then the offhand. When a throw empties the slot it refills from the first stack of the same grenade in slots 0-35 (the whole stack), never another type. A `+1` tops up the slot (up to 64) or fills it when empty, otherwise it goes through `add_item` (the pickup rules) and what does not fit is returned.
    - `settle_arsenal` applies `Grenade` deltas first (survival and adventure only; creative changes nothing). The weapon is turned into its item with `GunNames::item`: a lethal or tactical row is its own item, the Ballistic Knife gives a throwing knife. `Settled.dropped` holds what did not fit.
  - `render_anim`: `minecraft_fork_items::published` passes `lethal` and `tactical` to the sim; the host (`minecraft_world.rs`) and each friend (`minecraft_players.rs` `step_friend`) drop `Settled.dropped` at their feet. Both go through the same `ForkLook::settle`, so each friend's throws take the friend's own grenades.
  - `docs/WEAPONS.md` 5.1 and 10.4: where the count lives, the creative line, the throw and pickup sites, the knife pickup without clip room and the overflow drop.
- **Tests:**
  - `sim` `item::arsenal_tests` (the test content now has semtex, throwing knife, claymore, C4, flash, smoke and an airdrop marker with their offhand classes):
    - `a_published_grenade_count_limits_throws`: frag 2; each round applies the arsenal with the queued deltas and throws when the ledger has one; two throws, the third finds nothing, two `Grenade { delta: -1 }` queued.
    - `pending_throws_are_subtracted_until_taken`: frag 5 with two of its own throws queued (another client's and a rounds entry ignored) gives 3 on the ledger and on the player state; the flash likewise; once taken, the slot count alone; 40 frags stay 40 although `maxAmmo` is 1.
    - `the_lethal_item_replaces_the_class_lethal`: semtex replaces frag and smoke replaces flash (`offhand_primary` 1, `offhand_secondary` 2, `loadout.lethal`/`tactical` follow; the airdrop marker stays); throwing knife gives class 4, claymore class 5; a flash published as the lethal is refused and the claymore stays with 0.
    - `an_empty_slot_keeps_the_c4_detonator`: C4 1, then an empty slot: C4 stays with 0, is not refilled, `offhand_primary` stays 5, and a frag added by a spawn is removed again.
    - `a_client_that_never_published_keeps_unlimited_grenades`: five throws by a client that never published refill to full and queue nothing.
    - `creative_throws_and_knife_pickups_queue_nothing`: creative queues nothing; in survival a knife throw and pickup are queued and an airdrop marker is not.
  - `minecraft_survival` `guns::tests`: `a_throw_takes_one_from_its_slot` (frag 3 and flash 2 become 2 and 1 and are published; a frag thrown after the slot changed to semtex comes out of the backpack), `an_emptied_slot_refills_with_the_same_grenade_only` (the 5 frags of slot 20 move into the slot, the semtex never does), `creative_keeps_the_stack`, `a_picked_up_knife_comes_back_and_overflows_like_a_pickup` (63 knives become 64, a Ballistic Knife blade becomes a throwing knife in the backpack, with a full inventory it is dropped).
  - `test -p sim -p minecraft_survival` green (sim 141, the MW2-map tests included); `check -p launcher` green.
- **Decisions:**
  - **The ps stock setter does not clamp** (`set_ammo_not_in_clip` and `set_clip_for_hand` write the value as given; only GSC's `setWeaponAmmoClip`/`setWeaponAmmoStock` clamp). The count goes into the clip, as `apply_give_offhand` seeds offhands (`spawn_clip_count`, more than the clip size for a frag); the HUD and the weapon tick read clip plus stock, so 32 or 64 show and throw correctly.
  - **The throw site is one shared function** (`combat::use_offhand_round`) for throws and cook-offs, with the Minecraft gate passed in, so the unit test runs it without a block world.
  - **Knife pickup:** the site is `item::grab_projectile`. A publishing client picks a thrown knife up even with a full MW2 clip (otherwise a player with knives left could never pick one up), and the `+1` goes through the slot and pickup rules on the host; a Ballistic Knife blade gives back a throwing knife, as that gun reloads from throwing knives. What does not fit is dropped at the feet, since MW2 already removed the knife.
  - **A `-1` for a grenade that left its slot** (the stack was moved while the throw was on its way) is taken from slots 0-35, then the offhand; if none is left, nothing is taken.
  - **Refill takes the first stack of the same grenade in slots 0-35** (hotbar first, like vanilla's projectile search order), the whole stack, and only after a throw emptied the slot.
  - **A published weapon in the wrong slot** (class not lethal for 43 or not tactical for 44) is ignored and the last weapon stays with 0; `fits` already keeps such items out of the slots.
  - Transitional until the next commits: the two slots are not drawn yet, and new players do not get their 32 + 32 yet.
- **Owner's test steps** (survival, a new world `IW4L_MINECRAFT_WORLD=r9-grenade-throws`, a class with Frag and Flash, kit below; the slots are drawn from the next commit, so this commit is checked with MW2's grenade icons at the bottom right and `/clear ... 0`, which counts without removing):
  1. Join and open the inventory. Shift-click the stack of 2 frags and the flash stack: both vanish into the hidden slots. Close it: MW2's grenade icons show 2 frags and 8 flashes. **Screenshot 1.**
  2. Press G: a frag is thrown, 1. Press Shift+G: a flash, 7. Press G again: the slot empties and refills from the backpack's 16 frags, the icon shows 16. Hold G until the frag blows up in your hand: 15. `/clear @s iw4l:frag_grenade 0` finds 15 (no frag is left in the backpack). **Screenshot 2.**
  3. Open the inventory, shift-click the semtex stack: it goes to the hotbar, because the lethal slot holds frags. `/clear @s iw4l:frag_grenade`, then shift-click the semtex stack: it vanishes into the lethal slot, and G throws semtex (its icon shows).
  4. `/clear @s iw4l:semtex`, shift-click the C4: G places C4 four times; with the slot empty the C4 icon stays with 0, and G detonates the placed C4. **Screenshot 3.**
  5. Shift-click the throwing knives (8) into the empty lethal slot: throw one at a wall (7), walk over it: 8 again.
  6. `/clear @s iw4l:flash_grenade`, shift-click the stun grenades: Shift+G throws a stun grenade. `/clear @s iw4l:concussion_grenade`, shift-click the smoke grenades: Shift+G throws smoke.
  7. `/gamemode creative`: G throws and the count stays. `/gamemode survival`.
  8. Two windows (`scripts\mp-local.ps1`), the friend with the same kit: the friend repeats steps 1 and 2 in their window with their own kit; the host sees the friend's grenades, and the host's counts are unchanged. **Screenshot 4** (friend window).
- **Kit** (11 stacks):

```
IW4L_MINECRAFT_KIT=iw4l:frag_grenade*2,iw4l:frag_grenade*16,iw4l:semtex*16,iw4l:c4*4,iw4l:throwing_knife*8,iw4l:flash_grenade*8,iw4l:concussion_grenade*8,iw4l:smoke_grenade*8,minecraft:cobblestone*64,minecraft:torch*16,minecraft:cooked_beef*16
```

### 2. The hotbar and the inventory show the lethal and tactical grenade slots

Commit: the one after `edcd9cc5`.

- **Changed:**
  - `hud` `minecraft_hotbar`: `grenade_frames(cx, h)`: the lethal frame at x = cx-91-29-29 and the tactical frame at cx-91-29-58, both at y = h-23, drawn with vanilla's `hud/hotbar_offhand_left` sprite whenever the hotbar shows, empty or not, so they never move. Each slot's item sits at the frame's x+3, h-19 with its count and decorations, like the offhand. MW2's grenade icons are unchanged.
  - `hud` `minecraft_screens`: `LETHAL`, `TACTICAL`, `GRENADE_SLOTS`: the inventory screen has `McSlot::Inventory(43)` at (134, 62) and `(44)` at (152, 62), after the offhand in the layout; `draw_grenade_frames` draws `container/slot` behind them with the background. `grenade_slot_tooltip`: hovering an empty grenade slot with nothing carried shows "Lethal" or "Tactical", after every other tooltip.
  - `hud` `minecraft_creative`: the creative inventory tab has the two slots at (127, 20) and (145, 20), with their frames; the item tabs do not. Clicks on them are ordinary inventory clicks, which the host already checks with `fits`, and the creative menu indices 46 and 47 of the grenade-slot group.
  - `hud` `minecraft_inventory`: the empty-slot tooltip is the last fallback in the tooltip chain.
  - `render_anim` `minecraft_gui`: `container/slot` is loaded as a loose GUI sprite from the pack at runtime.
  - The friend's window draws the same: its hotbar and screens use the 45 slots of its `PlayerView`.
  - `docs/WEAPONS.md` 10.3: the creative tab positions and the free pixels, checked.
- **Tests:** `hud` `minecraft_creative::tests::the_grenade_slots_sit_in_free_panel_space_and_left_of_the_offhand` (creative tab (127, 20) and (145, 20), none in an item tab; inventory screen (134, 62) and (152, 62) and a mouse hit, none on the crafting table; hotbar frames at cx-149 and cx-178, y h-23, left of the offhand frame; the tooltips "Lethal" and "Tactical" only on empty grenade slots). `the_inventory_tab_places_armor_offhand_and_trash_like_vanilla` and `the_inventory_screen_follows_the_vanilla_menu_order` count the two new slots. `test -p hud -p render_anim` and `check -p launcher` green.
- **Decisions:**
  - **Free pixels checked** in the 26.3 jar's `inventory.png` and `tab_inventory.png` (decoded, not committed): both positions from WEAPONS.md are plain panel, so they stay.
  - **The tooltip names are English literals** ("Lethal", "Tactical"), like the other fork texts (MW2's class menu calls the rows Lethal and Tactical); vanilla has no key for them.
  - **The empty-slot tooltip shows only with nothing carried,** like item tooltips.
  - The frames are drawn with the background, before the hovered-slot highlight, so the highlight shows on them like on vanilla slots.
- **Owner's test steps** (survival, the world of commit 1 or a new one, kit below):
  1. Join with an empty offhand: left of the hotbar there are two empty offhand-style frames, with a gap where the offhand frame shows when it holds something. **Screenshot 1** (hotbar).
  2. Open the inventory: two slot frames right of the recipe book button, level with the offhand slot. Hover them: "Lethal" and "Tactical". **Screenshot 2** (inventory open, tooltip).
  3. Click the frags into the left one and the flashes into the right one. Cobblestone is refused in both. Close the inventory: the frames show the frag and the flash with 16 and 8; G and Shift+G lower the counts in the frames and on MW2's icons alike. **Screenshot 3.**
  4. Put the shield into the offhand: the offhand frame appears to the right of the grenade frames, which stay where they are.
  5. `/gamemode creative`, open the inventory tab of the creative screen: the two slots sit right of the leggings and boots slots and hold the same grenades; take the flashes out and put them back. **Screenshot 4.** `/gamemode survival`.
  6. Two windows (`scripts\mp-local.ps1`), the friend with the same kit: the friend's hotbar shows its own two frames, and the friend's inventory has the two slots; step 3 in the friend's window changes only the friend's counts. **Screenshot 5** (friend window).
- **Kit** (4 stacks):

```
IW4L_MINECRAFT_KIT=iw4l:frag_grenade*16,iw4l:flash_grenade*8,minecraft:cobblestone*64,minecraft:shield
```

### 3. New players start with 32 lethal and 32 tactical grenades

Commit: the one after `b47be4bb`.

- **Changed:**
  - `minecraft_survival` `loadout`: `give_first_loadout` (the C11.4 first-join loadout, once per player through the `iw4l_loadout_given` tag) also gives `CLASS_GRENADES` (32) of the class lethal into slot 43 and 32 of the class tactical into slot 44. The class weapon (`LoadoutSpec.lethal`/`tactical`) becomes its item through `GunNames::item`; with no class grenade, or one that is not a lethal (or tactical) item, it is frags and flashes. An empty slot gets the 32; a slot holding the same grenade is topped up to 64; the rest, or all 32 when the slot holds another grenade, goes to the backpack like the ammo, and what does not fit is dropped at the feet.
  - Nothing else changed: the host and every friend already call `ForkLook::first_loadout` with their own class (`class_of`), and the tag is saved with the player file. The sim's `follow_grenades` only rewrites `loadout.lethal`/`tactical` once a slot holds a grenade, so the loadout still reads the class grenades on the first frame.
- **Tests:** `minecraft_survival` `loadout::tests::a_new_player_gets_32_of_each_class_grenade_once` (a class with semtex and stun grenades: 32 semtex in 43 and 32 stun in 44, nothing dropped, a second call gives nothing; a class with no grenades gives frags and flashes, and 40 frags already in 43 are topped up to 64 with 8 in the backpack). The test sits next to the other loadout tests in `loadout.rs`, not in `guns.rs` as the brief listed it, since the loadout function lives there. `test -p minecraft_survival` and `check -p launcher` green.
- **Decisions:**
  - **A slot that already holds a grenade is respected:** the same grenade is topped up, another one is left alone and the 32 go to the backpack. This covers a round 8 player file whose player picked up grenades before this round.
  - **The fallbacks also cover a class grenade the fork has no item for** (none today: every lethal and tactical MW2 offers has a row).
  - The tactical insertion counts as a tactical grenade here, so a class with it starts with 32 of them, like WEAPONS.md 6 says for any class tactical.
- **Owner's test steps** (survival, a new world `IW4L_MINECRAFT_WORLD=r9-grenade-start`, no kit, a class with Semtex and Stun):
  1. Join: the lethal frame shows 32 semtex and the tactical frame 32 stun grenades, next to the class guns and their ammo. **Screenshot 1** (hotbar).
  2. Throw one of each (31 and 31), save and quit, load the world: still 31 and 31, no new 32. **Screenshot 2.**
  3. `/kill` and respawn: the grenades dropped with the rest, and respawning gives none.
  4. `/tag @s remove iw4l_loadout_given`: the guns, ammo and 32 + 32 grenades are given again (picked-up grenades top up their slot or fill the backpack).
  5. Two windows (`scripts\mp-local.ps1`), the friend with a class with Frag and Flash in a new friend player file: the friend's first join gives 32 frags and 32 flashes in the friend's frames; the host's counts are unchanged. **Screenshot 3** (friend window).
- **Kit:** none (the loadout comes from the class).

## Combined kit for group grenade-throws

```
IW4L_MINECRAFT_KIT=iw4l:frag_grenade*2,iw4l:frag_grenade*16,iw4l:semtex*16,iw4l:c4*4,iw4l:throwing_knife*8,iw4l:flash_grenade*8,iw4l:concussion_grenade*8,iw4l:smoke_grenade*8,minecraft:cobblestone*64,minecraft:shield,minecraft:torch*16,minecraft:cooked_beef*16
```

## C12.3 Tactical insertion and the class menu

### 1. A planted tactical insertion is a one-use respawn point

Commit: the one after `3cab4adb`. **Not vanilla** (WEAPONS.md 10.6).

- **Plant site (found):** MW2 plants the flare in GSC: `monitorTIUse` in `maps/mp/perks/_perkfunctions.gsc` (`common_mp.ff`, read from the owner's install, not committed) waits for `grenade_fire` with `flare_mp`, spawns the glowstick at the ground below `self.TISpawnPosition` (the thrower's last on-ground origin) and sets `self.setSpawnPoint`; `spawnPlayer` in `_playerlogic.gsc` then spawns at `setSpawnPoint.playerSpawnPos` and deletes it. That script runs only for the equipment perk `specialty_tacticalinsertion` (`setTacticalInsertion`), but our class menu offers `flare_mp` in the tactical row (`ui/src/classes/setup.rs`), which `_class.gsc` does not treat as a perk, so the GSC flare never exists in this game. The plant site is therefore the throw itself: `sim::combat::plant_flare`, called for `OffhandUsed` right after `use_offhand_round`, under the same Minecraft gate (`publishes_snapshot() && voxel::active()`).
- **Changed:**
  - `sim/src/voxel.rs`: `Spent::FlareGone`; `FLARES` (the last planted flare per MW2 client id, cleared in `deactivate` and `forget_arsenal`); `plant_flare(client, pos)` (only for a client that publishes an arsenal, so bots and MW2 maps queue nothing; a second plant replaces the first) and `blast_flares(damage_at)` (a flare taking 20 or more is removed and `FlareGone` queued); `FLARE_WEAPON` = `flare_mp`.
  - `sim/src/combat.rs`: `plant_flare(world, id, weapon)`: for `flare_mp` only, at the thrower's origin traced down to the ground (up to 2048 units, MW2's ground search; a trace that finds nothing keeps the origin).
  - `sim/src/damage.rs`: both voxel blast sites (`apply_explosion_blast`, `apply_script_blast`) call `push_voxel_blasts`, which runs `push_mob_blasts` as before and then `blast_flares` with `flare_blast_damage` (MW2's `radius_damage_amount` falloff at the flare's spot, the claymore cone respected, no visibility scale).
  - `minecraft_survival/src/guns.rs`: `Spent::Flare { at: Place }` and `Spent::FlareGone`; `Binding` (per UUID in `Player.arsenal`, never saved) keeps `flare: Option<Place>` with `flare()`, `take_flare()` and `set_flare()`; `settle_arsenal` applies them in queue order and keeps the flare when it rebuilds the binding. `Binding` is no longer `Eq` (a `Place` holds floats).
  - `minecraft_survival/src/respawn.rs`: `respawn_at_flare(globals, level, player)`, a separate check at the top of `respawn` (the bed, anchor and `level_spawn` logic below it is unchanged): it takes the flare; in the same dimension it checks the spot with `flare_stands` and sets `arrival`; in another dimension it switches there with the flare as the arrival; either way `respawning` is cleared. `flare_stands(flare, open)`: the blocks at feet + 0.5 and feet + 1.5 must be open (`level::open_for_respawn`: no collision box, no fluid; an unloaded chunk counts as open), so standing on a slab is fine.
  - `minecraft_survival/src/bands.rs`: `holds(dimension, map_z)`, the same band test as `sim::voxel::band_of`.
  - `render_anim/src/minecraft_fork_items.rs`: `flare_place(dimension, origin, yaw)` turns the MW2 position into a `Place` with `sim::voxel::to_block` and the level's origin when the position lies in that dimension's band (otherwise it is dropped with a warning); `ForkLook::settle` takes it and `spent()` converts `Flare` and `FlareGone`. The host (`minecraft_world.rs`) and each friend (`minecraft_players.rs`) pass their own dimension, level origin and MW2 yaw, through the same `settle_arsenal`.
  - `docs/WEAPONS.md` 5.1 (`FlareGone`, where the flare lives) and 10.6 (the GSC finding, the plant site, the destroy rule, the spot check), and the "not verified" line.
- **Tests:**
  - `sim` `item::arsenal_tests::a_thrown_tactical_insertion_plants_a_flare_that_a_second_one_replaces_and_a_blast_destroys` (the test content has `flare_mp`, class 2): a frag plants nothing, a client that never published plants nothing, two plants queue two `Flare`s, a blast at the replaced first spot does nothing, 19 damage does nothing, 20 queues one `FlareGone` and a second blast nothing more.
  - `sim` `damage::tests::a_planted_flare_takes_the_blast_damage_at_its_spot`: 130 at the centre, the falloff at 128 units, 0 out of reach and behind a claymore.
  - `minecraft_survival` `guns::tests::a_planted_flare_is_kept_per_player_until_replaced_or_destroyed`: plant, kept, replaced by a second plant, cleared by `FlareGone`, a plant after a blast stands, `take_flare` uses it up, and another player's binding is untouched.
  - `minecraft_survival` `respawn::tests::a_flare_stands_where_the_body_fits` (feet and head blocks, the floor, a slab, an unloaded spot) and `a_flare_wins_over_the_respawn_point_once_for_its_own_player` (needs the 26.3 data, gated like its neighbours): the friend's flare wins over the friend's forced point once and is used up, the next respawn is at the point, the host is not moved by the friend's flare, and a flare in the Nether switches the host there with the flare as the arrival.
  - `minecraft_survival` `bands::tests::each_map_height_belongs_to_one_dimension`.
  - `test -p sim -p minecraft_survival -p render_anim` green (sim 143 with the MW2-map tests, survival 690 plus `vanilla_world`, render_anim 173), `check -p launcher` green.
- **Decisions:**
  - **The plant site is the throw** (above), since MW2's GSC flare never runs here. The position is MW2's `playerSpawnPos` (the thrower's feet), dropped to the ground below when thrown in the air. The respawn faces the yaw the player had when the throw was settled, pitch 0, like MW2's `spawnAngles` (the glowstick takes the thrower's angles).
  - **The flare lives in `guns::Binding`** (the per-UUID, never-saved host state the arsenal already uses) instead of a new `Player` field, so `settle_arsenal` stores it without another argument.
  - **Destroyed by MW2 blasts only:** MW2's glowstick dies from 20 damage, the owner's included. Bullets cannot hit it (no glowstick model is spawned) and Minecraft explosions (creepers, TNT, beds, anchors) never reach the sim's blast path, so they leave it. The thrown flare's own landing does not destroy it (a tactical deals no damage).
  - **Spot check at the respawn** like MW2's `tiValidationCheck` (`canSpawn`): a solid block or a fluid at the feet or head drops the flare and the bed, anchor or world spawn is used. A flare in another dimension is not checked (its level is not loaded yet).
  - **Creative plants too** (the flare is MW2's, not an item rule); the throw itself uses no item there, as before.
  - The sim keeps its copy of the flare after the host used it; a later blast there queues a `FlareGone` that finds no flare on the host, which is harmless, and the next plant replaces it.
  - No visible marker stays: the thrown `flare_mp` projectile is the only flare drawn, as before. A glowstick model is left for later.
- **Owner's test steps** (survival, a new world `IW4L_MINECRAFT_WORLD=r9-insertion`, kit below; F3 shows the coordinates):
  1. Place the bed and right-click it ("Respawn point set" by day). Open the inventory and shift-click the tactical insertions into the tactical slot (the right one). Walk 30 blocks away, face north, press Shift+G: the flare is thrown and the slot shows 3. Note the coordinates. **Screenshot 1.**
  2. Walk 20 blocks on, `/kill`: you respawn where you stood when you threw it, facing north. **Screenshot 2.**
  3. `/kill` again: you respawn at the bed.
  4. Plant one at spot A, walk 20 blocks to spot B, plant another, walk away, `/kill`: you respawn at B.
  5. Plant one, step back 6 blocks, throw a frag (G) onto the spot and wait for the blast, `/kill`: you respawn at the bed.
  6. Plant one, put two cobblestone blocks on the spot (one above the other), `/kill`: you respawn at the bed.
  7. Plant one, build a Nether portal (obsidian, flint and steel) and go to the Nether, `/kill` there: you respawn at the flare in the Overworld. **Screenshot 3.**
  8. Plant one, save and quit, load the world, `/kill`: you respawn at the bed (the flare is not saved).
  9. Two windows (`scripts\mp-local.ps1`), the friend with the same kit: the friend plants a flare and walks away; the host `/kill`s: the host respawns at the host's bed; the friend `/kill`s: the friend respawns at the friend's flare. **Screenshot 4** (friend window).
- **Kit** (6 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:white_bed,iw4l:tactical_insertion*4,iw4l:frag_grenade*16,minecraft:cobblestone*64,minecraft:obsidian*10,minecraft:flint_and_steel
```

### 2. On Minecraft maps the class menu changes perks and killstreaks only

Commit: the one after `d9247254`. **Not vanilla** (WEAPONS.md 5.5).

- **Changed:**
  - `ui/src/classes/setup.rs`: `MINECRAFT_ITEM_ROWS` ("Guns and grenades are items in Minecraft worlds"); `ClassEditRow::is_item_row` (Primary, Secondary, Lethal, Tactical) and `minecraft_refusal(minecraft)` (the note for an item row on a Minecraft map, else nothing); `ClassSlotState::with_items_of(kept)` (takes the guns, their attachments and the grenades of another class); `ClassLoadoutCatalog::fit_attachments(slot)`, the attachment trim the class picker already did after a pick, moved here unchanged so the reset can use it too.
  - `console/src/class_menu.rs`: `minecraft = sim::voxel::active()`. `ui_class_edit` and `ui_class_attachments` on an item row and `ui_class_pick` for one return the note as the error (shown in `ui_class_status`), so no picker opens; `ui_class_reset` keeps the item rows (`with_items_of`, then `fit_attachments`); the status line shows the note whenever no error and no "Class unavailable" reason is shown, and drops it again once the world is left. The perk and deathstreak rows work as before.
  - `docs/WEAPONS.md` 5.5: the gate, where the note shows, the reset, and why One Man Army and Bling change nothing.
- **One Man Army and Bling (checked, no code needed):** `apply_arsenal` already overrides both. One Man Army's `onemanarmy_mp` bag takes the secondary's place, a primary-slot weapon, which `item::is_gun_slot` removes like any gun that is not on the hotbar; a class change through it runs GSC `giveLoadout`, whose guns, stock and grenades `apply_arsenal` overwrites at the next step (WEAPONS.md 5.1). Bling only lets the class guns carry two attachments; item attachments never read perks (`gun_crafting` allows two whenever MW2 has the combination).
- **Tests:** `ui` `classes::setup::tests::on_minecraft_maps_only_the_perk_rows_are_edited` (MW2 maps refuse no row; Minecraft maps refuse exactly the four item rows with the note) and `a_reset_on_a_minecraft_map_keeps_the_guns_and_grenades` (the item rows and attachments come from the old class, the perks, deathstreak and name from the default). `test -p ui -p console` green, `check -p launcher` green.
- **Decisions:**
  - **The gate is `sim::voxel::active()`**, which the host and every friend's game set while a Minecraft world is loaded; the main menu and MW2 maps are unchanged.
  - **Where the note shows:** the class editor draws only the row labels, images, the perk descriptions and one status line, so the note is the status line (whenever no error or lock reason is shown) and the answer to a click on a weapon or grenade row. The row labels keep the class guns and grenades, since they still make the first-join loadout and its 32 + 32 grenades.
  - **Reset on a Minecraft map** restores the perks and the deathstreak only.
  - **Dropping Bling still trims the class guns' second attachment** (the picker always did), because MW2's class check (`sim` `max_attachments`) refuses a class with two attachments without Bling. It changes no item, only the first-join guns of a player who has not joined yet.
  - Killstreaks are not in this editor; nothing changes for them.
- **Owner's test steps** (a Minecraft world, any class; a second world on an MW2 map or the main menu for step 5):
  1. In the Minecraft world, open the class menu and edit a custom class: the status line at the bottom reads "Guns and grenades are items in Minecraft worlds". **Screenshot 1.**
  2. Click Primary, Secondary, Lethal and Tactical: no list opens and the status line shows the note.
  3. Click Perk 1 and pick One Man Army, close the menu, `/kill` and respawn with that class: the hotbar guns and the grenade slots are unchanged and MW2's weapon list shows only the hotbar guns, no One Man Army bag. **Screenshot 2.**
  4. Pick Bling as Perk 1, then Reset the class: the perks go back to the default, the weapon and grenade rows stay as they were.
  5. Leave to the main menu (or load an MW2 map) and edit the same class: the weapon and grenade rows open their lists again and the note is gone. **Screenshot 3.**
- **Kit** (2 stacks):

```
IW4L_MINECRAFT_KIT=iw4l:m4,iw4l:rifle_ammo*64
```

## Combined kit for group insertion-classmenu

```
IW4L_MINECRAFT_KIT=minecraft:white_bed,iw4l:tactical_insertion*4,iw4l:frag_grenade*16,minecraft:cobblestone*64,minecraft:obsidian*10,minecraft:flint_and_steel,iw4l:m4,iw4l:rifle_ammo*64
```

## C13 Guns against players' armour

### 1. Guns do half damage to players in Minecraft worlds and worn armour takes twice vanilla's share of it

Commit: the one after `43b4e0df`. **Not vanilla** (WEAPONS.md 11).

- **Changed:**
  - `sim/src/gun_damage.rs` (new, edition 2024, rustfmt-clean): `GunHit { Bullet, Explosion, Melee }`; `gun_hit(means)` with WEAPONS.md 11.1's means lists (Minecraft's `MOD_TRIGGER_HURT`, falls, suicide, crush and unknown are none); `gun_damage_to_player(amount, max_health, guard, pierce, kind)`, the WEAPONS.md 11.2 formula with the named constants `GUN_PLAYER_SCALE` 0.5, `GUN_ARMOR_FACTOR` 2.0 and `GUN_PROTECTION_CAP` 0.8 (vanilla's `CombatRules.getDamageAfterAbsorb` on the halved hit, Protection EPF at 4 % a point capped at 20, both under the 80 % ceiling, then Resistance at 20 % a level); `pierce(lmg, sniper, fmj)` and the pierce constants for the next commit; `verdict(minecraft, means, amount, pierce, victim)`, which says unchanged, refused or a new amount.
  - `sim/src/voxel.rs`: `GunGuard { armor, toughness, epf_bullet, epf_blast, epf_melee, resistance, totem_grace }`, `set_gun_guard`, `forget_gun_guard` and the crate's `gun_guard`, cleared in `deactivate`.
  - `sim/src/script/host/players.rs` `player_damage`: before GSC `Callback_PlayerDamage`, when `voxel::active()` and the hit is not a Minecraft hit, `gun_verdict` reads the victim's `ps.max_health` (100 when absent) and guard. A new amount goes to GSC and `current_hit` as a copy of the hit with `amount` and `commit.amount` changed; a refused hit (0, Resistance V) returns before GSC.
  - `minecraft_survival/src/survival.rs`: `GunArmor` (the same fields) and `Survival::gun_armor(inventory)`, which reads `survival::armor(inventory)`, the worn enchantments' EPF through the existing `Protection::against` with `minecraft:arrow` (bullets, `#is_projectile`), `minecraft:player_explosion` (blasts, `#is_explosion`) and `minecraft:player_attack` (the knife), and the Resistance level.
  - `render_anim/src/minecraft_fork_items.rs`: `mirror_gun_guard(client, survival, inventory)`, called next to `set_death_protected` for the host (`minecraft_world.rs`) and in the shared friend step (`minecraft_players.rs`); `minecraft_host.rs` forgets a leaving friend's guard.
  - `docs/WEAPONS.md` 11.1 (what was built), 11.3 (the table recomputed from the owner's data) and the "not verified" line.
- **The owner's damage values (checked):** a throwaway probe (not committed) walked the owner's `common_mp.ff` with the fork's own `fastfile_iw4` zone loader and printed each weapon's damage and hit-location table. M4A1 30/20, USP .45 40/25, Desert Eagle 50/30, RPD, L86 and AUG HBAR 40, MG4 and M240 30 and the knife 135 match the memory values. Wrong in WEAPONS.md: the Intervention and Barrett are 70 with x1.5 on the upper torso (105, not 98); the frag is 150 at the centre (the 130 was a sim test's stand-in); the RPG-7's blast is 160 (its direct impact is 1000). WEAPONS.md 11.3 now uses them and adds semtex or C4 (200) and the knife.
- **Tests:**
  - `sim` `gun_damage::tests::gun_hits_are_bullets_explosions_or_melee`; `guns_do_half_damage_to_an_unarmoured_player` (30 -> 15, 98 -> 49, 130 -> 65, 1 -> 1 for every kind, the same with 400 max health); `armour_takes_twice_vanilla_s_share_up_to_four_fifths` (the WEAPONS.md 11.3 rows without pierce and without an x.5 cell: M4A1, USP, Desert Eagle, frag, RPG-7, semtex; the ceiling with Protection on netherite; a 1000 blast through diamond); `protection_and_resistance_count_under_the_same_ceiling` (Protection IV on four pieces 15 -> 5 and a frag 75 -> 27, Blast Protection IV a frag -> 15 and no help against bullets, Resistance III, Resistance V refuses); `mw2_maps_pass_the_amount_through` (inactive world, no guard, Minecraft and fall means, a 0 hit); `mob_damage_is_unchanged` (only `players.rs` applies the rule; `damage.rs`, `voxel.rs`, `script_player.rs` and `combat.rs`, where `MobShot` and `MobBlast` are made, do not).
  - `minecraft_survival` `survival::tests::guns_meet_the_worn_armour_its_protection_and_resistance` (needs the 26.3 data, gated like its neighbours): a diamond set with Protection IV, Projectile Protection IV and Blast Protection IV reads 20 armour, 8 toughness, EPF 12 for bullets, 12 for blasts and 4 for the knife, and Resistance II as 2.
  - No existing sim test runs GSC damage with a block world active: only `damage/height_bands.rs` activates one, and it calls `finish_damage` directly. The `damage.rs` tests that run `apply_damage_attempt` publish no guard, so they keep MW2's amount even if another test has a block world active. `test -p sim` (149) and `test -p minecraft_survival` green, `check -p launcher` green.
- **Decisions:**
  - **Only players with a published guard are changed.** Every Minecraft player, host and friend, publishes one each frame; a client that never does (a bot, any MW2 map) keeps MW2's amount, like the track's rule for clients without an arsenal.
  - **A hit taken to 0 is refused before GSC** (no health change, no hit marker), since GSC would raise it to 1. Only Resistance V does this.
  - **Rounding** is Rust's `round` (half away from zero) in f64. The table's x.5 cells (the sniper unarmoured and in diamond or netherite, the knife unarmoured and in diamond or netherite) round up and are left out of the test.
  - The `DeathCommit` that `script_player::damage` returns for a kill keeps MW2's amount; only the copy GSC and `finishplayerdamage` see is changed.
  - MW2's location multipliers (headshots, the sniper chest) are applied before the rule, as before.
- **Owner's test steps** (two windows with `scripts\mp-local.ps1`, a survival world, `/difficulty normal`, pvp on; host H and friend F, both with the kit below; full health before each step):
  1. H unarmoured. F shoots H once in the chest with the M4A1 from about 5 blocks: H loses 1.5 hearts (before: 3). **Screenshot 1.**
  2. H puts on the leather set: one M4A1 chest hit takes under 1 heart (0.8). H puts on the iron set instead: about a third of a heart. **Screenshot 2.**
  3. F fires the Barrett into H's chest: unarmoured H dies on the second hit. In iron, H takes about 9 chest hits for now (the Barrett's armour pierce comes in the next commit).
  4. H in the diamond set throws a frag (G) at H's own feet: H loses 1.5 hearts at most. Unarmoured, a frag at the feet takes about 7.5 hearts and no longer kills from full health. **Screenshot 3.**
  5. H swaps the plain diamond chestplate for the Protection IV one: the frag at the feet takes a little less.
  6. `/summon minecraft:zombie ~ ~ ~5`: the M4A1 kills it in about 4 body hits, as before. **Screenshot 4.**
  7. Swap roles: H shoots F; F's own window shows the same losses.
- **Kit** (24 stacks, for each player):

```
IW4L_MINECRAFT_KIT=iw4l:m4,iw4l:m4,iw4l:rifle_ammo*64,iw4l:rifle_ammo*64,iw4l:fmj,iw4l:barrett,iw4l:sniper_rounds*16,iw4l:frag_grenade*16,minecraft:crafting_table,minecraft:leather_helmet,minecraft:leather_chestplate,minecraft:leather_leggings,minecraft:leather_boots,minecraft:iron_helmet,minecraft:iron_chestplate,minecraft:iron_leggings,minecraft:iron_boots,minecraft:diamond_helmet,minecraft:diamond_chestplate+protection=4,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*16,minecraft:zombie_spawn_egg*2,minecraft:cooked_beef*32
```

### 2. LMGs, snipers and FMJ pierce part of a player's armour

Commit: the one after `54504900`. **Not vanilla** (WEAPONS.md 11.1, 11.2).

- **Changed:**
  - `sim/src/voxel.rs`: `set_pierce_table(Vec<f32>)` (weapon index -> pierce) and the crate's `pierce(weapon)`, 0 for an index outside the table.
  - `sim/src/script/host/players.rs` `gun_verdict`: the hit's weapon's pierce goes into `gun_damage::verdict`, where `kept = 1 - (1 - kept) * (1 - pierce)` cuts the armour and enchantment protection (the formula and `pierce(lmg, sniper, fmj)` came with the previous commit: LMG 0.25, sniper 0.5, FMJ +0.25, at most 0.75).
  - `render_anim/src/minecraft_fork_items.rs`: `pierce_table(guns, count)` builds one entry per weapon index from the `GunContent` bucket (MW2's weapon class: LMG, sniper) and an `fmj` attachment in the configuration's selection; `ForkLook::set_weapons` pushes it whenever the weapon registry changes, for the host and in a friend's game alike.
  - `docs/WEAPONS.md` 11.1: how the table is built.
- **Tests:** `sim` `gun_damage::tests::pierce_cuts_the_protection` (the pierce sums and the WEAPONS.md 11.3 rows M4A1 + FMJ, RPD, RPD + FMJ, MG4, Intervention and Intervention + FMJ, every cell but the x.5 ones; pierce above 0.75 is capped; Resistance V still refuses a pierced hit); `render_anim` `minecraft_fork_items::tests::lmgs_snipers_and_fmj_pierce_armour_by_weapon_index`. `test -p sim` (150), `test -p render_anim` (174) green, `check -p launcher` green.
- **Decisions:**
  - **The table is pushed when the weapon content changes, not per world**, and `deactivate` keeps it: it describes the weapon indices, which only change with the registry. The rule itself still only runs in a Minecraft world.
  - **Pierce follows the hit's weapon index**, so a grenade launcher or shotgun attachment on an LMG or sniper pierces nothing, and an akimbo or FMJ configuration counts as its own index.
  - The WEAPONS.md 11.3 pierce rows use the owner's values from the previous commit (snipers 105 on the chest, LMGs 40 or 30).
- **Owner's test steps** (two windows with `scripts\mp-local.ps1`, survival, pvp on; host H in the iron set, full health; friend F with the kit below):
  1. F shoots H in the chest with the bare M4A1: about a third of a heart per hit.
  2. F puts the M4A1 and an FMJ into the inventory crafting grid and takes the M4A1 with FMJ: each chest hit now takes about 0.6 heart. **Screenshot 1.**
  3. F fires the RPD (lmg ammo) into H: about 0.8 heart a hit.
  4. F fires the Barrett into H's chest: about 3 hearts a hit, the fourth kills. With the second FMJ on the Barrett: about 4 hearts a hit, the third kills. **Screenshot 2** (H's hearts after one Barrett hit).
  5. H puts on the diamond set: the bare M4A1 still takes about a third of a heart, the Barrett the same 3 hearts as against iron.
- **Kit** (17 stacks, for each player):

```
IW4L_MINECRAFT_KIT=iw4l:m4,iw4l:rifle_ammo*64,iw4l:fmj*2,iw4l:barrett,iw4l:sniper_rounds*16,iw4l:rpd,iw4l:lmg_ammo*64,minecraft:crafting_table,minecraft:iron_helmet,minecraft:iron_chestplate,minecraft:iron_leggings,minecraft:iron_boots,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*16,minecraft:cooked_beef*32
```

## C14 The totem's grace

### 1. A popped totem of undying turns bullets and explosions away for four seconds

Commit: the one after `415a85cb`. **Not vanilla** (WEAPONS.md 12).

- **Changed:**
  - `sim/src/voxel.rs`: `spend_death_protection(client, tick)` also records the grace until `tick + TOTEM_GRACE_TICKS` (80 sim ticks of 50 ms) in `TOTEM_GRACE`; the crate's `totem_grace(client, tick)`; cleared in `deactivate`.
  - `sim/src/script_player.rs`: `finish_damage` takes the tick and passes it to `spend_death_protection`; its callers `script/host/natives/player.rs` (`finishplayerdamage`) and the two `damage/height_bands.rs` tests pass theirs (the band tests' assertions are unchanged).
  - `sim/src/gun_damage.rs`: `GunHit::turned_by_totem` (bullets and explosions); `Victim.totem_grace`; `verdict` refuses a bullet or explosion gun hit while the sim grace or the mirrored `GunGuard.totem_grace` runs, so GSC is not called, health does not change and no hit marker shows. The knife and impacts are not covered. `script/host/players.rs` `gun_verdict` reads the sim grace at the hit's tick.
  - `minecraft_survival/src/survival.rs`: `Survival.totem_grace_until`, set by `protect` to the Minecraft tick + `TOTEM_GRACE_TICKS` (80); `totem_grace()`; `hurt` refuses a `DamageCause::is_explosion()` hit while it runs (knockback still comes through the level's `players_pushed`); `respawn` clears it (it rebuilds the state); `/kill` and the void use their own paths and are unaffected. `gun_armor` mirrors it as `GunGuard.totem_grace`.
  - `minecraft_survival/src/death.rs`: `IS_EXPLOSION` (the 26.3 jar's `#minecraft:is_explosion` tag: fireworks, explosion, player_explosion, bad_respawn_point, read from the jar) and `DamageCause::is_explosion`; a new `DamageCause::Explosion` (`minecraft:explosion`, message `explosion`) for a blast with no source entity.
  - `docs/WEAPONS.md` 12: what was built and the sourceless blast change.
- **Found:** TNT and end crystal blasts reach a player with no source entity, and `DamageCause::mob` turned them into `Generic`, so they were not explosions at all ("<name> died", only plain Protection). They are now `DamageCause::Explosion`, vanilla's `explosion(null, null)`: the grace covers them, the death message is vanilla's "<name> blew up", and Blast Protection softens them like vanilla. A creeper (with a source) stays `explosion.player`, a bed or anchor `badRespawnPoint`.
- **Tests:**
  - `sim` `gun_damage::tests::a_spent_totem_turns_guns_away_for_four_seconds` (a pop at tick 100: bullets, headshots, grenade splash and projectiles refused at 100 and 179, a hit lands at 180; another client is not covered; the mirrored flag refuses at 500; MW2 maps unchanged; one pop reported) and `a_knife_still_hurts_during_the_grace` (melee and impact land halved, a bullet is refused, Minecraft damage unchanged); `voxel::tests::a_totem_saves_one_death_and_reports_it` updated for the tick.
  - `minecraft_survival` `survival::tests::a_popped_totem_turns_explosions_away_for_four_seconds` (a creeper, a sourceless blast and a bad respawn point refused with health and absorption kept; mob melee and a fall still hurt; still refused after 79 ticks, lands after 80; the mirrored flag follows), `the_totem_grace_ends_on_respawn`, `death::tests::explosions_are_the_vanilla_is_explosion_types`.
  - `test -p sim` (152) and `test -p minecraft_survival` (694 and `vanilla_world`) green, `check -p launcher` green.
- **Decisions:**
  - **Two clocks, as designed:** the sim grace covers the rest of the same tick (the other shotgun pellets, a second grenade) before the host's next frame mirrors the Minecraft one. Under `/tick freeze` the Minecraft clock stops while the sim clock runs on, so the sim grace ends after 4 s while the mirrored one lasts until the freeze ends: accepted.
  - **The grace is checked before the guard.** A refused gun hit needs no armour; a client with a sim grace always has a guard anyway.
  - **A death during the grace (a knife, a fall)** respawns without the Minecraft grace; the sim grace keeps its last ticks after such a respawn, which needs a death and a respawn inside 4 s and is left as is.
  - **Sourceless blasts are vanilla's `explosion`** (above). A TNT lit by a player is told as "blew up" too: the vendored level keeps no igniter.
- **Owner's test steps** (two windows with `scripts\mp-local.ps1`, survival, pvp on; host H and friend F, both with the kit below):
  1. H unarmoured with a totem in the offhand. F shoots H down to 2-3 hearts, then holds the trigger on H: the totem pops (it rises, glitter, sound), and for about 4 s H's hearts do not drop and F sees no hit markers. Then the absorption hearts go first. **Screenshot 1** (during the 4 s, F visibly firing).
  2. H takes a new totem into the offhand and stands next to TNT, F lights it (flint and steel) and shoots H down just before it explodes: H is pushed but loses no hearts. **Screenshot 2.**
  3. Right after a pop, F knifes H (melee): H takes damage.
  4. Right after a pop, F throws a frag at H's feet: no damage, the blast still pushes.
  5. Swap roles: F's totem protects F the same way. **Screenshot 3** (F's window).
  6. Without a totem, H stands next to TNT that explodes: the death message is "H blew up".
- **Kit** (12 stacks, for each player):

```
IW4L_MINECRAFT_KIT=iw4l:m4,iw4l:rifle_ammo*64,iw4l:frag_grenade*16,minecraft:totem_of_undying*4,minecraft:tnt*8,minecraft:flint_and_steel,minecraft:golden_apple*16,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:cooked_beef*32
```

### 2. Gun armour is read without unwrap_or_default, as clippy asks

Commit: the one after `7660486d`. `Survival::gun_armor` (from `54504900`) used `Option::unwrap_or_default`, which the workspace's clippy configuration disallows; it now reads `unwrap_or((0.0, 0.0))`. No behaviour change, no new test; `clippy -p sim -p minecraft_survival -p render_anim --all-targets` reports nothing in this group's code, `test -p minecraft_survival` green. No owner steps or kit.

## Combined kit for group damage-totem

```
IW4L_MINECRAFT_KIT=iw4l:m4,iw4l:rifle_ammo*64,iw4l:fmj*2,iw4l:barrett,iw4l:sniper_rounds*16,iw4l:rpd,iw4l:lmg_ammo*64,iw4l:frag_grenade*16,minecraft:crafting_table,minecraft:leather_chestplate,minecraft:iron_helmet,minecraft:iron_chestplate,minecraft:iron_leggings,minecraft:iron_boots,minecraft:diamond_helmet,minecraft:diamond_chestplate+protection=4,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:totem_of_undying*4,minecraft:tnt*8,minecraft:flint_and_steel,minecraft:golden_apple*16,minecraft:zombie_spawn_egg*2,minecraft:cooked_beef*32
```

## Review of C11.9 to C14

### 1. The tactical insertion goes in the lethal slot, where MW2 keeps it

Commit: the one after `6c184122`. Fixes `edcd9cc5` and `d9247254`.

- **Defect:** `iw4l:tactical_insertion` was a tactical item (slot 44, Shift+G), but `flare_mp` is MW2 offhand class 5 ("other"), which the sim's `step::offhand_slot` gives to the lethal slot. `item::follow_grenades` refuses a weapon whose class does not fit its slot, so a tactical insertion in slot 44 was never given to MW2 and Shift+G could not plant it in the real game. The sim tests used class 2 as a stand-in, so they passed.
- **Checked in the owner's data:** a throwaway Python probe (not committed) inflated the owner's `common_mp.ff` and read each grenade's `WeaponDef.offhandClass` at the loader's x64 offset 104 (`fastfile_iw4` `load/weapon.rs`): frag 1, semtex 5, throwing knife 4, claymore 5, C4 5, flash 3, stun 2, smoke 2, `flare_mp` 5, M4A1 0. MW2 itself carries the tactical insertion as equipment.
- **Changed:**
  - `minecraft_terrain/src/fork_items.rs`: the tactical insertion row is `Kind::Lethal` and moves after C4, so the creative tab and the recipe book keep lethals before tacticals. `grenade_slot` now sends it to 43, so `fits`, shift-click, pickups and the first-join loadout (a class whose lethal is `flare_mp` now gets 32 of them instead of frags) follow.
  - `minecraft_terrain/data/fork_recipes.json`: its recipe group is `iw4l_lethal` (the recipe itself is unchanged).
  - `docs/WEAPONS.md` 10.1 (the table), 10.6 (G plants it, the probe's values) and the test step 9.
- **Tests:** `sim` `item::arsenal_tests`: the test content's `flare_mp` has the real class 5, and `a_tactical_insertion_is_thrown_from_the_lethal_slot_like_mw2_equipment` (in the lethal slot it becomes the lethal offhand with class 5 and its count; published in the tactical slot it is refused and has 0). `minecraft_terrain` `fork_items::tests`: 6 lethal and 3 tactical rows, `grenade_slot` of the tactical insertion is 43. `minecraft_survival` `menus::tests`: the lethal slot accepts it, the tactical slot refuses it. `test -p minecraft_terrain -p minecraft_survival -p sim` (sim 153) and `check -p launcher` green.
- **Decision:** follow the data (the brief's rule for a WEAPONS.md fact the data proves wrong). Making the sim treat `flare_mp` as a tactical would have needed a second class-5 offhand next to C4 in the same MW2 throw path.
- **Owner's test steps** (survival, a new world `IW4L_MINECRAFT_WORLD=r9-insertion-lethal`, kit below; replaces steps 1 and 4-9 of "A planted tactical insertion is a one-use respawn point", which said tactical slot and Shift+G):
  1. Place the bed and right-click it. Open the inventory and shift-click the tactical insertions: they go into the lethal slot (the left frame). Click them into the tactical slot: refused. **Screenshot 1.**
  2. Walk 30 blocks away, face north, press G: the flare is thrown and the lethal frame shows 3. Walk 20 blocks on, `/kill`: you respawn where you threw it, facing north. **Screenshot 2.**
  3. `/kill` again: you respawn at the bed.
  4. Take the frags into the lethal slot instead and press G: a frag is thrown, as before.
- **Kit** (4 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:white_bed,iw4l:tactical_insertion*4,iw4l:frag_grenade*16,minecraft:cobblestone*64
```

### 2. A knife hit pierces no armour, whatever gun is held

Commit: the one after `bd497d69`. Fixes `415a85cb`.

- **Defect:** `gun_verdict` looks the pierce up by the hit's weapon index, and MW2 tags a knife lunge (`combat.rs` melee, `DamageSource::Melee`) with the held weapon. A knife hit with a sniper in hand therefore pierced 0.5 (0.75 with FMJ), and with an LMG 0.25, although WEAPONS.md 11.3 gives the knife pierce 0: against iron a lunge took 56 instead of 23 with an FMJ Intervention held.
- **Changed:** `sim/src/gun_damage.rs` `verdict` passes the pierce on only for `GunHit::Bullet`; explosions and melee get 0. `docs/WEAPONS.md` 11.1 says so.
- **Tests:** `sim` `gun_damage::tests::only_bullets_pierce_whatever_gun_is_held` (a knife lunge with an FMJ sniper held takes 23 against iron, the same as with no pierce; a frag with an LMG's pierce is unchanged; an FMJ sniper bullet still takes 42). `test -p sim` (154) and `check -p launcher` green; `gun_damage.rs` still passes rustfmt.
- **Owner's test steps** (two windows with `scripts\mp-local.ps1`, survival, pvp on; host H in the iron set at full health, friend F with the kit below):
  1. F holds the Barrett with FMJ (FMJ and the Barrett in the crafting grid) and knifes H (melee key): H loses about 2.3 hearts, the same as a knife with the M4A1 held. **Screenshot 1.**
  2. F fires the Barrett into H's chest: about 4 hearts, as before.
- **Kit** (9 stacks, for each player):

```
IW4L_MINECRAFT_KIT=iw4l:m4,iw4l:barrett,iw4l:fmj,iw4l:sniper_rounds*16,minecraft:crafting_table,minecraft:iron_helmet,minecraft:iron_chestplate,minecraft:iron_leggings,minecraft:iron_boots
```

### 3. Grenade, test and class menu code passes clippy

Commit: the one after `7b15736d`. Final verification of the track: `clippy --all-targets` on every crate the track touched (`console`, `frame`, `hud`, `minecraft_commands`, `minecraft_net`, `minecraft_survival`, `minecraft_terrain`, `render_anim`, `sim`, `ui`, `weapon_iw4`, `minecraftoss-player`), with the warnings matched against the lines added since `r9-c2-base`. Three were on this track's lines; older warnings stay.

- `sim/src/combat.rs` (from `edcd9cc5`): the offhand round spend is one `if let ... && let ...` instead of two nested `if let`s (`collapsible_if`).
- `console/src/class_menu.rs` (from `43b4e0df`): the class status is read with `unwrap_or("")` instead of the disallowed `unwrap_or_default`.
- `hud/src/minecraft_creative.rs` (test from `b47be4bb`): the grenade slot tooltip test builds its `MinecraftUi` with the slots in the initializer (`field_reassign_with_default`).
- `test -p` on all twelve crates green (console 52, frame 28, hud 174, minecraft_commands 140, minecraft_net 18, minecraft_survival 694 + 1, minecraft_terrain 1204 with 3 ignored, render_anim 174, sim 154, ui 9, weapon_iw4 1, minecraftoss-player 198), `check -p launcher` green.
- No behaviour change, no new test, no owner steps or kit. The `minecraftoss-player` tests need the stand-in `third_party/scenarios/survival` files that `scripts\full-suite.ps1` creates; they were made for the run and removed again.

## Second review of the whole track

### 1. A thrown copy's clip is not written onto another copy of the same gun

Commit: the one after `114a788b`. Fixes `settle_arsenal` from the group arsenal.

- **Defect:** when a `Spent::Clip` report arrived after its bound copy had left the hotbar slot (thrown with Q while firing, moved to the cursor or a chest, or moved higher while another copy sat lower), `settle_arsenal` wrote the clip onto the first hotbar copy of the same MW2 weapon. With two M4s (thrown copy 25 rounds, other copy 0) the other copy got 23 rounds: rounds were duplicated, or lost in the reverse case.
- **Changed:** `minecraft_survival/src/guns.rs` `settle_arsenal` falls back only to a hotbar slot whose gun still has exactly the clips last bound for that weapon (the copy that moved); otherwise the report is dropped and the next binding reloads MW2 from the copy that is still there.
- **Tests:** `minecraft_survival` `guns::tests::a_thrown_copy_s_clip_is_not_written_onto_the_other_copy` (the copy left behind keeps 0 and is rebound with 0; a lone copy moved within the hotbar still takes its report). `test -p minecraft_survival` and `check -p launcher` green.
- **Owner's test steps** (survival, kit below):
  1. `/clear @s iw4l:rifle_ammo`, so nothing reloads. Put the two kit M4s on hotbar slots 1 and 2 if the loadout took those slots.
  2. Select the first M4 and fire it empty: its item bar shows one red pixel.
  3. Select the second M4, hold fire and press Q after a few rounds while still firing. **Screenshot 1** of the hotbar: the first M4 still shows the empty bar.
  4. Pick the thrown M4 up and select it: its clip is the full 30 minus what was fired before the throw, and the first M4 still clicks empty. **Screenshot 2.**
- **Kit** (2 stacks):

```
IW4L_MINECRAFT_KIT=iw4l:m4,iw4l:m4
```

### 2. Crafting a vanilla recipe unlocks it, like vanilla

Commit: the one after the clip fix. Fixes the recipe book group (C11.7.3).

- **Defect:** vanilla 26.3 adds a recipe to the book when its result is taken (`ResultSlot.checkTakeAchievements` calls `awardUsedRecipes`, which calls `Player.awardRecipes` for every non-special recipe; checked with javap). The fork unlocked crafting recipes only through `inventory_changed` advancements, so planks taken onto the cursor and put straight back into the grid made a crafting table whose recipe never reached the book or the player file.
- **Changed:**
  - Vendored `player/src/crafting.rs`: `RecipeBook::matching_id` returns the id of the recipe `matching` would use.
  - `minecraft_terrain/src/crafting.rs` `take_into_cursor`, which every result take goes through (click, shift-click `craft_all`, the number keys and F `swap`, Q `throw_result`): when the take succeeds, the recipe that matched is unlocked with `Inventory::unlock_recipe`. Fork recipes are skipped (they are always known and never saved), and the gun recipes of `gun_crafting` (attachments, akimbo, split) are special like vanilla's and unlock nothing. `Book::update` then highlights the new recipe and saves it in `recipeBook` like any other unlock, for the host and friends.
- **Tests:** `minecraft_terrain` `crafting::tests::taking_a_result_unlocks_the_recipe_that_made_it` (a log taken from the 2x2 grid unlocks the planks, the planks put straight back from the cursor make sticks with `craft_all` and unlock the stick recipe, Q on the result and a number key swap unlock theirs, nothing else is unlocked). `test -p minecraft_terrain -p minecraft_survival` and `check -p launcher` green.
- **Owner's test steps** (survival, a new world `IW4L_MINECRAFT_WORLD=r9-craft-unlock`, kit below):
  1. Open the inventory, put the log into the 2x2 grid and click the planks result: 4 planks on the cursor.
  2. Right-click the four grid cells so each holds one plank, without putting the planks into the inventory, and click the crafting table result. **Screenshot 1** of the recipe book: the crafting table is in it (with the new-recipe highlight).
  3. Save and quit, join again, open the book: the crafting table is still there. **Screenshot 2.**
- **Kit** (1 stack):

```
IW4L_MINECRAFT_KIT=minecraft:oak_log
```

### 3. Three new files pass rustfmt again

Commit: the one after the recipe unlock fix. Fixes `3cab4adb`, `d9247254` and `415a85cb`.

- **Defect:** `loadout.rs`, `minecraft_fork_items.rs` and `item/arsenal_tests.rs` are new in this track and passed `rustfmt --check` at `d69f2140`, but the segment c2 commits left them failing: the flash tuple in `GRENADE_STAND_INS`, the `minecraft_net::Dimension` import order, the long `pierce` call, flare `warn!` and `pierce_table` assert, and the flare test's asserts and `near` closure. The c2 clippy pass checked only `gun_damage.rs`, and a stdin `rustfmt --check` exits 0 even when it prints a diff.
- **Changed:** the layout rustfmt asks for at exactly those spots; nothing else.
- **Tests:** every `.rs` file added since `24dfdf71` passes `rustfmt --check` on its committed content with the repo's settings and its crate's edition (`fork_items.rs` with 2021). `check -p launcher` green. No behaviour change, no owner steps or kit.

### 4. An akimbo reload loads only the rounds there are

Commit: the one after the rustfmt fix. Fixes the ammo group (C11.9).

- **Defect:** both akimbo hands start a tick with the same copy of the reserve, and when both reload in the same tick (R, or both clips empty) each `reload_clip` takes from its own copy. With 12 pistol rounds and two empty USP clips each hand loaded 12, `spend_rounds` queued 24, `take_rounds` could remove only the 12 items there were, and `Spent::Clip` wrote 12 + 12 onto the item: 12 rounds made from nothing. A reserve that covers both clips was not affected.
- **Changed:** `sim/src/combat.rs`: for a client with a published arsenal in a Minecraft world, `loaded_within_stock` caps the rounds loaded in the tick at the reserve the tick started with, takes the extra back out of the left hand's clip first, then the right's, and leaves no reserve. Only that many rounds are queued. MW2 maps and bots keep the old path.
- **Tests:** `sim` `item::arsenal_tests::an_akimbo_reload_short_of_both_clips_loads_only_the_rounds_there_are` (12 rounds for two empty clips: 12 loaded, right 12 and left 0; 20 rounds: 12 and 8; 30 rounds: both full, reserve untouched). `test -p sim` (155) and `check -p launcher` green.
- **Owner's test steps** (survival, kit below):
  1. `/clear @s iw4l:pistol_ammo`. Put both USPs into the 2x2 grid and take the akimbo pair, put it on the hotbar and fire both pistols empty.
  2. `/give @s iw4l:pistol_ammo 12` and press R. **Screenshot 1:** MW2's counter shows 12 rounds in all (one pistol full, the other empty) and no pistol ammo is left in the inventory.
  3. Fire until both click: exactly 12 shots. Put the pair alone in the 2x2 grid to split it: both USPs are empty. **Screenshot 2.**
- **Kit** (2 stacks):

```
IW4L_MINECRAFT_KIT=iw4l:usp,iw4l:usp
```

## Combined kit from the brief (C11 to C14)

25 stacks, the brief's line for the whole track: guns, ammo, grenades, the tactical insertion, attachments, the crafting and grindstone ingredients, the iron set and two totems.

```
IW4L_MINECRAFT_KIT=iw4l:barrett,iw4l:usp,iw4l:usp,iw4l:sniper_rounds*16,iw4l:semtex*16,iw4l:tactical_insertion*2,iw4l:fmj,iw4l:red_dot_sight,minecraft:crafting_table,minecraft:grindstone,minecraft:chest*2,minecraft:iron_ingot*64,minecraft:iron_block*4,minecraft:copper_ingot*32,minecraft:gunpowder*64,minecraft:iron_nugget*32,minecraft:tripwire_hook*8,minecraft:redstone*16,minecraft:repeater*4,minecraft:iron_helmet,minecraft:iron_chestplate,minecraft:iron_leggings,minecraft:iron_boots,minecraft:totem_of_undying*2,minecraft:white_bed
```

## Vendored minecraftoss files

Patched by this track so far (7): `player/src/item_catalog.rs`, `core/src/item.rs`, `entities/src/world/piglins.rs`, `world/src/level/burning.rs` (itself a fork-added file), `player/src/crafting.rs`, `player/src/inventory.rs`, `player/src/chest.rs` (a test index, group grenade-slots). Added: none. Group damage-totem patched and added none.

## Protocol

`minecraft_net::PROTOCOL` is 4 (from 3), bumped once in "The recipe book places ingredients for the host and friends". It covers `ToClient::RecipeBook`, `ToServer::RecipeBookSettings`, `ToServer::RecipeSeen` (codec indices 31, 12 and 13) and `MenuAction::PlaceRecipe` (the last `MenuAction` variant). The grenade-slot group must not bump it again. Tracks A and B bump it too: whoever merges later takes the next number and renumbers the new codec indices.

Since the merge of tracks A and B into track C (`c27f9bc9`) `PROTOCOL` is 5. The grenade-slot group did not bump it: `PlayerView.slots` is a list, so its 45 slots (from 41) need no new message shape. Release 1 shipped protocol 4, and protocol 5 has not been released yet, so no released pair of builds disagrees on the slot count.

## State doc changes

- `MINECRAFT-SURVIVAL.md`, the command list (next to `/spawnpoint`, line 165): "`/tag <targets> add|remove|list` (level 2) adds, removes and lists a player's tags with vanilla's messages; the tags are saved in the player file's vanilla `Tags` list. Players only." And next to `/give`: "`/give`, `/clear` and the kit also take the fork's `iw4l:` items (guns, ammo, grenades, attachments, gun parts); a given gun comes with a full magazine, and `/give` takes `minecraft:custom_data`, e.g. `iw4l:m4[minecraft:custom_data={iw4l:{attachments:["reflex"]}}]`."
- `MINECRAFT-SURVIVAL.md`, the inventory section: "Fork items show MW2 pictures: guns their kill icon, grenades and attachments their class-menu picture, ammo and gun parts tinted vanilla sprites; a gun's tooltip lists its attachments, its clip and its ammo type before MW2's stat lines."
- `MINECRAFT-SURVIVAL.md`, the saving section: "Fork items (guns, ammo, grenades, attachments, gun parts) are saved in the player file, the ender chest, chests, shulker boxes and bundles as a vanilla `minecraft:carrot_on_a_stick` (guns) or an uneatable `minecraft:poisonous_potato` (everything else) with `custom_data.iw4l` (id, clip, attachments) and a translatable `item_name`, so vanilla 26.3 keeps them and our game turns them back. Killstreak items are never saved."
- `MULTIPLAYER.md` (what a friend sees): "Fork items reach a friend's slots and menus in the same vanilla form they are saved in (a carrot on a stick or poisonous potato with `custom_data.iw4l`), and the friend's game turns them back, so a friend sees the same guns, clips, attachments and names as the host. Ground items carry the item id itself."
- `MULTIPLAYER.md` (version checks): "The fork item table's hash is part of the registry fingerprint, so a friend whose build has another item table is refused."
- `MINECRAFT-SURVIVAL.md`, the guns section, replacing the free Intervention, SPAS-12 and UMP45 and the class guns as hotbar mirrors: "MW2 holds exactly the fork guns on the hotbar, in hotbar order, each gun once. A gun in the backpack, a chest or on the cursor is not in MW2. Selecting a hotbar gun raises it. The loaded rounds live on the item, so a gun put away and taken back keeps its clip, and two copies of one gun keep one clip each. MW2's own weapon drop and pickups are off in Minecraft worlds. For now the reserve ammo and the grenades still refill; ammo items and grenade slots come later. Killstreak weapons are still hotbar mirrors."
- `MULTIPLAYER.md` (per-player state on the host): "Each friend's guns follow their own hotbar the same way, through the same host function, and the host keeps each player's bound gun copies per UUID."
- `MINECRAFT-SURVIVAL.md`, the guns section: "Killstreak items cannot go into a chest or any other container cell, into the offhand, or be dropped with Q or from the cursor."
- `MINECRAFT-SURVIVAL.md`, the guns section: "Guns drop with Q, go into chests and every other container, and drop on death like any item, with their loaded rounds; `keep_inventory` keeps them. Killstreak items vanish on death. No gun goes into the offhand (the riot shield included). The Barrett and the M240 are netherite and survive fire and lava on the ground."
- `MINECRAFT-SURVIVAL.md`, the guns section: "Dropped fork items and fork items in item frames are drawn as their inventory picture, extruded like a vanilla item (16 pixels, a few pixels thick): guns as their MW2 kill icon, grenades and attachments as their class-menu picture, ammo and gun parts as tinted vanilla sprites. A grenade, ammo or an attachment held in the hand is drawn like a vanilla item. A picture the MW2 files lack is shown as a crossbow (guns) or a firework star."
- `MINECRAFT-SURVIVAL.md`, the guns section: "On the first join (also for a player file from round 8 or earlier, and for every friend) a player who is alive with an MW2 class gets the class's primary and secondary as items with their attachments and full magazines, on hotbar slots 1 and 2 when free, and three magazines of ammo per gun in the backpack; what does not fit drops at the feet. The player is tagged `iw4l_loadout_given` (vanilla `Tags`), so it happens once per player in all dimensions; `/tag @s remove iw4l_loadout_given` gives it again. A class gun the fork has no item for becomes the first gun of its category."
- `MULTIPLAYER.md` (per-player state on the host): "Each friend's first-join loadout is given on the host from the friend's own class, through the same function as the host's, and the tag is saved in the friend's player file."
- `MINECRAFT-SURVIVAL.md`, the crafting section: "Crafting results keep the components their recipe gives, so a crafted suspicious stew carries its effect (eating it does not apply the effect yet). Every fork gun, gun part, attachment, ammo type and grenade has one crafting recipe (`equipment`), our own vanilla-format JSON in `crates/minecraft_terrain/data/fork_recipes.json`; none matches a vanilla 26.3 grid. A crafted gun comes out empty. Fork recipes are always known and never saved in `recipeBook`."
- `MULTIPLAYER.md` (version checks): "The fork recipes are part of the same fork table hash."
- `MINECRAFT-SURVIVAL.md`, the crafting section: "Attachments go onto a gun in any crafting grid, the 2x2 included: one gun plus one or two attachment items, when MW2 has that combination (at most two attachments, akimbo counted). The gun keeps its name, its loaded rounds and its other attachments; rounds above a smaller clip come back as ammo. Two identical guns with the same attachments make an akimbo pair if MW2 offers akimbo for the gun; an akimbo gun alone in the grid splits, and the second gun stays in its cell. These recipes are not in the recipe book; attachment tooltips say how to use them."
- `MULTIPLAYER.md` (per-player state on the host): "A friend's gun crafting (attachments, akimbo, split) runs on the host through the same function as the host's; each screen shows the result from its own MW2 content."
- `MINECRAFT-SURVIVAL.md`, the grindstone (stations section): "A gun alone in the grindstone comes out with its attachments removed (akimbo kept), its clip cut to the bare size; the attachments and the extra rounds are lost, and it gives a little XP like disenchanting (3-5 for one attachment, 5-9 for two). Without MW2 content a gun gives no result there."
- `MINECRAFT-SURVIVAL.md`, line 398 (no recipe book), replaced by: "The inventory and the crafting table have vanilla's recipe book: the green button, the panel to the left with the Search, Equipment, Building, Misc and Redstone tabs, 20 recipes a page, the search box (fork items by their MW2 names), \"Showing craftable\", red buttons for missing ingredients, groups behind one button with the right-click alternatives, and ghost ingredients for a clicked recipe. Every fork recipe is always in it (Equipment, after vanilla's); vanilla recipes appear once unlocked. Not built: the furnace books and keyboard or pad navigation of the book."
- `MINECRAFT-SURVIVAL.md`, the saving section: "Each player's vanilla recipe unlocks, the recipes still waiting to be shown, and the recipe book's open and \"Showing craftable\" flags are saved in vanilla's `recipeBook` compound of the player file; a recipe id the 26.3 data does not know is dropped on load like vanilla, and fork recipes are never written (they are always known). Recipes unlock the vanilla way, from items entering the inventory, including the items carried when joining."
- `MULTIPLAYER.md` (per-player state on the host): "Each friend's recipe unlocks and book settings live on the host in the friend's player file; the host sends the friend's unlocked recipes when they change, and the friend sends its book settings and the recipes its book has shown."
- `MINECRAFT-SURVIVAL.md`, the recipe book sentence: "A click on a recipe moves its ingredients from the inventory into the grid (what was in the grid goes back first), another click adds one more set and shift-click places as many as fit; a recipe that cannot be made empties the grid and shows red ghost items."
- `MULTIPLAYER.md` (version checks): "The Minecraft multiplayer protocol is 4: friends place recipe-book recipes with `MenuAction::PlaceRecipe`, which the host applies to the friend's own grid."
- `MINECRAFT-SURVIVAL.md`, the creative section: "Every fork item is at the end of the creative Combat tab, in the fork table's order (pistols, machine pistols, SMGs, assault rifles, shotguns, specials, snipers, LMGs, launchers, then gun parts, attachments, ammo, lethal and tactical grenades); guns come without attachments and with a full magazine. The creative search finds them by their MW2 names."
- `MINECRAFT-SURVIVAL.md`, line 134 ("Guns sit in the hotbar with unlimited ammo"), the ammo half: "A gun's reserve is its ammo items in the offhand and slots 0-35 (never containers or the grenade slots), up to MW2's maximum: MW2's ammo counter shows the clip and that reserve. A reload takes exactly the rounds it loads, from the offhand first, then the main hand, then the inventory in slot order; with no ammo items an empty gun clicks. Pistols, machine pistols and SMGs share pistol ammo; the grenade launcher uses explosive rounds and the underbarrel shotgun shells; the Ballistic Knife reloads from throwing knives. Respawning, Scavenger and One Man Army refill nothing. In creative the reserve is always full and reloads use no items, but the clip still empties and reloads."
- `MULTIPLAYER.md` (per-player state on the host): "Each friend's gun reserve is counted from the friend's own inventory on the host, and the friend's reloads take the friend's items, through the same function as the host's."
- `MINECRAFT-SURVIVAL.md`, the inventory section: "A gun's loaded rounds show as an item bar under its icon (in the hotbar, the inventory and every menu, for the host and friends): green and full width when the clip is full, yellowing as it empties, one red pixel when empty; an akimbo pair shows both clips together."
- `MINECRAFT-SURVIVAL.md`, the inventory section: "The player inventory has two fork slots after the offhand: 43 takes only lethal grenades (frag, semtex, throwing knife, claymore, C4) and 44 only tactical ones (flash, stun, smoke, tactical insertion), up to 64 each; clicks, number keys, drags and creative refuse anything else. A shift-click on a grenade in the inventory screen moves it into its slot when that slot is empty or holds the same grenade; a picked-up grenade tops up its slot, or fills it when empty, before the backpack. Not vanilla."
- `MINECRAFT-SURVIVAL.md`, the saving section: "The grenade slots are saved in the player file under the fork's top-level key `iw4l_grenades` (a slot list, `Slot:0b` lethal, `Slot:1b` tactical, in the same vanilla carrier form as other fork items); vanilla 26.3 ignores the key and drops it if it re-saves the player."
- `MINECRAFT-SURVIVAL.md`, the death and `/clear` lines: "The grenade slots drop after the saddle on death unless `keep_inventory` is on; `/clear` covers them, and like vanilla also the (normally empty) body and saddle slots."
- `MULTIPLAYER.md` (per-player state on the host): "Each friend's grenade slots live on the host in the friend's inventory and player file; the friend's view carries all 45 slots, and the host checks the friend's clicks on 43 and 44 with the same rules as the host's."
- `decisions.md` (Not vanilla): "Two fork inventory slots, 43 lethal and 44 tactical, saved under the top-level key `iw4l_grenades`, never inside `equipment`."
- `MINECRAFT-SURVIVAL.md`, the guns section: "G throws the grenade in the lethal slot and Shift+G the one in the tactical slot, whatever is held; each throw, and a frag that blows up in the hand, uses one item, and when a throw empties the slot it refills from the inventory with the same grenade. MW2's grenade icons show the slot counts. An empty slot keeps the last grenade's icon with 0, so placed C4 can still be detonated. Picking a thrown knife up gives the item back. In creative throws use nothing. Bots keep unlimited grenades."
- `MULTIPLAYER.md` (per-player state on the host): "Each friend's throws take the friend's own grenade items on the host, through the same function as the host's."
- `MINECRAFT-SURVIVAL.md`, the inventory section: "The grenade slots are drawn as two offhand-style frames left of the hotbar (lethal nearer to the hotbar), always, with their item and count, and as two slots right of the recipe book button in the inventory and right of the leggings and boots in the creative inventory tab; an empty one shows the tooltip Lethal or Tactical."
- `MINECRAFT-SURVIVAL.md`, the first-join loadout sentence, added: "It also puts 32 of the class lethal grenade into the lethal slot and 32 of the class tactical into the tactical slot (frags and flashes when the class has none)."
- `MINECRAFT-SURVIVAL.md`, the respawn section: "A planted tactical insertion (Shift+G with it in the tactical slot) is a one-use respawn point ahead of the bed or respawn anchor: the next respawn stands where the player stood when throwing it, facing the same way, also from another dimension, and uses it up. A second plant moves it, an MW2 blast dealing 20 or more at its spot destroys it, a block or fluid on the spot drops it, and it is not saved. Not vanilla."
- `MULTIPLAYER.md` (per-player state on the host): "Each friend's tactical insertion respawn lives on the host per UUID, set from the friend's own throws through the same function as the host's."
- `decisions.md` (Not vanilla): "The tactical insertion's flare is a one-use respawn point ahead of the bed, kept on the host and never saved; only MW2 blasts destroy it."
- `MINECRAFT-SURVIVAL.md`, the guns section: "In a Minecraft world the class editor changes the perks and the deathstreak only: a click on a weapon or grenade row opens nothing and shows "Guns and grenades are items in Minecraft worlds" (also the editor's status line), and Reset keeps the class's guns and grenades. One Man Army and Bling change no item. MW2 maps and the main menu are unchanged."
- `MULTIPLAYER.md` (what a friend sees): "A friend's class editor follows the same rule while the friend is in the host's Minecraft world."
- `MINECRAFT-SURVIVAL.md` line 374 (MW2 damage through armour deferred), replaced by: "In a Minecraft world every MW2 weapon hit on a player (bullets, grenades, launchers, killstreaks, the knife, a player's own explosives) is halved and then reduced by the player's Minecraft armour at twice vanilla's armour share; armour and Protection, Projectile Protection (bullets) or Blast Protection (explosions) together take at most 80 %, and Resistance applies after that like vanilla (level V blocks every gun). Damage to mobs and Minecraft damage to players are unchanged. Not vanilla."
- `MULTIPLAYER.md` section 13 (MW2 damage ignores Minecraft armour), replaced by: "Gun hits between players run on the host through one rule for everyone: each frame the host mirrors every player's armour, enchantment protection and Resistance into the sim, and the victim's own armour reduces the hit before MW2's scripts see it."
- `decisions.md` (Not vanilla): "MW2 weapon damage to players in Minecraft worlds is halved and armour takes twice vanilla's share (armour and Protection together at most 80 %, Resistance after), so iron, diamond and netherite all reach the ceiling against light rounds; bots and MW2 maps are unchanged."
- `MINECRAFT-SURVIVAL.md`, after the gun damage sentence: "LMGs pierce a quarter of that protection, snipers half, and the FMJ attachment adds a quarter (at most three quarters), so heavy guns stay the answer to diamond and netherite."
- `decisions.md` (Not vanilla): "LMGs (0.25), snipers (0.5) and FMJ (+0.25, at most 0.75) pierce part of a player's armour and enchantment protection against guns."
- `MINECRAFT-SURVIVAL.md` line 55 (the totem's activation), added: "A popped totem, from Minecraft or MW2 damage, also turns away MW2 bullets and explosions and Minecraft explosions (TNT, creepers, end crystals, fireworks, beds and anchors) for 4 s: no hearts are lost and the shooter sees no hit marker, and a blast still pushes. The knife, Minecraft melee, arrows, fire, falls, the void and `/kill` still hurt. Respawning ends it. Not vanilla."
- `MINECRAFT-SURVIVAL.md`, the death messages line: "A blast with no source entity (TNT, end crystals) is vanilla's `minecraft:explosion`: \"<name> blew up\", softened by Blast Protection."
- `MULTIPLAYER.md` section 13, after the gun damage sentence: "Each player's totem grace is mirrored the same way, so a friend's popped totem turns the host's bullets and grenades away for 4 s and the other way round."
- `MULTIPLAYER.md` section 20, "PvP without guns": drop "MW2 damage through Minecraft armour" (built in round 9).
- `decisions.md` (Not vanilla): "A popped totem of undying turns MW2 bullets and explosions and Minecraft explosions away for 80 ticks (4 s), on the sim clock and the Minecraft clock; knockback stays, the knife and other damage still hurt. Vanilla has only the 20-tick hurt cooldown."
- Correction (review of C11.9 to C14), replacing the wording above wherever it names the tactical insertion: the grenade slot sentence reads "43 takes only lethal grenades (frag, semtex, throwing knife, claymore, C4, tactical insertion) and 44 only tactical ones (flash, stun, smoke)", and the respawn sentence begins "A planted tactical insertion (G with it in the lethal slot, where MW2 keeps it as equipment)".

### By line, as the brief names them

The entries above, gathered by the lines and sections the brief names (the line numbers are those of `MINECRAFT-SURVIVAL.md` and `MULTIPLAYER.md` at `c27f9bc9`):

- `MINECRAFT-SURVIVAL.md` line 55 (totem): the totem grace sentence above.
- `MINECRAFT-SURVIVAL.md` line 59 ("MW2 guns stay in their hotbar slots"), replaced by: "Guns, ammo, attachments and grenades drop like any item, with their loaded rounds; the grenade slots drop after the saddle; `keep_inventory` keeps them. Killstreak items vanish on death."
- `MINECRAFT-SURVIVAL.md` line 134 ("Guns sit in the hotbar with unlimited ammo"), replaced by the two guns-section entries above: MW2 holds exactly the fork guns on the hotbar, in hotbar order (the hotbar half), and a gun's reserve is its ammo items, taken on reload (the ammo half); then the gun damage sentence and its pierce sentence (round 9 group damage-totem).
- `MINECRAFT-SURVIVAL.md` line 350 ("MW2 guns ... not saved"), replaced by the saving entry above: fork items are saved as a vanilla `carrot_on_a_stick` or uneatable `poisonous_potato` with `custom_data.iw4l`; the grenade slots under `iw4l_grenades`; killstreak items and the tactical insertion flare are never saved.
- `MINECRAFT-SURVIVAL.md` line 374 ("For everyone"): drop "MW2 damage through Minecraft armour"; the gun damage sentence goes into the MW2 integration section after line 134.
- `MINECRAFT-SURVIVAL.md` line 398 ("GUI gaps"): drop "recipe book"; the recipe book entry above describes it.
- `MULTIPLAYER.md` section 10.3, replaced by: "Guns are fork items (`iw4l:` ids) in each player's inventory, saved and sent in their vanilla carrier form. The host settles every player's guns through the same function (`settle_arsenal`): MW2 holds exactly the hotbar guns, the selected slot drives the MW2 weapon through `weapon_request` for host and friends (with the \"needs sky\" refusal), reloads and throws take the player's own items, and killstreak weapons stay MW2 mirrors that are never saved. `SelectSlot` from the friend sets `selected`; the host changes it only on an MW2-initiated switch and sends it in `PlayerView`."
- `MULTIPLAYER.md` section 13, the "Armour" bullet, replaced by the gun damage and totem grace sentences above.
- `MULTIPLAYER.md` section 20: drop "MW2 damage through Minecraft armour" from "PvP without guns".
