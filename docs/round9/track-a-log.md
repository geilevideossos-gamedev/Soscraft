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

## A5. Dragon respawn tests pin vanilla's crystal spots on the exit portal rim

Commit: "Minecraft dragon respawn tests pin vanilla's crystal spots on the exit portal rim", the one after `8e5a1567` (A2). Tests only, no behaviour change.

- **Finding:** four end crystals placed in spots the owner thought were wrong still brought a new dragon (screenshot 9).
- **Cause:** no defect. Our `try_respawn` (`minecraft_terrain/src/server.rs`) does what vanilla 26.3's `EnderDragonFight.tryRespawn` does: for each horizontal direction it looks for crystals whose 2x2x2 box meets the 1x1x1 cell at `portal.above(1).relative(dir, 3)` (strict intersection, `END_CRYSTAL` sized 2.0 x 2.0). A crystal stands at the top centre of the block it was placed on, so its box reaches one block to each side. Any of the three straight rim blocks on a side counts; only the four diagonal rim blocks at (+-2, +-2) do not. In screenshot 9 the bottom crystal was one block off the middle, still on a straight rim block.
- **Tests:**
  - `minecraft_terrain` `end_crystal::tests::a_crystal_counts_for_a_cell_its_two_block_box_touches`: the side's middle block and both straight blocks beside it count; a corner block counts for neither neighbouring side; two blocks off the middle does not; a crystal raised one block does not; one block lower just outside the rim does.
  - `minecraft_terrain` `server::tests::only_crystals_touching_the_four_side_cells_of_the_exit_portal_start_a_respawn` (real 26.3 data through `respawn_world`): four crystals on the diagonal rim blocks start nothing and report nothing; then on the side-end blocks (1, -3), (3, 1), (-1, 3), (-3, -1) three are not enough and the fourth gives exactly one `FightReport::Respawning` at stage `start` with 4 crystals. In a fresh world a crystal on obsidian over the rim's middle (standing a block higher) plus three good ones starts nothing.
  - Run: `test -p minecraft_terrain --locked end_crystal` and `... respawn` (both green; the server test was checked to run on the real data, not skip).
- **Note on the brief's test plan:** the corner crystals have to be removed before the side-end crystals go down. A crystal on a corner block fills the rim block beside it (its box reaches over it), so a side-end crystal cannot be placed next to a standing corner crystal, as in vanilla (`EndCrystalItem.useOn` refuses when an entity is in the 1x2x1 space). The test asserts that refusal too. So the respawn's 4 crystals are the side-end ones only.
- **Owner's test steps** (needs a dead dragon: kill the respawned one first; kit below):
  1. `/gamemode creative`, fly over the exit portal and look straight down. The rim has three straight blocks on each side and one diagonal block at each corner.
  2. One crystal on each of the four diagonal corner blocks. `/gamemode survival`, wait 10 s: no beams, no new dragon. **Screenshot 1** from above.
  3. Break the four corner crystals (a crystal beside a corner one can't be placed while it stands, as in vanilla). One crystal on an end block of each straight side, one block beside the middle. Right after the fourth: the portal goes out, beams rise, the pillars are rebuilt, and after about 25 s the new dragon and its bar appear. Vanilla 26.3 does the same, because the crystal's 2x2 box touches the middle block's column. **Screenshot 2.**
  4. Multiplayer: the friend places the four side crystals instead. Both windows see the beams and the new dragon.
  5. If a corner-only set in step 2 does start a respawn, press F3 and note the coordinates of each crystal's block and of the portal pillar. **Screenshot 3.**
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:end_crystal*16,minecraft:obsidian*8,minecraft:diamond_sword+sharpness=5,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots+feather_falling=4,minecraft:golden_apple*16,minecraft:cobblestone*64,minecraft:elytra,minecraft:firework_rocket*64
```

## A4. Worn mob heads and carved pumpkins

### Commit 1: worn heads drawn on the MW2 body

Commit: "Worn mob heads and carved pumpkins are drawn on the MW2 body", the one after `51002a83` (A5).

- **Cause** (as in the brief, re-checked at `51002a83`): head-slot items were only drawn when they were armour. `append_armor_on_rig` draws the layers from `armor_asset`, which knows only the helmets and the turtle helmet, so every skull and the carved pumpkin drew nothing: not on other players, not on the own F5 body and not on the inventory character, for the host and every friend. The gear already reaches every window (slot 39 is `armor[0]` of the replicated `GearView`), so no net change is needed.
- **Vanilla 26.3** (`CustomHeadLayer.submit`, checked with javap): after `translateToHead`, a skull block item is `scale(1.1875)` then `SkullBlockRenderer.submitSkull(animationPos, ..)` with no foil; any other item that the armour layer does not draw takes `translate(0, -0.25, 0)`, `rotY(180)`, `scale(0.625, -0.625, -0.625)` and its `head` display transform, with foil.
- **Changed** (`minecraft_terrain`, plus one default field in `render_anim`):
  - `special_models.rs`: `push_head` takes the animation position. The dragon jaw is `((a*pi*0.2).sin() + 1)*0.2`, the piglin's left ear `-((a*pi*0.2*1.2).cos() + 2.5)*0.2` and the right ear `((a*pi*0.2).cos() + 2.5)*0.2` (`DragonHeadModel`, `PiglinHeadModel`). Placed and held heads pass 0.0, so they are unchanged. New `worn_skull(item_id)` (the seven skull items) and `worn_head_model(head, animation)` (the skull at `SKULL_SCALE`). `Head` is now `pub(crate)`.
  - `mesh.rs`: the element loop of `ItemVisuals::append_model` moved unchanged into `pub(crate) fn append_elements`. New `HeldDisplay::Head` and `ItemVisual.head` from the item's `head` display transform (identity apart from vanilla's centring for blocks without one, such as the carved pumpkin through `block/block.json`).
  - New `worn_head.rs`: `SKULL_SCALE = 1.1875`, `skull_pose`, `item_pose`, `is_helmet` and `append_worn_head`. It draws nothing without an item or a head frame, or when the armour layer draws the item (a helmet). A skull is drawn through `append_elements` at the body's light with no glint. Anything else goes through `append_held_items` with `HeldDisplay::Head`, with glint when the item has it.
  - `body_render.rs`: `BodyRig.head_swing` (0 by default; commit 2 fills it) and an `append_worn_head` call right after the armour. The skull and entity textures were already in every window's atlas (`special_models::textures()`).
  - `render_anim` `anim/minecraft_gear.rs`: `body_rig` sets `head_swing: 0.0` (the new field only).
- **Tests** (`minecraft_terrain`):
  - `special_models`: `worn_skulls_are_the_seven_heads_and_not_pumpkins_or_helmets`, `the_dragon_jaw_follows_the_walk_and_rests_where_the_block_does` (0.2 rad at 0 like the placed block, 0.4 at 2.5, 0 at 7.5), `piglin_ears_flap_at_their_own_rates` (+-0.7 at 0 like the placed block; at 5.0 the right ear 0.3 and the left `-(cos(1.2 pi) + 2.5)*0.2`).
  - `worn_head`: `a_worn_skull_is_the_head_cube_scaled_like_vanilla` (6 faces spanning 9.5/16, uvs inside the skeleton sheet, the body's light, no glint), `a_worn_dragon_head_points_its_snout_forward` (snout below -15 px, the jaw present, only the jaw moves with the walk), `a_worn_carved_pumpkin_is_its_block_model_centred_on_the_head` (6 faces of 10 px centred at head pixel (0, -4, 0), the carved face at the lowest z, glint only when enchanted), `a_helmet_is_left_to_the_armour_layer`.
  - `body_render`: `worn_heads_are_drawn_with_the_body_and_lit_where_it_is` (a zombie head gives 12 faces, lit at the rig's cell, no glint of either kind, nothing without a head frame).
  - `mesh`: `the_head_display_is_identity_for_blocks_without_one`.
  - Run: `test -p minecraft_terrain --locked` (all green apart from `sections::tests::background_full_update_matches_the_synchronous_one`, which failed once under load in the full run and passes alone; it does not touch this code), `check -p launcher --locked`.
- **Owner's test steps** (alone, then two windows: `& scripts\mp-local.ps1 -World r9-a4 -Fresh`, kit below in both; host `/time set day`, `/op Friend`):
  1. **Inventory.** E, drag the dragon head into the helmet slot. The small character wears it with the snout forward (the screenshot 7 case). **Screenshot 1.**
  2. **F5.** The dragon head sits on the soldier's head (the screenshot 8 case). **Screenshot 2.** (The jaw stays at rest until commit 2.)
  3. **The other heads.** Skeleton skull, wither skeleton skull, zombie head, creeper head, player head and piglin head in turn: each sits on the head, a little bigger than it. **Screenshot 3** of any where the MW2 helmet or face pokes through.
  4. **Carved pumpkin.** F5: the pumpkin with its carved face forward sits on the head, the size of the MW2 head. **Screenshot 4.**
  5. **Diamond helmet.** Drawn as before, with no head or pumpkin under it.
  6. **Two windows.** The host wears the dragon head and the friend the carved pumpkin. Each sees the other's head item on the body, and their own in F5 and in the inventory. **Screenshot 5** (both windows).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:dragon_head,minecraft:piglin_head,minecraft:skeleton_skull,minecraft:wither_skeleton_skull,minecraft:zombie_head,minecraft:creeper_head,minecraft:player_head,minecraft:carved_pumpkin,minecraft:diamond_helmet,minecraft:spyglass
```

### Commit 2: the walk swing moves the dragon jaw and the piglin ears

Commit: "A worn dragon head opens its jaw and a piglin head flaps its ears while walking", the one after `580d7723` (A4 commit 1).

- **Vanilla 26.3:** `wornHeadAnimationPos` is the wearer's walk animation position (`WalkAnimationState`, run every client tick by `calculateEntityAnimation`): horizontal distance only, `target = min(dist*4, 1)`, `speed += (target - speed)*0.4`, `position += speed`, rendered at `position - speed*(1 - partialTick)`. Every client computes it for every player it shows.
- **Changed** (`render_anim`, plus `PartialEq` on `minecraft_terrain::walk_animation::WalkAnimation`):
  - `anim/minecraft_glide.rs`: `GlideInput.alive` (`ps.pm_type < PM_TYPE_DEAD`). `BodyGlide` keeps a `WalkAnimation` and a 20 Hz clock: each whole tick of frame time (at most 10 a frame) calls `walk.update(hypot(motion x, motion y), false)`, or `walk.stop()` while dead. `BodyGlide::walk_position()` is the position at the clock's partial tick; `McGlide::walk_position(client)` is 0 for a client it does not know.
  - `anim/minecraft_gear.rs`: `gear_rigs` sets `head_swing` from `glide.walk_position(body.client)`, next to the wings.
  - `step_body_glides` already runs on the host and on every friend for every presented player, from that player's presented `PlayerState`, so the jaw and ears move the same on other players' bodies, the own F5 body and the inventory character (vanilla's inventory doll shows the live walk too). No protocol, server or save change.
- **Tests** (`render_anim`):
  - `minecraft_glide`: `the_walk_swing_advances_once_per_tick_and_eases_like_vanilla` (one 0.05 s step at motion [0.3, 0, 0] gives speed 0.4 and position 0.4; two half steps give the same; the next tick 0.64 and 1.04; a 2 s frame runs only 10 ticks; vertical motion alone does not swing; dead stops at 0; the rendered swing interpolates within the tick; an unknown client is 0).
  - `minecraft_gear`: `gear_rigs_take_each_clients_walk_swing` (a walking client's rig and its puppet take its swing, a standing one and one without a glide entry stay at 0).
  - Run: `test -p render_anim --locked` (155 passed), `check -p launcher --locked`.
- **Owner's test steps** (kit of commit 1):
  1. **Dragon head, F5.** Wear the dragon head, F5, walk forward: the lower jaw opens and closes about twice a second at walking speed (faster when sprinting). Stop: it eases to a halt and stays where it is. **Screenshot 1** with the jaw open.
  2. **Inventory.** Walk, stop with the jaw open, press E: the small character's jaw is open the same amount as the F5 body's was.
  3. **Piglin head.** Wear it and walk in F5: both ears flap, a little out of step with each other. **Screenshot 2.**
  4. **Two windows.** The host wears the dragon head and walks: the friend sees the jaw move. The friend wears the piglin head in the Nether and walks: the host sees the ears flap. **Screenshot 3** (both windows).
  5. **Respawn.** Die while wearing the dragon head (`/kill`), respawn, put it on again and stand still in F5: the jaw is at rest (0.2 rad open, like a placed dragon head).

### Commit 3: the carved pumpkin blur

Commit: "A worn carved pumpkin blurs the first-person view like vanilla", the one after `f5d652cb` (A4 commit 2).

- **Vanilla 26.3** (`Hud.extractCameraOverlays`): only in first person; while scoping only the spyglass overlay is drawn; otherwise every equipped item whose `equippable` has a `camera_overlay` is blitted over the whole screen at alpha 1. In 26.3 only `minecraft:carved_pumpkin` has one, `misc/pumpkinblur` (256x256, `"blur": true`, so linear filtering).
- **Changed:**
  - `frame/src/minecraft_ui.rs`: two fields appended to `MinecraftUi`, `pumpkin_blur: Option<Handle<Image>>` and `camera_overlay: bool` (fields only; track B edits this file too).
  - `minecraft_terrain` `body_gear.rs`: `camera_overlay(slots)`, true when slot 39 holds `minecraft:carved_pumpkin`.
  - `render_anim` `minecraft_portal.rs`: `PUMPKIN_BLUR = "minecraft:misc/pumpkinblur"` and `blurred(..)`, the `strip` image with `ImageSampler::linear()`. The HUD's tess pass binds each image's own sampler (`render_gpu` `iw_tess.rs` uses `gpu_image.sampler`), so no `HudImages` change was needed.
  - Host (`minecraft_world.rs`): loads the blur next to the spyglass scope and sets `ui.camera_overlay = alive && camera_overlay(&inventory.slots)` next to `ui.scoping`.
  - Friend (`minecraft_remote.rs`): loads the blur next to its spyglass scope and sets `ui.camera_overlay` from its replicated inventory next to `ui.scoping`. While the friend is dead `ui.active` is false, which hides the overlay.
  - `hud` `minecraft_spyglass.rs`: pure `overlay_job(active, scoping, first_person, wearing, has_texture)` (`None`, `Scope` or `PumpkinBlur`) and `camera_overlay_quad(size)`. When the job is the blur, one full-screen quad (`mc_pumpkin_blur`, st 0..1, white, alpha 1) goes into the spyglass pass; the scope branch is unchanged.
- **Tests:** `minecraft_terrain` `body_gear::tests::only_a_carved_pumpkin_on_the_head_blurs_the_view`; `hud` `minecraft_spyglass::tests::the_pumpkin_blur_covers_the_screen_only_in_first_person_and_not_while_scoping`. Run: `test -p minecraft_terrain`, `-p render_anim`, `-p hud`, `-p frame` and `check -p launcher` (all `--locked`).
- **For track D (friend overlays, not done here):** a friend's window never sets `ui.freeze_outline` (only `minecraft_world.rs` loads it, next to the spyglass scope), so the powder-snow outline is probably missing for friends. Loading it in `minecraft_remote.rs` next to the spyglass scope and the pumpkin blur would be the fix.
- **Owner's test steps** (kit of commit 1; alone, then two windows):
  1. **First person.** Put the carved pumpkin in the helmet slot and close the inventory: the orange pumpkin blur covers the screen, dark at the edges, with the carved eye holes clear. It is smooth, not pixelated. **Screenshot 1.**
  2. **F5.** No blur; the pumpkin with its carved face forward sits on the soldier's head. F5 again (front view): no blur either. Back in first person: the blur is back.
  3. **Spyglass.** In first person, hold the spyglass and right-click: only the scope shows, no blur. Release: the blur is back. **Screenshot 2** while scoping.
  4. **Take it off.** Move the pumpkin out of the helmet slot: the blur goes. Put on the diamond helmet: no blur.
  5. **Death.** Die wearing the pumpkin: no blur on the death screen.
  6. **Two windows.** The friend wears the pumpkin: the friend has the blur, their inventory character and F5 show the pumpkin, the host sees the pumpkin on the friend's body and has no blur. Then the other way round. **Screenshot 3** (both windows).

## A3. The ender dragon is drawn from the client's own ticks

### Commit 1: the dragon

Commit: "The Minecraft dragon is drawn from its own client ticks, smooth for the host and friends", the one after `7fbcf9e7` (A4 commit 3).

- **Cause** (as in the brief, re-checked at `7fbcf9e7`): `EntityView::receive` replaced `self.dragons` with the newest server `DragonView`, and `meshes` drew it with the server's `previous_position.lerp(position, partial)` and the server's flight history, at the partial of the view's own client clock. Snapshots arrive off the client tick (the friend's state frames come over the relay with jitter and in batches; the host's level thread answers asynchronously), so at each client tick the drawn dragon snapped back up to a tick of flight and jumped forward when the next snapshot landed. The flap sound and growls were played per received snapshot, so merged frames lost some.
- **Vanilla 26.3:** the dragon keeps `updateInterval` 3; the client glides to each server target over 3 client ticks (`SteppedInterpolationHandler`), and `EnderDragon.aiStep` runs on the client: the flap (with its sound), `flightHistory.record(getY(), getYRot())` from the client's own body and the growl timer; `tickDeath` and `tickCount` run on both sides. The renderer reads only client values.
- **Changed:**
  - New `minecraft_terrain/src/client_dragon.rs` (`pub mod client_dragon` next to `client_mobs`):
    - `ClientDragons { bodies: ClientMobs, dragons }`. `receive(views)` turns each `DragonView` into a `ServerMob` (update interval 3, size 16 x 8, eye height 6.8, head yaw = yaw, `dead` at health 0) and hands them to an unchanged `ClientMobs`, which replays vanilla's tracker (delta packets every 3rd snapshot, a full sync for a long move, a snap beyond 64 blocks) and the 3-step interpolation. Gone ids are dropped; a new id starts at its position with the host's flap, `tick_count` and `death_time` and an empty flight history.
    - Per snapshot: the flap step is the host's `flap_time - o_flap_time`; a step of 0 (no AI, dying) holds the host's flap; `tick_count` and `death_time` resync to the host when more than 2 ticks off.
    - `tick()` once per client tick: the bodies tick, then each dragon counts `tick_count`, keeps `o_flap_time`, and either counts `death_time` (dying, up to 200, nothing recorded) or adds its flap step and records `(body y, wrap_degrees(body yaw))` into its own `FlightHistory`. It reports whether the wings passed the top of their beat (`flapping`, moved here from `entities::dragon_flapping`).
    - `views()` gives the drawn `DragonView`s: the client body's position, previous position and yaw, the client flap, clocks and history (the host's history until the first client tick), and the host's health, hurt time, sitting, phase, parts and nearest crystal.
  - `minecraft_survival/src/entity_view.rs`: a private `drawn_dragons` next to `dragons`. `receive` feeds it after replacing `self.dragons`, which stays the host's view for `shot_boxes`, `end_on_ray`, the entity lookups and the parts. `advance` ticks it inside its client tick and plays the flap sound and the growls from there at the drawn position (no longer from `receive`). `meshes` passes the drawn views to `append_dragons`, `append_beams` and `append_rays`. `pub fn drawn_dragons()` for tests.
  - `entities.rs`: `dragon_flapping` removed (now `client_dragon::flapping`); its test imports the moved fn.
  - The host's `Entities.view` and the friend's `RemoteLevel.view` are both `EntityView`, so this covers every window: the host's own, the friend's view of the host's dragon and every friend's. No `minecraft_net`, wire, session, `mob_sender`, `remote.rs` (code), `client_mobs.rs`, server-state or hit-box change, no protocol bump.
- **Tests:**
  - `minecraft_terrain` `client_dragon::tests` (no game data): `a_drawn_dragon_never_steps_back_when_frames_arrive_off_the_client_tick` (frames per client tick `[1,0,2,1,1,0,2,1,1,1]` four times, sampled at partials 0, 0.25, 0.5 and 0.75: the drawn x never decreases and no step exceeds 1.5 blocks, while the old newest-server-view method goes back), `the_drawn_dragon_reaches_a_stopped_server_position_within_a_few_ticks` (within 1/4096 after at most 5 client ticks, for three stop phases against the tracker's interval), `the_flight_history_is_recorded_from_the_drawn_body_each_client_tick_and_not_while_dying`, `the_wings_beat_at_the_host_rate_between_frames` (one flap in ten ticks at a step of 0.1, a step of 0 holds the host's flap), `a_dragon_missing_from_a_snapshot_is_forgotten_and_a_new_one_starts_at_its_position`, `a_dragon_more_than_64_blocks_away_snaps_instead_of_gliding` (a 10-block move still glides), `the_beam_and_ray_clocks_follow_client_ticks_and_resync_to_the_host`.
  - `minecraft_survival` `remote::tests::the_friend_s_dragon_glides_on_its_own_ticks_when_frames_come_late` (real 26.3 data, checked to run): a friend in the End gets `StateFrame`s with a dragon flying +x in the irregular pattern, `view.advance` runs in quarter ticks; the drawn x never decreases and `shot_boxes(None)` holds the host's 8 part boxes of the newest frame.
  - Kept green: `dragon_render::tests`, `entities::tests::a_dragon_flaps_when_its_wings_pass_the_top_of_their_beat`, `dragon_parts_are_flat_hit_boxes_keyed_by_dragon_and_part`, the `session`, `mob_sender`, `heard`, `dragon_fight` and remote dragon tests.
  - Run: `test -p minecraft_terrain --locked --lib` (all green apart from the known `sections::tests::background_full_update_matches_the_synchronous_one`, which failed under load and passes alone), `test -p minecraft_survival --locked` (594 passed), `check -p launcher --locked`.
- **Owner's test steps** (two windows: `& scripts\mp-local.ps1 -World mp-dragon -Fresh`, kit below in both; host `/op Friend`):
  1. Host: `/execute in minecraft:the_end run tp @a 0 70 0`. Both put on the diamond armour.
  2. Both stand still near the middle and watch the dragon circle the pillars for about 30 s, in each window. The body, neck and tail glide without twitching, the wings beat evenly and the flap sound keeps time with them, the heal beam stays fixed to the dragon. **Screenshot 1** in each window while it circles close by (the screenshot 5 case).
  3. The friend holds the spyglass on the circling dragon for 10 s: smooth.
  4. Host: `/tick freeze`. The dragon stops dead in both windows without shaking. `/tick unfreeze`: it flies on smoothly.
  5. Shoot crystals until it perches. Watch the landing, the breath and the take-off from the friend window: no snapping. Shoot the dragon from both windows: both boss bars go down. Note it if shots near the drawn head miss while it flies fast (the drawn dragon trails the hit boxes by about 1.5 to 3 ticks, as other mobs do).
  6. Kill it. It rises smoothly with the light rays turning evenly in both windows. **Screenshot 2.**
  7. Close the friend window. Host alone: `/summon minecraft:ender_dragon ~ ~20 ~`. It flies as smoothly as in step 2.
  8. If the dragon still stutters, look for `Can't keep up!` in `latest.log`: that would be the End level itself falling behind, which this fix does not address.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:bow,minecraft:arrow*64,minecraft:spyglass,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*16,minecraft:cooked_beef*32,minecraft:cobblestone*64,minecraft:water_bucket,minecraft:ender_pearl*16,minecraft:snowball*16
```

### Commit 2: the dragon's fireballs

Commit: "Dragon fireballs fly smoothly for every player", the one after `51555474` (A3 commit 1).

- **Cause:** the dragon's fireballs were drawn like the dragon, from the host's newest `FireballView` (`previous_position.lerp(position, partial)` at the client's partial in `EntityView::meshes`), so they jumped back and forward when frames arrived off the client tick. (The brief's `entity_view.rs:957` is the ghast fireball line just below; the dragon's are the `dragon_fireball::append_fireballs` call above it. Ghast and blaze fireballs come from the mirrored `EntityWorld` and are not part of this fix.)
- **Vanilla 26.3:** `DRAGON_FIREBALL` has `updateInterval(10)` and the client moves the fireball itself each tick (`AbstractHurtingProjectile.tick`: the velocity gains `accelerationPower` along itself and is scaled by the inertia, then the position moves by it).
- **Changed:**
  - `minecraft_terrain/src/client_dragon.rs`: `ClientFireballs`. `receive(views)` drops gone ids, starts a new id at the host's position and previous position, and takes the host's last velocity (`position - previous_position`) on every frame. It snaps to the host's position only when the host is more than a block off the client copy's track, that is more than a block from every point within two ticks of travel before or after it, so the frame timing alone never snaps it. `tick()` once per client tick: the velocity takes vanilla's step (`(v + normalize(v)*0.1)*0.95`, the constants from `minecraftoss_entities::fireball`), then the position moves by it. `views()` gives `FireballView`s.
  - `minecraft_survival/src/entity_view.rs`: a private `drawn_fireballs` next to `drawn_dragons`, fed in `receive` after `self.dragon_fireballs`, ticked in `advance` inside the client tick, and drawn by `dragon_fireball::append_fireballs`. `self.dragon_fireballs` (the host's) still places the breath trail particles; hits are judged on the host.
  - Every window again: the host's own view and every friend's `RemoteLevel.view`. No protocol, wire or server change.
- **Tests:** `minecraft_terrain` `client_dragon::tests::a_drawn_fireball_moves_every_client_tick_and_snaps_when_far_off` (a host fireball with vanilla's motion; frames per client tick `[1,0,2,1,1,0,2,1,1,1]` three times: the drawn copy moves on every client tick, never snaps while on its track, its x never goes back at partials 0 to 0.75 while the host's newest view does, and it stays within two ticks of travel plus a block of the host; a 5-block sideways jump snaps it to the host; a fireball missing from a frame is forgotten).
  - Run: `test -p minecraft_terrain --locked --lib` (1137 passed), `test -p minecraft_survival --locked` (594 passed), `check -p launcher --locked`.
- **Owner's test steps** (as commit 1, two windows in the End, kit of commit 1):
  1. Shoot crystals and wait until the dragon flies at a player and shoots a fireball (it strafes the player it targets). Watch it from the other window too: the purple fireball flies in a smooth line and speeds up, with no back-and-forth. **Screenshot 1** of a fireball in flight from the friend window.
  2. Stand where it lands: the breath cloud appears where the fireball hits, in both windows.

## A6. The creative inventory for a friend in creative mode

### Commit 1: creative slot sets on the wire

Commit: "Friends' creative slot changes reach the host and are checked like vanilla", the one after `6ef71048` (A3 commit 2).

- **Cause** (as in the brief, re-checked at `6ef71048`): a friend had no way to send a creative slot change. `ToServer` had nothing like vanilla's `ServerboundSetCreativeModeSlotPacket`, and `read_inbox` (`render_anim/src/minecraft_host.rs`) and `MenuInput` (`minecraft_survival/src/menus.rs`) had no creative input. The server-side check already existed (`minecraft_survival::creative::set_slot`).
- **Vanilla 26.3** (`handleSetCreativeModeSlot`): nothing unless the player has infinite materials (`abilities.instabuild`); slot 1..=45 of the inventory menu; the stack empty or at most its max stack size; a known, enabled item; a drop is slot -1. No op check.
- **Changed:**
  - `minecraft_net/src/messages.rs`: `pub enum CreativeTarget { Menu(u8), Carried, Drop }` and `ToServer::CreativeSlot { slot, stack: Option<Nbt> }`, appended at the end of `ToServer`. **`PROTOCOL` 3 -> 4** (the only bump in this track; track B bumps 3 -> 4 too, so the second merge takes 5 and renumbers its codec test index). An older friend is refused with the usual version message.
  - `minecraft_survival/src/menus.rs`: `MenuInput::Creative { slot, stack }`, in the same per-friend queue as clicks, renames and closes, so a creative set and a click sent in one host tick apply in the order sent. `take_inputs` gains `known: &dyn Fn(&str) -> bool`. A creative input (`creative_set`) is ignored while the player is dead; the Nbt is decoded with `views::view_stack` against the player's recipe book, and an Nbt that does not decode is ignored (never turned into a clear); then `creative::set_slot` on that friend's `Player` checks game mode, slot, item and count. `Ok(Some(drop))` goes to `clicked.thrown`, which `friend_menus` already tosses from the friend's eye. A refusal writes one debug line and changes nothing.
  - `render_anim/src/minecraft_players.rs` `friend_menus`: passes `known`, the same closure as the host's (`globals.chat.registries().items`).
  - `render_anim/src/minecraft_host.rs` `read_inbox`: `ToServer::CreativeSlot` pushes `MenuInput::Creative` to that friend's session (`MENU_INPUTS = 256` caps the queue).
  - Replication back needs nothing new: slots 0-40 go out in `PlayerView.slots`, the cursor and the 2x2 grid in the inventory menu's `MenuSlots`.
- **Tests:**
  - `minecraft_net` `codec::tests::every_message_to_the_server_round_trips`: index 12, `seen = [false; 13]`, samples `Menu(36)` with a stack, `Carried` with none, `Drop` with a stack.
  - `minecraft_survival` `menus::tests`: `a_creative_friends_slot_set_reaches_their_inventory` (Menu(36) stone x64 lands in slot 0, Menu(5) a diamond helmet in slot 39, a Carried set goes out as the inventory menu's new carried), `a_survival_or_spectator_friends_creative_sets_are_refused` (survival, adventure, spectator), `creative_sets_refuse_unknown_items_bad_slots_and_overfull_stacks` (unknown id, Menu(0), Menu(46), 65 stone, 2 swords with `max_stack_size` 1, an `iw4:weapon/` stack refused by `known`, undecodable Nbt never clears a slot), `a_creative_drop_is_thrown_not_stored`, `creative_sets_and_clicks_apply_in_the_order_sent` (Carried torch x16, then a click on slot 3: the torch is in slot 3 and the cursor is empty), `a_dead_friends_creative_set_is_ignored`.
  - Run: `test -p minecraft_net --locked` (15 passed), `test -p minecraft_survival --locked` (600 passed), `check -p launcher --locked`.
- **Owner's test steps:** nothing visible yet; the friend's game sends creative sets from commit 2. Both windows must run this build (protocol 4): an older build on either side is refused at join with the version message. A friend in survival plays exactly as before.
- **For track C:** the friend path checks items with the same `known` closure as the host, built in `friend_menus` from `globals.chat.registries().items`. When guns become real items, change `known` in both places (`minecraft_world.rs` near `creative.prepare` and `minecraft_players.rs` `friend_menus`).

### Commit 2: the friend's creative screen

Commit: "A friend in creative mode gets the creative inventory with tabs and search", the one after `848da31d` (A6 commit 1).

- **Cause** (re-checked at `848da31d`): the friend's `show_player` (`render_anim/src/minecraft_remote.rs`) never called `CreativeUi::prepare`, `set_item_tags`, `apply` or `hotbar_key`, so the tab loader never started and the HUD fell back to `McScreen::Inventory`; `show_menus` also forced any creative screen back to the inventory; the friend had no item tags for `#tag` search, and C/X plus a digit was never consumed.
- **Changed:**
  - `render_anim/src/minecraft_creative.rs`: `CreativeUi::apply` is split into `sets(ui, inventory) -> Vec<SetSlot>` (takes the creative clicks out of `ui.clicks` as before, leaves the rest, resets the screen to the inventory when not creative) and `apply` (`sets` plus `creative::apply`, so the host behaves as before). `sets` works out each click from a scratch copy of the slots and cursor with the earlier clicks of the same frame applied, so two clicks in one frame chain like the host's did; the given inventory is never changed. `hotbar_key` is split the same way into `hotbar_sets` (a load returns the nine sets; a save writes the hotbar file locally and returns nothing) and `hotbar_key`. A private `hotbar_file` field (None = `iw4l-artifacts\hotbar.nbt`) lets the test write to a temp file. The tabs are not touched (track C).
  - `minecraft_terrain/src/locate.rs`: `pub fn datapack_tags(pack, registry)` holds the old body of `tags`, which now calls it.
  - `minecraft_remote.rs`:
    - the friend's `Prepared` gains `item_tags`, built on the loader thread with `datapack_tags(&registries.datapack, "item")` (the registry fingerprint check makes them the host's);
    - `show_player` builds the mirrored `Inventory` (slots, the cursor from the open menu or the inventory menu, the 2x2 grid) before `show_menus` and whether or not the friend is alive; then `creative.prepare(view.abilities.instabuild, command_level >= 2 && minecraft_operator_items)`, `set_item_tags`, and every set from `sets` and `hotbar_sets` goes out through the new pure `creative_message(slot, stack)` as `ToServer::CreativeSlot`. `creative_message` returns None for a menu index above 255 and for a Some stack that `slot_nbt` cannot encode, so it never sends a clear by mistake. After `show_menus` the cursor is read again (a close in the same frame changes the menu), and the same inventory goes on to `publish`;
    - `show_menus` no longer forces `McScreen::Creative` back to the inventory (`sets` does that when the friend is not creative);
    - the `friend` system takes `Option<Res<frame::GameSettings>>` (in the camera tuple) for `minecraft_operator_items`.
  - Ordinary inventory clicks inside the creative screen still go out as `ToServer::Click` on the inventory menu.
- **Tests:**
  - `render_anim` `minecraft_creative::tests`: `sets_take_creative_clicks_without_touching_the_inventory` (clone stone from slot 2 gives a carried 64 stone; a clone of slot 3 in the same frame gives nothing because the cursor is now full; a plain inventory click stays in `ui.clicks`; the trash clears the cursor; the inventory is unchanged), `hotbar_sets_load_nine_slots_and_save_locally` (C+2 writes the row to the file and returns no set; X+2 returns Menu(36..=44) with the torch at Menu(40); nothing while not creative), `a_loaded_creative_ui_turns_the_inventory_screen_into_the_creative_screen`; `losing_creative_turns_the_creative_screen_into_the_inventory` still passes.
  - `render_anim` `minecraft_remote::tests::creative_message_maps_targets_and_never_sends_a_clear_for_an_unencodable_stack`.
  - `minecraft_terrain` `locate::tests::datapack_tags_are_the_tags_of_the_world_generator` (data-gated: the same map as `tags(gen, "item")`, and `minecraft:logs` holds `minecraft:oak_log`).
  - Run: `test -p render_anim --locked` (159 passed), `test -p minecraft_terrain --locked` (1138 passed, 3 ignored), `check -p launcher --locked`.
- **Owner's test steps** (two windows, the host and one friend, both on this build; the friend is not op; kit below in both windows):
  1. Host: `/gamemode creative <friend>`. Friend: press E: the creative screen opens with the tabs and no Operator Utilities tab. **Screenshot 1.**
  2. Friend: the Search tab, type `oak`, then `#logs`: only logs and stems are shown. **Screenshot 2.**
  3. Friend: take an oak log to hotbar slot 1 and shift-click a diamond sword from a tab into the hotbar, close. The host sees the friend holding them. **Screenshot 3** from the host window.
  4. Friend: Q and Ctrl+Q over cobblestone in a tab: one and then a stack of cobblestone fly out in both windows. **Screenshot 4.**
  5. Friend: the Survival Inventory tab: put a diamond helmet on the head (the host sees it on the friend's body), drop a stack onto the trash slot, then shift-click the trash slot: the inventory is cleared. **Screenshot 5.**
  6. Friend: middle-click an item in a tab and a stack in the inventory: a full stack is on the cursor each time.
  7. Friend, screen closed: fill the hotbar, C+1 saves it ("Saved hotbar (restore with X+1)"), clear it with the trash, X+1 restores it; the Saved Hotbars tab shows the row. The file is the friend's own `iw4l-artifacts\hotbar.nbt`.
  8. Host: `/gamemode survival <friend>` while the friend's creative screen is open: it turns into the survival inventory. `/gamemode creative <friend>`: the next E opens the creative screen again.
  9. Host: `/op <friend>`. Friend: `minecraft_operator_items=true` in their `settings.cfg`, restart: the Operator Utilities tab appears. `/deop <friend>`: it is gone on the next open.
  10. The host's own creative screen works as before. **Screenshot 6.**

```
IW4L_MINECRAFT_KIT=minecraft:cobblestone*64,minecraft:diamond_sword,minecraft:oak_log*16,minecraft:diamond_helmet
```

### Commit 3: creative drops are throttled

Commit: "Creative drops are throttled like vanilla", the one after `27fdf953` (A6 commit 2).

- **Vanilla 26.3** (checked with javap): `ServerGamePacketListenerImpl.dropSpamThrottler` is `TickThrottler(20, 1480)`. A valid creative drop (slot < 0) is thrown only while `isUnderThreshold()` (count < 1480), then `increment()` adds 20; otherwise the server logs "Player {} was dropping items too fast in creative mode, ignoring." and drops nothing. `tick()` takes 1 off every server tick down to 0. So 74 drops in a burst pass, the 75th is ignored, and after that one drop passes every 20 ticks (one a second).
- **Changed:**
  - `minecraft_survival/src/creative.rs`: `DropThrottle { count }` with `tick(ticks)` and `allow(name)` (refuses at 1480 or more with vanilla's warning, else adds 20), `DROP_STEP = 20`, `DROP_THRESHOLD = 1480`.
  - `minecraft_survival/src/player.rs`: `Player.drop_throttle`, appended at the end, not saved (vanilla keeps it on the connection).
  - `minecraft_survival/src/menus.rs` `creative_set` (the friend path): a valid Drop set is thrown only when `drop_throttle.allow` passes.
  - `render_anim/src/minecraft_world.rs` (the host path): the stacks `CreativeUi::apply` throws go through the host player's `drop_throttle` the same way.
  - `render_anim/src/minecraft_players.rs` `movement` (the per-player tick for the host and every friend): `drop_throttle.tick(hand_ticks)` beside the item cooldowns.
- **Tests:** `minecraft_survival` `menus::tests::creative_drops_are_throttled_like_vanilla` (75 drops in one batch throw 74 and leave the count at 1480; one tick later one more passes; then nothing until 20 more ticks; menu sets are never throttled).
  - Run: `test -p minecraft_survival --locked` (601 passed), `test -p render_anim --locked` (159 passed), `check -p launcher --locked`.
- **Owner's test steps** (host and friend, both in creative, kit of commit 2):
  1. Friend: open the creative screen and hold Ctrl+Q over cobblestone in a tab for ten seconds (or click Q very fast): about 74 stacks fly out at once, then one more about every second, in both windows. **Screenshot 1.**
  2. The host does the same in its own window: the same limit.
  3. Normal throwing (a few Q presses) is not affected.

## A7. Players stuck inside each other after a spawn or a teleport

### Commit 1: players pass through each other

Commit: "Players in Minecraft worlds pass through each other instead of sticking", the one after `e5f969ed` (A6 commit 3).

- **Cause** (as in the brief, re-checked at `e5f969ed`): to each other's pmove, players were hard MW2 capsules (`alive_body_clips` fed `ClipBackend`, whose `trace` ends in `clip_move_to_players`: a 30x70-unit capsule with `CONTENTS_BODY`). A player who starts inside another gets startsolid and, for any horizontal move, allsolid, so `slide.rs` does not move them, and `correct_solid` only tries 1-unit nudges. MW2's own fix, `phase_stuck_in_client`, sets an eject velocity, but the allsolid traces never let it move anyone. Both players were frozen.
- **Vanilla 26.3:** players are never solid to each other (`canBeCollidedWith` is false); they only push each other (commit 2).
- **Changed:** `sim` `step.rs` only.
  - `alive_body_clips` calls the new `player_body_clips(world, block_world)` with `crate::voxel::active()`. In a Minecraft world it returns no bodies, so pmove, the script slide and `script_mantle` trace through other players. On MW2 maps the list is the same as before (alive clients and the prediction's remote bodies, minus noclip spectators).
  - `phase_stuck_in_client` (MW2's 25 u/s eject plus 300 ms of hard landing) no longer runs in a Minecraft world.
  - The `CONTENTS_BODY` area links (`link_player_area`) are untouched, so bullets, melee and lag compensation still hit players.
  - The friend predicts through the same `alive_body_clips` (`step_predicted`), so the friend no longer collides with the host or other friends either. No protocol or message change.
- **Tests** (`sim` `step::tests`, independent of the process-global voxel world):
  - `block_world_players_are_no_body_to_bump_into`: four alive clients (one a noclip spectator): `player_body_clips(&world, true)` is empty; with false it is clients 1, 3 and 4.
  - `a_trace_starting_inside_another_player_is_free_in_a_block_world`: a 15-unit move starting 5 units from another player's origin is free (startsolid 0, fraction 1) with the Minecraft list, and startsolid and allsolid with the MW2 list.
  - `spectators_are_no_body_for_other_players_to_bump_into` now calls `player_body_clips(&world, false)`: through `alive_body_clips` it read the process-global voxel flag, which another test in the same run turns on.
  - Run: `test -p sim --locked` (119 passed), `check -p launcher --locked`.
- **Owner's test steps** (two windows: `& scripts\mp-local.ps1 -World r9-a7 -Fresh`, kit below in both; host `/op Friend`):
  1. The friend: `/tp <host name>`. They land inside the host. Both can walk away at once; nobody is frozen. (They do not drift apart on their own until commit 2.) **Screenshot 1** right after the teleport, from both windows.
  2. The host: `/tp <friend name>`: the same.
  3. Walk into each other: you pass through (until commit 2 adds the push). Crouch and jump inside each other: nothing sticks.
  4. With pvp on, shoot the other with an MW2 gun and hit them with the diamond sword: hits still register.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:white_bed,minecraft:red_bed,minecraft:oak_planks*64,minecraft:cobblestone*64,minecraft:obsidian*14,minecraft:flint_and_steel,minecraft:cooked_beef*32,minecraft:diamond_sword
```

### Commit 2: overlapping players push each other apart

Commit: "Overlapping Minecraft players push each other apart like vanilla", the one after `bb470a28` (A7 commit 1).

- **Vanilla 26.3** (checked with javap): `LivingEntity.pushEntities` takes `Level.getPushableEntities(this, box)`; on the client `EntitySelector.pushableBy` keeps only the local player (and only when `isPushable`: alive, not a spectator, not on a climbable), so each client pushes its own player once per overlapping player per tick. `Entity.push(other)`: nothing for passengers of the same vehicle or `noPhysics`; `dx`, `dz` to the other; `d = absMax(dx, dz)`; nothing below 0.01; `d = sqrt(d)`, `dx /= d`, `dz /= d`, times `min(1, 1/d)` and 0.05; the pushed player moves away by that, added to its motion. On the ground the friction (0.6 x 0.91 = 0.546 a tick) turns a steady push p into p / (1 - 0.546), about 2.2 p, blocks a tick.
- **Changed** (`sim`):
  - `world.rs`: `PredictionRemoteBody` gains `height` (`voxel::player_hull(ps).1[2]`, so crouching and prone bodies are lower) and `riding` (`MC_RIDING`), filled where the friend's prediction takes the snapshot's other players.
  - `step.rs`:
    - `PushBody { entnum, origin, height, riding }` and `push_bodies(world)`: the alive clients and the prediction's remote bodies, minus noclip spectators. Built only in Minecraft worlds, next to `alive_body_clips`, before the mutable borrow of the moving player.
    - `entity_push(dx, dz)`: a copy of vanilla's `Entity.push` step (the vendored `pushing.rs` one is private and `sim` does not use minecraftoss).
    - `minecraft_push(own, own_height, others, self_entnum) -> [f64; 2]`: in blocks (x = map x / 36, z = -map y / 36, y = map z / 36), each body a 0.6-wide box with its own height; only strictly overlapping boxes count; the impulses of all overlapping players are summed. Riders are skipped.
    - `pushable(ps)`: `pm_type` 0, not noclip, not riding, not on a ladder (`LADDER` stands for `onClimbable`).
    - `push_move(origin, push, seconds, trace)`: the push is applied as a collision-checked move, not as velocity, because MW2 ground friction (stopspeed 100) would eat every vanilla-sized push and idle players would never separate. The step is `push x 20 x 36 x GAIN x dt` map units with `GAIN = 1/(1 - 0.546)` and `dt` the command's time clamped to 0.2 s (as the prediction does); map dx = +x, map dy = -z. The move is traced with the player's hull (`voxel::trace_as`, the same mover as `push_out`), and nothing moves when the trace starts solid. `ps.velocity` is untouched.
    - `run_players_system`: in Minecraft worlds, after `voxel::push_out` and before `pmove`, a pushable player is moved by `minecraft_push` against everyone else through `push_move`.
  - The host steps every player through `run_players_system`, and the friend predicts its own push through the same code (`step_predicted`, with the snapshot's remote bodies), as in vanilla where each client pushes its own player. No new messages, no protocol change. The host's result is authoritative; a small difference between the friend's prediction and the host is corrected like any other prediction error.
  - MW2 maps: `voxel::active()` is false, so no push bodies are built and nothing is pushed.
- **Tests** (`sim` `step::tests`, pure functions, no voxel world):
  - `the_push_matches_vanilla_entity_push`: (0.3, 0) against (0, 0) gives 0.3/sqrt(0.3) x 0.05 = 0.027386 and 0; (0.005, 0.009) gives nothing; d = 4 is scaled by 0.5 and d = 0.25 is not; the push points away in x and z (and a push in +z moves the map origin to -y); two players on both sides cancel out; the own entry is ignored.
  - `overlapping_players_drift_apart_within_a_second`: two players 0.1 blocks apart, both pushed in turn with a free trace: 20 commands of 50 ms and 62 of 16 ms both end more than 0.6 blocks apart, both stop overlapping within 0.4 s (about 0.25 s) and within 50 ms of each other; a steady push moves the same distance per second at 50 ms and 16 ms within 5% (4% from 62 x 16 = 992 ms), 0.02 x 20 x GAIN blocks in one second; a 2 s command moves only 0.2 s worth.
  - `the_push_respects_height_spectators_and_riders`: a player standing on the other's head (feet at its top) does not push, one 0.1 blocks lower does; a crouched player (1.5 blocks) is not pushed by one at 1.6; one below does not push; a rider does not push; 0.6 blocks apart is no overlap; `push_bodies` leaves out the noclip spectator, marks the rider and gives the crouched player a lower height from `player_hull`; `pushable` is false for noclip, riding, ladder and `pm_type` other than 0.
  - `a_push_stops_at_a_wall`: a player pushed toward a wall stops at it; a trace that starts solid moves nothing; no push, no move.
  - Run: `test -p sim --locked` (123 passed), `check -p launcher --locked`.
- **Owner's test steps** (two windows: `& scripts\mp-local.ps1 -World r9-a7 -Fresh`, kit of commit 1 in both; host `/op Friend`):
  1. Stand next to each other and walk into the other: you slide into them a little and get nudged sideways, as in vanilla; no wall, no stop. Stand still half inside each other: you slide apart within about a quarter of a second. **Screenshot 1** while overlapping, from both windows.
  2. The friend: `/tp <host name>`. A teleport lands on exactly the same point, where vanilla does not push either (the push needs at least 0.01 blocks between the centres). As soon as either player moves a step, both slide apart. The host: `/tp <friend name>`: the same.
  3. One player crouches next to the other, then jumps onto the other's head: standing on the head does not push.
  4. Push the other against a wall: they stop at the wall and do not go into it.
  5. One player in `/gamemode spectator` flies through the other: no pushing either way.
  6. One player rides a pig with a saddle (or a horse) next to the other: the rider is not pushed and does not push.
  7. One player climbs a ladder while the other stands at its foot: the climber is not pushed off the ladder.
  8. With pvp on, shoot the other with an MW2 gun: hits still register.

### Commit 3: respawns and new friends spread over the respawn radius

Commit: "Minecraft respawns and new friends spread over the respawn radius like vanilla", the one after `43bd7026` (A7 commit 2).

- **Cause** (as in the brief, re-checked at `43bd7026`): `TerrainStream.player_spawn` is drawn once when the level opens and was reused for every bedless respawn (`respawn::level_spawn`), for an arrival with no place (`minecraft_players.rs`) and for every new friend (`Server::join_friend`), so a new friend appeared inside a host standing at spawn and two bedless respawns appeared inside each other. The per-call port of vanilla's `PlayerSpawnFinder.findSpawn` (`TerrainStream::respawn_position`) was never called.
- **Vanilla 26.3** (checked with javap): `PlayerSpawnFinder.findSpawn` uses `fixupSpawnHeight` only when the world's game type is ADVENTURE; otherwise the radius is `max(0, respawn_radius)` (clamped to the world border), up to 1024 candidates in a coprime walk from a fresh random offset per call. There is no skylight check in 26.3. A saved position and `/tp` are exact.
- **Changed:**
  - `minecraft_terrain` `terrain.rs`: `respawn_position(radius)` (still unseeded, now with the radius) and `respawn_position_at(radius, offset)` for tests; a negative radius counts as 0.
  - `minecraft_survival` `respawn.rs`: `level_spawn(globals, &mut Level)`. In the spawn dimension, on the host's stream, and unless the world's default game mode is adventure, it returns `stream.respawn_position(respawn_radius)` with the spawn angle; otherwise `player_spawn` as before (the Nether, and the End's exact platform). `respawn_in` takes `&mut Level`.
  - Followed through: `the_end::travel` and `render_anim` `minecraft_end::travel` take `&mut Level` (the caller in `minecraft_portal.rs` already had one); `respawn_block.rs` already passed a mutable level; `minecraft_players.rs` (an arrival with no place) calls `level_spawn(globals, level)`.
  - `Server::join_friend`: a new friend's `Joining.spawn` is `level_spawn` on the Overworld, so it lands within the radius. A friend with a saved place still arrives exactly where they left (`join_from` replaces the spawn with the saved place).
  - Unchanged: the host's first spawn and `stream.player_spawn` (the Overworld's MW2 map origin), `/tp`, MW2's `spawn_clear_of`, bed and anchor respawns.
  - Every spawn is still worked out on the host, per player; no protocol change.
- **Tests** (all data-gated and checked to run on the 26.3 data):
  - `minecraft_terrain` `terrain::tests::respawn_positions_spread_over_the_radius`: a stream with a known spawn (40, 90, -24): five offsets give positions within 10 blocks of the spawn, each with two air blocks to stand in over a solid, dry block, on at least three different columns; radius 0 gives the spawn column for any offset; `respawn_position(10)` stays within the radius.
  - `minecraft_survival` `respawn::tests::bedless_respawns_land_within_the_respawn_radius` (four bedless respawns in the Overworld, each within 10 blocks, with the spawn angle and no switch) and `respawn_radius_zero_respawns_on_the_spawn_column` (`/gamerule respawn_radius 0`: two players both on the spawn column).
  - `minecraft_survival` `server::tests::a_new_friend_arrives_within_the_radius_and_a_returning_one_where_they_left` (a new friend within 10 blocks; after leaving at a place 30 blocks away and rejoining, exactly at that place; with radius 0 a new friend on the spawn column).
  - The `server.rs` respawn test that asserted the exact `player_spawn` now asserts the dimension, the spawn yaw and that the arrival is within the radius.
  - Run: `test -p minecraft_terrain --locked --lib` (1139 passed, 3 ignored), `test -p minecraft_survival --locked` (604 passed), `test -p render_anim --locked` (159 passed), `check -p launcher --locked`.
- **Risk, not measured:** `spawn::player_spawn` loads candidate chunks with `ChunkMap::load_now`, which blocks while it loads or generates. Near the spawn the chunks are normally loaded (and they were generated when the world was made), so it costs nothing. If a respawn or a friend's join hitches while every player is far from the spawn, the next step is to try only loaded chunks and fall back to `player_spawn`. Note in the owner's steps.
- **Owner's test steps** (two windows: `& scripts\mp-local.ps1 -World r9-a7 -Fresh`, kit of commit 1 in both; host `/op Friend`, `/gamerule keep_inventory true`):
  1. The host stands still at the spawn (note the F3 position). A new friend joins (a fresh name, or delete their `players/data/<uuid>.dat`): they appear within 10 blocks of the spawn, usually not inside the host. If they do overlap, both can walk and they slide apart. **Screenshot 1** from the host window.
  2. The friend walks 30 blocks away, quits and rejoins: they come back exactly where they left.
  3. Without beds, both `/kill` and respawn, 3 times: each lands on a different column within 10 blocks of the spawn, usually apart. Write down the F3 positions. **Screenshot 2.**
  4. `/gamerule respawn_radius 0`, both `/kill`: both land on the spawn column, one inside the other; neither is frozen, and a step by either slides them apart (two players on exactly the same point are not pushed, as in vanilla). Set it back with `/gamerule respawn_radius 10`.
  5. Two beds side by side, each player sleeps in (or right-clicks) their own bed to set the spawn, both `/kill`: each respawns at their own bed, exactly as before.
  6. Through a Nether portal together and back: nobody is frozen at either end and they separate.
  7. The host walks 300 blocks from the spawn, the friend too; both `/kill`: note it if the respawn takes noticeably longer than at the spawn (the spawn chunks may have to load).

## Review

The track's adversarial review read every commit from `24dfdf71` to `877973ed` against the briefs, vanilla 26.3 (javap: `CustomHeadLayer`, `SkullBlockRenderer.submitSkull`, `PlayerSpawnFinder.findSpawn`, `ServerGamePacketListenerImpl.handleSetCreativeModeSlot`) and the rules (no comments, new files rustfmt-clean, MW2 maps gated on `voxel::active()`, one log entry and the trailer per commit).

### Review fix: a player in bed is not pushed

Commit: "A Minecraft player lying in bed is held like a rider and not pushed, pinned by a test", the one after `877973ed` (A7 commit 3).

- **Defect:** the A7 push decision and its state doc wording said "a sleeping player can be pushed". That is wrong: a player lying in bed is held in place with `SimWorld::ride` (`minecraft_world.rs` for the host, `minecraft_players::lay_friends` for friends), which sets `MC_RIDING`, so `pushable` is false and `push_bodies` marks them as a rider. They are not pushed, as in vanilla. The difference from vanilla is the other way round: a player in bed does not push others, where vanilla's sleeper still does with its 0.2-block box.
- **Changed:** the A7 push decision and the `MINECRAFT-SURVIVAL.md:143` wording under "State doc changes". No game code changed.
- **Test added:** `sim` `step::tests::a_player_lying_in_bed_is_held_like_a_rider_and_not_pushed`: a player put in bed with `ride` is not pushable, is a rider in `push_bodies` and does not push the player next to them; after `ride(None)` both push again.
- **Owner's test step** (two windows, kit of A7 commit 1; host `/time set night`): both place a bed side by side, the host lies down and the friend walks into the host's bed: the host stays in bed, the friend is not pushed off and passes through. **Screenshot** from the host window.

## Final verification

### Clippy fix: the scene entity cell report

Commit: "The Minecraft scene entity cell report is one let chain, clean under clippy", the one after `624250b5` (review fix).

- **Changed:** `render_frontend` `adapters/anim/dyn_ent_brush.rs`: the A1 one-time report of a scene entity without a cell is one `if` with a let chain instead of an `if` inside an `if` (`clippy::collapsible_if`). Same behaviour.
- **Clippy:** `clippy -p frame -p hud -p minecraft_net -p minecraft_survival -p minecraft_terrain -p render_anim -p render_frontend -p sim --locked --all-targets -- -A clippy::approx_constant`: this was the only warning on a line the track added; the older warnings stay.
- **Tests:** none added (no behaviour change).
- **Owner's test steps:** as A1; nothing new to see.
- **Kit:** none.
- **Combined kit:** `minecraft:golden_apple*16` and `minecraft:red_bed` make room for `minecraft:bow` and `minecraft:arrow*64`, which the A3 dragon steps need to shoot the crystals; each window has its own white bed for the A7 bed step.

### Test run

Commit: "Track A log records the final test run", the one after `2a8707a7` (clippy fix). Log only.

- **Tests:** on the code of `2a8707a7`, `test -p <crate> --locked` for every touched crate: `frame` 24, `hud` 152, `minecraft_net` 15, `minecraft_survival` 604 + 1, `minecraft_terrain` 1139 (3 ignored; the known flaky `sections::tests::background_full_update_matches_the_synchronous_one` failed once under load and passed on the re-run), `render_anim` 159, `render_frontend` 4, `sim` 124, all passed. `check -p launcher --locked` clean.
- **Owner's test steps:** none; the steps of each commit above stand, with the combined kit below.

## Second review fixes

### A rider's worn head stops its walk swing

Commit: "A rider's worn dragon head and piglin head stop their walk swing like vanilla", the one after `7a13e4c7` (test run).

- **Defect:** `BodyGlide::advance` stopped the walk swing only for a dead player. While riding, `SimWorld::ride` and `movement_iw4::ride::seat` zero the velocity, so the swing's speed eased to 0 but its position stayed where it was: a player who got into a boat or minecart mid-stride kept the dragon jaw open and the piglin ears tilted until they got out.
- **Vanilla 26.3** (javap): `LivingEntity.calculateEntityAnimation` calls `walkAnimation.stop()` while `isPassenger()`, and `LivingEntityRenderer.extractRenderState` sets `walkAnimationPos` to 0 for a passenger. `wornHeadAnimationPos` is then the vehicle's walk position when the vehicle is a `LivingEntity`, else 0.
- **Changed** (`render_anim` `anim/minecraft_glide.rs`): `GlideInput.riding` (`MC_RIDING`); `advance` calls `walk.stop()` when dead or riding. A boat or minecart rider's jaw and ears go to rest, as in vanilla.
- **Left out:** on a living mount (horse, pig, strider, camel) vanilla moves the jaw and ears with the mount's gait. The render side does not know which entity a player rides, so a rider on a living mount shows the rest pose too (see Decisions, A4 walk swing).
- **Test:** `render_anim` `minecraft_glide::tests::a_rider_stops_the_walk_swing_like_vanilla` (a walking body with a non-zero swing gets `MC_RIDING`: speed and position go to 0 on the next tick and stay there; `glide_input` reads `riding` from `MC_RIDING`). Run: `test -p render_anim --locked minecraft_glide` (14 passed), `check -p launcher --locked`.
- **Owner's test step** (kit of A4 commit 1, plus `/give @s minecraft:oak_boat` and `/give @s minecraft:saddle`, `/summon minecraft:pig ~2 ~ ~`): wear the dragon head, F5, walk into a boat on land or water mid-stride and sit down: the jaw closes to its rest (a small gap) and stays there while you sit, rowing or not. Get out and walk: it moves again. Saddle the pig and ride it: the jaw stays at rest too (vanilla would move it with the pig's gait, not done). **Screenshot** in the boat.

## Decisions

- **A1:** in a Minecraft world the stand-in map's DPVS no longer decides which MW2 scene entities are drawn; the frustum, sphere and LOD culling and the dimension band gate do, like vanilla's frustum and distance check. The cell walk is skipped rather than patched (for example by treating leaf 0 as cell 0), because the stand-in map's cells say nothing about a Minecraft world.
- **A2:** the hand anchor is the posed wrist plus a fixed step along the forearm, never the MW2 weapon tags, because the tags are only at the hands while the playing MW2 animation keys them (it depends on the last raised gun's class). The item keeps the chest's orientation, not the forearm's, as in round 7 ("as a vanilla player holds it with the arm down"). `GEAR_FIT.fist = 3.5` is a first guess (vanilla's fist centre is about 4 map units past the wrist at the 0.9375 item scale; MW2's hand is a little smaller) and is tuned from the owner's screenshots.
- **A5:** keep vanilla's rule. A respawn starts when each of the four side cells (one block above the rim, three out from the portal centre) is touched by a crystal's 2x2x2 box, so any of the three straight rim blocks per side counts and only the four diagonal corner blocks fail. Accepting only the exact middle blocks would be one line in `Crystals::in_cell` (compare `crystal.block_position() == cell`), but it is not vanilla, so it is not done. If the owner's step 2 ever shows four corner crystals starting a respawn, log the exit portal location, the four cells and each crystal's block in `try_respawn` and check `fight.exit_portal` against the visible podium (a later fix).

- **A4:** worn heads follow vanilla's `CustomHeadLayer`: the seven skull items are drawn as skull models at 1.1875 times the head with no glint; every other head-slot item that is not a helmet (the carved pumpkin, or any item put there by command) is drawn as its item model with the `head` display transform and glint. A non-head armour piece in the head slot (only possible by command) is drawn as an item, as vanilla's `shouldRender(stack, HEAD)` check does. A player head is drawn with the default skin (the `profile` component is not read), as before for placed and held player heads.
- **A4 walk swing:** the swing is computed on each window from the presented player's velocity (blocks per tick, horizontal part), not from the position change between snapshots, because `BodyGlide` already reads the presented `PlayerState` every frame for the elytra. A rider's swing stops, as vanilla stops a passenger's walk animation, so in a boat or a minecart the jaw and ears rest like vanilla's. On a living mount vanilla shows the vehicle's swing instead; the fork's ridden mobs would need the vehicle's swing (or its entity id) in the presented state, so a rider on a horse, pig or strider shows the rest pose.
- **A4 camera overlay:** the pumpkin blur is the only camera overlay, read from the item id (`minecraft:carved_pumpkin` in slot 39), not from the `equippable` component, because 26.3 has no other item with a `camera_overlay` and the fork does not read `equippable` for the client view. It is drawn in the spyglass HUD pass, so it sits where the scope does and goes with the HUD.

- **A3 dragon:** every window draws the dragon from its own client copy (vanilla's tracker and 3-step interpolation on the client's ticks), while the hit boxes, `end_on_ray` and the entity lookups stay on the host's newest view. So the drawn dragon trails the host's hit boxes by about 1.5 to 3 ticks, the same rule as every other mob in the fork. That is not vanilla, where the client's parts follow the client's body; it is kept because hits are judged on the host. `DragonView.history` is still sent (the drawn dragon uses it only until its first client tick), so there is no protocol bump. The flap sound and the growls follow the client ticks, as in vanilla's client `aiStep`, and no longer the received snapshots.
- **A3 fireballs:** the drawn fireball is moved by vanilla's own motion step (acceleration 0.1 along its velocity, inertia 0.95) from the host's last velocity, not by the last velocity alone as the brief said, because the dragon's fireballs accelerate from 0.1 to almost 1.9 blocks a tick and a constant velocity would fall behind. It snaps to the host only when the host is more than a block off its track within two ticks of travel, not when it is more than a block from the drawn point: a fast fireball is up to 1.9 blocks from the host's point just from the frame timing, and snapping on that would bring the stutter back. When the host's fireball hits something, the drawn one can fly on for a tick or two until the next frame removes it.

- **A6 wire:** the creative target has a `Carried` variant, which vanilla does not have (vanilla keeps the creative cursor on the client). The fork keeps every cursor on the server (`inventory.cursor`), so the friend sets its creative cursor on the host like any other slot. A refused or undecodable set changes nothing and is not answered, as in vanilla (which only logs); the friend's screen shows the host's state on the next change. **Protocol: 4** in this track; the second of tracks A and B to merge takes 5.
- **A6 friend screen:** the friend's creative screen does not predict, like every friend menu: the cursor and the slots show the host's state one round trip later, and a second creative click before the host answers is worked out from the old mirror (within one frame, clicks chain on a scratch copy). Vanilla keeps the creative cursor on the client, so it has no such delay. The Operator Utilities tab needs command level 2 and the friend's own `minecraft_operator_items` setting, like vanilla's `canUseGameMasterBlocks` plus the operator-items option; the host checks no op level for a creative set (vanilla does not either). Saved hotbars stay in each player's own `hotbar.nbt`, as in vanilla, and a load is sent as nine creative sets.
- **A6 drop throttle:** vanilla's `TickThrottler(20, 1480)` per player, on the host and the friend path alike (vanilla's integrated server throttles the host too). It is ticked with the player's hand ticks in `movement`, which runs every frame for every player, so a player whose `movement` stops (none today) would keep its count; vanilla ticks it with the connection. It is not saved, as in vanilla.
- **A7 no body collision:** in a Minecraft world players are never solid to each other's movement, as in vanilla, instead of MW2 capsules plus MW2's StuckInClient eject. Hits (`CONTENTS_BODY` links) are unchanged. MW2 maps keep MW2 player collision and StuckInClient.
- **A7 push:** the push uses vanilla's `Entity.push` impulse, but it is applied as a move traced against the blocks with `GAIN = 1/(1 - 0.546)` (the distance vanilla's ground friction turns a steady push into), not added to the velocity, because MW2's friction would eat it. That is not vanilla: the push does not carry momentum, is the same in the air and on ice as on the ground, and is scaled by each command's time. Riders neither push nor are pushed (vanilla moves a rider with its vehicle, and the fork's mounts are mobs that do not push players; a vanilla rider's own box would still push a walker it reaches). A player lying in bed is held in place with `MC_RIDING` (`SimWorld::ride`), so like a rider they are not pushed, as vanilla's `LivingEntity.push` does nothing while sleeping; they do not push others either, where vanilla's sleeper still pushes with its 0.2-block sleeping box. Not done, a vanilla behaviour missing: there is no cramming damage between players (`max_entity_cramming`), only between mobs. Team collision rules are not read (there are no teams).
- **A7 spawns:** bedless respawns, arrivals with no place and new friends use vanilla's `findSpawn` over `respawn_radius` (default 10) in the spawn dimension, from a fresh unseeded offset each time; ADVENTURE (the world's default game mode, as vanilla's `getWorldData().getGameType()`) keeps the fixed spawn. The world border clamp of the radius is left out (the fork has no world border). The host's own first spawn keeps `player_spawn`, because that point is also the Overworld's MW2 map origin; a vanilla host's first spawn would be spread too. A friend's game never computes a spawn (`is_remote`), so the host decides all of them.

## State doc changes

Wording for the state docs, to be applied when the round's state is written (this track does not edit them):

- **`docs/MINECRAFT-SURVIVAL.md`, multiplayer and rendering:** "MW2 bodies (other players, corpses, grenades, dropped guns) are drawn in every dimension and at every depth. In Minecraft worlds the stand-in map's (`mp_rust`) cells no longer cull MW2 scene entities: every scene entity is linked into every cell, and only the frustum, sphere and LOD culling and the dimension band gate decide. MW2 maps are unchanged. MW2 FX and marks were not checked for the same culling (`adapters/fx/world_mark.rs`)."
- **`docs/MINECRAFT-SURVIVAL.md`, held items on the MW2 body:** "Minecraft items in the main hand and offhand sit in the MW2 body's right and left fist: the posed wrist plus 3.5 map units along the forearm (`GEAR_FIT.fist`), upright with the chest. The MW2 weapon tags are no longer used for Minecraft items, so the item no longer depends on which gun's animations are playing, and the two hands never overlap. The same rig serves other players' bodies, the own F5 body and the inventory character, for the host and every friend."
- **`docs/MINECRAFT-SURVIVAL.md`, the dragon respawn:** "Four end crystals respawn the dragon like vanilla 26.3: one must touch each of the four cells one block above the exit portal's rim, three blocks out on each side. A crystal's box is two blocks wide, so a crystal on any of the three straight rim blocks of a side counts, and one on a diagonal corner block does not. A crystal on a corner block also blocks placing one on the rim block beside it."
- **`docs/MINECRAFT-SURVIVAL.md:406`, the limitation:** drop "heads and carved pumpkins on the head". Add to the MW2 body section: "Mob heads (skeleton, wither skeleton, zombie, creeper, dragon, piglin, player) and the carved pumpkin worn in the helmet slot are drawn on the MW2 body like vanilla's `CustomHeadLayer`: skulls at 1.1875 times the head, other items with their `head` display transform, for other players, the own F5 body and the inventory character, for the host and every friend. A player head always shows the default skin."
- **`docs/MINECRAFT-SURVIVAL.md`, overlays:** "A worn carved pumpkin blurs the first-person view with `misc/pumpkinblur` (linear filtering) like vanilla's camera overlay, for the host and every friend; not in F5 and not while scoping with the spyglass." Also, for `docs/MULTIPLAYER.md` section 20 (friend limits, track D): "A friend's window does not load the powder-snow outline (`ui.freeze_outline`)."
- **`docs/MULTIPLAYER.md` 9.4, the dragon:** "Every game, the host's and each friend's, draws the ender dragon from its own client copy (`minecraft_terrain::client_dragon`): the host's `DragonView`s go through vanilla's tracker and 3-step interpolation on that game's own client ticks, and the flap, the flight history, `tick_count` and `death_time` run on the client (the clocks resync to the host when more than 2 ticks off). The drawn dragon no longer jumps back and forth when state frames arrive off the client tick. The hit boxes (the 8 parts), `end_on_ray` and the lookups stay the host's newest view, so the drawn body trails them by about 1.5 to 3 ticks like other mobs. The flap sound and the growls are played on the client ticks. The dragon's fireballs are drawn the same way from client copies (`ClientFireballs`) that move with vanilla's fireball motion each client tick and snap to the host's position only when it is more than a block off their track."
- **`docs/MINECRAFT-SURVIVAL.md`, the End paragraph:** "The dragon is drawn from a client copy that glides on the client's own ticks (vanilla's 3-step interpolation), with its own flight history, wing beat and death clock, for the host and every friend; the hit boxes stay the server's. The dragon's fireballs are drawn from client copies that move with vanilla's fireball motion every client tick and snap to the host only when they leave its track."

- **`docs/MULTIPLAYER.md:660`, creative inventory (10.2):** "**Creative inventory:** a player whose `abilities.instabuild` is true gets the creative screen, the host and every friend alike: the tabs from the jar, search by name and `#tag` (the friend reads the item tags from its own data pack), the inventory tab with the trash, middle-click clone and the C/X saved hotbars in each player's own `hotbar.nbt`. A friend's game applies nothing itself: every creative change goes to the host as `ToServer::CreativeSlot { slot: Menu(0..=45) | Carried | Drop, stack }` (vanilla's `ServerboundSetCreativeModeSlotPacket` plus a carried target, because the fork keeps the cursor on the host), and the host checks it with `creative::set_slot` on that friend's `Player`, in order with the friend's clicks: not creative, a bad slot, an unknown item or a stack over its maximum is ignored, as is any set while dead; a drop is thrown from the friend's eye. The friend's creative screen does not predict either. The Operator Utilities tab needs command level 2 and the player's `minecraft_operator_items` setting. Protocol 4 (A6)."
- **`docs/MULTIPLAYER.md:1330`, section 20:** drop the line "Creative inventory for friends (`ServerboundSetCreativeModeSlotPacket`)."
- **`docs/MINECRAFT-SURVIVAL.md:368`, a friend's limits:** replace "the creative inventory (a friend in creative mode gets the survival screen), and command suggestions" with "command suggestions"; the creative inventory now works for friends.

- **`docs/MULTIPLAYER.md` 10.2, creative inventory (add to the A6 wording):** "Creative drops are throttled like vanilla (`TickThrottler(20, 1480)` per player, host and friends): 74 in a burst, then one a second."

- **`docs/MINECRAFT-SURVIVAL.md:143`, after "Every MW2 spawn and respawn lands on the Minecraft spawn, ...":** add "In Minecraft worlds players are not solid to each other's movement, as in vanilla: pmove, the script slide and mantling ignore other players, and MW2's StuckInClient eject does not run, so two players who overlap after a spawn or a teleport are never frozen. Bullets, melee and lag compensation still hit players. MW2 maps keep MW2 player collision." Then: "Overlapping players push each other apart with vanilla's `Entity.push` step (0.6-wide boxes with each player's own height; spectators, riders, players in bed and players on a ladder are not pushed), applied as a move traced against the blocks at about 2.2 times the per-tick impulse, the distance vanilla's ground friction turns a steady push into. Each player's own game pushes them (the host for the host and every friend, the friend's prediction for the friend). Not vanilla: the push carries no momentum; a player in bed does not push others; players deal each other no cramming damage."
- **`docs/MINECRAFT-SURVIVAL.md:143`, the spawn sentence:** replace "Every MW2 spawn and respawn lands on the Minecraft spawn, or on the nearest free dry column within 10 blocks when someone stands there." with "The host's first spawn lands on the Minecraft spawn. Bedless respawns, arrivals with no place (such as leaving the End without a bed) and new friends land on a random spot within the `respawn_radius` game rule (10 by default) of the world spawn, found like vanilla's `PlayerSpawnFinder.findSpawn`; adventure worlds keep the exact spawn, and `respawn_radius 0` gives the spawn column. A friend who has played before arrives exactly where they left, and `/tp` is exact."
- **`docs/MINECRAFT-SURVIVAL.md:390`, the switch limitation:** drop "and respawns are not spread at random over vanilla's radius of 10", ending the sentence after "(the player only waits for their chunk's collision, so terrain may still fade in)".

## Combined kit

From the brief's shared points, with a bow and arrows for the A3 dragon steps (27 stacks; `/give` anything else a step's own kit lists, such as `minecraft:oak_log` or `minecraft:oak_planks`):

```
IW4L_MINECRAFT_KIT=minecraft:obsidian*14,minecraft:flint_and_steel,minecraft:diamond_sword+sharpness=5,minecraft:torch*64,minecraft:cobblestone*64,minecraft:totem_of_undying,minecraft:enchanted_book,minecraft:spyglass,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:cooked_beef*32,minecraft:end_crystal*16,minecraft:elytra,minecraft:firework_rocket*64,minecraft:dragon_head,minecraft:piglin_head,minecraft:skeleton_skull,minecraft:wither_skeleton_skull,minecraft:zombie_head,minecraft:creeper_head,minecraft:player_head,minecraft:carved_pumpkin,minecraft:white_bed,minecraft:bow,minecraft:arrow*64
```
