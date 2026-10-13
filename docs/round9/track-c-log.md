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

## Vendored minecraftoss files

Patched by this track so far (3): `player/src/item_catalog.rs`, `core/src/item.rs`, `entities/src/world/piglins.rs`. Added: none.

## Protocol

`minecraft_net::PROTOCOL` not changed yet (it goes up once in the group book-place-creative).

## State doc changes

None yet.
