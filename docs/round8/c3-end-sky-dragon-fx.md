# C3: End sky flashes, dragon effects and the End music (design brief)

Track C (End and visual leftovers), item c3. Paths and lines are as of e2313625. Vanilla facts were checked with javap on the obfuscation-free 26.3 `client.jar`, with the 26.3 data pack and with the pack's `sounds.json`. No Mojang code is copied; this brief gives behaviour and numbers only.

## 1. The owner's request

"The flashing End sky (26.3 has the End flash: the End sky flashes with a light beam and a sound at random intervals), and the dragon leftovers: knockback (wing knockback of players already exists? check), dragon breath particles (the dragon_breath particle clouds of the fireball and the breath attack), the death particles and beams (the dragon dies over 200 ticks with explosion_emitter particles, rises, light beams, XP), and the boss music (music.dragon in the End during the fight, music.end otherwise; how our music system picks tracks)."

Answers to the questions in the request, from round 7's log (`docs/round7/track-end-log.md`):
- **Wing knockback of players exists** (entry 41: the wings fling players and mobs, the head bites). What is missing is the other direction: **players' hits do not knock the dragon back** (state doc `docs/MINECRAFT-SURVIVAL.md:283`). This brief adds it.
- **Rising, the death rays and the experience exist** (entry 28). Missing: the `explosion` particles every tick while it dies, the `explosion_emitter` bursts on ticks 180 to 200, and the dissolve. This brief adds the particles; the dissolve is an optional last group.
- **Our "music system" plays no Minecraft music at all.** Minecraft sounds are one-shots decoded whole (`audio/src/minecraft.rs:44-89`), and the asset setup leaves every `sounds/music/` file on Mojang's servers (`assets/src/minecraft_setup.rs:166-176`). This brief fetches the three End tracks and adds vanilla's `MusicManager` for them.

## 2. What exists today

| Area | State | Where |
|---|---|---|
| End sky | Drawn per pixel from `end_sky.png` (`SkyRenderer.renderEndSky`). No flash: round 7 left `renderEndFlash` for later. | `render_gpu/src/drawsurf/minecraft_world.rs:1047-1068`, `:1116-1126`; log entry 6 |
| Boss fog | While a shown boss bar has `world_fog` (the dragon's always does), the sky shader gets the `none` skybox, so the fog colour replaces the End sky (vanilla's `GameRenderer` drops the sky pass when `shouldCreateWorldFog`). | `render_anim/src/minecraft_world.rs:2291-2296` |
| Sky uniform | `view.environment` (16 vec4s): slot 6 is `[sky_light_color, sky_light_factor]`, slot 14 `w` the skybox code. `end_portal: [time, 0, 0, 0]` has three free floats. | `render_anim/src/minecraft_world.rs:2261-2303`; `render_gpu/.../minecraft_world.rs:100-103`, `:597-604`, `:860-881` |
| Lightmap | `lightmap()` adds `light.rgb * brightness(sky) * light.w`: the End's `sky_light_factor` is 0, so an End flash only has to add to `light.w`. | `render_gpu/.../minecraft_world.rs:910-924` |
| Celestial atlas | 9 slots of 32 px (sun, 8 moons), built from the pack; the shader picks `(slot + u) / 9`. | `render_anim/src/minecraft_world.rs:333-356`; `render_gpu/.../minecraft_world.rs:1078-1099` |
| Environment | `Environment::tick(clock_ticks, ...)` runs 20 times a second with `globals.default_clock_ticks(dimension)` (the End reads `minecraft:the_end`'s clock); `sky_state` lerps by the partial tick. `SkyState` has no flash field. | `minecraft_terrain/src/environment.rs:111`, `:157-221`; `day_cycle.rs:18-45`; caller `render_anim/src/minecraft_world.rs:2227-2249` |
| End clock | `Globals.end_clock`, saved, advanced with `advance_time`, `/time` can pause it. | `minecraft_survival/src/server.rs:42`, `:56-68`, `:111-120` |
| Minecraft sounds | `Sounds::play(packs, event, position, volume, pitch)` resolves `sounds.json` and queues a one-shot; falloff is linear to `attenuation_distance` (16) blocks. `weather.end_flash` (8 files in `ambient/weather/`) is in the pack. | `render_anim/src/minecraft_sounds.rs:95-148`; `audio/src/minecraft.rs:66-89` |
| Music files | Not downloaded: the asset filter skips `minecraft/sounds/music/` and `records/`. The index lists `music/game/end/boss.ogg` (3.76 MB), `the_end.ogg` (11.3 MB) and `alpha.ogg` (7.1 MB, the credits). The fetched folder is marked `.complete`, so setup never runs again on the owner's install. | `assets/src/minecraft_setup.rs:166-189`, `:72-76`; startup hook `assets/src/minecraft_map.rs:83-87`, `assets/src/plugin.rs:13` |
| Audio building blocks | Symphonia decodes Ogg Vorbis (`decode_audio_bytes`, whole file into PCM). A custom streaming `Decodable` exists as a pattern (`MatchBusAudio`). Script music shows how a long voice is faded and stopped and how `master_volume` is applied. | `audio/src/pcm.rs:320-380`; `audio/src/match_bus.rs:29-80`, `:193`; `audio/src/playback.rs:162-163`; `audio/src/script_music.rs:56-63` |
| Boss bar | `McBossBar { name, from, target, elapsed, world_fog }`, set per local player by `show_boss_bar` from `DragonFight::boss_bar(uuid)`. No `play_music` flag. | `frame/src/minecraft_ui.rs:204-245`; `render_anim/src/minecraft_end.rs:121-126`, `:230-246`; `minecraft_survival/src/the_end/dragon_fight.rs:287` |
| Credits | `ui.credits` is set while the end poem runs. | `render_anim/src/minecraft_end.rs:195-228`; `minecraft_world.rs:619` |
| Ambient particles | Client particle system with vanilla's base particle, `generic_0..7` sprites, `explosion` (`huge_explosion`, full bright, quad 2.0 for size 0) and `explosion_emitter` (8 ticks, 6 flashes a tick). | `minecraft_terrain/src/ambient_particles.rs:10-39`, `:62-95`, `:154-169`, `:676-730`; owner `minecraft_survival/src/entities.rs:70`, `:220`, `:1170-1175` |
| Breath clouds | Drawn with the potion spell particles in `BREATH_COLOR` (#CB00E5), each client tick from the last snapshot's `CloudView`s. The fireball's burst is a 100-particle potion splash in that colour. No fireball trail, no mouth breath. | `entities.rs:851-853`, `:911-913`; `spell_particles.rs:102-156`; `effect_cloud.rs:15`, `:35-41` |
| Dragon on the server | `EnderDragon::tick` → `tick_death` (rise, roar, experience) → `ai_step`, which returns at once while dying. `flaming_tick` counts `flame.ticks`. `head_look_vector` is `getHeadLookVector`. `DragonEvents.wall_bursts` (level event 2008 cells) is filled but never sent. | `minecraft_terrain/src/ender_dragon.rs:1172-1235`, `:1577-1600`, `:722-733`, `:1903` |
| Dragon to the client | `EntitySnapshot` is built every server tick; `breath_bursts` is the "take" pattern to copy. `DragonView` has the phase, parts, `death_time`, position and previous position. | `server.rs:211-224`, `:2837`; `dragon_fireball.rs:28-32`, `:123-129`; `ender_dragon.rs:814-832` |
| Dragon damage | `EndDamage { amount, player, kind }`; `hurt_part` → `really_hurt` returns `Hurt { applied, full, died }`. The helper `knockback(velocity, on_ground, xd, zd)` (`LivingEntity.knockback(0.4, ...)`) exists for mobs. Hits come from `Entities::punch` (eye) and `Entities::shoot` (the shooter's position). | `end_crystal.rs:17-47`; `ender_dragon.rs:1011-1060`, `:588-600`; `entities.rs:512-520`, `:750-755`; `server.rs:1848-1863` |
| Death rays | `append_rays` (white rays fading in the last fifth), drawn translucent. | `dragon_render.rs:304`; `entities.rs:1235` |

## 3. Vanilla 26.3 behaviour (javap)

### 3.1 The End flash (`EndFlashState`, `ClientLevel.tick`, `SkyRenderer`, `LightmapRenderStateExtractor`, `DirectionalSoundInstance`)

- **Who has it:** `DimensionType.hasEndFlashes` is `skybox == END`. `ClientLevel` creates one `EndFlashState` for such a level and ticks it every client tick with `Level.getDefaultClockTime()` (the dimension's `default_clock`, `minecraft:the_end` for the End; its `world_clock/the_end.json` is `{}`).
- **The schedule, a pure function of the clock** (constants: interval 600, max offset 200, duration 100 to 380, sound delay 30):
  - `window = clock / 600` (long division). When it differs from the last window (`flashSeed`), `RandomSource.createThreadLocalInstance(window)` (a `SingleThreadedRandomSource`: the same LCG as Java's `Random(seed)` and our `LegacyRandom::new`) draws, in order: one `nextFloat` (thrown away), `offset = nextInt(201)`, `duration = 100 + nextInt(min(380, 600 − offset) − 100 + 1)`, `xAngle = −60 + nextFloat·70`, `yAngle = −180 + nextFloat·360`.
  - **Window 0 never flashes.** `flashSeed` starts at 0 (a Java field default), so a fresh `EndFlashState` in window 0 never draws: `offset` and `duration` stay 0, `t > 0` gives 0, and `t = 0` gives `Mth.sin(0/0)`, which is the table's `SIN[(long)NaN & 65535] = SIN[0] = 0`. The first flash of a new world's End clock comes in window 1 (ticks 600-1199).
  - `t = clock % 600`. The intensity is 0 outside `[offset, offset + duration]`, otherwise `Mth.sin((t − offset)·π / duration)` (the table sine).
  - Each tick keeps the old intensity; `getIntensity(partial) = lerp(partial, old, new)`. `flashStartedThisTick` is `new > 0 && old <= 0`.
  - So there is exactly one flash every 30 seconds (except in window 0), starting 0 to 10 s into the window and lasting 5 to 19 s. A paused clock freezes it.
- **The sound:** when a flash starts and the credits are not open, `weather.end_flash` (category weather, volume 1, pitch 1, one of 8 files) is played **30 ticks later**, as a `DirectionalSoundInstance`: attenuation none, positioned every tick at the camera plus `directionFromRotation(xAngle, yAngle) · 10` (`x = −sin(y)·cos(x)`, `y = −sin(x)`, `z = cos(y)·cos(x)`).
- **The sky quad** (`SkyRenderer.render` → `renderEndSky`, then `renderEndFlash` only when the intensity is above 1e-5):
  - Pose: `rotate YP(180 − yAngle)`, then `rotate XP(−90 − xAngle)`, then `translate(0, 100, 0)` and `scale(60, 1, 60)` (`applyCelestialBodyTransform(pose, 100, 60)`).
  - The quad is the celestial quad: corners (−1, 0, −1), (1, 0, −1), (1, 0, 1), (−1, 0, 1) with UVs (u0, v0), (u1, v0), (u1, v1), (u0, v1) of the `end_flash` sprite (`textures/environment/celestial/end_flash.png`, 64x64, a soft purple glow).
  - Worked out: the quad's centre lies along `directionFromRotation(xAngle, yAngle)`, 100 blocks out; its local X axis is `(−cos y, 0, −sin y)` and its local Z axis `(−sin x·sin y, cos x, sin x·cos y)` (x, y the two angles). Its half size seen from the eye is 0.6 (60 over 100; the sun is 0.3).
  - Pipeline `CELESTIAL`: `core/position_tex` (texel times `ColorModulator`, discard at alpha 0) with the colour modulator `(i, i, i, i)`, blend `OVERLAY` (source alpha, one). So the sky gains `texel.rgb · texel.a · i²`.
  - The flash is part of the sky pass, so it is **not drawn while the dragon's boss bar creates world fog** (round 7 entry 24), but its light and sound still come.
- **The light:** `LightmapRenderStateExtractor` adds the intensity to the lightmap's `skyFactor` (`intensity / 3` while the boss overlay creates world fog). The End's `sky_light_color` is `#ac60cd` and its factor 0, so sky-lit blocks and mobs glow purple with the flash.

### 3.2 Dragon's breath particles (`DragonBreathParticle`, `AreaEffectCloud.clientTick`, `DragonFireball`, `AbstractHurtingProjectile`, `LevelEventHandler` 2006, the two phases' `doClientTick`)

- **The particle** (`DragonBreathParticle`, provider takes a `PowerParticleOption`):
  - Built with the 4-argument base particle: `lifetime = (int)(4 / (nextFloat·0.9 + 0.1))`, then `quadSize = 0.1·(nextFloat·0.5 + 0.5)·2` (draw order: lifetime, then size). No jitter: the motion is exactly the given `(xd, yd, zd)`.
  - Colour: red `nextFloat(0.7176471, 0.8745098)`, green 0 (no draw: `Mth.nextFloat` with min ≥ max returns min), blue `nextFloat(0.8235294, 0.9764706)`. Then `quadSize ·= 0.75` and `lifetime = (int)(20 / (nextFloat·0.8 + 0.2))` (20 to 100 ticks). Friction 0.96, `hasPhysics = false` (so it never lands), sprites `generic_5`, `generic_6`, `generic_7` by age (`age·2 / lifetime`).
  - `setPower(p)`: `xd ·= p`, `yd = (yd − 0.1)·p + 0.1`, `zd ·= p`.
  - Tick: age, then if `y == yo` after the move, `xd` and `zd` grow by 1.1; then `xd`, `zd` times friction. (The "hit ground" branch never runs without physics.)
  - Drawn size `quadSize · clamp((age + partial) / lifetime · 32, 0, 1)` (it pops up over the first 1/32 of its life), layer opaque, lit by the level's light at its position (not full bright).
- **Breath clouds** (both carry `PowerParticleOption(DRAGON_BREATH, 1)`): each client tick, while waiting (first 20 ticks) a `nextBoolean` may skip the tick, otherwise 2 particles at radius 0.2 with no motion; once active, `ceil(π·r²)` particles at `angle = nextFloat·2π`, `distance = sqrt(nextFloat)·r` around the cloud's feet, with motion `((0.5 − nextDouble)·0.15, 0.01, (0.5 − nextDouble)·0.15)`.
- **The fireball's burst** (level event 2006 at the burst's block position): 200 particles, each `r = nextFloat·4`, `a = nextFloat·2π`, `x = cos(a)·r`, `y = 0.01 + nextDouble·0.5`, `z = sin(a)·r`, placed at `(bx + x·0.1, by + 0.3, bz + z·0.1)` with motion `(x, y, z)` and power `r`. Then the sound (already played).
- **The fireball's trail** (`AbstractHurtingProjectile.tick` → `createParticleTrail`, `DragonFireball.getTrailParticle`): one particle a tick at `(x, y + 0.5, z)`, no motion, power 1.
- **Breath from the mouth while landing** (`DragonLandingPhase.doClientTick`, every client tick in phase `Landing`): `look = getHeadLookVector(1).normalize()` (the `yRot(−π/4)` and `yRot(π/16)` calls return new vectors that are thrown away, so the look never turns: keep that quirk). 8 particles at the head part's `(x, y + height/2, z)` plus `nextGaussian/2` on each axis, motion `(−look.x·0.08 + dx, −look.y·0.3 + dy, −look.z·0.08 + dz)` where `d` is the dragon's delta movement, power 1.
- **Breath while flaming** (`DragonSittingFlamingPhase.doClientTick`): the phase's `flameTicks` counts up each tick from its `begin`; on even values below 10 (2, 4, 6, 8): 8 points at the head centre plus `nextGaussian/2` per axis, and at each point 6 particles `j = 0..5` with motion `(−look.x·0.08·j, −look.y·0.6, −look.z·0.08·j)`, power 1. 192 particles a flame.

### 3.3 The dragon's death particles (`EnderDragon.aiStep`, `tickDeath`, `LevelEventHandler` 2008)

- **Every tick while dead or dying**, `aiStep` draws `(nextFloat − 0.5)·8`, `(nextFloat − 0.5)·4`, `(nextFloat − 0.5)·8` from the **dragon's own random** (on the server too, where the particle is a no-op) and adds one `explosion` at `(x + a, y + 2 + b, z + c)`, no motion, then returns.
- **`tickDeath`**, after `dragonDeathTime++`, on death times **180 to 200** draws the same three offsets and adds an `explosion_emitter` there. (This runs before `aiStep` in the tick.)
- **Level event 2008** (from `checkWalls`): one `explosion` at the cell's centre, no motion.
- **The dissolve** (`EnderDragonRenderer.submit`): while `deathTime > 0` the body is drawn only with `entityCutoutDissolve(dragon.png, dragon_exploding.png)` and the vertex colour `ARGB.white(1 − deathTime/200)`; `entity.fsh` with `DISSOLVE` discards a fragment when the vertex alpha is below the mask's alpha at the same UV, then draws it opaque. The eyes layer is still drawn; the rays come after.

### 3.4 Knockback on the dragon (`EnderDragon.reallyHurt`, `EnderDragon.knockback`, `LivingEntity.hurtServer`, `dealDefaultKnockback`)

- `hurt(part)` → `reallyHurt` → `Mob.hurtServer`. Unless the damage type is in `#no_knockback` (explosions, fire, magic, `dragon_breath`, ...) or fully blocked, `dealDefaultKnockback` pushes with strength 0.4 away from the source position (`source.x − x`, `source.z − z`, the dragon's own position, not the part's). This also happens on a partial hit inside the invulnerability window; a hit that does nothing returns earlier.
- `EnderDragon.knockback` does nothing while the phase is sitting. Otherwise `LivingEntity.knockback`: `motion = (old.x/2 − nx·0.4, old.y, old.z/2 − nz·0.4)` (the dragon is never on the ground). The dragon has no knockback resistance.

### 3.5 Music (`MusicManager`, `Musics`, `Minecraft.getSituationalMusic`, `BackgroundMusic`, `ClientPacketListener.handleRespawn`, `WinScreen`)

- **Tracks** (`Musics`: sound, min delay, max delay, replace current): `CREDITS` (`music.credits` → `music/game/end/alpha`, 0, 0, true), `END_BOSS` (`music.dragon` → `music/game/end/boss`, 0, 0, true), `END` (`music.end` → `music/game/end/the_end`, 6000, 24000, true; the same values sit in `the_end.json`'s `audio/background_music`). All three are `"stream": true`, volume 1.
- **`getSituationalMusic`:** a screen's own music first (`WinScreen` → `CREDITS`); else, in the End, `END_BOSS` when any shown boss bar has `playBossMusic` (`EnderDragonFight`'s bar has `setPlayBossMusic(true)` and `setCreateWorldFog(true)`); else the camera's `background_music` attribute (the End: `END`).
- **`MusicManager.tick`** (every client tick; `DEFAULT` frequency caps delays at 20·1200 = 24000 ticks; the delay starts at 100):
  1. With no situational music: `delay = max(delay, 100)`, done (what plays keeps playing).
  2. If something plays: when `canReplace` (the new music replaces and its sound differs from what plays) stop it and `delay = Mth.nextInt(0, minDelay/2)` (inclusive). If it is no longer active: forget it and `delay = min(delay, nextInt(min(minDelay, 24000), min(maxDelay, 24000)))`.
  3. `delay = min(delay, maxDelay)`.
  4. With nothing playing and no level loading screen: `--delay`; at 0 or below, start the situational music and set `delay = i32::MAX`.
- **`stopPlaying()`:** stop, then `delay = nextSongDelay(situational) + 100`. It runs on every **dimension change** (`handleRespawn` when the dimension differs) and when the credits close (`WinScreen.removed` → `stopPlaying(CREDITS)` if credits play).
- What that means in play: entering the End while the dragon lives, its boss bar shows and the boss music starts on the next tick and loops back-to-back. When the bar goes (dragon dead), the boss music is replaced: it stops and the End music starts 0 to 3000 ticks later. Entering the End with no fight, the End music starts 5 to 20 minutes after arrival (6000 to 24000 + 100 ticks). The exit portal's credits play `alpha` at once; closing them stops it.

## 4. The plan (commit groups, in order)

General rules for every group: no comments; `minecraft_terrain` is edition 2021 (no let-chains); new files pass `rustfmt --check`; edits to existing files touch only the lines they need. All client presentation (flash, music, particles) stays client-side and derived from per-level server data; no new "player 0" state.

### Group 1. "The Minecraft End sky flashes like vanilla"

- **New `minecraft_terrain/src/end_flash.rs`** (`pub mod end_flash;` in `lib.rs`): `EndFlash { window: i64, offset: i32, duration: i32, x_angle: f32, y_angle: f32, intensity: f32, old_intensity: f32 }`, `Default` with every field 0 like vanilla's fresh `EndFlashState` (so window 0 never draws and never flashes; guard the `0 / 0` so it gives 0, not NaN), with:
  - `tick(&mut self, clock_ticks: i64)`: section 3.1's schedule with `minecraftoss_core::random::LegacyRandom::new(window)` and `minecraftoss_player::mth::sin`; `window` and `t` use floor division and `rem_euclid` so a negative clock is safe.
  - `intensity(partial) -> f32`, `started_this_tick() -> bool`, `angles() -> (f32, f32)`, `direction() -> DVec3` (`directionFromRotation`).
  - `pub fn light_boost(intensity: f32, world_fog: bool) -> f32` (`intensity` or `intensity / 3`).
- `minecraft_terrain/src/environment.rs`: `Environment` holds `end_flash: Option<EndFlash>`, created fresh (`EndFlash::default()`) whenever the presentation's skybox becomes `End` and dropped when it stops being `End`, as vanilla makes a new `ClientLevel` on each dimension change; `tick` (`:111`) ticks it with `clock_ticks`; `sky_state` (`:157-221`) fills a new `SkyState.end_flash: [f32; 3]` (intensity at the partial tick, x angle, y angle; zero intensity outside the End). Add the field to `SkyState` (`day_cycle.rs:18-45`) and to every `SkyState { .. }` literal (grep `SkyState {`, including `lava_fog.rs`).
- `render_anim/src/minecraft_world.rs`:
  - `celestial_image` (`:333-356`): 10 slots of 64 px (sun and moons scaled up with `Nearest`, which keeps them exact), `environment/celestial/end_flash` in slot 9.
  - Slot 6 of `view.environment` (`:2268-2273`): `sky_light_factor + end_flash::light_boost(sky.end_flash[0], world_fog)`, where `world_fog` is the same boss-bar test as `:2292`.
  - `view.end_look.end_flash = sky.end_flash` next to `:2305`.
- `render_anim/src/minecraft_end_look.rs:18`: `pub end_flash: [f32; 3]` on the end look view. `render/src/extract.rs:1101`: copy it to `frame.end_flash`.
- `render_gpu/src/drawsurf/minecraft_world.rs`:
  - `MinecraftWorldFrame.end_flash: [f32; 3]` (`:100-103`), packed into `end_portal: [time, intensity, x_angle, y_angle]` (`:602`).
  - WGSL: `celestial()`'s divisor becomes 10 (`:1091`). In `sky_fragment` (`:1124-1126`), after `end_sky(ray)`, when `view.end_portal.y > 1e-5`: an `end_flash(ray)` helper computes the direction and the two quad axes of section 3.1 from the angles, projects the ray onto the quad plane (`p = ray / dot(ray, d)`, `u = dot(p, ax)/0.6`, `v = dot(p, az)/0.6`), returns nothing outside `[-1, 1]`, samples slot 9 at `((u+1)/2, (v+1)/2)`, discards alpha 0, and adds `texel.rgb · texel.a · i²` (clamped to 1).
- **Tests:**
  - `end_flash`: `window_zero_never_flashes` (a fresh `EndFlash` ticked through clock 0-599 keeps intensity 0 and never starts); `a_window_draws_its_flash_like_java_random` (windows 1, 2 and 7: offset, duration and angles against values computed once with `jshell` and `new java.util.Random(seed)`; the numbers go in the test, the jshell lines in the track log, since code comments are not allowed); `one_flash_every_600_ticks_rising_and_falling_as_a_sine` (intensity 0 before the offset, the table sine at the middle, 0 after the end, `started_this_tick` exactly once a window); `a_paused_clock_holds_the_flash`; `the_flash_lights_the_world_a_third_as_much_in_boss_fog`; `the_flash_points_where_direction_from_rotation_points` (x = −90 is straight up).
  - `environment`: `only_the_end_flashes` (overworld and Nether keep a zero `end_flash`).
  - `render_gpu`: `the_end_flash_quad_centres_on_its_direction_and_spans_point_six` (port the WGSL axes into a Rust test helper as round 7 did for `end_sky_uv`, check the centre maps to UV (0.5, 0.5) and a ray 0.6 out along each axis to the edges); `the_celestial_atlas_has_ten_slots`.

### Group 2. "Minecraft End flashes rumble a second and a half after they start"

- `end_flash.rs`: `EndFlash` also counts the sound: `started_this_tick` with `credits_open == false` sets `sound_in = 30`; `tick` returns `Some((x_angle, y_angle))` on the tick it reaches 0. Change the signature to `tick(&mut self, clock_ticks: i64, credits_open: bool) -> Option<(f32, f32)>`; `Environment::tick` takes `credits_open` too and returns that value (its test callers pass `false` and ignore it).
- `render_anim/src/minecraft_sounds.rs`: `Sounds::play_without_falloff(packs, event, position, volume, pitch)`, the same as `play` but with `attenuation = f32::INFINITY` on the queued request, so the gain stays at the volume and only MW2's pan places it (`audio/src/minecraft.rs:66-71` already handles it: `1 − d/∞ = 1`).
- `render_anim/src/minecraft_world.rs` (the environment tick, `:2231-2238`): pass `ui.credits.is_some()`, and when a sound is due play `minecraft:weather.end_flash` at the camera plus `direction · 10` blocks, converted to map units the way the other world sounds are.
- **Decision:** the sound is placed once when it starts (vanilla moves it with the camera every tick). It lasts a few seconds, so only a fast turn shows the difference.
- **Tests:** `end_flash`: `the_sound_comes_thirty_ticks_after_the_flash_starts_and_never_during_the_credits`. `minecraft_sounds`: `a_sound_without_falloff_keeps_its_volume_far_away` (with `MINECRAFTOSS_ROOT`, like `the_portal_hum_fades_out_at_ten_blocks`).

### Group 3. "Minecraft dragon's breath is drawn with vanilla's purple breath particles"

- `minecraft_terrain/src/ambient_particles.rs`:
  - `Kind::DragonBreath` and `Particle::dragon_breath(at, motion, power, random)` with section 3.2's draws in order (a new bare constructor: lifetime, then quad size, no jitter), `physics: false` (add a flag if `Particle` has none; the move then ignores blocks), the 1.1 spread when `y == previous.y`, friction 0.96, sprites `generic_5..7` by age, the size ramp `clamp((age + partial)/lifetime · 32, 0, 1)`, light from the level like the other non-explosion kinds.
  - `pub fn breath_cloud(&mut self, feet: DVec3, radius: f32, waiting: bool)` (the cloud rule), `pub fn breath_burst(&mut self, at: DVec3)` (event 2006 at `floor(at)`), `pub fn breath_trail(&mut self, at: DVec3)` (one particle at `at + (0, 0.5, 0)`).
- `minecraft_survival/src/entities.rs`:
  - `:851-853`: clouds with `breath` go to `self.ambient.breath_cloud`; potion clouds keep `self.spell.cloud`.
  - `:911-913`: `breath_bursts` go to `self.ambient.breath_burst` instead of the spell burst.
  - After `self.dragon_fireballs = ...` (`:910`): one `breath_trail` per fireball view, once per snapshot (one snapshot per server tick, so one particle a tick).
- **Tests** (`ambient_particles`): `a_breath_particle_draws_its_lifetime_size_and_colour_in_vanilla_order` (hand-worked from a seeded `LegacyRandom`: lifetime 20 to 100, red and blue in range, green 0), `power_scales_the_motion_like_set_power`, `a_breath_particle_drifts_through_blocks_and_spreads_when_it_stops_rising`, `a_waiting_breath_cloud_puffs_two_or_none_and_an_active_one_fills_pi_r_squared`, `the_fireball_burst_sprays_two_hundred_breath_particles_from_its_block`.

### Group 4. "Minecraft ender dragon breathes purple from its mouth while it lands and flames"

- `minecraft_terrain/src/ender_dragon.rs`:
  - `pub enum BreathKind { Landing, Flame }` and `pub struct DragonBreath { pub head: DVec3, pub look: DVec3, pub motion: DVec3, pub kind: BreathKind }`; `DragonEvents.breath: Vec<DragonBreath>` (`:722-733`).
  - In `ai_step` after the phase ticks (`:1261-1265`): in `Phase::Landing`, push `Landing` with `head = parts[HEAD] + (0, 0.5, 0)`, `look = head_look_vector(env).normalize()`, `motion = velocity`. In `flaming_tick` (`:1589-1600`), after the count, when `flame.ticks % 2 == 0 && flame.ticks < 10`, push `Flame` (motion zero).
- `minecraft_terrain/src/server.rs`: `end_events` (`:1635-1637`) keeps the breath in a `dragon_breath: Vec<DragonBreath>` buffer; a new `EntitySnapshot.dragon_breath` field (append it at the end of the struct `:211-224` and at the end of the literal at `:2837`) takes it.
- `minecraft_terrain/src/ambient_particles.rs`: `pub fn mouth_breath(&mut self, breath: &DragonBreath)` with section 3.2's 8 particles (landing) or 8 × 6 (flame), gaussians from the particle random.
- `minecraft_survival/src/entities.rs` (snapshot handling, near `:911`): each `DragonBreath` goes to `self.ambient.mouth_breath`.
- **Tests:** `ender_dragon`: `a_landing_dragon_breathes_every_tick_from_its_head` and `a_flaming_dragon_breathes_on_its_second_to_eighth_flame_tick` (exactly 4 events a flame, look from `head_look_vector`). `ambient_particles`: `landing_breath_streams_eight_particles_back_along_the_look`, `flame_breath_fans_six_speeds_at_eight_points`. `server`: extend `a_landing_dragon_perches_on_the_podium_breathes_flame_and_the_flame_goes_out_when_it_takes_off` to see breath in the snapshots while landing and four flame puffs.

### Group 5. "Minecraft ender dragon bursts into explosions while it dies and where it breaks blocks"

- `minecraft_terrain/src/ender_dragon.rs`: `DragonEvents.explosions: Vec<(DVec3, bool)>` (`true` for an emitter).
  - `tick_death` (`:1201-1229`): after `death_time += 1`, for `180..=200` draw the three offsets from `self.random` and push an emitter at `position + (a, 2 + b, c)`.
  - `ai_step`'s dying return (`:1233-1235`): first draw the three offsets from `self.random` and push a plain explosion. These draws are vanilla's server draws, so the dragon's random stays in step with vanilla's.
- `minecraft_terrain/src/server.rs`: `end_events` turns `wall_bursts` into plain explosions at the cell centres (`+ 0.5`), and both go to a new `EntitySnapshot.dragon_explosions: Vec<(DVec3, bool)>` (append at the end, as in group 4).
- `minecraft_survival/src/entities.rs`: each one is `self.ambient.add(Kind::Explosion | Kind::ExplosionEmitter, at, DVec3::ZERO)` (size 0 gives the 2.0 quad).
- **Tests:** `ender_dragon`: `a_dying_dragon_explodes_every_tick_and_bursts_from_tick_180_to_200` (200 plain, 21 emitters, offsets within ±4, ±2 around +2), `the_dying_dragon_s_random_draws_three_floats_a_tick` (the sequence after death matches a fresh `LegacyRandom` stepped the same way). `server`: `the_dragon_s_wall_bursts_reach_the_client_as_explosions` (extend `the_dragon_breaks_cobblestone_it_flies_through_but_not_end_stone`), and `the_first_dragon_drops_12000_experience_and_a_later_one_500` sees 21 emitters.

### Group 6. "Minecraft players' hits knock the flying ender dragon back"

- `minecraft_terrain/src/end_crystal.rs:25-47`: `EndDamage.from: Option<DVec3>` (where the hit came from); `attack(amount, player, from)`; `explosion` keeps `None`. Update the struct literals (`end_crystal.rs:469`, `:474` and `ender_dragon.rs:2208`) and the tests.
- `minecraft_terrain/src/ender_dragon.rs` `hurt_part` (`:1011-1034`): when the hurt was applied, the kind is `Attack`, the phase is not sitting and `from` is set, `velocity = knockback(velocity, false, from.x − position.x, from.z − position.z)`.
- `minecraft_survival/src/entities.rs`: `punch` (`:519`) passes the eye, `shoot` (`:753`) passes `from`. No new player constant: they already use `PLAYER` for the blame, as before.
- **Decision:** MW2 bullets knock the dragon back like a player's melee hit, from the shooter's position, because a bullet on a mob is already "an attack from where it was fired" (`entities.rs:748-749`). Grenades and other blasts are explosions, which vanilla's `#no_knockback` exempts.
- **Tests:** `ender_dragon`: `a_hit_knocks_a_flying_dragon_away_from_the_attacker` (halved motion less 0.4, y kept), `a_sitting_or_dying_dragon_is_not_knocked_back`, `an_explosion_does_not_knock_the_dragon_back`, `a_partial_hit_in_the_cooldown_still_knocks_back_but_a_wasted_one_does_not`.

### Group 7. "Minecraft fetches the End's three music tracks from Mojang"

- `assets/src/minecraft_setup.rs`:
  - `const END_MUSIC: [&str; 3] = ["minecraft/sounds/music/game/end/boss.ogg", "minecraft/sounds/music/game/end/the_end.ogg", "minecraft/sounds/music/game/end/alpha.ogg"];` and `fn wanted_asset(key: &str) -> bool` (the current filter at `:170-174` plus these three).
  - `pub fn begin_end_music(root: &Path)`: on a `minecraft-music` thread, read `root/asset-index.json`, and for each key whose file under `root/resourcepacks/local/minecraft-26.3/assets/` is missing or has the wrong SHA-1, fetch `OBJECTS_URL/<hash[0..2]>/<hash>` with the existing `fetch` (curl, hash checked) into a `.partial` file and rename it into place. Log `Minecraft End music ready` or a warning; never fail the game.
- `assets/src/minecraft_map.rs:83-87` (`prepare`): when `root()` is `Some`, call `begin_end_music`. A first-time fetch already gets the files through `wanted_asset`.
- **Decision:** only these three tracks (22 MB) are fetched. The rest of Mojang's music (259 MB) stays on their servers, so the overworld and the Nether stay silent, as now. Nothing is committed: the files come from Mojang's servers at run time like every other asset.
- **Tests:** `minecraft_setup`: `only_the_end_music_is_fetched_of_mojangs_music` (`wanted_asset` on the three keys, on `music/game/creative/...` and on a record), `the_end_music_lands_in_the_resource_pack_path`.

### Group 8. "Minecraft plays the End music, the dragon's boss music and the credits music like vanilla"

If this grows past one reviewable commit, split it in this order: 8a "Minecraft music streams from its Ogg file" (the `audio` side and its decode test), then 8b the client manager, the boss bar flag and the wiring.

- **Audio side, new `audio/src/minecraft_music.rs`:**
  - `McMusicStream(Arc<[u8]>)`, a `Decodable` asset that decodes Ogg Vorbis with Symphonia packet by packet as rodio pulls samples (the `MatchBusAudio` pattern in `match_bus.rs:29-80`; never the whole file: `the_end.ogg` is about 13 minutes). Register it with `add_audio_source::<McMusicStream>()` beside `playback.rs:162-163`.
  - `#[derive(Resource, Default)] pub struct McMusic { pub wanted: Option<McMusicTrack>, pub finished: Option<String> }` with `McMusicTrack { key: String, bytes: Arc<[u8]>, volume: f32 }`.
  - System `play_minecraft_music`: when `wanted` names a different key than the playing voice, stop the voice and spawn the new one (`PlaybackSettings::DESPAWN`, volume `volume · MIX_GAIN · master_volume`, no pan); when `wanted` is `None`, stop; when the voice's sink is empty or the entity is gone, set `finished = Some(key)`. Registered from `audio/src/minecraft.rs:38-42` (one line) with `mod minecraft_music;` and the `pub use` in `audio/src/lib.rs`.
- **Client side, new `render_anim/src/minecraft_music.rs`:**
  - `Music { event: &'static str, min_delay: i32, max_delay: i32, replace: bool }` with `CREDITS`, `END_BOSS`, `END` from section 3.5.
  - `pub fn situational(credits_open: bool, in_end: bool, boss_music: bool) -> Option<Music>`.
  - `MusicManager { current: Option<&'static str>, delay: i32, dimension: Option<Dimension>, random: LegacyRandom }` with `tick(situation, active, loading) -> Option<MusicAction>` (`Start(Music)` or `Stop`), `stop_playing(situation)` and `dimension_changed`. It is section 3.5 step by step: delay 100 at start, `nextInt` inclusive with the `min >= max` rule, the 24000 cap, `i32::MAX` after a start.
  - The random is seeded from the clock time the manager is made (vanilla's `RandomSource.create()`); tests inject a seed.
- `render_anim/src/minecraft_sounds.rs`: `Sounds::music_file(packs, event) -> Option<(String, Arc<[u8]>, f32)>` resolves the event's single `sounds.json` entry to its file and volume.
- `frame/src/minecraft_ui.rs:204-245`: `McBossBar.music: bool` and a fourth `McBossBar::new` argument (`EnderDragonFight`'s bar: `true`); update `render_anim/src/minecraft_end.rs:241` and the frame test at `:558`.
- `render_anim/src/minecraft_world.rs`:
  - `Runtime` (`:88-102`) gets `music: MusicManager`.
  - The `update` system takes `ResMut<audio::McMusic>` (add it to the first resource tuple at `:405-411`).
  - In the 20 Hz environment block (`:2228-2240`): the local player's dimension (a change calls `stop_playing`), `situational(ui.credits.is_some(), dimension == End, ui.boss_bars.iter().any(|bar| bar.music))`, `active = mc_music.finished` is not the current key, `loading = ui.loading_world`; a `Start` resolves the file with `music_file` and sets `mc_music.wanted`; a `Stop` clears it. When the credits close (`ui.credits` goes from `Some` to `None`) call `stop_playing` if the credits play.
  - `stop()` (`:2878`) and the torn-down path clear `mc_music.wanted` and reset the manager.
- **Decisions:** the music volume is the game's master volume times the same mix gain as the other Minecraft sounds (vanilla's defaults put music and sounds both at 100%). There is no "now playing" toast. A missing file (no internet at first start) logs once and plays nothing.
- **Tests** (`minecraft_music`): `the_boss_music_starts_on_the_tick_its_bar_shows_and_loops`, `the_end_music_replaces_the_boss_music_within_3000_ticks_of_the_kill`, `arriving_in_the_end_without_a_fight_waits_6100_to_24100_ticks`, `the_credits_music_replaces_anything_and_stops_with_the_credits`, `a_dimension_change_stops_the_music`, `nothing_counts_down_while_a_level_loads`, `outside_the_end_nothing_new_starts_but_the_current_track_plays_on`. `audio`: `the_music_stream_decodes_the_same_samples_as_the_whole_file_decoder` (with `MINECRAFTOSS_ROOT`, on `ambient/weather/end_flash1.ogg`: same channel count, rate, length and first samples). `frame`: the boss bar test passes the new flag.

### Group 9 (optional, may move to a later round). "Minecraft ender dragon dissolves while it dies"

Only if groups 1 to 8 are in. It needs a second texture sample in the entity shader.
- `minecraft_terrain/src/mesh.rs`: register `minecraft:entity/enderdragon/dragon_exploding` (`:505-506`); `ChunkMesh.dissolve: Vec<(Range<u32>, f32)>`; `section_vertices` (`:94-106`) writes `pad[1] = 1 + floor(threshold · 254)` for those ranges (the test at `:3614-3618` that expects `pad[1] == 0` keeps passing for meshes without dissolve).
- `minecraft_terrain/src/dragon_render.rs` `append_dragons` (`:349`): while `death_time > 0`, push the body's range with `1 − (death_time + partial)/200`.
- `render_gpu/src/drawsurf/minecraft_world.rs`: `TerrainView.dissolve: vec4` (the atlas UV offset from the dragon tile to the mask tile, filled from the atlas); the vertex passes `flags.y` on; `opaque()` discards when `flags.y > 0` and the threshold is below the mask's alpha at `uv + offset`. Both tiles are 256x256, so one offset maps them.
- **Tests:** `mesh`: `a_dissolving_range_flags_its_vertices_with_the_threshold`; `render_gpu`: `the_dissolve_discards_where_the_mask_is_above_the_threshold` (WGSL rule ported to a Rust helper).

### Group 10. "The Minecraft state doc covers the End flashes, the dragon's effects and the End music"

- `docs/MINECRAFT-SURVIVAL.md`: under the End (`:180-191`), the flash, the breath particles, the death particles, the knockback and the music; remove from `:282` "the End sky has no flashes" (A1 edits the same line: merge by hand), from `:283` the knockback, the mouth breath, the fireball trail, the wall explosion particle, the death particles (and the dissolve if group 9 is in) and "There is no boss music", and from `:284` "and no music" for the credits.
- The track log entry with the jshell lines used for group 1's test values.

## 5. The owner's test steps

Kit for the whole item (11 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:golden_apple*16,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots+feather_falling=4,minecraft:diamond_sword+sharpness=5,minecraft:cobblestone*128,minecraft:oak_planks*64,minecraft:water_bucket,minecraft:end_crystal*4
```

1. Start the game once with an internet connection. In `C:\Games\Mashup\2010-Rust-Rewrite-Mashup\iw4l-artifacts\minecraft-26.3\resourcepacks\local\minecraft-26.3\assets\minecraft\sounds\music\game\end\` there are now `alpha.ogg`, `boss.ogg` and `the_end.ogg`, and the game log says `Minecraft End music ready`.
2. In the `.env`, set `IW4L_MINECRAFT_WORLD=c3fx` (a new world), `IW4L_MINECRAFT_DIMENSION=the_end` and the kit above. `/difficulty normal`. Start on the obsidian platform.
3. As soon as the pink "Ender Dragon" bar shows, the boss music starts. When it ends it starts again at once.
4. Stand still and watch the island for a minute and a half (in a new world the End clock's first 30 seconds have no flash, as in vanilla). About every 30 seconds the end stone and the dragon turn faintly purple for 5 to 19 seconds, rising and fading. About 1.5 seconds after each glow starts, a deep rumble comes from one side (above you). The sky itself stays the dark fog colour while the bar shows, as before. **Screenshot 1** at the brightest moment.
5. Walk onto the island and let the dragon swoop at you. Its purple fireball leaves a trail of purple puffs. Where it bursts, purple puffs spray out low around the spot, and the cloud on the ground is made of slowly rising purple puffs (no more round sparkles). **Screenshot 2** (the trail or the cloud).
6. Shoot the flying dragon from the side with a rapid-fire weapon as it passes: it lurches away from you with each burst. Hit its head with the sword when it swoops low: it jolts away.
7. Shoot all ten crystals. When it comes in to land on the pillar, purple breath streams from its mouth all the way down. **Screenshot 3.** When it flames, four quick purple bursts pour from its mouth toward the ground at the start of each flame. **Screenshot 4.** Shooting it while it sits does not move it.
8. Build a wall of cobblestone and oak planks about 6 wide and 20 high under its circle (40 blocks from the centre, top at about 80 to 90). Where it tears through, small explosion flashes pop at the broken blocks.
9. Kill it. While it rises under the white rays, explosion flashes flicker around its body every moment, and in its last second big clusters of explosions burst. **Screenshot 5.** The bar goes and the boss music stops. Within two and a half minutes the calmer End music starts.
10. Stand on the island after the kill and look around the sky. About every 30 seconds a soft purple glow appears somewhere between straight up and just below the horizon, swells and fades over 5 to 19 seconds, the island glows purple with it, and the rumble follows from that direction. **Screenshot 6** with the glow in view.
11. Place the 4 end crystals on the exit portal's rim (one on each side). When the new dragon's bar shows, the boss music cuts in at once.
12. Kill that dragon too (screenshot not needed), then walk into the exit portal. The credits play their own music from the start. Press Escape: the music stops, and in the overworld no music plays.
13. Quit, remove the `IW4L_MINECRAFT_DIMENSION` line.

If group 9 is in: in step 9 the dragon's body is eaten away by a dark pattern from the edges until only its glowing eyes and the rays are left.

## 6. Risks, shared files and decisions

**Shared files (merge one track at a time, expect small conflicts):**
- `minecraft_terrain/src/server.rs` `EntitySnapshot` (`:211-224`) and its one-line literal at `:2837`: A1 (gateways), C1 (shulker bullets) and C2 (item frames) all add fields. Append new fields at the end of both; the conflict is mechanical.
- `minecraft_survival/src/entities.rs`: A1, A2, A5, B2, C1, C2 all touch it. Keep this item's edits inside the snapshot block (`:906-916`), the cloud loop (`:851-853`) and `punch`/`shoot`.
- `render_anim/src/minecraft_world.rs`: every track. Keep to the lines named above; the `update` signature change (one more resource) is the most likely conflict.
- `minecraft_terrain/src/ambient_particles.rs`: A1 may add `end_gateway` particles through `display_ticks.rs`; if it also adds a `Kind`, merge the enum by hand.
- `docs/MINECRAFT-SURVIVAL.md`: c1's and c2's docs commits run before group 10 in this track, so find the lines by their text, not by the e2313625 numbers.
- `minecraft_terrain/src/mesh.rs` texture registration (`:503-506`): A1 and C2 add textures there (only group 9 touches it here).
- `render_gpu/src/drawsurf/minecraft_world.rs` and `render/src/extract.rs`: c5 group 10 in this track adds a weather pipeline, frame fields and bindings to the same file and copies them in `extract.rs`; group 9 here edits `opaque()`. Run c5's weather after this item and keep each edit to its own functions. D (skating) does not touch it.
- `minecraft_terrain/src/day_cycle.rs` `SkyState`: group 1 adds `end_flash` to every `SkyState { .. }` literal; c5 group 4 (planned before this item in the track) adds `lava_fog::in_powder_snow`, so grep `SkyState {` after it lands and give that literal a zero `end_flash` too.
- `minecraft_terrain/src/ambient_particles.rs`: c1 group 7 (end rods, beside it) and c5 groups 2 and 5 (`Snowflake`, `Totem`) add kinds in this track. Add each `Kind` at the end of the enum.
- `docs/MINECRAFT-SURVIVAL.md:282-284`: A1 edits `:282`, C1 and C2 the same block.
- `frame/src/minecraft_ui.rs` and `render_anim/src/minecraft_end.rs` (boss bar): no other brief.

**Per-player and server tick boundary (track E):**
- The flash is a pure function of the End clock. The host reads `Globals.end_clock`; a friend's client needs the End clock ticks from the server (E's per-level clock message). Nothing else is needed, and no state is per player.
- The music manager is client state in `render_anim` (vanilla's is client-only too), driven by the local player's dimension, the local boss bar (`DragonFight::boss_bar(uuid)` is already per player) and the local credits.
- The breath and explosion particles are level events in `EntitySnapshot` (`dragon_breath`, `dragon_explosions`), drawn with client randoms, like `breath_bursts`. When E serializes the snapshot, the new types need `serde`.
- `EndDamage.from` crosses `Command::EndHit`. The blame stays `PLAYER` in `entities.rs`, unchanged; E replaces it with the hitting player's id there, and `from` is that player's eye or gun.

**Risks:**
- **Random order:** group 5's draws from the dragon's random change the sequence after its death; tests that step a dying dragon and then expect later draws must be updated, and that is correct (vanilla draws them).
- **Particle count:** a flame adds 192 particles and a landing 8 a tick; the burst 200. The ambient cap is 16384 (`ambient_particles.rs:9`), far above.
- **Streaming decode:** rodio pulls from the audio thread. The decoder must not allocate per sample; decode a packet into a reusable `SampleBuffer` and hand out its samples. A decode error ends the track (the manager then counts down for the next one).
- **First start offline:** no music, no failure; the fetch is tried again on the next start.
- **Boss fog hides the sky flash during the fight** (vanilla); the owner may expect a visible beam then. Step 4 says so.
- **Group 1 is the biggest commit** (sky, atlas, uniform, light). If it grows too large, split the light (`light_boost`) into its own commit after the sky quad.

**Decisions (vanilla-like where there was a choice):**
- The flash state is recomputed from the clock on each client, never sent or saved, as in vanilla.
- The flash sound is placed once at its start (group 2).
- `hideLightningFlash` has no option here; flashes always show.
- Breath clouds switch from spell sparkles to dragon's breath particles; potion clouds keep the spell particles.
- MW2 bullets knock the dragon back from the shooter (group 6); blasts don't.
- Only the three End tracks are fetched; other music stays out (group 7).
- The dissolve is optional and last (group 9); everything else stands without it.
