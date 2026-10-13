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

## Vendored minecraftoss files

Patched by this track so far (5): `player/src/item_catalog.rs`, `core/src/item.rs`, `entities/src/world/piglins.rs`, `world/src/level/burning.rs` (itself a fork-added file), `player/src/crafting.rs`. Added: none.

## Protocol

`minecraft_net::PROTOCOL` not changed yet (it goes up once in the group book-place-creative).

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
