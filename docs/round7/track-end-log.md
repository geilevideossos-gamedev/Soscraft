# Track END log: the End

Branch `end`, worktree `C:\Dev\mc-wt\end`. Briefs: `e3-end.md` (commits E3-1 to E3-29), E2-4 of `e2-nether.md` (the Nether sky, moved here) and `e0-phase3-plan.md` (groups X1a, X1b, X2, ...). Base: 3aef2ebe (the dimension switch merged into `minecraft-survival`).

A commit can't name its own hash, so each entry names the previous commit's hash; the last entry's hash is in `git log` under its subject.

## Group X1a: End pieces that need no travel

### 1. Minecraft end city shulkers are no longer deleted far from players (E3-1)

Previous commit: 3aef2ebe

- **Defect:** mobs the entity world doesn't simulate yet wait dormant on the server thread (`minecraft_terrain/src/server.rs`, `ServerSim.dormant`). Every tick, a dormant mob of the monster category was deleted when it was more than 128 blocks from every player, or in peaceful. Shulkers are of the monster category, so the shulkers an end city places, and the ones saved in its chunks, were deleted as soon as the player was 128 blocks away, and the chunk was saved without them.
- **Changed:**
  - New `minecraft_terrain/src/dormant.rs`: `despawns(kind, persistent, tag, peaceful, nearest_squared)` is vanilla's `Mob.checkDespawn` for a dormant mob, with the two rules looked up by type: `allowed_in_peaceful` (`EntityType.isAllowedInPeaceful`) and `removed_when_far_away` (`Mob.removeWhenFarAway`).
  - `minecraft_terrain/src/server.rs` (`despawn_mobs`, the dormant part only): the dormant list is filtered through `dormant::despawns`. Mobs of other categories stay, as before.
- **Checked with javap (26.3):**
  - `Shulker extends AbstractGolem`, and `AbstractGolem.removeWhenFarAway` returns false. `Warden.removeWhenFarAway` returns false too. `EnderDragon.checkDespawn` does nothing, and `WitherBoss.checkDespawn` only removes it in peaceful.
  - `ZombieVillager.removeWhenFarAway` is false while it converts or once it has trade XP.
  - The monster-category types built without `notInPeaceful()` in `EntityTypes`: camel husk, ender dragon, piglin, shulker, sulfur cube, zombie horse and zombie nautilus. So shulkers also stay in peaceful.
  - `Mob.checkDespawn` removes a mob in peaceful before it looks at persistence, so a name-tagged dormant zombie still goes in peaceful (it stayed before).
- **Tests:** `dormant`: `shulkers_stay_far_from_players_and_in_peaceful`, `plain_monsters_go_beyond_128_blocks_and_in_peaceful`, `peaceful_keeps_the_vanilla_exceptions_but_far_away_still_removes_them`, `a_curing_or_traded_zombie_villager_stays`, `other_categories_are_left_alone`. `server`: `dormant_shulkers_stay_far_from_players_while_dormant_monsters_go` (a summoned shulker and pillager wait dormant; with the player 3000 blocks away the pillager goes and the shulker stays, also in peaceful).
- **Decisions:**
  - The idle despawn (one in 800 after 600 idle ticks) still never applies to dormant mobs, since they don't tick.
  - A raider in a raid (`Raider.removeWhenFarAway`) is not handled, as there are no raids yet.
- **Test steps:** shulkers aren't drawn until the outer End group (X6), so this can't be seen in game yet; the unit tests cover it. Optional, with vanilla 26.3 Java:
  1. In the `.env`, set `IW4L_MINECRAFT_WORLD=x1atest`. Start, and type `/summon minecraft:shulker ~ ~ ~2`.
  2. Type `/tp ~400 ~ ~`, wait 10 seconds, then `/tp ~-400 ~ ~`, and quit through the menu.
  3. Open `x1atest` in vanilla 26.3: the shulker sits where you summoned it.
- **Kit:** none needed.

### 2. Minecraft end portal frames take eyes of ender and open the portal (E3-2)

Previous commit: a4dbd42b

- **Changed:**
  - New vendored `third_party/minecraftoss/generator/src/feature/end_portal.rs` (MinecraftOSS patch): `EndPortalRing::find(cell, pos)` is `EndPortalFrameBlock.getOrCreatePortalShape().find(level, pos)`. It runs vanilla's `BlockPattern.find` as it is: the pattern `?vvv?`, `>???<` three times, `?^^^?` (`v` an eye-filled frame facing north, `^` south, `>` west, `<` east, `?` any loaded block), tried with its front top left at every block from `pos` to `pos + (4, 4, 4)` and in the 24 orientations, in vanilla's order. `interior()` is the 3x3 at `frontTopLeft.offset(-3, 0, -3)` in vanilla's fill order, and `centre()` the block vanilla's global event uses. `RingCell` (unloaded, other block, or a frame with its facing and eye) is what the caller reads.
  - New vendored `third_party/minecraftoss/world/src/level/end_portal.rs` (MinecraftOSS patch): `Level::place_ender_eye(pos)` is the server half of `EnderEyeItem.useOn`: on a frame without an eye it sets `eye=true` (flags 2), updates comparators (`updateNeighbourForOutputSignal`), plays level event 1503 (`block.end_portal_frame.fill` at the frame, volume 1, pitch 1), then looks for the ring; on a match each of the 9 inner blocks is destroyed with drops (`destroyBlock(pos, true)`) and becomes `end_portal` (flags 2), and event 1038 plays (`block.end_portal.spawn`, volume 1, pitch 1). It answers `Filled` or `Opened(ring)`, or nothing when the block takes no eye. `Level::ring_cell(pos)`.
  - `minecraft_terrain/src/server.rs`: `ItemOnBlock::EnderEye` runs `place_ender_eye` on the server thread (two lines).
  - New `minecraft_terrain/src/ender_eye.rs`: `eye_use(player, world, held)` is the client half: an ender eye aimed at a frame without an eye sends `ItemOnBlock::EnderEye` and is spent when the server says yes (not in creative), through the existing item-use answer.
  - `render_anim/src/minecraft_world.rs`: one call line, `ender_eye::eye_use` after bone meal in the item-on-block chain.
- **Tests:** generator `end_portal`: `a_full_ring_is_found_from_each_of_its_12_frames`, `the_portal_fills_the_3x3_inside_the_ring`, `eleven_eyes_find_nothing` (each of the 12 left empty, tried from every frame), `one_frame_facing_out_or_sideways_finds_nothing`, `a_ring_facing_outward_finds_nothing`, `an_unloaded_corner_finds_nothing`. `ender_eye`: `an_eye_on_an_empty_frame_asks_the_server_and_is_spent`, `a_filled_frame_or_another_block_passes_the_eye_on`. `server`: `twelve_eyes_in_an_inward_ring_of_frames_open_the_end_portal` (stone takes no eye, a filled frame takes no second one, 11 eyes give only the fill sounds, the 12th opens the 3x3 with the spawn sound at the centre, the frames keep their facing, the cobblestone inside drops as an item), `a_frame_facing_out_keeps_the_portal_shut`.
- **Checked with javap (26.3):** `EnderEyeItem.useOn` (PASS on a filled frame, `setBlock(.., 2)`, `shrink(1)`, `levelEvent(1503)`, the 3x3 loop with `destroyBlock(pos, true, null)` and `setBlock(end_portal, 2)`, `globalLevelEvent(1038, frontTopLeft.offset(-3,0,-3).offset(1,0,1))`), and `LevelEventHandler` (1503: `END_PORTAL_FRAME_FILL`, blocks, 1, 1, plus 16 smoke puffs; 1038: `END_PORTAL_SPAWN`, hostile, 1, 1, played 2 blocks from the camera toward the portal).
- **Decisions:**
  - The frames must face inward, as in vanilla: a frame placed while you face north faces south (toward you), so place each one standing inside the ring looking out.
  - Event 1038 is a global event in vanilla: every player in the level hears it at full volume. Here it is a sound at the portal centre, heard within 16 blocks; the one who places the last eye is always within reach of it. With several players it should go to everyone in the level (multiplayer hand-over).
  - Not done: the 16 smoke puffs of event 1503, and `Block.pushEntitiesUp` when the eye makes the frame taller (13 to 16 pixels high).
  - Until X1b's commit 3 the end portal blocks have no look of their own, and until X2 they don't take you anywhere.
- **Test steps:**
  1. Keep `IW4L_MINECRAFT_WORLD=x1atest` and the kit below. Start on flat ground.
  2. Clear a 5x5 square. Stand in its middle and place 3 frames on each side of the inner 3x3, turning to face each side before you place it, so all 12 face the middle. Leave the four corners empty.
  3. Put an eye in each frame. Each eye plays the fill sound and one eye goes from the stack. **Screenshot 1** after 11 eyes: the eleven eyes show, nothing opens.
  4. Put in the twelfth: the portal sound plays and the 3x3 inside changes (the end portal; it gets its starfield look in X1b). **Screenshot 2.**
  5. Click a filled frame with an eye: nothing happens and the eye stays in your hand.
  6. Optional: build a second ring with one frame turned outward. Twelve eyes go in, but it doesn't open.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:end_portal_frame*24,minecraft:ender_eye*32,minecraft:diamond_pickaxe+efficiency=5,minecraft:cobblestone*64
```
