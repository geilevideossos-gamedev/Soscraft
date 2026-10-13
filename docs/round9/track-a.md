# Track A: multiplayer fixes

The owner tested round 8's release 2 (03e0c9be) with two windows. Screenshots are in `scrennshots/screenshots-03e0c9b/screenshots-03e0c9b/` (untracked, in `C:\Dev\mashup-crafting`). Screenshot N is the file whose name starts with N. Track A has seven items:

- **A1:** players are invisible to each other in the Nether.
- **A2:** held items float away from the hands and overlap.
- **A3:** the ender dragon stutters for the friend.
- **A4:** worn mob heads and carved pumpkins are not drawn, and the pumpkin overlay is missing.
- **A5:** four end crystals "in the wrong spots" still brought a new dragon.
- **A6:** a friend in creative mode gets the survival inventory.
- **A7:** players get stuck inside each other after a spawn or a teleport.

Working in release 2: joining with two windows, seeing each other in the Overworld and the End, chat, shared chests, damage between players, and both players travelling to the Nether and the End.

Track A runs beside tracks B (items and the screen), C (guns and grenades as items) and D (finishing survival). It is merged together with track B before the round's first release build.

Paths and line numbers are as of f1dddf78, so re-check them before editing. Vanilla facts were checked with javap on the 26.3 `client.jar`. No Mojang code or data is copied.

Every section below has the same parts: the owner's finding, the cause, what vanilla does, the plan, the tests, and the owner's in-game steps with a kit line. The commit subjects are the ones listed in each plan.

Every fix must work in both directions: the host seeing a friend, a friend seeing the host, and a friend seeing another friend. All game state stays on the host, per player and keyed by UUID.

---

## A1. Players are invisible to each other in the Nether

### The owner's finding
Screenshot 1 (`1_nether_spieler_unsichtbar.png`). In the Nether the other player's MW2 body, held items, armour and name are not drawn. This happens for the host and for the friend. Hit boxes and damage work. In the Overworld and the End both players see each other.

### Cause
The dimension gate is not the cause. The other body is never posed, because the renderer's DPVS hides it. The cell tree of the MW2 stand-in map (`mp_rust`) has no cell for a point 45,000 units below the map.

1. **The stand-in map.** The Minecraft world is played on top of the MW2 map `mp_rust` (`crates/assets/src/minecraft_map.rs:1-8`). Its world surfaces are hidden on the GPU only (`render_gpu/src/drawsurf/minecraft_world.rs:251`, `HIDES_MAP`). Its DPVS still decides on the CPU which MW2 scene entities are drawn. The owner's log shows `dpvs: cells=1 planes=2687 nodes=1978` for `mp_rust`.
2. **Linking.** Every MW2 body, corpse or grenade is a scene dobj. `link_scene_ents` (`crates/render_frontend/src/adapters/anim/dyn_ent_brush.rs:322-391`, `cache.link` at `:358` and `:363`) puts it in the cells its bounds reach. The walk is `dpvs_iw4::scene_ent_cells` (`crates/dpvs_iw4/src/scene_ent.rs:255-300`). Leaf 0 means "no cell", so an entity there is linked nowhere.
3. **Marking.** Only entities linked to a visible cell are marked visible and skinned (`dyn_ent_brush.rs:493-590`). They must also pass `reaches_cell` (`:394-399`, called at `:558` and `:638`). After the walk (`scene_ent_walked = true` at `:496`), `GfxScene::scene_ent_hidden` (`crates/render_scene/src/gfx_scene.rs:233-235`) is true for every live entity that was not marked.
4. **No pose.** `pose_remote_bodies` returns `SceneHidden` before posing (`crates/render_anim/src/occupancy/remote_body.rs:551`, `:905-906`). So:
   - there is no skinned body and no MW2 gun;
   - there are no `PosedGearFrame` joints, so armour and held items are not drawn (`gear_rigs`, `anim/minecraft_gear.rs:503-532`);
   - there is no posed head, so the name is dropped (`crates/hud/src/overhead_names.rs:206-218`).
5. **Where the Nether is.** The bands (`crates/minecraft_survival/src/bands.rs:8`, `:48`, `NETHER_ORIGIN_Y` 1314) put a Nether player at map z = (y − 1314) × 36, about −44,800 at y 70. That is far below `mp_rust`. Overworld players near spawn height are inside the map's cell. End players (about +43,000) are above the map, and that region evidently maps to the cell.

**Evidence from the owner's logs** (`iw4l-artifacts/mp-local/{host,friend}/iw4l-artifacts/logs/1791872574.log` and `1791872579.log`):
- The band gate passes. The overhead-names loop reaches the head lookup only after `sim::voxel::shown_to` passes (`overhead_names.rs:187-193`). Its gap "entity 1 has no same-Present posed-head product" stays live for the whole Nether stay (host log lines 7198-7275), even while the host shoots the friend in the torso (line 7226).
- The body is never submitted. The render gap "remote-body-animation: playeranim evaluator missing" (host line 7190, friend line 4156) is cleared only when a body is submitted (`remote_body.rs:1494`, `:1575`). It stays live until both players are back in the Overworld. A body that is `SceneHidden` every frame returns at `remote_body.rs:757-760` without clearing it, which matches.
- The friend's log shows the same for entity 0 (the host).

The Minecraft side is correct: `shown_to`/`band_of` (`crates/sim/src/voxel.rs:63-73`), the bands and the snapshots. Damage works because the sim's hit boxes do not use the renderer's DPVS.

### Vanilla 26.3
There is no portal or BSP visibility for entities. `EntityRenderer.shouldRender` does three things:
1. `Entity.shouldRender(x, y, z)`, the distance check against the view scale.
2. If `affectedByCulling`, the inflated culling box against `Frustum.isVisible(AABB)`.
3. The leash-holder exception.

So every player tracked in the viewer's own level is drawn whenever their box is in the frustum and within render distance, with armour, held items and name tag, in every dimension.

### Plan
One commit, client side only, in `crates/render_frontend/src/adapters/anim/dyn_ent_brush.rs`.

1. **"MW2 bodies show in the Nether and deep caves: the stand-in map's cells no longer hide them in Minecraft worlds"**
   - **`link_scene_ents` (`:322`):**
     - Read `let every_cell = sim::voxel::active();` once, after the early returns. `render_frontend` already depends on `sim` and already calls `sim::voxel::active()` (`prepare/scene/model_lighting_cache.rs:198`).
     - For `scene_models` and `scene_dobjs`, when `every_cell` is true, link the entity into every cell instead of `cache.link(...)`. Put the loop in a small helper `fn link_every_cell(bits: &mut [u32], cell_count: usize, ent_id: u32)` that calls `dpvs_iw4::add_scene_ent_to_cell` for each cell.
     - `bits.bits.fill(0)` already runs every frame. Leave the cache untouched in this branch.
     - Leave the brush branch (`filter_bmodel_into_cells`) as it is.
   - **`exec_cell_scene_ent_cmds` (`:471`) and `drain_dpvs_ent_cmds` (`:607`):**
     - Build `dpvs` as `cull.filter(|_| !minecraft).map(...)`, with `minecraft` read once per system. `reaches_cell(None, ..)` then returns true. In both functions `dpvs` is only used by `reaches_cell`.
     - A pure helper such as `fn cell_planes(cull, minecraft) -> Option<DpvsPlanes>` makes this testable.
   - Sphere, frustum and LOD culling stay, so bodies off screen are still skipped.
   - **Optional log:** under `every_cell`, once per session (a `Local<bool>`), log with `diag::info!(World, ...)` how many scene entities would have had no cell. Walk only until the first hit. This confirms the cause in the owner's next log.
   - **Nothing else changes.** The band gate stays (`shown_to` in `remote_body.rs`, `minecraft_gear.rs`, `overhead_names.rs`), so players in another dimension stay hidden. On MW2 maps `voxel::active()` is false and DPVS behaves as before. No `minecraft_net` message changes.

Side effect: other MW2 scene entities (corpses, thrown grenades, dropped guns) in the Nether and in deep Overworld caves start drawing too. That is wanted.

Not in this item: MW2 FX and marks may be culled by `mp_rust`'s cells too (`render_frontend/src/adapters/fx/world_mark.rs` builds `DpvsPlanes`). This was not checked. Note it in the track log as a follow-up.

### Tests
In a new `#[cfg(test)] mod tests` at the end of `dyn_ent_brush.rs`. Use a hand-built one-cell tree: `planes = [CPlane { normal: [0.0, 0.0, 1.0], dist: 0.0, r#type: 2 }]`, `nodes = [2, 3, 1, 0]`, `cell_count = 1`. The front (z > 0) is leaf 1 (cell 0), the back is leaf 0 (no cell), like the ground under the stand-in map.
- `a_body_below_the_stand_in_map_reaches_no_cell`: `scene_ent_cells` for a body box at z −45,000 yields nothing, and `reaches_cell(Some(&dpvs), box, 0)` is false. A box at z +43,000 yields cell 0, which is why the End works.
- `a_minecraft_world_links_every_scene_entity_into_every_cell`: `link_every_cell` on bits sized with `scene_ent_cell_bits_len(1)` sets the entity's bit in cell 0 (`scene_ent_in_cell`), and `reaches_cell(None, box, 0)` is true at z −45,000.
- `cell_planes` returns None when `minecraft` is true and Some otherwise.
- The band gate test `only_bodies_in_the_viewers_band_are_shown_to_them` (`sim/src/voxel.rs:1988`) already covers Nether to Nether.

Run `test -p render_frontend`, `test -p dpvs_iw4` and `check -p launcher`.

### Owner's steps (idea)
Two windows: `& scripts\mp-local.ps1 -World a1-nether -Fresh`.
1. **Overworld baseline.** Host: `/op Friend`. Both: put on the helmet and chestplate and hold the sword, a few blocks apart. Each sees the other's soldier with armour and sword, and the name when aiming. Screenshot both windows.
2. **Deep in the Overworld.** Host: `/fill ~-4 -40 ~-4 ~4 -36 ~4 minecraft:air`, `/tp @s ~ -40 ~`, `/tp Friend ~3 ~ ~`. Place torches. Both see each other with armour, sword and name. Screenshot both. Then go back up.
3. **The Nether.** Build a portal and walk in together. Stand 3-4 blocks apart. Each sees the other's soldier, dimly lit like the terrain, with armour, sword and name (the screenshot 1 case). Screenshot both.
4. **Moving.** The friend walks 30 blocks away and back: the host sees them all the way, with leg animation. The friend switches to a gun: the host sees it in their hands.
5. **Third person.** Both press F5: their own body shows in the Nether with armour.
6. **Still hidden across dimensions.** The friend goes back through the portal: in the Nether the host sees no body, name or floating sword. The friend comes back: both see each other again.
7. **The End.** Both go to the End: they still see each other.
8. **No stand-in props.** Back at spawn: no MW2 desert props or crates appear anywhere.

```
IW4L_MINECRAFT_KIT=minecraft:obsidian*10,minecraft:flint_and_steel,minecraft:iron_helmet,minecraft:iron_chestplate,minecraft:iron_sword,minecraft:torch*32,minecraft:cobblestone*64
```

---

## A2. Held items float away from the hands and overlap

### The owner's finding
Screenshots 2, 3 and 4 (`2_items_schweben_inventar.png`, `3_items_schweben_sos.png`, `4_items_schweben_friend.png`). Held items float away from the hands, and the main-hand and offhand items overlap. This shows on other players' MW2 bodies and on the inventory's character. In screenshot 3 the totem in Sos's offhand floats low on Sos's right. In screenshot 2 the inventory character's item is above its right shoulder.

### Cause
Every Minecraft hand item is placed on the MW2 weapon tags `tag_weapon_right` and `tag_weapon_left`, using the tag's posed matrix. That only works while the playing MW2 animation keys those tags at the hands. In the bind pose they are not at the hands.

- `crates/render_anim/src/anim/minecraft_gear.rs:305-324` `hand_frame()` takes `joints[HAND_TAGS[side]].posed` whenever the tag exists (`:310-311`). The wrist fallback (`:312-315`) never runs, because the kit has both tags.
- The owner's log line "remote kits: axis=... Minecraft gear bones missing: []" (printed by `occupancy/remote_body.rs:169-177`) gives:
  - right hand tag (2.3, −9.9, −5.2) from the right wrist, about 11 units away;
  - left hand tag (2.3, −47.1, −5.2) from the left wrist, about 48 units (1.3 blocks) away.
  - Assuming symmetric wrists, both tags share one bind point on the body's right side.
- So whenever an animation does not key a tag, the item rides the bind offset. The offhand item crosses to the right side and lands on the main-hand item.
- The screenshots show pistol-style poses. In round 7 the owner saw correct F5 gear with a rifle raised. MW2 keeps the last raised gun's animations while a Minecraft item is selected (`minecraft_survival/src/guns.rs` `Follow`, `net/src/client/runtime.rs:1102`), so the result depends on the gun class.
- The local F5 body and the inventory character use the same path (`remote_body.rs:440-449`, `:458-459`, `:970-981`, and `minecraft_world.rs:2140-2165` `draw_body_gear`). Nothing on the hand path changed in round 8.

### Vanilla 26.3
- `ItemInHandLayer.submit` draws the right-hand item on `HumanoidArm.RIGHT` and the left on `LEFT`. With `mainArm` RIGHT, the main hand is in the right arm and the offhand in the left.
- `submitArmWithItem`: `translateToHand(arm)` (root pose, then the arm part, so the item rides the arm bone), then `Rx(−90)`, `Ry(180)`, then `translate((left ? −1 : 1)/16, 2/16, −10/16)` for adults. Then the item with the hand's third-person display.
- So the item always sits at the fist of its own arm. The two hands never share a point.

For the MW2 body the matching anchor is the posed wrist, which always skins the hand mesh, plus a short step along the forearm to the middle of the fist.

### Plan
One commit, all in `crates/render_anim/src/anim/minecraft_gear.rs`. No protocol, server or `minecraft_terrain` change.

1. **"Minecraft held items sit in the MW2 body's fists from the wrists, not on the weapon tags"**
   - **`GearFit`:** add `fist: f32`, the map units from the wrist along the forearm to the middle of the fist. `GEAR_FIT.fist = 3.5`. (Vanilla's fist centre is 2 px past the wrist; one px is 36/16 = 2.25 units, times the 0.9375 item scale, so about 4. MW2's hand is a little smaller.) Keep `item_scale` and `hand_offset`.
   - **Rewrite `hand_frame(joints, side, fit)`** so it never reads `HAND_TAGS`:
     - `let wrist = joints[WRISTS[side]]?; let at = wrist.bind.w_axis.truncate();`
     - `let along = joints[ELBOWS[side]].and_then(|elbow| (at - elbow.bind.w_axis.truncate()).try_normalize()).unwrap_or(Vec3::ZERO);`
     - `let fist = wrist.skin().transform_point3(at + along * fit.fist);` The bind-space fist point is carried by the wrist's skinning delta, so it follows every MW2 arm pose exactly like the hand mesh.
     - **Orientation upright with the chest** (the round 7 rule: "as a vanilla player holds it with the arm down, not turned with the forearm"): `let turn = [SPINE4, SPINE_UPPER, SPINE_LOWER].into_iter().find_map(|j| joints[j]).unwrap_or(wrist).skin();` and `let base = Mat4::from_cols(turn.x_axis, turn.y_axis, turn.z_axis, fist.extend(1.0));`. The skin delta carries the body's yaw, the inventory character's root scale, the full aim pitch (`xmodel_runtime/src/dobj.rs:505-507` pitches spine4), crouch, prone and the glide tilt. Its axes are MW2 model axes, which is what `model_to_hand()` expects.
     - The rest stays: `base * model_to_hand() * scale(BLOCK * item_scale) * translate(Vec3::new(x * mirror, y, z) / 16.0)`.
   - **Keep `tag_weapon_right` and `tag_weapon_left` in `GEAR_BONES`.** `measures()` and the log line still name them, and the MW2 riot shield uses `tag_weapon_left` through sim.

This is presentation only. Host and friend both build every body's rig from their own `PosedGearFrame` and `McBodyGear`, and `draw_body_gear` runs the same `gear_meshes` on both. So one fix corrects other players' bodies, the own F5 body, the inventory character, and gliding, sleeping and crouching bodies, on both sides. Corpses and skating bodies are unaffected.

### Tests
In `minecraft_gear.rs` tests. Rewrite the three tests that pin the old tag rule; keep their coverage:
- `a_hand_tag_turns_into_the_vanilla_hand_frame_in_blocks` becomes `a_wrist_turns_into_the_vanilla_hand_frame_in_blocks`: right wrist, elbow and SPINE4 with identity skin and `fist` 0; the origin lands at the wrist's block point; forward is block +x, up +y, right +z; `rig.hands[1]` is None without a left wrist.
- `the_hand_offset_is_in_pixels_and_mirrored_for_the_left_hand`: same assertions with wrists in place of tags.
- `without_a_hand_tag_the_hand_rides_the_wrist_upright_from_the_bind_pose` becomes `the_fist_rides_the_wrist_and_the_frame_stays_upright_with_the_chest`: turn the wrist 90 degrees; the fist point moves with the wrist; the forward axis still follows SPINE4.

New:
- `held_items_ignore_unkeyed_weapon_tags`: `standing()` joints, both tags at the same point on the right, (2.3, −28.5, 30). Each hand origin is within `fist` + 1 of its own wrist, and the two origins are more than 15 units apart.
- `the_fist_steps_along_the_forearm`: in the bind pose and after an elbow bend.
- `the_hand_frame_turns_with_the_chest_and_the_puppet_scale`: SPINE4 turned 90 degrees about Z turns the forward axis; a placement scaled by 0.1 scales the frame axes by 0.1 and keeps the fist at the scaled wrist.
- `the_hand_frame_falls_back_to_the_wrist_without_a_spine`: no spine uses the wrist's skin; no wrist gives no hand.

Keep `only_bodies_with_gear_get_a_rig_and_the_puppet_is_kept_apart`, `gear_is_drawn_only_on_bodies_in_the_viewers_band` and the `body_render` tests unchanged.

### Owner's steps (idea)
Two windows: `& scripts\mp-local.ps1 -World r9-a2 -Fresh`. Host: `/time set day`, `/op Friend`. Stand 4 blocks apart, facing each other.
1. **Host, F5 front view.** Diamond sword in the main hand, torch in the offhand (F), F5 twice. The sword comes out of the right fist, the torch out of the left; nothing floats and they don't touch. Select a pistol, wait, select the sword again: still in the fists. Repeat after a rifle and after an SMG. Screenshot each.
2. **Inventory.** E: the small character holds the sword in its right fist and the torch in its left, nothing above the shoulder (the screenshot 2 case). Turn it: the items stay in the hands.
3. **Friend's view of the host.** Same sword and torch in the fists. The host walks, crouches, goes prone and looks up and down: the items stay in the fists and tilt with the chest.
4. **Totem.** The friend empties the main hand and holds the totem in the offhand. In the host window the totem is in the friend's left fist, not low on the right (the screenshot 3 case).
5. **Book.** The friend selects the enchanted book. Their inventory character and the host window show it in the right fist (the screenshot 4 case).
6. **Gun plus offhand.** The host holds a gun with cobblestone in the offhand: the gun as before, the cobblestone in the left fist.
7. **Log.** `latest.log` still has `Minecraft gear bones missing: []`. If an item sits a little inside or outside the fist, send the screenshot: `GEAR_FIT.fist` and `hand_offset` are the tuning values.

```
IW4L_MINECRAFT_KIT=minecraft:diamond_sword,minecraft:torch*64,minecraft:cobblestone*64,minecraft:totem_of_undying,minecraft:enchanted_book,minecraft:iron_helmet
```

---

## A3. The ender dragon stutters for the friend

### The owner's finding
Screenshots 5 and 6 (`5_drache_ruckelt.png`, `6_drache_nah.png`). The dragon stutters and looks out of sync for the friend. Screenshot 5 shows the circling dragon mid-twitch. Screenshot 6 is a close-up of the head and wing and shows no defect that can be pinned down (the flat grey wing may be lighting, which is not part of this item).

### Cause
The dragon is the one large, fast entity with no client-side copy. Every frame it is drawn from the newest server snapshot (`DragonView`), with the server's own `previous_position`/`position` pair and flight history. The partial tick comes from the view's free-running client clock, and nothing ties that clock to when snapshots arrive. Mobs don't have this problem, because `ClientMobs` replays vanilla's tracker and interpolates over 3 steps on the client's own ticks.

1. `EntityView::receive` replaces `self.dragons` with whatever arrived (`crates/minecraft_survival/src/entity_view.rs:207`).
2. `meshes` draws it with `partial = clock / tick_seconds` (`:665`, `:809-818`). That clock is advanced only by `advance` (`:108-118`).
3. `dragon_render` draws `feet = previous_position.lerp(position, partial)` (`crates/minecraft_terrain/src/dragon_render.rs:369-372`) and builds the neck and tail from `sample_at(history, back, partial)` (`:70-75`, `ender_dragon.rs:129-142`). The heal beam (`:202-206`, `:232-234`) and the death rays (`:320-324`) work the same way.

When a snapshot arrives at a different moment than the client tick wraps, the drawn dragon jumps back and forth once per tick. At the client tick the partial resets to 0 on the old snapshot, so the dragon snaps back one tick of flight (up to about 1.1 blocks, `ender_dragon.rs:1796-1809`). When the next snapshot lands it jumps forward. The whole body jitters with it.

- **Friend (the reported case).** State frames come over the relay with jitter and are drained in a batch each frame (`crates/render_anim/src/minecraft_remote.rs:260-264`), applied at once (`minecraft_survival/src/remote.rs:64-67`, `:267-283`), while the friend's clock ticks in `present_entities` (`minecraft_remote.rs:1497`). So the offset changes every tick: irregular twitching. On top of that:
  - the host sends at most one `StateFrame` per host frame and merges two level outputs that land in one frame (`session.rs:1236-1287`, `mob_sender.rs:120-147`);
  - the state lane keeps only the newest frame and drops lost ones (`crates/net/src/minecraft_wire.rs:623-636`, `:724-729`, `:825-827`).
- **Host (also affected, less visible).** The server ticks on `TickRate::plan`'s schedule (`tick_rate.rs:227-260`), the level thread answers asynchronously (`minecraft_terrain/src/server.rs:3222-3227`, `minecraft_survival/src/entities.rs:1261-1277`), and `view.advance` runs on its own accumulator (`entities.rs:1207`).

### Vanilla 26.3
- `EntityTypes.ENDER_DRAGON` keeps the builder's default `updateInterval` of 3 and sets `clientTrackingRange(10)`.
- On the client, `Entity.commonTick` calls `setOldPosAndRot()` and then `getInterpolation().interpolate()`. `SteppedInterpolationHandler` uses `updateInterval()` steps, so the client glides to each server target over 3 client ticks. `EnderDragon` overrides none of this.
- `EnderDragon.aiStep` also runs on the client: `processFlappingMovement` (the flap sound plays on the client through `onFlap`), `oFlapTime`/`flapTime`, `flightHistory.record(getY(), getYRot())` from the client's own interpolated body, then `doClientTick()`. While dying it spawns particles and returns. `tickDeath` and `tickCount` run on both sides.
- The renderer reads only client-side previous and current values with the client's own partial tick. Arrival timing never reaches the drawn pose.
- `DRAGON_FIREBALL` has `clientTrackingRange(4).updateInterval(10)`, and the vanilla client moves fireballs itself. Our fireballs are drawn like the dragon (`entity_view.rs:957`) and have the same back-jump.

### Plan
Draw the dragon (and its fireballs) from a client-side copy that ticks on the view's own clock, on the host and on the friend. Drawing changes and nothing else: no protocol or wire change, and the server state, hit boxes and rules stay.

1. **"The Minecraft dragon is drawn from its own client ticks, smooth for the host and friends"**
   - **New file `crates/minecraft_terrain/src/client_dragon.rs`** (`pub mod client_dragon;` next to `client_mobs` in `lib.rs`; rustfmt-clean, edition 2021).
     - `pub struct ClientDragons { bodies: crate::client_mobs::ClientMobs, dragons: Vec<ClientDragon> }`, with `Default`.
     - `struct ClientDragon { latest: DragonView, history: FlightHistory, flap_time, o_flap_time, step, tick_count, death_time, recorded }`.
     - `receive(&mut self, views: &[DragonView])`:
       - build one `ServerMob` per view: `id`, `position`, `y_rot`, `x_rot` 0, `y_head_rot` = `y_rot`, `on_ground` false, `eye_height` = `EYE_HEIGHT`, `max_head_y_rot` 75, `update_interval` 3, `dead` = `health <= 0`, `size` (16, 8), the rest default;
       - `self.bodies.receive(mobs)`, ignoring the poofs. This replays vanilla's tracker with 3-step interpolation, and snaps beyond 64 blocks as `ClientMob::receive` already does;
       - drop dragons whose id is gone; a new id starts with the view's flap, `tick_count`, `death_time` and an empty `FlightHistory` (vanilla starts empty; the first `record` fills it);
       - for every view: `latest = view.clone()`, `step = view.flap_time - view.o_flap_time`; resync `tick_count` or `death_time` when they differ by more than 2; with `step == 0`, hold `flap_time = o_flap_time = view.flap_time` (covers `no_ai`'s 0.5 and the dead dragon).
     - `tick(&mut self) -> Vec<(u64, DVec3, bool)>` (id, drawn position, flapped this tick): `self.bodies.tick()`, then per dragon `tick_count += 1`, `o_flap_time = flap_time`; while `health <= 0` count `death_time` and record nothing; otherwise `flap_time += step` and `history.record(body.position.y, wrap_degrees(body.y_rot))`. Report whether `cos(2π·flap)` rose through −0.3, the same test as `entities::dragon_flapping` (move that pure fn here as `pub fn flapping`, or duplicate it).
     - `views(&self) -> Vec<DragonView>`: position, previous position and yaw from the client body, the client flap, `tick_count` and `death_time`, the client history once recorded (else `latest.history`), and the host's health, `hurt_time`, sitting, phase, parts and `nearest_crystal`.
   - **`crates/minecraft_survival/src/entity_view.rs`** (minimal edits):
     - add `drawn_dragons: ClientDragons`;
     - in `receive` after `:207`, call `self.drawn_dragons.receive(&self.dragons)`. `self.dragons` stays the server view, because `shot_boxes`, `end_on_ray` (`entities.rs:779`), the lookups and the parts need it;
     - in `advance`, inside `if ticked`, call `self.drawn_dragons.tick()` and move the dragon voices there: the flap sound at the drawn position, and growls with the existing `growls` map from the drawn views. Remove the `dragon_voices()` call from `receive`. Vanilla plays both from the client's `aiStep`, so they now follow client ticks and are not lost to merged frames;
     - in `meshes`, pass `&self.drawn_dragons.views()` to `append_dragons`, `append_beams` and `append_rays` (`:809-818`);
     - add `pub fn drawn_dragons(&self) -> Vec<DragonView>` for tests.
   - Nothing changes in `minecraft_net`, `session.rs`, `mob_sender.rs` or `remote.rs`. The host's `Entities.view` and the friend's `RemoteLevel.view` are both `EntityView`, so one change fixes every window.
   - Do not change `client_mobs.rs`. Reuse it as it is.
2. **"Dragon fireballs fly smoothly for every player"**
   - A client copy of each `FireballView` in `EntityView`: it moves along its last host velocity once per client tick and snaps to the host position when it is more than a block off or a new id appears. Draw from it at `entity_view.rs:957`. Hit tests stay on the host's views.
   - This is how vanilla treats fireballs (`updateInterval` 10, client-side motion).

### Tests
In `client_dragon.rs` (`mod tests`, no game data). A helper moves a `DragonView` along +x at 1 block per tick with matching `previous_position` and `history`.
- `a_drawn_dragon_never_steps_back_when_frames_arrive_off_the_client_tick`: frames in an irregular pattern per client tick (`[1,0,2,1,1,0,2,1,1,1]`), sampled at partials 0, 0.25, 0.5 and 0.75. The drawn x never decreases and no step exceeds 1.5 blocks. Also show that the old method on the newest server view does go backwards, to document the bug.
- `the_drawn_dragon_reaches_a_stopped_server_position_within_a_few_ticks`: equal within 1/4096 after at most 5 client ticks.
- `the_flight_history_is_recorded_from_the_drawn_body_each_client_tick_and_not_while_dying`.
- `the_wings_beat_at_the_host_rate_between_frames`, including a step of 0 holding the host's flap.
- `a_dragon_missing_from_a_snapshot_is_forgotten_and_a_new_one_starts_at_its_position`.
- `a_dragon_more_than_64_blocks_away_snaps_instead_of_gliding`.
- `the_beam_and_ray_clocks_follow_client_ticks_and_resync_to_the_host`.

In `crates/minecraft_survival/src/remote.rs` tests (real 26.3 data, skipped without it, like `the_boss_bar_follows_the_host_and_the_dragons_bursts_show_in_the_level`): `the_friend_s_dragon_glides_on_its_own_ticks_when_frames_come_late`. Log in to the End, apply `StateFrame`s with a moving dragon in an irregular pattern, call `level.view.advance(0.0125, ...)` between them. The drawn x never decreases, and `level.view.shot_boxes(None)` still holds the host's parts of the newest frame.

For fireballs: `a_drawn_fireball_moves_every_client_tick_and_snaps_when_far_off`.

Keep green: `dragon_render::tests`, `entities::tests::a_dragon_flaps_when_its_wings_pass_the_top_of_their_beat` (adjust its path if the fn moves), `dragon_parts_are_flat_hit_boxes_keyed_by_dragon_and_part`, and the `session`, `mob_sender` and `remote` dragon tests.

### Owner's steps (idea)
Two windows: `& scripts\mp-local.ps1 -World mp-dragon -Fresh`.
1. Host: `/execute in minecraft:the_end run tp @a 0 70 0`. Both put on the diamond armour.
2. Both stand still near the middle and watch the dragon circle the pillars for about 30 s, in each window. The body, neck and tail glide without twitching, the wings beat evenly, the heal beam stays fixed to the dragon. Screenshot each window while it circles close by (the screenshot 5 case).
3. The friend holds the spyglass on the circling dragon for 10 s: smooth.
4. Host: `/tick freeze`. The dragon stops dead in both windows without shaking. `/tick unfreeze`: it flies on smoothly.
5. Shoot crystals until it perches. Watch the landing, the breath, the fireballs and the take-off from the friend window: no snapping, the fireballs fly smoothly. Shoot the dragon from both windows: both boss bars go down. Note it if shots near the drawn head miss while it flies fast.
6. Kill it. It rises smoothly with the light rays turning evenly in both windows.
7. Close the friend window. Host alone: `/summon minecraft:ender_dragon ~ ~20 ~`. It flies as smoothly as in step 2.
8. If the dragon still stutters, look for `Can't keep up!` in `latest.log`. That would be the End level itself falling behind, which this fix does not address.

```
IW4L_MINECRAFT_KIT=minecraft:bow,minecraft:arrow*64,minecraft:spyglass,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*16,minecraft:cooked_beef*32,minecraft:cobblestone*64,minecraft:water_bucket,minecraft:ender_pearl*16,minecraft:snowball*16
```

---

## A4. Worn mob heads and carved pumpkins, and the pumpkin overlay

### The owner's finding
Screenshots 7 and 8 (`7_drachenkopf_im_slot.png`, `8_drachenkopf_nicht_sichtbar.png`). A dragon head in the helmet slot is not drawn on the MW2 body (the elytra on the back is). The same holds for every mob head, the player head and the carved pumpkin, in F5, in the inventory preview and on other players, for the host and the friend. The pumpkin blur overlay is also missing in first person.

### Cause
Head-slot items are only drawn when they are armour. Nothing in the fork works like vanilla's `CustomHeadLayer` or the camera-overlay part of `Hud.extractCameraOverlays`. `docs/MINECRAFT-SURVIVAL.md:406` already lists it as a limitation.

- **The gear already reaches every window.** Slot 39 is read into `armor[0]` (`minecraft_terrain/src/body_gear.rs:4`, `:39-56`): for the host's body at `render_anim/src/minecraft_world.rs:1480`, for friends on the host at `minecraft_host.rs:477`, and on a friend's window from the replicated `GearView` (`minecraft_net/src/messages.rs:228-231`, `:501-506`; `minecraft_remote.rs:461-466`). No net change is needed.
- **Drawing.** `gear_rigs` builds a `BodyRig` (`anim/minecraft_gear.rs:474-532`) whose head frame (`:393-402`) maps vanilla head-model pixels to world blocks. `draw_body_gear` (`minecraft_world.rs:2140-2165`) calls `append_body_gear` (`minecraft_terrain/src/body_render.rs:40-71`) for world bodies and the inventory puppet, on host and friend.
- **The gap.** `append_body_gear` draws armour (`append_armor_on_rig`), the elytra and the hand items. For slot 0, `append_armor_on_rig` (`armor_render.rs:216-233`) draws only the layers from `armor_layers` (`:206-214`), which come from `armor_asset` (`:22-46`), which knows only helmets and the turtle helmet. So every skull and the carved pumpkin draw nothing.
- **Missing pieces.**
  - The skull geometry exists in `special_models.rs:845-910` (`push_head`), but its animation is fixed at 0: the dragon jaw at `:876` uses `0.0f32.sin()` and the piglin ears at `:890` use `0.0f32.cos()`.
  - `HeldDisplay` (`mesh.rs:1300-1307`) and `ItemVisual` (`mesh.rs:871-882`) have no HEAD display context.
  - `MinecraftUi` (`frame/src/minecraft_ui.rs:210-211`) has `freeze_outline` and `spyglass_scope` but no camera overlay. `hud/src/minecraft_spyglass.rs:73-100` draws only the scope.

### Vanilla 26.3
**Which path a head-slot item takes** (`LivingEntityRenderer.extractRenderState`):
- A `BlockItem` whose block is an `AbstractSkullBlock`: `wornHeadType` is that skull type, with the `PROFILE` component.
- Otherwise, if `!HumanoidArmorLayer.shouldRender(stack, HEAD)`: `headItem` with `ItemDisplayContext.HEAD`. This is the carved pumpkin's path.
- Otherwise (helmets) the armour layer draws it.
- `wornHeadAnimationPos` is the walk animation position; when riding a living entity, the vehicle's.

**`CustomHeadLayer.submit`** (default transforms):
- `root.translateAndRotate`, then `translateToHead`: head-part space in blocks, y down, face at −z.
- Skull: `scale(1.1875)` (`SKULL_SCALE`), then `SkullBlockRenderer.submitSkull(animationPos, ...)`. No foil. A player head without a profile uses the default skin.
- Other items: `translate(0, −0.25, 0)`, `rotY(180)`, `scale(0.625, −0.625, −0.625)`, then the item with its HEAD display transform and foil. `block/block.json` has no `head` display, so the carved pumpkin is a 10 px pumpkin centred on the head, face forward.

**Model animation:**
- `DragonHeadModel`: `jaw.xRot = (sin(pos·π·0.2) + 1) · 0.2`.
- `PiglinHeadModel`: `leftEar.zRot = −(cos(pos·π·0.2·1.2) + 2.5) · 0.2`, `rightEar.zRot = (cos(pos·π·0.2) + 2.5) · 0.2`.
- `SkullModel` (skeleton, wither skeleton, zombie, creeper, player): no walk animation. Zombie and player have the 0.25 hat layer.

**Walk animation** (`WalkAnimationState`, run every client tick by `calculateEntityAnimation`): horizontal distance only; `target = min(dist·4, 1)`, `speed += (target − speed)·0.4`, `position += speed`. It stops while riding or dead.

**Camera overlay** (`Hud.extractCameraOverlays`, skipped when the HUD is hidden):
- Only in first person. While scoping, only the spyglass is drawn.
- Otherwise, for each equipment slot whose item's `EQUIPPABLE` has a `cameraOverlay`, blit `textures/<overlay>.png` over the whole GUI at alpha 1.
- `Items.CARVED_PUMPKIN` is the only 26.3 item with one: `misc/pumpkinblur`, 256x256, with `"blur": true` (linear filtering).

### Plan
Client-side presentation only. Every window draws every presented player's head from the `GearView` it already has, and computes each body's walk animation from that player's presented `PlayerState`, as vanilla computes it on the client for remote players. No protocol, server or save change.

1. **"Worn mob heads and carved pumpkins are drawn on the MW2 body"** (`minecraft_terrain`)
   - **`special_models.rs`:**
     - `push_head` gets an `animation: f32` parameter. The dragon jaw becomes `((animation·π·0.2).sin() + 1.0)·0.2`; the piglin ears follow the vanilla formulas above. The existing callers (`block_model` `:291`, `item_model` `:394`/`:396`) pass 0.0, so placed and held heads stay exactly as now.
     - `pub(crate) fn worn_skull(item_id) -> Option<Head>` for skeleton_skull, wither_skeleton_skull, zombie_head, creeper_head, dragon_head, piglin_head and player_head.
     - `pub(crate) fn worn_head_model(head, animation) -> Result<ResolvedModel>`: `push_head` with pose `Mat4::from_scale(Vec3::splat(SKULL_SCALE))`.
   - **`mesh.rs`:**
     - Move the element loop of `append_model` (`:1265-1292`) into `pub(crate) fn append_elements(...)`, called by `append_model`. Change only those lines.
     - Add `HeldDisplay::Head` and an `ItemVisual.head` field from `item_display_transform(packs, &resource, "head")` (in `model()` and the frame literal at `:1228`), plus the `HeldDisplay::Head => visual.head` arm at `:1174`.
   - **New file `crates/minecraft_terrain/src/worn_head.rs`** (`pub mod worn_head;`, rustfmt-clean, edition 2021):
     - `pub const SKULL_SCALE: f32 = 1.1875;` and `const ITEM_SCALE: f32 = 0.625;`
     - `pub fn skull_pose(head: Mat4) -> Mat4 { head * Mat4::from_scale(Vec3::splat(16.0)) }` (`rig.head` maps pixels, so ×16 turns the elements' px/16 units back into pixels).
     - `pub fn item_pose(head: Mat4) -> Mat4`: `head · S(16) · T(0, −0.25, 0) · Ry(π) · S(0.625, −0.625, −0.625)`.
     - `pub fn append_worn_head(mesh, items, head: Option<Mat4>, item: Option<&GearItem>, animation, light, packs, atlas, sky_light, sky, block) -> Result<()>`: return without an item or head frame, or when `armor_asset(&item.id)` is Some. A skull draws `worn_head_model` at `skull_pose` with no glint. Anything else goes through `append_held_items` with `HeldDisplay::Head` at `item_pose`, with glint when the item has it.
     - The head and entity textures are already in the atlas (`special_models::textures()` at `mesh.rs:483`, on host and friend).
   - **`body_render.rs`:** add `pub head_swing: f32` to `BodyRig` (default 0). In `append_body_gear`, after `append_armor_on_rig` (`:54`), call `append_worn_head(..., gear.armor[0].as_ref(), rig.head_swing, ...)`.
2. **"A worn dragon head opens its jaw and a piglin head flaps its ears while walking"** (`render_anim`)
   - In `anim/minecraft_glide.rs`, extend `BodyGlide` with `walk: minecraft_terrain::walk_animation::WalkAnimation` and `walk_clock: f32`. Add `alive: bool` to `GlideInput` (from `ps.pm_type < PM_TYPE_DEAD`).
   - In `BodyGlide::advance`, run a 20 Hz accumulator (at most 10 steps a frame): each whole tick calls `walk.update(hypot(motion x, motion y), false)`, or `walk.stop()` when dead.
   - `walk_position()` on `BodyGlide` and `McGlide::walk_position(client)` (0 for an unknown client).
   - In `gear_rigs`, set `head_swing: glide.walk_position(body.client)` next to `wings`.
   - This covers remote players, the own F5 body and the inventory puppet (vanilla's inventory doll also shows the live walk). `step_body_glides` already runs on host and friend for every presented player.
3. **"A worn carved pumpkin blurs the first-person view like vanilla"**
   - `frame/src/minecraft_ui.rs`: add `pub pumpkin_blur: Option<Handle<Image>>` and `pub camera_overlay: bool` to `MinecraftUi`. Add fields only; track B also edits this file.
   - `minecraft_terrain/src/body_gear.rs`: `pub fn camera_overlay(slots) -> bool`, true when slot 39 holds `minecraft:carved_pumpkin`.
   - `minecraft_portal.rs`: `PUMPKIN_BLUR = "minecraft:misc/pumpkinblur"` and a `blurred(...)` variant of `strip` with `ImageSampler::linear()`.
   - Host (`minecraft_world.rs`): load it next to `spyglass_scope` (`:634`) and set `ui.camera_overlay = alive && body_gear::camera_overlay(&inventory.slots)` next to `:1390`.
   - Friend (`minecraft_remote.rs`): load it next to `:410`, and set the same flag from the friend's replicated inventory next to `:1063`.
   - `hud/src/minecraft_spyglass.rs` `update`: keep the scope branch. When not scoping, but `ui.active && first_person && ui.camera_overlay` and the texture exists, insert `mc_pumpkin_blur` and emit one full-surface quad (st 0..1, white, alpha 1) from a pure `camera_overlay_quad(size)`. Make the choice a pure `overlay_job(active, scoping, first_person, wearing, has_texture)`.
   - If the HUD binds its own nearest sampler for runtime images, the blur will look blocky. Then pass the sampler through `HudImages`.

Item A2 changes `hand_frame` in `minecraft_gear.rs`. This item only adds a `BodyRig` field and one call, so it goes after A2 without conflict.

Seen on the side, not fixed here: a friend's window never sets `ui.freeze_outline` (only `minecraft_world.rs:633` does), so the powder-snow outline is probably missing for friends. Friend overlays belong to track D's friend limits; note it in the track log for track D.

### Tests
**`minecraft_terrain`:**
- `special_models.rs`:
  - `worn_skulls_are_the_seven_heads_and_not_pumpkins_or_helmets`.
  - `the_dragon_jaw_follows_the_walk_and_rests_where_the_block_does`: at animation 0 the jaw equals the placed block's (0.2 rad); 0.4 rad at 2.5; 0 at 7.5.
  - `piglin_ears_flap_at_their_own_rates`: ±0.7 rad at 0; at 5.0 the right ear is 0.3 and the left −(cos(1.2π) + 2.5)·0.2.
- `worn_head.rs`:
  - `a_worn_skull_is_the_head_cube_scaled_like_vanilla`: 6 faces with a span of 9.5/16, uvs inside the skeleton sheet's region.
  - `a_worn_dragon_head_points_its_snout_forward`: minimum head-space z below −15 px, and the jaw is present.
  - `a_worn_carved_pumpkin_is_its_block_model_centred_on_the_head`: 6 faces of 10/16, centred at head pixel (0, −4, 0), the carved face at the lowest z. Glint only when the item has it.
  - `a_helmet_is_left_to_the_armour_layer`.
- `body_render.rs`: `worn_heads_are_drawn_with_the_body_and_lit_where_it_is` (a zombie head gives 12 faces with the rig's light and no armour glint).
- `mesh.rs`: `the_head_display_is_identity_for_blocks_without_one`.
- `body_gear.rs`: `only_a_carved_pumpkin_on_the_head_blurs_the_view`.

**`render_anim`:**
- `minecraft_glide.rs`: `the_walk_swing_advances_once_per_tick_and_eases_like_vanilla` (one 0.05 s step at motion [0.3, 0, 0] gives speed 0.4 and position 0.4; two half steps give the same; vertical motion alone does not swing; dead stops at 0).
- `minecraft_gear.rs`: `gear_rigs_take_each_clients_walk_swing`.

**`hud`:** `the_pumpkin_blur_covers_the_screen_only_in_first_person_and_not_while_scoping`.

### Owner's steps (idea)
Alone (host):
1. Inventory (E): drag the dragon head into the helmet slot. The preview soldier wears it with the snout forward (the screenshot 7 case).
2. F5: the dragon head sits on the soldier's head (the screenshot 8 case). Walk: the lower jaw opens and closes about twice a second; stop and it holds.
3. Piglin head: the ears flap while walking, at slightly different rates.
4. The skeleton, wither skeleton, zombie, creeper and player heads: each sits on the head, a bit bigger than it. Send a screenshot if the MW2 helmet pokes through.
5. Carved pumpkin: in first person the orange blur covers the screen edges, smooth, not pixelated. F5: no blur, and the pumpkin with its face forward sits on the head. Back in first person, the spyglass replaces the blur while scoping.
6. Diamond helmet: as before.

Two windows:
7. The host wears the dragon head and walks. The friend sees the jaw moving.
8. The friend wears the carved pumpkin: the friend has the blur, their inventory preview and F5 show the pumpkin, the host sees it on the friend's body and has no blur.
9. The friend wears the piglin head in the Nether: the host sees the ears flap.

```
IW4L_MINECRAFT_KIT=minecraft:dragon_head,minecraft:piglin_head,minecraft:skeleton_skull,minecraft:wither_skeleton_skull,minecraft:zombie_head,minecraft:creeper_head,minecraft:player_head,minecraft:carved_pumpkin,minecraft:diamond_helmet,minecraft:spyglass
```

---

## A5. Four end crystals "in the wrong spots" still brought a new dragon

### The owner's finding
Screenshot 9 (`9_endkristalle_falsch_platziert.png`). The owner placed four end crystals on the exit portal, in spots they thought were wrong, and a new dragon still came. The owner wants the respawn only with crystals on the vanilla spots.

### Cause
No defect was found. Our trigger does the same check as vanilla 26.3, and vanilla also accepts crystals that are a block off the side centres.

- `crates/minecraft_terrain/src/server.rs:1772-1777`: placing an end crystal calls `try_respawn`, as `EndCrystalItem.useOn` does.
- `server.rs:2044-2071` `try_respawn`: returns unless the dragon is dead and no respawn runs; takes the exit portal location; for each of `crystal_cells(location)` collects `crystals.in_cell(cell)`; returns if any cell has none; otherwise starts `Respawn::new(crystals)`.
- `dragon_respawn.rs:122-130` `crystal_cells`: (x, y+1, z−3), (x+3, y+1, z), (x, y+1, z+3), (x−3, y+1, z).
- `end_crystal.rs:419-428` `in_cell`: every crystal whose 2x2x2 `pick_box` (`:150-156`, `WIDTH`/`HEIGHT` 2.0 at `:8-9`) meets the 1x1x1 cell box. The test is strict (`ender_dragon.rs:876-883` `boxes_meet`).
- The portal location matches vanilla (`third_party/minecraftoss/world/src/level/end_portal.rs:146-201`, `generator/src/feature/end_portal.rs:173`).

What follows, in vanilla and in ours:
- A crystal stands at the top centre of the block it was placed on, so its box spans one block on each side of that block's centre.
- The rim has 16 bedrock blocks: three straight on each side and four diagonals at (±2, ±2).
- **Any of the three straight blocks on a side counts.** A crystal on (1, y, 3) has a box from x 0.5 to 2.5, which meets the cell from 0 to 1.
- **Only the four diagonal blocks fail.** Their box runs from 1.5 to 3.5 on both axes.
- A crystal raised one block does not count. One block lower, just outside the rim, does.

In screenshot 9 (seen roughly from above), three crystals sit about on the side axes and the bottom one about a block off, which is still a straight-edge block. The beams show the respawn really started through `try_respawn`, not the 1200-tick dragon search (`dragon_fight.rs:381-396`).

Multiplayer: only the host checks. Every player's item use reaches the End level's `ServerSim::item_on_block` (`actions.rs:1110`, `entities.rs:805`).

### Vanilla 26.3
- `EnderDragonFight.tryRespawn()` (the class is `EnderDragonFight`): `if (dragonKilled && respawnStage == null)`; portal = `exitPortalLocation`, or `findExitPortal()`, or `spawnExitPortal(true)`; `centre = portal.above(1)`; for each horizontal direction, `getEntitiesOfClass(EndCrystal.class, new AABB(centre.relative(dir, 3)))`; return if empty; then `respawnDragon(all)`.
- `AABB(BlockPos)` is the 1x1x1 block; `AABB.intersects` is strict; `END_CRYSTAL` is `sized(2.0F, 2.0F)`.

### Plan
Do not change the trigger. Accepting only the exact centre blocks would break with vanilla. Pin the vanilla rule with tests and record the decision.

1. **"Minecraft dragon respawn tests pin vanilla's crystal spots on the exit portal rim"** (test only)
   - `crates/minecraft_terrain/src/end_crystal.rs` tests: `a_crystal_counts_for_a_cell_its_two_block_box_touches`. With the helpers the existing tests use (`fresh_tag(placed_at(base), false)`):
     - on base (1, 62, 3): in `in_cell((0, 63, 3))`;
     - on base (0, 62, 3): in it;
     - on base (2, 62, 2): in neither (0, 63, 3) nor (3, 63, 0);
     - on base (0, 63, 3), standing at y 64: not in (0, 63, 3);
     - on base (0, 61, 4), standing at y 62 just outside: in (0, 63, 3).
   - `crates/minecraft_terrain/src/server.rs` tests, next to `four_crystals_on_the_exit_portal_rebuild_the_pillars_and_summon_a_new_dragon`: `only_crystals_touching_the_four_side_cells_of_the_exit_portal_start_a_respawn`.
     - Start with `let Some((mut server, at)) = respawn_world() else { return };`.
     - Crystals on the four diagonal rim blocks (assert each is bedrock first): `server.respawn` stays None and `take_fight_reports()` is empty.
     - Then crystals on the side-end blocks (1, −3), (3, 1), (−1, 3), (−3, −1): a single `FightReport::Respawning` with exactly 4 crystals (the diagonal ones are not included).
     - Optional third case in a fresh world: a crystal on obsidian on the rim centre (standing at y+2) gives no respawn.
     - Keep the file's style. Do not rustfmt `server.rs`.
   - Record the decision in the track log.
2. **Only if** the owner's test below shows four diagonal crystals still triggering a respawn: log `at`, the four cells and each crystal's block in `try_respawn`, and check whether `fight.exit_portal` disagrees with the visible podium (for example a stale saved location). That would be a later fix, not part of this run.

If the owner later wants the exact centres anyway, the change is one line in `in_cell` (compare `crystal.block_position() == cell`), logged as not vanilla.

### Tests
See the plan. Run `test -p minecraft_terrain --locked end_crystal` and `... respawn` (the `respawn_world` tests need `MINECRAFTOSS_ROOT`, which `cargo-slot.ps1` sets).

### Owner's steps (idea)
Needs a dead dragon (kill the respawned one first).
1. `/gamemode creative`, fly over the exit portal and look straight down. The rim has three straight blocks on each side and one diagonal block at each corner.
2. One crystal on each of the four diagonal corner blocks. `/gamemode survival`, wait 10 s: no beams, no new dragon. Screenshot from above.
3. One crystal on an end block of each straight side, one block beside the middle. Right after the fourth: the portal goes out, beams rise, the pillars are rebuilt, and after about 25 s the new dragon and its bar appear. Vanilla 26.3 does the same, because the crystal's 2x2 box touches the middle block's column.
4. Multiplayer: the friend places the four side crystals instead. Both windows see the beams and the new dragon.
5. If a corner-only set in step 2 does start a respawn, press F3 and note the coordinates of each crystal's block and of the portal pillar. Screenshot.

```
IW4L_MINECRAFT_KIT=minecraft:end_crystal*16,minecraft:obsidian*8,minecraft:diamond_sword+sharpness=5,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots+feather_falling=4,minecraft:golden_apple*16,minecraft:cobblestone*64,minecraft:elytra,minecraft:firework_rocket*64
```

---

## A6. The creative inventory for a friend in creative mode

### The owner's finding
Reported in text, no screenshot. A friend in creative mode gets the survival inventory instead of the creative inventory with tabs and search. The host gets the creative screen because of its game mode, not because it is op. Round 8 deferred this on purpose (`docs/MULTIPLAYER.md:660`, `:1330`).

### Cause
The host's creative path depends only on the game mode. The friend's game never sets up the creative screen, and `minecraft_net` has no message for a creative slot set.

1. **Host (correct).** `render_anim/src/minecraft_world.rs:1274-1280` calls `inventory_ui.creative.prepare(!player_mode.consumes_items(), operator)`. `consumes_items()` is `!abilities.instabuild` (`minecraft_terrain/src/game_mode.rs:127`). `operator` (command level 2 plus the `minecraft_operator_items` setting) only shows or hides the Operator Utilities tab. After that come `set_item_tags`, `creative.apply` and `creative.hotbar_key`.
2. **Friend.** `show_player` (`render_anim/src/minecraft_remote.rs:953-1086`) never calls `prepare`, `set_item_tags`, `apply` or `hotbar_key`. So in `CreativeUi` (`render_anim/src/minecraft_creative.rs`) the tab loader never starts (`:260-277`), `refresh` returns (`:312-314`) and `screen()` returns None (`:290-297`). The HUD's open logic (`hud/src/minecraft_inventory.rs:254-260`) falls back to `McScreen::Inventory`, although `ui.game_mode` is Creative (`minecraft_remote.rs:996`).
3. `show_menus` (`minecraft_remote.rs:1141-1143`) also forces any `McScreen::Creative(_)` back to `Inventory` while no station menu is open.
4. **No transport.** `menu_slot`/`action_of` (`render_anim/src/minecraft_inventory.rs:587-610`) return None for `McSlot::Creative`, `McSlot::Trash` and `McClick::Clone`. `ToServer` (`minecraft_net/src/messages.rs:33-57`) has nothing like `ServerboundSetCreativeModeSlotPacket`, and neither do `read_inbox` (`minecraft_host.rs:101-178`) and `MenuInput` (`minecraft_survival/src/menus.rs:162-166`).
5. **No item tags on the friend** for `#tag` search. The host gets them from `commands::command_data` (`commands.rs:895-897`, `minecraft_terrain/src/locate.rs:50`). The friend's `Prepared` (`minecraft_remote.rs:40-70`) has `registries.datapack` but no tags.
6. On the friend, C/X plus a digit sets `ui.creative.hotbar_key` (`hud/src/minecraft_inventory.rs:297-300`), and nothing consumes it.

The server-side check already exists: `minecraft_survival::creative::set_slot` (`creative.rs:73-114`: NotCreative, BadSlot, UnknownItem, TooMany).

### Vanilla 26.3
- `InventoryScreen` swaps itself for `CreativeModeInventoryScreen` when `hasInfiniteMaterials()`, which is `abilities.instabuild`. The game mode alone decides.
- The Operator Utilities tab needs `canUseGameMasterBlocks()` (instabuild and permission level 2) plus the operator-items option.
- Every creative slot change is sent as `ServerboundSetCreativeModeSlotPacket(slot, stack)`; a drop uses slot −1.
- `handleSetCreativeModeSlot`: nothing unless `hasInfiniteMaterials()`; return if the item is not enabled; `validSlot` 1..=45; `validData` empty or count ≤ max stack size; a valid set goes through `setByPlayer`, `setRemoteSlot` and `broadcastChanges`; a drop (slot < 0, valid data) is thrown through `dropSpamThrottler` (`TickThrottler(20, 1480)`: +20 a drop, −1 a tick; above 1480 it warns "Player {} was dropping items too fast in creative mode, ignoring.").
- No op check on the server. The creative cursor stays on the client in vanilla. The fork keeps the cursor on the server (`inventory.cursor`, `CreativeSlot::Carried`), so the wire needs a Carried target too.

### Plan
A friend whose `abilities.instabuild` is true gets the host's creative screen: tabs from the jar, search with names and `#tags`, the inventory tab with trash, middle-click clone, and C/X+digit saved hotbars in the friend's own `iw4l-artifacts\hotbar.nbt`. The friend applies nothing locally. It sends vanilla-style creative slot sets, and the host checks each one with `creative::set_slot` on that friend's `Player`.

1. **"Friends' creative slot changes reach the host and are checked like vanilla"**
   - **`minecraft_net/src/messages.rs`:** `pub enum CreativeTarget { Menu(u8), Carried, Drop }` (Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize), and `ToServer::CreativeSlot { slot: CreativeTarget, stack: Option<Nbt> }` appended at the end of `ToServer`. Bump `PROTOCOL` from 3 to 4 (once for the whole track).
   - **`codec.rs` tests:** `ToServer::CreativeSlot { .. } => 12` in `server_index`, `seen = [false; 13]`, and samples for Menu(36) with a stack, Carried with None and Drop with a stack.
   - **`menus.rs`:** `MenuInput::Creative { slot, stack }`, in the same queue as the friend's clicks, so a creative set and a click in one host tick apply in the order sent. `take_inputs` gains `known: &dyn Fn(&str) -> bool`. For Creative:
     - map Menu(n) to `CreativeSlot::Menu(usize::from(n))`, Carried and Drop to theirs;
     - decode with `views::view_stack(&nbt, &player.inventory.recipes)`; an Nbt that does not decode is ignored, never turned into a clear;
     - ignore it when the player is dead;
     - call `creative::set_slot(&player.mode, &mut player.inventory, slot, stack, known)`; `Ok(Some(drop))` goes to `clicked.thrown`; errors are ignored like vanilla (a debug log is fine).
   - **`friend_menus`** (`render_anim/src/minecraft_players.rs:1815-1870`) passes `known` from `world.globals.chat.registries().items`. If that borrow clashes with the mutable `entities` borrow, build the known set before `level` is borrowed. Thrown items are already tossed from the friend's eye.
   - **`minecraft_host.rs` `read_inbox`:** `Ok(ToServer::CreativeSlot { slot, stack })` pushes `MenuInput::Creative` to that friend's session. `MENU_INPUTS = 256` already caps the queue.
   - Replication back needs no new code: slots 0-40 go out in `PlayerView.slots`, the cursor and 2x2 grid in the inventory menu's `MenuSlots` (`menus::changes`).
2. **"A friend in creative mode gets the creative inventory with tabs and search"**
   - **`minecraft_creative.rs`:** split `CreativeUi::apply` into `pub(crate) fn sets(&mut self, ui, inventory) -> Vec<SetSlot>` (takes the creative clicks out of `ui.clicks` exactly as today, leaves the rest, resets the screen to Inventory when not creative) and `apply`, which calls `sets` and then `creative::apply`, so the host behaves as before. Split `hotbar_key` the same way into `hotbar_sets` (load sets out, save stays local) and `hotbar_key`. Keep the split minimal: track C changes the creative tabs in this file.
   - **`minecraft_terrain/src/locate.rs`:** `pub fn datapack_tags(pack, registry) -> BTreeMap<String, Vec<String>>` (the current body); `tags(gen, r)` calls it.
   - **Friend `Prepared`:** `item_tags`, built in `prepare()` with `datapack_tags(&registries.datapack, "item")` on the loader thread. The registry fingerprint check makes them match the host's.
   - **`show_player`:** build the mirrored `Inventory` before `show_menus` (slots from `view.slots`, the cursor from `(remote.menu or remote.inventory_menu).carried`, the crafting grid), independent of `alive`. Then:
     - `let creative = view.abilities.instabuild;`
     - `inventory_ui.creative.prepare(creative, remote.command_level >= 2 && operator_items)`;
     - `inventory_ui.creative.set_item_tags(&prepared.item_tags)`;
     - for every set from `sets` and `hotbar_sets`, push `creative_message(slot, stack)`, a new pure fn to `ToServer::CreativeSlot` that returns None when a Some stack fails `slot_nbt`, so it never sends a clear by mistake.
     - Reuse the same inventory value further down for `publish`.
   - **`show_menus`:** `:1141` becomes `if menu.is_none() && !creative && matches!(ui.screen, McScreen::Creative(_))`, or drop it, since `sets` already resets the screen.
   - Add `Option<Res<frame::GameSettings>>` to a friend system and pass `minecraft_operator_items` into `show_player`.
   - Ordinary inventory clicks inside the creative screen still go through `show_menus`/`action_of` as `ToServer::Click` on the inventory menu.
   - The friend's screen does not predict: cursor and slots show the host's state one round trip later, as in every friend menu (`docs/MULTIPLAYER.md` 10.2).
3. **"Creative drops are throttled like vanilla"**
   - `creative::DropThrottle { count: u32 }`: +20 a drop, −1 a tick, refuse above 1480. A non-saved field on `Player`, ticked in the per-player tick and checked before a Drop set, on the friend path and the host path alike.

Out of scope: world pick-block (middle-click a block in the world) exists for nobody yet.

Track C will make guns real items, which changes `known` and the creative tabs. Today an `iw4:weapon/` stack is refused by `known` for host and friend alike. Note in the track log that the friend path uses the same `known` closure in `friend_menus`.

### Tests
- **`minecraft_net`** `every_message_to_the_server_round_trips`: CreativeSlot with Menu, Carried and Drop, with and without a stack.
- **`minecraft_survival` `menus.rs`:**
  - `a_creative_friends_slot_set_reaches_their_inventory`: Menu(36) stone ×64 lands in slot 0; Menu(5) diamond_helmet sets slot 39; after a Carried set, `changes()` sends the new carried.
  - `a_survival_or_spectator_friends_creative_sets_are_refused`.
  - `creative_sets_refuse_unknown_items_bad_slots_and_overfull_stacks`: an unknown id, Menu(0), Menu(46), 65 stone, 2 swords, and an `iw4:weapon/…` stack refused by `known`.
  - `a_creative_drop_is_thrown_not_stored`.
  - `creative_sets_and_clicks_apply_in_the_order_sent`: Carried torch ×16, then a click on inventory slot 3, puts the torch in slot 3 with an empty cursor.
  - `a_dead_friends_creative_set_is_ignored`.
  - `creative_drops_are_throttled_like_vanilla`: 74 drops pass, the 75th is ignored, 20 ticks later one more passes.
- **`render_anim` `minecraft_creative.rs`:**
  - `sets_take_creative_clicks_without_touching_the_inventory`.
  - `hotbar_sets_load_nine_slots_and_save_locally`.
  - `a_loaded_creative_ui_turns_the_inventory_screen_into_the_creative_screen`.
  - The existing `losing_creative_turns_the_creative_screen_into_the_inventory` still passes.
- **`render_anim` `minecraft_remote.rs`:** `creative_message_maps_targets_and_never_sends_a_clear_for_an_unencodable_stack`.
- **`minecraft_terrain` `locate.rs`:** `datapack_tags` gives the same map as `tags(gen)`, and `minecraft:logs` contains `minecraft:oak_log` when the data is present.

### Owner's steps (idea)
Two windows.
1. Host: `/gamemode creative Friend` (the friend is not op). Friend: E opens the creative tabs, with no Operator Utilities tab. Screenshot.
2. Friend: Search tab, type `oak`, then `#logs`: only logs. Screenshot.
3. Friend: take an oak log to hotbar slot 1, shift-click a diamond sword into slot 2, close. The host sees the friend holding them.
4. Friend: Q and Ctrl+Q over cobblestone in the grid: the drops appear in both windows.
5. Friend: Survival Inventory tab: a diamond helmet on the head (the host sees it), a stack onto the trash slot, shift-click the trash slot to clear.
6. Friend: middle-click a grid item and an inventory stack: a full stack goes to the cursor.
7. Friend: C+1 saves the hotbar, clear it, X+1 restores it; the Saved Hotbars tab shows it.
8. Host: `/gamemode survival Friend` while the screen is open: it turns into the survival screen. Creative again: the creative screen again.
9. Host: `/op Friend`. Friend: `minecraft_operator_items=true` in their `settings.cfg`, restart: the Operator Utilities tab appears. `/deop Friend`: gone on the next open.
10. The host's own creative screen is unchanged.

```
IW4L_MINECRAFT_KIT=minecraft:cobblestone*64,minecraft:diamond_sword,minecraft:oak_log*16
```

---

## A7. Players get stuck inside each other after a spawn or a teleport

### The owner's finding
Reported in text, no screenshot. After spawning or teleporting, players can end up inside each other and cannot move.

### Cause
Two causes that add up.

**1. Players are hard MW2 capsules to each other's pmove, and an overlap traps both.**
- `alive_body_clips` (`crates/sim/src/step.rs:1806-1833`) collects every alive, non-noclip client and, during prediction, every `prediction_remote_bodies` entry. It feeds `ClipBackend` (`:381-394`), whose `trace` ends in `clip_move_to_players` (`:1880`, `:2093-2131`): a 30x70-unit MW2 capsule with `CONTENTS_BODY`.
- A mover that starts inside another capsule gets fraction 0 with startsolid, and allsolid for any horizontal move (`crates/clipmap_iw4/src/temp_box.rs:311-325`).
- `movement_iw4/src/slide.rs:88-91` then does not move. `ground.rs:36-43` calls `correct_solid`, which only tries ±1-unit nudges (`correct_solid.rs:5-32`), while the overlap is up to 30 units. Both players are frozen.
- MW2's own fix, `phase_stuck_in_client` (`step.rs:164-195`, run at `:541-543`), sets a 25 u/s eject plus 300 ms of `TIME_HARDLANDING` (`gamemode_iw4/src/stuck_in_client.rs:85-93`), but the allsolid traces never let it move anyone.

**2. Every spawn without a bed uses the same exact point.**
- `TerrainStream.player_spawn` is drawn once when the level opens (`crates/minecraft_terrain/src/terrain.rs:1058-1061`) and reused for every bedless respawn (`minecraft_survival/src/respawn.rs:49-57`, `:76-80`), for an arrival with no place (`render_anim/src/minecraft_players.rs:149-157`) and for a new friend (`minecraft_survival/src/server.rs:670-676`, `:700-704`, `:871`).
- The host's first MW2 spawn lands there too (`minecraft_world.rs:624`, `sim/src/script_player.rs:427-440`).
- `place_friend`/`place_player` (`minecraft_world.rs:1937-1970`) teleport exactly to the arrival, overriding MW2's free-column search (`voxel::spawn_clear_of`, `voxel.rs:1229-1235`).
- So a new friend appears inside the host standing at spawn, and two bedless respawns appear inside each other. Each case then hits cause 1.
- A per-call port of vanilla's `PlayerSpawnFinder.findSpawn` exists but is never called: `TerrainStream::respawn_position` (`terrain.rs:1177-1180`).

### Vanilla 26.3
**Pushing.**
- Players are never solid to each other (`canBeCollidedWith` is false). They push each other with `Entity.push` from `LivingEntity.pushEntities`.
- `Entity.push(other)`: nothing for passengers of the same vehicle or `noPhysics`; `dx`, `dz` to the other; `d = absMax(dx, dz)`; nothing below 0.01; otherwise `d = sqrt(d)`, `dx /= d`, `dz /= d`, `pow = min(1, 1/d)`, both times `pow · 0.05`; push self by (−dx, 0, −dz) and the other by (dx, 0, dz).
- `LivingEntity.isPushable`: alive, not spectator, not on a climbable. `LivingEntity.push` does nothing while sleeping.
- `EntitySelector.pushableBy`: on the client only the local player can be pushed. So each client pushes its own player away from overlapping players, once per tick, by at most 0.05 blocks/tick of motion. Ground friction (0.6 × 0.91 = 0.546 per tick) turns a steady push p into about 2.2·p blocks a tick.

**Spawning.**
- A new player without a saved position, and a bedless respawn, both go through `PlayerSpawnFinder.findSpawn`: radius from the game rule `respawn_radius` (default 10), up to 1024 candidates in a coprime walk from a fresh random offset per spawn, each checked with `getLevelRespawnPos` and `noCollisionNoLiquid` (other players don't count), with `fixupSpawnHeight` as the fallback. ADVENTURE uses `fixupSpawnHeight` only.
- A saved position is used exactly. `/tp` is exact. Overlaps are left to the push.

The vendored `third_party/minecraftoss/world/src/spawn.rs:99-133` (`player_spawn`) matches this, offset included, and `game_rules.rs:154` has `respawn_radius`.

### Plan
Three commits, in this order. Every change is gated on Minecraft worlds (`crate::voxel::active()`), so MW2 maps keep MW2 player collision.

1. **"Players in Minecraft worlds pass through each other instead of sticking"** (`crates/sim/src/step.rs`)
   - Split `alive_body_clips` into `fn player_body_clips(world: &FrameWorld, block_world: bool)`, empty when `block_world`, and have `alive_body_clips` call it with `crate::voxel::active()`. This covers pmove (`:381`), the script slide (`:2012`) and `script_mantle` (`:2057`).
   - Gate MW2 StuckInClient: `:541` becomes `if allow_move && world.publishes_snapshot() && !crate::voxel::active()`.
   - Leave the `CONTENTS_BODY` area links alone (`link_player_area`, `world.rs:1460-1499`), so bullets, melee and lag compensation still hit players.
2. **"Overlapping Minecraft players push each other apart like vanilla"** (`crates/sim`)
   - Add `height: f32` and a riding flag to `PredictionRemoteBody` (`world.rs:252-259`, filled at `:3208-3215` from `voxel::player_hull(ps).1[2]` and `MC_RIDING`). Add `fn push_bodies(world) -> Vec<PushBody { entnum, origin, height, riding }>` from the same two sources as `alive_body_clips` (alive clients plus remote bodies, minus `MC_NOCLIP`), built before the mutable borrow next to `step.rs:381`.
   - A pure `minecraft_push(own, own_height, others, self_entnum) -> [f64; 2]`: block units (bx = map x / BLOCK, bz = −map y / BLOCK, by = map z / BLOCK); each body a 0.6-wide box with its own height; skip bodies whose boxes don't overlap (strict); for each overlap, a copy of vanilla's push step (the vendored `pushing.rs:425-439` is private and sim does not depend on minecraftoss, so copy it as a tiny fn); sum the impulses.
   - Apply it as a collision-checked move, not as velocity. MW2 ground friction (`friction.rs:16-27`, stopspeed 100) would eat every vanilla-sized push, so idle players would never separate. In `run_players_system`, in the block at `step.rs:411-422`, after `voxel::push_out` and before `pmove`, when the own player is in a Minecraft world, not noclip, not riding, `pm_type` 0, not on a ladder (`pm_flags::LADDER` approximates `onClimbable`), and the push is non-zero:
     - `dt` = the cmd's delta in seconds, clamped to 0.2 s (as `ClientPrediction::predict`);
     - map step = impulse × 20 × BLOCK × GAIN × dt with GAIN = 1/(1 − 0.546); map dx = +x, map dy = −z;
     - `voxel::trace_as(origin, target, hull mins, hull maxs, Some(Mover::of(ps, &cmd)))`; if not startsolid, `ps.origin = t.endpos`.
   - Leave `ps.velocity` untouched.
   - The host steps every client through `run_players_system`; the friend predicts its own push through `step_predicted` (`net/src/client/predict.rs:321-357`) with remote bodies from the snapshot. That mirrors vanilla, where each client pushes its own player. No new messages.
3. **"Minecraft respawns and new friends spread over the respawn radius like vanilla"** (`minecraft_terrain`, `minecraft_survival`, `render_anim`)
   - `terrain.rs:1177-1180`: `respawn_position(&mut self, radius: i32)`, still unseeded, plus `respawn_position_at(radius, offset)` for tests.
   - `respawn.rs`: `level_spawn` takes `&mut Level` and the radius. In the spawn dimension, on the host's stream (`!level.terrain.stream.is_remote()`), return `Place::standing(dimension, stream.respawn_position(radius), yaw, pitch)`. Otherwise keep `player_spawn` (the Nether and the End's exact platform). ADVENTURE keeps `player_spawn` without fuzz. The radius is `globals.game_rules.int("respawn_radius")`.
   - `respawn_in` takes `&mut Level`; follow it through `the_end::travel` (`the_end.rs:165`), `render_anim/src/minecraft_end.rs:31` (its caller at `minecraft_portal.rs:72` already holds a mutable level), `respawn_block.rs:1204`, `minecraft_players.rs:154` and the `server.rs` tests at `:2519`, `:2542`, `:2559`.
   - `Server::join_friend` (`server.rs:670-676`): a new friend's `Joining.spawn` uses `respawn_position(radius)` on the Overworld. A friend with a saved place still arrives exactly where they left.
   - The host's first spawn keeps `player_spawn`. `stream.player_spawn` itself stays fixed, because it is the Overworld's MW2 map origin (`bands::origins(spawn)`, `minecraft_world.rs:573`).
   - Risk: `spawn.rs` loads candidate chunks with `ChunkMap::load_now`, which blocks while generating (`chunk_map.rs:699-719`). If that hitches far from spawn, try only loaded chunks and fall back to `player_spawn`.

Do not change `/tp` (exact, like vanilla) or MW2's `spawn_clear_of`.

### Tests
**`sim`** (pure functions; the voxel world is process-global, so tests must not depend on it):
- `block_world_players_are_no_body_to_bump_into`: `player_body_clips(&world, true)` is empty for three alive clients; with false it matches `spectators_are_no_body_for_other_players_to_bump_into` (`step.rs:2287`).
- `a_trace_starting_inside_another_player_is_free_in_a_block_world`: startsolid 0 with the soft list; the hard list gives allsolid 1 with origins 5 units apart.
- `the_push_matches_vanilla_entity_push`: (0.3, 0) against (0, 0) gives 0.3/sqrt(0.3)·0.05 ≈ 0.027386 and 0; (0.005, 0.009) gives none; d = 4 scales by 0.5; the sign points away.
- `overlapping_players_drift_apart_within_a_second`: two bodies 0.1 blocks apart, 20 cmds of 50 ms with a free trace closure, end more than 0.6 blocks apart; 16 ms steps give the same within 5%.
- `the_push_respects_height_spectators_and_riders`.
- `a_push_stops_at_a_wall`.

**`minecraft_terrain`:** `respawn_positions_spread_over_the_radius` (data-gated like `terrain.rs:2380`): two offsets give different columns within 10 of the world spawn, standing on a full-topped block; radius 0 gives the spawn column.

**`minecraft_survival`** (data-gated like `respawn.rs::level_and_globals`):
- `bedless_respawns_land_within_the_respawn_radius`.
- `respawn_radius_zero_respawns_on_the_spawn_column`.
- `a_new_friend_arrives_within_the_radius_and_a_returning_one_where_they_left`.
- Update `server.rs:2519-2525`, which asserts the exact `player_spawn`: assert the yaw and that the arrival is within the radius.

### Owner's steps (idea)
Two windows. For step 1 the friend uses a fresh name (or delete their `players/data/<uuid>.dat`).
1. The host stands still at spawn. The new friend joins: they appear near spawn (within 10 blocks), usually not inside the host. If they overlap, both can still walk, and standing still they slide apart within about a second.
2. The friend: `/tp <host name>`. They land inside the host; neither is frozen; they drift apart while idle. Then the host: `/tp <friend name>`: the same.
3. Walk into each other: no wall, a small overlap and a sideways nudge, as in vanilla. Crouch and jump next to each other: nothing sticks.
4. Without beds, both `/kill` and respawn 3 times: each lands on a varying column within 10 blocks of spawn, usually apart.
5. `/gamerule respawn_radius 0`, both `/kill`: both on the spawn column, not stuck, drifting apart. Set it back to 10.
6. Two beds side by side, each player sets their spawn, both `/kill`: each respawns at their own bed.
7. Through a Nether portal together and back: at both ends nobody is frozen and they separate.
8. One player in `/gamemode spectator` flies through the other: no pushing either way.
9. With pvp on, shoot the other with an MW2 gun: hits still register.

```
IW4L_MINECRAFT_KIT=minecraft:white_bed,minecraft:red_bed,minecraft:oak_planks*64,minecraft:cobblestone*64,minecraft:obsidian*14,minecraft:flint_and_steel,minecraft:cooked_beef*32
```

---

## Shared points for the track
- **The protocol.** `minecraft_net::PROTOCOL` goes from 3 to 4 once in this track, in A6's first commit. Track B also bumps it from 3 to 4. Whoever merges second takes the next number and renumbers the new message indices in the codec tests. Record the final number in the log.
- **Shared files.**
  - `frame/src/minecraft_ui.rs`: A4 adds two fields to `MinecraftUi`; track B adds book fields and changes `McStack`. Add fields only.
  - `render_anim/src/minecraft_remote.rs`: A4 and A6 edit it; track B (friend books) and track D (friend limits) do too. Keep each edit small and local.
  - `render_anim/src/minecraft_creative.rs`: A6 splits `apply` and `hotbar_key`; track C changes the tabs.
  - `minecraft_net/src/messages.rs`: A6 appends one `ToServer` variant at the end; track B appends its own.
- **Order inside the track.** A2 before A4 (both touch the gear rig). A6 before track C merges if possible.
- **State docs.** Do not edit `docs/MINECRAFT-SURVIVAL.md`, `docs/MULTIPLAYER.md`, `CLAUDE.md`, `docs/PROGRESS.md` or `docs/round9/decisions.md`. Write the wording for them in the track log under "State doc changes": A1 (MW2 bodies drawn in every dimension), A3 (every game draws the dragon from its own client copy; boxes stay the host's; `MULTIPLAYER.md` 9.4), A4 (drop "heads and carved pumpkins on the head" from the limitation at `MINECRAFT-SURVIVAL.md:406`), A5 (the vanilla crystal rule), A6 (`MULTIPLAYER.md:660`, `:1330`, `MINECRAFT-SURVIVAL.md:368`), A7 (`MINECRAFT-SURVIVAL.md:143` and `:390`).
- **Combined kit line** for the track (27 stacks; the other heads come from the A4 kit or `/give`):

```
IW4L_MINECRAFT_KIT=minecraft:obsidian*14,minecraft:flint_and_steel,minecraft:diamond_sword+sharpness=5,minecraft:torch*64,minecraft:cobblestone*64,minecraft:totem_of_undying,minecraft:enchanted_book,minecraft:spyglass,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*16,minecraft:cooked_beef*32,minecraft:end_crystal*16,minecraft:elytra,minecraft:firework_rocket*64,minecraft:dragon_head,minecraft:piglin_head,minecraft:skeleton_skull,minecraft:wither_skeleton_skull,minecraft:zombie_head,minecraft:creeper_head,minecraft:player_head,minecraft:carved_pumpkin,minecraft:white_bed,minecraft:red_bed
```

## Rules
One commit per fix or feature, with tests. Run clippy, and rustfmt on new files. No code comments. Use gentle cargo mode. Owner steps and the kit line for each commit go in `docs/round9/track-a-log.md`.
