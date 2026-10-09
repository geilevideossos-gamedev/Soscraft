# Track D log: saving

Branch `track-d`, worktree `C:\Dev\mc-wt\track-d`. Brief: `d-saving.md`.

A commit can't name its own hash, so each entry is written with its commit and its hash is filled in by the next commit; the last entry's hash is in `git log` under its subject.

## Group: world basics

### 1. Minecraft dropped items save their components as vanilla NBT

Commit: (filled in by the next entry)

- **Changed:**
  - New `minecraft_terrain/src/item_nbt.rs` holds `tag_json` and `json_tag`, moved out of `server_mobs.rs`, plus `components_tag`, `components_from_text`, `saved_components` and `components_text`.
  - The server level's item stacks now always keep their components as an NBT compound, the way vanilla's `DataComponentPatch` is saved. `stack_of`, `keep_dropped_contents` and the player world's `spawn_item`/`spawn_popped_item` convert the client's JSON into a compound, and `components_of` turns a compound back into JSON text for the client. Before, the level kept the JSON as an NBT string, `level_entity_tags` wrote `Item.components` as that string (vanilla rejects the stack), and a stack loaded with a compound reached the client as Rust debug text.
  - `load_level_entity` reads a compound or the old string form, and `level_entity_tags` always writes a compound.
  - `glint.rs` also reads `enchantment_glint_override` saved as a byte (0 or 1), since NBT has no booleans.
- **Tests:** `item_nbt`: `enchantments_and_a_name_become_a_compound_of_ints_and_strings`, `a_shulker_box_keeps_its_nested_items_and_their_potions`, `a_plain_potion_round_trips_through_its_text`, `components_saved_as_text_by_older_builds_still_load`, `booleans_become_bytes_and_large_numbers_stay_whole`. `server`: `an_enchanted_dropped_item_saves_its_components_as_vanilla_nbt` (save, load, and an old string-form save that is written back as a compound). `glint`: `a_glint_override_saved_as_a_byte_still_counts`. `a_shulker_box_the_server_blows_up_drops_once_with_its_contents` now spells its contents in NBT key order, which is the order the JSON text now comes back in.
- **Decisions:**
  - A stack loaded from disk keeps its NBT exactly, so its tag types (bytes, shorts, int arrays) survive. Only a stack that passed through the client's JSON is rebuilt from JSON: whole numbers become ints (a long when they don't fit), other numbers doubles and booleans bytes. That is what vanilla's codecs accept; `custom_data` can come back with other number types (brief risk "Types").
  - Empty component maps are left out, as vanilla leaves out an empty `components`.
  - Server-level stacks that carry components now merge and stack by their real components, because every one of them holds the same compound form.
- **Clippy:** `minecraft_terrain` has no clippy lints set and was not clippy-clean before this track (`approx_constant` errors in the mob renderers, `type_complexity` and `unwrap_or_default` warnings). None of the findings is on a line this commit touches.
- **Test steps:**
  1. In the `.env`, set `IW4L_MINECRAFT_WORLD=savetest` (any name but `temp`) and the kit below. Start.
  2. Put the cobblestone into the shulker box and break the box, so it drops with its contents.
  3. Throw (Q) the sword, the pickaxe and the enchanted book next to the shulker box. **Screenshot 1:** the four items on the ground; the sword, pickaxe and book glint.
  4. Quit through the menu and start again with the same world. **Screenshot 2:** the same four items in the same place, still glinting.
  5. Pick them up. Hover the sword (Sharpness V), the pickaxe (Efficiency V) and the book (Mending). Place the shulker box and open it: the cobblestone is inside. **Screenshot 3:** the sword's tooltip; **screenshot 4:** the open shulker box.
  6. Optional: copy `iw4l-artifacts\saves\savetest` into `.minecraft\saves` and open it in Minecraft 26.3. Before this commit vanilla threw the dropped sword away; now it lies there enchanted.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:diamond_sword+sharpness=5,minecraft:diamond_pickaxe+efficiency=5,minecraft:enchanted_book+mending,minecraft:red_shulker_box,minecraft:cobblestone*64
```
