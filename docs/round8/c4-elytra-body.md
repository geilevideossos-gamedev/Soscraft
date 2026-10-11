# C4: the elytra and the glide pose on the MW2 body (design brief)

Track C, item c4. Paths and lines are as of `e2313625`. Vanilla facts were checked with javap on the 26.3 `client.jar`. No Mojang code is copied: only behaviour and numbers.

## 1. The owner's request

> "MW2 body: the elytra on the MW2 third-person body (and on other players, multiplayer is coming), and the glide pose (vanilla: the elytra model attached to the body, wings spread while fall-flying; the player model lies along the flight direction)."

Round 7 left both out. The End log says: "the elytra on the MW2 body (no wing model on the MW2 rig yet; the body plays its in-air animation)" (`docs/round7/track-end-log.md:469`, `:549`). The state doc lists it at `docs/MINECRAFT-SURVIVAL.md:284` and `:299`.

## 2. What exists today

| Area | State (file:line) |
|---|---|
| Glide movement | `movement_iw4/src/glide.rs:23-48` (`prepare`: `MC_GLIDING` set on a fresh jump in the air, cleared on ground, in liquids, on climbables, while flying or without `MC_MAY_GLIDE`), `:58-104` (`glide_move`: `ps.velocity` holds the vanilla motion in map units per second, `PER_TICK = 36 / 0.05`, `:9`). Bits: `playerstate_iw4/src/playerstate.rs:182-184` (`MC_GLIDING 0x0200_0000`, `MC_MAY_GLIDE 0x0400_0000`), replicated in `pm_flags` (`net/src/transport/netfields.rs:242`). |
| Glide rules (server-owned, per player) | `minecraft_survival/src/elytra.rs` (`can_glide`, wear, wall damage), `Player.elytra` (`player.rs:39`). The adapter `render_anim/src/minecraft_elytra.rs:57` reads `MC_GLIDING`. |
| What a body wears | `minecraft_terrain/src/body_gear.rs:25-57`: `BodyGear.armor` (head, chest, legs, feet) from slots 39-36. An elytra in slot 38 already lands in `armor[1]` as a `GearItem { id: "minecraft:elytra", glint, dye }`. Filled per MW2 client number from the server-side `Player` by `minecraft_survival/src/gear.rs` in `minecraft_world.rs:2203-2210` (`McBodyGear`, `anim/minecraft_gear.rs:191-237`). Track E later fills other players' entries from equipment messages. |
| Why the elytra is invisible | `armor_render::armor_asset` (`armor_render.rs:22-47`) only knows `*_helmet/_chestplate/_leggings/_boots`, so `armor_layers` (`:206-214`) returns nothing for the elytra, and `append_armor_on_rig` (`:216-268`) draws nothing for it. |
| Armour on the MW2 body | `anim/minecraft_gear.rs`: `GearBones` (`:11-46`, 19 joints incl. `j_spine4`, `j_neck`, `j_head`, hips, knees, ankles), `part_frame` (`:312-327`, maps vanilla model pixels x left, y down, z back onto MW2 axes), `bound_armor` (`:373-432`, the torso frame starts at `shoulders + torso_top` and rides `j_spine4` for its upper slice), `body_rig` (`:456-481`), `gear_rigs`/`gear_meshes` (`:483-528`). Terrain side: `BodyRig` (`body_render.rs:12-17`), `append_body_gear` (`:38-61`), `cube_posed_slice` (`armor_render.rs:146-198`, box-UV cuboid with mirror, 64x32 sheet `:108`). |
| Drawing | `draw_body_gear` (`minecraft_world.rs:2675-2692`) appends the world gear to `view.particles` (opaque item pass, item and armour glint flags) and the inventory puppet's to `view.puppet_gear`. It runs after `WorkerCmdSet::SkinModel`. |
| Entity atlas | `mesh.rs:614-622` adds the humanoid armour textures. `entity/equipment/wings/elytra` is not in the atlas. |
| Body pose | `occupancy/remote_body.rs:768-962` `pose_one_remote`: the MW2 anim tree (legs/torso anims chosen by the sim's anim script; in the air that is the jump/fall anim), then `pose_remote_dobj` (`anim/remote_body.rs:1050-1062`) with the aim controller `remote_player_controller` (`:1064-1080`, view pitch, prone, crouch). Skating replaces the pose with `skate::rig::pose` (`:907-913`). The posed `world` bones feed the skin, the fx bolts, `publish_posed_player_head` and `publish_posed_gear` (`:927-945`). |
| Controller mechanics | `xmodel_runtime/src/dobj.rs:582-605` `apply_player_controller`: control tags `back_low/back_mid/back_up/pelvis` (`:510`) and bone 0 (`tag_origin`, the root at the feet) via `set_local_tag` (`:538-550`). `compose` (`:395-430`): bone 0 is `placement * local`; a `control` bone takes `root * control * root⁻¹ * parent` and drops its own animated rotation (`:415-421`). The controller closure runs before `apply_duplicates` (`retained.rs:425-443`), so the head model's duplicate bones copy any edited body local. |
| Root placement | `sync_remote_bodies` (`occupancy/remote_body.rs:326-460`): yaw only (`Quat::from_rotation_z(yaw)`); the local F5 body is placed at the predicted origin (`:432-438`); the puppet uses `puppet.root` (`:440-442`). |
| Replicated state per body | `PresentedSnapshot::player(id)` (`net/src/client/presentation/presented.rs:257-267`) returns the full interpolated `PlayerState` of every client (predicted for the local one), with `pm_flags`, `e_flags` (`DUCK 0x4`), `velocity`, `viewangles`, `pm_type`. Remote bodies are built from it (`net/src/client/runtime.rs:1786-1812`). |
| Hits | MW2 bullets hit the authority's own DObj pose (`sim/src/bullet_collision.rs:152-184`), driven by the anim state, not by the renderer. |

## 3. Vanilla 26.3 (javap)

**`ElytraModel`** (`net.minecraft.client.model.object.equipment.ElytraModel`):
- `createLayer`: texture 64x32. `left_wing`: `texOffs(22, 0)`, `addBox(-10, 0, 0, 10, 20, 2, CubeDeformation(1.0))`, pose `offsetAndRotation(5, 0, 0, 0.2617994, 0, -0.2617994)`. `right_wing`: `texOffs(22, 0).mirror()`, `addBox(0, 0, 0, 10, 20, 2, CubeDeformation(1.0))`, pose `(-5, 0, 0, 0.2617994, 0, 0.2617994)`.
- `setupAnim(HumanoidRenderState)`: `leftWing.y = isCrouching ? 3 : 0`; `leftWing.xRot = elytraRotX`, `zRot = elytraRotZ`, `yRot = elytraRotY`; `rightWing.yRot = -leftWing.yRot`, `rightWing.y = leftWing.y`, `rightWing.xRot = leftWing.xRot`, `rightWing.zRot = -leftWing.zRot`.
- `ModelPart.translateAndRotate`: translate by `(x, y, z)/16`, then `Quaternionf.rotationZYX(zRot, yRot, xRot)` (Rz·Ry·Rx).

**`WingsLayer.submit`**: draws only when the chest stack's `EQUIPPABLE` has an `assetId`. It translates `(0, 0, 0.125)` (2 pixels toward the back) in the model root frame, not on the body part, then `EquipmentLayerRenderer.renderLayers(LayerType.WINGS, assetId, model, …)`. `equipment/elytra.json`: one `wings` layer, `texture minecraft:elytra`, `use_player_texture true` (the player's elytra or cape skin wins when present; we have no skins, so it is always `textures/entity/equipment/wings/elytra.png`, 64x32). A foil stack uses the armour glint render type, as armour does.

**`ElytraAnimationState.tick`** (run every tick from `LivingEntity.tick`, on the client too):
- Gliding (`isFallFlying`): `k = 1`; if `motion.y < 0`: `k = 1 - (-normalize(motion).y)^1.5`. Targets `x = lerp(k, 0.2617994, 0.34906584)`, `z = lerp(k, -0.2617994, -1.5707964)`, `y = 0`.
- Else crouching: `x = 0.6981317`, `z = -0.7853982`, `y = 0.08726646`.
- Else: `x = 0.2617994`, `z = -0.2617994`, `y = 0`.
- Each of rotX, rotY, rotZ moves `0.3` of the way to its target per tick, starting from 0 (field defaults). `getRotX(partial)` lerps old to new.
- So: wings fold at 15 degrees standing, spread to 90 degrees in level or climbing flight, and fold back toward 15 degrees in a vertical dive.

**`LivingEntity.tick`**: `fallFlyTicks++` while fall flying, else `fallFlyTicks = 0`; then `elytraAnimationState.tick()`.

**`AvatarRenderer.extractFlightData`**: `fallFlyingTimeInTicks = fallFlyTicks + partial`. With `motion` lerped from the previous tick's and `view = getViewVector(partial)`: if both horizontal lengths squared exceed `1.0e-5`, `shouldApplyFlyingYRot = true` and `flyingYRot = signum(motion.x*view.z - motion.z*view.x) * acos(min(1, |dot(horizontal(motion).normalize(), horizontal(view).normalize())|))`; else `false` and 0. The `abs` means the roll never exceeds 90 degrees.

**`AvatarRenderState.fallFlyingScale`** = `clamp(t*t/100, 0, 1)` with `t = fallFlyingTimeInTicks`: full after 10 ticks (0.5 s).

**`AvatarRenderer.setupRotations`**, while fall flying: the living rotations (yaw), then, unless `isAutoSpinAttack`, `rotateDegrees(XP, scale * (-90 - xRot))`, then if `shouldApplyFlyingYRot`, `rotate(YP, flyingYRot)`. `LivingEntityRenderer.submit` calls it at the entity position before `scale(-1, -1, 1)` and `translate(0, -1.501, 0)`, so the body pivots at the feet. When the glide ends the tilt goes at once (no ease).

**`HumanoidModel.setupAnim`**: while fall flying `head.xRot = -0.7853982` (head 45 degrees back from the body, looking ahead), and the walk swing is divided by `speedValue`, which `HumanoidMobRenderer` sets to `max(1, (lengthSqr(motion)/0.2)^3)` while fall flying: the arms and legs hang straight along the body.

## 4. Plan: commit groups

Every new file must pass `rustfmt --check` on its staged content. No comments. Edits to upstream files (`occupancy/remote_body.rs`, `anim/remote_body.rs`) stay a few lines each; the logic lives in the new files.

### Group 1. "Minecraft elytra wings are modelled and eased like vanilla"

- New `crates/minecraft_terrain/src/elytra_render.rs` (pure, no MW2 types):
  - `WINGS_TEXTURE = "minecraft:entity/equipment/wings/elytra"`, `wings_texture(item_id) -> Option<ResourceId>` (only `minecraft:elytra`, the one vanilla equipment asset with a `wings` layer).
  - `WingAngles { x, y, z }` (radians, the left wing's) with `REST`, `CROUCH`, and `target(gliding, crouching, motion: [f64; 3]) -> WingAngles` with motion in blocks per tick, map axes (z up).
  - `ElytraAnim { old: WingAngles, now: WingAngles, fall_fly_ticks: u32, partial: f32 }` starting at zero like vanilla. `advance(&mut self, seconds, gliding, crouching, motion)` runs whole 50 ms ticks (each tick: `fall_fly_ticks` +1 or reset to 0, then the 0.3 ease toward `target`) and keeps the fraction in `partial`. `wings() -> WingAngles` lerps old to now by `partial`. `scale() -> f32` is `fall_flying_scale(fall_fly_ticks + partial)` while gliding, 0 otherwise.
  - `fall_flying_scale(t) = clamp(t*t/100, 0, 1)`.
  - `flying_roll(look: [f64; 3], motion: [f64; 3]) -> Option<f32>`: the vanilla formula in map axes, where the sign is `(look_h × motion_h).z` (map `to_block` is `(x, z, -y)`, so vanilla's `motion.x*view.z - motion.z*view.x` becomes `look.x*motion.y - look.y*motion.x`), `None` below the `1.0e-5` threshold.
  - `append_elytra(mesh, atlas, frame: Mat4, wings: WingAngles, crouching: bool, glint: bool, sky, block)`: two `cube_posed_slice` boxes on the 64x32 panel at UV (22, 0), size 10x20x2, grow 1. Left: `frame * T(5, 0, 2) * Rz(z)·Ry(y)·Rx(x)`, from (-10, 0, 0), no mirror. Right: `frame * T(-5, 0, 2) * Rz(-z)·Ry(-y)·Rx(x)`, from (0, 0, 0), mirrored. The `2` is `WingsLayer`'s 0.125 block. Pushes `mesh.armor_glint` when `glint`. Skips when the atlas lacks the texture.
- `lib.rs`: `pub mod elytra_render;` next to `body_render` (`:115`).
- `mesh.rs:614-622`: insert `WINGS_TEXTURE` into the entity atlas when the pack has it, like the armour.
- **Tests** (`elytra_render`): `only_the_elytra_has_wings`; `the_wings_fold_at_15_degrees_spread_to_90_and_crouch_at_40_and_45` (targets for standing, crouch, level glide, climbing glide, a straight-down dive = rest, a 45-degree dive with `k = 1 - 0.7071^1.5 = 0.4054`); `the_wings_ease_three_tenths_a_tick_from_zero` (one tick from zero gives `x = 0.0785398`; half a tick later `wings()` is halfway; 30 ms and 20 ms frames match one 50 ms tick); `fall_flying_scale_is_ticks_squared_over_100` (5 ticks 0.25, 10 and 20 ticks 1, 0 the frame the glide ends); `the_flying_roll_is_the_signed_angle_between_motion_and_look` (motion 30 degrees to the left gives +0.5236, to the right -0.5236, at right angles +-1.5708, straight backward 0 because of the `abs`, a vertical dive `None`); `an_elytra_is_two_mirrored_wings_on_the_back` (12 faces, every UV inside the panel's (22..46, 0..22) box-UV area; at zero angles both boxes span x -6..6, y -1..21, z 1..5 pixels; the right wing's faces are wound the other way); `an_enchanted_elytra_shimmers_with_the_armour_glint`.

### Group 2. "A worn Minecraft elytra is drawn on the MW2 body's back"

- `body_render.rs`: `BodyRig` gets `wings: Option<WingAngles>` and `crouching: bool` (`None` draws `WingAngles::REST`). `append_body_gear` draws the elytra when `gear.armor[1]` has `wings_texture`, on `rig.armor.body.map(|part| part.upper)`, the torso frame the chestplate uses (it starts at the neck, rides `j_spine4`, and is fitted to the MW2 torso by `GEAR_FIT`). Same light as the armour; glint from the `GearItem`.
- No change to `armor_render.rs`: the elytra stays outside `armor_asset`.
- **Tests** (`body_render`): `a_worn_elytra_is_drawn_on_the_back_and_shimmers_when_enchanted` (a rig with a body frame and an enchanted elytra in `armor[1]` gives 12 faces, all in `armor_glint`; without a body frame, nothing; an elytra in the head slot, nothing).
- In game the wings now show folded at rest, with no glide state yet.

### Group 3. "Each MW2 body keeps its Minecraft glide and wing state from its player's replicated state"

- New `crates/render_anim/src/anim/minecraft_glide.rs`:
  - `GlideInput { gliding, crouching, motion, look, pitch_deg }` from `glide_input(ps: &PlayerState)`: `gliding = pm_flags & MC_GLIDING != 0 && pm_type < PM_TYPE_DEAD`; `crouching = e_flags & eflags::DUCK != 0`; `motion = velocity / (BLOCK * 20)` (blocks per tick); `look` from `viewangles`.
  - `#[derive(Resource, Default)] McGlide { by_client: HashMap<u32, BodyGlide> }` with `BodyGlide { anim: ElytraAnim, input: GlideInput }`, `get(client)`, `retain(clients)`, `clear()`.
  - `step_body_glides(time, presented, glide)`: for every client in `presented.snapshot()` read `presented.player(id)`, advance its `ElytraAnim` by the frame time, and drop clients that are gone. This is client-side presentation, as vanilla's `fallFlyTicks` and `ElytraAnimationState` are ticked on every client per entity. It reads only replicated per-player state, so no player-0 or global state is added.
  - `GlidePose { scale, roll: Option<f32>, pitch_deg, wings, crouching }` from `McGlide::pose(client)`.
- `anim/mod.rs`: `pub mod minecraft_glide;`.
- `occupancy/remote_body.rs:81-82`: `init_resource::<McGlide>()` and the system `step_body_glides.after(PresentedPublished).before(pose_remote_bodies)`.
- `anim/minecraft_gear.rs`: `gear_rigs`/`gear_meshes` take `&McGlide` and set `BodyRig.wings` and `crouching` from the client's pose. `minecraft_world.rs:2675-2686` `draw_body_gear` passes `Res<McGlide>`; the world teardown in `update` clears it next to `body_gear.clear()` (`:454`, the `ResMut<McBodyGear>` is at `:423`; `stop` at `:2878` takes no gear resources).
- **Tests** (`minecraft_glide`): `a_glider_is_read_from_its_pm_flags_and_a_dead_one_is_not`; `velocity_becomes_blocks_per_tick` (720 map units per second is 1 block per tick); `each_client_keeps_its_own_glide_and_gone_clients_are_dropped`; `the_wings_spread_over_frames_while_gliding_and_fold_after_landing`. `minecraft_gear`: `gear_rigs_take_each_clients_wing_angles_and_rest_without_one`.
- In game: the wings spread while gliding, fold in a dive and on landing, and open a little when crouching. The body is still upright.

### Group 4. "Gliding MW2 bodies lie along their flight and roll into turns like vanilla"

- `minecraft_glide.rs`:
  - `glide_root(scale, pitch_deg, roll) -> Quat` in the MW2 body frame (X forward, Y left, Z up): `Ry(scale * (90 + pitch))` then `Rz(roll)` (post-multiplied, so the roll turns about the tilted long axis). `Ry(+θ)` moves up toward forward, which matches vanilla's `XP(scale * (-90 - xRot))` with MW2 pitch positive downward. Vanilla's `YP` and MW2's `Rz` turn the same way about the body's up axis.
  - `aim_pitch(pose, view_pitch) = view_pitch * (1 - scale)`: the tilt takes over the pitch, so the spine does not bend on top of it.
  - `apply_glide_root(locals, pose)`: `locals[0].rotation = glide_root(..) * locals[0].rotation`, after `apply_player_controller` has written bone 0. Bone 0 is `tag_origin` at the feet, so the body pivots at the feet as vanilla's does. Every posed bone follows (skin, gun, fx bolts, `PosedPlayerFrame`, `PosedGearFrame`), so items, armour and wings ride along with no extra code.
- `anim/remote_body.rs:1050-1062`: `pose_remote_dobj` takes `glide: Option<GlidePose>` and calls `apply_glide` in its closure after the controller (3 lines).
- `occupancy/remote_body.rs`: `RemotePoseFrame` gets `glide: &McGlide` (`:486-509`, `:552-561` tuple, `:672` init). In `pose_one_remote` (`:827-830`, `:914-918`): for a living, non-skating body, `let glide = self.glide.pose(client)`; the controller gets `aim_pitch`. The inventory puppet uses `puppet.pitch` for the tilt and no roll (vanilla's inventory figure also lies down while gliding). Corpses and skating bodies get none.
- **Tests** (`minecraft_glide`): `the_body_lies_along_the_look_at_full_scale` (`glide_root(1, 30, None) * Z` equals the look `(cos 30, 0, -sin 30)`; scale 0 is identity; scale 0.5 tilts by 60 degrees); `the_roll_turns_about_the_long_axis` (`* Z` is unchanged by the roll, `* Y` turns by it); `the_aim_pitch_fades_as_the_body_tips`; on a synthetic skeleton (the `minecraft_gear` test skeleton plus `tag_origin`): `the_root_turns_every_bone_about_the_feet` (the feet stay put, `j_spine4` swings forward to the expected point).

### Group 5. "A gliding MW2 body looks ahead and stretches its legs like vanilla"

- `minecraft_glide.rs` `apply_glide_limbs(dobj, locals, pose)`, run in the same closure before the root tilt. It uses one `dobj.compose(locals, IDENTITY)` to read each parent's model-space rotation `P`, and turns a bone by a rotation `R` given in the body frame as `local' = (P⁻¹·R·P)·local`. That keeps the joint where it is and needs no assumption about MW2 bone axes. Avoid `control` here: it would drop the bone's own bind rotation (`dobj.rs:415-421`).
  - Head: `j_neck` by `Ry(-45° * scale)` (vanilla `head.xRot = -0.7853982`: the face turns from the ground toward the flight).
  - Legs: for each side, the hip by the arc from its hip→knee direction to body `-Z`, then (after a second compose) the knee by the arc from knee→ankle to `-Z`, each slerped from identity by `scale`, so the legs trail straight like vanilla's swing-free legs.
  - The arms keep MW2's grip (gun or Minecraft item), as on the ground.
- **Tests**: `the_head_tilts_back_45_degrees_from_the_spine_at_full_scale`; `the_legs_trail_straight_down_the_body_at_full_scale` (hip→knee and knee→ankle parallel to body -Z within 1e-3, from a bent synthetic pose); `nothing_moves_at_scale_zero`.
- This is its own commit because it is the most calibration-sensitive part. If the owner's screenshots show it wrong, it can be reverted without losing groups 1-4.

### Group 6. "The Minecraft state doc draws the elytra and the glide pose on the MW2 body"

- Line numbers are as of e2313625; the other C items' docs commits move them, so find each line by its text.
- `docs/MINECRAFT-SURVIVAL.md:193` (the elytra line gains "drawn on the body, wings spread and the body lies along the flight"), `:284` (drop the elytra gap), `:299` (drop "the elytra" from the not-drawn list; add the limits from section 6: hits use the upright pose, no rocket or wind particles).
- Then the runner's clippy entry for `minecraft_terrain` and `render_anim`.

## 5. The owner's test steps

Start `iw4l-survival.exe map minecraft:overworld` in survival with this kit (7 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:elytra,minecraft:elytra+unbreaking=3,minecraft:firework_rocket*64,minecraft:cobblestone*64,minecraft:diamond_sword,minecraft:torch*64,minecraft:diamond_chestplate
```

1. Put the plain elytra in the chest armour slot and press F5. Back view: two grey wings folded on the back, from the shoulders to about the knees, slightly open at the bottom. F5 again for the front view: the wing edges show behind the arms. **Screenshots 1 and 2.**
2. Crouch: the wings open a little wider and tilt back. Stand up: they fold back over about a quarter second. **Screenshot 3.**
3. Open the inventory (E): the character in the box wears the wings. **Screenshot 4.**
4. Swap in the Unbreaking III elytra: the wings shimmer with the broad armour glint. Put the diamond chestplate on instead: the wings are gone and the chestplate shows. Put the plain elytra back.
5. Type `/tp ~ ~40 ~`, keep F5 on the back view, and press jump in the air. Over half a second the body tips forward until it lies along where you look, and the wings spread wide. Look level: the body lies flat. Look down to dive: the body points head first down, and the wings fold back narrower as the dive steepens. Look up: the body angles up. **Screenshots 5 (level) and 6 (dive).**
6. While gliding, flick the mouse sharply left, then right: the body rolls into each turn for a moment and levels out as the flight follows the look. **Screenshot 7** mid-roll.
7. From group 5 on: in the front view while gliding, the head looks ahead along the flight and the legs trail straight. **Screenshot 8.**
8. Select the sword, put the torch in the offhand (F) and glide: both stay in the hands and tip with the body. Select an MW2 gun: it still fires while gliding, and the body stays tipped. Right-click a rocket: the body stays along the faster flight.
9. Land: the body is upright at once and the wings fold within about a quarter second.
10. Open the inventory mid-glide: the character in the box lies tipped like the body (vanilla does the same). Say if it leaves the box.
11. For anything floating off the back, sunk into it, too big or turned wrongly, say which and by about how much.
12. Later, once track E is merged: a friend with an elytra shows the same wings and pose on your screen.

## 6. Risks, shared files and decisions

**Decisions**
- **No MW2 animation for the glide.** IW4 has none. Prone was rejected: its anims are ground poses (elbows down), the stand-to-prone transition takes about a second, the leaf depends on the weapon class, and driving it needs `eflags::PRONE`, which changes hit locations, view height, footsteps and accuracy. The tree keeps playing the MW2 in-air anim, and the glide is a pose override after the tree: the root tilt and roll on `tag_origin`, the aim pitch faded out, and the head and leg turns. That is vanilla's own split: the renderer rotates the whole model, and the model sets the head and calms the limbs.
- **The tilt is applied on bone 0, not on the entity `Transform`.** Then the skin, the gun, fx bolts, the posed head and the gear frames all follow from one place, and `sync_remote_bodies` stays untouched.
- **The pivot is at the feet, as in vanilla.** The MW2 hull and eye stay standing (round 7 decision), so in F5 the lying body sits lower in view than vanilla's 0.4-block eye would show it.
- **The glide and wing state is client-side presentation per client number**, ticked at 20 Hz from the replicated `PlayerState` (`pm_flags`, `velocity`, `viewangles`, `e_flags`). This mirrors vanilla, where every client ticks `fallFlyTicks` and `ElytraAnimationState` for every entity. No new server state, no new net field, nothing keyed to player 0.
- **The wings ride the torso's upper frame** (the chestplate's, on `j_spine4`), so they bend with MW2 crouch and prone. In vanilla they sit on the model root. The crouch target angles are used, but not the `y = 3` drop, because the MW2 spine already lowers.
- **The elytra is known by its item id** (`minecraft:elytra`), as the armour is by table. A custom `equippable` asset override is ignored. There are no player skins, so the texture is always the default elytra.
- **Flight details kept from vanilla:** the snap upright on landing, the roll capped at 90 degrees by the `abs`, and wings that start at zero for a body seen for the first time. The riptide exception (`isAutoSpinAttack`) is not modelled.

**Risks**
- **Hits:** MW2 bullets hit the authority's upright in-air pose (`sim/src/bullet_collision.rs`), while the body is drawn lying. In PvP, shots at the drawn body can miss. Vanilla's 0.6-block gliding box also doesn't match its model. Leave it, and note it for track E.
- **Culling:** LOD and bounds assume an upright body. A lying body reaches about 2 blocks ahead at foot height, so it may pop at screen edges.
- **Calibration:** the wing size follows `GEAR_FIT`'s torso. Group 5's turns are axis-free but untested on the real kit, so the screenshots decide.
- **Cutout:** the wing shape comes from the texture's alpha. Vanilla draws it with no culling, so check that the item pass doesn't lose the inner faces where the wing is cut.
- **Inventory puppet:** the tilt may poke out of the character box (step 10). If it does, the fallback is to give the puppet no tilt.

**Files shared with other round 8 tracks**
- `occupancy/remote_body.rs` and `anim/remote_body.rs` (`pose_one_remote`, `pose_remote_dobj`): **track D** (skating poses the body in the same branch). Keep the glide edits to a few lines beside the skate branch and skip the glide while skating. Merge order doesn't matter beyond trivial conflicts.
- `McBodyGear`, `minecraft_world.rs` `draw_body_gear`, `gear.rs`: **track E** fills other players' gear (the elytra comes with the chest slot). The glide needs E to keep relaying each remote player's `pm_flags` (with `MC_GLIDING`), `velocity`, `viewangles` and `e_flags` in the presented `PlayerState`, as today. If E moves remote players to entity states only, it needs an `e_flags` bit or a message for the glide.
- `mesh.rs` entity atlas list (`:614-622`): **track C's** other items (item frames, shulkers) and **track B** (piglins, hoglins, striders) add textures there too. The conflicts are trivial.
- `minecraft_world.rs`: every track. Only `draw_body_gear`, the `update` resource tuple and the teardown at `:454` are touched here.
- **Inside track C:** c1 group 2 changes `minecraft_elytra.rs` (no glide while levitating) and the glide gravity; the glide pose reads `MC_GLIDING` from `pm_flags`, so a levitating player simply shows no glide. No shared lines.
- `docs/MINECRAFT-SURVIVAL.md`: every track. Group 6 edits only lines 193, 284 and 299.
- `minecraft_camera.rs` (F5, **track A**): not touched. Test the F5 views together.
