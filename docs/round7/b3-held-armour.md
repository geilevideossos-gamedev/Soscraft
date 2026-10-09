# B3: held items and worn armour on the third-person MW2 body

Owner, screenshot 2 (`2_tod_auf_hoehe_survival_items_am_kopf.png`): "In third person and the death cam, the held block and the offhand item seem to float at the head instead of sitting in the hands." Also: "Worn armour isn't shown on the MW2 body. Show it as Minecraft armour pieces on the MW2 skeleton, with glint when enchanted."

## 1. What exists today

- **No Minecraft item is ever attached to the MW2 body.**
  - The only Minecraft held-item mesh is the first-person one. It is built in view space (`minecraft_world.rs:1855-1894`) and drawn with the fixed hand projection (`render_gpu/.../minecraft_world.rs:712-718`, `hand_vertex` :1037).
  - That mesh is skipped while the F5 camera is detached or the inventory puppet is shown (`:1861 !puppet.active && !mc_detached`). It is also skipped when dead, because `holding_item` needs `alive` (`:765`).
  - The body draws only the MW2 kit: body, head on `j_spine4`, and the last MW2 gun on `tp_weapon_attach_tag` (`xmodel_runtime/src/dobj.rs:758-773`, `anim/remote_body.rs:799-950`). That gun is `remote_pose_sample(runtime).weapon` (`occupancy/remote_body.rs:242`, `:791`), which equals `ps.weapon`. `docs/MINECRAFT-SURVIVAL.md:194` says the same.
- **What floats at the head in screenshot 2 is the death drops.**
  - `death_drops` → `world_items.drop_on_death(stack, death.spot.eye)` (`minecraft_world.rs:1814-1819`) spawns each stack at eye − 0.3, moving up 0.2 and up to 0.5 sideways per tick (`third_party/minecraftoss/player/src/items.rs:254-270`), as vanilla `Player.drop(stack, true, false)` does. The cobblestone and the hook-shaped item at head height are drops in flight. The other drops are already scattered on the ground.
  - Client drops are drawn as frozen copies at their spawn point until the server acknowledges them (`Entities::server_items_tick`, `minecraft_entities.rs:356-375, 440-444`). A slow server tick, for example right after a death, therefore makes them hang at eye height for longer.
- **The second standing body is not the local player.**
  - Bodies are gated by `remote_body_submit_kind` (`net/src/client/presentation/centity_runtime.rs:140-169`), which returns `Dead` for `EFLAGS_DEAD`.
  - That flag is set from `pm_type >= PM_TYPE_DEAD` in `player_state_to_entity_state` (`:266-268`) and by `clonePlayer` (`sim/src/script_player.rs:388-396`).
  - The standing soldier wears a different faction kit from the corpse. The kit depends only on the client's team (`select_remote_models` → `kits.kit(axis)`, `anim/remote_body.rs:807-814`; `kit_assignment_is_axis`, `asset_model/src/soldiers.rs:30`). A corpse uses its victim's meta (`occupant_from_corpse`, `net/.../entities.rs:246-251`). One client therefore can't show two kits, so the standing body is another FFA client, almost certainly an MW2 bot.
  - To confirm, see test step 5.
- **Bones available.**
  - The bones in use in the code are `j_mainroot`, `j_spinelower`, `j_spineupper`, `j_spine4`, `j_neck` and `j_head` (`skate/rig.rs:22-48`). So are, on each side (`_le`/`_ri`), `j_clavicle`, `j_shoulder`, `j_elbow`, `j_wrist`, `j_hip`, `j_knee`, `j_ankle` and `j_ball`.
  - The tags in use are `tag_weapon_right`, `tag_inhand`, `tag_weapon_left` and `tag_shield_back` (`sim/src/shield.rs:8-12`).
  - `PreparedRemoteKit` already resolves bones by name with `dobj.find` (`bolt_bones`, `head_bone`, `anim/remote_body.rs:539-550`).
  - Posed world bones are published per entnum in `HostDObjPoseFrame::publish` (`anim/dobj_pose.rs:63`) from `publish_remote_dobj`. Next to that, `publish_posed_player_head` fills `PosedPlayerFrame` (`occupancy/remote_body.rs:884-902`). Both run for live bodies, the F5 body (posed at the predicted origin, `:413-422`) and the inventory puppet.
- **Minecraft side.**
  - `armor_render::append_humanoid_armor` (`minecraft_terrain/src/armor_render.rs:63-104`) draws mob armour with `cow_render::cube_scaled` (`:518`). It has the right deformations, slot parts and leather overlay, but a constant undyed colour (`:57`) and no glint.
  - `append_held_items` (`mesh.rs:1104-1121`) has only `HeldDisplay::RightHand/Ground/Fixed` and no glint (`:1209-1228`). The left-hand mirror exists only for first person (`model.rs:196-226`).
  - Glint is a per-vertex flag: `ChunkMesh.glint` → `GLINT_VERTEX = 255` (`mesh.rs:87-100`), and `glint_light` samples the item glint (`render_gpu/.../minecraft_world.rs:843-853`). Items reach the GPU through `view.particles` (`minecraft_world.rs:1928-1933`), while `entity_meshes` carry no glint flag.
- Map units to blocks: `BLOCK = 36` and `to_block` maps (x, y, z) to (x, z, −y)/36 (`sim/src/voxel.rs:15, 613-630`).

## 2. Vanilla 26.3 (javap, client.jar)

- **`ItemInHandLayer.submitArmWithItem`:**
  1. `translateToHand(arm)`.
  2. `rotX(-90)`, then `rotY(180)`.
  3. `translate((left ? -1 : 1)/16, 2/16, -10/16)` for adults.
  4. A stab swing (`SpearAnimations`), then `ArmPose.animateUseItem` while the item is in use.
  5. The item with `THIRD_PERSON_LEFT/RIGHT_HAND`.
- In the hanging-arm rest pose, the resulting frame has x to the entity's right, y forward and z up, at the hand.
- **`PlayerItemInHandLayer`:** a spyglass in use (`heldOnHead`) is drawn at the head with `CustomHeadLayer.translateToHead` and `translate(±2.5/16, 0, -1/16)`. That is the one vanilla case of an item at the head.
- **`HumanoidArmorLayer.submit`:**
  - Draw order is CHEST, LEGS, FEET, HEAD.
  - `shouldRender` needs `Equippable.assetId` and a matching slot.
  - LEGS use `usesInnerModel`, which means the `HUMANOID_LEGGINGS` layer.
- **`EquipmentLayerRenderer.renderLayers`:**
  - Each layer's colour comes from `getColorForLayer(layer, DyedItemColor.getOrDefault(stack, 0))`. `equipment/leather.json` has a dyeable layer with `color_when_undyed -6265536`, then `leather_overlay`.
  - `stack.hasFoil()` selects `RenderTypes.armorCutoutNoCullGlint`, which uses `ENCHANTED_GLINT_ARMOR` (`misc/enchanted_glint_armor.png`, 128², a different file from the item glint) and `ARMOR_ENTITY_GLINT_TEXTURING`, scale 0.16. Items use `GLINT_TEXTURING` (8.0) and entities `ENTITY_GLINT_TEXTURING` (0.5), per the `TextureTransform` lambdas.
  - Trims come after, with `RenderTypes.armorTrim`.
- **Equipment sync:**
  - `LivingEntity.detectEquipmentUpdates` → `collectEquipmentChanges` runs every tick.
  - Changes go out as `ClientboundSetEquipmentPacket(entity, [(EquipmentSlot, ItemStack)])`.

## 3. Commits

1. **"Minecraft players' worn and held items are published per player"**
   - A pure `body_gear(inventory, selected) -> BodyGear` in `minecraft_survival.rs`, next to `armor()` (`:863`), which moves to `crates/minecraft_survival` in the split. `BodyGear` holds:
     - `main` and `off`, each an `Option<GearItem { id, glint, dye }>`.
     - `armor: [Option<GearItem>; 4]`, head to feet.
     - `hides_gun`.
   - The slots are 39/38/37/36, `OFFHAND` 40 and `selected`. MW2 weapon items (`weapon_of`) are skipped.
   - `frame::McBodyGear(HashMap<u32 client, BodyGear>)` is written in `minecraft_world::update` for the local client while it is alive and is cleared off the map. The multiplayer track later fills it from `minecraft_net` equipment messages, keyed by UUID → client on the host.
2. **"MW2 bodies publish their limb and hand bones for Minecraft gear"**
   - `GearBones` holds the indices of the bones listed in section 1 (head, spine, pelvis, the arm and leg joints, `tag_weapon_right/left`). It is resolved in `PreparedRemoteKits::prepare` (`anim/remote_body.rs:533-552`), and the existing "remote kits: prepared" log names any missing ones.
   - `PosedGearFrame`, beside `PosedPlayerFrame` (`dobj_pose.rs:18`), holds per entnum `transform × world[i]` in map space. It is filled by one new call next to `publish_posed_player_head` (`occupancy/remote_body.rs:894`).
   - Edits to these upstream files stay a few lines each.
3. **"The MW2 body puts its gun away while a Minecraft item is held"**
   - When `McBodyGear[client].hides_gun`, the kit weapon becomes 0 in both `occupy_remote_scene_ents` (`:242`) and `pose_one_remote` (`:791`).
   - This also updates `MINECRAFT-SURVIVAL.md:194`.
4. **"Minecraft blocks and items sit in the MW2 body's hands in third person like vanilla"**
   - New `render_anim/src/minecraft_gear.rs` with:
     - `hand_frame(bone_map, origin, left) -> Mat4`: map→block basis × bone × `Rz(-90°)` × `scale(BLOCK)`, which turns MW2's tag (X forward, Y left, Z up) into vanilla's hand frame (x right, y forward, z up). A small offset table holds tuning values.
     - A system ordered after `WorkerCmdSet::SkinModel` and after `update`, which needs a `SystemSet` label in `minecraft_world::register`. It reads `NonSend<Runtime>` (packs, atlas, light) and appends to `view.particles`.
   - In `minecraft_terrain`:
     - `HeldDisplay::LeftHand` plus `ItemVisual.left_hand` (`mesh.rs:834-843, 871`).
     - `model::item_third_person_transform(.., left_hand)` that reuses the mirror in `display_transform`.
     - `HeldItem.glint` pushes `mesh.glint` ranges. Mob and campfire callers pass `false`.
5. **"Minecraft armour is worn on the MW2 body like vanilla"**
   - In `armor_render.rs`:
     - `cube_posed_slice(mesh, part: Mat4, from, size, grow, uv, y0..y1, tint, mirror)`, a box-UV cuboid cut to a y-range: sides cropped in V, the top face only at y0 = 0 and the bottom only at y1 = h. It also replaces `cube_scaled`'s fixed frame.
     - `append_armor_on_frames(mesh, atlas, &GearFrames, &[Option<ArmorPiece>; 4], light)`, which keeps the existing slot and part rules and draw order.
     - Leather tint from `dye`, falling back to `UNDYED_LEATHER`.
   - `render_anim` builds `GearFrames` from joint positions:
     - Head: on `j_head`, sized to the MW2 head.
     - Torso: `j_neck`→`j_spinelower` lengthwise, `j_shoulder_le`→`_ri` across.
     - Limbs: each vanilla 12-pixel limb split in two halves. The upper half runs shoulder→elbow and hip→knee, the lower half elbow→wrist and knee→ankle, so the pieces bend with MW2's elbows and knees.
     - One `GEAR_FIT` table holds the girths in map units.
6. **"Enchanted Minecraft armour shimmers on the MW2 body like vanilla"**
   - `ChunkMesh.armor_glint` plus `ARMOR_GLINT_VERTEX` (e.g. 128) in `section_vertices`.
   - The `enchanted_glint_armor` texture is loaded next to `glint_texture` (`minecraft_world.rs:304`), passed through `render/src/extract.rs:995`, and bound in `render_gpu`.
   - `glint_light` takes the armour texture and the 0.16/64 scale for that flag. Update the uniform-layout test (`render_gpu/...:1379`).
7. **"The inventory's character wears its Minecraft armour and holds its items like vanilla"**
   - The puppet is drawn in the view-model depth band (`DEPTH_HACK_SCENE_ENTNUM`, `minecraft_world.rs:172-227`), so its gear goes into a `view.puppet_gear` buffer drawn with the backdrop's band.

Later, not in this round: armour trims, carved pumpkins and heads (`CustomHeadLayer`), elytra, the spyglass at the eye, vanilla arm swing and use poses on the MW2 rig, and gear on corpses with `keep_inventory`.

## 4. Unit tests

- `body_gear`: slot mapping, MW2 guns skipped, `hides_gun` only for a non-gun selection, glint from `has_glint`, `dyed_color` (undyed → `None`).
- `hand_frame`: an identity tag at map (36, 0, 72) lands at its `to_block` point. Tag +X maps to block forward, tag +Z to block +Y, and the left frame mirrors x.
- `GearFrames` on a synthetic skeleton: the upper slice ends where the lower slice starts (the elbow), slice lengths equal bone distances, and a bent elbow turns only the lower slice.
- `cube_posed_slice`: range 0..h equals the full cube. The two halves have 10 faces between them, and their V ranges meet at the middle.
- `append_armor_on_frames`: a helmet emits only head cubes, leggings use `humanoid_leggings`, leather emits two layers with the dye only on the first, and the order is chest, legs, feet, head.
- `section_vertices` marks armour ranges with `ARMOR_GLINT_VERTEX` and items with `GLINT_VERTEX`. `HeldDisplay::LeftHand` uses `thirdperson_lefthand` with the mirror.
- `remote_body_submit_kind` returns `Dead` for `player_state_to_entity_state` with `PM_TYPE_DEAD`, which pins the death-cam finding. `hides_gun` → kit weapon 0.

## 5. In-game test steps

```
IW4L_MINECRAFT_KIT=diamond_helmet+protection=4,iron_chestplate+protection=4,leather_leggings,iron_boots,leather_helmet,leather_chestplate,golden_helmet,netherite_chestplate,chainmail_leggings,diamond_boots+feather_falling=4,turtle_helmet,copper_chestplate,shield,totem_of_undying,cobblestone*64,torch*64,diamond_sword+sharpness=5,diamond_pickaxe,lever*8,spyglass,red_dye*8,oak_log*16
```

Start `iw4l-survival.exe map minecraft:overworld`.

1. Wear the diamond helmet, iron chestplate, leather leggings and iron boots. Press F5 for the back view: you should see boxy Minecraft armour on the head, torso and arms, hips and legs, and boots. The helmet and chestplate shimmer purple, and the leggings are brown with the overlay. Press F5 again for the front view. Walk, crouch, go prone and look up and down: the pieces bend at the elbows and knees. Screenshots 1 and 2.
2. Select the cobblestone and put the torch in the offhand (F). In F5, the cobblestone is in the right hand, the torch in the left, and there's no MW2 gun. Select the sword: it shimmers. Screenshot 3.
3. Select an MW2 gun slot. The gun is back, the right hand is empty, and the torch stays in the left hand. Screenshot 4.
4. Open the inventory (E). The character in the box wears the same armour and holds the items. Screenshot 5.
5. Death cam: press Tab and note whether a bot is in the match. Hold the cobblestone and type `/kill`. The drops fly out of your eye height and land within about a second. No standing copy of your soldier appears. If a standing soldier appears and Tab shows no other player, say so: that would contradict section 1. Screenshot 6.
6. Swap in the leather helmet, golden helmet, turtle helmet, netherite and copper chestplates, chainmail leggings and diamond boots, and check each texture. If dyeing leather works in the crafting grid (red dye), the dyed colour shows on the body.

## 6. Risks and overlaps

- **Crate split:** `BodyGear` and `body_gear` go to `crates/minecraft_survival`, and equipment messages to `crates/minecraft_net`, modelled on `ClientboundSetEquipmentPacket` and sent on change only. Gear stays server-owned per UUID, and the renderer only reads `McBodyGear`.
- **Upstream files** (`occupancy/remote_body.rs`, `anim/remote_body.rs`, `dobj_pose.rs`): keep each edit to a few lines and put the logic in `minecraft_gear.rs`.
- **Calibration:** MW2 tag and joint axes are inferred from IW conventions, so the first build may show turned or offset items. All constants sit in one table, tuned from screenshots 1-3.
- **Rendering:** commits 6 and 7 touch the shared Minecraft world shader, its bind group and the depth bands. That collides with other glint or world-shader work (state doc line 199).
- **Track D** saves the armour and offhand slots. **Track A** owns F5 (`minecraft_camera.rs`), so test the F5 views together.
- **Not this track:** the frozen-drop delay (`server_items_tick`) only makes drops linger at the head. Long-hanging drops belong to the death-at-height item (Phase 1 item 1).
- **Skating** poses go through `skate::rig::pose`, and the gear follows the published bones.
