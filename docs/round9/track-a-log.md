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

## A2. Held items sit in the MW2 body's fists from the wrists

Commit: "Minecraft held items sit in the MW2 body's fists from the wrists, not on the weapon tags", the one after `76c3565a` (A1).

- **Cause** (as in the brief): every Minecraft hand item was placed on the MW2 weapon tags `tag_weapon_right` and `tag_weapon_left`, using the tag's posed matrix. That only works while the playing MW2 animation keys those tags at the hands. In the bind pose both tags share one point on the body's right side (the owner's log: the right tag about 11 units from the right wrist, the left tag about 48 units from the left wrist), so with an unkeyed tag the offhand item crossed to the right and landed on the main-hand item, and the inventory character's item sat above its shoulder. The wrist fallback never ran, because the kit has both tags.
- **Vanilla 26.3:** `ItemInHandLayer` draws each hand's item at the fist of its own arm (`translateToHand`, then `Rx(-90)`, `Ry(180)`, then `(+-1/16, 2/16, -10/16)`), so the two hands never share a point.
- **Changed:** `render_anim` `anim/minecraft_gear.rs` only.
  - `GearFit.fist` (`GEAR_FIT.fist = 3.5` map units): the step from the wrist along the forearm to the middle of the fist.
  - `hand_frame` never reads `HAND_TAGS`. The fist point is the wrist's bind position plus `fist` along the elbow-to-wrist bind direction, carried by the wrist's skinning delta, so it follows every MW2 arm pose like the hand mesh. The orientation comes from the skin delta of the first of `SPINE4`, `SPINE_UPPER`, `SPINE_LOWER` (else the wrist), so the item stays upright with the chest (the round 7 rule) and takes the body's yaw, the aim pitch, crouch, prone, the glide tilt and the inventory character's scale. The rest is unchanged: `model_to_hand`, the item scale and `hand_offset`, mirrored on x for the left hand.
  - `tag_weapon_right` and `tag_weapon_left` stay in `GEAR_BONES`: `measures()` and the `Minecraft gear bones missing` log line still name them, and the MW2 riot shield uses `tag_weapon_left` through sim.
  - Presentation only: host and friend build every body's rig from their own `PosedGearFrame`, so this fixes other players' bodies, the own F5 body and the inventory character on both sides. No protocol, server or `minecraft_terrain` change.
- **Tests:** `render_anim` `anim::minecraft_gear::tests`:
  - rewritten from the tag rule: `a_wrist_turns_into_the_vanilla_hand_frame_in_blocks` (was `a_hand_tag_turns_...`), `the_hand_offset_is_in_pixels_and_mirrored_for_the_left_hand` (wrists in place of tags), `the_fist_rides_the_wrist_and_the_frame_stays_upright_with_the_chest` (was `without_a_hand_tag_the_hand_rides_the_wrist_upright_from_the_bind_pose`);
  - new: `held_items_ignore_unkeyed_weapon_tags` (both tags at one right-side point; each hand within `fist` + 1 of its own wrist, the hands more than 15 units apart), `the_fist_steps_along_the_forearm` (bind pose and a 90 degree elbow bend), `the_hand_frame_turns_with_the_chest_and_the_puppet_scale`, `the_hand_frame_falls_back_to_the_wrist_without_a_spine` (and no wrist gives no hand).
- **Owner's test steps** (two windows: `& scripts\mp-local.ps1 -World r9-a2 -Fresh`, kit below in both; host: `/time set day`, `/op Friend`; stand 4 blocks apart, facing each other):
  1. **Host, F5 front view.** Diamond sword in the main hand, torch in the offhand (F), F5 twice. The sword comes out of the right fist, the torch out of the left; nothing floats and they don't touch. Select a pistol, wait, select the sword again: still in the fists. Repeat after a rifle and after an SMG. **Screenshot 1** (one per gun class).
  2. **Inventory.** E: the small character holds the sword in its right fist and the torch in its left, nothing above the shoulder (the screenshot 2 case). Turn it: the items stay in the hands. **Screenshot 2.**
  3. **Friend's view of the host.** Same sword and torch in the fists. The host walks, crouches, goes prone and looks up and down: the items stay in the fists and tilt with the chest. **Screenshot 3.**
  4. **Totem.** The friend empties the main hand and holds the totem in the offhand. In the host window the totem is in the friend's left fist, not low on the right (the screenshot 3 case). **Screenshot 4.**
  5. **Book.** The friend selects the enchanted book. Their inventory character and the host window show it in the right fist (the screenshot 4 case). **Screenshot 5.**
  6. **Gun plus offhand.** The host holds a gun with cobblestone in the offhand: the gun as before, the cobblestone in the left fist.
  7. **Log.** `latest.log` still has `Minecraft gear bones missing: []`. If an item sits a little inside or outside the fist, send the screenshot: `GEAR_FIT.fist` and `hand_offset` are the tuning values.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:diamond_sword,minecraft:torch*64,minecraft:cobblestone*64,minecraft:totem_of_undying,minecraft:enchanted_book,minecraft:iron_helmet
```

## Decisions

- **A1:** in a Minecraft world the stand-in map's DPVS no longer decides which MW2 scene entities are drawn; the frustum, sphere and LOD culling and the dimension band gate do, like vanilla's frustum and distance check. The cell walk is skipped rather than patched (for example by treating leaf 0 as cell 0), because the stand-in map's cells say nothing about a Minecraft world.
- **A2:** the hand anchor is the posed wrist plus a fixed step along the forearm, never the MW2 weapon tags, because the tags are only at the hands while the playing MW2 animation keys them (it depends on the last raised gun's class). The item keeps the chest's orientation, not the forearm's, as in round 7 ("as a vanilla player holds it with the arm down"). `GEAR_FIT.fist = 3.5` is a first guess (vanilla's fist centre is about 4 map units past the wrist at the 0.9375 item scale; MW2's hand is a little smaller) and is tuned from the owner's screenshots.

## State doc changes

Wording for the state docs, to be applied when the round's state is written (this track does not edit them):

- **`docs/MINECRAFT-SURVIVAL.md`, multiplayer and rendering:** "MW2 bodies (other players, corpses, grenades, dropped guns) are drawn in every dimension and at every depth. In Minecraft worlds the stand-in map's (`mp_rust`) cells no longer cull MW2 scene entities: every scene entity is linked into every cell, and only the frustum, sphere and LOD culling and the dimension band gate decide. MW2 maps are unchanged. MW2 FX and marks were not checked for the same culling (`adapters/fx/world_mark.rs`)."
- **`docs/MINECRAFT-SURVIVAL.md`, held items on the MW2 body:** "Minecraft items in the main hand and offhand sit in the MW2 body's right and left fist: the posed wrist plus 3.5 map units along the forearm (`GEAR_FIT.fist`), upright with the chest. The MW2 weapon tags are no longer used for Minecraft items, so the item no longer depends on which gun's animations are playing, and the two hands never overlap. The same rig serves other players' bodies, the own F5 body and the inventory character, for the host and every friend."

## Combined kit

Starting point from the brief's shared points (27 stacks; the other heads come from the A4 kit or `/give`):

```
IW4L_MINECRAFT_KIT=minecraft:obsidian*14,minecraft:flint_and_steel,minecraft:diamond_sword+sharpness=5,minecraft:torch*64,minecraft:cobblestone*64,minecraft:totem_of_undying,minecraft:enchanted_book,minecraft:spyglass,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*16,minecraft:cooked_beef*32,minecraft:end_crystal*16,minecraft:elytra,minecraft:firework_rocket*64,minecraft:dragon_head,minecraft:piglin_head,minecraft:skeleton_skull,minecraft:wither_skeleton_skull,minecraft:zombie_head,minecraft:creeper_head,minecraft:player_head,minecraft:carved_pumpkin,minecraft:white_bed,minecraft:red_bed
```
