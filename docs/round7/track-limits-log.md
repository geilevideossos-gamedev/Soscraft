# Track LIMITS log: phase 4, the known limitations

Branch `limits`, worktree `C:\Dev\mc-wt\limits`. Briefs: the limitation lists in `docs/MINECRAFT-SURVIVAL.md` (Rendering, Movement, Survival and combat, Blocks and items, Mobs). Base: 4c4900e.

A commit can't name its own hash, so each entry is written with its commit and its hash is in `git log` under its subject.

## Group particles-sounds

Missing particles and sounds from the Rendering limitations. Every behaviour below was checked with javap on the 26.3 client jar.

### 1. Minecraft fire, lava and campfires make their ambient sounds

- **Changed:** `minecraft_terrain/src/display_ticks.rs` (`ClientLevel.animateTick`'s per-block step, which already ran for nether portals) now also runs:
  - `BaseFireBlock.animateTick` for `fire` and `soul_fire`: one time in 24, `block.fire.ambient` at the block centre, volume 1 + r, pitch 0.3 + 0.7 r.
  - `LavaFluid.animateTick` for `lava` with air above: one time in 100, `block.lava.pop` at a random spot of the top face (volume 0.2 + 0.2 r, pitch 0.9 + 0.15 r); one time in 200, `block.lava.ambient` at the block's corner (vanilla plays it at `getX()`, not the centre), same volume and pitch.
  - `CampfireBlock.animateTick` for lit `campfire` and `soul_campfire`: one time in 10, `block.campfire.crackle` at the centre, volume 0.5 + r, pitch 0.6 + 0.7 r.
  - The adapter already played every sound the display tick returns, so `render_anim` needed no change.
- **Tests:** `display_ticks`: `fire_roars_once_in_twenty_four_ticks`, `lava_under_air_pops_on_its_surface_and_hums_from_its_corner`, `covered_lava_is_silent`, `lit_campfires_crackle_once_in_ten_ticks`.
- **Decisions:** the display tick samples blocks around the player as vanilla does (667 tries in each of the 16 and 32 block cubes a tick), so the sounds come from nearby blocks at vanilla's rate without a list of fires or lava.
- **Test steps:**
  1. Stand next to a lava pool (or pour the lava bucket into a hole): within a few seconds the lava pops and hums. **Screenshot 1** of the pool.
  2. Light a netherrack block with flint and steel and stand by it: the fire crackles and roars about every second.
  3. Place the campfire and stand next to it: it crackles every half second or so. Douse it with the water bucket: it falls silent.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:lava_bucket,minecraft:water_bucket,minecraft:netherrack*4,minecraft:flint_and_steel,minecraft:campfire*2,minecraft:soul_campfire,minecraft:hay_block*2
```
