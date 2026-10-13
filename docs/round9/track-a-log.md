# Track A log: multiplayer fixes

Branch `track-a`, worktree `C:\Dev\mc-wt\track-a`, target dir `D:\iw4l-wt\track-a`. Brief: `track-a.md` (the owner's findings A1 to A7 from the two-window test of round 8's release 2, `03e0c9be`). Base: `24dfdf71`.

A commit can't name its own hash, so each entry is written with its commit and names the commit before it; the last entry's hash is in `git log` under its subject.

## A1. MW2 bodies show in the Nether and deep caves

Commit: "MW2 bodies show in the Nether and deep caves: the stand-in map's cells no longer hide them in Minecraft worlds", the one after `24dfdf71`.

- **Cause** (as in the brief, re-checked at `24dfdf71`): every MW2 scene entity (the other players' bodies, corpses, grenades, dropped guns) is linked into the cells of the stand-in map `mp_rust` that its bounds reach. A Nether player is at map z of about -45,000, far below `mp_rust`, where the DPVS tree gives leaf 0 (no cell). The body was linked nowhere, so it was never marked visible, `scene_ent_hidden` was true and `pose_remote_bodies` returned `SceneHidden`: no body, no MW2 gun, no `PosedGearFrame` joints (so no armour or held items) and no posed head (so no name). Overworld players near the surface are inside the map's cell and End players (z about +43,000) land in it too, which is why only the Nether (and deep caves) failed.
- **Changed:** `render_frontend` `adapters/anim/dyn_ent_brush.rs` only.
  - `link_scene_ents` reads `sim::voxel::active()` once. In a Minecraft world every scene model and scene dobj is linked into every cell (`link_every_cell`, `dpvs_iw4::add_scene_ent_to_cell` per cell) instead of the cached DPVS walk. The brush branch (`filter_bmodel_into_cells`) is unchanged.
  - `exec_cell_scene_ent_cmds` and `drain_dpvs_ent_cmds` build their cell planes through `cell_planes(planes, minecraft)`, which drops them in a Minecraft world, so `reaches_cell(None, ..)` passes.
  - Sphere, frustum and LOD culling stay, so bodies off screen are still skipped. The band gate (`sim::voxel::shown_to` in `remote_body.rs`, `minecraft_gear.rs` and `overhead_names.rs`) is unchanged, so players in another dimension stay hidden.
  - One info line per session in `latest.log` the first time a scene entity would have had no cell: `minecraft scene ents: entity N at map z Z has no cell in the stand-in map, linked into every cell` (it confirms the cause in the owner's next log).
  - MW2 maps: `voxel::active()` is false, so linking and cell checks are exactly as before. No `minecraft_net` or protocol change.
- **Side effect (wanted):** other MW2 scene entities in the Nether and deep Overworld caves (corpses, thrown grenades, dropped guns) are drawn too.
- **Tests:** `render_frontend` `adapters::anim::dyn_ent_brush::tests` with a hand-built one-cell tree (plane z = 0, front is cell 0, back is leaf 0 like the ground under the stand-in map):
  - `a_body_below_the_stand_in_map_reaches_no_cell`: a body box at z -45,000 has no cell and `reaches_cell(Some(..))` is false; at z +43,000 it is in cell 0 (why the End worked).
  - `a_minecraft_world_links_every_scene_entity_into_every_cell`: `link_every_cell` sets the entity's bit in every cell (one and three cells) and no other entity's; `reaches_cell(None, ..)` is true at z -45,000.
  - `the_cell_planes_are_dropped_only_in_minecraft_worlds`.
  - `the_first_scene_entity_without_a_cell_is_found_below_the_map` (the log line's walk stops at the first entity below the map).
  - The band gate test `only_bodies_in_the_viewers_band_are_shown_to_them` (`sim/src/voxel.rs`) already covers Nether to Nether.
- **Follow-up, not checked:** MW2 FX and marks may be culled by `mp_rust`'s cells too: `render_frontend/src/adapters/fx/world_mark.rs` builds its own `DpvsPlanes`. If bullet impacts, blood or explosion effects are missing in the Nether, look there first.
- **Owner's test steps** (two windows: `& scripts\mp-local.ps1 -World a1-nether -Fresh`, kit below in both):
  1. **Overworld baseline.** Host: `/op Friend`. Both put on the helmet and chestplate and hold the sword, a few blocks apart. Each sees the other's soldier with armour and sword, and the name when aiming. **Screenshot 1** (both windows).
  2. **Deep in the Overworld.** Host: `/fill ~-4 -40 ~-4 ~4 -36 ~4 minecraft:air`, `/tp @s ~ -40 ~`, `/tp Friend ~3 ~ ~`. Place torches. Both see each other with armour, sword and name. **Screenshot 2** (both windows). Then go back up.
  3. **The Nether.** Build a portal and walk in together. Stand 3-4 blocks apart. Each sees the other's soldier, dimly lit like the terrain, with armour, sword and name (the screenshot 1 case of the finding). **Screenshot 3** (both windows).
  4. **Moving.** The friend walks 30 blocks away and back: the host sees them all the way, with leg animation. The friend switches to a gun: the host sees it in their hands.
  5. **Third person.** Both press F5: their own body shows in the Nether with armour.
  6. **Still hidden across dimensions.** The friend goes back through the portal: in the Nether the host sees no body, name or floating sword. The friend comes back: both see each other again.
  7. **The End.** Both go to the End: they still see each other.
  8. **No stand-in props.** Back at spawn: no MW2 desert props or crates appear anywhere.
  9. **Log.** The host's `latest.log` has one line `minecraft scene ents: entity ... has no cell in the stand-in map, linked into every cell` after entering the Nether.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:obsidian*10,minecraft:flint_and_steel,minecraft:iron_helmet,minecraft:iron_chestplate,minecraft:iron_sword,minecraft:torch*32,minecraft:cobblestone*64
```

## Decisions

- **A1:** in a Minecraft world the stand-in map's DPVS no longer decides which MW2 scene entities are drawn; the frustum, sphere and LOD culling and the dimension band gate do, like vanilla's frustum and distance check. The cell walk is skipped rather than patched (for example by treating leaf 0 as cell 0), because the stand-in map's cells say nothing about a Minecraft world.

## State doc changes

Wording for the state docs, to be applied when the round's state is written (this track does not edit them):

- **`docs/MINECRAFT-SURVIVAL.md`, multiplayer and rendering:** "MW2 bodies (other players, corpses, grenades, dropped guns) are drawn in every dimension and at every depth. In Minecraft worlds the stand-in map's (`mp_rust`) cells no longer cull MW2 scene entities: every scene entity is linked into every cell, and only the frustum, sphere and LOD culling and the dimension band gate decide. MW2 maps are unchanged. MW2 FX and marks were not checked for the same culling (`adapters/fx/world_mark.rs`)."

## Combined kit

Starting point from the brief's shared points (27 stacks; the other heads come from the A4 kit or `/give`):

```
IW4L_MINECRAFT_KIT=minecraft:obsidian*14,minecraft:flint_and_steel,minecraft:diamond_sword+sharpness=5,minecraft:torch*64,minecraft:cobblestone*64,minecraft:totem_of_undying,minecraft:enchanted_book,minecraft:spyglass,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*16,minecraft:cooked_beef*32,minecraft:end_crystal*16,minecraft:elytra,minecraft:firework_rocket*64,minecraft:dragon_head,minecraft:piglin_head,minecraft:skeleton_skull,minecraft:wither_skeleton_skull,minecraft:zombie_head,minecraft:creeper_head,minecraft:player_head,minecraft:carved_pumpkin,minecraft:white_bed,minecraft:red_bed
```
