# E3: the End (design brief)

Owner: "throwing eyes of ender to find a stronghold, filling the portal frames, the End spawn platform, the End sky. The full dragon fight: end crystals and cages, healing, breath, perching, boss bar, XP, exit portal, dragon egg. Also end gateways to the outer islands, end cities, shulkers, endermen, and elytra with fireworks. The exit portal takes me back to my spawn."

Paths are as of a43d0fc. This track runs after the dimension switch (e1) and the Nether track's portal check, track D ([d-saving.md](d-saving.md)), track B commits 7, 8 and 12, and track C commits 10-12.

## 1. What exists

- **State.**
  - `Server.levels` is keyed by `Dimension` (`minecraft_survival/src/server.rs:19-23`), and `update()` picks the level by `player.dimension` (`render_anim/src/minecraft_world.rs:556`).
  - Still overworld only: `load()` (`:229`, `:250`, `:280`), load completion (`:434`), `player.rs:65`, `entities.rs:160` (`ServerSim::new(.., "minecraft:overworld")`), `server.rs:44-50` and `save.rs:16`, `:259`.
- **End terrain.** `for_dimension_at(.., End, ..)` puts the arrival at 100.5, 49, 0.5 (`minecraft_terrain/src/terrain.rs:1017-1063`). It has never run.
- **Worldgen** (`generator/src/feature/kinds/the_end.rs`): spikes, cages and crystals (`:34-131`); a private `EndPlatform` without drops (`:146-173`); gateways with exits (`:175-223`); no `end_podium` (`:13-22`).
- **Stronghold.** The 12 frames are at `structure/kinds/stronghold.rs:839-868`, and `ring_chunks`/`locate_pos` at `structure/placement.rs:200-211`. B8 adds the eye throw and the nearest-ring search.
- **End city.**
  - Shulkers and the elytra frame (`end_city.rs:301-335`) go dormant (`minecraft_terrain/src/server.rs:449`).
  - **Bug:** dormant shulkers more than 128 blocks away are deleted (`:532-544`).
- **Gaps:** the live `World` impl (`world/src/level/mod.rs:856-930`) drops `add_entity` and `set_gateway_exit` (`generator/src/feature/mod.rs:647-670`); saved data goes only to the root (`level_storage.rs:161-182`); portal entry is only logged (`level/entity.rs:1263-1271`); the skybox mode is sent (`minecraft_world.rs:1955-1959`) but `sky_fragment` ignores it (`render_gpu/src/drawsurf/minecraft_world.rs:926-956`).
- **Bullets:** one box per key `kind<<56|id` (`entities.rs:715-773`, `:939-982`); `shoot` and `blast` attack as player 0 (`:525-548`); MW2 scales damage by hit height (`sim/src/voxel.rs:228`, `combat.rs:1244`).
- **Movement and mobs:** creative flight is a pmove mode (`movement_iw4/src/fly.rs`, `MC_FLYING` at `playerstate_iw4/src/playerstate.rs:187`); no levitation (`entities/src/effects.rs:17-27`); the enderman is the only simulated End mob; `ChunkMap::tick` takes one player centre (`chunk_map.rs:612`).

## 2. Vanilla 26.3 (javap)

- **`EnderEyeItem.useOn`.**
  - On a frame without an eye: set `eye`, take one eye, event 1503.
  - Then `getOrCreatePortalShape().find`. On a match the 3x3 at `frontTopLeft.offset(-3,0,-3)` gets `destroyBlock(true)` and then `end_portal`, and global event 1038 plays.
- **`EnderEyeItem.use`.** It passes on a frame, otherwise `findNearestMapStructure(#eye_of_ender_located = stronghold, pos, 100)`.
- **`EyeOfEnder`.**
  - A target more than 12 blocks away becomes the point 12 toward it and 8 up.
  - It dies after 80 ticks and drops if `nextInt(5)>0`.
- **`EndPortalBlock`.**
  - **Into the End.** `createEndPlatform((100,49,0), breakBlocks=true)`, which places a 5x5 of obsidian at y 48 and clears 49-51, dropping what it breaks. Players arrive at 100.5, 49, 0.5 facing west, keeping pitch and velocity.
  - **Out of the End.** Players go to `findRespawnPositionAndUseSpawnBlock`. A player without `seenCredits` first gets `showEndCredits()` (`WIN_GAME`).
  - The transition is instant. The player's portal cooldown is 10 ticks.
- **`WinScreen`.** `texts/end.txt` and `texts/credits.json`. Esc respawns at once.
- **`EnderDragonFight`** is saved in the End's own data storage as `ender_dragon_fight`.
  - **Boss bar:** pink, with music and fog, for living players within 192 blocks of (0, 128, 0).
  - **While players are near:** an arena ticket of radius 9 around chunk 0,0 (it waits for chunks -8 to 8). The first scan builds the inactive podium. With no dragon, or none seen for 1200 ticks, one spawns at (0, 128, 0). Crystals are recounted every 100 ticks.
  - **On a kill:** the podium becomes active and the egg appears (first kill only). The next of 20 seed-shuffled gateways goes at `floor(96·cos/sin(2(−π+i·π/20)))`, y 75.
  - **Respawn:** 4 crystals beside the exit portal start the stages START, PREPARING (100 ticks), PILLARS (40 per spike), SUMMONING_DRAGON (100) and END.
- **`EnderDragon`.**
  - **Body:** 200 health; parts head 1x1, neck 3x3, body 5x3, three tails 2x2, two wings 4x2.
  - **Damage:** a part other than the head takes `d/4+min(d,1)`; only players or `#always_hurts_ender_dragons` (explosions) count; more than 25% of max while sitting makes it take off.
  - **Crystals:** a 1-in-10 chance per tick to pick the nearest within 32, which heals 1 every 10 ticks; breaking it does 10 to the head.
  - **Contact:** wings push 4.0 out and 0.2 up for 5 damage while flying; head and neck do 10. `checkWalls` breaks blocks except `#dragon_immune`, `#dragon_transparent`, or with `mob_griefing` off.
  - **Death:** 200 ticks rising 0.1 per tick; XP 12000 on the first kill, else 500 (8% every 5 ticks after tick 150, 20% at 200).
  - **Phases:** HoldingPattern, StrafePlayer, LandingApproach, Landing, Takeoff, SittingFlaming, SittingScanning, SittingAttacking, ChargingPlayer, Dying, Hovering.
- **Breath:** a fireball leaves a Harming II cloud, radius 3 growing to 7 over 600 ticks; a perched flame is radius 5 for 200 ticks.
- **`EndCrystal`.** Any non-dragon damage destroys it. It explodes with power 6 unless an explosion killed it, and keeps fire burning on top.
- **`TheEndGatewayBlockEntity`:** the exit search steps along the direction from 0,0 scaled to 1024, 16 blocks at a time (back while the chunk isn't empty, forward while it is). It then takes the end stone nearest the origin with room above, or places an `end_island` at y 75, then `findTallestBlock` within 16, and builds a return gateway 10 above. Cooldown 40 ticks.
- **`DragonEggBlock`:** up to 1000 tries at `(r16−r16, r8−r8, r16−r16)` into air.
- **Elytra:** `updateFallFlyingMovement` (pitch lift, drag 0.99/0.98/0.99), wall damage `Δspeed·10−3`, 1 durability every 20 ticks. A rocket adds `look·0.1+(look·1.5−v)·0.5` per tick for `10·(1+flight)+rand(6)+rand(7)` ticks.
- **Shulker:** 30 health; its bullet does 4 damage and Levitation for 200 ticks.
- **End sky:** a cube of `end_sky.png` (UVs 0-16, #282828), no sun or moon.

**Credits decision:** vanilla. The first exit shows the credits once per player (`seenCredits`), and Esc skips them. Travel to the respawn point starts behind the credits, so they hide the loading. Later exits go straight there.

## 3. Needed from other tracks

- **Dimension switch ([e1-dimension-switch.md](e1-dimension-switch.md)).**
  - `LevelLoader` and `Entities::new(.., dimension)` (e1 commit 5).
  - `Player.switching = Some(Switch { to: End, arrival, reason: END_PORTAL })` (e1 commit 8), with an arrival that can keep the pitch and the velocity.
  - The end portal loading screen (e1 commit 9).
  - The respawn switch (e1 commit 10).
  - A closing End level must save its fight data (e1 commit 6).
- **Nether track:**
  - the per-tick portal check on the feet box, returning the portal kind and position, with a transition time per kind and the 10-tick cooldown;
  - the sky branch for `none`.
- **Added here if e1 and the Nether track don't add them:**
  - `LevelStorage::read_saved_in`/`write_saved_in(dimension, id)`;
  - the `the_end` clock;
  - `ChunkMap::tick` with several ticket centres.
- **Track D.**
  - `Dimension`, `Pos` and `Rotation` are read before the first tick, so a player saved in the End joins there.
  - `Player.respawn: Option<RespawnConfig>`, plus a pure `respawn_target(&Globals, &Player) -> Arrival` that falls back to the `level.dat` spawn with its dimension.
  - `seenCredits` round-trips in `player_nbt`.
  - D10's `record_block_entities`.
- **Track B:** `effect_cloud.rs` (B7), the eye throw and locate (B8), and a pmove impulse (B12).
- **Track C:** `/locate`, `/spawnpoint` and `/execute in`.

## 4. Commits

Mob AI goes in the vendored `entities` crate, the server thread and renderers in `minecraft_terrain`, and per-player state in `minecraft_survival`. Each mob is wired as in NETHER-END.md "Wiring in a new mob kind".

**A. A playable End**

1. "Minecraft end city shulkers are no longer deleted far from players" (vanilla golems never despawn by distance)
2. "Minecraft end portal frames take eyes of ender and open the portal" (`EndPortalRing` beside `PortalShape`, `ItemOnBlock::EnderEye`, level events)
3. "Minecraft end portals and gateways are drawn" (a screen-space `end_portal.png` layer over `end_sky`)
4. "Minecraft live levels keep feature entities and gateway exits" (`add_entity` and `set_gateway_exit` on the live `Level`)
5. "Minecraft end portals take players to the End" (an e1 `Switch`)
6. "Minecraft End spawn platform is rebuilt on every arrival and drops what it breaks"
7. "Minecraft End sky is drawn with its texture" (mode 1 in `sky_fragment`, no clouds)

**B. The dragon fight, step by step**

8. "Minecraft End arena chunks stay loaded while a player is near"
9. "Minecraft end crystals sit on the pillars, burn and explode"
   - The entity, the `EndCrystalModel` renderer and a bullet box of key kind 17.
   - The `end_crystal` item.
10. "Minecraft End dragon fight is saved per End level" (`dragon_fight.rs`, `scanState`, a new `end_podium` feature arm)
11. "Minecraft ender dragon flies its holding pattern over the End"
    - `dragon.rs` with `DragonFlightHistory`, the parts and the 24-node path.
    - The `EnderDragonModel` renderer, and flap and growl sounds.
12. "Minecraft ender dragon has a boss bar" (`Output.boss` per player, a `MinecraftUi` field, the `boss_bar/pink_*` sprites, fog)
13. "MW2 bullets, grenades and Minecraft hits hurt the ender dragon through its body parts"
    - 8 boxes with key kind 16 and `id<<3|part`, and the vanilla part rule.
    - A flat MW2 hit location for the parts.
14. "Minecraft end crystals heal the ender dragon with a beam"
15. "Minecraft ender dragon strafes players and spits fireballs" (breath clouds on B7)
16. "Minecraft ender dragon perches on the portal and breathes fire" (the landing, sitting, takeoff and charge phases)
17. "Minecraft ender dragon knocks players back and breaks blocks as it flies"
18. "Minecraft ender dragon dies like vanilla" (rays, rise, sound, XP orbs)
19. "Minecraft exit portal opens and the dragon egg and a gateway appear"
20. "Minecraft dragon eggs teleport when clicked or hit"
21. "Minecraft exit portal takes players to their respawn point and shows the credits once"
22. "Minecraft end crystals on the exit portal respawn the dragon"

**C. The outer End**

23. "Minecraft end gateways throw players to the outer islands"
    - The exit search uses `ChunkMap::load_now` behind the `SpawnGate`, and builds the return gateway.
    - Pearls also go through.
24. "Minecraft item frames show their item and drop it when hit"
25. "Minecraft shulkers peek, shoot homing bullets and teleport"
26. "Minecraft levitation lifts the player" (vendored `EffectKind::Levitation`; pmove `vy += (0.05·(amp+1)−vy)·0.2`)
27. "Minecraft elytra glide on MW2 movement"
    - `movement_iw4/src/glide.rs` with `MC_MAY_GLIDE`/`MC_GLIDING` bits, kept clear of e1's `MC_NO_SKY`.
    - Durability, wall damage, and the fall reset through `steps.air_peak`.
28. "Minecraft firework rockets boost elytra flight"
29. Optional: "Minecraft End sky flashes like 26.3"

## 5. Unit tests

- **Portal ring:** found from any of the 12 frames; 11 eyes or an outward frame find nothing.
- **Platform:** 25 obsidian cells and 75 air cells.
- **Fight:** a fresh start gives the inactive podium, then the dragon; NBT round-trips; the 20 gateway positions; the egg only on the first kill.
- **Podium:** block counts per layer, active and inactive.
- **Part damage:** head 20 gives 20, body 20 gives 6, no player gives none; the sitting threshold.
- **XP:** the death adds up to exactly 12000 or 500.
- **Crystal:** an explosion kills it without a blast, and a dragon hit does nothing.
- **Gateway search** on synthetic chunks, and an **egg** in a full box stays put.
- **Glide and boost:** one tick matches hand-worked vanilla numbers, the boost converges to look·1.5, durability drops every 20 ticks. **Levitation** converges to 0.05·(amp+1).
- **Respawn and credits:** `respawn_target` falls back to the world spawn's dimension, and credits show only without `seenCredits`.

## 6. Owner test

`.env`: `IW4L_MINECRAFT_WORLD=endtest`, plus this kit (23 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:ender_eye*32,minecraft:end_portal_frame*12,minecraft:elytra,minecraft:firework_rocket*64,minecraft:end_crystal*8,minecraft:diamond_sword+sharpness=5,minecraft:diamond_pickaxe+efficiency=5,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots+feather_falling=4,minecraft:shield,minecraft:totem_of_undying,minecraft:golden_apple*16,minecraft:cooked_beef*32,minecraft:water_bucket,minecraft:ender_pearl*16,minecraft:cobblestone*128,minecraft:obsidian*8,minecraft:glass_bottle*8,minecraft:torch*64,minecraft:oak_planks*64
```

1. Throw eyes from three places. They fly the way `/locate structure minecraft:stronghold` points. **Screenshot 1.**
2. Stand in the middle of a cleared 5x5 and place the 12 frames around you. Fill them: the portal opens with its sound. **Screenshot 2.**
3. Jump in. You stand on obsidian at 100, 49, 0 facing west, under the dark End sky, and the boss bar shows. **Screenshot 3.**
4. Place cobblestone at 101, 50, 0, type `/kill` and come back through the portal: the cobblestone has dropped and the platform is whole.
5. Watch a heal beam (**screenshot 4**), then shoot crystals. They explode, but one killed by a grenade does not.
6. Fight. Head shots hurt more. It perches and breathes, its wings throw you, and it breaks cobblestone but not end stone. **Screenshot 5.**
7. Kill it: rays, a big XP flood (level 68 from 0), the portal, the egg and a gateway. **Screenshot 6.**
8. Click the egg: it jumps away.
9. Pearl into the gateway. You land on an outer island with a return gateway above. **Screenshot 7.**
10. In an end city, shulkers levitate you. Take the elytra from the ship's frame. **Screenshot 8.**
11. Wear the elytra, jump off a ledge and press jump: you glide, a rocket boosts you, and durability drops. **Screenshot 9.**
12. Use `/spawnpoint` (in the overworld first), then go into the exit portal. The credits roll (Esc skips) and you arrive at your point, keeping everything. A second exit has no credits.
13. Quit while in the End and restart: you are still in the End, and the fight stays won.
14. Place 4 crystals on the portal's edges. The pillars rebuild and a new dragon comes. It gives only 500 XP and no egg.

## 7. Risks

- **Ordering:** commits 5, 21 and 23 wait for e1 commits 5-10 and D7/D9; 15 and 17 wait for B7 and B12.
- **One voxel world per process** (`sim/src/voxel.rs:41`) blocks friends in different dimensions until the dimension switch solves it.
- **Entities aren't networked yet:** the dragon's parts and beams add to that work. Targeting must use every player, not `PLAYER = 0` (`entities.rs:34`).
- **A second dragon** can spawn if the dragon's chunk unloads; hold the arena ticket.
- **Gateway exits** generate chunks up to 1024 blocks out on the spot; keep the loading screen up.
- **MW2 netcode:** the new `pm_flags` bits and boost state must survive deltas and prediction, and MW2 headshot scaling must not stack on dragon parts.
- **Still missing:** wings on the third-person body, chorus fruit teleport, end rod particles, dragon's breath bottles.
- **Mojang files** (credits, textures, models) are read at runtime and never committed.
