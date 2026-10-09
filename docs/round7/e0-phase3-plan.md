# Phase 3 execution plan: dimension switch, Nether, End

Inputs: [e1-dimension-switch.md](e1-dimension-switch.md) (15 commits), [e2-nether.md](e2-nether.md) (30), [e3-end.md](e3-end.md) (29), [d-saving.md](d-saving.md) and CRATE-SPLIT.md, all at a43d0fc. "E2-12" means commit 12 of the E2 brief. The group names (S, N, X) are new.

## 0. Gate

Phase 3 starts once track D is merged. Exactly what it needs from D:
- **D2:** the weather read/write.
- **D4:** `write_player` writing `Dimension`.
- **D7:** `Dimension`, `Pos` and `Rotation` read before the first tick, `Frame::teleport_facing`, and a `SpawnGate` watching a target chunk. E1-12 removes D7's fallback to the world spawn for a non-overworld position.
- **D9:** `Player.respawn` with a setter.
- **D10:** `record_block_entities`.
- **Ask D now** to add `PortalCooldown` (int) and `seenCredits` (byte) to the keys `player_nbt` owns, so neither E track edits `player_nbt.rs`.

From the other tracks:
- **C and B:** C12 gates E1-11, and B7 and B12 gate X5 and the elytra. C10, C11 and B8 are only needed for tests.

**X1a can start before D:** it shares no file with D or E1. E3-1 should land early anyway, because today it deletes saved end city shulkers (`minecraft_terrain/src/server.rs:532-544`).

## 1. Dependency order

Two cargo slots (`scripts\cargo-slot.ps1`). Each cell is one session. The two cells of a row share no file apart from the append-only and by-region ones in section 2.

| Step | Slot 1 | Slot 2 | Owner checkpoint |
|---|---|---|---|
| 1 | S1 server-wide state | X1a End pieces that need no travel | |
| 2 | S2 levels open and close | X1a, if not done | |
| 3 | S3 the switch | N1 a Nether level that runs | CP1 |
| 4 | N2 portal travel | X1b skies and portals drawn | CP2 |
| 5 | N3 Nether mobs on existing AI | S4 MW2 in other dimensions | |
| 6 | N4 ghasts and blazes | X2 into the End | CP3 |
| 7 | N5 Nether rules, docs | X3 crystals and the flying dragon | |
| 8 | stretch: elytra (section 4) | X4 killing the dragon, going home | CP4, release |

**Hard edges:**
- **S2 → N1, X1b:** levels on demand and the debug start.
- **S3 → N2, X2:** the switch, the loading screen and the respawn switch.
- **N1 → X3:** E2-12's public blast, which crystals explode through.
- **N2 → X2:** E2-6's `PortalKind` check.
- **N4 → X3:** the mob wiring files, so the two never run at once.
- **S4 → elytra:** the pm_flags bits.

## 2. File ownership

| Shared file | Owner | Rule for the others |
|---|---|---|
| `render_anim/src/minecraft_world.rs` | E1 | Others write new adapter files (`minecraft_portal.rs`, `minecraft_end.rs`) and add one call line in `update()`. |
| `minecraft_survival/src/{server,level,player,save}.rs` | E1 | Others add one field each (`Level.rules`, `Player.portal`, `Level.dragon_fight`), with the logic in their own modules. |
| `minecraft_survival/src/respawn.rs` (new in E1-10) | E1 | Pure `respawn_target(&Globals, &Player, spawn_block)`. N5 plugs beds and anchors into `spawn_block`, and X4 calls it. |
| Per-dimension saved data (`read_saved_in`/`write_saved_in`) and the `the_end` clock | E1, in S1 | X2 saves `ender_dragon_fight` through it. |
| `sim/src/voxel.rs`, `sim/src/damage.rs`, `net/src/client/runtime.rs` | E1 | Nobody else. |
| `playerstate_iw4/src/playerstate.rs` pm_flags (`MC_*` at :181-187) | E1 (`MC_NO_SKY`) | Glide takes the next free bits after S4. Bits are never picked in parallel. |
| `frame/src/minecraft_ui.rs`, `hud/src/lib.rs` | E1 (the loading reason) | Append only: the portal overlay (N2), the boss bar (X3), the credits (X4). |
| `render_gpu/src/drawsurf/minecraft_world.rs` (`sky_fragment` :926-956, clouds) | End | **E2-4 moves to X1b**, so one agent owns the sky shader. |
| `minecraft_survival/src/portal.rs` (new in E2-6) | Nether | `PortalKind { Nether, End, Gateway }` with a transition time per kind and the 10-tick cooldown. X2 fills in the End arm only. |
| Vendored `explosion.rs` | Nether (E2-12, moved into N1) | X3 calls the public blast. |
| Vendored `the_end.rs`, `stronghold.rs`, and the live `World` in `level/mod.rs` | End | Nobody else. |
| Vendored `chunk_map.rs` | D10, then the End (E3-8 needs several ticket centres; `tick` takes one, :612) | Nobody else. |
| Mob wiring: `server_mobs.rs`, `client_mobs.rs`, `mesh.rs`, `minecraft_survival/src/entities.rs` (`boxes`, `encode`), vendored `entities/src/{world,effects}.rs` | Nether during N3-N4 | X3 appends after rebasing on N4, keeping both sides. Sorted lists stay sorted (`SIMULATED_KINDS`, `server_mobs.rs:388`). E3-13 only adds a dragon loop to `boxes()`. |
| `minecraft_terrain/src/server.rs` | By region | E1: `ServerSim::new` and the weather. Nether: `Command::Blast`, lava, spawners. End: the dormant despawn, arena, gateways. Rebase before every group. |
| `minecraft_terrain/src/terrain.rs` | E1 (E1-4) | N1 adds the cardinal flag after S1. |
| `movement_iw4/src/*` | End (glide) | Glide work starts only after B12's impulse is merged. |

**Must not touch:**
- **Nether:** the voxel, damage and playerstate files, `save.rs`, `level_storage.rs`, the End generator, `chunk_map.rs`, the sky shader.
- **End:** `portal.rs` beyond its arm, `explosion.rs`, the respawn internals, `player_nbt.rs`, and `server.rs`/`save.rs` beyond its field.
- **E1:** mob code, generator features, the sky shader, and any hud file other than the loading screen.

## 3. Groups

Each group is one session. It ends with its brief's unit tests, clippy and its owner steps.

**Dimension switch (one agent)**
- **S1, server-wide state:** E1-1 (plus the per-dimension saved data), E1-2 (plus the `the_end` clock), E1-3, and E1-4, the first run of the Nether and End generators, so their bugs show here and not mid-switch.
- **S2, levels open and close:** E1-5, 6, 7 and 12. E1-5's `LevelLoader::open(dimension, prepare)` runs `prepare: FnOnce(&mut Terrain) -> Arrival + Send` on the loader thread before the player is placed; E2's portal search and build go there. `Arrival` carries feet, yaw, pitch and velocity (the End keeps the last two).
- **S3, the switch:** E1-8 (`Switch { to, arrival, reason }` with `NETHER_PORTAL`, `END_PORTAL`, `OTHER`), E1-9, E1-10, and E1-11, which moves to S4 if C12 isn't merged.
- **S4, MW2 in other dimensions:** E1-13, E1-14 and E1-15.

**Nether**
- **N1, a Nether level that runs:** E2-1, 2, 3, 5 and 12. E2-12 doesn't depend on the dimension and unblocks beds, anchors and crystals.
- **N2, portal travel:** E2-6 to E2-11.
- **N3, Nether mobs on existing AI:** E2-18 to E2-22, then E2-25.
- **N4, ghasts and blazes:** E2-23 and E2-24, with decision 2.
- **N5, Nether rules:** E2-13 to E2-16, then E2-30 (docs, including what is left).
- **N6, brain mobs:** E2-26 to E2-29 and E2-17. Deferred, see section 4.

**End**
- **X1:**
  - X1a, any time: E3-1, E3-2 and E3-4.
  - X1b, after S2: E2-4 (the Nether: no sky, sun, stars or clouds), then E3-3 and E3-7.
- **X2, into the End:** E3-5, E3-6, E3-8 and E3-10.
- **X3, crystals and the flying dragon:** E3-9 and E3-11 to E3-14.
- **X4, killing the dragon and going home:** E3-18, 19, 21 and 22, plus docs.
- **X5, a dragon that fights back:** E3-15, 16, 17 and 20 (needs B7 and B12).
- **X6, the outer End:** E3-23 to E3-29.

## 4. Estimate

The crate split (24 commits) took about a session-day. These groups are smaller but riskier: untried generators, AI ports, owner-test round-trips.

**This round, realistically:** about 59 commits in 15 groups (steps 1-8).
- **The switch:** all of S1-S4.
- **The Nether:** N1-N5, that is travel, the look, beds and anchors, fortress spawns and spawners, plus zombified piglins (with portal piglins), magma cubes, wither skeletons, ghasts and blazes.
- **The End:** X1-X4, that is eyes and frames, travel, the platform and the sky, and a dragon that circles, heals from crystals, takes MW2 damage through its parts, dies with vanilla XP and opens the exit portal home, with the credits once.

**Stretch:** elytra and rockets (E3-27, E3-28). They don't need the dragon, test in the overworld with the kit, and fit a free slot after S4 and B12.

**Leave for "finish survival":**
- N6: striders, hoglins, piglin bartering, brutes, zoglins and lodestones.
- X5: strafing, breath, perching, wing knockback, block breaking and the egg teleport. Until then the dragon only does contact damage.
- X6: gateways, item frames, shulkers, levitation and the End flashes.
- Per-dimension voxel worlds, which go with multiplayer (roadmap 1.4).

## 5. Decisions to take now

1. **Air support.** Keep E1-14: refused where the dimension type has `has_ceiling` (the Nether), with the streak kept and a line saying why; allowed in the End and the overworld. Vanilla has no such rule; its nearest are the ceiling rules (no weather, beds explode), and under the roof the aircraft would sit in netherrack.
2. **Ghast fireballs: MW2 bullets and melee deflect them along the shooter's aim**, replacing E2's proposed default (melee only). Checked with javap: `Projectile.onHit` calls `onRedirectProjectile` → `deflect(AIM_DEFLECT, owner)` for `#redirectable_projectile` (`fireball`, `wind_charge`, `breeze_wind_charge`), so arrows and snowballs deflect them in vanilla; `Player.deflectProjectile` does it for melee. `AbstractHurtingProjectile.hurtServer` returns false, so damage alone never deflects: grenades only push them by knockback. Blaze small fireballs aren't in the tag.
3. **Credits: vanilla.** Once per player (`seenCredits`), Esc skips them, the respawn trip loads behind them, and the texts come from the jar at runtime.
4. **Elytra on MW2 movement:** a pmove glide mode (`movement_iw4/src/glide.rs`).
   - It steps vanilla's `updateFallFlyingMovement` per 50 ms tick, scaled by frame time as `fly.rs:104` does, with MW2 collision.
   - A fresh jump press in the air starts it; landing, water or levitation stops it. The rocket boost counts down in the pmove state, so prediction matches.
   - MW2 guns stay usable, as vanilla lets a gliding player attack and use items. The body plays the in-air animation.
5. **Beds, anchors and crystals** deal vanilla explosion damage to players and mobs through E2-12's blast, not the MW2 blast rule (x0.8). Otherwise "Intentional Game Design" can never kill.
6. **MW2 damage on the dragon:** one flat MW2 hit location on all parts, and vanilla's part rule decides: full damage to the head, `d/4+min(d,1)` elsewhere. MW2 blasts count as explosions (`#always_hurts_ender_dragons`), and MW2 headshot scaling never stacks.
7. **Portal search without POI files:** keep E2's choice (loaded and saved chunks, never generating). Checked with javap: `PoiManager.ensureLoadedAndValid` loads the square at `ChunkStatus.EMPTY`, so vanilla doesn't generate either, and `checkConsistencyWithBlocks` rebuilds missing POIs from blocks, so vanilla finds the portals in our saves.
8. **Friends in other dimensions:** not this round. There is one voxel world (`sim/src/voxel.rs:41`), and bots stay in the host's level. All new state still goes on `Player` (by UUID) or `Level`.
9. **The End before multiplayer.** CLAUDE.md orders multiplayer (1.4) before the End (1.5). X1-X4 are fine now on two conditions:
   - the dragon, the crystals and the boss bar take their targets from the players in the End level, never `PLAYER = 0` (`minecraft_survival/src/entities.rs:34`);
   - the fight is saved per level.

## 6. Owner checkpoints

| CP | After | Run |
|---|---|---|
| CP1 | step 3 | E1 owner steps 1-3 and 5-7, with the e1 kit. If E1-11 slipped to S4, start step 1 with `IW4L_MINECRAFT_DIMENSION` instead |
| CP2 | step 4 | The steps below |
| CP3 | step 6 | E1 step 4, E2 steps 11-12 (e2 mob kit, without the strider, piglin and hoglin checks, which are N6), E3 steps 1-4 (no boss bar yet) |
| CP4 | step 8, `--release` | E2 steps 6-10 (e2 travel kit), E3 steps 5, 7 and 12-14 (no perching, breath or egg teleport; those are X5) |

CP2, with `IW4L_MINECRAFT_WORLD=cp2` and this kit (18 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:obsidian*64,minecraft:flint_and_steel,minecraft:diamond_pickaxe+efficiency=5,minecraft:cooked_beef*32,minecraft:cobblestone*64,minecraft:torch*64,minecraft:water_bucket,minecraft:totem_of_undying,minecraft:ender_eye*16,minecraft:end_portal_frame*12,minecraft:diamond_sword,minecraft:shield,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:ender_pearl*16,minecraft:glass*64
```

1. `/tp 800 ~ -1600`, build and light a portal, step in. Purple overlay and trigger sound; after about 4 s the portal loading screen; then the Nether with no sky, sun or clouds and red fog. **Screenshot 1.**
2. `/tp ~ ~ ~` shows about (100, y, -200). Staying in the arrival portal does nothing; step out and back in and you return to your first portal. **Screenshot 2.**
3. In the overworld, place the 12 frames around a cleared 5x5, facing inward, and fill them with eyes: the portal opens with its sound and starfield. **Screenshot 3.** (Travel comes with X2.)
4. Quit, add `IW4L_MINECRAFT_DIMENSION=the_end`, start: a dark textured End sky, no sun or clouds. **Screenshot 4.**

## 7. Risks

- **The switch is the critical path.** S1's generator test brings the first-run panics forward.
- **A portal search into an open level** (a return within 15 s, later a friend there) runs on the main thread against that level's `TerrainStream`, which owns its `ChunkMap` (`terrain.rs:949`), and can read 19x19 saved chunks. Spread it over ticks behind the loading screen.
- **Machine limits:** B and C may hold cargo slots, and a target dir is 6-11 GB on a short D:, so keep at most two phase 3 worktrees and delete each after its merge.
- **Mob wiring** is shared by N3, N4 and X3. The schedule keeps them in separate steps, and X3 rebases on N4.
- **Owner test round-trips** gate N2 and X2, which are the riskiest groups.
