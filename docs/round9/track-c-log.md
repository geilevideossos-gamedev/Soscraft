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

## Vendored minecraftoss files

Patched by this track so far (3): `player/src/item_catalog.rs`, `core/src/item.rs`, `entities/src/world/piglins.rs`. Added: none.

## Protocol

`minecraft_net::PROTOCOL` not changed yet (it goes up once in the group book-place-creative).

## State doc changes

- `MINECRAFT-SURVIVAL.md`, the command list (next to `/spawnpoint`, line 165): "`/tag <targets> add|remove|list` (level 2) adds, removes and lists a player's tags with vanilla's messages; the tags are saved in the player file's vanilla `Tags` list. Players only." And next to `/give`: "`/give`, `/clear` and the kit also take the fork's `iw4l:` items (guns, ammo, grenades, attachments, gun parts); a given gun comes with a full magazine, and `/give` takes `minecraft:custom_data`, e.g. `iw4l:m4[minecraft:custom_data={iw4l:{attachments:["reflex"]}}]`."
- `MINECRAFT-SURVIVAL.md`, the inventory section: "Fork items show MW2 pictures: guns their kill icon, grenades and attachments their class-menu picture, ammo and gun parts tinted vanilla sprites; a gun's tooltip lists its attachments, its clip and its ammo type before MW2's stat lines."
- `MINECRAFT-SURVIVAL.md`, the saving section: "Fork items (guns, ammo, grenades, attachments, gun parts) are saved in the player file, the ender chest, chests, shulker boxes and bundles as a vanilla `minecraft:carrot_on_a_stick` (guns) or an uneatable `minecraft:poisonous_potato` (everything else) with `custom_data.iw4l` (id, clip, attachments) and a translatable `item_name`, so vanilla 26.3 keeps them and our game turns them back. Killstreak items are never saved."
- `MULTIPLAYER.md` (version checks): "The fork item table's hash is part of the registry fingerprint, so a friend whose build has another item table is refused."
