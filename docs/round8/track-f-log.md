# Track F log: fixes from the interim test of release 1

Branch `track-f`, worktree `C:\Dev\mc-wt\track-f`, target dir `D:\iw4l-wt\track-f`. Brief: `track-f.md` (the owner's findings F1 to F4 from section A and kit A of release 1, `4d45e4fe`). Base: `aaf968e1`.

A commit can't name its own hash, so each entry is written with its commit and names the commit before it; the last entry's hash is in `git log` under its subject.

## F1. MW2 models get their light grid and reflections again

Commit: the one after `aaf968e1`.

- **Cause:** every MW2 pass binds its textures through one bind group of four binding arrays: 2D (layout count 8192), cube (256), 3D (32) and samplers (128). `ExactTextureTable::create_bind_group` bound only the views the table had collected so far (a few hundred 2D views, a handful of cubes, two 3D views: the placeholder and the model lighting atlas). On this PC (DX12, RTX 4050) the descriptors of a partially filled array after the first one are not found where the shader looks for them: the cube and 3D arrays read zeros. So every MW2 model lost its model lighting atlas (3D, the light grid ambient) and its reflection probe (cube, also the MW2 sky box). On MW2 maps the sun still lit the models, which hid it. In a Minecraft world the models take no primary light by design (`neutral_light_grid_sample`, picked primary 0), so the atlas is their only light: the gun, the hands and the third-person body came out exactly black (0, 0, 0) everywhere, by day in the Overworld too, not only in the Nether. The elytra, armour and Minecraft items are drawn by the Minecraft pass and were not affected.
- **How it was found** (a debug build in its own run folder on a hidden desktop, a copy of `r7final` and fresh worlds, nothing of the owner's install written): the pass hook's raw MW2 colour was already black before the Minecraft lightmap; the shader's registers showed the colour map and the fog fine and the 3D atlas lookup at zero; `textureLoad` on the bound atlas at the eye's tile returned zero although the tile upload and even a full-texture write to the same `GpuImage` (same view id as table index 1) had gone through; with every array filled to its layout count the gun was lit. The same black showed with the round 7 build (`e2313625`) and the `3c2c62b` release, with Cold-Blooded and other classes, default settings and a cold shader cache, so it was not a round 8 change.
- **Changed:** `render_gpu` `drawsurf/texture_table.rs`: `full_binding_array` fills each array of the table bind group up to its layout count with the array's first entry (the placeholder view or sampler), so every real entry keeps its index and every descriptor slot is written. The sun-caster group masks the sun shadow views first, then fills.
- **Side effect (MW2 maps):** models on MW2 maps now get their light grid ambient and reflections as in MW2, and the MW2 sky box (a cube) shows; before it was black.
- **Tests:** `render_gpu` `texture_table`: `a_binding_array_is_filled_to_its_layout_count_after_its_own_entries` (own entries keep their order, the rest is the first entry, an empty array stays empty) and `every_table_array_is_bound_with_the_layout_count` (for the 2D, cube, 3D and sampler entries of `texture_table_bind_entries`, the table capacity equals the layout count and the filled array has that length).
- **Decisions:** the fix is in the MW2 texture table, not in the Minecraft lighting hook: the hook and the Moody lightmap were right (the lightmap view showed the body lit by the Nether's ambient and the lava's block light). Filling costs one descriptor per slot when a table's bind group is rebuilt (once per table epoch), not per draw.
- **Owner's test steps** (survival, `iw4l-survival.exe map minecraft:overworld`, kit below, any class):
  1. By day in the Overworld, hold an MW2 gun: the gun and the gloves show their textures and shading (they were black). **Screenshot 1.**
  2. Press F5: the MW2 body is lit, with the armour and the elytra on it. F5 again for the front view. **Screenshot 2.**
  3. Stand under a roof at night, away from torches: the body and the gun are dark, as Minecraft mobs are there; place a torch next to you: they brighten.
  4. Build a Nether portal (obsidian, flint and steel) and go through. In third person the body is dimly lit like the terrain, warmer near lava, and never a black silhouette. **Screenshot 3.**
  5. Optional, an MW2 map (`iw4l-survival.exe map mp_rust`): the sky shows clouds instead of black, and guns show reflections on their metal.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:obsidian*14,minecraft:flint_and_steel,minecraft:torch*64,minecraft:elytra,minecraft:iron_chestplate
```

## F2a. The spyglass and the spears show their inventory sprite as icon

Commit: the one after `456bef90`.

- **Cause:** the inventory icon took its model from `item_model_reference`, which follows a `minecraft:select`'s `fallback` first. The 26.3 spyglass definition is a `minecraft:display_context` select whose `gui`, `ground`, `fixed` and `on_shelf` case is `item/spyglass` (the flat sprite) and whose fallback is `item/spyglass_in_hand` (the small 3D tube with `item/spyglass_model`). So the hotbar and inventory drew the in-hand texture strip (screenshot 2). The six spears (`*_spear` with `*_spear_in_hand`) have the same definition and showed their in-hand sprite too. The bundles and the trident were not affected (same model in both cases, or the trident's special path).
- **Changed:** `minecraft_terrain` `interface.rs`: `item_gui_model_reference` picks the first `display_context` case whose `when` holds `gui`, else falls back to `item_model_reference`. `item_icons.rs` uses it for the icon. The held, dropped and framed models are unchanged (they keep `item_model_reference` and their own display contexts).
- **Tests:** `interface::tests::the_inventory_icon_takes_the_gui_case_of_a_display_context_select` (spyglass, bundle, a select with no `gui` case, a select on another property) and, with the 26.3 pack, `item_icons::tests::vanilla_spyglass_and_spear_icons_are_their_inventory_sprites` (the 32 px icon equals `item/spyglass` and `item/iron_spear` scaled with nearest).
- **Decisions:** the fix follows vanilla's `ItemModelResolver` for the GUI context instead of special-casing the spyglass, so every item with a GUI case (today the spyglass and the spears) is right.
- **Owner's test steps** (survival or creative, kit below):
  1. Open the inventory: the spyglass is the brass-and-glass sprite lying diagonally, like in vanilla, not a thin vertical strip. **Screenshot 1.**
  2. Put it on the hotbar: the same sprite there. Hold it: in hand it is still the small 3D spyglass.
  3. The iron spear in the inventory is the diagonal spear sprite. **Screenshot 2.**
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:spyglass,minecraft:iron_spear
```
