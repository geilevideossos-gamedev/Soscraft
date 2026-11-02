# Track E3: known gaps from the release test files

Track E3 closes the known gaps that the release test files list as open: `docs/round9/release-1-tests.md`, `docs/round9/release-2-tests.md` and `docs/round10/release-1-tests.md` to `release-6-tests.md`, plus the "left for later" lines of the round 10 track logs (D, D0, W, E1, E2, F, M1). Every collected gap was checked against `minecraft-survival` this round. Some are already fixed, owned by another track, roadmap work, by design or too big for a fix; the rest are planned here in 16 areas, each with its commit groups, the vanilla 26.3 facts (javap on the 26.3 `client.jar`), the tests, the owner's in-game steps and a kit line. "Gap status" lists every gap with what happens to it, so none is dropped silently.

- **Base:** `minecraft-survival`. When this brief was revised it was at 80aabd8e: E2's e2b (44d96765) and D0's d0b (c0366f3a, then 558d59b8) are merged, release 6 is built with `PROTOCOL` 11 (789f6d23, 80aabd8e), W's w3 is running, and M1, M2 and M3 run their segments. d0b changed `server.rs`, `world.rs`, `walk_path.rs`, `entity_view.rs` and `families.rs`, so anchors taken at 44d96765 or earlier have moved there.
- **Worktree:** `C:\Dev\Soscraft\mc-wt\track-e3`, branch `track-e3`, target dir `D:\iw4l-wt\track-e3`.
- **Log:** `docs/round10/track-e3-log.md`: commits, decisions, "## Protocol", "State doc changes", the owner's steps and one combined kit line per segment.
- **Runs:** 26 segments, `docs/round10/runs/e3a.json` to `e3z.json`, each run from the base tag `r10-<segment>-base` (for example `r10-e3a-base`) with the saved `track-runner` workflow.

**Order of the sections.** Small, safe and high-value areas come first; each section keeps the order of its own groups.

1. **E3-1. Tests, tooling, protocol and docs** (4 groups). First: a clippy script for the vendored crates and the old clippy and rustc warnings fixed, so every later review starts from a clean baseline. Its docs group runs last.
2. **E3-2. Horses, mounts, riding and leads** (5 groups). rider-falls and mount-odds change the horse landing and `world/riding.rs`, which M3's m3d edits, so they must merge before m3d starts. `m3d.json` has no such wait and E3 may not edit it, so the orchestrator decides before e3a starts (see the segment table).
3. **E3-3. Margit and the Elden Ring bosses** (3 groups). Three small groups in Margit's own files: water and lava, a spawn egg, heals punished.
4. **E3-4. Friends, multiplayer views and skating** (4 groups). For the internet test with friends (roadmap step 2): frost outline, jump bar, skaters, Tab suggestions, the tick state and smaller inventory updates.
5. **E3-5. Players: melee, PvP, pushing and cramming** (5 groups). Melee recoil, Fire Aspect, pushing, cramming and MW2 kill credit between players.
6. **E3-6. Guns, grenades and MW2 items** (3 groups). Armour wear from every hit, bullets and the knife on mobs and knots, the tactical insertion.
7. **E3-7. Explosions, withers, the dragon and fireballs** (8 groups). MW2 blasts pushing players, blast owners, fireballs, the wither and the dragon.
8. **E3-8. Doors, blocks, dispensers and redstone** (6 groups). Placement facing, outlines, pressure plates, dispensers, named containers, door cracks.
9. **E3-9. Fireworks, projectiles and bells** (6 groups). Hits on registered mobs, rockets, target blocks, projectiles breaking blocks.
10. **E3-10. Raids, illagers, villagers and trading** (4 groups). Evoker fangs and vexes, omen particles, the captain's banner, explorer maps.
11. **E3-11. Mobs, mob AI, spawning and the mob registry** (6 groups). Age locks and Silent, blame for sent-back shots, floating and jump boost, structure zombies, wolf armour, rabbits' bites on carrots.
12. **E3-12. Items, tooltips, kits, crafting, effects and loot** (12 groups). Item looks, limits, recipes, food, enchantments, name tags, heads, pots, hive bees and the trial potion effects.
13. **E3-13. Screens, settings, commands and statistics** (7 groups). Commands, the recipe book and the F3 screen.
14. **E3-14. Saving, maps, trial chambers, vaults and compasses** (8 groups). Trial spawner drops, the trial clock, vaults, lodestones, saved projectiles, map markers.
15. **E3-15. Particles, models and drawing** (8 groups). Particles, held items, dyes, hearts, compasses and the first-person hand.
16. **E3-16. Sculk, vibrations, darkness and the warden** (11 groups). The darkness icon, the warden and the remaining vibrations: the largest area, last.

The sections were researched and written separately, so each one names the commit its file:line anchors come from, and a few repeat the shared points below in their own words. Owner screenshot names such as "Screenshot E3-1" in E3-4 or `e3-9-1` in E3-1 are file names for the owner's screenshots, not section numbers.

**Segments.** Groups run in the order of this brief. A group that waits for another track's merge (W's w3, M3's m3a, M1's m1c, m1d and m1i, M2's m2i) sits in a later segment that names the wait. E2's e2b and D0's d0b are merged and release 6 is built, so every "waits for e2b", "after d0b" and "release 6 built" in the sections is met.

**M3's m3d and e3a/e3b.** rider-falls (e3a) and mount-odds (e3b) must merge before M3's m3d starts, and `m3d.json` (which may run right after m3b) carries no such wait. Before e3a starts, the orchestrator picks one: add "starts only after E3's e3a and e3b are merged" to `m3d.json`'s notes and to `docs/PROGRESS.md` "How to resume", or, if m3d has already started, take rider-falls and mount-odds out of e3a and e3b and run them in e3z after M3's m3e merge. A run never makes this choice once it has started.

| Segment | Groups (section) | Commits | Waits for |
|---|---|---|---|
| e3a | tool-clippy-vendored (E3-1), tool-lints-terrain (E3-1), tool-lints-fork (E3-1), rider-falls (E3-2) | 8 | nothing open; rider-falls only while M3's m3d has not started (see "M3's m3d and e3a/e3b") |
| e3b | mount-odds (E3-2), mount-viewers (E3-2), mount-figure (E3-2) | 7 | nothing open; mount-odds only while M3's m3d has not started |
| e3c | margit-water (E3-3), margit-egg (E3-3), margit-heals (E3-3) | 6 | nothing open |
| e3d | friend-hud (E3-4), friend-suggestions (E3-4), friend-tick-state (E3-4), friend-slot-changes (E3-4) | 7 | nothing open |
| e3e | pl-melee (E3-5), pl-fire-aspect (E3-5), pl-pushing (E3-5) | 7 | nothing open |
| e3f | pl-cramming (E3-5), pl-kill-credit (E3-5), guns-armour (E3-6), guns-targets (E3-6) | 8 | nothing open |
| e3g | guns-flare (E3-6), blast-push (E3-7), blast-owners (E3-7) | 8 | nothing open |
| e3h | fireball-owners (E3-7), fireball-trails (E3-7), boss-darkening (E3-7), dragon-potions (E3-7), wither-flight (E3-7), wither-armour (E3-7) | 8 | M3's m3a merged (wither-armour); D0's d0b (wither-flight) is merged (c0366f3a) |
| e3i | placed-facing (E3-8), cross-outlines (E3-8), plate-bodies (E3-8), dispenser-blocks (E3-8) | 9 | nothing open |
| e3j | placed-names (E3-8), door-cracks (E3-8), registered-hits (E3-9) | 6 | nothing open |
| e3k | rocket-end-targets (E3-9), elytra-rockets (E3-9), angled-rockets (E3-9), target-blocks (E3-9), knot-hits (E3-2) | 8 | W's w3 merged (w3's events-items edits `tick_rockets` and the rocket hits) |
| e3l | evoker-summons (E3-10), omen-particles (E3-10), raid-banners (E3-10), cartographer-maps (E3-10) | 8 | nothing open |
| e3m | mob-age-silent (E3-11), mob-shots-blame (E3-11), mob-floats-jumps (E3-11) | 9 | nothing open |
| e3n | structure-zombies (E3-11), wolf-armour (E3-11), firework-looks (E3-12) | 7 | nothing open |
| e3o | component-limits (E3-12), item-rarity (E3-12), book-dye-recipes (E3-12) | 7 | nothing open |
| e3p | food-effects (E3-12), tool-enchantments (E3-12), name-tags (E3-12) | 7 | nothing open |
| e3q | heads-hives (E3-12), decorated-pots (E3-12), projectile-breaks (E3-9), trial-effects (E3-12) | 9 | nothing open (projectile-breaks needs decorated-pots' placed pot sides) |
| e3r | setworldspawn (E3-13), book-keys (E3-13), debug-screen (E3-13), item-replace (E3-13), recipe-bounce (E3-13) | 8 | nothing open |
| e3s | kill-selectors (E3-13), oven-book (E3-13), trial-arrows (E3-14) | 8 | W's w3 merged (trial-arrows); D0's d0b (kill-selectors) is merged |
| e3t | ominous-shots (E3-14), trial-clock (E3-14), vault-looks (E3-14), lodestones-unloaded (E3-14), thrown-saved (E3-14) | 8 | W's w3 merged; D0's d0b is merged |
| e3u | map-banners (E3-14), draw-particles (E3-15), draw-mob-held (E3-15) | 8 | nothing open |
| e3v | draw-dyed (E3-15), draw-birth-hearts (E3-15), draw-ride-heads (E3-15), draw-compass (E3-15) | 7 | nothing open |
| e3w | draw-hand (E3-15), draw-map-light (E3-15), sculk-darkness-icon (E3-16), sculk-warden (E3-16) | 8 | M1's m1d merged (sculk-warden commit 2 builds on goats-ram's `MeleePush.blocked`) |
| e3x | sculk-visuals (E3-16), sculk-thrower (E3-16), sculk-sources (E3-16), sculk-projectiles (E3-16), sculk-mob-events (E3-16) | 9 | W's w3 merged (sculk-mob-events) |
| e3y | sculk-deaths (E3-16), sculk-saves (E3-16), sculk-blocks (E3-16) | 8 | nothing open |
| e3z | sculk-pots-shelves (E3-16), map-frames (E3-14), trial-effects-players (E3-12), hive-bees (E3-12), rabbit-nibble (E3-11) | 8 | M2's m2i merged (trial-effects-players); M1's m1i merged (hive-bees); M1's m1c merged (rabbit-nibble) |

## E3-0. Shared points

Read this before any group. Each section adds the points of its own area; where a section is more specific, it wins for its groups.

### Code rules

- **No code comments** of any kind: no `//`, `/* */`, `///` or `//!`, in Rust, WGSL or PowerShell. When a vendored doc comment goes stale, delete its line and write nothing new.
- **No rustfmt on existing files.** Never run `rustfmt` or `cargo fmt` on a file that exists. Edits change only the lines they need, in the surrounding style. A new file must pass `rustfmt --check` on its staged content.
- **New things next to related ones.** New enum variants, struct fields, match arms, constants and list entries go next to a related one, never at the end of a shared list, so merges with the parallel tracks stay small. The only exceptions are lists whose position is their meaning, which their sections name: `ToClient` variants (the codec pins their wire tags, E3-4), `McAction` and `INPUT_COMMAND_NAMES` (command ids are positions, E3-13) and `ambient_particles::Kind` (appended by that enum's own convention, E3-10 and E3-15).
- **Commits.** One commit per subject, with the brief's subject as written: plain English, no prefix.
- **PROTOCOL is only noted.** No commit bumps `minecraft_net::PROTOCOL`; the release step bumps it once. A commit that changes a `ToClient` or `ToServer` message, a serde struct friends decode or the codec adds a line under "## Protocol" in the log.
- **Vanilla facts** come from javap on the 26.3 `client.jar` (`C:\Games\Mashup\2010-Rust-Rewrite-Mashup\iw4l-artifacts\minecraft-26.3\client.jar`, read with JDK 21's javap) and from the 26.3 data, read at runtime. Never commit a Mojang file: tables hold names, the data is read when the game runs.
- **Vendored code.** Count every new and patched file under `third_party/minecraftoss` in the log.
- **Game events** go through W's helpers, one line per site, after the state change it reports (W0 rule 1 in `docs/round10/track-w.md`). When a site another track adds merges second, whichever merges second adds the line (W0 rule 3).

### Builds

- Cargo only through `scripts\cargo-slot.ps1` from PowerShell, with `+target=D:\iw4l-wt\track-e3`. `check -p` and `test -p` on the crates a commit changes; `check -p launcher` before a commit that touches game code; clippy on the touched crates once per segment. Once E3-1's tool-clippy-vendored has landed, the vendored crates are linted through `scripts\clippy-vendored.ps1`.
- LOAD: builds run at `-j 4` per slot; never a second cargo command, the game or another long CPU-heavy process while a build runs.
- Tests go in the crate that owns the site. Tests that need the 26.3 data skip without `MINECRAFTOSS_ROOT`.

### Anchors and waits

- Each section names the commit its file:line anchors come from (9c6efebd, 4301058a, d565a3d2 or 44d96765). E2's e2b (44d96765) and D0's d0b (c0366f3a) are merged since, and W's w3 and the M tracks keep merging while E3 runs: re-check every anchor before editing. d0b moved lines in `server.rs` (+159), `world.rs`, `walk_path.rs`, `entity_view.rs` and `families.rs`.
- Every "waits for e2b", "after d0b" and "release 6 built" in the sections is met. The run specs put the groups that wait for w3, M3's m3a, M1's m1c, m1d or m1i, or M2's m2i into later segments that name the wait.

### Hooks other tracks own

`docs/round10/decisions.md` ("The M briefs give every cross-track hook one owner") gives every shared hook one owner, so the parallel tracks change different lines and merge cleanly. E3 calls these hooks but does not edit, move or reorder them. If a group needs one changed, it writes the need in the log and leaves the hook alone.

- **M1:** `PlayerControls.sprinting` and the survival `PlayerView.sprinting`, next to `sneaking` (E3's new fields go right after `jumping`); `PlayerCandidate` and its literals; the target-slot lines in `monster_ai.rs` and `wolf_ai.rs`; the `World` trait lines in `player/src/lib.rs` (98-238); the loot-context rewrite (`EntityLootContext.damage_type`, `damage_source_properties`); `MeleePush.blocked` and the `landing` lines (goats-ram, m1d: sculk-warden builds on it and adds no field); bees and beehives (`world/src/level/beehive.rs`, m1i: hive-bees builds on it); `World::destroy_effect` (foxes-hunt, m1c: rabbit-nibble uses it); the rabbit family, whose carrot nibble site M1's brief left to "whichever merges second" (rabbit-nibble adds those two lines).
- **M2:** `EffectKind`, its `ALL` length and the player-effect queue, until m2i; `EffectGrant`'s `any_mode` and `cue`; the water hooks (`swim.rs`, `Breath`, `swim_sounds`, `quiet_steps`, `OWN_DYING_TRAVEL`, `travel_swimming`, `move_self`); `movement.rs` `travel_air_flying`, `restitute` and `Body.physics` (`BodyPhysics`, m2e: its restitute posts a block bounce's `bounce`); the family knockback hook (m2e `sulfur-cubes-kicks`: `Family::knockback`, `Knock`, `Knocker`, `EntityWorld::player_knocker`, asked first by the three `knock_back` callers in `combat.rs`, `mob_hits_mob`, `apply_arrow_hit` and `hurt_by_blast`); m2a's `walk_path.rs` refactor; the nautilus lines in `mount_inventory.rs` and `Command::MountInventory`; the zombie nautilus jockey (`spawning.rs:1583`); the fish bucket arm in `dispenser.rs`; `set_dolphins_grace` in `frame.rs` and `carrier.rs`.
- **M3:** the cow store's horse-family lines, `world/equines.rs`, `horse_render.rs` and the horse arms of `world/riding.rs` (m3d, m3e); `walk_target`; `finalize_husk`; `travel_water_slowed`; the copper golem build (`golem_build.rs:191-199`); the creaking heart; the additive pass of `lightning-drawn`.
- **D0:** the registry's generic arms (`pushing.rs` `push_body`, the arms of `exposed_mut`, `pushes_on_local_player`) and `flying.rs`, which M1 and M3 share.
- **W:** the game event helpers and w3's sites: lead knots, firework rocket launches, the ominous item spawner, mob equipment, villagers and piglins at doors, silverfish in infested blocks, and bells rung by projectiles.

The few single lines E3 must change in these files are named in their groups. Whichever track merges second keeps both sides.
- **M3's horse code:** in `world/equines.rs`, `:624` (the spit's shoot line) and the spit hit `:704-715` (sculk-projectiles, e3x), `:706` (target-blocks, e3k), `:724` (registered-hits, e3j) and `:989` `breed_horses` (draw-birth-hearts, e3v); the horse arm of `spawn_saved` (`server_mobs.rs:642`, its `load_age` line, mob-age-silent, e3m; m3d and m3e extend that arm's kinds, which then get the line too); `take_rider_falls` and `vehicle_health` in `world/riding.rs` and the cow store's dying branch (`world.rs:7838`) (rider-falls and mount-odds, e3a and e3b, before m3d). The horse's own fall (`horse_fall`, which M3's camels reuse) stays untouched: mob-floats-jumps logs its jump boost term for M3.
- **Elsewhere:** the `draw_glowing` callback in D0's `families.rs`, the visibility of `exposed_mut` in `pushing.rs`, and the two lines at the rabbit's nibble site in M1's `families/land/rabbits.rs` (rabbit-nibble, e3z, W0 rule 3). The registry's generic landing in `families.rs` (a shared root site) keeps its fixed safe fall distance (mob-floats-jumps logs it).
- **M2's family knockback hook:** registered-hits (e3j), evoker-summons (e3l), mob-shots-blame (e3m), ominous-shots (e3t) and sculk-warden commit 3 (e3w) edit the arms the hook sits in front of, or add `knock_back` calls. Each keeps both sides at a merge, and a new `knock_back` on a registry family goes through the hook (`Knocker` None for an ownerless hit) once m2e has merged.

### Docs

- **A run never edits** `CLAUDE.md`, `docs/MINECRAFT-SURVIVAL.md`, `docs/MULTIPLAYER.md`, `docs/PROGRESS.md` or `docs/round10/decisions.md`. New wording for them goes under "State doc changes" in `docs/round10/track-e3-log.md`, and the orchestrator copies it at round end.
- **The one exception is E3-1,** whose tooling and its docs must change together. tool-clippy-vendored and tool-lints-terrain change the clippy lines of `CLAUDE.md`, the "Per track" steps of `docs/MINECRAFT-SURVIVAL.md` and `.claude/workflows/track-runner.js`. docs-margit is not run by E3: the orchestrator copied track F's merge text into `CLAUDE.md`, `docs/MINECRAFT-SURVIVAL.md`, `docs/MULTIPLAYER.md` and `README.md` on 2026-11-02, because release 6 ships Margit and README called him not playable (decisions). `docs/PROGRESS.md` and `decisions.md` stay the orchestrator's.
- **Owner's steps.** Every group ends with the owner's in-game steps and its kit line, in its own code block (at most 27 stacks), written into the log.

### Cross-area overlaps

The areas were planned in parallel, and a few of their groups touch the same code. The later group builds on the earlier one instead of adding a second copy:

- **Ownerless projectile hits.** registered-hits (E3-9, e3j) makes the family arm of `hurt_by_typed_blast` pass `kind` (`world.rs:3078`); later groups do not edit that line again. mob-shots-blame (E3-11, e3m) adds `ownerless_projectile_hurt`, `thrown::hurt_unowned` and the burst arm for a charge whose breeze is gone. ominous-shots (E3-14, e3t) extends that same function (the `DamageSourceKind`, the closed shulker), routes `thrown::hurt_by`'s `MOB_THROWER` case through `hurt_unowned` and folds e3m's burst arm into the unguarded path, instead of adding an `unowned_projectile_hurt`.
- **Projectile shoot events.** fireball-trails (E3-7, e3h) posts `projectile_shoot` on the first tick of fireballs, wither skulls and shulker bullets, with `HasBeenShot` saved. sculk-projectiles (E3-16, e3x) adds no second shoot line for those three; it keeps its shoot lines for witch potions and llama spit, and all its landing lines. Those landing lines go beside the block-hit pushes in the shape target-blocks (E3-9, e3k) gave them (`ProjectileBlockHit`).
- **Lead knots.** guns-targets (E3-6, e3f) makes MW2 bullets and blasts break knots, which is horses-11's bullet half (E3-2's "Left out" still lists it). knot-hits (E3-2, e3k) adds arrows, thrown things and explosions and keeps that route.
- **Placed components.** placed-names (E3-8, e3j) turns the banner-only filter in `actions.rs` into `components::takes_components`. heads-hives (heads) and decorated-pots (pots) in e3q, and hive-bees (hives) in e3z, add to that helper instead of a second `keeps_components`.
- **Decorated pot sherds.** decorated-pots (E3-12, e3q) adds the player's harvest side and makes a placed pot store its sides (a plain pot item stores four bricks, the item default `PotDecorations.allBrick`). projectile-breaks (E3-9) runs right after it in e3q and adds the `minecraft:dynamic` sherds drop on the level side (core `loot.rs`, `drops.rs`). Both use the 26.3 rule (javap `DecoratedPotBlock.lambda$getDrops$0`): left, back, front, right, each through `Optional.ifPresent`, so an empty side drops nothing and a block entity without `sherds` (`PotDecorations.EMPTY`) shatters into nothing. 26.3's `PotDecorations` is a record of four optional sides and has no `ordered()`. E3-12's "Left out" line on projectiles shattering pots is superseded by projectile-breaks.
- **New `PlayerControls` fields.** margit-heals (E3-3, e3c) adds `consuming` right after `jumping`, and elytra-rockets (E3-9, e3k) adds `gliding` right after `consuming`. M1's `sprinting` stays next to `sneaking`.
- **Rockets and W's w3.** rocket-end-targets and elytra-rockets (E3-9) run in e3k, after w3's events-items, which posts the rockets' projectile events in `tick_rockets`. rocket-end-targets adds w3's rocket `projectile_land` line at its new crystal and dragon hit (W0 rule 3).
- **Mob effects.** mob-floats-jumps (E3-11, e3m) adds `MobEffect::JumpBoost`, and trial-effects (E3-12, e3q) adds its four effects after `Wither`, so `ALL` grows from 17 to 21.
- **Name tags.** name-tags (E3-12, e3p) also shows mob names over mobs, which closes drawing-18 as a HUD label (the drawing check had judged a world-space text renderer too big). The wither boss bar's name (explosions-12) is still left.

## Gap status

Every gap collected from the release test files and the track logs, with what happens to it. **Planned:** a group in this track closes it (section and segment given; a part left out is named). **Left out:** open, but E3 does not plan it, with the reason; the area's "Left out" list has the details. **Fixed, Owned, Later (roadmap), By design, Too big:** as checked against `minecraft-survival` this round; Owned means another track's brief or run covers it. Gaps that E2's e2b closed count as fixed now that e2b is merged.

| Status | Gaps |
|---|---|
| Planned | 176 |
| Left out | 26 |
| Fixed | 19 |
| Owned | 38 |
| Later (roadmap) | 15 |
| By design | 43 |
| Too big | 14 |
| **All** | **331** |

**E3-1. Tests, tooling, protocol and docs** (11 gaps)

| Gap | What | Status | Where, or why not |
|---|---|---|---|
| code-1 | minecraft_net::PROTOCOL is still 10 (crates/minecraft_net/src/messages.rs:7). | Fixed | Release 6's step bumped it to 11 (789f6d23, with the four codec asserts). E3's own wire changes wait for the next release step's bump, never a run's. |
| code-2 | Clippy warnings older than the tracks remain in the touched crates: minecraft_terrain needs -A clippy::approx_constant; block_particles.rs has len_without_is_empty; server.rs has warnings at lines 666 and 3502 and … | Planned | Groups `tool-lints-terrain` (E3-1, e3a) and `tool-lints-fork` (E3-1, e3a); left out: the upstream MW2 files' warnings. |
| code-3 | third_party/minecraftoss is outside the workspace, so cargo clippy checks its crates with plain rustc; they are linted only through RUSTC_WRAPPER=clippy-driver after touching lib.rs. | Planned | Group `tool-clippy-vendored` (E3-1, e3a); left out: the vendored crates' older warnings, [lints] tables. |
| code-4 | The friend's dial publish path in render_anim minecraft_remote has no render_anim test of its own; only the context test and the dials tests cover it. | By design | Track D's decision (docs/round10/track-d-log.md:547, Group lodestones 3 Decisions): the publish path needs the whole friend setup, so the context test and the dials tests cover its two halves |
| code-5 | The test live_shulkers_stay_far_from_players_while_dormant_monsters_go uses minecraft:phantom as its dormant monster. | Owned | Track M3 segment m3a (running). docs/round10/runs/m3a.json:29 makes the phantom commit change 'the server.rs live_shulkers test ... as decision 2 says'. track-m3.md:437 and :682 recommend keeping the … |
| code-6 | The families::interact hook has no family that uses it and no real-family test. | Fixed | Track M1 (m1a: rabbits e862fc31; polar bears feed5dab), merged into minecraft-survival |
| code-7 | families::is_vehicle in carries and the mount inventory has no real-family test, because no registered mount exists yet. | Owned | Track M2 (nautiluses): docs/round10/track-m2.md:2715 gives Nautilus `MOUNT = true`, and :2751 lists carries_loads_a_drowned_jockey_from_its_passengers as 'D0-9.4's first is_vehicle test'. |
| code-8 | FamilyDraw.held has no family that uses it and no test. | Owned | Tracks M1 (foxes-drawn: the mouth item; pandas drawn: the held item with HeldDisplay::Ground into out.held, track-m1.md:791), M2 (dolphins: 'The held item goes to out.held', track-m2.md:1198) and M3 … |
| code-9 | Shulker bullets pass their block hit on for bell rings without a test of their own (track-e2 e2b), because a bullet needs a target to fly. | By design | E2's decision (track-e2-log.md:1107: a bullet needs a target to fly), merged with e2b in 44d96765. |
| code-10 | Two vendored comments in monster_ai.rs (Prey::Absent, enderman_targets) still say that no endermites live; they were left stale on purpose. | By design | Track D0's decision (docs/round10/track-d0-log.md:236): the two vendored comments stay because the fork writes no comments and the code is right |
| code-11 | Track F's state-doc wording (docs/MULTIPLAYER.md 9.2, CLAUDE.md roadmap step 3, and the Bosses section and Mobs limitations in docs/MINECRAFT-SURVIVAL.md) was left for the merge to copy in. | Fixed | Group `docs-margit` (E3-1), done by the orchestrator on minecraft-survival on 2026-11-02; left out: other tracks' round-end wording. |

**E3-2. Horses, mounts, riding and leads** (13 gaps)

| Gap | What | Status | Where, or why not |
|---|---|---|---|
| horses-1 | The mount's model is not drawn in its inventory screen. | Planned | Group `mount-figure` (E3-2, e3b). |
| horses-2 | Only one player at a time can open a mount's inventory; vanilla allows several viewers. | Planned | Group `mount-viewers` (E3-2, e3b); left out: right-click changes while the screen is open. |
| horses-3 | A rider hears no block fall sound when the mount lands from a fall. | By design | Vanilla too: playBlockFallSound looks 0.2 below the rider's own feet, the air above the horse (E1, track-e1-log.md:476). |
| horses-4 | A mob riding a horse takes no share of the horse's fall damage: take_rider_falls skips mob passengers, so only player riders share it. | Planned | Group `rider-falls` (E3-2, e3a); left out: falls of vehicles other than horses and striders. |
| horses-5 | Players can ride only the horse family (horses, donkeys, mules, llamas) and striders, and vehicle_health covers only those, so mount hearts show only for them. | Planned | Group `mount-odds` (E3-2, e3b). |
| horses-6 | A dying horse still plays the generic fall sound through dying_travel instead of the horse land sound from AbstractHorse.causeFallDamage. | Planned | Group `mount-odds` (E3-2, e3b). |
| horses-7 | A wind charge's push does not shorten a rider's share of the horse's fall; vanilla clips it at the rider's impulse height. | Planned | Group `rider-falls` (E3-2, e3a). |
| horses-8 | There are no live zombie or skeleton horses (left to M3's undead-horses-live group). | Owned | M3 segment m3d, groups undead-horses-live, undead-horses-ride and undead-horses-drawn (track-m3.md:1300 M3-5, groups :1461-1463, 'can_gallop is true for both' :1490). decisions.md 'D0's shape' and … |
| horses-9 | A player on a registered mount is always drawn at seat 0, because survival's ride_seat calls passenger_point(vehicle) without a seat index, although the server-side seat is right. | Owned | M3 group camels-riding (segment m3e/m3f): 'Per-seat ride_seat (minecraft_survival/src/entities.rs) and the forward seat in ServerMob.vehicle', track-m3.md:87, :1714, :1902 ('ride_seat asks the … |
| horses-10 | After the fence under a lead knot breaks, the leashed mob is freed only within about 5 seconds instead of at once. | By design | Vanilla's interval: the fence is checked every 100 ticks (BlockAttachedEntity.tick, track-d-log.md:1021); release-3 L-4 expects it. |
| horses-11 | Arrows, tridents and thrown items do not break lead knots; vanilla projectiles destroy a knot. | Planned | Group `knot-hits` (E3-2, e3k); the MW2 bullet half is in guns-targets (E3-6, e3f), not left out as E3-2's list says. |
| horses-12 | Explosions do not break lead knots. | Planned | Group `knot-hits` (E3-2, e3k); left out: blasts from a source in water. |
| horses-13 | Lead knots ignore the entity_drops game rule (a broken knot always drops its leads) and the mob_griefing game rule (vanilla stops non-player damage to the knot when it is off). | Planned | Group `knot-hits` (E3-2, e3k). |

**E3-3. Margit and the Elden Ring bosses** (17 gaps)

| Gap | What | Status | Where, or why not |
|---|---|---|---|
| margit-1 | Margit is our own stand-in: a cube model with a 64x64 skin painted by code (elden_skin.rs, elden_render.rs). | Later (roadmap) | Roadmap step 3 (real Elden Ring data for Margit). |
| margit-2 | Margit's move table and numbers (health, stance, damage, timings) are our own estimates (DataSource::Estimate). | Later (roadmap) | Roadmap step 3: real numbers need NpcParam/AtkParam_Npc from the game (design doc 14.2). decisions.md:12 says his numbers stay estimates until real game data exists. |
| margit-3 | All of Margit's sounds are vanilla sounds (illusioner, evoker, ravager, iron golem step, trident hit), and his boss music is vanilla music.dragon; no ELDEN RING audio. | Later (roadmap) | Roadmap step 3: the real audio is in the game's Wwise banks under sd/ (design doc 14.2) and needs the game install and the baker. |
| margit-4 | The host does not yet load ELDEN RING assets from its own install or send them to friends. | Later (roadmap) | Roadmap step 3 and the owner list: the host loads Elden Ring assets from its own install and sends them to friends (CLAUDE.md Goal and Roadmap 3). |
| margit-5 | Margit walks through water and lava as on dry ground (no swimming or fluid movement); lava and fire still hurt him. | Planned | Group `margit-water` (E3-3, e3c); left out: waterlogged blocks in the fade check, a swimming pose. |
| margit-6 | No ELDEN RING game data baker exists (design doc 14.2), so boss data cannot be read from the game. | Later (roadmap) | Roadmap step 3 (design doc 14.2): the baker needs the game installed and unpacked. |
| margit-7 | There is no Elden Ring style bottom health bar; Margit uses a vanilla red top boss bar. | Later (roadmap) | Design doc 10 ("Later: an Elden Ring style bar at the bottom") and 17 ("Systems that come with them": the bottom bar with a damage trail) put it with the later boss work of roadmap step 3. |
| margit-8 | There is no gate arena with fog walls; he is summoned anywhere by /summon or the lantern altar. | Later (roadmap) | Design doc 11.3 and 17: a fork structure set for arenas with fog walls, found with /locate, with the later boss work of roadmap step 3. |
| margit-9 | Margit's Shackle, the boss item, does not exist. | Later (roadmap) | Design doc 11.3 and 17 ("Margit's Shackle and other boss items") put it with the later boss work of roadmap step 3. |
| margit-10 | There is no spawn egg for iw4l:margit. | Planned | Group `margit-egg` (E3-3, e3c); left out (decision): the egg does nothing on a spawner. |
| margit-11 | There is no lock-on targeting for boss fights. | Too big | A new player camera and targeting system on top of MW2 view angles, aim assist and input bindings, for host and friends. |
| margit-12 | Players have no dodge roll. | Too big | New MW2 movement code: a roll state with i-frames that every hit path must honour (Margit's blows.rs deal, mob hits, PlayerHitKind), synced to friends. |
| margit-13 | Hits have no hit-stop. | Too big | A new combat-feel layer. It would freeze the player's swing (Minecraft hand / MW2 viewmodel) and Margit's clip clock for a few frames per hit. |
| margit-14 | There is no status buildup (bleed, frost) on him or on players. | Too big | A new status system: buildup meters on him and on players, decay, procs, HUD and sync. |
| margit-15 | Margit has a single 1.4 x 3.0 hurtbox, with no extra hurtboxes or weak points beyond the MW2 headshot. | Later (roadmap) | Design doc 17 ties several hurtboxes to later bosses (Godrick "needs several hurtboxes and real data"). |
| margit-16 | Margit does not punish heals: he does not react to a player eating or drinking. | Planned | Group `margit-heals` (E3-3, e3c); left out: heals of fighters other than his target. |
| margit-17 | A fighter who is not in the level when Margit dies (for example gone to another dimension) is skipped and gets no GREAT ENEMY FELLED line or chime. | By design | Track F commit 21 chose this (track-f-log.md:758: "A fighter not in the level (gone to another dimension) is skipped"), and a test asserts it. |

**E3-4. Friends, multiplayer views and skating** (11 gaps)

| Gap | What | Status | Where, or why not |
|---|---|---|---|
| friends-1 | Friends get no command (Tab) suggestions; only the host does. | Planned | Group `friend-suggestions` (E3-4, e3d). |
| friends-2 | A friend's window does not draw the frost outline (freezing overlay) in powder snow. | Planned | Group `friend-hud` (E3-4, e3d). |
| friends-3 | A friend's horse jump bar lags behind the host by the network round trip, because it is not predicted on the friend's side. | Planned | Group `friend-hud` (E3-4, e3d). |
| friends-4 | A friend's effect icon blink (an effect running out) lags behind the host by the network round trip. | By design | Vanilla parity: no doc says so explicitly, but vanilla's client lags its server the same way, so there is nothing to port |
| friends-5 | A friend's creative inventory screen shows each change one network round trip late, like every friend menu, instead of updating at once on the client. | By design | docs/MULTIPLAYER.md:659 (friend menus don't predict, 'acceptable on localhost and with friends') and docs/round9/track-a-log.md:521 (the creative screen follows that rule on purpose) |
| friends-6 | A friend carrying a full 100-page book is sent 100 to 300 KB on every inventory change, because the whole book is sent again each time. | Planned | Group `friend-slot-changes` (E3-4, e3d). |
| friends-7 | The friend client does not know the host's tick state (frozen or stepping), so under /tick freeze other things in the friend's window can still move a little. | Planned | Group `friend-tick-state` (E3-4, e3d). |
| friends-8 | A friend must convert Skate 3 on their own PC: the friend zip has no Skate 3 assets and the host does not send them, so a friend over the internet cannot skate and sees skaters as standing soldiers without a board. | Later (roadmap) | CLAUDE.md Roadmap step 2 (friends need only the exe, Minecraft Java and MW2; the host sends its assets) and step 5 (Skate 3, starting with skating for friends) |
| friends-9 | Other players' skateboards are drawn with the trucks and wheels stiff on the deck (no truck or wheel animation). | By design | Track D decision in docs/round9/track-d-log.md:2366: the wire pose carries 23 bones (22 body bones plus SKATEBOARD_ROOT), and trucks and wheels ride the deck rigidly on other players' boards. |
| friends-10 | After a host stall of more than one second, other skaters show as standing soldiers until the next skate update arrives. | Planned | Group `friend-hud` (E3-4, e3d). |
| friends-11 | Each skater costs every other player in the same dimension about 23 KB/s; it is unclear whether this is acceptable for 4 to 5 players. | Later (roadmap) | CLAUDE.md Roadmap step 2 (the internet test with 4 to 5 players judges bandwidth) and step 5 (skating for friends); a friend over the internet can't skate until the host sends Skate 3 (friends-8) |

**E3-5. Players: melee, PvP, pushing and cramming** (13 gaps)

| Gap | What | Status | Where, or why not |
|---|---|---|---|
| players-1 | After a sprint knockback hit on a player, the attacker is not slowed (vanilla multiplies the attacker's velocity by 0.6 and stops the sprint). | Planned | Group `pl-melee` (E3-5, e3e). |
| players-2 | Fire Aspect and the other after-hit enchantment effects do not work when hitting players. | Planned | Group `pl-fire-aspect` (E3-5, e3e); left out: Thorns. |
| players-3 | A sweep attack around a mob also hits the players beside it even when the targeted mob took no damage; vanilla needs the hit to land. | Planned | Group `pl-melee` (E3-5, e3e). |
| players-4 | A player killed by Minecraft melee or TNT gives the killer no MW2 kill credit (score or killstreak). | Planned | Group `pl-kill-credit` (E3-5, e3f). |
| players-5 | The push between players has no momentum: players are moved apart directly instead of getting velocity added as in vanilla's Entity.push. | By design | Decision A7 push (docs/round9/track-a-log.md:336 and :524), following CLAUDE.md's goal of MW2 movement |
| players-6 | The player push is the same in the air and on ice as on the ground. | By design | Decision A7 push (docs/round9/track-a-log.md:524: 'is the same in the air and on ice as on the ground'), a consequence of the traced-move push and MW2 movement |
| players-7 | A player in bed is not pushed and does not push other players; in vanilla a sleeping player still pushes. | Planned | Group `pl-pushing` (E3-5, e3e). |
| players-8 | Players take no entity-cramming damage when crowded together. | Planned | Group `pl-cramming` (E3-5, e3f). |
| players-9 | A player riding a mob does not push players who are walking. | Planned | Group `pl-pushing` (E3-5, e3e). |
| players-10 | Under /tick freeze a player on foot does not push a ridden strider or horse. | Planned | Group `pl-cramming` (E3-5, e3f). |
| players-11 | The host's own first spawn in a new world is not spread around the world spawn. | Left out | Effectively closed: the host's first spawn already uses vanilla's spread within 10 blocks; only a non-default respawn_radius in a foreign world is ignored. |
| players-12 | A respawn far from the world spawn may hitch while the spawn's chunks load (test A7-15). | Left out | Never measured (A7-15 has no result); an asynchronous respawn is risky in five files. Measure first. |
| players-13 | There is no world border, so the spawn radius is never clamped to it as in vanilla. | Too big | Needs a whole world border system, not a fix: level.dat Border* fields, /worldborder, the drawn border wall, damage and pushes outside it, and the clamps that read it. |

**E3-6. Guns, grenades and MW2 items** (16 gaps)

| Gap | What | Status | Where, or why not |
|---|---|---|---|
| guns-1 | MW2 drops no gun model on death; only the Minecraft items drop. | By design | docs/WEAPONS.md 5.1 and decision 7 (round 9 track C) |
| guns-2 | Killstreak items vanish when the player dies instead of dropping or being kept. | By design | docs/WEAPONS.md 5.4 and decision 6 |
| guns-3 | Killstreak items cannot be put into chests or the offhand. | By design | docs/WEAPONS.md 5.3 and 5.4, decision 6 (offhand ban C1-8) |
| guns-4 | Gun hits do not wear down the durability of the target's armour. | Planned | Group `guns-armour` (E3-6, e3f). |
| guns-5 | Gun damage to mobs is MW2's own and is not adjusted for Minecraft mobs or their armour. | By design | Owner's C13 words and docs/WEAPONS.md 11.1 |
| guns-6 | The gun attachment, akimbo and akimbo-split crafting recipes are not in the recipe book; only the tooltips explain them. | By design | docs/WEAPONS.md 9.2 |
| guns-7 | The grindstone destroys a gun's attachments and the loaded rounds above the bare clip instead of giving them back. | By design | docs/WEAPONS.md 9.2 and decision 17 |
| guns-8 | Only the lethal and tactical grenade slots throw grenades. | By design | docs/WEAPONS.md 4.5 and 10.4 |
| guns-9 | Placed claymores and C4 are not saved and vanish when the host quits. | By design | docs/WEAPONS.md 10.4 and 15 (saving summary) |
| guns-10 | A world saved again by vanilla Minecraft loses the contents of the fork's two grenade slots. | By design | docs/WEAPONS.md 10.5 and 15; docs/round9/track-c.md:189 |
| guns-11 | A planted tactical insertion leaves no marker (flare) on the ground. | Planned | Group `guns-flare` (E3-6, e3g). |
| guns-12 | A planted tactical insertion is not saved: after a save and reload the player respawns at the bed (C2-17). | By design | docs/WEAPONS.md 10.6, decision 26 and section 15 |
| guns-13 | Only MW2 blasts destroy a planted tactical insertion; TNT and creeper explosions do not. | Planned | Group `guns-flare` (E3-6, e3g). |
| guns-14 | MW2 bullet impacts, blood and explosions in the Nether were never checked (test A1-9), so they may not work there. | Planned | Group `guns-targets` (E3-6, e3f); a check, not a commit: owner step T-10; code only if the Nether alone shows a gap. |
| guns-15 | The MW2 knife does not hit mobs, Margit included. | Planned | Group `guns-targets` (E3-6, e3f). |
| guns-16 | MW2 bullets pass through lead knots without hitting or breaking them. | Planned | Group `guns-targets` (E3-6, e3f). |

**E3-7. Explosions, withers, the dragon and fireballs** (19 gaps)

| Gap | What | Status | Where, or why not |
|---|---|---|---|
| explosions-1 | MW2 blasts (grenades, rockets, C4) push no player. | Planned | Group `blast-push` (E3-7, e3g). |
| explosions-2 | Blast Protection does not reduce the knockback an explosion gives a player, as vanilla's explosion knockback resistance does. | Planned | Group `blast-push` (E3-7, e3g). |
| explosions-3 | TNT set off by a creeper's explosion does not record the creeper as its owner, so its damage and death line do not name the creeper. | Left out | Not a gap: vanilla discards the creeper right after its blast, so its TNT names no creeper either; the release-1 limit line gets corrected (E3-7, Left out). |
| explosions-4 | An end crystal blown up by a player's TNT does not credit its explosion to that player. | Planned | Group `blast-owners` (E3-7, e3g). |
| explosions-5 | A ghast fireball that a player sends back still hurts players on a direct hit when the pvp game rule is off. | Planned | Group `fireball-owners` (E3-7, e3h). |
| explosions-6 | Primed TNT is not saved with the world, so it is lost on save and reload. | Planned | Group `blast-owners` (E3-7, e3g). |
| explosions-7 | Under /tick freeze, respawn anchor and end crystal explosions wait for the next normal tick instead of going off at once. | Planned | Group `blast-owners` (E3-7, e3g). |
| explosions-8 | The sky does not darken under a wither's boss bar (BossEvent darkenScreen is not applied). | Planned | Group `boss-darkening` (E3-7, e3h). |
| explosions-9 | Wither skulls and fireballs leave no smoke particle trail. | Planned | Group `fireball-trails` (E3-7, e3h). |
| explosions-10 | A wither skull that kills a player does not heal its wither (vanilla heals 5 on any kill). | Planned | Group `fireball-owners` (E3-7, e3h). |
| explosions-11 | Withers fly straight at their goal with no pathing, and stop at a wall until they break it or give up. | Planned | Group `wither-flight` (E3-7, e3h). |
| explosions-12 | The wither's boss bar is always named "Wither", because the fork has no name tags to rename it. | Too big | Needs name tags as a system. name-tags (E3-12, e3p) now builds them; the bar's name is still left. |
| explosions-13 | The wither's blue armour layer is drawn see-through instead of glowing (emissive), so it looks a little darker than vanilla. | Planned | Group `wither-armour` (E3-7, e3h). |
| explosions-14 | A fireball or wither skull loaded from a save finds a mob shooter only in its own dimension, and a player shooter only once that player has been in that dimension this session. | Left out | Little gain: only a reloaded, sent-back fireball whose player owner is in another dimension; mobs never change dimension in the fork. |
| explosions-15 | Fireballs, small fireballs and wither skulls do not save HasBeenShot, because the fork does not track it. | Planned | Group `fireball-trails` (E3-7, e3h). |
| explosions-16 | The drawn ender dragon, like every other mob, trails its hit boxes by about 1.5 to 3 ticks, so a shot at the drawn head can miss while it flies fast. | By design | Round 9 track A decision |
| explosions-17 | A dragon fireball can keep flying for a tick or two after it hits, before it bursts or is removed. | By design | Round 9 track A decision |
| explosions-18 | A perched dragon does not set a trident alight as vanilla's sitting phases do, because the fork's tridents cannot burn. | Left out | Visual only (one second of flame) and needs fire state on tridents and on the wire. |
| explosions-19 | Splash and lingering potions break on end crystals and dragon parts with no effect. | Planned | Group `dragon-potions` (E3-7, e3h). |

**E3-8. Doors, blocks, dispensers and redstone** (16 gaps)

| Gap | What | Status | Where, or why not |
|---|---|---|---|
| doors-1 | Drowned do not break doors, because they run their own controller instead of zombie_goals, so BreakDoorGoal never applies to them. | Left out | Drowned would need vanilla's AmphibiousNodeEvaluator, which M2 leaves for later; a door goal alone changes nothing. |
| doors-2 | In a raid a vindicator seldom breaks a door, because it opens the door in the same tick as it chops at it. | By design | E2 decision (docs/round10/track-e2-log.md:326, 'Kept from the brief: ... a raid door seldom breaks, because the open goal opens it in the same pass and that ends the chop') |
| doors-3 | When a mob opens, shuts or breaks a door, only villagers plan their paths again; vanilla tells every navigating mob about the block change. | Left out | A path recompute over about 15 mob stores; better as one registry hook after M1-M3 merge. |
| doors-4 | A mob's crack on a door disappears as soon as the mob dies or stops breaking; vanilla keeps an orphaned crack for up to 20 seconds. | Planned | Group `door-cracks` (E3-8, e3j). |
| doors-5 | A comparator, repeater, piston or calibrated sculk sensor placed by hand always faces north instead of facing from the player or along the player's look. | Planned | Group `placed-facing` (E3-8, e3i). |
| doors-6 | Players and mobs standing on pressure plates do not press them (only dropped items do), so they also make no block_activate or block_deactivate vibration. | Planned | Group `plate-bodies` (E3-8, e3i). |
| doors-7 | Walls, glass panes and iron bars still count as full blocks for the hand ray, the selection outline and the crack overlay; only fences have their post-and-arms shape. | Planned | Group `cross-outlines` (E3-8, e3i). |
| doors-8 | A cat sitting on a chest keeps only the chest screen shut: hoppers still pull items from it and comparators ignore the cat. | Left out | Nothing to see: comparators read the level's own copy of a chest, which never holds the player's items; waits for one container truth. |
| doors-9 | Only plain and trapped chests report their openers to the server level. | Owned | Track M3, segment m3c (copper-golems-sorting, commit 1): docs/round10/track-m3.md:1285 'If E2 did not make plain and copper chests count, copper-golems-sorting commit 1 does'; golem openers at … |
| doors-10 | A dispenser placing a carved pumpkin does not complete an iron or snow golem. | Planned | Group `dispenser-blocks` (E3-8, e3i). |
| doors-11 | A dispenser placing the last wither skeleton skull does not build a wither. | Planned | Group `dispenser-blocks` (E3-8, e3i). |
| doors-12 | Dispensers play no click sound and show no smoke when they dispense; only the firework shoot sound was added. | Planned | Group `dispenser-blocks` (E3-8, e3i). |
| doors-13 | The level event handler (level_events.rs) has no arms for level events 1018 (entity.blaze.shoot) and 1051 (entity.wind_charge.throw), so anything that sends them, such as a dispenser, plays no sound. | Left out | Dispensers can't be loaded in game and don't shoot fire or wind charges yet; shared projectile work for a mechanic nobody can reach. |
| doors-14 | Only banners apply a placed stack's components to their block entity (shulker boxes use their own path). | Planned | Group `placed-names` (E3-8, e3j); left out: block_entity_data, lock and container components. |
| doors-15 | A banner broken within a tick or two of being placed, before the server's block entity reaches the stream, drops plain without its patterns. | Planned | Group `placed-names` (E3-8, e3j). |
| doors-16 | Output.block_events is filled only by bell rings; vanilla sends every block event within 64 blocks. | By design | E1 decision (docs/round10/track-e1-log.md:1177 'Only bells fill it for now; it has a generic name because vanilla sends every block event within 64 blocks'); the channel is complete, and each further … |

**E3-9. Fireworks, projectiles and bells** (17 gaps)

| Gap | What | Status | Where, or why not |
|---|---|---|---|
| projectiles-1 | Players cannot shoot bows or use crossbows (only mobs shoot), so a player's crossbow never shows the pull and a player's bow never pulls; vanilla arrow test steps use a trident or the MW2 gun. | Too big | Not a fix. It needs a whole ranged-weapon system for players: bow and crossbow use and charge (today only the trident charges, minecraft_survival/src/trident.rs), player-owned arrows in the entity … |
| projectiles-2 | On minecraft-survival, arrows and thrown things do not ring bells; only hands, levers, villagers and wind charge bursts do. | Fixed | E2's e2b, merged in 44d96765 (1247a5c0, a0285a18: projectiles ring bells). |
| projectiles-3 | Snowballs, tridents and firework rockets do not hit end crystals or the ender dragon on minecraft-survival. | Planned | Group `rocket-end-targets` (E3-9, e3k). |
| projectiles-4 | Every starred firework burst draws the same generic spark burst in its stars' colours, with no star shapes (large ball, star, creeper, burst), fade colours, trails or twinkle. | Fixed | E2's e2b, merged in 44d96765 (a4abdfa5, e3acf73a: star shapes, fades, trails, twinkles). |
| projectiles-5 | A firework star with no colours draws its burst black. | By design | Matches vanilla, so nothing to fix. The release note's 'unclear' can be dropped: a star with no colours bursts black in vanilla too. |
| projectiles-6 | A firework rocket is drawn as a billboard sprite, so a rocket shot level from a dispenser looks flat; the angled rocket model is missing. | Planned | Group `angled-rockets` (E3-9, e3k). |
| projectiles-7 | A firework rocket from a player who has since left still hurts other players with pvp off, because the host no longer knows the owner was a player. | By design | Matches vanilla (E2 decision fw-launch > 3, 'as with TNT'). |
| projectiles-8 | Llama spit never hits registered mobs (breezes, silverfish, Margit), because none is in its target list. | Planned | Group `registered-hits` (E3-9, e3j). |
| projectiles-9 | hurt_for_player still gives a registry family minecraft:player_attack for thrown hits instead of the real damage type (snowball, wind charge and so on), and its family arm ignores the angers flag. | Planned | Group `registered-hits` (E3-9, e3j); the anger half matches vanilla (not a gap). |
| projectiles-10 | The EntityKey::Family arm of hurt_by_typed_blast passes blaster.damage_type() instead of kind, so registered families never see minecraft:fireworks from a firework burst. | Planned | Group `registered-hits` (E3-9, e3j). |
| projectiles-11 | Only a player's throw skips the mount the thrower rides. | By design | E2 decision (ti3-mounts > 1): mob throwers and dead or spectating throwers keep the old rule. |
| projectiles-12 | The host does not know when a player stops gliding, so an elytra rocket keeps the hand offset until it goes off; vanilla moves it to the rider's feet once the rider lands (track-e2 e2b). | Planned | Group `elytra-rockets` (E3-9, e3k). |
| projectiles-13 | An elytra rocket that bursts early on a mob or a block still boosts the player for its full time. | Planned | Group `elytra-rockets` (E3-9, e3k). |
| projectiles-14 | A second rocket used during an elytra boost extends it to the longer of the two (launch_rocket takes the maximum), while vanilla adds both boosts. | Left out | Needs a second replicated MW2 player-state field; two boosts change the glide speed by under 1%. elytra-rockets keeps a cut boost consistent with the rider's other rockets. |
| projectiles-15 | Projectile hits do not trigger target blocks; projectile_hit_block only rings bells. | Planned | Group `target-blocks` (E3-9, e3k). |
| projectiles-16 | Projectile hits do not break chorus flowers. | Planned | Group `projectile-breaks` (E3-9, e3q). |
| projectiles-17 | Projectile hits do not break decorated pots. | Planned | Group `projectile-breaks` (E3-9, e3q, right after decorated-pots). |

**E3-10. Raids, illagers, villagers and trading** (6 gaps)

| Gap | What | Status | Where, or why not |
|---|---|---|---|
| raids-1 | Evoker fangs deal damage but give no knockback. | Planned | Group `evoker-summons` (E3-10, e3l). |
| raids-2 | Vexes ignore water and lava, missing the fluid handling vanilla gives them. | Planned | Group `evoker-summons` (E3-10, e3l). |
| raids-3 | A ravager walks to the village on its own pathing even while a raider rides it, instead of the rider steering it. | Left out | Risky: needs per-goal navigation routing for every illager goal and M3's riding.rs; the visible difference is small. |
| raids-4 | Raiders do not pick up dropped ominous banners. | Planned | Group `raid-banners` (E3-10, e3l). |
| raids-5 | There are no Raid Omen particles around the player. | Planned | Group `omen-particles` (E3-10, e3l). |
| raids-6 | Cartographer villagers do not sell explorer maps; their trades still skip them. | Planned | Group `cartographer-maps` (E3-10, e3l). |

**E3-11. Mobs, mob AI, spawning and the mob registry** (41 gaps)

| Gap | What | Status | Where, or why not |
|---|---|---|---|
| mobs-1 | Silverfish never hide in stone: SilverfishMergeWithStoneGoal does only its RandomStrollGoal part, so no host block becomes infested. | Fixed | D0's d0b, merged in c0366f3a (85917c3c 'Minecraft silverfish hide in stone and wake their friends'). |
| mobs-2 | Breaking an infested block (strongholds, mountains) spawns no silverfish. | Fixed | D0's d0b, merged in c0366f3a (85917c3c). |
| mobs-3 | A hurt silverfish does not wake the silverfish hidden in infested blocks nearby: SilverfishWakeUpFriendsGoal only counts its clock down. | Fixed | D0's d0b, merged in c0366f3a (85917c3c). |
| mobs-4 | No poof particles show when a silverfish merges into stone. | Fixed | D0's d0b, merged in c0366f3a (4238dec8 'Minecraft silverfish leave a puff when they merge into stone'). |
| mobs-5 | A thrown ender pearl never spawns an endermite where it lands; vanilla has a 1 in 20 chance. | Fixed | D0's d0b, merged in c0366f3a (45de7c5d 'Endermites come from ender pearls'). |
| mobs-6 | Phantoms do not exist yet, so cats cannot scare them either. | Owned | Track M3 segment m3a (running; phantoms are M3's first group, decisions.md:6) |
| mobs-7 | Copper golems do not exist yet (M3). | Owned | Track M3 (copper golems; track-m3.md, runs m3a-m3g) |
| mobs-8 | Zombie nautilus jockeys (drowned riding a zombie nautilus) are missing. | Owned | Track M2 segment m2h, group nautiluses-live commit 2 'Minecraft nautiluses spawn in oceans and drowned ride zombie nautiluses' |
| mobs-9 | Drowned spawned in ocean ruins do not get their special trident roll. | Planned | Group `structure-zombies` (E3-11, e3n); left out: the zombie nautilus jockey (M2's m2h). |
| mobs-10 | Foxes are not in yet: fox spawn eggs hatch nothing, so polar bears hunting foxes cannot be seen (M1's later segments). | Owned | Track M1 (foxes: m1c, groups foxes-live/foxes-hunt) |
| mobs-11 | Goats are not in yet: goat spawn eggs hatch nothing (M1's later segments). | Owned | Track M1 (goats, a later m1 segment) |
| mobs-12 | Parrots are not in yet: parrot spawn eggs hatch nothing. | Owned | Track M1 segment m1h (parrots, after d0b's flying navigation 73bacd48) |
| mobs-13 | Pandas are not in minecraft-survival yet: panda spawn eggs hatch nothing. | Owned | Track M1 segment m1b (running) |
| mobs-14 | Turtles are not in yet: turtle spawn eggs hatch nothing (M1's later segments). | Owned | Track M1 (turtles, M1-8, later segment) |
| mobs-15 | Armadillos are not in yet: armadillo spawn eggs hatch nothing (M1's later segments). | Owned | Track M1 (armadillos, M1-6, later segment) |
| mobs-16 | Sniffers are not in yet: sniffer spawn eggs hatch nothing (M1's later segments). | Owned | Track M1 (sniffers, M1-7, later segment) |
| mobs-17 | Bees are not in yet: bee spawn eggs hatch nothing. | Owned | Track M1 segment m1i (bees, after d0b) |
| mobs-18 | Allays are not in the game; they wait for d0b (M3's m3f). | Owned | Track M3: allays in m3f (after d0b), allay note blocks in m3g |
| mobs-19 | Happy ghasts are not in the game; they wait for d0b (M3's m3f). | Owned | Track M3 segment m3f (happy ghasts, after d0b) |
| mobs-20 | The M2 water mobs are not in the game: squids, fish, dolphins, axolotls, frogs and tadpoles, guardians, nautiluses and the sulfur cube. | Owned | Track M2 segments m2a-m2i |
| mobs-21 | The rest of M3 (camels, the creaking with its creaking hearts, and P8 lightning) is not in the game. | Owned | Track M3: camels, creaking, lightning (m3a, cdd3cfed on track-m3) and creaking hearts (m3g) |
| mobs-22 | When frogs come, a frog will not hold its legs still mid-jump as in vanilla, because M3's walk_target hook gets no mob (left for later). | Owned | Track M2 (frogs); decisions.md:20 leaves the still legs for later because M3's walk_target hook gets no mob |
| mobs-23 | Wolves cannot wear wolf armour yet. | Planned | Group `wolf-armour` (E3-11, e3n). |
| mobs-24 | A rabbit's hop raises no dust (vanilla entity event 1, spawnSprintParticle). | Owned | Track M1 (rabbits, done in m1a). No M1 run spec (m1b-m1i) plans it: track-m1.md:3124 keeps it 'left for later' |
| mobs-25 | A rabbit's bite on a carrot does not play level event 2001, so there are no block crumbs and no break sound. | Planned | Group `rabbit-nibble` (E3-11, e3z, after M1's m1c brings `World::destroy_effect`). No M1 segment or W group plans this line (track-m1-log.md "Left for later"). |
| mobs-26 | A rabbit spawn egg cannot carry a rabbit colour (variant). | Owned | Track M1 (rabbits). Left for later in track-m1.md:3126 (the egg's minecraft:rabbit/variant component) |
| mobs-27 | The shared FloatGoal uses a fixed water height of 0.4, while vanilla's getFluidJumpThreshold is 0 when the eye is below 0.4, so baby rabbits and silverfish float like adults. | Planned | Group `mob-floats-jumps` (E3-11, e3m). |
| mobs-28 | The older mob stores do not save AgeLocked (only rabbits do), so a golden dandelion age lock is lost on reload for those mobs. | Planned | Group `mob-age-silent` (E3-11, e3m). |
| mobs-29 | MeleeAttackGoal's mustReach check is ported nowhere, so a polar bear may pick a target it cannot reach. | By design | Not a gap: vanilla 26.3 never reads mustReach on this path (checked with javap) |
| mobs-30 | Water does not slow a polar bear (vanilla's 0.98). | Owned | Track M1 (polar bears; 'left for later' in track-m1.md:539 and :3131). |
| mobs-31 | A standing (rearing) polar bear keeps its low hitbox instead of vanilla's taller client-side standing box. | Owned | Track M1 (polar bears; 'left for later' in track-m1.md:3131 and polar-bears-drawn decisions) |
| mobs-32 | Polar bears are not immune to freezing in powder snow, although vanilla lists them in #freeze_immune_entity_types. | Owned | Track M1 (polar bears; 'freeze immunity (nothing freezes yet)' in track-m1.md:3131) |
| mobs-33 | The old AngerTime key is not read when a polar bear is loaded. | Owned | Track M1 (polar bears). The same gap is shared by every old NeutralMob store, which no track owns |
| mobs-34 | Breeze targeting has no invisibility or sneaking factor, like the fork's other brains; vanilla's ATTACK_TARGET_CONDITIONS applies the visibility percent to new targets. | Left out | Risky: per-player sight data in 19 goal contexts, and PlayerCandidate is M1's this round. |
| mobs-35 | A breeze aims at a riding target at 0.3 of its height instead of vanilla's 0.8, because the brain's view of a living entity does not know whether it rides. | Planned | Group `mob-shots-blame` (E3-11, e3m). |
| mobs-36 | A breeze wind charge whose breeze is gone hurts no mob; vanilla still deals 1 damage with no cause. | Planned | Group `mob-shots-blame` (E3-11, e3m). |
| mobs-37 | The breeze long jump ignores Jump Boost (the extra lift in calculateOptimalJumpVector), because the fork's mob effects have no Jump Boost yet. | Planned | Group `mob-floats-jumps` (E3-11, e3m). |
| mobs-38 | The old mob families outside the registry treat a player's arrow or fireball (PLAYER_TARGET + id) as Target::Mob instead of Target::Player; only registered mobs blame the player as vanilla does. | Planned | Group `mob-shots-blame` (E3-11, e3m). |
| mobs-39 | check_spawn_obstruction refuses any liquid in the spawn box except for striders and drowned, while vanilla's WaterAnimal.checkSpawnObstruction asks only isUnobstructed. | Owned | Track M2, group squids-live (track-m2.md:91: one InWater line in check_spawn_obstruction) |
| mobs-40 | family_alert (HurtByTargetGoal.alertOthers for registered families) matches by kind and has neither the tamable-owner check nor the ally-of-attacker check that vanilla's alertOthers skips on. | By design | D0 decision (docs/round10/track-d0-log.md:231: 'no tamable or ally check (no registered tamable exists yet)') |
| mobs-41 | Mobs have no Silent flag, so for example a warden's heartbeat always plays; vanilla skips it for a silent warden. | Planned | Group `mob-age-silent` (E3-11, e3m). |

**E3-12. Items, tooltips, kits, crafting, effects and loot** (29 gaps)

| Gap | What | Status | Where, or why not |
|---|---|---|---|
| items-1 | The fork has no name tags, so players cannot name mobs: a 'Johnny' vindicator comes only from /summon and a named rabbit like Toast only from a loaded CustomName. | Planned | Group `name-tags` (E3-12, e3p). |
| items-2 | Written books cannot be copied: the book-cloning crafting recipe is missing. | Planned | Group `book-dye-recipes` (E3-12, e3o). |
| items-3 | Book pages are plain text, with no colours, styles or click and hover events (no JSON text components). | Too big | It needs a whole text component system (parsing, styled drawing, click and hover events), which is new work and not a fix. |
| items-4 | A text selection in the book editor is drawn as a plain blue box instead of vanilla's inverted-colour highlight. | Left out | Needs two new blend modes in the HUD GPU path for a visual nuance; the flat colour already approximates it. |
| items-5 | Book page and title limits count characters (Unicode scalars) instead of vanilla's UTF-16 units, so emoji count differently. | Planned | Group `component-limits` (E3-12, e3o). |
| items-6 | A crafted suspicious stew carries its effect, but eating it applies no effect. | Planned | Group `food-effects` (E3-12, e3p); left out: stews whose effect the player lacks (night vision, jump boost, blindness, saturation). |
| items-7 | A firework star's icon keeps the model's default grey (the minecraft:firework tint) instead of its explosion colours, although the colours are stored. | Planned | Group `firework-looks` (E3-12, e3n). |
| items-8 | Firework stars (shape, colours, fade, trail, twinkle) and firework rockets (flight duration and stars) have no tooltip lines. | Planned | Group `firework-looks` (E3-12, e3n). |
| items-9 | An unknown banner pattern id in /give, /kit or the kit components is kept; vanilla refuses it. | Planned | Group `component-limits` (E3-12, e3o). |
| items-10 | A rocket flight_duration above 255 is refused; vanilla wraps it, because it is stored as a byte. | Planned | Group `component-limits` (E3-12, e3o). |
| items-11 | Tooltips have no rarity colours for any item, so the ominous bottle's name is white instead of vanilla's yellow (uncommon). | Planned | Group `item-rarity` (E3-12, e3o). |
| items-12 | The vendored SNBT reader accepts a trailing comma in a list ([{id:"arrow"},]) instead of reporting it as malformed. | By design | matches vanilla, as the E1 E-kit decision (track-e1-log.md:41) already recorded |
| items-13 | A true has_trail or has_twinkle from the kit is stored as the number 1 in the JSON components. | Fixed | firework.rs flag() takes true or any non-zero number, tested (firework.rs:218-230). |
| items-14 | Firework stars and firework rockets cannot be crafted on minecraft-survival. | Fixed | E2's e2b, merged in 44d96765 (ceadbff0: firework star and rocket recipes). |
| items-15 | A shield cannot be decorated with a banner in the crafting grid on minecraft-survival. | Fixed | E2's e2b, merged in 44d96765 (6d7cfdaf: shield decoration). |
| items-16 | Leather items cannot be dyed in the crafting grid (minecraft:crafting_dye, leather_horse_armor_dyed and the leather armour dye recipes). | Planned | Group `book-dye-recipes` (E3-12, e3o). |
| items-17 | Eating chorus fruit does not teleport the eater, so it also posts no teleport game event. | Planned | Group `food-effects` (E3-12, e3p). |
| items-18 | The older shear sites in mob_actions.rs (cutting a lead off a mob, shearing sheep, snow golems and mooshrooms) take a flat one point of wear instead of going through durability::wear, so Unbreaking does not apply there. | Planned | Group `tool-enchantments` (E3-12, e3p). |
| items-19 | The Wind Charged effect (from the ominous lingering potions) does nothing: no wind burst when the affected mob dies. | Planned | Groups `trial-effects` (E3-12, e3q) and `trial-effects-players` (E3-12, e3z); player half in trial-effects-players. |
| items-20 | The Weaving effect does nothing: no cobwebs on death and no change to the cobweb slowdown. | Planned | Group `trial-effects` (E3-12, e3q). |
| items-21 | The Oozing effect does nothing: no slimes spawn on death. | Planned | Groups `trial-effects` (E3-12, e3q) and `trial-effects-players` (E3-12, e3z); player half in trial-effects-players. |
| items-22 | The Infested effect does nothing: no silverfish come out when the affected entity is hurt. | Planned | Group `trial-effects-players` (E3-12, e3z). |
| items-23 | A loot copy_components function from block_entity now lets the roll through, but copy_components from any other source still voids the whole roll, so that loot drops nothing. | By design | the E1 decision (track-e1-log.md:942: other sources still void the roll), and no vanilla table is affected |
| items-24 | A broken player head drops the plain item without its skin profile, because a placed head keeps no profile. | Planned | Group `heads-hives` (E3-12, e3q). |
| items-25 | An unbroken decorated pot drops a plain pot without its sherds (pot_decorations is not copied). | Planned | Group `decorated-pots` (E3-12, e3q). |
| items-26 | A cracked decorated pot still drops nothing, because the loot table's dynamic entry (the sherds) is not ported. | Planned | Group `decorated-pots` (E3-12, e3q). |
| items-27 | Bee nests and beehives still drop nothing when broken, because their loot tables use copy_state, which voids the roll. | Planned | Group `hive-bees` (E3-12, e3z, after M1's m1i hive storage). |
| items-28 | The loot function minecraft:furnace_smelt is not ported in vendored core/src/loot.rs (entity loot tables use it). | By design | the E2 maps-loot decision (track-e2-log.md:386): only entity tables use it, and they are rolled elsewhere |
| items-29 | The loot function minecraft:enchanted_count_increase is not ported in vendored core/src/loot.rs (entity loot tables use it). | Planned | Group `tool-enchantments` (E3-12, e3p). |

**E3-13. Screens, settings, commands and statistics** (12 gaps)

| Gap | What | Status | Where, or why not |
|---|---|---|---|
| screens-1 | The recipe book has no furnace (smelting) version. | Planned | Group `oven-book` (E3-13, e3s). |
| screens-2 | The recipe book has no keyboard navigation. | Planned | Group `book-keys` (E3-13, e3r); left out: the book on a gamepad. |
| screens-3 | The recipe book does not bounce or highlight newly unlocked recipes. | Planned | Group `recipe-bounce` (E3-13, e3r). |
| screens-4 | The effect list beside the inventory and the ominous bottle's tooltip turn ticks into time at a fixed 20 ticks a second, so under /tick rate they show the time at the normal rate. | Left out | The fork's player timers run at 20 a second at any tick rate, so dividing by 20 already shows the real time left; it belongs with player timers following /tick. |
| screens-5 | There is no F3 debug screen; players read their position with /tp @s ~ ~ ~. | Planned | Group `debug-screen` (E3-13, e3r); left out: F3 key combos, F3+F6, the profiler and server lines. |
| screens-6 | There are no gamma (brightness) or darkness-effect-scale options: gamma stays at 0.5 and the scale at 1.0, so the darkness brightness formula and pulse always use the defaults. | Later (roadmap) | Roadmap step 6 (the Minecraft-style main settings menu): gamma and the darkness effect scale are video and accessibility options, and track W's decision rests on the fork having no video settings |
| screens-7 | Sounds have no source categories and there are no sound category sliders, so the killer bunny's sounds cannot be turned down on their own, and the empty map's take-result sound does not use the player's sound source. | Later (roadmap) | Roadmap step 6 (main settings menu with sound sliders). |
| screens-8 | There is no left-handed main arm setting, so the elytra rocket and the hand side generally are always the right hand, unless only the offhand holds the rocket. | Later (roadmap) | Roadmap step 6 (main settings menu): the main hand is a client option in vanilla's Skin Customization settings that has to be sent to the host |
| screens-9 | The fork keeps no statistics: no drop count for thrown stacks, no FALL_ONE_CM, no bell_ring for projectile rings and no ITEM_USED for empty maps. | Too big | A whole statistics system, not a fix: a per-player counter for about 75 custom stats plus 9 typed categories (mined, crafted, used, broken, picked_up, dropped, killed, killed_by, custom), awarded from … |
| screens-10 | /kill takes only players (the minecraft_commands kill node has a players argument), so /kill @e[type=...] cannot remove mobs. | Planned | Group `kill-selectors` (E3-13, e3s); left out: @e in other commands; projectiles, the dragon and end crystals. |
| screens-11 | There is no /setworldspawn command, so for example a friend's compass cannot be seen following a moved world spawn. | Planned | Group `setworldspawn` (E3-13, e3r). |
| screens-12 | There is no /item command, so for example a banner cannot be put on a player's head (vanilla allows that only by command). | Planned | Group `item-replace` (E3-13, e3r); left out: item modify, from, fill and override, mob slots. |

**E3-14. Saving, maps, trial chambers, vaults and compasses** (23 gaps)

| Gap | What | Status | Where, or why not |
|---|---|---|---|
| world-1 | Tipped arrows that an ominous trial spawner drops from above always apply their effect at level I, because the fork's arrow effects have no amplifier; the strong slowness arrow slows at I instead of IV. | Planned | Group `trial-arrows` (E3-14, e3s). |
| world-2 | Arrows dropped from above carry only the first effect of their potion_contents (spawn_effect_arrow takes one effect); vanilla applies every effect of the potion. | Left out | No 26.3 table shoots such an arrow; carrying a list through PlayerHit and the survival Hit is risky. |
| world-3 | Arrows that an ominous trial spawner drops from above cannot be picked up, because entity-world arrows with owner 0 have no pickup. | Planned | Group `trial-arrows` (E3-14, e3s). |
| world-4 | A wind charge, snowball or egg dropped from above by an ominous spawner does not hurt a mob it hits directly, because thrown items hurt mobs only when a player threw them; vanilla's wind charge does 1 damage. | Planned | Group `ominous-shots` (E3-14, e3t). |
| world-5 | Tridents and firework rockets in an items_to_drop_when_ominous table fall as plain item entities instead of being shot as projectiles (the vanilla table never drops them). | Planned | Group `ominous-shots` (E3-14, e3t). |
| world-6 | The ominous item spot above a player uses the standing height 1.8; vanilla uses the current bounding box, which is 1.5 when crouching. | Planned | Group `trial-clock` (E3-14, e3t); left out: swimming and gliding heights. |
| world-7 | A mob that a trial spawner sends away when it turns ominous does not drop its preserved equipment (items it picked up, drop chance above 1); vanilla drops them. | Left out | No fork mob picks items up and every 26.3 trial config drops nothing, so there is nothing to drop. |
| world-8 | Each level counts only its own ticks from the world's game time, so a level without players does not advance: a trial spawner's 30-minute cooldown and next_mob_spawns_at pause while nobody is in its dimension. | Planned | Group `trial-clock` (E3-14, e3t). |
| world-9 | Unclear whether level.dat's Time (still saved from the day clock) matches the levels' game time (Globals::game_time) across a save and load. | Fixed | Checked: level.dat Time and Globals::game_time match by construction; only world-8's lag drifts (trial-clock). |
| world-10 | After a save and reload in the middle of a vault ejection, the ejection sound's pitch keeps rising to the top; vanilla starts it lower again. | Planned | Group `vault-looks` (E3-14, e3t). |
| world-11 | Vault loot rolls always use luck 0, because the fork has no luck attribute, so Luck and Unluck do not change vault loot. | Left out | No 26.3 vault table uses luck; Luck and Unluck would need M2's EffectKind this round. |
| world-12 | When a vault reward is a stack, the extra copies use the fork's dropped-item offsets (at most 0.15 per axis, none in depth for flat items) instead of vanilla's ejection, which stacks flat items in depth. | Planned | Group `vault-looks` (E3-14, e3t). |
| world-13 | A lodestone in a chunk that is not loaded always counts as still there; vanilla's LodestoneTracker.tick checks PoiManager.existsAtPosition, which reads the saved POI data. | Planned | Group `lodestones-unloaded` (E3-14, e3t). |
| world-14 | Holding a filled map shows nothing on minecraft-survival. | Fixed | E2's e2b, merged in 44d96765 (5e83e1f0: the held map in first person). |
| world-15 | Maps do not fill in as the player walks around on minecraft-survival. | Fixed | E2's e2b, merged in 44d96765 (bde09f23, 9c8c70c5: maps fill in, empty maps). |
| world-16 | Opening or generating a camp's secret chest can pause the game for up to 2 seconds while its explorer map searches for a target structure. | Left out | Moving the search off the frame needs pending menu state in E2's files; the pause is capped at 2 s, and vanilla stalls its server thread too. |
| world-17 | An explorer map search that runs out of its time budget gives no map at all. | By design | none: the 2 s budget is a recorded decision |
| world-18 | A map's banners and frames lists are kept as raw NBT exactly as read and never updated, because the fork tracks no banner or frame markers. | Planned | Groups `map-banners` (E3-14, e3u) and `map-frames` (E3-14, e3z). |
| world-19 | Banners in the world never become map markers (checkBanners is not ported), and MapMark carries no marker name, so banner names cannot reach friends. | Planned | Group `map-banners` (E3-14, e3u). |
| world-20 | Maps in item frames are not ticked (tickCarriedBy is ported without item frames), so they show no frame markers. | Planned | Group `map-frames` (E3-14, e3z). |
| world-21 | The block map colour reader does not work out the copper collections' block names; they are checked by registration place only. | By design | E2's decision (track-e2-log.md:1269): ef4fae51 created this reader and is its only commit; the copper collections are still placeholder names (`block_map_colors.rs:1117-1118`). The registration place decides the block, so every copper state still gets its vanilla colour and nothing shows in game. |
| world-22 | Our chunks save no structure starts (chunk_nbt.rs:246), so vanilla's per-start references field is neither written nor read. | Too big | needs vanilla serialization of every structure piece type, a whole new system |
| world-23 | Thrown things other than breeze wind charges (snowballs, eggs, ender pearls, potions, a player's wind charges) are not saved with their chunk. | Planned | Group `thrown-saved` (E3-14, e3t); left out: ender pearls (vanilla keeps them in the player file). |

**E3-15. Particles, models and drawing** (30 gaps)

| Gap | What | Status | Where, or why not |
|---|---|---|---|
| drawing-1 | Where held items sit in the fists (third person, other players) is a first guess to be tuned from the owner's screenshots, not taken from vanilla's item transforms. | Left out | A tuning job that needs the owner's bind-size log lines and screenshots; no release test reports the place as wrong. |
| drawing-2 | Player heads, worn or placed, always show the default skin instead of their profile's skin. | Too big | Needs a new skin system: resolve the head's profile (name or id) through Mojang's session service, download and cache the skin texture, and give each skin its own entity texture in the atlas. |
| drawing-3 | On a horse, pig, strider or camel, a worn dragon head's jaw and a piglin head's ears stay at rest; vanilla moves them with the mount's walk. | Planned | Group `draw-ride-heads` (E3-15, e3v). |
| drawing-4 | Ghasts, blazes, magma cubes and endermites never show potion effect swirls. | Fixed | Ghasts and blazes (a8700f4f), magma cubes as slimes and endermites as a registry family (9eecb2ff) all swirl now. |
| drawing-5 | Players show swirls only for the effects the game simulates, so swiftness, strength, jump boost, invisibility, night vision and water breathing give none. | Left out | M2 owns EffectKind this round, and the six effects' behaviours are a large gameplay item; next round, after M2. |
| drawing-6 | Your own effect swirls are not hidden while you scope with the spyglass. | Planned | Group `draw-particles` (E3-15, e3u). |
| drawing-7 | The host's own first-person held items still show with the UI hidden (ui_draw 0); a friend's hide as in vanilla. | Planned | Group `draw-hand` (E3-15, e3w). |
| drawing-8 | A candle put out by a wind charge burst gives no smoke particles. | By design | Vanilla gives no smoke either (the server's Level.addParticle is empty); the release-1 note can go. |
| drawing-9 | No mob reads its keyframe animations from the game's files yet. | Fixed | keyframes.rs reads AnimationDefinition from the 26.3 jar for rabbits, breezes and wardens; only the bat is left (drawing-10). |
| drawing-10 | The bat runs on the shared keyframe code but still with its old hard-coded tables, not vanilla's BatAnimation definitions (a round 9 leftover). | Left out | Nothing changes on screen: the bat's tables already equal the jar's BatAnimation, pinned by a test. |
| drawing-11 | A drowned raising its trident to throw is drawn with the plain held trident model instead of the throwing pose model. | Planned | Group `draw-mob-held` (E3-15, e3u). |
| drawing-12 | The trial spawner's detection sparks fully face the camera, so they tilt when seen from far above or below. | Planned | Group `draw-particles` (E3-15, e3u). |
| drawing-13 | Sculk charge sparks are drawn upright, without vanilla's half turn on the top and bottom faces, because ambient particles always face the camera upright. | Planned | Group `draw-particles` (E3-15, e3u). |
| drawing-14 | A compass, plain or lodestone, held by another player, in an item frame or on the ground is drawn with the default model, so its needle never turns and a lodestone compass looks plain. | Planned | Group `draw-compass` (E3-15, e3v). |
| drawing-15 | A lodestone compass's needle turns only from the second frame it shows; on its first frame it shows the plain compass's needle. | Planned | Group `draw-compass` (E3-15, e3v). |
| drawing-16 | Dyed wolf armour is drawn undyed, because the item model walker always takes on_false for a has_component condition. | Planned | Group `draw-dyed` (E3-15, e3v). |
| drawing-17 | Item frames show dyed items, such as dyed leather, undyed: FrameView carries the potion colour and the look but no dye. | Planned | Group `draw-dyed` (E3-15, e3v). |
| drawing-18 | Mob custom names are not drawn above mobs; for example the killer bunny's name is saved for vanilla but never shown. | Planned | Group `name-tags` (E3-12, e3p); names show as HUD labels when looked at; the check had judged a world-space renderer too big. |
| drawing-19 | Banner and shield patterns are drawn cut-out instead of alpha-blended, so a gradient pattern such as the blue banner's white fade ends in a hard edge near the bottom. | Left out | Needs a blended route through every item path and render_gpu; E1 chose cut-out on purpose. |
| drawing-20 | Placed banner flags hang still and do not wave (vanilla's phase is (floorMod(x*7 + y*9 + z*13 + gameTime, 100) + partial) / 100). | Left out | Plain banners are baked into their sections (E1's cost decision); waving needs every banner in view drawn apart. |
| drawing-21 | Skeletons and other mobs that are not illagers or piglins key their held items by plain id, so a skeleton's bow pull is not drawn. | Planned | Group `draw-mob-held` (E3-15, e3u). |
| drawing-22 | No love hearts show on the parent when a baby is born from breeding. | Planned | Group `draw-birth-hearts` (E3-15, e3v). |
| drawing-23 | The particles video setting (All, Decreased, Minimal) does not thin the dust a breeze kicks up, as for the fork's other mob particles. | Later (roadmap) | Roadmap step 6 (the Minecraft-style main settings menu) |
| drawing-24 | Only raiders revealed by a bell glow. The other glowing sources (spectral arrows, the Glowing effect from /effect or potions, glowing players) outline nothing. | Left out | Nothing makes a non-raider glow in the fork today; glowing players need M2's EffectKind. The raiders' held-item outline is in draw-mob-held. |
| drawing-25 | The glow outline is always white; vanilla draws it in the entity's team colour, but the fork has no teams. | By design | Vanilla's outline colour for an entity on no team is white, and the fork has no teams (track-e1-log.md:1239). |
| drawing-26 | A registered mob's glowing outline leaves out its held items, as the illager loop does. | Planned | Group `draw-mob-held` (E3-15, e3u). |
| drawing-27 | The held map is drawn at full brightness; vanilla lights it with the light at the player (RenderTypes.text). | Planned | Group `draw-map-light` (E3-15, e3w). |
| drawing-28 | The first-person arms are not drawn holding the map, as for every held item in the fork. | By design | The player model is MW2's (CLAUDE.md Goal), so there is no Minecraft arm to draw. |
| drawing-29 | The equip lowering (inverseArmHeight) is always 0 for every held item, the held map included. | Planned | Group `draw-hand` (E3-15, e3w). |
| drawing-30 | The hands do not lag behind the camera for any held item, the held map included. | Planned | Group `draw-hand` (E3-15, e3w). |

**E3-16. Sculk, vibrations, darkness and the warden** (57 gaps)

| Gap | What | Status | Where, or why not |
|---|---|---|---|
| sculk-1 | Effects carry no visibility flags, so the darkness from shriekers and from the warden's pulse shows its HUD icon; vanilla hides it (visible false, showIcon false). | Planned | Group `sculk-darkness-icon` (E3-16, e3w). |
| sculk-2 | A dropped item never makes a shrieker shriek, because dropped item entities keep no thrower for vanilla's tryGetPlayer to find. | Planned | Group `sculk-thrower` (E3-16, e3x). |
| sculk-3 | The shrieker's rising ring particles pass through a ceiling; vanilla's stop under it. | Planned | Group `sculk-visuals` (E3-16, e3x). |
| sculk-4 | A shrieker broken or replaced during a command responds one tick later than in vanilla. | By design | none: recorded decision in track-w-log Group shriekers-catalysts > 1 Decisions (line 384) |
| sculk-5 | Composters post no game events, because using a composter is not in the fork at all. | Planned | Group `sculk-blocks` (E3-16, e3y). |
| sculk-6 | Eating from a cake block posts no game events. | Planned | Group `sculk-blocks` (E3-16, e3y). |
| sculk-7 | Putting a candle out by hand posts no block_change; only wind puts candles out in the fork. | Planned | Group `sculk-blocks` (E3-16, e3y). |
| sculk-8 | Decorated pots post no game events. | Planned | Group `sculk-pots-shelves` (E3-16, e3z). |
| sculk-9 | Chiseled bookshelves post no game events when books go in or come out. | Planned | Group `sculk-pots-shelves` (E3-16, e3z). |
| sculk-10 | Jukeboxes post no game events (jukebox_play and jukebox_stop_play). | Too big | Needs a music disc playback system: disc insert and eject, the jukebox_song registry with song lengths, record audio streaming, has_record state, comparator and redstone. |
| sculk-11 | Tripwire hooks and tripwires post no game events. | Too big | Tripwire hooks and string are not simulated at all: no attach, no string scan, no entity detection, no redstone. |
| sculk-12 | Beehives post no game events. | Owned | M1 (bees and beehives; docs/round10/track-m1.md lines 2789, 2805 and 3009 list the beehive block_change, shear and fluid_pickup game events, one line each) |
| sculk-13 | Bats, parrots and bees post no flap game events; only chickens set a flap speed. | Planned | Group `sculk-mob-events` (E3-16, e3x); bats here; parrots and bees are M1's. |
| sculk-14 | Bouncing on slime blocks and beds posts no bounce game event. | Planned | Group `sculk-mob-events` (E3-16, e3x): items (the level's `restitute`); left out: mobs and players, whose `Body::restitute` M2's m2e rewrites and posts `bounce` from (`track-m2.md:1772`). |
| sculk-15 | A hurt item entity (by fire or an explosion) posts no entity_damage game event; vanilla's ItemEntity.hurtServer posts one. | Planned | Group `sculk-mob-events` (E3-16, e3x). |
| sculk-16 | Armour stands post no game events when placed, hit, equipped or broken. | Too big | Armour stands are not an entity in the fork (no placing, poses, equipment slots, breaking, drawing or saving). |
| sculk-17 | Mob armour and equipment changes post no equip or unequip game events; only player armour slots do (left to w3). | Owned | Track W segment w3, group events-mobs (docs/round10/runs/w3.json: mob equipment, vanilla LivingEntity.onEquipItem) |
| sculk-18 | The entity_interact and entity_action game events are never posted. | Planned | Group `sculk-mob-events` (E3-16, e3x); the generic mob interaction, the wolf shake and the ravager roar here; leash interactions are W's w3, the new families' actions M1-M3's. |
| sculk-19 | Witch potions thrown in the entity world post no projectile_shoot or projectile_land. | Planned | Group `sculk-projectiles` (E3-16, e3x). |
| sculk-20 | Fireballs (ghast, blaze and others) post no projectile game events. | Planned | Shoot: group `fireball-trails` (E3-7, e3h, first tick, `HasBeenShot`). Landing and the dragon's fireballs: group `sculk-projectiles` (E3-16, e3x). |
| sculk-21 | Shulker bullets post no projectile game events. | Planned | Shoot: group `fireball-trails` (E3-7, e3h). Landing: group `sculk-projectiles` (E3-16, e3x). |
| sculk-22 | Llama spit posts no projectile game events. | Planned | Group `sculk-projectiles` (E3-16, e3x). |
| sculk-23 | Firework rockets post no game events at launch or in flight; only the burst (explode) is heard (left to w3). | Owned | Track W segment w3, group events-items (firework rockets as projectiles: launched by a player, a crossbow or a dispenser, hitting a block or a mob) |
| sculk-24 | The ominous item spawner in trial chambers posts no game events (left to w3). | Owned | Track W segment w3, group events-items (the ominous item spawner) |
| sculk-25 | Lead knots post no game events when tied, punched off (ENTITY_DIE) or cut with shears (SHEAR), and shears cutting a lead (vanilla's dropAllLeashConnections(player)) post no shear event (left to w3). | Owned | Track W segment w3, group events-items (lead knots: tying, punching a knot off, shearing a lead) |
| sculk-26 | Silverfish breaking out of infested blocks post no block-destroy game event (left to w3, which waits for d0b). | Owned | Track W segment w3, group events-mobs, which waits for D0 segment d0b (silverfish hiding in and breaking out of infested blocks) |
| sculk-27 | Villagers opening and closing doors post no block_open or block_close game events (left to w3). | Owned | Track W segment w3, group events-mobs (villagers at doors) |
| sculk-28 | Piglins opening and closing doors post no game events (left to w3). | Owned | Track W segment w3, group events-mobs (piglins at doors) |
| sculk-29 | A rabbit biting a carrot crop posts no block_change game event. | Planned | Group `rabbit-nibble` (E3-11, e3z). track-m1.md line 270 left it to whichever of W and rabbits-behaviour merged second; both have merged (w1 81eaed3f, m1a a898c41c) and the nibble site (`families/land/rabbits.rs:499-501`) still has no line. |
| sculk-30 | A vibration saved while in flight arrives up to 0.8 seconds later after a load than in vanilla. | Planned | Group `sculk-saves` (E3-16, e3y). |
| sculk-31 | A vibration restored in flight after a load has its ripple particle drawn only once. | Planned | Group `sculk-saves` (E3-16, e3y). |
| sculk-32 | A shrieker that receives a vibration restored in flight after a load stays quiet, because the loaded vibration knows only its source's UUID and no player is found. | Planned | Group `sculk-saves` (E3-16, e3y). |
| sculk-33 | A vibration loaded from a save knows only its source's UUID, so the sensor's sculk_sensor_tendrils_clicking game event after a load has no source entity. | Planned | Group `sculk-saves` (E3-16, e3y). |
| sculk-34 | A wind charge burst's explode game event has no source or projectile owner, because the burst does not know its thrower; vanilla's source is the charge with its thrower, which a warden reads. | Planned | Group `sculk-projectiles` (E3-16, e3x). |
| sculk-35 | A container closed by the game's own opener recheck (recheck_lids, no player) posts no container_close; vanilla's recheckOpeners posts it with no source. | Planned | Group `sculk-sources` (E3-16, e3x). |
| sculk-36 | A bucket used on a cauldron posts its game event with the player as source; vanilla gives it no source entity. | Planned | Group `sculk-sources` (E3-16, e3x). |
| sculk-37 | An MW2 hurt that absorption hearts take entirely still posts entity_damage; vanilla posts nothing when no health is lost. | Planned | Group `sculk-sources` (E3-16, e3x). |
| sculk-38 | A mob's killing blow can lose its hurt game event (frequency 7); the death event (15) still wins, so a sensor's output is the same. | Planned | Group `sculk-deaths` (E3-16, e3y). |
| sculk-39 | entity_mount is posted only for players (ServerSim::mount); mobs getting on a mount post nothing. | By design | none: decision in track-w-log Group vibrations-more > 2 Decisions (line 281, players are the one way the fork mounts at runtime) |
| sculk-40 | A rider dropped because its mount died posts no entity_dismount. | Planned | Group `sculk-deaths` (E3-16, e3y). |
| sculk-41 | An arrow turned away (for example by a creative player) posts no projectile_land. | Planned | Group `sculk-projectiles` (E3-16, e3x). |
| sculk-42 | An MW2 bullet that hits a player posts no projectile_land; only the hurt player's own entity_damage (7) is heard. | Left out | Needs a player-hit position in upstream's MW2 combat path for a frequency-2 landing; the shot and the hurt are already heard. |
| sculk-43 | When a friend leaves with a chest open and was the last player in that level, the level stops, so the chest close is heard only when someone comes back. | By design | none: decision in track-w-log line 497 (the level stops ticking and the close goes out with its next tick, matching a level frozen when the last player left) |
| sculk-44 | The model lightmap (mc_lightmap for MW2 models, weapons and the view model in a Minecraft world) keeps a brightness mix of 0, so only the darkness pulse reaches models, not the brightness change. | By design | none: deliberate model lightmap choice |
| sculk-45 | The ender dragon's experience is not a death report, so sculk catalysts never take it or bloom for it. | Planned | Group `sculk-deaths` (E3-16, e3y). |
| sculk-46 | The projectile_hit_block seam takes no owner, so a bell rung by a projectile posts no BLOCK_CHANGE with the thrower (track-e2 e2b, to go through Level::ring_bell). | Owned | W's w3, group events-items: the bell ring gets the shooter player as source (docs/round10/runs/w3.json). |
| sculk-47 | A mob spawned by a trial spawner posts its entity_place game event with no source entity, because the mob has no id yet; vanilla uses the spawned entity. | Planned | Group `sculk-deaths` (E3-16, e3y). |
| sculk-48 | D0's Family::emits_sounds turns off a family's whole movement emission, so a family that opts out may also lose its step and swim game events. | Fixed | Done in w1 and w2: Family::emits_sounds filters sounds only, the events stay (tested). |
| sculk-49 | The vibration ripple to a warden flies to where its eyes were when the vibration was picked; vanilla's particle follows the moving warden. | Planned | Group `sculk-visuals` (E3-16, e3x). |
| sculk-50 | The warden's finalizeSpawn follow-range triangle bonus is rolled but not applied: the warden family always reads follow range 24, although eggs and /summon write the bonus into the tag. | Planned | Group `sculk-warden` (E3-16, e3w). |
| sculk-51 | The finalizeSpawn left-handed roll for a summoned warden is rolled but not applied. | Planned | Group `sculk-warden` (E3-16, e3w). |
| sculk-52 | Unknown memories in a loaded warden's Brain tag are dropped on save, as for the fork's other brains. | By design | none: vanilla's Brain codec drops them too |
| sculk-53 | A raised shield is still pushed back by the warden's extra knockback, because the shared survival MeleePush.landing throws a shielded player; vanilla's causeExtraKnockback needs a hurt. | Planned | Group `sculk-warden` (E3-16, e3w). |
| sculk-54 | The warden's own motion slows (x0.6 across) after every swing at a player, even when the player was not hurt, because the entities side does not know whether the hit landed. | Planned | Group `sculk-warden` (E3-16, e3w). |
| sculk-55 | A sonic boom on a mob goes through mob_hits_mob, so the victim's armour reduces it; vanilla's sonic_boom damage bypasses armour. | Planned | Group `sculk-warden` (E3-16, e3w). |
| sculk-56 | A witch takes the full sonic boom damage; vanilla's sonic_boom is in #witch_resistant_to, so a witch takes only 15 %. | Planned | Group `sculk-warden` (E3-16, e3w). |
| sculk-57 | The warden's five glowing layers get no red hurt overlay, because the translucent mesh's vertex alpha holds the layer alpha. | Left out | Needs an overlay lane in the translucent entity mesh and shader; a CPU tint can't add vanilla's red. |

## E3-1. Tests, tooling, protocol and docs

This area has three open gaps:

- `code-2`: old clippy and rustc warnings in the fork's crates.
- `code-3`: no clippy run reaches the vendored minecraftoss crates.
- `code-11`: F's merge text for Margit never reached the docs.

None of them changes game behaviour or the wire. There is nothing for the log's "## Protocol" and no `PROTOCOL` bump. No vanilla 26.3 behaviour stands behind the tooling and lint groups. Track F already checked the Margit facts of the docs group with javap (track-f-log.md commit 29).

Anchors are at `minecraft-survival` 9c6efebd. Check them again before editing: M1-M3, w3 and d0b move lines in `minecraft_terrain`.

**Order inside E3.** Run `tool-clippy-vendored`, `tool-lints-terrain` and `tool-lints-fork` before the other E3 areas' groups. Two reasons:
- E3's own review and verify then start from a clean clippy baseline.
- E3 groups that touch `third_party/minecraftoss` can lint it with the new script.

`docs-margit` was done by the orchestrator on 2026-11-02 (decisions), so no E3 segment runs it.

**Docs.** W0's rule tells tracks to leave the state docs alone. This area is the exception: it edits `CLAUDE.md`, `docs/MINECRAFT-SURVIVAL.md`, `docs/MULTIPLAYER.md`, `README.md` and `.claude/workflows/track-runner.js` directly.
- These are the tooling docs, which must change with the tooling, plus the merge text F left in `docs/round10/track-f-log.md:73-80`.
- No other round 10 track edits these files; they write "State doc changes" in their logs. The round-end copy of their wording goes on top.
- E3's other areas keep writing their wording under "State doc changes" in `docs/round10/track-e3-log.md`.

**Shared lines.** Every lint fix is a single line, in files that M1-M3, w3 and d0b also edit: `server.rs`, `server_mobs.rs`, `mesh.rs`, `cow_render.rs`, `mob_actions.rs`, `walk_animation.rs` and render_anim `minecraft_world.rs`.
- No fix touches a hook the M briefs own (decisions.md "The M briefs give every cross-track hook one owner").
- M3's walk target (`walk_animation.rs:21-22`) and the `mob_actions.rs` horse arm (`:176-190`) are not touched. E3 edits only `walk_animation.rs:67`, `:76` and `mob_actions.rs:68`.
- If a later merge conflicts on one of these lines, keep the other track's line and redo the same fallback on it.

### Group `tool-clippy-vendored` (S), 1 commit

**Gaps:** code-3.

**Cause**
- `Cargo.toml:9` has `exclude = ["skate", "third_party/minecraftoss"]`, so the five minecraftoss crates are path dependencies, not workspace members.
  - `cargo clippy` wraps only members with `clippy-driver`.
  - So `clippy -p minecraftoss-entities` builds the crate with plain rustc and reports nothing (track-f-log.md:560, :570; track-e1-log.md:1311).
- The only way that works today is by hand, three steps:
  1. Touch the crates' `lib.rs`.
  2. Run `check -p minecraftoss-* --all-targets` with `RUSTC_WRAPPER=clippy-driver`.
  3. Touch them again.

  A wrapper change does not change cargo's fingerprints. The check units built through clippy-driver therefore replay clippy's warnings on a later plain `check` in the same target dir (track-f-log.md:570).
- No script does this. `scripts/cargo-slot.ps1` only lists `clippy-driver` among the processes it keeps off power throttling (`:142`).
- The per-track clippy steps that would call such a script are `docs/MINECRAFT-SURVIVAL.md:491-494` and `.claude/workflows/track-runner.js:37`.
- The vendored crates have no `[lints]` table. Their editions: core, world and generator 2024; player and entities 2021.
- The root `clippy.toml` (disallowed `unwrap_or_default`) still applies: clippy-driver finds it by walking up from the crate's manifest dir. Track F confirmed this.
- The entities lib test build has about 79-84 older clippy warnings that nobody sees (track-f-log.md:1047).

**Vanilla 26.3:** n/a. This is build tooling.

**Plan**

Commit **"A script lints the vendored minecraftoss crates with clippy"**

- New `scripts/clippy-vendored.ps1`, built like `scripts/full-suite.ps1` (`$repo`, `$slot`, no comments).
  - **Arguments:** `+target=`, `+wait=` and `+exclusive` work as for `cargo-slot.ps1`. Every other argument is a crate name, short (`entities`) or full (`minecraftoss-entities`). With no crate it lints all five. An unknown name prints the usage and exits 2.
  - **Its own lint folder:** `<target>\clippy-vendored`. `<target>` is the `+target=` dir, else `$env:CARGO_TARGET_DIR`, else `D:\iw4l-target`.
    - In that folder every unit is always built through clippy-driver. When cargo replays a fresh unit, it replays clippy's warnings, just as `cargo clippy` does for members.
    - The track's own target dir never gets clippy-built units. Nothing needs touching, and later plain checks replay nothing.
    - The folder holds only the vendored crates' dependencies: serde, glam, zip and its compression crates, flate2, sha2, md-5 and `elden_boss`. There is no Bevy, so it stays small. It sits inside the track's target dir and is deleted with it.
  - **Environment:**
    - Save `$env:RUSTC_WRAPPER` and `$env:CARGO_TARGET_DIR`.
    - Set `$env:RUSTC_WRAPPER = "clippy-driver"`. cargo-slot puts `%USERPROFILE%\.cargo\bin` on the `Path` (`:78`), so the rustup proxy picks the repo's toolchain.
    - Run `& $slot +target=<lint dir> <+wait/+exclusive> check -p minecraftoss-<crate> ... --locked --all-targets`.
    - In `finally`, restore both variables, and remove `RUSTC_WRAPPER` if it was unset before. The restore matters because cargo-slot sets `CARGO_TARGET_DIR` in the caller's session (`:85`).
  - **Output:** one closing line like full-suite's: `clippy-vendored: exit <code>, <crates>, lint dir <dir>, <minutes> min`. The script exits with cargo's code.
  - **No lint flags.** The vendored crates have no `approx_constant` hit: `rg` finds only `0.017453292`, which is no std constant. The root `clippy.toml` reaches them through clippy-driver.
  - `elden_boss` is a member and a path dependency of the entities crate, so it is linted with it. It is clippy-clean.
- `docs/MINECRAFT-SURVIVAL.md`, "### Per track" step 1 (`:492`): add this sub-bullet:
  > The five minecraftoss crates are outside the workspace, so `cargo clippy` builds them with plain rustc and reports nothing for them. Lint them with `& scripts\clippy-vendored.ps1 +target=<dir> entities player` (short or full names; all five without a name). It runs `check --all-targets` through `clippy-driver` in a `clippy-vendored` folder of the target dir, so normal builds never replay its warnings. Their older warnings stay; there must be none on lines the track touched.
- `CLAUDE.md:70` ("Once per track: clippy on the touched crates"): add "(the vendored minecraftoss crates through `scripts\clippy-vendored.ps1`)".
- `.claude/workflows/track-runner.js:37`, in the verify prompt's template string only. After step 2 add: "If the track touched `third_party/minecraftoss`, also run `& .\\scripts\\clippy-vendored.ps1 +target=${A.target} <crate> <crate>` (short names like entities) and fix its warnings on lines the track added; older ones stay." Keep `${A.target}` and the doubled backslashes.

**Tests.** Log each run with its counts.
1. `& scripts\clippy-vendored.ps1 +target=<E3 target> entities`:
   - It shows clippy's warnings for the entities crate (lib, lib test, bins): about 79-84 older ones, none in Margit's files.
   - Afterwards `$env:RUSTC_WRAPPER` is empty and `$env:CARGO_TARGET_DIR` is what it was. Print both.
2. The same command again: the same warnings, replayed in seconds.
3. `& scripts\cargo-slot.ps1 +target=<E3 target> check -p minecraftoss-entities --locked --all-targets`: no clippy lint names in the output, because this is plain rustc in the track's own folder.
4. `& scripts\clippy-vendored.ps1 +target=<E3 target> nosuch`: the usage line, exit 2, and no cargo run.
5. `& scripts\clippy-vendored.ps1 +target=<E3 target>`: all five crates. Log the warnings per crate and the folder's size.

There are no Rust tests: this is tooling only. The script is new: it has no comments, and it is PowerShell, so rustfmt does not apply.

**Owner's steps:** none in game (tooling only). No kit line.

### Group `tool-lints-terrain` (M), 2 commits

**Gaps:** code-2, parts 1-3.

**Cause**
- `crates/minecraft_terrain/Cargo.toml` is the only workspace member without `[lints] workspace = true`. Every other member has it, `xtask` included.
  - So it misses `Cargo.toml:194` `approx_constant = "allow"` and `:191` `excessive_precision = "allow"`.
  - That is why every clippy call carries `'--' -A clippy::approx_constant` (`docs/MINECRAFT-SURVIVAL.md:492`, `CLAUDE.md:61`, `track-runner.js:37`).
  - The crate has no constant assertions (`rg` for `assert!(true|false|CONST`), so inheriting `:195` `assertions_on_constants = "deny"` is safe.
- About 34 older lib warnings, plus one in a test. They come from the newest cached clippy outputs of track-e2 and track-w (11-02) and were checked against 9c6efebd:

| Site | Lint | Fix (same behaviour) |
|---|---|---|
| `block_particles.rs:455` `len` | `len_without_is_empty` | add `pub fn is_empty(&self) -> bool { self.fragments.is_empty() }` after `len` |
| `server.rs:695` `merchant_view` offers | disallowed `unwrap_or_default` | `.map_or_else(Vec::new, <[_]>::to_vec)` |
| `server.rs:3989` `pickup` return type | `type_complexity` | return `Vec<PickedStack>`, the alias with the same tuple at `server.rs:4435` |
| `mob_actions.rs:68` `death_remains` loot | `unwrap_or_default` | `loot.map_or_else(Vec::new, \|book\| book.death_drops(deaths))` (W already aliased `DeathRemains`, `:60`) |
| `item_icons.rs:36` tints, `:94` language | `unwrap_or_default` | write the fallback out (see below) |
| `mesh.rs:417`, `:974` item tints | `unwrap_or_default` | `:974`: `.transpose()?.into_iter().flatten().map(..)` replaces `.unwrap_or_default().into_iter().map(..)`; `:417` the same way or `map_or_else` |
| `mesh.rs:677` block models | `map_entry` | `if let std::collections::hash_map::Entry::Vacant(slot) = models.entry(key)`, then `slot.insert(variants)` (`models` is a `HashMap`, `:479`) |
| `mesh.rs:1642` face normal | `neg_cmp_op_on_partial_ord` | `length <= 0.0 \|\| !length.is_finite()`: a NaN length still returns `"up"` through `!is_finite()` |
| `mesh.rs:1975` animated sheets | `manual_is_multiple_of` | `!sheet.width().is_multiple_of(width)` and the same for height; the `width == 0 \|\| height == 0` test before it keeps the zero case |
| `mesh.rs:2236` `ao_axes` | `unnecessary_lazy_evaluations` | `full_cube.then_some(match dir { .. })`: the match only picks constant arrays |
| `mesh.rs:2528` `fluid_quad` (8 args) | `too_many_arguments` | `#[allow(clippy::too_many_arguments)]`, as at `mesh.rs:1357`, `:2263`, `:2379` |
| `mesh.rs:2621` fluid corners `nw`, `ne`, `sw`, `se` | `needless_late_init` | `let (nw, ne, sw, se) = if h >= 1.0 { (1.0, 1.0, 1.0, 1.0) } else { ..; (fluid_corner_height(..), ..) };` |
| `model.rs:687` textures | `unwrap_or_default` | `.map_or_else(serde_json::Map::new, Clone::clone)` in place of `.cloned().unwrap_or_default()` |
| `model.rs:1002`, `pack.rs:110`, `:113` | `unnecessary_map_or` | `is_none_or(..)` in place of `map_or(true, ..)`, as F did in `arena.rs` |
| `pack.rs:204` | `unwrap_or_default` | fold into the `map` before it with `map_or_else(Vec::new, ..)` |
| `item_icon.rs:443` `raster_triangle` (10 args), `:486-487` | `too_many_arguments`, `needless_range_loop` | `#[allow(clippy::too_many_arguments, clippy::needless_range_loop)]` on the fn: `x` and `y` are pixel coordinates (`put_pixel`) as well as indices |
| `scene.rs:321` `chunk_edits`; `cow_render.rs:154` `boxes`, `:216` `horns`; `skeleton_render.rs:77` `MUSHROOMS` | `type_complexity` | a type alias next to each, as W's `DeathRemains` and F's `WalkKeys` |
| `server_mobs.rs:437` UUID | `unwrap_or_default` | `unwrap_or(&[])` (D0's pattern) |
| `server_mobs.rs:1313`, `:2158`, `:2220` | `unwrap_or_default` | write the fallback out (see below) |
| `walk_animation.rs:67`, `:76` | `unwrap_or_default` | `self.0.get(&id).map_or_else(WalkAnimation::default, \|walk\| *walk)`, dropping `.copied()` |
| `fluid.rs:830` (tests only) | `unnecessary_filter_map` | `.map(move \|z\| (x, z))` |

How to write a fallback out, using the fork's existing patterns:
- `map_or_else(<Empty>::new, f)` folded into the `map` before it (E2 `firework.rs`).
- `map_or_else(<Type>::default, ..)` (W).
- `map_or_else(BTreeMap::new, Clone::clone)` (E1).
- `unwrap_or(&[])` or `unwrap_or(0)` for borrowed or `Copy` values.
- `into_iter().flatten()` after a `transpose()?` (track C).
- Never `unwrap_or_else(Vec::new)` or `unwrap_or(Vec::new())`. Both trip `clippy::unwrap_or_default`, while `clippy.toml` disallows `unwrap_or_default` itself.

Already fixed, nothing to do: banner patterns (b38edc77, `banner_look.rs` `map_or_else`), `horse_render` `some_filter`, `item_frame`, `item_look` and `mob_tracking`.

**Vanilla 26.3:** n/a. Nothing may change in behaviour.

**Plan**

Commit 1 **"minecraft_terrain takes the workspace lints, so clippy needs no extra flag"**
- `crates/minecraft_terrain/Cargo.toml`: add `[lints]` with `workspace = true` at the end, as in `crates/minecraft_survival/Cargo.toml`. The crate's `approx_constant` warnings go with it.
- Drop ` '--' -A clippy::approx_constant` from `docs/MINECRAFT-SURVIVAL.md:492` and `.claude/workflows/track-runner.js:37`.
- `CLAUDE.md:61` keeps its point (quote the `--`) with an example that still needs it: `& scripts\cargo-slot.ps1 test -p minecraft_terrain --locked '--' darkness`.
- Add one sentence to `docs/MINECRAFT-SURVIVAL.md` "### Per track" step 1: "Since round 10 E3, `minecraft_terrain`, `minecraft_survival` and the fork's `minecraft_*` files in `render_anim` and `hud` have no clippy warnings, so any warning there is new." The rule itself stays: fix warnings on lines the track added; older ones stay. Branches made before E3 still carry the old warnings.

Commit 2 **"minecraft_terrain's older clippy warnings are fixed with no change in behaviour"**
- First run `clippy -p minecraft_terrain --locked --all-targets` once and take the list from that run. The table above is the 9c6efebd state, and merges since then may have moved lines or added warnings.
- Then fix every site with the table's fix: one line each, in the surrounding style, with no comment.

**Tests**
- No new tests: nothing changes in behaviour.
- `clippy -p minecraft_terrain --locked --all-targets`, with no extra flag: no warnings at all.
- `test -p minecraft_terrain --locked`: the same pass count as before the group (log both). The known flaky tests are `sections::tests::background_full_update_matches_the_synchronous_one` and `poof_particles::a_puff_rises_and_fades_within_its_lifetime`.
- `check -p launcher --locked`: green.
- `git diff -U0` of each commit: no comment added and no line changed beyond the warning sites.

**Owner's steps.** Release build, survival, Normal, a new world in a plains with water nearby, by day. Nothing should look or behave differently from the release before; these steps cover every place the commit edits. Name the screenshots `e3-9-1.png` and so on.
1. Look round: grass and leaves are green, water and lava animate, fences connect, and block corners are shaded. Place the water bucket on a slope and the lava bucket in a stone pit: the flowing water runs down its slopes and both surfaces animate as before. **e3-9-1**
2. Open the inventory: the cobblestone, grass block, fence and leaves icons are 3D, and the grass and leaves icons are green. Hover over them: the names show. **e3-9-2**
3. Use both cow eggs, then `/summon minecraft:cow ~ ~ ~4 {variant:"minecraft:warm"}`: the warm cow has its horns and its orange coat. Hold wheat: the cows follow you and their legs swing as they walk. Then `/summon minecraft:bogged ~ ~ ~6 {NoAI:1b}`: it has red and brown mushrooms on its head. **e3-9-3**
4. Kill a cow with the sword: it drops beef, maybe leather, and experience orbs. Walk over them: they go into your inventory.
5. Place the composter, use the villager egg beside it and wait until the villager becomes a farmer (green sparkles, a straw hat). Right-click it: the trading screen lists its offers. **e3-9-4**
6. Save and Quit, then load the world: the cows, the bogged and the farmer are where they were, and the farmer offers the same trades.

```
IW4L_MINECRAFT_KIT=minecraft:water_bucket,minecraft:lava_bucket,minecraft:cow_spawn_egg*2,minecraft:wheat*16,minecraft:villager_spawn_egg,minecraft:composter,minecraft:iron_sword,minecraft:grass_block*16,minecraft:oak_leaves*16,minecraft:oak_fence*16,minecraft:cobblestone*64,minecraft:cooked_beef*16
```

### Group `tool-lints-fork` (S), 2 commits

**Gaps:** code-2, parts 4-6.

**Cause**
- `crates/minecraft_survival/src/menus.rs:45`: `enum Open` triggers `large_enum_variant`. It is the crate's only warning (track-f-log.md:1047).
- `crates/hud/src/minecraft_screens.rs:887`: `enum SlotDraw` triggers `large_enum_variant`.
- The fork's Minecraft files in `render_anim`:
  - `minecraft_world.rs:147` `place_inventory_puppet` (8 params) and `:2293` `draw_body_gear` (9 params) lack the `#[allow(clippy::too_many_arguments)]` that `:468` and `:1921` carry.
  - `minecraft_host.rs:516`: `unwrap_or_else(Vec::new)` (`clippy::unwrap_or_default`).
  - `minecraft_inventory.rs:448`: `Result::unwrap_or_default`.
  - `minecraft_sounds.rs:75`: `Option::unwrap_or_default`.
- `check -p launcher` prints "the old render_frame and skate warnings" (track-f-log.md:570, :965, :1052). These are rustc warnings:
  - `crates/render_frame/src/retained.rs:156`: upstream 4979ef31 put the `DEPTH_HACK_SCENE_ENTNUM` static between `#[must_use]` and the function it decorated; 0f387098 had the attribute on `host_viewmodel_render_fx_flags`. Now the attribute sits on a static (`unused_attributes`), and the const fn has lost it.
  - `skate/crates/skate-core/src/animation/commands/buffers.rs:9`: deprecated `AtomicUsize::fetch_update`, renamed `try_update` (toolchain 1.99).
  - `skate/crates/skate-host/src/physics/footplant.rs:16` (`pub(super) settings: Settings`) and `skater.rs:59` (`pub respawn: super::respawn::Runtime`): `private_interfaces`, because the types are `pub(super)` (`footplant/settings.rs:4`, `respawn.rs:16`).

**Vanilla 26.3:** n/a. Nothing may change in behaviour.

**Plan**

Commit 3 **"The fork's Minecraft files in survival, render_anim and hud pass clippy"**
- `menus.rs:45`: add `#[allow(clippy::large_enum_variant)]` under the derive. There is one `Open` per player. Boxing the merchant or mount variant would change 33 match and build sites across survival, render_anim and hud, next to M2's nautilus lines.
- `minecraft_screens.rs:887`: the same allow. A `SlotDraw` is built and matched once per slot per frame.
- `minecraft_world.rs:147` and `:2293`: `#[allow(clippy::too_many_arguments)]` directly above the `fn` (they are Bevy systems). The `///` lines above `:147` stay as they are.
- `minecraft_host.rs:516`: `let edits = host.edit_books.remove(&session.client).into_iter().flatten().collect();`, as `clicks` does six lines above (`edits` is `BookEdits`, `:70`).
- `minecraft_inventory.rs:448`: `unwrap_or_else(|_| HashMap::new())`, D's pattern for a `Result` in `trial_spawner.rs`.
- `minecraft_sounds.rs`: the `.map(|sounds| ..)` at `:51` becomes `.map_or_else(Vec::new, |sounds| ..)`, and the `.unwrap_or_default()` line at `:75` goes.

Commit 4 **"The launcher check builds without the old render_frame and skate warnings"**
- `retained.rs`: move `#[must_use]` from `:156` back above `pub const fn host_viewmodel_render_fx_flags` (`:166`). Every caller uses the result (`render_gpu/src/drawsurf/depth_range.rs:22`, `render_backend/src/material_exec.rs:175`, `:359`), so no new warning appears. The `///` lines above the static stay.
- `buffers.rs:9`: `fetch_update` becomes `try_update`, with the same arguments and result.
- Raise the types, not the fields, so no access changes:
  - `footplant/settings.rs:4`: `pub(super) struct Settings` becomes `pub(in crate::physics) struct Settings`.
  - `respawn.rs:16`: `pub(super) struct Runtime` becomes `pub(crate) struct Runtime`.
- These files come from upstream IW4L and from the vendored SK8-ENGINE (cb79689, `skate/Cargo.toml:1-2`) and are identical to `upstream/main`. Each changes by one line. The log lists them as patches, so an upstream merge or a skate re-vendor keeps or drops them knowingly.

**Tests**
- No new tests: nothing changes in behaviour.
- `clippy -p minecraft_survival --locked --all-targets`: no warnings.
- `clippy -p render_anim -p hud --locked --all-targets`: none in `minecraft_*` files. The upstream MW2 files keep their roughly 77 and 61; log the counts.
- `test -p minecraft_survival -p render_anim -p hud --locked`: the same pass counts as before.
- `check -p skate-core -p skate-host --locked --all-targets`: green.
- `check -p launcher --locked`: no warning at all. Log the output's tail.

**Owner's steps.** Same release build and setup as above; nothing should change. Use this group's kit.
1. Put on the iron helmet and chestplate and open the inventory: the character in the inventory wears them and turns towards the mouse. In third person (F5) the MW2 body wears them too. **e3-9-5**
2. Place the chest and open it. With the left button held, drag the stack of 64 cobblestone over four empty slots: while you drag, each slot shows 16, and the counts stay when you let go. Open the crafting table: it works as before. **e3-9-6**
3. Place the composter and use the villager egg beside it. Once it is a farmer, right-click it: the trading screen opens and closes as before. You hear steps, the chest opening and the villager's voice.
4. Write two pages in a book and quill, press Done and open it again: the pages are kept. Optional, with two windows (`scripts\mp-local.ps1`): the friend does the same with the second book and quill, and their pages are kept too (this is the host path the commit edits). **e3-9-7**
5. Press **J**: you skate as before. Bail off a ledge: you get back on the board. **e3-9-8**

```
IW4L_MINECRAFT_KIT=minecraft:iron_helmet,minecraft:iron_chestplate,minecraft:chest,minecraft:crafting_table,minecraft:cobblestone*64,minecraft:writable_book*2,minecraft:villager_spawn_egg,minecraft:composter,minecraft:cooked_beef*16
```

### Group `docs-margit` (S), 1 commit

**Done by the orchestrator on `minecraft-survival` on 2026-11-02** (decisions: release 6 ships Margit, and README still called him not playable). No E3 segment runs this group.

**Gaps:** code-11.

**Cause**
- F's merge 02ea99f2 did not copy its "State doc changes" (`docs/round10/track-f-log.md:73-80`) into the docs.
- `docs/MINECRAFT-SURVIVAL.md` never mentions Margit or Elden.
  - There is no "Bosses" section after `### Survival and combat` (`:51`, before `### Mining and placing` at `:83`).
  - There is no Margit bullet in the `### Mobs` limitations (`:457`).
  - `## Crate layout` (`:269`) has no `crates/elden_boss`.
- `docs/MULTIPLAYER.md` section 9.2 (`:573`) has no `MobShow.elden` or `EldenShow` bullet after the "Flags that renderers read" bullet (`:583`).
- `CLAUDE.md:33` roadmap step 3 still reads only "Margit first, then more...".
- `README.md:22` still says Margit "is being built and is not playable in a release yet". Release 6 (80aabd8e) shipped f3, so that line is wrong in a shipped release now. The group still runs last, so the docs describe Margit as E3 leaves him.

**Vanilla 26.3:** n/a, docs only. Track F checked the bed fact with javap: `Monster.isPreventingPlayerRest` is true, and `ServerPlayer.startSleepInBed` checks an 8 x 5 x 8 box (track-f-log.md commit 29).

**Plan**

Commit **"Docs describe Margit, the first Elden Ring boss"**

Check every fact against the code first:
- the id `iw4l:margit` (`crates/elden_boss/src/margit.rs:10`);
- wake and leash, both 24 (`:676-677`);
- hit key 96 (`third_party/minecraftoss/entities/src/world/families/bosses/elden_bosses.rs:19`);
- `MobShow.elden` (`crates/minecraft_terrain/src/mob_tracking.rs:90`) and `EldenShow` (`elden_show.rs:15`).

Then:
- **`docs/MINECRAFT-SURVIVAL.md`:**
  - A new `### Bosses (round 10, track F)` after `### Survival and combat`. It holds F's paragraph from track-f-log.md:79 word for word, plus two facts F's later commits added: "A player cannot sleep in a bed while he is within 8 blocks across and 5 up or down of it, asleep or awake, as beside any monster. Lava and fire hurt him."
  - In `## Limitations`, `### Mobs`, F's bullet (`:80`): "Margit: he walks through water and lava with his dry ground walk; his fog ring blocks nothing." Add: "Loading the world resets his fight by design: full health, phase 1, and he walks back to his anchor." Write it as things stand when this group runs: if E3's Margit water and lava group has landed, leave its clause out.
  - In `## Crate layout`, after the `crates/minecraft_terrain` bullet: "**`crates/elden_boss`** (edition 2024, only glam): the Elden Ring bosses as data (`margit.rs`) and the pure fight brain, shared by the vendored entities crate and `minecraft_terrain`."
- **`docs/MULTIPLAYER.md`** 9.2: F's bullet (track-f-log.md:77) as a sub-bullet after `:583`, at the same indent.
- **`CLAUDE.md:33`:** replace roadmap step 3 with F's text (track-f-log.md:78).
- **`README.md:22`:** "**Elden Ring:** the first boss, Margit, the Fell Omen, is playable as a proof of concept: `/summon iw4l:margit` or a lantern on two gold blocks, fought by the host and friends with MW2 guns and Minecraft weapons. He is our own stand-in model with vanilla sounds; the real Elden Ring assets come later. Details in [docs/MINECRAFT-SURVIVAL.md](docs/MINECRAFT-SURVIVAL.md), the plan in [docs/ELDEN-RING-BOSSES.md](docs/ELDEN-RING-BOSSES.md)."

Change no other lines. The other tracks' round-end wording stays for the orchestrator.

**Tests:** none, docs only. Every fact above was checked against the code, and every relative link resolves.

**Owner's steps:** after the push, open `README.md` and `docs/MINECRAFT-SURVIVAL.md` on GitHub (origin).
- README's "What works today" says Margit is playable.
- MINECRAFT-SURVIVAL.md has "Bosses (round 10, track F)" and the Margit limitation.

No kit line.

### Left out
- The roughly 77 `render_anim` and 61 `hud` clippy warnings in upstream MW2 files.
- The roughly 79-84 older clippy warnings in the vendored minecraftoss crates.
- `[lints]` tables in the vendored crates.
- A hard `-D warnings` gate in the runner.
- Other tracks' round-end wording.

The reasons are in the "left" list.

## E3-2. Horses, mounts, riding and leads

The owner's known limits this section closes:

> **Horses:** the mount is not drawn in its inventory screen, and only one player can open a mount's inventory at a time. A rider hears no block fall sound, and a mob riding a horse takes no share of its fall. Mount hearts show for horses, donkeys, mules, llamas and striders, the only mounts so far. (`docs/round10/release-2-tests.md:565`)
>
> Arrows, tridents, thrown items and explosions don't break lead knots, and MW2 bullets pass through knots. [...] Knots ignore the `entity_drops` and `mob_griefing` rules. (`docs/round10/release-3-tests.md:658`, `:671`)

Three more from E1's log: a dying horse lands with the generic fall sound (`docs/round10/track-e1-log.md:443`), a wind charge's impulse does not shorten a rider's share (`:477`), and no rider shares a strider's fall (vanilla passes every living vehicle's fall on). "A rider hears no block fall sound" stays: it is vanilla (E1 decision, `track-e1-log.md:476`).

**Paths and line numbers** are as of 9c6efebd. E3 starts after E2's e2b merges, and e2b moves lines in `server.rs`, `world.rs`, `entity_view.rs`, `minecraft_world.rs` and `minecraft_remote.rs`. Re-check every anchor before editing.

**Other tracks**
- **M3** owns the cow store's horse-family lines, `world/equines.rs`, `horse_render.rs` and the horse arms of `world/riding.rs` for m3d (undead horses) and m3e (camels) (decisions "D0's shape", `track-m3.md:33`, `:119`, `:1876-1881`). Groups `rider-falls` and `mount-odds` edit `riding.rs`, `world.rs`, `equines.rs` tests and the strider store. They run and merge before m3d starts. The orchestrator settles this before e3a starts (E3-0 "M3's m3d and e3a/e3b"): it holds m3d until e3a and e3b are merged, or, if m3d has already started, moves both groups to e3z after m3e's merge. `mount-figure` does not edit `horse_render.rs` or `llama_render.rs`: it moves the vertices those renderers already build.
- **M2** owns the nautilus lines in `mount_inventory.rs` (`:22`, `:31`, `:198`) and `server.rs` `Command::MountInventory` (`track-m2.md:96`, `:2731`). No E3 group edits them. `mount-viewers` keeps `Entities::set_mount_inventory` as the only write-back.
- **W's w3** posts game events at the knot sites (tie, punch, shear; `runs/w3.json` group `events-items`). `knot-hits` does not run alongside it. Whichever merges second adds W's one line at the other's new break sites (W0 rule 3). If w3 put its line inside `break_knot`, the new sites get it for free.
- **Owned elsewhere:** M3 (m3e) adds the camel's renderer call to `EntityView::mount_puppet` (camels are cow-store `HorseKind`s, which M3 may edit). Registered mounts (M2's nautilus, M3's happy ghast) come through `mount-figure`'s generic registry arm and need no line in `entity_view.rs`, which D0-3's checklist keeps free of family lines.

**Protocol:** no group changes a message. Friends already get the mount id in `MenuExtras::Mount.mount` (`crates/minecraft_survival/src/menus.rs:1657-1663`). Knots and lead drops reach them through the existing knot and item sync, and mob hurts through `MobShow`.

**Run order:** `rider-falls`, `mount-odds`, `mount-viewers`, `mount-figure`, `knot-hits`.

---

### Group `rider-falls` (M): horses-4, horses-7

**Gaps**
- **horses-4:** a mob riding a horse takes no share of its fall, and no rider shares a strider's fall.
- **horses-7:** a rider's share is not clipped at a wind charge's burst height.

**Cause**
- **Mob riders get nothing.** `third_party/minecraftoss/entities/src/world/riding.rs:683-702` `take_rider_falls` drains each horse's `pending_fall` and maps `Rider::Mob(_) => None` (`:698`). `pending_fall` is set by `world/equines.rs:117-147` `horse_fall`, only when the horse itself is hurt. A mob rider's own fall is zeroed each tick in `riding.rs:571-605` `end_ride_tick`. That part is right (vanilla's `rideTick` resets it), but it leaves nothing that hurts a mob rider.
- **No cross-store fall hurt.** Nothing hurts a mob by id in every store with a fall's damage type. The only cross-store hurt by `EntityKey` is `world.rs:3037-3090` `hurt_by_typed_blast`, which uses the explosion source and zombie armour. However, `world/pushing.rs:194` `exposed_mut(key)` already reaches every store and every D0 family as `&mut dyn Exposed` (`world/hazards.rs:15-25`, impls `:173-245`). Its `hurt_hazard` is how cramming (`pushing.rs:404`) and suffocation (`hazards.rs:79`) hurt any mob. `Hazard` (`entities/src/inside_blocks.rs:30-44`) has no fall.
- **Striders.** `world/striders.rs:442-449` lands through `living::fall_damage` and records nothing for its riders. `take_rider_falls` walks only `self.cows`.
- **horses-7.** `crates/render_anim/src/minecraft_players.rs:864-865` calls `survival.rider_fall(distance, multiplier)` with the horse's distance unchanged. On foot, `:438` clips the fall with `survival.impulse_fall(peak - feet[1], feet[1])` (`crates/minecraft_survival/src/survival.rs:571-580`). Every push already starts an impulse for a seated rider (`minecraft_players.rs:956`, `takes_pushes` `:1016-1020`); only the call is missing (`track-e1-log.md:477`).

**Vanilla 26.3 (javap)**
- **`Entity.causeFallDamage(d, mult, src)`:** false for `#fall_damage_immune`; otherwise `propagateFallToPassengers(d, mult, src)` (offsets 0-21). `Entity.propagateFallToPassengers` calls `causeFallDamage(d, mult, src)` on every passenger (offsets 0-53).
- **`LivingEntity.causeFallDamage`** runs in this order:
  1. The impulse clip (offsets 0-55): `d = min(d, currentImpulseImpactPos.y - getY())` when `isIgnoringFallDamageFromCurrentImpulse`, then `resetCurrentImpulseContext` when `d <= 0`, else `tryResetCurrentImpulseContext`.
  2. `Entity.causeFallDamage` (offset 67). So a living vehicle passes its fall on before its own damage, whatever that damage is.
  3. `calculateFallDamage` = `floor((d + 1e-6 - SAFE_FALL_DISTANCE) * mult * FALL_DAMAGE_MULTIPLIER)`.
  4. Above 0 (offsets 81-116): `resetCurrentImpulseContext`, `getFallDamageSound(dmg)` (hostile or generic; small up to 4, big above), `playBlockFallSound`, `hurt(src, dmg)`. The fall type is in `#bypasses_armor`.
- **`AbstractHorse.causeFallDamage`** propagates only when the horse takes damage (E1's `horse_fall`, unchanged). The strider does not override it: a strider dropping 4 blocks takes 1 and hands its rider `d = 4` (1 damage).
- **`LivingEntity.setIgnoreFallDamageFromCurrentImpulse(true, pos)`** sets a 40-tick grace and the impact position (offsets 0-23). `ServerPlayer.onExplosionHit` sets it for wind charges. The rider's clip uses the rider's own impact height against the vehicle's `d`.
- **No block sound for riders:** `playBlockFallSound` looks 0.2 below the rider's own feet, which is the air above the mount.

**Plan** (three commits)
1. **"Minecraft mobs riding a horse share its fall"**
   - **The fall hazard:** `inside_blocks.rs:30-44` gets `Hazard::Fall` beside `InWall` and `Cramming`. `bypasses_armor` (`:58`) and `damage_type` (`:63`, `"minecraft:fall"`) include it; `panics` and `is_fire` do not. `crates/minecraft_survival/src/death.rs:169-170` and `:214-215` get its arms beside `Cramming`, with the words of `Self::Fall`. Players never get it: they fall through `DamageCause::Fall`.
   - **Visibility:** `pushing.rs:194` `exposed_mut` becomes `pub(super)`.
   - **The share** (`riding.rs`, beside `take_rider_falls`): `pub(super) fn share_vehicle_falls(&mut self, world: &impl World, game_time: i64)`.
     - It takes every horse's `pending_fall` (the body of `:684-689`).
     - Player riders: it queues `(player, distance, multiplier)` in a new `EntityWorld.rider_falls: Vec<(u64, f64, f32)>`. Declare it beside `player_dismounts` (`world.rs:2658`), in `Default` beside `:2773`, and in the copy beside `:5808`.
     - Mob riders: `damage = floor((distance + 1e-6 - 3) * multiplier)` (the default safe distance 3 and multiplier 1). Above 0, push the fall sound onto `exposed.voices()` at the rider: `entity.hostile.small_fall`/`big_fall` for monsters (the stores' own landings pass `hostile`; families through `families::category(key, kind) == Some("monster")`), otherwise `entity.generic.*`. Then `exposed.hurt_hazard(Hazard::Fall, damage, world, game_time)`.
     - A dead rider takes nothing, and no block sound plays.
   - **Drain:** `take_rider_falls` becomes `std::mem::take(&mut self.rider_falls)`, so `server.rs:5148` is unchanged.
   - **Tick:** one line in `world.rs` after the last `end_ride_tick` of the tick (`:8185-8187`): `self.share_vehicle_falls(&*world, game_time);`.
2. **"Minecraft strider riders share its fall"**
   - `striders.rs:442-449`: a landing out of lava with `fallen > 0` records `self.pending_fall = Some((fallen * reduction, multiplier))`, using `living::landed_block` (`living.rs:125-128`), whatever the strider's own damage. `StriderEntity.pending_fall: Option<(f64, f32)>` (`striders.rs:12`) is `None` on spawn and load.
   - `share_vehicle_falls` takes striders' falls too: their player riders through `rider_falls` (the player's `rider_fall` gives 0 up to 3 blocks), and mob riders (strider jockeys) as in commit 1.
3. **"Minecraft riders thrown up by a wind charge count only the fall below the burst"**
   - `survival.rs:540` becomes `rider_fall(distance, multiplier, feet_y)`, which first clips `distance` through `self.impulse_fall(distance, feet_y)`.
   - `minecraft_players.rs:865` passes `feet[1]` (set at `:787`).
   - The existing rider tests (`survival.rs:3311-3340`) get the third argument and no impulse, so their numbers stay.

**Tests**
- **Shares (`world::riding::tests`):**
  - `a_husk_riding_a_horse_fifteen_blocks_down_takes_twelve_with_the_big_fall_sound`: the horse takes 4, the husk goes from 20 to 8 and voices `entity.hostile.big_fall`, and armour does not soften it.
  - `a_mob_rider_takes_nothing_when_its_horse_takes_nothing` (7 blocks).
  - `a_player_rider_s_share_is_queued_once` (two drains in a row give one entry, then none).
  - `a_registered_mob_riding_a_horse_takes_the_fall_through_its_family` (D0's probe as rider, set up as in `families/tests.rs:1040-1050`).
  - `a_strider_dropping_six_blocks_hands_its_rider_six` and `a_strider_landing_in_lava_hands_nothing_on`.
- **Hazard:** `inside_blocks::tests`: `Hazard::Fall` is `minecraft:fall`, bypasses armour and makes nothing panic.
- **Wind charge:** `survival::tests::a_rider_thrown_up_by_a_wind_charge_counts_only_the_fall_below_the_burst`. After `start_impulse(70.0)`, `rider_fall(20.0, 1.0, 60.0)` gives 7 (not 17), and the impulse is gone afterwards.

**Owner's steps** (survival, a new world, by day on flat grass, the kit below, full health before each drop)
1. Build a 3-high pillar and stand on it: `/fill ~2 ~ ~ ~2 ~2 ~ cobblestone`, then `/tp @s ~2 ~3 ~`.
2. `/summon minecraft:horse ~4 ~12 ~ {Passengers:[{id:"minecraft:husk"}]}`: the horse falls 15 blocks with the husk on it.
   - On landing you hear the horse's thud and a heavy fall crunch from the husk, and both flash red.
   - The husk rides on (it lost 12 of its 20).
   - **Screenshot e3h01**, just after the landing.
3. `/summon minecraft:horse ~4 ~22 ~ {Passengers:[{id:"minecraft:husk"}]}` (25 blocks): the horse flashes red and lives, and the husk dies on landing. **Screenshot e3h02.** Before this build both husks rode on unhurt. Kill the survivors with the sword.
4. Strider: at a fresh spot, run `/fill ~3 ~ ~-3 ~9 ~5 ~3 netherrack` (6 high), then `/tp @s ~6 ~6 ~`.
   - Use a strider egg on the top, saddle the strider and get on.
   - Hold the warped fungus on a stick and ride off the edge.
   - You lose 1.5 hearts and hear your fall; the strider loses 1.5 of its 10 mount hearts.
   - **Screenshot e3h03**, right after landing.
5. Repeat from 4 high (`/fill ~3 ~ ~-3 ~9 ~3 ~3 netherrack`, `/tp @s ~6 ~4 ~`): you lose half a heart. From 3 high you lose nothing.
6. Optional, wind charge:
   - Run `/fill ~3 ~ ~-3 ~9 ~11 ~3 stone` (12 high), `/tp @s ~6 ~12 ~`, then `/summon minecraft:horse ~ ~ ~2 {Tame:1b}`. Saddle the horse and ride it off the edge: you lose 4.5 hearts.
   - Get back up (`/gamemode creative`, fly up, `/gamemode survival`). Summon and saddle a new tame horse and ride it.
   - Look straight down, throw a wind charge at its feet, and ride off the edge within two seconds. You again lose 4.5 hearts, not more.
7. Friend (two windows): F does step 4 on F's own strider. F loses 1.5 hearts, and the host hears F land. **Screenshot e3h04**, F's window.

```
IW4L_MINECRAFT_KIT=minecraft:saddle,minecraft:saddle,minecraft:saddle,minecraft:strider_spawn_egg*2,minecraft:warped_fungus_on_a_stick,minecraft:wind_charge*16,minecraft:cobblestone*64,minecraft:golden_apple*8,minecraft:cooked_beef*32,minecraft:iron_sword
```

**State doc changes:** release-2's "a mob riding a horse takes no share of its fall" goes, and E1's `:477` note is resolved.

---

### Group `mount-odds` (S): horses-5, horses-6

**Gaps**
- **horses-5:** a player riding a registered (D0) mount keeps the food bar instead of the mount's hearts.
- **horses-6:** a dying horse lands with the generic fall sound.

**Cause**
- **horses-5.** `riding.rs:719-726` `vehicle_health` answers for the cow store's horse family and for striders. `vehicle_horse` (`:213-217`) matches any `HorseKind`, so M3's camels and undead horses need nothing.
  - A D0 family a player rides (`carries_riders` `:224`, `seats > 0`) gets `None`.
  - M2's nautiluses ride through D0's generic arms and never edit `riding.rs` (`track-m2.md:2791`), and M3's happy ghast doesn't either. No M brief mentions `vehicle_health`.
  - E1 asked for every new living vehicle to be added there (`track-e1-log.md:517`).
- **horses-6.** `world.rs:7834-7839`: a dead cow-store mob, horses and llamas included, runs `dying_travel(..., Some(false))`. Its landing goes to `living.rs:44-46` → `fall_damage` (`:104-123`), which plays `entity.generic.small_fall`/`big_fall` and the block sound. Living horses land through `equines.rs:149-161` `cow_fall` → `horse_fall` (`:117-147`). E1 left the dying path on purpose (`track-e1-log.md:443`).

**Vanilla 26.3 (javap)**
- **Mount hearts:** `Hud.getPlayerVehicleWithHealth` returns the camera player's vehicle whenever `Entity.showVehicleHealth` holds. That is `this instanceof LivingEntity`, and no vehicle class overrides it. The hearts come from `getMaxHealth` and `getHealth`.
- **Horse landing:** `AbstractHorse.causeFallDamage` (offsets 0-67): when `d > 1` it plays `HORSE_LAND` (a foal: `HORSE_LAND_BABY`) at volume 0.4 and pitch 1. Then `calculateFallDamage`, and above 0: hurt, propagate, block sound. It never plays `getFallDamageSound`.
- **Dying horses:** a dying horse still moves through `LivingEntity.aiStep` (immobile travel) → `checkFallDamage` → `Block.fallOn` → its own `causeFallDamage`. `LivingEntity.die` ejects nobody (only `tickDeath` removes the entity, after 20 ticks), so riders share a dying horse's fall as they do a living one's.

**Plan** (two commits)
1. **"Minecraft riders see the hearts of every living mount"**
   - `riding.rs:719-726`, before the strider line: `if let Some((_, core)) = self.families.core(vehicle) { return Some((core.health, core.max_health)); }` (`families.rs:1137`; `MobCore.health`/`max_health` at `:294-295`).
   - That one generic arm covers every D0 family a player rides.
2. **"Minecraft dying horses land with their own thud"**
   - `world.rs:7838`: pass `entity.horse.is_none().then_some(false)` as `hostile`, so `dying_travel` plays nothing for a horse.
   - Send the landing it returns into `equines::cow_fall(entity.horse.as_mut(), &entity.cow.body, &*world, fallen, baby, &mut entity.voices)`, and drop the damage (the mob is dead).
   - Cows keep the generic sounds. Only that line of the dying branch changes.

**Tests**
- `world::riding::tests::vehicle_health_names_a_registered_mount_s_health`: D0's probe with `seats = 1` and a player riding it gives `Some((core.health, core.max_health))`. The horse and strider asserts at `:1332-1360` are unchanged.
- `world::equines::tests::a_dying_horse_falling_ten_blocks_plays_the_horse_land_and_no_generic_fall`: the land sound and the stone's block sound.
- `a_dying_llama_lands_without_a_land_sound` and `a_dying_cow_keeps_the_generic_fall`.

**Owner's steps** (survival, by day, flat grass, the kit below)
1. Run `/fill ~3 ~ ~-3 ~9 ~9 ~3 stone` (10 high), `/tp @s ~6 ~10 ~`, then `/summon minecraft:horse ~2 ~ ~ {Health:1.0f}`. A horse with half a heart stands between you and the edge.
   - Hit it once with the Knockback sword. It dies and flies off, and lands 10 blocks down with the horse's heavy hoof thud and the stone's fall sound, not the crunchy generic big-fall sound.
   - If it stays on top: stand at the foot instead, run `/summon minecraft:horse ~ ~30 ~3 {Health:1.0f}`, and throw the trident at it as it falls (players have no bow in the fork). The hit kills it in the air, and it lands dead with the same thud.
   - This is a sound test, so no screenshot.
2. Mount hearts: tame (golden carrots) and saddle a horse, then saddle a strider (egg). Ride each: the mount hearts show as before (`release-2-tests.md` H-7 and H-17). A registered mount (M2's nautilus, M3's happy ghast) will show its hearts with no further change, but there is nothing to see yet.

```
IW4L_MINECRAFT_KIT=minecraft:iron_sword+knockback=2,minecraft:trident,minecraft:saddle,minecraft:saddle,minecraft:strider_spawn_egg,minecraft:warped_fungus_on_a_stick,minecraft:golden_carrot*16,minecraft:cooked_beef*16
```

**State doc changes:** release-2's "Mount hearts show for horses, donkeys, mules, llamas and striders, the only mounts so far" becomes "for every living mount".

---

### Group `mount-viewers` (M): horses-2

**Gap:** only one player can open a mount's inventory at a time.

**Cause**
- **The lock:** `crates/minecraft_survival/src/menus.rs:2043-2050` skips opening while `entities.mount_viewer(vehicle)` is another player (the lock came with 2ac3b93b). `entities.rs:104` `mount_viewers: HashMap<u64, u64>` maps vehicle → viewer; `:1157-1166` has `mount_viewer` and `set_mount_viewer`, and `:1297` clears the entry on leave.
- **Private copies:** each viewer edits its own copy.
  - It is taken from `MountInventory::of(&entities.view.world, ..)` at open (`menus.rs:2051-2058`).
  - Clicks change `Open::Mount(mount).cells` and push the whole copy to `menus.mount_updates` (`menus.rs:1020-1031`).
  - The copy is written back whole: `:2071-2073` → `entities.rs:1148-1150` `set_mount_inventory` → `server.rs:4722-4723` `Command::MountInventory`.
  - Two viewers would overwrite each other and could double items. `track-e.md:766`'s "Keep the lock" only deferred the fix.
- **Chests already do it the vanilla way.** Their cells live once per level in `Containers` (`crates/minecraft_terrain/src/containers.rs:603-613`). Every viewer's click goes through `menus::apply(player, containers, ..)`, one player after another on the host's thread: the host at `render_anim/src/minecraft_inventory.rs:359` and friends through `take_inputs` in `minecraft_players.rs:2167-2175`.

**Vanilla 26.3 (javap)**
- `AbstractMountInventoryMenu` (fields `mountContainer` and `mount`) builds its slots on the horse's own `SimpleContainer` (`AbstractHorse.inventory`) and its equipment slot containers, so every viewer reads and writes one container.
- `ServerPlayer.openHorseInventory(AbstractHorse, Container)` opens a menu for each player, with no lock.
- `AbstractMountInventoryMenu.stillValid` checks that the container is unchanged, the mount is alive and the player is in reach.

**Plan** (two commits)
1. **"Minecraft mount screens share one set of slots between viewers"**
   - **Shared store** (`containers.rs:603-613`): `mounts: HashMap<u64, (MountInventory, u32)>` (the shared copy and its idle ticks) and `changed_mounts: BTreeSet<u64>`, beside `chests`. Methods:
     - `share_mount(fresh: MountInventory) -> &MountInventory`, which keeps an existing copy.
     - `mount(vehicle) -> Option<&MountInventory>`.
     - `set_mount_cells(vehicle, cells)`, which marks the mount changed.
     - `take_changed_mounts() -> Vec<MountInventory>` and `forget_mount(vehicle)`.
     - It is never saved.
   - **Clicks** (`menus.rs:1020-1031`, `apply`): copy the shared cells into `mount.cells` first, then run `mount_action`. When the cells changed, call `containers.set_mount_cells(mount.vehicle, mount.cells.clone())` instead of `menus.mount_updates.push`.
   - **Reads:** `slot` (`:486`) and `contents` (`:1438`) read the shared cells when present. Every viewer's screen, and every friend's `MenuView`, then shows the other viewer's moves on the next frame.
   - **`mount()`** (`menus.rs:2026`) gains `containers: &mut Containers`:
     - A newly opened screen takes the cells of `containers.share_mount(inventory)`: the first viewer's fresh copy from the world, which later viewers adopt.
     - Every changed mount from `take_changed_mounts` goes out once through `entities.set_mount_inventory` (this replaces `:2071-2073`).
     - A shared copy nobody views is forgotten once `MountInventory::of(&entities.view.world, ..)` equals it (the server has it), or after 2 seconds (40 game ticks) at most. A screen reopened at once therefore never starts from a world copy that has not caught up.
     - Callers pass their level's containers: `render_anim/src/minecraft_world.rs:1346` (in scope from `:1085`) and `minecraft_players.rs:2199` (`friend_menus`, `:2158`).
     - `Menus.mount_updates` (`menus.rs:122`, `:141`) is removed.
   - The lock stays in this commit.
2. **"Minecraft mounts open their screen for several players at once"**
   - Drop the `mount_viewer(..) != id` check (`menus.rs:2043-2050`).
   - `mount_viewers` (`entities.rs:104`) becomes player → vehicle, and `mount_viewer` (`:1157`) becomes `mount_viewed(vehicle) -> bool` for the forgetting rule.
   - `set_mount_viewer` (`:1161-1166`) and `leave` (`:1297`) keep their shape.

`mount_inventory.rs` and `server.rs` `Command::MountInventory` (M2's lines) are not edited.

**Tests** (`menus` tests, in the style of the donkey tests around `:3058-3101`)
- `two_viewers_of_one_donkey_see_each_other_s_moves`.
- `two_viewers_clicking_one_stack_in_one_frame_never_double_it`: one cursor gets the stack and the other stays empty, and the server gets the slot emptied once.
- `the_last_viewer_s_change_still_reaches_the_server`: move a stack and close in the same frame.
- `a_screen_reopened_at_once_reads_the_shared_copy_not_the_old_world`.
- `a_friend_s_menu_view_shows_the_host_s_move`: the friend's sent cells.

**Owner's steps** (two windows, host H and friend F, survival, by day; F joins with the join code)
1. H: `/summon minecraft:donkey ~ ~ ~3 {Tame:1b}`, then right-click the donkey with a chest: chest bags appear on its sides.
2. H crouches and right-clicks the donkey, and its screen opens with 15 chest slots. F walks up to within 3 blocks and does the same: F's screen opens too. Before this build nothing happened for F. **Screenshot e3h10**, both windows.
3. H shift-clicks the 64 cobblestone into the donkey: within a moment the stack shows in F's screen. **Screenshot e3h11**, F's window.
4. F moves the cobblestone into F's backpack: H's screen shows the slot empty. H has no cobblestone and F has 64.
5. H puts 16 diamonds in, and H and F click that stack on a count of three.
   - Only one of you holds it on the cursor; the other's cursor stays empty.
   - Put it back and try twice more. Count all diamonds afterwards: still 16.
   - **Screenshot e3h12**, both windows.
6. H puts a saddle in the saddle slot: F's screen shows it and the donkey wears it. Both close with Esc; H reopens at once and everything is where it was. Save and quit, load, and F rejoins: still there.
7. F walks 10 blocks away with the screen open: F's screen closes, and H's stays open.

```
IW4L_MINECRAFT_KIT=minecraft:chest*4,minecraft:saddle,minecraft:saddle,minecraft:cobblestone*64,minecraft:diamond*16,minecraft:golden_carrot*16,minecraft:cooked_beef*16
```

**State doc changes:** release-2's "only one player can open a mount's inventory at a time" goes. Write the new open limit (see "Left out") in its place.

---

### Group `mount-figure` (L): horses-1

**Gap:** the mount is not drawn in its inventory screen.

**Cause**
- **No figure in the screen.** `crates/hud/src/minecraft_screens.rs:527-532` with `mount` (`:537-550`) draws only the chest strip and the saddle and armour slot frames, over the full sheet (`draw_background`'s default blit at `:511`). No model is drawn.
- **No window for one.** `crates/hud/src/minecraft_inventory.rs:643-649` opens a hole for a 3D figure, and `:717-729` sets `ui.character_box` and `ui.puppet_turn`. Both happen only for the player inventory and the creative inventory tab.
- **The puppet is the player's alone.** `crates/render_anim/src/minecraft_world.rs:147-201` `place_inventory_puppet` stands only the local player's MW2 body there. It sits on a black card drawn in the view-model depth band (`crates/render_gpu/src/drawsurf/minecraft_world.rs:806-813`), and `view.puppet_gear` (`minecraft_world.rs:2293-2318`) carries only that body's gear.
- **Mob meshes are world-only.** They are built only in world space from `ClientMobs` poses: `horse_render.rs:352` `append_horses` and `llama_render.rs:296` `append_llamas`, called from `crates/minecraft_survival/src/entity_view.rs:1051-1067` inside `EntityView::meshes` (`:937`). The host calls `meshes` at `minecraft_world.rs:1517-1545`, a friend at `minecraft_remote.rs:2015-2038`.
- **Out-of-date note:** `track-e.md:765` says "the player inventory has no figure either". The puppet card and its depth band exist now.
- **Precedent:** spawner displays already draw a mob into fresh meshes and then move the new vertices (`spawner_display.rs:785-801`). This group does the same, so `horse_render.rs` (which M3's m3d edits) and `llama_render.rs` stay untouched.

**Vanilla 26.3 (javap)**
- **The call:** `AbstractMountInventoryScreen.extractBackground` calls `InventoryScreen.extractEntityInInventoryFollowsMouse(g, leftPos+26, topPos+18, leftPos+78, topPos+70, 17, 0.25f, mouseX, mouseY, mount)` (bipush 26/18/78/70/17, ldc 0.25f, invokestatic at offset 200).
- **Following the mouse** (`extractEntityInInventoryFollowsMouse`, offsets 0-242):
  - From the box centre: `a = atan((cx - mouseX)/40)` and `b = atan((cy - mouseY)/40)`.
  - Rotation `rotZ(pi) * rotX(b*20 deg)`.
  - Render state: `bodyRot = 180 + a*20`; `yRot = a*20` (the head, relative to the body); `xRot = -b*20` (0 while fall flying). The bounding box is divided by `scale`, and then `scale = 1`.
  - Translation `(0, bbHeight/2 + 0.25, 0)`, then `GuiGraphicsExtractor.entity(state, 17, translation, rotation, rotX(b*20 deg), box)`.
- **Drawing** (`GuiEntityRenderer.renderToTexture`): it sets the UI entity lighting (`Lighting.setupFor`, offset 12), then translates, then rotates. The feet sit `h/2 + 0.25` blocks below the box centre, the model turns about its feet, and the world's light plays no part.

**Plan** (three commits)
1. **"Minecraft mount screens open a window for the mount"**
   - **HUD box** (`crates/hud/src/minecraft_screens.rs:7`): `pub(crate) const MOUNT_BOX: [i32; 4] = [26, 18, 78, 70];` beside `PUPPET_BOX`. In `draw_background` (`:493-512`), a `McScreen::Mount { .. }` arm beside the `Inventory` arm (`:498-505`) blits the sheet around `MOUNT_BOX` in the same four parts, without the grenade frames. The `Mount` arm of the second match (`:527-532`) stays.
   - **HUD window** (`minecraft_inventory.rs:643-646`): `McScreen::Mount { .. } => Some(crate::minecraft_screens::MOUNT_BOX)` in `puppet_box`. The height at `:719` uses the box's own height for it, as for `PUPPET_BOX`. The gradient hole (`:649`), `ui.character_box` and `ui.puppet_turn` (`puppet_turn_in` `:734`, already vanilla's formula) then come for the mount screen too.
   - **Frame resource** (`crates/frame/src/minecraft_ui.rs:570-577`): `InventoryPuppet.mount: Option<MountPuppet>`, with `pub struct MountPuppet { pub centre: [f32; 3], pub down: [f32; 3], pub axes: [[f32; 3]; 3], pub head: [f32; 2] }` in blocks. `down` and each axis are one block long.
   - **Placement** (`place_inventory_puppet`, `minecraft_world.rs:147-201`): set `puppet.mount = None` beside `puppet.active = false` (`:158`). When `ui.screen` (`frame/src/minecraft_ui.rs:195`) is `McScreen::Mount`:
     - Place the card as now (`:192-197`), but not the MW2 body: `active` stays false and there is no depth hack.
     - Fill `puppet.mount` from a pure `mount_frame(..)` beside it. The centre is `at(cx, cy, distance)`. One block is the box's world height × 17/52 (scale 17 in a 52-pixel box). `axes` are `minecraft_gui::puppet_axes(fwd, right, up, yaw, tilt)` (`minecraft_gui.rs:103-110`) at one block each, `down` is `-up` at one block, and `head = [yaw, pitch]`.
     - Points and directions go to blocks with `sim::voxel::to_block`, as the card corners do.
2. **"Minecraft mount screens show the mount, turned towards the mouse"**
   - **Pose override** (`crates/minecraft_terrain/src/client_mobs.rs:1043`): `show_in_screen(id, head_yaw, head_pitch)` and `forget_screen(id)` beside `show_in_spawner`, kept in their own map. `pose` (`:1036-1039`) applies it after the spawner override: `turn` identity, `body_rot` 180 (so `body_rotation` is the model's own frame), `head_yaw` and `head_pitch`.
   - **Puppet mesh** (`entity_view.rs`, beside `meshes` `:937`): `pub fn mount_puppet(&mut self, vehicle, centre, down, axes, head, atlas, light, partial) -> [ChunkMesh; 2]`.
     - It finds the cow in `self.world.cows()` and sets `show_in_screen`.
     - It runs `horse_render::append_horses` and `llama_render::append_llamas` over that one entity into two fresh meshes (models and translucent), then forgets the override.
     - It moves every new vertex: `local = position - pose.feet`, then `position = centre + down * (h/2 + 0.25) + axes * local`, where `h` is the cow body's height. Sky and block light are set to 15 (the UI light).
     - **A generic registry arm:** a vehicle whose kind `crate::families::knows` is drawn through `crate::families::draw` into the same fresh meshes, as spawner cages draw family mobs (`minecraft_terrain/src/spawner_display.rs:532`), then moved the same way with the family body's height. Registered mounts (M2's nautilus, M3's happy ghast) then need no line in `entity_view.rs`, which D0-3's checklist keeps free of family lines. If that draw cannot reach a live family mob, leave the arm out and log registered mount figures as left for later.
     - An unknown vehicle gives empty meshes, and the card shows alone.
   - **Host** (after `entities.view.meshes(..)`, `minecraft_world.rs:1517-1545`): when `puppet.mount` is set and `player.menus.open` is `Open::Mount(m)`, set `view.puppet_mount = [raw(models), raw(translucent)]`; otherwise leave it empty (`update` already holds `puppet` at `:478`).
     - `MinecraftWorldView.puppet_mount: [(Vec<u8>, Vec<u32>); 2]` sits beside `puppet_gear` (`:66`) and is cleared where `puppet_gear` is (`:2587`).
     - It is handed over beside `crates/render/src/extract.rs:1055`.
   - **GPU** (`crates/render_gpu/src/drawsurf/minecraft_world.rs`): `MinecraftWorldFrame.puppet_mount` beside `puppet_gear` (`:97`), uploaded like the entity meshes (`:422-438`). It is drawn right after the puppet gear (`:815-823`), in the view-model depth band, with the `entity` and then the `entity_translucent` pipelines.
3. **"Friends see their mount in its screen"**
   - `crates/render_anim/src/minecraft_remote.rs`, after `entities.meshes(..)` (`:2015-2038`): the same call, with the friend's own `InventoryPuppet` and the vehicle of the friend's mirrored `Open::Mount` (`MenuExtras::Mount.mount`, mirrored at `menus.rs:1985-2000`).
   - No wire change.

**Tests**
- `hud::minecraft_screens::tests::the_mount_screen_leaves_its_mount_window_open`: no blit covers 26..78 × 18..70, and the slots, the saddle and armour frames and the chest strip still draw.
- `hud::minecraft_inventory::tests::the_mount_screen_turns_its_mount_towards_the_mouse`: like `:1075`, with `MOUNT_BOX`; `character_box` is centred on the window.
- `render_anim::minecraft_world::tests::a_mount_frame_gives_a_block_seventeen_fifty_seconds_of_the_box_and_faces_the_camera`.
- `client_mobs::tests::a_screen_pose_turns_only_its_mob_and_only_until_forgotten`.
- `entity_view::tests::the_mount_puppet_stands_in_the_screen_frame_full_bright`, with a saddled horse at (100, 64, 100):
  - Every puppet vertex lies within reach of `centre`, at light 15.
  - The head end is nearest the camera.
  - The world meshes still hold the horse at its place.
- `entity_view::tests::a_llama_puppet_wears_its_carpet_and_chest`.
- `entity_view::tests::a_registered_mount_puppet_is_drawn_through_its_family` (D0's probe family with a seat), unless the arm is logged as left.
- `render_gpu`: the new frame field's size test, beside `:1786`.

**Owner's steps** (survival, by day, flat grass, the kit below)
1. `/summon minecraft:horse ~ ~ ~3 {Tame:1b}`. Crouch and right-click it with an empty hand: in the dark square left of the slots, the horse stands about two thirds of the square tall, lit evenly and facing you. **Screenshot e3h20.**
2. Move the pointer far right of the square, then far left, then above and below. The horse turns its body, turns its head further towards the pointer, and tilts with its head raised or lowered. **Screenshot e3h21**, with the pointer at the right.
3. Put the saddle and then the red leather horse armour in their slots: the figure wears them at once. Take the armour out: it is gone from the figure. **Screenshot e3h22.**
4. Close the screen, ride the horse (right-click with an empty hand) and press E: the same screen shows the horse alone.
5. `/time set midnight` and open the screen again: the figure is as bright as by day.
6. `/summon minecraft:llama ~ ~ ~3 {Tame:1b}`, right-click it with a chest, open its screen and put the red carpet in its body slot: the llama in the square wears the carpet and the chest bags. **Screenshot e3h23.** A donkey (`/summon minecraft:donkey ~ ~ ~3 {Tame:1b}`, plus a chest) shows its chest the same way.
7. Press E on foot: your MW2 character still stands in the player inventory as before.
8. Friend (two windows): F summons a tame horse and opens its screen. F's window shows F's horse turning towards F's pointer. **Screenshot e3h24**, F's window.

```
IW4L_MINECRAFT_KIT=minecraft:saddle,minecraft:saddle,minecraft:leather_horse_armor@red,minecraft:iron_horse_armor,minecraft:red_carpet*2,minecraft:chest*2,minecraft:golden_carrot*16,minecraft:cooked_beef*16
```

**State doc changes:** release-2's "the mount is not drawn in its inventory screen" goes. Note for M3: camels (cow-store horses) add their renderer's call to `mount_puppet`. Registered mounts such as M2's nautilus come through the generic registry arm with no `entity_view.rs` line.

---

### Group `knot-hits` (M): horses-13, horses-11, horses-12

**Gaps**
- **horses-13:** knots ignore `entity_drops`.
- **horses-11:** arrows, tridents and thrown things pass through knots, and knots ignore `mob_griefing` for hits from mobs.
- **horses-12:** explosions leave knots.

**Cause**
- **horses-11: knots are in no projectile target list.**
  - **Arrows:** `third_party/minecraftoss/entities/src/world.rs:3365` `hit_boxes` lists only mobs. Arrows use it through `arrow_targets` (`:3467-3480`), and their hits are resolved at `:6815-6845`.
  - **Thrown things, tridents and rockets** take `thrown::targets` (`crates/minecraft_terrain/src/thrown.rs:580-582`), plus item frames added beside it at each flight site: rockets `server.rs:1750`, tridents `:2410`, thrown `:2599`.
  - **`thrown::targets` is also used for splash and cloud victims and for wind pushes** (`splash.rs:35`, `:121`, `server.rs:2276`, `wind.rs:147`), so knots must not go into it.
  - **The frames' pattern:** frames are hit through `FRAME_TARGET` ids (`item_frame.rs:20`, `:563-576`, `struck_frame` `:633`) and acted on after the loop (`server.rs:2632-2634`, `:2699-2701`; tridents `:2433-2437`).
  - **Knots live in the entity world:** `leashes.rs:375` `knots()` and `:757-770` `break_knot`. Their box comes from `knot_position` (`:61`) and `KNOT_WIDTH`/`KNOT_HEIGHT` (`:11-12`), as `leash_render.rs:108-115` builds it.
  - **Lead drops** are spawned by `server.rs:1851-1855` `drop_leads`, which also runs after every mob tick (`:1445`).
- **horses-12: explosions skip knots.** Explosions break item frames through `server.rs:3889-3897` `break_blasted_frames`: effects with `interacts` set (`explosion.rs:162-168`), within `end_crystal::in_blast` (`end_crystal.rs:107-110`). It runs at `:5176`. Knots have no such step.
- **horses-13: knots always drop their leads.**
  - `break_knot` always drops (`drop_leash(mob, true)` and `drop_saved_leash`, `leashes.rs:750-755`), and `drop_leads` spawns each lead with no check.
  - `entity_drops` reaches only the level (`server.rs:3661`) and the frames. The entity world has `mob_griefing` (`world.rs:2575`, set at `server.rs:3671`) but no `entity_drops`.
  - Every knot removal goes through `break_knot`: the fence gone (`tick_knots` `:725-738`), a punch (`server.rs:1863-1870`), and, after this group, shots and blasts.

**Vanilla 26.3 (javap)**
- **Pickable:** `Projectile.canHitEntity` → `Entity.canBeHitByProjectile`, which is alive and pickable. `BlockAttachedEntity.isPickable` is true.
- **`BlockAttachedEntity.hurtServer`** (offsets 0-71):
  - It returns false when the source's entity is a `Mob` while `mob_griefing` is off.
  - Otherwise it calls `kill`, `markHurt` and `dropItem`, whatever the amount.
  - `LeashFenceKnotEntity.dropItem` only plays `item.lead.untied`. The leashed mobs drop their leads on their next `Leashable.tickLeash`.
- **`Leashable.tickLeash`** (offsets 39-95): a holder that can no longer interact (a removed knot) leads to `dropLeash()`, which spawns `Items.LEAD`, when `ENTITY_DROPS` is on, and to `removeLeash()` when it is off. The give-up drop in `restoreLeashFromSave` (offsets 114-117) has no rule check; it is kept as is.
- **Hits that break a knot** (they hurt it):
  - `AbstractArrow.onHitEntity`: the arrow is spent; a refused hurt bounces it back.
  - `ThrownTrident.onHitEntity`: the trident bounces and falls.
  - `Snowball`, `ThrownEgg` and `ThrownEnderpearl.onHitEntity`: damage 0 still kills the knot.
  - `AbstractWindCharge.onHitEntity`.
- **Hits that leave a knot:**
  - `FireworkRocketEntity.onHitEntity` only explodes (offsets 0-27), and `dealExplosionDamage` reaches only `LivingEntity`: the rocket bursts on the knot and leaves it.
  - A splash potion shatters on a knot and does not hurt it.
- **Explosions:** `ServerExplosion.hurtEntities` → `BlockAttachedEntity.ignoreExplosion` (offsets 0-36). The knot ignores the blast when the direct source is in water or `Explosion.shouldAffectBlocklikeEntities` is false. That is false for `WIND_CHARGE` and `BREEZE_WIND_CHARGE`, and, with `mob_griefing` off, for every blast except `DESTROY` and `DESTROY_WITH_DECAY`. A knot in reach dies even behind walls, because its hurt ignores the amount.

**Plan** (three commits)
1. **"Minecraft lead knots drop their leads only with entity drops on"**
   - `world.rs`: `entity_drops: bool` beside `mob_griefing` (`:2575`; default true beside `:2721`; copied beside `:5808`), and `set_entity_drops` beside `set_mob_griefing` (`:4496`).
   - `leashes.rs:757-770` `break_knot` calls `drop_leash(mob, self.entity_drops)`. `drop_saved_leash` (`:750-755`) gains `item: bool` like `drop_leash`: `break_knot` passes `self.entity_drops`, and shearing (`:783`) passes `true` (unchanged).
   - `server.rs:3671`: `self.mobs.set_entity_drops(rules.bool("entity_drops"));` beside `set_mob_griefing`.
2. **"Minecraft arrows, tridents and thrown things break lead knots"**
   - **Knot targets** (`leashes.rs`):
     - `pub const KNOT_TARGET: u64 = u64::MAX / 8`. It sits below the frames' `FRAME_TARGET` (`u64::MAX / 4`, `item_frame.rs:20`) and far above any mob id.
     - `knot_targets(&self) -> Vec<PotionTarget>` over `knots()` sorted by position, with ids `KNOT_TARGET + index` and the knot box.
     - `struck_knot(&self, id) -> Option<Pos>`, through the same sorted list.
     - `shoot_knot(&mut self, pos, by_mob: bool) -> bool`: false when `by_mob && !self.mob_griefing`, otherwise `break_knot(pos)`.
   - **Arrows:** `arrow_targets` (`world.rs:3467-3480`) chains `knot_targets()`. In the arrow arm (`:6815-6845`), a knot hit is accepted when `shoot_knot(pos, owner is a mob)` holds. Otherwise `resolve_entity_hit(false)` bounces the arrow, as for a creative player. The leads drop through the tick's `drop_leads` (`server.rs:1445`).
   - **Thrown things, tridents and rockets** (`server.rs`): add `self.mobs.knot_targets()` beside `self.frames.targets()` at rockets (`:1750`), tridents (`:2410`) and thrown (`:2599`), never inside `thrown::targets`.
     - **Thrown:** snowballs, eggs, ender pearls and wind charges collect a knot hit the way `frame_hits` does (`:2632-2634`), and the mob line (`:2635`) skips it.
     - After the loop, beside `:2699`, act with `shoot_knot(pos, shooter is a mob or MOB_THROWER)`, then `drop_leads()`.
     - Eggs still roll their chicks and pearls still teleport. Splash and lingering potions shatter at the knot and leave it.
     - **Tridents:** a `TridentEvent::Struck` arm for a knot beside the frame arm (`:2433-2437`); `mob_owner` is the mob test.
     - **Rockets:** a knot in the list makes a rocket burst there; the knot is not touched.
3. **"Minecraft explosions break lead knots"**
   - `server.rs`: `break_blasted_knots` beside `break_blasted_frames` (`:3889-3897`). For each effect with `interacts`, it calls `self.mobs.break_knot(pos)` on every knot whose `knot_position` lies within `end_crystal::in_blast`, then `self.drop_leads()`. It is called beside `break_blasted_frames` at `:5176`.
   - Wind charges push no explosion effect (`explosion.rs:171`), so their bursts leave knots, as in vanilla.
   - A creeper blast with `mob_griefing` off is `Keep` and has `interacts` false, so it leaves knots, as it leaves frames.

**W's w3:** if it merged first, add its knot game-event line at each new break site (the arrow arm, the thrown and trident acts, `break_blasted_knots`), unless the line sits inside `break_knot`.

**Tests**
- **Drops (`leashes::tests`):** `a_knot_broken_with_entity_drops_off_frees_its_mobs_and_drops_no_lead` (the punch and fence-gone paths), and `shearing_still_drops_its_leads_with_entity_drops_off`.
- **Targets:** `leashes::tests::knot_targets_sit_below_the_frames_and_map_back_to_their_knot`.
- **Arrows:** `world::tests::an_arrow_breaks_a_knot_and_is_spent` and `a_skeleton_s_arrow_bounces_off_a_knot_without_mob_griefing`.
- **Thrown things, tridents and rockets (`server::tests`):**
  - `a_snowball_breaks_a_knot_and_its_leads_drop`, `a_trident_breaks_a_knot_and_falls`, `an_ender_pearl_breaks_a_knot_and_still_teleports`.
  - `a_rocket_bursts_on_a_knot_and_leaves_it`, `a_splash_potion_shatters_on_a_knot_and_leaves_it`.
  - `a_snow_golem_s_snowball_leaves_a_knot_without_mob_griefing`.
- **Explosions (`server::tests`):** `tnt_breaks_a_knot_behind_an_obsidian_wall`, `a_wind_charge_burst_beside_a_knot_leaves_it`, `a_kept_blast_leaves_a_knot`.

**Owner's steps** (survival, by day, flat grass, a fresh world, the kit below)

Setup: place 6 oak fence posts in a row, 4 blocks apart. Use a cow egg by each post, put a lead on each cow and right-click its post: every post has a knot. Players have no bow or crossbow in the fork (gap projectiles-1), so arrows on knots are tested through a skeleton (step 7) and rockets on knots by `a_rocket_bursts_on_a_knot_and_leaves_it`.

1. Stand 6 blocks from the first post, side-on, and throw a snowball at its knot (aim at the upper half of the post).
   - The knot vanishes with the untie sound and the snowball's white crumbs, a lead drops by the cow and the cow is free.
   - **Screenshot e3h30.**
2. Four more posts, one projectile each:
   - Egg: the same, and sometimes a chick.
   - Trident: the knot breaks, and the trident drops and flies back to you.
   - Ender pearl: the knot breaks and you land there.
   - Wind charge: the knot breaks and the burst pushes the cow.
3. Tie a cow to the last post. Throw the splash water bottle at its knot: it splashes there and the knot stays. **Screenshot e3h31.**
4. Throw a wind charge at the ground a block away from a knot, not at the knot: the burst leaves the knot.
5. TNT: tie a cow to a post and build an obsidian wall 3 wide and 3 high, 2 blocks from the post. Put TNT against the far side of the wall, light it and step well away. After the blast the post still stands, the knot is gone and a lead lies by the cow. **Screenshot e3h32.**
6. `/gamerule entity_drops false`.
   - Tie a cow and punch the knot off: the cow is free and no lead drops.
   - Tie another cow and break its post with the axe: no lead either.
   - `/gamerule entity_drops true`, then punch a knot off: a lead drops again.
   - **Screenshot e3h33**, the freed cow with no lead on the ground.
7. Skeleton arrows. `/time set night`. Tie a cow to a post, hatch a skeleton 10 blocks in front of the post, and stand 1 block behind the post with the shield raised, so the knot is in its line of fire. It may take a few shots.
   - An arrow that hits the knot breaks it: the knot vanishes, the arrow is gone and a lead drops. **Screenshot e3h34.**
   - `/gamerule mob_griefing false`, tie the cow again and wait for more shots: arrows that hit the knot bounce off it and the knot stays.
   - Kill the skeleton, then `/gamerule mob_griefing true` and `/time set day`.
8. Friend (two windows): F throws a snowball at one of H's knots. Both windows show the knot vanish and the lead drop. **Screenshot e3h35**, H's window.

```
IW4L_MINECRAFT_KIT=minecraft:lead*16,minecraft:cow_spawn_egg*8,minecraft:oak_fence*16,minecraft:snowball*16,minecraft:egg*16,minecraft:ender_pearl*4,minecraft:trident+loyalty=3,minecraft:wind_charge*16,minecraft:splash_potion@water,minecraft:tnt*4,minecraft:flint_and_steel,minecraft:obsidian*32,minecraft:skeleton_spawn_egg*2,minecraft:shield,minecraft:wheat*16,minecraft:iron_sword,minecraft:iron_axe,minecraft:cooked_beef*16
```
18 stacks.

**State doc changes:** release-3's "Arrows, tridents, thrown items and explosions don't break lead knots" and "Knots ignore the `entity_drops` and `mob_griefing` rules" go. "MW2 bullets pass through knots" stays (see "Left out").

---

### Left out

- **MW2 bullets still pass through knots.** Bullets hit what `shot_boxes` lists (`entity_view.rs:846-871`) by a 56-bit hit key with a group byte. Knots are keyed by block position, have no id that fits, and have no hit-key byte of their own.
- **Other vehicles' falls are not passed on:** spider and ravager jockeys, and D0 family mounts. Each store's own landing would need a pending-fall line, and the family landing (`families.rs:1532`) is a shared root site. Horses and striders are every mount a player can ride today.
- **Right-click changes during an open screen.** A saddle, armour or chest put on a mount by right-clicking it while its screen is open is overwritten by the screen's next change, as before. The shared copy lives on the host thread and the horse on the server thread; vanilla shares the horse's own container.
- **Blasts from a source in water still break knots** (and frames). The level's explosion effects don't record where the source was (`BlockAttachedEntity.ignoreExplosion`).
- **A rider hears no block fall sound.** This is vanilla (E1 decision, `track-e1-log.md:476`).

### Files shared

- **Vendored entities** (`world.rs`, `world/riding.rs`, `world/striders.rs`, `world/leashes.rs`, `world/pushing.rs`, `inside_blocks.rs`): M3 (horse arms and `leashes.rs:338`, `:829`), M2, W (w3 knot events), D0.
- **`crates/minecraft_terrain/src/server.rs`:** M2 (`Command::MountInventory`, not touched here), W (w3).
- **Mount screen and puppet files** (`menus.rs`, `entities.rs`, `containers.rs`, `client_mobs.rs`, `entity_view.rs`): M2's nautilus lines beside each `llama`.
- **Rendering and HUD** (`render_anim` `minecraft_world.rs`, `minecraft_remote.rs`, `minecraft_players.rs`; `render_gpu` `minecraft_world.rs`; `hud`; `frame`): M2 and M3 (one-liners).

## E3-3. Margit and the Elden Ring bosses

Track F finished Margit for round 10 (02ea99f2, `docs/round10/track-f-log.md`). Three gaps remain that need no Elden Ring game data, so they belong here rather than in roadmap step 3. Each group below handles one of them, in this order:

1. **`margit-water`** (margit-5): he walks through water and lava with his dry ground walk. This is the open line in `docs/MINECRAFT-SURVIVAL.md` Limitations, from the track F log, "State doc changes".
2. **`margit-egg`** (margit-10): he has no spawn egg (design doc `docs/ELDEN-RING-BOSSES.md` 11.3 and 16).
3. **`margit-heals`** (margit-16): he does not punish heals (design doc 16).

**Where the anchors come from.** File:line anchors are on `minecraft-survival` at 4301058a. E2's e2b (d565a3d2) and D0's d0b (4c42b0ae) merge before E3 runs. They touch `server.rs`, `actions.rs`, survival `entities.rs`, `render_anim/src/minecraft_world.rs`, `walk_path.rs` and `navigation.rs`, so check every anchor again before editing. Vanilla facts were checked with javap on the 26.3 `client.jar` this round.

**Rules for every group:**
- No code comments of any kind.
- Edits change only the lines they need, in the surrounding style.
- New enum variants and match arms go next to related ones.
- New files must pass `rustfmt --check`.
- `minecraft_net::PROTOCOL` is never bumped. None of the three groups changes a message, so each writes "none" under "## Protocol" in the log. `margit-egg` also notes that the fork item table, and with it the registry fingerprint (`session.rs:800-816`), changes, so host and friends need the same build.
- Wording for `docs/MINECRAFT-SURVIVAL.md` and `docs/ELDEN-RING-BOSSES.md` (11.3, 16 and the 5.4 dagger note) goes under "State doc changes" in the track log. Track runs do not edit those docs.

**Shared with other tracks** (decisions.md, "The M briefs give every cross-track hook one owner"):
- **M2 owns the water hooks:** `swim.rs`, `Breath`, `swim_sounds`, `quiet_steps` and `OWN_DYING_TRAVEL`. M2's m2a `water-paths` also refactors the `walk_path.rs` A* over a `NodeSource`.
  - `margit-water` uses none of these. It uses the existing `Body::travel_water` (`entities/src/movement.rs:297`), `Body::travel_lava` (`:395`), `Body::living_jump` (`:251`) and the existing `WalkProfile.can_float`.
  - It edits only Margit's own files. M2's refactor keeps `can_float`'s meaning, since the walk tests are unchanged. If m2a merged first, run the arena tests again after taking `minecraft-survival` in.
- **M1 owns `PlayerControls.sprinting` and `PlayerView.sprinting`** (`armadillos-live`, placed next to `sneaking`). `margit-heals` adds a sibling field, `consuming`, and always puts it right after `jumping`, never next to `sneaking`.
  - Hunks in multi-line literals then stay apart.
  - Two test literals are written on one line each (`crates/minecraft_survival/src/entities.rs:2574` and `:2664`). Whichever of E3 and M1 merges second keeps both fields there.
- **M3 owns `walk_target`.** Margit's `WalkTarget::Monster` (`arena.rs:46`) stays as it is.
- Wardens (W) and breezes (D) already show the fluid travel pattern: `wardens.rs:443-460` and `breezes.rs:293-316`. They are read-only references.

---

### Group `margit-water` (S, two commits)

**Closes:** margit-5.

**Cause**
- `third_party/minecraftoss/entities/src/world/families/bosses/elden_bosses/arena.rs:293-343` `carry_out` always calls `body.travel_air`: at `:319` for move and leap motion, at `:337` while walking and at `:339` while idle.
  - In water he therefore falls at full gravity, stands on the floor and walks along it at his land speed. In lava he does the same.
  - `FluidFrame` is sampled only for path planning, at `:152` (survey) and `:240` (stride).
- `profile()` at `:38-48` sets `can_float = false` (`:44`). The ground path therefore follows a water bottom: `walk_path.rs:510` (`first_non_water_below` when `!can_float`).
- `stride()` passes `can_float: false` to `ground_view` at `:269` (`navigation.rs:669`).
- Lava is avoided by the path's default `-1` malus (`player/src/path_type.rs:46`). That does not help once he is in lava (a summon, an egg, a fade or a leap).
- The brain starts a move only when `sense.on_ground` (`crates/elden_boss/src/brain.rs:430`). `think` fills it from `core.body.on_ground` (`elden_bosses.rs:128`). Once he floats, a player treading water in a deep pool would be safe from him.
- `stands()` at `:79-94` checks only block collision and support.
  - A fade (`spot_near` `:100-135`, used by `carry_out` at `:296`) can therefore land him in water or lava.
  - So can the "room beside the player" of the stuck rule (`room_beside` `:137-140`, `survey` `:186-190`).
- Fire and lava already hurt him (`elden_bosses.rs` tests `:724`, `:995`). D0's `family_base_tick` (`families.rs:1364-1394`) already runs fluids, burning and drowning for him. Unchanged since 55daa528.

**Vanilla 26.3 (javap)**
- **`LivingEntity.travel(Vec3)`:**
  - When `shouldTravelInFluid(level.getFluidState(blockPosition()))` holds (`isInLiquid && isAffectedByFluids && !canStandOnFluid`), it calls `travelInFluid`.
  - Otherwise it calls `travelFallFlying` or `travelInAir`.
  - `travelInFluid` calls `travelInWater` when `isInWater` (acceleration 0.02, drag 0.8, gravity/16 when sinking slowly), else `travelInLava` (drag 0.5 when deeper than the jump threshold, gravity/4). The fork has both as `Body::travel_water` and `Body::travel_lava`.
- **`FloatGoal(Mob)`:**
  - It passes `FluidTags.ENTITY_FLOATABLE` and calls `PathNavigation.setCanFloat(true)` in its constructor.
  - `canUse` is `isInFluidDeeperThan(getFluidJumpThreshold(), ENTITY_FLOATABLE) || isInLava()`.
  - `tick` calls `JumpControl.jump()` when `getRandom().nextFloat() < 0.8f`.
- **`Entity.getFluidJumpThreshold`:** `getEyeHeight() < 0.4 ? 0.0 : 0.4`, so 0.4 for Margit (eye 2.55).
- **The jump:** `LivingEntity.aiStep` turns it into `jumpInLiquid` (+0.04 up) in water or lava deeper than the threshold, or into `jumpFromGround` with the 10-tick `noJumpDelay`. This is the fork's `Body::living_jump`.
- **`LivingEntity.checkPositionAndTeleport`:** it refuses a box unless `Level.noCollision(box)` and `!Level.containsAnyLiquid(box)`. That is the rule for his fade.

**Plan**
1. **"Margit swims and wades through water and lava like vanilla mobs"**
   - **`arena.rs`, the float rule:**
     - Add `pub const FLUID_JUMP: f64 = 0.4;` beside the other constants (`:16-25`).
     - Add `pub fn floating(fluid: FluidFrame) -> bool`. It is `FloatGoal.canUse`: `fluid.in_water() && fluid.water_height > FLUID_JUMP || fluid.in_lava()`, as `wardens/brain.rs:313-316` does.
   - **`profile()` `:44`:** `can_float = true` (FloatGoal's `setCanFloat`).
   - **`stride()` `:269`:** `ground_view(level, body, fluid, true)`. The path then starts at the water surface and keeps vanilla's water malus, so he goes around a small pond and swims across a moat.
   - **`carry_out()` `:293-343`:**
     - After the fade branch, sample `FluidFrame::sample(level, position, width, height)` once.
     - Just before each travel, after the move's velocity is set, run FloatGoal's tick:
       - when `floating(fluid)`, `jumping = core.random.next_float() < 0.8` (no draw otherwise);
       - count `state.no_jump_delay` down as `breezes.rs:293-295` does;
       - call `body.living_jump(level, fluid, jumping, &mut state.no_jump_delay, FLUID_JUMP)`.
     - The three travel calls (`:319`, `:337`, `:339`) go through one private helper. It picks `travel_water(level, input, yaw)`, `travel_lava(level, input, yaw, fluid.lava_height)` or `travel_air(level, input, speed, yaw)` the way `wardens.rs:451-460` does.
     - The input stays `(0, 0, speed)` with the walk's heading. Vanilla's `zza` is the speed, so he swims at vanilla's slow mob speed of about 0.03 a tick.
     - Moves keep setting his velocity from `act.motion` as now, so steps and leaps still carry him in water.
   - **`elden_bosses.rs`:**
     - Add `EldenBoss.no_jump_delay: i32` (struct `:37-58`), set to 0 in `boss()` (`:67-102`).
     - In `think()` (`:117-150`), the `Sense.on_ground` becomes `core.body.on_ground || arena::floating(FluidFrame::sample(level, position, core.body.width, core.body.height))`. Vanilla melee goals do not need the ground, so he may start moves while he swims and a deep pool is no hiding place.
     - Nothing else in the brain changes.
2. **"Margit never fades into water or lava"**
   - **`arena.rs` `stands()` `:79-94`:** also false when `FluidFrame::sample(world, at, width, height)` is in water or lava, as `checkPositionAndTeleport` refuses `containsAnyLiquid`.
     - This covers the fade target (`spot_near`) and the stuck rule's room beside a player (`room_beside`).
     - A player whose surroundings are all water now gets daggers (`rules.pinned`, `fight.rs:104-107`) instead of a fade into the pool.
     - The reset's fade home keeps `spot_near(..).unwrap_or(to)` (`:296`), so an anchor inside a lake still works.

**Tests** (`arena.rs` tests; the test `Scene` gains `water` and `lava` sets that `block()` answers first)
- `margit_put_in_a_pool_floats_up_and_swims_out_at_water_speed`:
  - The pool is 7 by 7 and 3 deep, he is summoned on its floor and a player is 6 blocks past the far edge.
  - Within 60 ticks his feet rise above the floor by more than 1.5 blocks, and they stay there while he is in it.
  - His horizontal step in the water never exceeds 0.06.
  - He leaves the pool on the player's side within 600 ticks and takes no drowning damage.
- `margit_in_a_lava_pit_wades_out_slowly`:
  - The pit is 3 by 3 and 1 deep. His step in lava stays under a third of his land walk.
  - He hops (`velocity.y > 0` on some ticks), burns (`fire_ticks > 0`, health down) and is out within 400 ticks.
- `a_player_treading_water_in_a_deep_pool_is_still_fought`: a player at the surface of a 3-deep pool beside him is hit (`state.hits` has the player) within 600 ticks.
- `profile(&MARGIT).can_float` is true, inside one of the above.
- `a_standing_spot_must_be_dry`: extend `a_standing_spot_needs_a_block_under_him_and_room_for_his_box` (`:700`) or add a test.
  - `stands()` is false with water or lava in his box.
  - `spot_near` picks the nearest dry spot.
  - `room_beside` is `None` when every nearby spot is flooded.
- `margit_does_not_fade_into_a_flooded_ring`:
  - Use the ring of `margit_fades_inside_a_wall_ring_he_cannot_reach` (`:603`) with water on the floor inside, except the player's block.
  - For 600 ticks he never enters `State::Fade` and some move with throws starts (pinned).
  - Once the water is gone he fades in beside the player.
- Existing: `margit_walks_to_a_player_10_blocks_away`, the leash, leap, home and hole tests stay green, since no water means no change.

**Owner's steps** (release build; a fresh world `r10e3margit`; survival, Normal, by day, on flat open ground in the Overworld with at least 30 blocks of flat ground toward Z+)

Put the diamond armour on and keep golden apples on the hotbar. Margit's hits hurt, so eat one when you run low. Between steps, clear him with `/difficulty peaceful` and then `/difficulty normal`. Read your position with `/tp @s ~ ~ ~`.

1. **The pool.**
   - Stand in the middle of a block and run `/fill ~-3 ~-3 ~4 ~3 ~-1 ~10 minecraft:water`. This makes a pool 7 by 7 and 3 deep, 4 to 10 blocks from you toward Z+.
   - Run `/summon iw4l:margit ~ ~-3 ~7`. He appears on the pool floor in its middle and rises at once with the mirror sound.
   - Within about two seconds he floats up. His head and shoulders stay above the water and he bobs. Before this build he stayed on the floor and walked along it under water at full speed.
   - He comes toward you slowly, about half a block a second, with splashes. He steps out at the near edge and walks at you. He may throw a dagger from the water.
   - **Screenshot e3-margit-1** (him at the surface).
2. **No hiding in water.** Jump into the middle of the pool and tread water at the surface (hold Space).
   - He follows you into the pool and keeps fighting from the water with daggers, cane swings or a leap.
   - Climb out. Run `/difficulty peaceful`, then `/difficulty normal`.
3. **Lava.**
   - Run `/tp @s ~15 ~ ~` to fresh ground, then `/effect give @s minecraft:fire_resistance 600`.
   - Run `/fill ~-2 ~-1 ~4 ~2 ~-1 ~8 minecraft:lava` (a lava pool 5 by 5, 1 deep), then `/summon iw4l:margit ~ ~-1 ~6`.
   - He burns and his boss bar drops. He moves slowly in the lava, hopping, and climbs out on your side after a few seconds. Before this build he walked out at full speed.
   - **Screenshot e3-margit-2.** Then `/difficulty peaceful` and `/difficulty normal`.
4. **No fade into water.**
   - Run `/tp @s ~15 ~ ~`, stand in the middle of a block and build release 3's ring (M-9), 6 high and 4 blocks out:
     ```
     /fill ~-4 ~ ~-4 ~4 ~5 ~-4 cobblestone
     /fill ~-4 ~ ~4 ~4 ~5 ~4 cobblestone
     /fill ~-4 ~ ~-3 ~-4 ~5 ~3 cobblestone
     /fill ~4 ~ ~-3 ~4 ~5 ~3 cobblestone
     ```
   - Run `/fill ~-3 ~-1 ~-3 ~3 ~-1 ~3 minecraft:water` and then `/setblock ~ ~-1 ~ minecraft:cobblestone`. The ring's floor is now water, except your block.
   - Run `/summon iw4l:margit ~ ~ ~8`. He rises outside and cannot get in.
   - After about 5 seconds he does **not** fade into the water beside you. Instead a dagger forms every few seconds (amethyst chime) and clinks against the wall.
   - Run `/fill ~-3 ~-1 ~-3 ~3 ~-1 ~3 minecraft:dirt`. Within about 5 seconds the mirror sound plays and he appears beside you on the dry floor.
   - **Screenshot e3-margit-3** (him inside the ring). Then `/difficulty peaceful` and `/difficulty normal`.

Kit (10 backpack slots):

```
IW4L_MINECRAFT_KIT=minecraft:netherite_sword+sharpness=5,minecraft:shield,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*16,minecraft:cooked_beef*32,minecraft:iron_pickaxe,minecraft:water_bucket
```

---

### Group `margit-egg` (M, two commits)

**Closes:** margit-10.

**Cause**
- **The use side already works by name.**
  - `crates/minecraft_terrain/src/spawn_egg.rs:63-67` `spawned_kind` turns `iw4l:margit_spawn_egg` into `iw4l:margit`.
  - `egg_use` (`:73-112`) handles a clicked block and a liquid.
  - `crates/minecraft_survival/src/actions.rs:1304-1336` routes `EggUse::Spawn` to `Entities::hatch_egg` (`entities.rs:1318`) and `EggUse::Spawner` to `Entities::egg_on_spawner` (`:1325`). The egg is spent only when the server answers Ok (`EggsWaiting::answer`, `spawn_egg.rs:39`).
- **The server side does not hatch him.**
  - `ServerSim::hatch_egg` (`server.rs:1985-1999`) calls `level.summon_mob` (`world/src/level/spawning.rs:206`).
  - That calls `Spawns::summon` (`natural_spawner/spawning.rs:914-916`), which needs the vanilla entity catalog's `default_tag`. The catalog has no `iw4l:margit`, so the egg would fail with "Unable to summon iw4l:margit".
  - Only `ServerSim::summon` (`server.rs:1968-1976`) has the Elden branch: the Peaceful refusal and `elden_save::summon`. So the brief's "the spawn path already works" holds for `/summon` only.
- **The spawner path takes any id.** `ServerSim::spawner_egg` (`server.rs:2001-2021`) would write `iw4l:margit` into a trial spawner (`trial_spawner::spawn_egg`, `:2009`) or a monster spawner (`set_spawner_entity`, `:2018`). Neither can ever spawn him.
- **`spawn_egg.rs:114-127` `mob_width`** falls back to 0.6 for him, though he is 1.4 wide.
- **The item is missing.**
  - `fork_items.rs:9-16` `Kind` has no spawn egg, and `ROWS` (`:238`) has 94 rows and no egg.
  - `creative_entries` (`:1006`) and `add_to_creative` (`:1019`) put every row in `minecraft:combat` (`CREATIVE_TAB`, `:999`). Nothing goes in `minecraft:spawn_eggs` (`creative_tabs.rs:387`).
  - Without a row there is no name, no icon and no carrier (`item_nbt.rs:96-101` `carrier`, `:117-136` `carrier_components`).

**Vanilla 26.3 (javap)**
- **`SpawnEggItem.useOn`:**
  - It reads `getType(stack)` and fails (FAIL, no message) unless `EntityType.canSpawn(level)`, which needs the feature flag and `isAllowedInPeaceful() || difficulty != PEACEFUL`.
  - **On a block entity that is a `Spawner`:** it fails with `advMode.notEnabled.spawner` when spawner blocks are off. Otherwise it calls `Spawner.setEntityId(type, random)`, `sendBlockUpdated`, `gameEvent(BLOCK_CHANGE)` and `shrink(1)`.
  - **Anywhere else:** the spawn position is the clicked block when its collision shape is empty, else `relative(face)`. Then `spawnMob(type, player, stack, level, pos, true, !pos.equals(clicked) && face == UP)`, which calls `EntityType.spawn(.., SPAWN_ITEM_USE, ..)`, `consume(1)` and `gameEvent(ENTITY_PLACE)`.
- **`SpawnEggItem.use`:** `getPlayerPOVHitResult(SOURCE_ONLY)` on a liquid block spawns there. This is the fork's `egg_use` liquid branch.
- **Spawn eggs live in the creative spawn eggs tab** (`CreativeModeTabs`, read from the jar by `creative_tabs.rs`).
- **No vanilla spawn egg can name a modded id.** The fork egg is the fork's carrier: `minecraft:poisonous_potato` with `custom_data.iw4l.id`, its name, and food and consumable removed (`item_nbt.rs`). Its look is `Icon::Sprite` of a vanilla egg with a tint, like the ammo rows.

**Plan**
1. **"Spawn eggs summon Margit like /summon, and spawners refuse him"**
   - **`server.rs` `ServerSim::hatch_egg` (`:1985`):**
     - An `elden_boss::def(kind)` goes through `self.summon(kind, position, None, y_rot)`. That gives the Peaceful refusal with `elden_save::PEACEFUL`, and he sleeps with his anchor at that spot, saved as his marker.
     - Then post the same `ENTITY_PLACE` game event as `:1996-1997`, and return.
     - The egg's random yaw (`entities.rs:1320`) becomes his rest yaw.
   - **`server.rs` `ServerSim::spawner_egg` (`:2001`):** an `elden_boss::def(kind)` returns an `Err` before anything changes, for trial and monster spawners alike. The egg is not spent, no message shows, and the spawner keeps its mob.
   - **`spawn_egg.rs` `mob_width` (`:114`):** add `.or_else(|| elden_boss::def(kind).map(|def| def.width as f32))`, so `spawn_rise` measures his 1.4-wide footprint.
2. **"Margit has a spawn egg in the creative spawn eggs tab"**
   - **`fork_items.rs`, the row:**
     - Add `Kind::SpawnEgg` after `Part` (`:15`).
     - Add a `const fn spawn_egg(id, name, item, tint)` after `part` (`:204`). It sets `Bucket::Other`, `max_stack` 64, `Tier::Late`, no weapon, no ammo and not fire proof.
     - Append one row at the end of `ROWS` (`[ForkItem; 95]`): `spawn_egg("iw4l:margit_spawn_egg", "Margit Spawn Egg", "minecraft:skeleton_spawn_egg", Some(GOLD))`. The light grey 26.3 skeleton egg tinted with the existing `GOLD` (`:231`) reads as a gold egg.
   - **`fork_items.rs`, the creative tabs:**
     - Add `pub const SPAWN_EGG_TAB: &str = "minecraft:spawn_eggs";` beside `CREATIVE_TAB`.
     - `creative_entries` leaves `Kind::SpawnEgg` rows out.
     - `add_to_creative` appends the egg rows to the spawn eggs tab, after vanilla's eggs, and still returns whether the combat tab was found.
   - **Everything else comes with the row:**
     - `/give` and the kit (`minecraft_survival/src/commands.rs:134`);
     - both catalogs (`load`);
     - the name `item.iw4l.margit_spawn_egg` (`add_names`);
     - the poisonous potato carrier with name and no food (`item_nbt.rs`), so vanilla 26.3 keeps it as an inedible named potato;
     - the icon and the ground and hand look (`fork_look.rs:46-72` copies vanilla's egg model with a constant tint; render_anim's `icon`, `minecraft_fork_items.rs:282-301`, tints the sprite);
     - the use from either hand (`offhand.rs:104`).
   - **`fork_recipes.rs`:**
     - `group` (`:17-34`) gets a `(Kind::SpawnEgg, _) => "iw4l_spawn_eggs"` arm next to the `Part` arm. No recipe uses it: vanilla spawn eggs have none, and none is written.
     - The tests that pair every row with a recipe (`every_fork_item_has_exactly_one_recipe_of_its_kind_and_group`, `:289-317`, including its exhaustive kind match, and the recipe book test at `:449-466`) count only rows of other kinds.
   - **`fork_items.rs` tests:**
     - `the_roster_has_the_designed_size` (`:1152`) counts one `SpawnEgg`.
     - `the_creative_order_puts_guns_by_class_then_parts_attachments_ammo_and_grenades` (`:1231`) ranks `SpawnEgg` 14.
     - The combat tab tests (`:1281`, `:1335`) expect the rows without eggs.
   - **render_anim:** `the_creative_combat_tab_and_search_list_the_fork_items_by_their_mw2_names` (`minecraft_fork_items.rs:725`) expects combat without eggs.

**Tests**
- **server.rs, `margits_spawn_egg_hatches_him_asleep_with_his_anchor_there`:**
  - Real data via `door_world()`, as `summon_makes_one_asleep_margit_with_his_anchor_there` (`:7518`).
  - `egg_use` with `iw4l:margit_spawn_egg` on stone gives `Spawn { kind: "iw4l:margit", .. }`.
  - `hatch_egg` is Ok: one Elden boss, `State::Asleep`, anchor at that position, his marker tag claimed (`elden_save::claims`), and the `ENTITY_PLACE` event queued.
  - On Peaceful, `hatch_egg` is `Err(PEACEFUL)` and none is added.
- **server.rs, `a_spawner_refuses_margits_egg`:** `spawner_egg` with `iw4l:margit` on a monster spawner and on a trial spawner is an `Err`, and both keep their entity.
- **spawn_egg.rs, `a_margit_egg_rises_by_his_width`:** `mob_width("iw4l:margit") == 1.4`. With the existing fixtures, a fence post one block off the clicked block lifts him onto its top, but it does not lift a 0.6-wide mob.
- **fork_items.rs, `margits_spawn_egg_is_the_last_spawn_egg_and_not_a_weapon`:**
  - The row has a gold skeleton-egg sprite and a stack of 64.
  - `add_to_creative` puts it last in `minecraft:spawn_eggs`, not in combat, and the search list has it.
  - The `MINECRAFTOSS_ROOT` variant on the real tabs: vanilla's eggs come first and Margit's egg last.
- **item_nbt.rs:** the existing loops over every row (`:640`, `:661`) cover the carrier, the vanilla validity and the round trip.
- **minecraft_survival `commands.rs` `the_kit_and_give_accept_fork_items`:** `iw4l:margit_spawn_egg` is in the item registry.
- **minecraft_survival `entities.rs`:** `a_margit_egg_is_spent_only_when_he_hatches`, in the style of the egg queue test at `:3889`. It checks Ok spends one egg, and a Peaceful `Err` spends none.
- **render_anim:** the combat test above, plus a search for "Margit" that finds the egg.

**Owner's steps** (release build; the world from `margit-water` or a fresh one; survival, Normal, by day, flat open ground)

Armour on, golden apples on the hotbar.

1. **The tab.**
   - Run `/gamemode creative` and press E. Open the spawn eggs tab: its last egg is gold and its tooltip says "Margit Spawn Egg".
   - In the search tab, type `margit`: the egg is found.
   - **Screenshot e3-margit-4** (the tab with the tooltip). Then `/gamemode survival`.
2. **Hatch.**
   - Put the kit's eggs on the hotbar. Look at the grass about 6 blocks away and right-click.
   - One egg is used. Margit rises there at once with the mirror sound and golden mist, because you are within 24 blocks, then he walks at you.
   - **Screenshot e3-margit-5.** Then `/difficulty peaceful` and `/difficulty normal`.
3. **Peaceful.** Run `/difficulty peaceful`, then right-click the grass with an egg. Nothing appears and the egg count stays. Run `/difficulty normal`.
4. **Spawners.**
   - Place the monster spawner and the trial spawner from the kit 3 blocks apart, and right-click each with an egg.
   - Nothing changes: no Margit appears inside either, nothing spawns, and both eggs stay.
   - **Screenshot e3-margit-6** (both spawners with the egg count in view).
5. **Water.**
   - Dig a hole 3 by 3 and 2 deep with the shovel and pour the water bucket into its middle.
   - Use an egg on the water. Margit appears in it and, with `margit-water`, floats up and climbs out.
   - Then `/difficulty peaceful` and `/difficulty normal`.
6. **Saving.**
   - Drop one egg with Q. It lies on the ground as a small gold egg; pick it up again.
   - Save and quit through the menu, then load the world. The eggs are still gold and named "Margit Spawn Egg".
7. **Optional, two windows** (`scripts\mp-local.ps1`). The friend uses an egg 6 blocks from them. Margit rises in both windows, and the friend has one egg fewer.

Kit (14 backpack slots):

```
IW4L_MINECRAFT_KIT=iw4l:margit_spawn_egg*8,minecraft:spawner,minecraft:trial_spawner,minecraft:water_bucket,minecraft:iron_shovel,minecraft:netherite_sword+sharpness=5,minecraft:shield,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*16,minecraft:cooked_beef*32
```

---

### Group `margit-heals` (M, two commits)

**Closes:** margit-16.

**Cause**
- **The brain sees no use state.**
  - `crates/elden_boss/src/brain.rs:53-62` `Seen` holds only `id`, `feet`, `eye`, `ground`, `reachable`, `room` and `sight`.
  - `idle` (`:422-440`), `choose` (`:451-482`), `fight.rs:168-174` `reaches` and the hold in `play` (`brain.rs:517-531`, set at `:623-627`) have no healing rule.
  - Neither does `data.rs:346` `When` (`Always`, `Phase`, `Farther`).
- **The entity side passes none.**
  - `arena.rs:197-220` `seen` fills `Seen` from `PlayerCandidate` (`tempt.rs:6`, 134 literals, so it is better not grown).
  - The player's eat and drink state exists on the host for every player, friends included: `crates/minecraft_survival/src/survival.rs:1003` `eating()` returns a `FoodUse` while eating or drinking.
  - It is not passed on. The mobs' `PlayerView` (`entities.rs:130-153`) and `player_controls` (`:155-170`) carry hands, sneaking and sleeping. `third_party/minecraftoss/entities/src/controls.rs:2-15` `PlayerControls` derives `Default`, and its only full literal is `entities.rs:156`.
  - A family can read the controls at any tick: `EntityWorld::player_controls(id)` (`world.rs:5350`), as `rabbits.rs:388-399` does. They are set every tick from `TickInput.controls` (`server.rs:5127`).

**Vanilla 26.3 (javap)**
- No vanilla mob punishes heals. The state vanilla keeps is `LivingEntity.isUsingItem` / `getUseItem` / `getUseItemRemainingTicks`, started by `Consumable.startConsuming` for a stack with the `consumable` component (eating and drinking).
- The nearest vanilla mob that reads a player's hands is `BegGoal.playerHoldingInteresting`, mirrored as `PlayerCandidate.main_hand_wolf_interest` (`tempt.rs:18-20`).
- `Foods` golden apples are `alwaysEdible`, so the owner's steps need no hunger effect.
- The design's own rule (design doc 16): he throws a dagger at a player who is eating, and the brain decides from the view.

**Plan**
1. **"Minecraft mobs can tell when a player eats or drinks"**
   - **`controls.rs`:** add `pub consuming: bool` right after `jumping` (`:14`).
   - **survival `entities.rs`:** add `PlayerView.consuming` right after `jumping` (`:149`), copied in `player_controls` right after `jumping: player.jumping` (`:168`).
   - **The two non-test literals:**
     - `render_anim/src/minecraft_players.rs:1399` (`tick_actor`, every player, friends included): `consuming: alive && player.survival.eating().is_some()` after `jumping,`.
     - `render_anim/src/minecraft_world.rs:1147` (the host): `consuming: alive && survival.eating().is_some()` after `jumping: host_ride.2,`.
   - **The test literals** get `consuming: false` after `jumping`:
     - `actions.rs:2347`, `:2596`, `:2669`
     - `entities.rs:2382`, `:2574`, `:2664`, `:3267`, `:3315`, `:3366`
     - `level.rs:934`, `remote.rs:1933`, `the_end/dragon_fight.rs:1142`
   - It is host state only: no message changes and no friend change.
   - The vendored `controls.rs` counts as patched in the log.
2. **"Margit throws a dagger at a target who eats or drinks"**
   - **`elden_boss` `brain.rs`, `Seen`:** add `pub healing: bool` after `sight`. Fill it in every literal: `brain.rs:820` `player()`, `:1023`, `:1042`, `:1217`, `fight.rs:444-486` and `whole_fight.rs:134`.
   - **The rules,** each only for his current target when `healing && sight`:
     - **(a) In `idle`,** the remaining idle gap ends at once.
     - **(b) In `choose`, through `reaches`,** only moves with throws fit (`dagger_throw`, `backstep_daggers`), the same way `rules.pinned` works. Their distance window is ignored, but the angle and cooldowns stay. With both on cooldown nothing special happens.
     - **(c) In `play`,** a running hold (`playing.holding > 0`, `:517`) ends this tick. The dagger is released at tick 22, about 1.1 s into a 1.6 s meal, and a delayed cane or hammer swing comes down at once.
   - These rules draw no extra random numbers, so fights without heals play out exactly as before (the `whole_fight` test is unchanged).
   - **Entities side:**
     - `arena::seen` (`arena.rs:197`) takes `eating: &[u64]` and sets `healing`. The three arena test calls pass `&[]`.
     - In `elden_bosses.rs` `Family::tick` (`:308-339`), build the list before `family_mob_mut` from `world.player_controls(p.id).is_some_and(|c| c.consuming)` over the eligible players, and pass it through `think` (`:117`).

**Tests**
- **survival `entities.rs`, `an_eating_player_consumes_in_the_mob_controls`:** `player_controls` copies `consuming`, and a player who is not eating has false.
- **`brain.rs`, `a_target_who_eats_between_his_moves_gets_a_dagger_at_once`:**
  - Target at 12 blocks, in sight, healing.
  - For several seeds, the first move after the wake is `dagger_throw` or `backstep_daggers`, started on the first idle tick with no gap.
  - The dagger leaves within 22 ticks of the move's start, because the hold is cut.
- **`brain.rs`, `a_target_he_cannot_see_eating_gets_nothing_special`:** `sight` false gives the same moves and gap as not healing, for the same seed.
- **`brain.rs`, `a_heal_releases_a_held_cane_at_once`:** `begin("cane_combo")`. At the hold (tick 34) the target starts healing, and the clock moves on that same tick.
- **`fight.rs`:** `a_target_on_a_pillar_gets_daggers_not_a_fade` stays green.
- **Entities, `elden_bosses.rs`, `margit_throws_a_dagger_at_an_eating_player`:** `set_player_controls(&[PlayerControls { id: 1, consuming: true, ..Default::default() }])` with the player 12 blocks away. A dagger is in `state.daggers` within 25 ticks of his first idle tick, and the dagger's target is player 1.

**Owner's steps** (release build; survival, Normal, by day, flat open ground)

Armour on. Golden apples and the water bottles go on the hotbar; golden apples can always be eaten.

1. **A dagger for a meal.**
   - Run `/summon iw4l:margit ~ ~ ~10` and back away until you stand 10 to 14 blocks from him, facing him.
   - Wait until a move ends and he walks at you, then eat a golden apple (hold right-click).
   - At once a gold dagger forms in his hand (amethyst chime with a beacon sound). It flies at you before the apple is eaten and takes 2 hearts with the trident's hit sound. Sometimes he hops back and throws two or three.
   - **Screenshot e3-margit-7** (the dagger in the air).
2. **A drink.**
   - Drink a water bottle the same way: he answers the same.
   - Right after he has thrown, the next answer waits for his dagger cooldown of about 3 seconds.
3. **Out of sight.**
   - Clear him (`/difficulty peaceful`, then `/difficulty normal`).
   - Run `/fill ~-2 ~ ~2 ~2 ~2 ~2 cobblestone` (a wall 5 wide and 3 high, 2 blocks toward Z+), then `/summon iw4l:margit ~ ~ ~10` behind it.
   - Stay right behind the wall and eat as soon as he rises: no dagger answers while he cannot see you.
   - When he walks round the wall into view, eat again: the dagger comes.
   - **Screenshot e3-margit-8.**
4. **The delayed cane.**
   - Without the shield, stand 2 to 3 blocks in front of him.
   - In his cane combo, after the second swing he raises the cane and holds it with flaring eyes. Start eating right then: the cane comes down at once instead of after its pause.
5. **Optional, two windows.**
   - The friend hits him with the sword until he turns on the friend.
   - The friend eats: a dagger flies at the friend.
   - The host eats meanwhile, 10 blocks to the side: no dagger comes at the host.
   - **Screenshot e3-margit-9** (the friend's window).

Kit (12 backpack slots; water bottles take one slot each):

```
IW4L_MINECRAFT_KIT=minecraft:golden_apple*32,minecraft:potion@water,minecraft:potion@water,minecraft:potion@water,minecraft:cooked_beef*32,minecraft:netherite_sword+sharpness=5,minecraft:shield,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:cobblestone*64
```

## E3-4. Friends, multiplayer views and skating

The owner's findings, from the known limits of round 10 release 1 and the deferred list:

> **Friends:** no command suggestions, and no frost outline in powder snow. A friend's jump bar and effect blink lag the host by the round trip. (`docs/round10/release-1-tests.md:423`; "Only the host gets Tab suggestions", `release-2-tests.md:78`)
>
> **`/tick freeze`:** [...] The friend window doesn't know the host's tick state, so other things there can still move a little. (`release-1-tests.md:399`, `docs/MULTIPLAYER.md:1350`)
>
> **Skating:** [...] After a stall of more than a second on the host, other skaters show as standing soldiers until the next update. (`release-1-tests.md:398`)

The code check added friends-6: every change to a friend's health, air or frost resends all 45 inventory slots.

Paths and lines are at `minecraft-survival` 4301058a. E2's e2b merge (d565a3d2) lands first. It appends `ToClient::MapData` before `SpawnPosition`, adds `friend.held_maps = HeldMaps::load(..)` after `pumpkin_blur` in the friend's load block (`minecraft_remote.rs:492`), and gives `show_hand` an `images` parameter and a `held_maps.show(&held, ..)` line. After it, `minecraft_remote.rs` lines after :490 move by about 5 and lines after :1858 by about 13. Check every line again before editing. Vanilla facts were checked with javap on the 26.3 `client.jar` (JDK 21 `javap -c -p`).

### Shared points for this area

- **Host owns, friends mirror.** Every new piece of state comes from the host per session. The friend keeps only what it is sent, plus its own input and aim, as vanilla's client does.
- **Wire enums.** The serde variant index of `ToClient` is its wire tag, and the codec tests pin those tags (`client_index`, `codec.rs:226-263`; `seen` in `every_message_to_the_client_round_trips`, `:770-787`). New `ToClient` variants are therefore **appended at the end of the enum**, after `SpawnPosition` and after anything merged since (E2's `MapData`, M2's `GameEvent`). Each one gets its `client_index` arm, a sample in `to_client()` and one more `seen` slot. For a wire enum this is "next to related ones": no other variant's tag moves. Whichever of E3 and M2 merges second renumbers its own arm.
- **Protocol.** Groups `friend-suggestions`, `friend-tick-state` and `friend-slot-changes` change messages. Each such commit adds a line under "## Protocol" in `docs/round10/track-e3-log.md`. Never bump `minecraft_net::PROTOCOL` (11 since release 6, 789f6d23, `messages.rs:7`; decisions.md "PROTOCOL is bumped only at release time").
- **M1-M3 hooks.** No group edits a file or hook the M briefs own (decisions.md "The M briefs give every cross-track hook one owner").
  - The jump bar's HUD lines (`hud/src/minecraft_hotbar.rs:184-185`) and the horse code (`entities/src/horse.rs`, `world/riding.rs`) are M3's. E3 only reads the public `RideJump` and leaves both files alone.
  - M3's m3e sends a negative `jump_bar` for the camel's cooldown sprite (`track-m3.md:1994`). E3 passes it through unchanged.
- **Code rules.** No comments of any kind. Edits change only the lines they need, in the surrounding style. No new files.
- **Order of the groups:** `friend-hud`, `friend-suggestions`, `friend-tick-state`, `friend-slot-changes`.
- **Two windows for the owner's steps:** `& scripts\mp-local.ps1 -Exe iw4l-survival.exe -World <world> -Fresh -HostSound` from `C:\Dev\mashup-crafting`, as set up in `release-1-tests.md` Setup.
  - Host H is `Sos` and friend F is `Friend`.
  - Both windows read the group's kit line.
  - In every new world the host first types `/difficulty normal`, `/gamerule keep_inventory true` and `/op Friend`.

---

### Group `friend-hud` (gaps friends-2, friends-3, friends-10)

#### Cause
- **friends-2, no frost outline.**
  - The HUD draws the outline only when both `ui.vitals` and `ui.freeze_outline` are set (`hud/src/minecraft_freeze.rs:43`; field `frame/src/minecraft_ui.rs:266`).
  - The host loads it with its other screen overlays at world start (`render_anim/src/minecraft_world.rs:639-642`: `portal_frames`, `freeze_outline` from `minecraft_portal::FREEZE_OUTLINE` (`minecraft_portal.rs:153`), `spyglass_scope`, `pumpkin_blur`).
  - The friend's copy of that list (`minecraft_remote.rs:480-492`) has no `freeze_outline`, so it stays `None` on a friend.
  - The frost value itself already reaches the friend: `PlayerView.frozen` (`minecraft_net/src/messages.rs:493`) becomes `percent_frozen` in `HudClock::vitals` (`minecraft_survival/src/views.rs:258`), and `ui.vitals` is set at `minecraft_remote.rs:1300`. `views::tests::a_friends_frost_reaches_their_hearts_and_outline` (`views.rs:579`) passes while the outline never draws.
- **friends-3, the jump bar lags.**
  - The friend shows the host's value: `ui.jump_bar = view.jump_bar` (`minecraft_remote.rs:1302`), from `PlayerView.jump_bar` (`messages.rs:505`). That is filled at `views.rs:92` from `ride_jump_scale` (`render_anim/src/minecraft_players.rs:914`, `riding.rs:713`).
  - Only the host charges: `World::tick_ride_jumps` (`third_party/minecraftoss/entities/src/world/riding.rs:736`) runs `RideJump::tick` (`entities/src/horse.rs:523-560`) on the friend's JUMP button as the host saw it (`render_anim/src/minecraft_riding.rs:61-66`, `old_buttons`).
  - So the bar the friend sees is late twice: once for the input's trip up and once for the view's trip down.
- **friends-10, skaters turn into soldiers during a stall.**
  - `PoseTrack::at` returns `None` once the newest pose is older than `POSE_STALE` = 1 s (`render_anim/src/minecraft_skaters.rs:17`, `:360-375`), and `SkaterTracks::show` then removes the track (`:413-432`).
  - So when the host stalls for more than a second (no state frames), every other skater on a friend's screen becomes the idle soldier until the next frame.
  - On a friend this drop has no purpose:
    - `heard_all` runs on every accepted state frame (`minecraft_remote.rs:319-325`) and already removes any skater missing from the frame's list (`minecraft_skaters.rs:397-411`);
    - `stop()` forgets everything (`minecraft_remote.rs:223`);
    - a frame from a new level lists only that level's skaters.

#### Vanilla 26.3
- `Hud.extractCameraOverlays`: while `LocalPlayer.getTicksFrozen() > 0` it calls `extractTextureOverlay(POWDER_SNOW_OUTLINE_LOCATION, LocalPlayer.getPercentFrozen())`. The client draws it from its own copy of the frost value.
- `LocalPlayer.aiStep` counts `jumpRidingTicks` and sets `jumpRidingScale` on the client:
  - `ticks * 0.1` below 10 ticks, then `0.8 + 2 / (ticks - 9) * 0.1`;
  - only while `jumpableVehicle()` is not null, which needs `PlayerRideableJumping.canJump()`;
  - on release, `sendRidingJump` sends `ServerboundPlayerCommandPacket` `START_RIDING_JUMP` with `getJumpRidingScale() * 100`.
- `JumpableVehicleBar`: draws `jump_bar_cooldown` while `getJumpCooldown() > 0`, else the progress from `LocalPlayer.getJumpRidingScale()`. The bar is the client's own.
- A client entity is never dropped for silence. `ClientLevel` keeps it until `ClientboundRemoveEntitiesPacket`, and its interpolation holds the last target.

#### Plan
1. **"A friend sees the frost outline in powder snow"**
   - `render_anim/src/minecraft_portal.rs`: add `pub(crate) fn screen_overlays(packs: &PackStack, images: &mut Assets<Image>, ui: &mut frame::MinecraftUi)` after `blurred` (`:193`). It sets `ui.portal_frames`, `ui.freeze_outline`, `ui.spyglass_scope` and `ui.pumpkin_blur` exactly as the host's four lines do today.
   - The host (`minecraft_world.rs:639-642`) and the friend (`minecraft_remote.rs:480-492`) call it in place of their own lists, so the two loads can't drift apart again. The drift is the cause.
2. **"A friend's horse jump bar follows their own jump key"**
   - `minecraft_survival/src/views.rs` `HudClock` (`:198-209`, derives `Default`): add fields `ride_jump: minecraftoss_entities::horse::RideJump` and `jump_tick: Option<u64>`.
   - Add `pub fn jump_bar(&mut self, host: Option<f32>, jumping: bool) -> Option<f32>`:
     - `jumpable = host.is_some_and(|scale| scale >= 0.0)`;
     - it calls `ride_jump.tick(jumpable, jumping)` once for every HudClock tick since its last call (`vitals` advances the 20 Hz tick); the first call only records the tick;
     - it returns `Some(self.ride_jump.scale)` while jumpable, and otherwise `host` unchanged: `None` on foot, M3's negative cooldown as sent.
   - `render_anim/src/minecraft_remote.rs`: `show_player` (`:1170`, called at `:614-620`) takes `jumping: bool`. `friend` computes it from `prediction.as_deref().and_then(|state| state.0.last_cmd())` with `cmd.buttons & playerstate_iw4::buttons::JUMP != 0` (`net/src/client/predict.rs:330`). The host reads the same bit through `old_buttons`.
   - `:1302` becomes `ui.jump_bar = hud.jump_bar(view.jump_bar, jumping);`, after `hud.vitals` (`:1299`).
   - The host still decides the jump's power from its own copy of the button sequence (`riding.rs:736`).
3. **"A friend keeps other skaters' pose through a pause in the host's updates"**
   - `minecraft_skaters.rs`: `SkaterTracks` (`:378`) gets `held: bool`, which `heard_all` (`:397`) sets, because only a friend hears whole lists.
   - `PoseTrack` gets `fn last(&self) -> Option<frame::SkatePose>` (the `to` pose).
   - In `show` (`:413-432`), when `at` says stale, a held track keeps `last()` instead of being removed.
   - Removal stays with `heard_all`'s `leave` for skaters the list drops, and with `forget` (`:434`).
   - The host's tracks of friends (`minecraft_host.rs:525-527`, shown at `:579`) never call `heard_all`. They keep `POSE_STALE`, because a friend whose reports stop should be let go.

#### Tests
- `minecraft_portal::tests::the_screen_overlays_load_for_host_and_friend_alike`:
  - skipped without `MINECRAFTOSS_ROOT`, like `minecraft_end.rs:324-332`, with `Assets::<Image>::default()` and `PackStack::open` of `resourcepacks/local/minecraft-26.3`;
  - all four fields are set, and `freeze_outline`'s image has the 26.3 texture's size.
- `views::tests::a_friends_jump_bar_follows_their_own_key_and_passes_the_hosts_cooldown`:
  - with host `Some(0.0)`, holding for 5 and then 10 ticks gives the same scales as a `RideJump` fed the same sequence (0.5 after five held ticks, 1.0 at ten), and a release resets it 10 ticks later;
  - host `None` gives `None`;
  - host `Some(-0.5)` gives `Some(-0.5)` and charges nothing;
  - ticks that pass between calls are all stepped.
- `minecraft_skaters::tests::a_game_shows_the_skaters_it_hears_and_drops_the_ones_that_left` (`:872`): the stale line (`:892`) now expects `[0, 5]`, because a heard list holds through silence. A later list without 0 still drops it.
- New `the_host_lets_a_silent_friend_skater_go`: tracks fed only by `heard` drop after `POSE_STALE`.
- `others_see_a_skater_between_its_last_two_poses` (`:845`, `PoseTrack` still says stale) is unchanged.

#### Owner's steps
World `r10e3f`, two windows (`-World r10e3f -Fresh -HostSound`). Survival, Normal, by day, on flat open ground.
- **E3F-1 (two windows). Frost.**
  - F empties a powder snow bucket on the ground, walks into the white block and stands in it for 8 seconds.
  - In F's window an icy white frame grows in from the screen edges over about 7 seconds. Once it is full, F's hearts turn frozen and F loses half a heart every 2 seconds.
  - H's window shows no frame.
  - **Screenshot E3-1**, both windows. Until this build F's hearts froze, but no frame showed.
- **E3F-2.** F walks out: the frame fades away in about 3 seconds. H does E3F-1 in its own block: H's frame shows as before.
- **E3F-3. Jump bar.**
  - F types `/summon minecraft:horse ~ ~ ~2 {Tame:1b}`, saddles the horse and rides it on flat grass.
  - F holds Space: the jump bar replaces the XP bar and starts filling the moment Space goes down.
  - F lets go at about half: the horse jumps and the bar empties. **Screenshot E3-2** (F's bar half full).
- **E3F-4.** F holds Space until the bar is full and lets go: a high jump. F taps Space five times quickly: the bar flicks up on each tap, the horse hops each time, and the bar never fills on after Space is let go. H rides a second horse in the host window: both bars behave alike. The old lag shows most over the internet.
- **E3F-5.** F gets off (sneak): the XP bar is back at once.
- **Skaters (optional, only with Skate 3 set up as in `release-1-tests.md` part K, and Skating: Enabled in both windows):**
  - **E3F-6 (two windows).** H presses J and rolls past F: F sees H on the board in the skate pose. H presses J again: in F's window the board is gone at once and H stands in the idle pose (K-9 and K-10 still pass).
  - **E3F-7 (two windows).** H skates. Host: `/execute in minecraft:the_nether run tp Friend 0 128 0`: F sees no skater. Host: `/execute in minecraft:overworld run tp Friend ~ ~ ~`: F lands at H and sees H on the board again (K-12).
  - **E3F-8 (two windows, a stall).**
    - H stops on the board in F's view, then clicks and holds the host window's title bar for 3 seconds.
    - If the host window stops drawing while it is held, F's window shows H frozen in place, still on the board in the skate pose. Until this build H stood up as an idle soldier after a second.
    - H rolls on when the title bar is let go. **Screenshot E3-3** (F's window during the hold).
    - If the host window keeps drawing while held, skip this step; the tests cover it.

```
IW4L_MINECRAFT_KIT=minecraft:powder_snow_bucket,minecraft:powder_snow_bucket,minecraft:powder_snow_bucket,minecraft:saddle,minecraft:saddle,minecraft:cobblestone*64,minecraft:golden_apple*8,minecraft:cooked_beef*32
```

**For the log's "State doc changes":** drop "no frost outline in powder snow", "A friend's jump bar [...] lag[s]" and the skater stall sentence from the release known limits.

---

### Group `friend-suggestions` (gap friends-1)

#### Cause
- A friend runs the same suggestion engine as the host: `hud/src/minecraft_suggestions.rs:23` `complete(&chat.input, chat.cursor, chat.command_level, &chat.hints)`.
- Only the command level reaches the friend:
  - `Clientbound::Commands { level }` (`minecraft_commands/src/protocol.rs:40-42`) is sent by `Sessions::send_levels` (`minecraft_survival/src/session.rs:1080-1089`, called at `render_anim/src/minecraft_host.rs:625`);
  - it is applied at `remote.rs:493` and copied to `ui.chat.command_level` at `minecraft_remote.rs:551-554`.
- `ui.chat.hints` (`frame/src/minecraft_chat.rs:52`) stays `CommandHints::default()` on a friend. Only the host calls `commands::refresh_hints` (`minecraft_survival/src/commands.rs:833-858`, called at `minecraft_world.rs:929`).
- So a friend gets the command and literal names its level allows (`docs/MINECRAFT-SURVIVAL.md:374`), but:
  - no player names;
  - no item, block, entity, effect, enchantment, structure, biome or POI ids or tags;
  - no dimensions;
  - no aimed coordinates.
- `CommandHints` (`minecraft_commands/src/suggest.rs:27-60`) is already `Serialize`/`Deserialize`, but no message carries it (`messages.rs:286` `Chat(Clientbound)` has only `System`, `Player` and `Commands`).

#### Vanilla 26.3
- `ClientSuggestionProvider` gets each part from a different place:
  - `getOnlinePlayerNames` reads `ClientPacketListener.getOnlinePlayers` (`PlayerInfo.getProfile().name`, the tab list);
  - `getRelevantCoordinates` and `getAbsoluteCoordinates` read `Minecraft.hitResult`, and only for a `HitResult$Type.BLOCK` hit, `BlockHitResult.getBlockPos` through `prettyPrint`; otherwise the default `~ ~ ~`;
  - `levels()` is `ClientPacketListener.levels()`;
  - `registryAccess()` is `ClientPacketListener.registryAccess()`, the registries the server synced at login, which `suggestRegistryElements` lists;
  - only what the client can't list goes through `customSuggestion` (`ServerboundCommandSuggestionPacket`).
- So: registries come once from the server, names from the player list, and the aim from the client's own hit result.

#### Plan
1. **"The host sends a joining friend the command registries"**
   - `minecraft_survival/src/commands.rs`: add `pub fn registry_hints(chat: &CommandServer) -> CommandHints` after `refresh_hints` (`:833`). It returns `CommandHints::from_server(chat)` with `players` and `dimensions` emptied, because the friend fills those itself.
   - `minecraft_net/src/messages.rs`: append `CommandHints(Box<minecraft_commands::CommandHints>)` to `ToClient`, under the wire-enum rule above. Add its codec arm, sample and `seen` slot.
   - `session.rs` `Sessions::hello` (`:970`): after the `PlayerList` push (`:1021-1024`), `session.outbox.push(ToClient::CommandHints(Box::new(crate::commands::registry_hints(&server.globals.chat))))`.
     - It goes once per join on the reliable lane, like vanilla's login sync.
     - The registries don't change while a world runs.
   - `remote.rs`: add `pub hints: Option<Box<CommandHints>>` beside `command_level` (`:163`, `None` in `new`). The arm `ToClient::CommandHints(hints) => self.hints = Some(hints)` goes next to the `Clientbound::Commands` arm (`:493`).
   - Protocol note in the log.
2. **"Friends get Tab suggestions for names, ids, dimensions and the block they aim at"**
   - `commands.rs`: move the per-frame tail of `refresh_hints` (`:847-857`: players, dimensions, `hit`, `hit_block`) into `pub fn follow_hints(hints: &mut CommandHints, players: Vec<String>, dimensions: impl Iterator<Item = Dimension>, aim: Option<([f64; 3], [i32; 3])>)`. `refresh_hints` calls it with `chat.player_names()`, so the host and a friend fill these fields the same way.
   - `render_anim/src/minecraft_remote.rs` `friend`:
     - next to the command level (`:551-554`), `remote.hints.take()` moves into `ui.chat.hints`;
     - after `let target = aim(..)` (`:647`), call `follow_hints(&mut ui.chat.hints, names, minecraft_survival::switch::DIMENSIONS.into_iter(), aim)`;
     - `names` are the names in `remote.players` (`remote.rs:161`, from `ToClient::PlayerList`, host included);
     - `aim` is `Target::Block { pos, hit, .. }` as `(hit as f64, pos)`. A mob or player under the crosshair gives `None`, like vanilla's non-block hit result.
   - The friend's level still decides which commands show (`chat.command_level`).

#### Tests
- `commands::tests::registry_hints_carry_ids_and_tags_but_no_names`, built like `hints_follow_the_server_and_the_point_in_view` (`:1978`). That existing test still passes through `follow_hints`.
- `session::tests::a_joining_friend_gets_the_command_hints_once` (`host_server` `:2365`, `HELLO` `:2389`):
  - exactly one `ToClient::CommandHints` arrives after `Login` and `PlayerList`;
  - it has items, blocks, biomes and structures, and no players;
  - `send_levels` adds no second one;
  - its encoded size is under `RELIABLE_LIMIT / 8`; write the size into the log.
- `remote::tests::the_hosts_command_hints_wait_for_the_friends_chat`, beside `chat_lines_show_and_the_command_level_is_kept` (`:1670`).
- Codec: the new variant round trips.
- Optional, in `hud`: with hints players `["Sos", "Friend"]` at level 2, `/give ` suggests both names.

#### Owner's steps
World `r10e3s`, two windows (`-World r10e3s -Fresh -HostSound`). Survival, by day.
- **E3S-1 (two windows).** F presses T and types `/give ` (with the space): the list shows the selectors and both names, `Friend` and `Sos`. **Screenshot E3-4** (F's window). Until this build only the selectors showed.
- **E3S-2.** F types `/give @s minecraft:diamond_sw` and presses Tab: it becomes `minecraft:diamond_sword`. Enter: F gets the sword.
- **E3S-3.** With the sword in hand:
  - `/enchant @s minecraft:sha` and Tab give `minecraft:sharpness`;
  - `/effect give @s minecraft:night_` and Tab give `minecraft:night_vision`;
  - `/summon minecraft:co` lists `cod` and `cow`.
- **E3S-4.**
  - `/execute in ` lists `minecraft:overworld`, `minecraft:the_end` and `minecraft:the_nether`.
  - `/locate structure minecraft:village_` lists the five villages.
  - `/locate biome #minecraft:is_` lists biome tags.
  - **Screenshot E3-5.**
- **E3S-5.** F places a cobblestone, looks at it and types `/setblock `: the grey inline suggestion is that block's three coordinates, and Tab writes them. F looks at the sky: the suggestion becomes `~ ~ ~`. **Screenshot E3-6.**
- **E3S-6.** `/clear @s #minecraft:` lists item tags.
- **E3S-7.** Host: `/deop Friend`. F types `/`: only the commands for everyone show (`/help`, `/msg`, `/seed`, `/tell` and `/w`; the fork has no `/list` or `/me`, `minecraft_commands/src/commands.rs:96-106`), and `/msg ` lists both names. Host: `/op Friend`.
- **E3S-8.** H types `/give Friend minecraft:diamond_sw` and Tab: the host's suggestions work as before.

```
IW4L_MINECRAFT_KIT=minecraft:cobblestone*64,minecraft:cooked_beef*32
```

**For the log's "State doc changes":**
- `MINECRAFT-SURVIVAL.md:374` loses "and command suggestions of names and positions".
- `MULTIPLAYER.md:746` and `:1328` (section 20, "command suggestions (`CommandHints`) for friends") say what is sent.
- The release notes drop "Only the host gets Tab suggestions".

---

### Group `friend-tick-state` (gap friends-7)

#### Cause
- The host's `TickRate` (`minecraft_survival/src/tick_rate.rs:54`, `client_timing` `:127-139`, set by `/tick` through `commands.rs:716-730`) feeds only the host's entity views (`render_anim/src/minecraft_world.rs:1174`, `minecraft_levels.rs:447`). No message carries it.
- The friend passes `ClientTiming::default()` (`minecraft_remote.rs:1980`) to `EntityView::advance` (`entity_view.rs:137-148`).
  - So its client ticks keep running at 20 a second under `/tick freeze` or another rate (`:1981-1994`).
  - Those ticks drive campfire smoke and embers from `animate_tick`, rain, particles, item crumbs and its local game-time advance.
- The friend's clocks ignore the freeze too:
  - `Clock::advance` (`remote.rs:119-123`) adds 20 ticks a second through `advance_clocks` (`:379-386`, called at `minecraft_remote.rs:1549`);
  - `time_of` (`session.rs:829-846`) pauses a clock only for `advance_time` or its own pause;
  - so the sky is corrected only by `send_time` once a second (`session.rs:1166`, `minecraft_host.rs:735-745`).
- `git log` shows only host-side `/tick` fixes (4e171e63) and the dragon-only hold (230c8b16).

#### Vanilla 26.3
- Server:
  - `ServerTickRateManager.setFrozen` and `setTickRate` call `updateStateToClients`, which sends `ClientboundTickingStatePacket.from(TickRateManager)` (`tickRate`, `isFrozen`) through `PlayerList.broadcastAll`;
  - `stepGameIfPaused` and `stopStepping` call `updateStepTicks` (`ClientboundTickingStepPacket`, `tickSteps`);
  - `updateJoiningPlayer` sends both to a joining player.
- Client:
  - `ClientPacketListener.handleTickingState` calls `ClientLevel.tickRateManager().setTickRate` and `setFrozen`, and `handleTickingStep` calls `setFrozenTicksToRun`;
  - `ClientLevel.tick` runs `tickTime` (game time and `ClientClockManager.tick`) only while `TickRateManager.runsNormally()`;
  - `Minecraft.getTickTargetMillis` stretches the client tick to `max(default, millisecondsPerTick())` while `isLevelRunningNormally`. The fork's `ClientTiming` already is this rule.

#### Plan
**"Friends follow the host's /tick freeze, step and rate"**, one commit:
- `minecraft_net/src/messages.rs`: append `TickState { tick_seconds: f64, runs_normally: bool }` to `ToClient`. Add its codec arm, sample and `seen` slot.
- `session.rs`:
  - `Session.tick_state: Option<ClientTiming>` beside `spawn` (`:178`, `None` in `new`);
  - `Session::send_tick_state(&mut self, timing: ClientTiming)` beside `send_spawn` (`:383`), which pushes only when the timing changed;
  - `Sessions::send_tick_states(&mut self, timing)` beside `send_spawns` (`:1312`).
- `minecraft_host.rs`: `host.sessions.send_tick_states(server.globals.tick_rate.client_timing());` beside `send_spawns` (`:647`). A new session starts at `None`, so a friend gets the state right after its login, as with vanilla's `updateJoiningPlayer`.
- **Decision for the log.** The fork sends the host's resulting `ClientTiming` whenever it changes, instead of vanilla's rate, frozen flag and step count. A step sends `runs_normally: true`, then `false` when the host's steps run out. The friend then runs exactly what the host's own entity view runs and can't count steps differently.
- `remote.rs`:
  - `pub timing: ClientTiming` beside `clocks` (`:171`, default), with the arm next to `ToClient::Time` (`:503`);
  - `Clock::advance(seconds, timing)` (`:119`) adds `seconds / timing.tick_seconds` ticks, and nothing while `!timing.runs_normally`;
  - `Clocks::advance` (`:133`) and `advance_clocks` (`:379`) pass `self.timing`;
  - the sleep timer (`:381-385`) stays at 20 a second. It is a player timer, and vanilla's players tick under freeze (`TickRateManager.isEntityFrozen` spares them).
- `render_anim/src/minecraft_remote.rs`: `present_entities` (`:1959`) takes the timing from `remote.timing` at its call (`:1765-1777`) in place of `ClientTiming::default()` (`:1980`).
- Protocol note in the log.

#### Tests
- `session::tests::every_friend_gets_the_tick_state_on_join_and_on_each_change`:
  - two sessions each get one `TickState { 0.05, true }`;
  - the same timing again sends nothing;
  - a frozen timing sends `{ 0.05, false }` to both;
  - a session that joins later gets the current state on the next call.
- `remote::tests::a_friends_clocks_hold_while_the_host_is_frozen_and_follow_its_rate`:
  - after `TickState { 0.05, false }`, `advance_clocks(1.0)` leaves the overworld and End clocks where they were, while the sleep timer still counts;
  - after `{ 0.2, true }`, a second adds 5 ticks;
  - `time_weather_players_and_rejections_reach_the_friend` (`:1467`) is unchanged.
- The frozen `ClientTiming` in `EntityView::advance` is already covered (`entity_view.rs:1845`).
- Codec: the new variant round trips.

#### Owner's steps
World `r10e3t`, two windows (`-World r10e3t -Fresh -HostSound`). Survival, by day, flat open ground.
- **E3T-1 (two windows).** F places two campfires on cobblestone next to H, hatches two cows and drops one diamond (Q) between them. In both windows smoke rises from the campfires, the cows wander and the diamond turns.
- **E3T-2.** Host: `/tick freeze`. In both windows the smoke stops rising, the cows stand still and the diamond stops turning. **Screenshot E3-7**, both windows. Until this build the friend's smoke kept rising.
- **E3T-3.** Host: `/tick step 20`: both windows run for about a second, then stand still again.
- **E3T-4.**
  - F quits through the menu and joins again with `& scripts\mp-local.ps1 -Exe iw4l-survival.exe -FriendOnly`.
  - F's world is still frozen from the first frame: no smoke rises.
  - Host: `/tick unfreeze`: everything runs again in both windows.
- **E3T-5.** Host: `/tick rate 5`: in both windows the smoke rises about four times slower. **Screenshot E3-8** (F's window). Host: `/tick rate 20`.
- F's own timers (hunger, breath, effects) stay at 20 a second, as the host's do (`MINECRAFT-SURVIVAL.md:402`).

```
IW4L_MINECRAFT_KIT=minecraft:campfire*2,minecraft:cow_spawn_egg*4,minecraft:diamond*8,minecraft:cobblestone*64,minecraft:cooked_beef*32
```

**For the log's "State doc changes":**
- `MINECRAFT-SURVIVAL.md:379` (the friend's tick state) goes.
- `:402` loses "A remote friend has no copy of the tick state yet".
- `MULTIPLAYER.md:1350` moves out of section 20.
- The release known limit "The friend window doesn't know the host's tick state" goes.

---

### Group `friend-slot-changes` (gap friends-6)

#### Cause
- `PlayerView` carries all 45 inventory slots as NBT (`minecraft_net/src/messages.rs:501`, filled at `minecraft_survival/src/views.rs:69-78`).
- `Session::send_view` (`session.rs:372-381`) runs every host frame for every friend (`send_views`, `minecraft_host.rs:646`).
  - Each time it builds the whole view, 45 NBT writes included.
  - It resends the view whenever `sent_key` changes (`views.rs:154-166`, which zeroes only effect, cooldown and use ticks).
  - That covers every tick of freezing (`frozen`), of breath underwater (`air`) and of a charging jump (`jump_bar`), and every health, food, saturation or XP change.
- How big that gets:
  - a full book alone makes a view 100-300 KB (`books.rs:354` `a_player_view_with_a_full_book_is_measured`);
  - 27 enchanted tools make several KB;
  - the reliable lane gives each friend 64 KiB a second (`net/src/minecraft_wire.rs:27` `RELIABLE_BUDGET`, `:51`; `docs/MULTIPLAYER.md:117`).
- So a friend freezing or swimming underwater queues views faster than they drain, and every reliable message behind them (blocks, chat, menus) arrives late.
- The friend also decodes all 45 stacks from NBT every frame (`minecraft_remote.rs:1222-1225` through `view_stack`, `views.rs:187-197`), again for the hand (`:1898-1905`) and for books (`:1412-1416`).
- Only the crafting cells go per slot (`menus.rs:1416` `contents`, `:1695` `sync`).

#### Vanilla 26.3
- `AbstractContainerMenu.broadcastChanges` walks the slots and calls `synchronizeSlotToRemote` for each.
- The player's synchronizer `ServerPlayer$1.sendSlotChange` sends a `ClientboundContainerSetSlotPacket` for each changed slot only.
- `sendInitialData` sends the whole `ClientboundContainerSetContentPacket` when the menu starts.
- Health, food and XP travel in their own small packets (`ClientboundSetHealthPacket`, `ClientboundSetExperiencePacket`).

#### Plan
**"Friends get only the inventory slots that changed"**, one commit, because the wire and both ends change together:
- `minecraft_net/src/messages.rs`:
  - remove `slots` from `PlayerView` (`:501`) and its `Default` (`:528`); `PLAYER_SLOTS` (`:482`) stays;
  - append `PlayerSlots { slots: Vec<(u8, Option<Nbt>)> }` to `ToClient`, with its codec arm, sample and `seen` slot.
- `views.rs`:
  - `player_view` (`:38`) stops filling slots (`:69-78`);
  - add `pub fn slot_changes(sent: &mut Vec<Option<ItemStack>>, slots: &[Option<ItemStack>]) -> Vec<(u8, Option<Nbt>)>` after `slot_nbt` (`:176`). It compares the first `PLAYER_SLOTS` slots with `sent`, updates `sent`, and writes `slot_nbt` only for changed slots. `sent` is empty before the first call, so the first call returns all 45 (vanilla's `sendInitialData`).
- `session.rs`:
  - `slots: Vec<Option<ItemStack>>` beside `view` (`:177`, empty in `new`);
  - `send_view` (`:372`) first pushes `ToClient::PlayerSlots` when `slot_changes(&mut self.slots, &player.inventory.slots)` is not empty, then handles the view as now. Without slots in its key, a slot change no longer resends the vitals and a vitals change no longer resends the slots.
  - The lane is reliable and ordered, and the session lives for the whole visit, so a respawn or a dimension change needs no full resend.
- `remote.rs`:
  - `pub slots: Vec<Option<ItemStack>>` beside `player` (`:164`, `vec![None; PLAYER_SLOTS]` in `new`);
  - the arm `ToClient::PlayerSlots { slots }` next to `ToClient::Player` (`:467`) decodes each listed slot once with `crate::views::view_stack(&nbt, &self.recipes)`. `recipes` is the friend's prepared book, set at `minecraft_remote.rs:468` before anything is applied. An index of 45 or more is ignored.
- `render_anim/src/minecraft_remote.rs`:
  - `show_player` fills `inventory.slots` from `remote.slots` (`:1222-1225`);
  - `open_held_book` (`:1403`) takes `slots: &[Option<ItemStack>]` instead of `(view, recipes)`; its callers are `:1252` and the test at `minecraft_host.rs:906`;
  - `show_hand`'s `stack_in` (`:1898-1905`) reads `remote.slots`. After e2b it also feeds `held_maps.show`.
- Protocol note in the log.

#### Tests
- `session::tests::a_friend_gets_every_slot_first_and_then_only_the_slots_that_change`, with `Session::new` and `send_view`:
  - the first send has a `PlayerSlots` with 45 entries;
  - a changed slot 3 sends only `(3, Some(..))` and no view;
  - a health change sends a view and no `PlayerSlots`;
  - no change sends nothing.
- `books.rs:354` becomes `a_full_book_goes_once_in_its_slot_change_and_not_in_the_view`:
  - the book's slot change keeps the measured sizes (`width * text .. + 2048`);
  - with the full book in slot 0, a frost-tick view stays under 1 KB.
- `remote::tests::a_friend_decodes_only_the_slots_that_change`: a full `PlayerSlots`, then one with slot 40 only; the other slots stay the same stacks.
- `views::tests::the_view_carries_items_and_guns_and_they_come_back` (`:400`) runs through `slot_changes`.
- `minecraft_host.rs:869` (`a_friends_book_edit_on_the_host_changes_their_slot_and_goes_back_in_the_player_view`): rename it for slot changes. It reads the `PlayerSlots` message and passes the decoded slots to `open_held_book`.
- Codec:
  - `a_player_view_carries_both_grenade_slots` (`codec.rs:939`) becomes `player_slots_carry_both_grenade_slots` (43 and 44);
  - `player_view()` (`:386`) loses its slot lines;
  - `messages.rs:812` `a_new_player_view_has_every_slot` keeps only its `PLAYER_SLOTS` and vitals asserts.

#### Owner's steps
World `r10e3i`, two windows (`-World r10e3i -Fresh -HostSound`). Survival, Normal, by day, flat open ground. The kit fills the backpack with heavily enchanted gear, so each copy of the inventory is several KB.
- **E3I-1 (two windows).**
  - F opens the inventory: all 27 kit items are there, the enchanted ones with their glint.
  - Hovering the diamond sword shows seven enchantment lines.
  - F moves the sword, splits the cobblestone (right-click), drops a stack (Q) and picks it up: each change shows at once.
  - **Screenshot E3-9.**
- **E3I-2.** F scrolls through the hotbar: the first-person hand shows each item, and the M4A1 and USP raise as before. F presses F: the offhand swap shows.
- **E3I-3 (two windows).**
  - F empties a powder snow bucket and stands in it for 10 seconds.
  - Meanwhile H stands in F's view, places, breaks and places a cobblestone, one every 2 seconds, and types three chat lines: `one`, `two`, `three`.
  - F sees each block and each line at once while freezing. **Screenshot E3-10** (F's window, frost frame and H's lines).
  - Until this build, with this kit, H's blocks and lines reached F late and kept trickling in after F left the snow.
- **E3I-4.**
  - Host: `/kill Friend` (keep_inventory is on): after the respawn F's inventory is complete.
  - Host: `/execute in minecraft:the_nether run tp Friend 0 128 0`, then `/execute in minecraft:overworld run tp Friend ~ ~ ~`: F's inventory is complete in both places.
- **E3I-5.** F writes `Hello` in the book and quill and signs it `Test`: the signed book shows in the hand, and right-click opens it at its page.
- **E3I-6.** Host: `/gamemode creative Friend`. F takes a stack from a creative tab into the hotbar: it shows at once. Host: `/gamemode survival Friend`.

```
IW4L_MINECRAFT_KIT=minecraft:powder_snow_bucket,minecraft:powder_snow_bucket,minecraft:writable_book,minecraft:cobblestone*64,minecraft:cooked_beef*32,minecraft:diamond_sword+sharpness=5+looting=3+unbreaking=3+mending=1+fire_aspect=2+knockback=2+sweeping_edge=3,minecraft:diamond_pickaxe+efficiency=5+fortune=3+unbreaking=3+mending=1,minecraft:diamond_axe+efficiency=5+sharpness=5+unbreaking=3+mending=1,minecraft:diamond_shovel+efficiency=5+silk_touch=1+unbreaking=3+mending=1,minecraft:diamond_hoe+efficiency=5+fortune=3+unbreaking=3+mending=1,minecraft:diamond_helmet+protection=4+thorns=3+unbreaking=3+mending=1,minecraft:diamond_chestplate+protection=4+thorns=3+unbreaking=3+mending=1,minecraft:diamond_leggings+protection=4+swift_sneak=3+unbreaking=3+mending=1,minecraft:diamond_boots+protection=4+feather_falling=4+depth_strider=3+unbreaking=3+mending=1,minecraft:bow+power=5+punch=2+flame=1+infinity=1+unbreaking=3,minecraft:crossbow+quick_charge=3+multishot=1+unbreaking=3+mending=1,minecraft:trident+loyalty=3+impaling=5+channeling=1+unbreaking=3+mending=1,minecraft:fishing_rod+luck_of_the_sea=3+lure=3+unbreaking=3+mending=1,minecraft:shears+efficiency=5+unbreaking=3+mending=1,minecraft:iron_sword+sharpness=5+looting=3+unbreaking=3+mending=1+fire_aspect=2+knockback=2,minecraft:iron_pickaxe+efficiency=5+fortune=3+unbreaking=3+mending=1,minecraft:iron_axe+efficiency=5+smite=5+unbreaking=3+mending=1,minecraft:golden_sword+bane_of_arthropods=5+looting=3+unbreaking=3+mending=1,minecraft:netherite_sword+sharpness=5+looting=3+unbreaking=3+mending=1+fire_aspect=2+knockback=2+sweeping_edge=3,minecraft:netherite_pickaxe+efficiency=5+fortune=3+unbreaking=3+mending=1,minecraft:netherite_axe+efficiency=5+sharpness=5+unbreaking=3+mending=1,minecraft:golden_apple*8
```

**For the log's "State doc changes":** `MULTIPLAYER.md`'s description of the player view (grep `PlayerView`) says that slots now travel as `PlayerSlots` changes.

## E3-5. Players: melee, PvP, pushing and cramming

This section closes the open player gaps from release 1's known limits (`docs/round10/release-1-tests.md:396`, `:399`) and the round 9 A7/D leftovers: players-1, players-2 (Fire Aspect), players-3, players-4, players-7, players-8, players-9 and players-10. It leaves out players-11, players-12 and Thorns (the second half of players-2); the reasons are in the left list at the end.

All file:line anchors are on `minecraft-survival` at 9c6efebd. The e2b merge (d565a3d2) and d0b add a few lines to `entities.rs` and `minecraft_terrain/src/server.rs` before E3 starts, so check the anchors again before editing. Vanilla facts were checked with javap on the 26.3 `client.jar` (`C:\Games\Mashup\2010-Rust-Rewrite-Mashup\iw4l-artifacts\minecraft-26.3\client.jar`, JDK 21's javap) and against the 26.3 data in that jar.

### Shared points

- **Protocol: no change.** No group touches a `minecraft_net` message. The sleeper marker is a `pm_flags` bit, and MW2's player state already replicates `pm_flags` in full (`crates/net/src/transport/netfields.rs:242`). Write "no change" under "## Protocol" in the log.
- **Hooks the M briefs own, and how this section avoids them:**
  - `MeleePush` (`crates/minecraft_survival/src/impulse.rs:38-42`) and its literals belong to M1's goats-ram (`blocked`, `docs/round10/track-m1.md:1543`, table at `:129`). Fire Aspect does not add a field to it: the burn travels beside the hit, in `PlayerOutputs.burning`.
  - M2 adds `set_dolphins_grace` after `set_float` in `sim/src/frame.rs` and `carrier.rs` (`docs/round10/track-m2.md:1210`). E3 adds `knockback_recoil` after `set_sprint_blocked` and `lie_in_bed` after `ride`.
  - D0's generic arms in vendored `pushing.rs` (`push_body`, `exposed_mut`, `pushes_on_local_player`) stay untouched.
  - M3's lightning gives players one fire tick through its own `thunderHit` port, not through E3's `Survival::ignite`.
- **Placement and style:**
  - New fields go next to related ones: `PlayerOutputs.recoil`, `.burning` and `.crammed` next to `pushes` and `hits`; the `MobOutcome` and `MobResult` fields next to `broken`; `Output.players_crammed` next to `players_pushed`; `pm_flags::MC_SLEEPING` right after `MC_RIDING`.
  - No code comments. Vendored files keep their existing doc comments, and new lines add none.
  - Edits change only the lines they need.
- **Docs:** don't edit `docs/MINECRAFT-SURVIVAL.md`, `docs/MULTIPLAYER.md` or the release files from the track. Write the new wording under "State doc changes" in the log:
  - drop the five melee limits at `release-1-tests.md:396` and the `/tick freeze` push limit at `:399`;
  - drop "Fire Aspect isn't modelled" at `MINECRAFT-SURVIVAL.md:462`;
  - in `MULTIPLAYER.md` 13.3, a Minecraft kill by a player now counts as that player's MW2 kill.
- **World for the owner's steps:** `& scripts\mp-local.ps1 -Exe iw4l-survival.exe -World r10e3p -Fresh -HostSound`.
  - Host H (`Sos`), friend F, both in survival on Normal, on flat ground by day (`/time set day`).
  - The host types `/op Friend` and `/gamerule pvp true`.
  - The MW2 class is release 1's (an M4A1 with Red Dot Sight).
  - The kit drops everything into the backpack: move the named items to the hotbar first.

### Group `pl-melee`: knockback slows the attacker, and sweeps round a mob wait for its hurt (players-1, players-3)

**Cause**
- **players-1, the recoil:**
  - `pvp.rs:74-76` (`knockback()`) and `:162` add the sprint 0.5 and the Knockback level only to the victim's throw.
  - `Entities::punch_player` (`crates/minecraft_survival/src/entities.rs:678-700`) returns `Some(hit.hurt)`. Then `actions.rs:412-422` only exhausts and wears the weapon. Nothing touches the attacker's motion or sprint, and the `Body` trait (`actions.rs:105-115`) can only add velocity.
  - Mob hits: vendored `AttackResult.knockback` says in words that the player slows and stops sprinting (`third_party/minecraftoss/entities/src/world/combat.rs:61-63`). But `mob_actions::attack` (`crates/minecraft_terrain/src/mob_actions.rs:89-91`) only plays the sound, and `MobResult` (`crates/minecraft_terrain/src/server.rs:245-255`) has no flag for it. Logged at `docs/round9/track-d-log.md:3166`.
- **players-3, the sweep:**
  - `Entities::punch` (`entities.rs:978-984`) sends `server.mob_action`, then at once runs `pvp::swept` round the mob's view box and `strike_players`. That happens before the entity world has decided whether the mob took damage.
  - The entity world does know. `mob_actions::attack` returns early on a hit that does not land (`mob_actions.rs:92-95`). But `MobOutcome` (`:28-41`) and `MobResult` carry no flag for it, and mob results are routed at `entities.rs:358-366`.
  - Recorded as a simplification at `docs/round9/track-d-log.md:2219`.

**Vanilla 26.3** (javap `net.minecraft.world.entity.player.Player`)
- `Player.attack`:
  - `hurtOrSimulate` runs at bytecode 242. Only when it returns true (249 `ifeq 344`):
    - `causeExtraKnockback(target, getKnockback(target, source) + (sprint knockback ? 0.5 : 0), ...)` (252-281);
    - `doSweepAttack` when the sweep flag holds (284-296);
    - `itemAttackInteraction` (318-324).
  - A hit that does not land plays `entity.player.attack.nodamage` (344-348) and sweeps nothing.
- `Player.causeExtraKnockback`, when the strength is above 0:
  - the victim gets `LivingEntity.knockback(strength, sin(yRot), -cos(yRot))`;
  - then the attacker gets `setDeltaMovement(getDeltaMovement().multiply(0.6, 1.0, 0.6))` and `setSprinting(false)` (bytecode 100-120).
  - So a Knockback weapon slows its attacker even without a sprint. Sweep blows use their own `knockback(0.4, ...)` (`doSweepAttack` 190-225) and never slow the attacker.
- `LivingEntity.hurtServer` returns `!blocked || amount > 0` (bytecode 449-465), so a blow that a raised shield stops completely does not land.

**Plan** (3 commits)
1. **"Minecraft knockback hits on players slow the attacker and end its sprint like vanilla"**
   - `crates/sim/src/frame.rs`: `pub fn knockback_recoil(&mut self, id: ClientId) -> bool`, right after `set_sprint_blocked` (`:509-525`).
     - It multiplies `ps.velocity[0]` and `[1]` (MW2's horizontal axes) by 0.6 and keeps `[2]`.
     - Then it calls `movement_iw4::end_sprint` with the same default `UserCmd` that `set_sprint_blocked` builds. So a held sprint key starts the sprint again on the next command, like holding vanilla's sprint key, and a tapped one does not.
     - `carrier.rs`: the forwarder after `set_sprint_blocked` (`:318`).
   - `pvp.rs`:
     - `PvpHit.recoil: bool` next to `knockback` (`:97-104`).
     - `melee` sets it where the blow is made (`:162-163`): `knockback > 0.0` on a landed blow (a Knockback level, or a sprint at full strength).
     - A refused hit never recoils (pvp off, a creative victim, the attacker itself).
   - `entities.rs`:
     - `PlayerOutputs.recoil: bool` next to `pushes` (`:184`).
     - `punch_player` sets the attacker's own `out.recoil` when `hit.recoil`, right after the sounds (`:689-691`).
   - `crates/render_anim/src/minecraft_players.rs` `struck`: before the pushes loop (`:949`), `if outputs.recoil && alive { authority.knockback_recoil(client); }`.
     - The host acts before its own `struck` in the same frame (`minecraft_world.rs:963`, `:1253`).
     - A friend's recoil lands on its next step, one frame later: `step_friend` runs `struck` at `minecraft_players.rs:1755` before `friend_hand` acts at `:2017`.
     - The slower velocity and the ended sprint reach the friend through its player state, like every push.
2. **"Minecraft knockback hits on mobs slow the attacker like vanilla"**
   - `mob_actions.rs`: `MobOutcome.recoil: bool` next to `broken`. `attack` sets it after the `!result.hurt` return (`:92-95`): `result.knockback || bonus.knockback > 0.0`.
   - `server.rs`: `MobResult.recoil` next to `broken` (`:252`), filled in `mob_action` (`:598`). `mob_sweep` leaves it false.
   - `entities.rs` route (`:358-366`): `queue.out.recoil |= result.recoil;`. The attacker slows one entity tick after the hit, like every mob result.
3. **"A Minecraft sweep round a mob reaches the players beside it only when the mob was hurt"**
   - `mob_actions.rs`: `MobOutcome.sweep_from: Option<u64>`, the hit mob's id when `result.hurt && result.sweep`.
   - `server.rs`: `MobResult.sweep_from` next to `recoil`, filled in `mob_action`.
   - `entities.rs`:
     - `PlayerQueue.sweep_waiting: Option<(u64, crate::pvp::Attack, [f64; 6])>` (`:201-226`).
     - `punch` (`:979-984`) no longer strikes. When `pvp.sweeps()`, it stores `(hit.id(), pvp, bounds)` in the attacker's queue.
     - A new `fn strike_sweeps(&mut self, results: &[(u64, MobResult)])` runs right after the route block in `Entities::tick` (`:1742-1747`).
     - For a result whose `sweep_from` matches the waiting id, it takes the entry, runs `pvp::swept(&attack, bounds, None, &self.player_boxes)` and then `strike_players`.
     - A result without `sweep_from` leaves the entry alone, because an interact result can arrive first. The next sweeping punch replaces the entry.
   - Players are swept one entity tick after the hit, with the attacker's position and the mob's box from the moment of the punch.

**Decisions (for the log)**
- The recoil follows the attacker-side `hurt` of `pvp::melee`. So a blow that the victim's raised shield stops completely still slows the attacker, where vanilla's `hurtServer` says it did not land. The shield is decided on the victim's side (`minecraft_players.rs:797-803`), after the attacker has acted. Fire Aspect (next group) is applied on the victim's side, so it does follow the shield.
- Sweeps are paired with their hit by mob id. Two sweeping punches at two different mobs within one entity tick lose the first one's player sweep. That needs Haste far above anything a player gets in survival.

**Tests**
- `sim` `frame::tests::a_knockback_recoil_slows_the_player_and_ends_the_sprint`:
  - velocity (300, -200, 150) becomes (180, -120, 150);
  - `SPRINTING` is cleared and `sprint_button_up_required` stays 0;
  - an unknown client returns false.
- `pvp::tests::a_knockback_hit_makes_the_attacker_recoil_and_a_plain_or_refused_one_does_not`:
  - a full sprint hit and a half-strength Knockback II hit recoil;
  - a plain full hit, a half-strength sprint hit, pvp off and a creative victim do not.
- `entities::tests::a_knockback_punch_recoils_only_the_attacker`, set up like `a_punch_queues_one_hit_on_the_player_in_front_and_none_without_pvp` (`:3631`): the host's outputs have `recoil` after a sprint punch, and the friend's never do.
- `entities::tests::a_mob_hit_with_knockback_recoils_only_its_attacker`: route with `MobResult { recoil: true, .. }` for the friend.
- `mob_actions::tests::a_sprinting_or_knockback_hit_on_a_cow_recoils_and_a_plain_one_does_not`: also a hit inside the cow's hurt time that does not land gets neither `recoil` nor `sweep_from`.
- `entities::tests::a_sweep_round_a_mob_strikes_the_player_beside_it_only_after_the_mob_was_hurt`. With a waiting sweep and the player boxes set:
  - a result without `sweep_from` queues no hit;
  - a matching one queues one sweep hit on the player beside the mob and none on the attacker;
  - another mob's id queues none.

**Owner's steps** (the world from the shared points; both use the kit)
- **PM-1 (two windows).**
  - F stands 8 blocks from H with the plain iron sword and waits a second. Then F presses sprint, runs at H holding W and hits H at full strength.
  - H flies back with the knockback sound, as before.
  - F drops to walking speed at once and keeps walking while W is held, until F presses sprint again.
  - **pl01_sprint_recoil** (F's window right after the hit: F stops about where H stood instead of running on).
- **PM-2 (two windows, optional).**
  - F stands 2 blocks from H with the Knockback II iron sword, swings once at the air, then at once sprints into H and hits, a half-charged hit. H still flies back and F's sprint ends.
  - Same with the plain iron sword: H is barely pushed and F keeps sprinting.
  - **pl02_knockback_recoil.**
- **PM-3 (host).** H hatches a cow 8 blocks away, sprints at it with the diamond sword and hits at full strength. The cow flies back with the knockback sound, and H drops to walking speed. **pl03_mob_recoil.**
- **PM-4 (friend window).** F does PM-3 at a cow of its own: F's sprint ends the same way.
- **PM-5 (two windows), the sweep.**
  - H types `/summon minecraft:cow ~ ~ ~2 {NoAI:1b}`. F stands touching the cow's side.
  - H stands still and gives the cow a full diamond sword hit (not sprinting, not falling).
  - The sweep sound plays, the cow flashes red and F loses half a heart, as in release 1's P-5.
  - **pl04_sweep_lands** (F's hearts).
- **PM-6 (two windows), no sweep when the mob takes no damage.**
  - F eats a golden apple and waits until F's hearts are full.
  - H types `/summon minecraft:cow ~ ~ ~2 {NoAI:1b}` for a fresh cow (the PM-5 cow has 3 of its 10 health left, and the axe would kill it). F stands touching its side.
  - H types `/effect give @s minecraft:haste 60 255`, so every click is a full hit. H puts the iron axe in hotbar slot 1 and the diamond sword in slot 2.
  - H hits the fresh cow with the axe (9 of its 10 health), then at once presses 2 and clicks the cow with the sword. The sword click plays only the no-damage sound, with no red flash and no sweep sound, and F loses nothing.
  - **pl05_no_sweep** (F's hearts).
  - H waits two seconds and clicks the cow with the sword again: the sweep sound plays and F loses half a heart.
  - H: `/effect clear @s`.

```
IW4L_MINECRAFT_KIT=minecraft:iron_axe,minecraft:diamond_sword,minecraft:iron_sword,minecraft:iron_sword+knockback=2,minecraft:cow_spawn_egg*8,minecraft:golden_apple*16,minecraft:cooked_beef*32,minecraft:cobblestone*64
```

### Group `pl-fire-aspect`: Fire Aspect sets mobs and players alight (players-2, Fire Aspect only)

**Cause**
- `crates/minecraft_terrain/src/melee.rs:34-39` `weapon_bonus` reads only Sharpness and Knockback.
- `fire_aspect` appears nowhere in `crates/` or `third_party/minecraftoss` (rg, no match). So no Fire Aspect weapon sets anything alight, mob or player. For hoglins this is noted at `docs/MINECRAFT-SURVIVAL.md:462`.
- **Players:**
  - A player catches fire from an attack only through `DamageCause::ignites` (`crates/minecraft_survival/src/death.rs:300-308`, the small fireball), applied in `Survival::hurt` (`survival.rs:837-839`).
  - `pvp::hit` (`pvp.rs:240-250`) passes no effect and no fire. Logged at `docs/round9/track-d-log.md:3166`.
- **Mobs can already burn:**
  - Every mob body has `Body.fire_ticks` (`third_party/minecraftoss/entities/src/movement.rs:68`).
  - `EntityWorld::body_mut` (`world.rs:5051`) reaches it by id, registry families included through `families.core_mut`.
  - The fireball's ignite uses exactly that (`world/fireballs.rs:386-389`).
- The natural carrier for the players' fire, `MeleePush` (`crates/minecraft_survival/src/impulse.rs:38-42`), is M1's hook this round, so the fire travels beside the hit instead.

**Vanilla 26.3**
- `Player.attack`:
  - It calls `itemAttackInteraction` only on a landed hit (bytecode 318-324). That calls `EnchantmentHelper.doPostAttackEffectsWithItemSource`.
  - `doSweepAttack` calls `EnchantmentHelper.doPostAttackEffects` for each swept entity it hurt (bytecode 228-233).
- `fire_aspect.json` (26.3 data): `minecraft:post_attack`, enchanted `attacker`, affected `victim`, `minecraft:ignite` for 4 seconds plus 4 per level above the first. It requires `is_direct`, which a melee or sweep source is.
- The ignite itself:
  - `Ignite.apply` calls `Entity.igniteForSeconds(s)`, which calls `igniteForTicks(Mth.floor(s × 20))`.
  - `LivingEntity.igniteForTicks` scales the ticks to `Mth.ceil(BURNING_TIME × ticks)`. Fire Protection lowers `burning_time` by 0.15 per level, per armour piece (`fire_protection.json`).
  - `Entity.igniteForTicks` only raises `remainingFireTicks`, never lowers them, then calls `clearFreeze`.
- A blow that does not land sets nothing alight: one inside the hurt time, one a raised shield stops completely, or one with pvp off.
- A killing blow kills before the fire starts, so its drops are not cooked.

**Plan** (2 commits)
1. **"Minecraft Fire Aspect sets mobs alight like vanilla"**
   - `melee.rs`:
     - `pub const FIRE_ASPECT: &str = "minecraft:fire_aspect";` next to `KNOCKBACK`;
     - `pub fn fire_aspect_ticks(level: i32) -> i32`: 0 below level 1, else `(4 + 4 × (level − 1)) × 20`;
     - `pub fn weapon_fire_ticks(stack: Option<&ItemStack>) -> i32`, built on `level(stack, FIRE_ASPECT)`;
     - `WeaponBonus` stays as it is, so its literals in the `pvp.rs` tests stay untouched.
   - `mob_actions.rs` `attack` (`:79-115`):
     - After `death_remains` (`:102`), when the held weapon's fire ticks are above 0, set the target and every id in `result.swept` alight: `world.body_mut(id)` with `fire_ticks = fire_ticks.max(ticks)`.
     - After `death_remains`, so a mob the blow killed drops raw meat, as in vanilla.
   - `mob_actions::sweep` (`:116-122`, the mobs swept round a player victim) takes `fire_ticks: i32` and does the same for its swept ids. It is passed along through:
     - `ServerSim::mob_sweep` (`server.rs:601`);
     - `Command::MobSweep` (`:4237`, a new `fire_ticks` field);
     - the handle method (`:4756`) and its dispatch (`:4961`);
     - `punch_player` (`entities.rs:696`), which passes `minecraft_terrain::melee::weapon_fire_ticks(held)`.
   - Mob armour's Fire Protection is not read, because mob gear carries no burning time in the fork.
   - Fire-immune mobs take the ticks, and their hazard tick (`world/hazards.rs`) burns them off without damage, as it does after lava.
2. **"Minecraft Fire Aspect sets players alight like vanilla"**
   - `pvp.rs`: `Attack.fire_ticks: i32` and `pvp::Blow.fire_ticks: i32`.
     - `pvp_attack` (`entities.rs:655-669`) fills `Attack.fire_ticks` from `weapon_fire_ticks(held)`. The test helper `attack()` (`pvp.rs:270`) sets 0.
     - `blow()` (`:115-129`) copies it into `Blow.fire_ticks`, so the main blow and every sweep blow carry it.
   - `entities.rs`:
     - `PlayerOutputs.burning: Vec<(usize, i32)>` next to `hits`: for each blow that sets its victim alight if it lands, its index in `hits` and the ticks.
     - `strike_players` (`:671-675`) records it as it pushes the hit. A queue's `out` is only ever appended to and taken whole, so the index holds.
   - `survival.rs`:
     - `pub fn ignite(&mut self, ticks: i32)` next to `hurt`: `fire_ticks = fire_ticks.max(protection.burning_ticks(ticks))` and `ticks_frozen = 0`.
     - The small fireball's ignite (`:837-839`) calls it too.
   - `minecraft_players.rs` `struck`:
     - Enumerate the hits (`:793-797`).
     - After `survival.struck` (`:861`), a landed hit with a `burning` entry calls `survival.ignite(ticks)`.
     - A raised shield that stops the whole blow makes `struck` return false, so it stops the fire too, as in vanilla.

**Decision (for the log)**
- The burn rides in `PlayerOutputs.burning`, paired with the hits by index, not in `MeleePush`. `MeleePush` is M1's goats-ram hook this round.

**Tests**
- `melee::tests::fire_aspect_burns_four_seconds_a_level`:
  - 0, 80 and 160 ticks for levels 0, 1 and 2;
  - `weapon_fire_ticks` reads both `{"minecraft:fire_aspect": 2}` and `{"fire_aspect": 1}`.
- `mob_actions::tests::a_fire_aspect_hit_sets_the_cow_and_the_swept_mobs_alight`:
  - a Fire Aspect II diamond sword's full standing hit gives the target and the cow beside it 160 fire ticks;
  - a hit inside the hurt time that does not land sets nothing alight;
  - a plain sword sets nothing alight.
- `mob_actions::tests::a_sweep_round_a_player_sets_the_mobs_beside_them_alight`, next to `a_sweep_round_a_player_hurts_the_mobs_beside_them_and_not_one_far_away` (`:1027`).
- `pvp::tests::a_fire_aspect_blow_and_its_sweeps_carry_the_burn`: a refused hit carries none.
- `entities::tests::a_fire_aspect_punch_marks_the_victims_hit_as_burning`: the `burning` index points at the friend's hit.
- `survival::tests::ignite_keeps_the_longer_burn_and_fire_protection_shortens_it`:
  - 80 ticks after 40 stay 80;
  - four Fire Protection IV pieces give 0 (gate this on the protection data, like the other protection tests);
  - freezing is cleared.

**Owner's steps** (both use the kit)
- **PF-1 (host).**
  - Hatch a cow and hit it once at full strength with the Fire Aspect II diamond sword (the one without Sharpness).
  - It catches fire, runs off burning and flashes red about once a second.
  - It dies in the flames and drops cooked beef.
  - **pl06_burning_cow**, **pl07_steak**.
- **PF-2 (host).** Hatch a cow and kill it with one full hit of the Sharpness V Fire Aspect II sword. It drops raw beef, as in vanilla: the blow kills before the fire starts.
- **PF-3 (host).**
  - Type `/summon minecraft:cow ~-1 ~ ~2 {NoAI:1b}`, `/summon minecraft:cow ~ ~ ~2 {NoAI:1b}` and `/summon minecraft:cow ~1 ~ ~2 {NoAI:1b}`.
  - Stand still facing the middle cow and hit it at full strength with the Fire Aspect II diamond sword.
  - The sweep sound plays and all three cows burn.
  - **pl08_fire_sweep.**
- **PF-4 (two windows).**
  - F hits H at full strength with the Fire Aspect I iron sword. H burns for about 4 seconds in both windows, losing half a heart each second.
  - **pl09_burning_player** (both windows).
  - Then F hits H with the Fire Aspect II diamond sword: H burns for about 8 seconds.
  - H puts the fire out with the water bucket if needed.
- **PF-5 (two windows).**
  - H holds the shield in the offhand and raises it facing F. F hits with the Fire Aspect I iron sword.
  - The block sound plays, H loses no hearts and doesn't catch fire.
  - **pl10_shield_no_fire.**
- **PF-6 (two windows).**
  - H puts on the four Fire Protection IV pieces. F hits H with the Fire Aspect II diamond sword.
  - H loses hearts but never catches fire, because four Fire Protection IV pieces cut the burn time to nothing.
  - H takes the armour off.
- **PF-7 (two windows), a sweep burns a player.**
  - H types `/summon minecraft:cow ~ ~ ~2 {NoAI:1b}` and stands touching it.
  - F stands still 2 blocks away and hits the cow at full strength with the Fire Aspect II diamond sword.
  - The cow and H both catch fire.
  - **pl11_swept_player_burns.**

```
IW4L_MINECRAFT_KIT=minecraft:diamond_sword+fire_aspect=2,minecraft:diamond_sword+sharpness=5+fire_aspect=2,minecraft:iron_sword+fire_aspect=1,minecraft:shield,minecraft:diamond_helmet+fire_protection=4,minecraft:diamond_chestplate+fire_protection=4,minecraft:diamond_leggings+fire_protection=4,minecraft:diamond_boots+fire_protection=4,minecraft:cow_spawn_egg*16,minecraft:water_bucket,minecraft:golden_apple*16,minecraft:cooked_beef*32,minecraft:cobblestone*64
```

### Group `pl-pushing`: sleepers and riders push walkers (players-7, players-9)

**Cause**
- **Sleepers** are held in bed with `SimWorld::ride` (`crates/sim/src/frame.rs:465-482`, which sets `MC_RIDING`):
  - for the host at `crates/render_anim/src/minecraft_world.rs:747`;
  - for friends in `lay_friends` (`minecraft_players.rs:1477-1502`, the ride at `:1496`).
- Both `push_bodies` (`crates/sim/src/step.rs:1895-1916`) and the friend's prediction bodies mark a sleeper `riding`. The prediction body is `PredictionRemoteBody` (`crates/sim/src/world.rs:251-261`, filled at `:3216-3227`).
- `minecraft_push` skips riding pushers (`step.rs:1938`), so a sleeper pushes nobody. The test `a_player_lying_in_bed_is_held_like_a_rider_and_not_pushed` (`step.rs:2599`) pins that.
- The same filter stops a rider from pushing a walker. `the_push_respects_height_spectators_and_riders` pins it (`:2551`, the rider at `:2560-2561`).
- Both are recorded as not vanilla in `docs/round9/track-a-log.md:524`.
- `pushable` (`step.rs:1918-1921`) already keeps riders and sleepers from being pushed, which is right.
- Mobs never push players in the fork, because the vendored `pushes_on_local_player` (`pushing.rs:329`) has no caller. So the mount itself does not push a walker either. That stays as it is.

**Vanilla 26.3**
- `LivingEntity.push(Entity)` returns while `isSleeping()` (javap, bytecode 1-9): a sleeper is never pushed.
- `LivingEntity.aiStep` calls `pushEntities` unconditionally (bytecode 806-807), whether the entity is sleeping or riding.
  - `doPush(other)` calls `other.push(this)`, which moves the walker away.
  - The sleeper's box is `SLEEPING_DIMENSIONS` = `EntityDimensions.fixed(0.2, 0.2)` at its bed position.
  - A passenger ticks through `rideTick`, then `aiStep`, then `pushEntities` with its own box.
  - `Entity.push(Entity)` only skips `isPassengerOfSameVehicle`.
- On the walker's client, `EntitySelector.pushableBy` keeps only the local player. So the remote sleeper's or rider's `pushEntities` pushes the local player there.

**Plan** (2 commits)
1. **"A sleeping Minecraft player pushes others with vanilla's small sleeping box"**
   - `crates/playerstate_iw4/src/playerstate.rs`: `pm_flags::MC_SLEEPING: u32 = 0x0020_0000`, right after `MC_RIDING` (`:183`).
     - Check the bit first, the way round 8 checked `MC_POWDER_WALKER` (`docs/round8/track-c-log.md:128`): rg it in `movement_iw4`, `sim`, `net` and `playerstate_iw4`.
     - At 9c6efebd no code reads 0x0010_0000 or 0x0020_0000 as a pm flag. `eflags::RADAR_JAM` (0x200000) and the button bits are other fields.
     - IW4 itself uses 0x40000, 0x80000 and 0x400000.
   - `frame.rs`: `pub fn lie_in_bed(&mut self, id: ClientId, bed: Option<[f32; 3]>) -> bool`, right after `ride`.
     - It calls `ride(id, bed)`, then sets `MC_SLEEPING` when there is a bed and clears it when there is none.
     - `ride` itself clears `MC_SLEEPING` (one line), so a mount seat never keeps it.
     - `carrier.rs`: the forwarder after `ride` (`:310`).
   - `render_anim`: the bed holds call `lie_in_bed` instead of `ride`: `minecraft_world.rs:747` and `:750`, and `minecraft_players.rs:1491` and `:1496`. `minecraft_riding` keeps `ride`.
   - `step.rs`:
     - `PushBody.sleeping` next to `riding`, read from `MC_SLEEPING` in both halves of `push_bodies`.
     - `PredictionRemoteBody.sleeping` next to `riding` (`world.rs:260`, filled at `:3226`).
     - In `minecraft_push`, a sleeper passes the filter (`!other.riding || other.sleeping`). It overlaps with half width 0.1 and height 0.2 blocks at its origin (vanilla's 0.2 box) instead of 0.3 and its hull. The overlap test becomes `< PUSH_HALF_WIDTH + other_half`.
     - `pushable` is unchanged, so a sleeper is still never pushed.
2. **"Minecraft riders push players they bump into like vanilla"**
   - `step.rs:1938`: the riding filter goes. `PushBody.riding` and `PredictionRemoteBody.riding` go with it, because nothing else reads them (rg `\.riding` in `crates/sim`).
   - A rider pushes with its own hull at its seat.
   - Riders are still never pushed (`pushable`), which also covers vanilla's same-vehicle rule.

**Tests**
- `step.rs`: `a_player_lying_in_bed_is_held_like_a_rider_and_not_pushed` becomes `a_sleeper_is_not_pushed_and_pushes_with_its_small_box`:
  - after `lie_in_bed` the sleeper is not `pushable`;
  - a walker 0.3 blocks from its origin at bed height is pushed away;
  - a walker 0.5 blocks away is not, and neither is one whose feet are above the 0.2 box;
  - `lie_in_bed(None)` clears both flags, and the player pushes with a full box again.
- `the_push_respects_height_spectators_and_riders` (`:2551`):
  - commit 1: the `found` tuples become `(entnum, sleeping)`;
  - commit 2: the rider line expects a push.
- `frame::tests::lie_in_bed_holds_the_player_and_marks_it_sleeping`: `ride` to a seat clears the mark.
- `world.rs`: a remote snapshot player with `MC_SLEEPING` gives a `sleeping` prediction body, next to the existing prediction tests.

**Owner's steps** (both use the kit)
- **PP-1 (two windows).**
  - `/time set night`. H places the white bed on flat ground and lies down.
  - F walks onto the pillow half of the bed, onto H's head, and lets go of the keys. F slides slowly sideways off H and stops just beside H, while H keeps lying.
  - Before this build F could stand inside H.
  - **pl12_sleeper_push** (F's window).
- **PP-2 (two windows).** F stands on the foot half of the bed: F is not pushed.
- **PP-3 (two windows).**
  - H gets up. F places the red bed and lies down. H walks onto F's pillow and is pushed off it in H's window.
  - **pl13_sleeper_push_host.**
- **PP-4 (two windows).**
  - `/time set day`. H types `/summon minecraft:horse ~ ~ ~3 {Tame:1b}`, puts the saddle on, gets on and stands still.
  - F walks into the middle of the horse from its side, where H sits. F is pushed back out of H's spot, and H and the horse do not move.
  - **pl14_rider_push** (F's window).
- **PP-5 (two windows).** Swap roles: F rides and H walks in. H is pushed in H's window.

```
IW4L_MINECRAFT_KIT=minecraft:white_bed,minecraft:red_bed,minecraft:saddle,minecraft:cobblestone*64,minecraft:cooked_beef*16
```

### Group `pl-cramming`: frozen carriers are pushed, the cramming rule works, players are crammed (players-10, players-8)

**Cause**
- **players-10:**
  - `crates/minecraft_terrain/src/server.rs:1433-1436` runs `mobs.push_from_players` only when not frozen.
  - Under `/tick freeze`, the frozen tick (`tick_ridden_mobs`, `:1389-1394`) ticks the mobs that carry players, but no player pushes them.
  - This was kept "as before" in `docs/round9/track-d-log.md:2417` and logged at `:3167`.
  - Vendored `push_from_players` (`pushing.rs:299-323`) has no carrier filter, while `player_carriers()` (`world/riding.rs:811-821`) already lists the ticking stack.
- **players-8, the rule:**
  - `EntityWorld.max_entity_cramming` is fixed at 24 (`world.rs:2639`, `:2762`). Only the families test sets it (`world/families/tests.rs:612`).
  - `apply_game_rules` (`server.rs:3657-3671`) never passes it on, so `/gamerule max_entity_cramming` changes nothing even for mobs.
- **players-8, players:**
  - Cramming exists for mobs only (`pushing.rs:393-406`). `push_from_players` uses a player's `pushable_around` only to push mobs.
  - `DamageCause::Hazard(Hazard::Cramming)` and its death line exist (`crates/minecraft_survival/src/death.rs:170`, `:215`), but nothing hurts a player with it.
  - `docs/round9/track-a-log.md:524` lists it as a missing vanilla behaviour.

**Vanilla 26.3**
- `TickRateManager.isEntityFrozen` is `!runsNormally() && !(e instanceof Player) && e.countPlayerPassengers() <= 0`.
  - A ServerPlayer keeps ticking under freeze. Its `LivingEntity.pushEntities` calls `doPush`, then `Entity.push`, which adds velocity to the carrier, and the carrier ticks and moves.
  - A frozen mob would only store the velocity until the unfreeze.
- `LivingEntity.pushEntities` (javap, bytecode 1-157):
  - It gets `Level.getPushableEntities(this, box)`. On the server, `EntitySelector.pushableBy` keeps every pushable entity, mobs and players.
  - If `MAX_ENTITY_CRAMMING` is above 0, the list is longer than `rule − 1` and `this.random.nextInt(4) == 0`, it counts the entities that are not passengers.
  - Above `rule − 1`, it calls `hurtServer(damageSources().cramming(), 6.0)` (bytecode 133-153).
  - A ServerPlayer runs this in its own tick.
- The `cramming` damage type (26.3 data): exhaustion 0.0, message `cramming` ("%1$s was squished too much"), in `#bypasses_armor`, and through that in `#bypasses_shield` too.

**Plan** (3 commits)
1. **"Minecraft players on foot push the mounts other players ride while ticks are frozen"**
   - `pushing.rs`:
     - `push_from_players` becomes a thin call of a private `push_from_players_onto(players, ticks, only: Option<&HashSet<u64>>)`, which skips mobs outside `only`.
     - A new `pub fn push_carriers_from_players(&mut self, players, ticks)` passes `player_carriers()`.
   - `server.rs:1433-1436`: the frozen branch calls `push_carriers_from_players`.
2. **"The max_entity_cramming game rule reaches Minecraft mobs"**
   - Vendored `world.rs`: `pub fn set_max_entity_cramming(&mut self, limit: i32)` and `pub fn max_entity_cramming(&self) -> i32`, next to `set_mob_griefing` (`:4496-4503`).
   - `server.rs` `apply_game_rules`: `self.mobs.set_max_entity_cramming(rules.int("max_entity_cramming"));` after `set_mob_griefing` (`:3671`).
3. **"Minecraft players are hurt by cramming like vanilla"**
   - `pushing.rs` `push_from_players_onto`, before a player's pushes, for every pushable player, in both the normal and the frozen call:
     - It runs only when the rule is above 0 and the player's `pushable_around` list (mobs and other players) is longer than `rule − 1`.
     - It rolls `nextInt(4) == 0` on a random kept per player in the entity world, seeded from the player id. Never the level's random or a mob's, so mob replays keep their sequences.
     - It counts the entries that are not passengers (`vehicle_of`), and above `rule − 1` queues the player's id.
     - `pub fn take_crammed_players(&mut self) -> Vec<u64>` hands the queued ids out.
   - `server.rs`: `Output.players_crammed: Vec<u64>` next to `players_pushed` (`:4471`), filled in `run_tick` next to `take_player_hits` (`:5166`).
   - `entities.rs` route: next to the `players_pushed` loop (`:316-320`), add 1 to `PlayerOutputs.crammed: u32` for each entry.
   - `survival.rs`:
     - `pub fn crammed(&mut self, random: impl FnMut() -> f32) -> Option<PlayerSound>` runs `hurt_aloud(Hazard::Cramming, 6.0, Armor::default(), ...)` like the other hazards. No armour, because cramming is in `#bypasses_armor`.
     - `Cramming` joins `OnFire | Freeze` in the zero-exhaustion match (`:762`), because `cramming.json` has exhaustion 0.0.
     - `hurt_aloud`'s invulnerable check spares creative players.
   - `minecraft_players.rs` `struck`: one `survival.crammed(...)` per entry while alive, its hurt sound played the way the rider falls play theirs (`:864-880`).

**Decision (for the log)**
- Under `/tick freeze`, a frozen mob that is not carrying a player gets no push at all. Vanilla would store the velocity and throw the mob at the unfreeze.

**Tests**
- Vendored entities (in `world::pushing` tests, or next to `two_registered_mobs_push_apart_and_cram` in `families/tests.rs`):
  - `a_frozen_tick_pushes_only_the_player_carriers`: a ridden cow and a loose cow both overlap a player, and only the ridden one gets velocity.
  - `a_player_among_more_than_the_rule_allows_is_crammed_a_quarter_of_the_time`. With rule 3:
    - three cows overlapping the player queue it within 40 ticks;
    - two cows never do;
    - a passenger cow is not counted;
    - with rule 0 it is never queued.
- `server::tests::the_cramming_rule_reaches_the_mobs`, next to `command_weather_sets_rain_for_its_duration`: 24 by default, 3 after `apply_game_rules` with `max_entity_cramming 3`.
- `entities::tests::a_crammed_player_gets_its_cramming_and_no_one_else`: a route test.
- `survival::tests::cramming_takes_three_hearts_through_armour_and_names_the_death`: 6 damage through full diamond armour, death line `death.attack.cramming`; a creative player takes none.

**Owner's steps** (both use the kit)
- **PC-1 (two windows).**
  - `/time set day`. H types `/summon minecraft:horse ~ ~ ~3 {Tame:1b}`. F saddles it, gets on and stands still.
  - H types `/tick freeze` and walks into the horse's side. The horse, with F on it, slides away from H as it does without the freeze.
  - **pl15_frozen_push.**
  - H types `/tick unfreeze`.
- **PC-2 (host).**
  - With the shovel, dig a hole 1 by 1 and 2 deep. Type `/gamerule max_entity_cramming 3` and use 6 chicken eggs on the floor of the hole.
  - Within a few seconds chickens flash red and die, dropping feathers and raw chicken, until 3 are left. Before this build the rule was ignored and all 6 lived.
  - **pl16_cramming_rule.**
- **PC-3 (host).** `/gamerule max_entity_cramming 0`, then use 6 more eggs in the hole: no chicken is hurt.
- **PC-4 (host).**
  - `/gamerule max_entity_cramming 3`. H puts on the chestplate.
  - On open ground at least 30 blocks from the world spawn (so the respawn cannot land among the golems), type `/summon minecraft:iron_golem ~ ~ ~ {NoAI:1b,Health:1f}` three times, so three golems stand inside H.
  - H loses 3 hearts at a time with the hurt sound, the chestplate does not help, and H dies. The chat reads "Sos was squished too much".
  - **pl17_squished.**
  - After respawning, walk back and punch each golem once: with 1 health it dies. (`/kill` takes only players until kill-selectors in e3s; if a golem kept its 100 health, kill it with the sword.)
- **PC-5 (two windows).**
  - F does PC-4 in F's window, at another spot 30 blocks from the spawn. Both chats read "Friend was squished too much".
  - **pl18_friend_squished.**
  - Then F punches the three golems away, and H types `/gamerule max_entity_cramming 24`.

```
IW4L_MINECRAFT_KIT=minecraft:chicken_spawn_egg*16,minecraft:saddle,minecraft:diamond_chestplate,minecraft:iron_shovel,minecraft:diamond_sword+sharpness=5,minecraft:cobblestone*64,minecraft:golden_apple*8,minecraft:cooked_beef*16
```

### Group `pl-kill-credit`: Minecraft kills by a player give that player the MW2 kill (players-4)

**Cause**
- `settle` builds the Minecraft death and pushes it: `crates/render_anim/src/minecraft_players.rs:1074-1115`, the literal at `:1089`, `sim::voxel::MinecraftDeath { message, killer: String }`, then `push_player_damage`.
- `MinecraftDeath` and `PlayerDamage` (`crates/sim/src/voxel.rs:362-386`) carry no killer client.
- `apply_block_world_damage` (`crates/sim/src/damage.rs:181-226`) makes a Hit with `attacker: None` and `MINECRAFT_MEANS` (`MOD_TRIGGER_HURT`).
- `minecraft_killer` (`crates/sim/src/script/host/players.rs:142-180`) creates a synthetic `trigger_hurt` entity on the victim's opposite team. `finishplayerdamage` (`crates/sim/src/script/host/natives/player.rs:437-450`) passes it as inflictor and attacker, so MW2 credits no player with the score or the killstreak.
- The killer is already known on the Minecraft side:
  - `CombatTracker.hurt_by_player` (`crates/minecraft_survival/src/death.rs:382`, set at `:458-460`) holds it for 100 ticks (`KILL_CREDIT_TICKS`, `:11`; expired at `:470-475`);
  - it covers Player, PlayerExplosion and PlayerFireworks causes.
- Logged at `docs/round9/track-d-log.md:2261`.
- The Minecraft world runs `dm` with score limit 0 (`docs/MULTIPLAYER.md` 13.4), so credited kills can never end the world.

**Vanilla 26.3**
- `ServerPlayer.die` (javap): `CombatTracker.getDeathMessage` (bytecode 35), then `getKillCredit()` (244) and `killer.awardKillScore(this, source)` (269).
- `LivingEntity.getKillCredit` returns `lastHurtByPlayer` (kept 100 ticks), else `lastHurtByMob`.
- In MW2's terms, the credited attacker is the player object passed to `finishPlayerDamage` and on to `Callback_PlayerKilled`. A gun kill in a Minecraft world already works that way (`docs/MULTIPLAYER.md` 13.3: the obituary, killcam and card).

**Plan** (1 commit)
**"Minecraft kills by a player count as that player's MW2 kills"**
- `death.rs`: `CombatTracker::kill_credit(&self) -> Option<u32>` next to `hurt_by` (`:455-462`), the `hurt_by_player` client. `survival.rs`: `pub fn kill_credit(&self)` next to `death_line` (`:602`).
- `voxel.rs:362-366`: `MinecraftDeath.killer_client: Option<u32>`.
  - The literal in `settle` (`minecraft_players.rs:1089`) fills it from `survival.kill_credit()`. It keeps the client only when it is another client that is still in the chat roster (`globals.chat.name`).
  - The test literal at `voxel.rs:3137` gets `None`.
- `damage.rs:196-212`:
  - The Hit's `attacker` is that client on the killing push (`damage.death`), and `None` otherwise.
  - Building the Hit moves into a small `fn block_world_hit(damage: &PlayerDamage, origin) -> script_player::Hit`, so it can be tested.
  - The weapon stays 0 (`none`).
- `crates/sim/src/script/runtime/state.rs:56`: `minecraft_attacker: Option<u64>` next to `minecraft_killer`. It holds the player object of the hit's real attacker; `minecraft_damage` (`players.rs:116-132`) sets it before `player_damage` and clears it after.
- `finishplayerdamage` (`natives/player.rs:441-450`):
  - The synthetic killer stands in only when the hit has no player attacker (`minecraft && attacker.is_none()`).
  - Otherwise the inflictor and attacker come from the script's args. Then `script_player::kill` (`:487`) and the game type's `Callback_PlayerKilled` credit the player, as for a gun kill.
- The `obituary` native (`natives/player.rs:534-545`) and the `credited` check in `minecraft_damage` (`players.rs:127-131`) compare against the expected killer: `minecraft_attacker` if set, else `minecraft_killer`.
  - So the MW2 kill-feed line stays hidden for every Minecraft kill, the owner's round 7 choice (`docs/MULTIPLAYER.md` 13.3).
  - And `push_minecraft_death` (`damage.rs:223-225`) still puts the Minecraft killer on the kill card.
- Unchanged paths:
  - a self-kill (your own TNT) and a death with no player credit keep today's synthetic killer;
  - a killer who has left gives no credit, because the world entity stands in for an undefined player object (`players.rs:48-51`).

**Decisions (for the log)**
- Only the killing blow names the attacker, so Minecraft hits give no MW2 assists. Vanilla has no assists either.
- The MW2 killcam, score popup and killstreak count follow from the game type exactly as after a gun kill.
- Nothing in the repo runs the MW2 scripts, so the in-game steps below are the check of the script side.

**Tests**
- `death::tests::kill_credit_names_the_last_player_for_five_seconds`: a player hit gives credit; a mob hit after it keeps it; 101 ticks later it is gone.
- `sim` `damage::tests::a_minecraft_kill_names_its_killer_as_the_attacker_and_other_hits_do_not`, through `block_world_hit`.
- `players::tests`: `hides_minecraft_obituary` hides a Minecraft hit's obituary when the expected killer is a real player object, and still shows another attacker's (next to `:1304-1318`).

**Owner's steps** (both use the kit; `/gamerule pvp true` and `/gamerule keep_inventory true`)
- **PK-1 (two windows).**
  - Both hold Tab: the MW2 scoreboard shows 0 kills for both.
  - H kills F with the diamond sword. Both chats read "Friend was slain by Sos", and the MW2 kill feed in the corner shows no line, as before.
  - H holds Tab: H has 1 kill and F 1 death.
  - **pl19_sword_kill_scored** (H's Tab).
- **PK-2 (two windows).** H places a TNT beside F and lights it, and F stays. The chats read "Friend was blown up by Sos", and H's kills read 2.
- **PK-3 (two windows).**
  - F hits H once with a fist. Within 5 seconds H pours the lava bucket into a hole and walks into it.
  - The chats read "Sos tried to swim in lava to escape Friend", and F's kills read 1.
  - **pl20_lava_credit** (F's Tab).
- **PK-4 (two windows).**
  - H hatches a zombie and lets it kill H: "Sos was slain by Zombie", and nobody's kills change.
  - H lights a TNT 20 blocks away from F and stays: H's kills do not go up for killing H.
- **PK-5 (two windows).**
  - F kills H with the M4A1. The MW2 kill-feed line and the killcam come as before, and F's kills go up by one.
  - **pl21_gun_kill** (H's window).
  - If a Tab count stays at 0 after PK-1, send the host's newest log from `iw4l-artifacts\logs` with the screenshots.

```
IW4L_MINECRAFT_KIT=minecraft:diamond_sword,minecraft:tnt*8,minecraft:flint_and_steel,minecraft:lava_bucket,minecraft:zombie_spawn_egg*2,minecraft:golden_apple*16,minecraft:cooked_beef*32,minecraft:cobblestone*64
```

### Section kit (all five groups, 26 stacks)

```
IW4L_MINECRAFT_KIT=minecraft:iron_axe,minecraft:diamond_sword,minecraft:iron_sword,minecraft:iron_sword+knockback=2,minecraft:diamond_sword+fire_aspect=2,minecraft:diamond_sword+sharpness=5+fire_aspect=2,minecraft:iron_sword+fire_aspect=1,minecraft:shield,minecraft:diamond_helmet+fire_protection=4,minecraft:diamond_chestplate+fire_protection=4,minecraft:diamond_leggings+fire_protection=4,minecraft:diamond_boots+fire_protection=4,minecraft:cow_spawn_egg*16,minecraft:chicken_spawn_egg*16,minecraft:zombie_spawn_egg*2,minecraft:white_bed,minecraft:red_bed,minecraft:saddle,minecraft:tnt*8,minecraft:flint_and_steel,minecraft:lava_bucket,minecraft:water_bucket,minecraft:iron_shovel,minecraft:golden_apple*16,minecraft:cooked_beef*32,minecraft:cobblestone*64
```

## E3-6. Guns, grenades and MW2 items

This section has five open gaps in three commit groups, which run in this order:

| Group | Gaps | Size | Commits |
|---|---|---|---|
| `guns-armour` | guns-4 | M | 2 |
| `guns-targets` | guns-16, guns-15, and the guns-14 check (step T-10) | M | 2 |
| `guns-flare` | guns-13, guns-11 | L | 3 (the third starts with a go/no-go check) |

Paths and lines are as of `minecraft-survival` 9c6efebd. Check them again before editing, because d0b, e2b, w3 and the M tracks are merging in the meantime. Vanilla facts come from javap on the 26.3 `client.jar` and its data. MW2 facts come from the owner's `common_mp.ff`, which is read but never committed.

Rules for all three groups:
- No code comments. Edits change only the lines they need. New files pass `rustfmt --check`.
- `VoxelEvent::MobMelee` goes right after `MobShot`, in the enum and in every match. New fields and functions go next to related ones, never at the end of a shared struct.
- No group changes a `minecraft_net` message. The voxel queues stay on the host, and the glowstick reaches friends through MW2's own snapshot. So the log gets nothing under "## Protocol", and `PROTOCOL` stays as it is.
- No game events: those belong to track W (W0 rules).
  - Lead knot events are w3's. The bullet and knife routes here use the same `KnotAction::Punch`.
  - If w3 merges first and puts its knot line in `Entities::punch_knot` instead of the server's `knot_action`, add that same one line after each new knot route here (W0 rule 3).
- Every sim change runs only in Minecraft worlds: behind `world.publishes_snapshot() && crate::voxel::active()`, or only for clients that have a mirrored gun guard or a planted flare. MW2 maps and bots play exactly as before.
- Decisions go in `docs/round10/track-e3-log.md`.

### Group `guns-armour` (M): armour wears from every hit (guns-4)

**Gap guns-4.** No player armour ever loses durability, neither from guns nor from Minecraft hits.
- `docs/WEAPONS.md:609` lists this under "Not covered", because the fork had no player armour wear at all.
- Decision 14 (`docs/WEAPONS.md:779`, "no durability") is about the guns themselves, not about armour.

**Cause**
- **Minecraft hits.**
  - `Survival::hurt` (`crates/minecraft_survival/src/survival.rs:806-845`; armour bypass and `hurt_protected` at `:829-830`) and `Survival::hurt_aloud` (`:740-775`; at `:757-769`) both pass the armour to the vendored `SurvivalStatus::hurt_protected` (`third_party/minecraftoss/player/src/survival.rs:312-336`).
  - Its `actually_hurt` (`:341-346`) only lowers the damage. Neither function sees the inventory.
  - The worn armour comes from `survival::armor` (`survival.rs:1324-1337`). Mob, player and explosion hits get it in `minecraft_players::struck` (`crates/render_anim/src/minecraft_players.rs:790`, `:861`); hazards get it in `movement` (`:537-541`, `:562`).
- **Gun hits** are decided in the sim.
  - `player_damage` (`crates/sim/src/script/host/players.rs:25-82`) asks `gun_verdict` (`:84-104`).
  - `gun_verdict` runs `gun_damage::verdict` (`crates/sim/src/gun_damage.rs:112-136`) on the mirrored `GunGuard` (`crates/sim/src/voxel.rs:493-522`), which `crates/render_anim/src/minecraft_fork_items.rs:70-87` writes each frame.
  - The guard holds only numbers. The host learns of the hit only as lost MW2 health, in `Survival::observe` (`survival.rs:408-425`). Nothing tells it that a hit reached armour.
- **Wear that already exists:**
  - tools;
  - elytra flight (`crates/minecraft_survival/src/elytra.rs:124`);
  - pvp weapon wear (`pvp.rs:231`);
  - shield blocks (`minecraft_players.rs:812-830`);
  - `durability::wear` (`crates/minecraft_terrain/src/durability.rs:8-24`, which uses the tool rate of Unbreaking).

**Vanilla 26.3**
- **`LivingEntity.getDamageAfterArmorAbsorb(source, damage)`** calls `hurtArmor(source, damage)` first, unless the source is in `#minecraft:bypasses_armor`.
  - The damage it passes is the damage before armour. Because this runs inside `actuallyHurt`, it is after a shield and after the cooldown difference.
  - The 26.3 tag holds `on_fire`, `in_wall`, `cramming`, `drown`, `fly_into_wall`, `generic`, `wither`, `dragon_breath`, `starve`, `fall`, `ender_pearl`, `freeze`, `stalagmite`, `magic`, `indirect_magic`, `out_of_world`, `generic_kill`, `sonic_boom` and `outside_border`.
- **`Player.hurtArmor`** calls `LivingEntity.doHurtEquipment(source, damage, FEET, LEGS, CHEST, HEAD)`.
  - Nothing happens when `damage <= 0`. Otherwise `amount = (int) Math.max(1, damage / 4)`.
  - Each slot whose stack has an `equippable` with `damageOnHurt()`, `isDamageableItem()` and `canBeHurtBy(source)` gets `ItemStack.hurtAndBreak(amount, entity, slot)`.
- **Which pieces are skipped.**
  - `Items.ELYTRA` is the only item built with `Equippable.Builder.setDamageOnHurt(false)`.
  - `canBeHurtBy` checks the `damage_resistant` component. So netherite (`#is_fire`) takes no wear from fire, lava, campfires, magma or fireballs.
- **`ItemStack.hurtAndBreak(int, ServerLevel, ServerPlayer, Consumer)`** calls `processDurabilityChange`.
  - A player with `hasInfiniteMaterials()` (creative) takes no wear.
  - Otherwise `EnchantmentHelper.processDurabilityChange` applies Unbreaking. In `data/minecraft/enchantment/unbreaking.json` this is a `remove_binomial` with chance (2 + 2(L-1)) / (10 + 5(L-1)) = 2L/(5L+5) for `#enchantable/armor`, and L/(L+1) for everything else (the rate `durability::wear` already uses).
- **A piece that breaks:** `LivingEntity.breakItem` plays the stack's break sound (default `entity.item.break`) at volume 0.8 and pitch 0.8 + 0.4 × random, and spawns 5 item particles.

**Plan**

Commit 1: **"Minecraft armour wears down when its wearer is hurt like vanilla"**
- **Vendored `player/src/survival.rs`:** add `pub fn armor_damage(&self, damage: f32) -> Option<f32>` next to `hurt_protected` (`:312`).
  - It returns the amount `hurt_protected` would pass to `actually_hurt`: `None` when the player is dead or when a hit inside the cooldown is not stronger than `last_hurt`; the excess over `last_hurt` inside the cooldown; otherwise the damage itself.
  - It is a pure query. Count the file as patched in the log.
- **`crates/minecraft_terrain/src/durability.rs`:** add `pub fn wear_armor(inventory: &mut Inventory, damage: f32, damage_type: &str, roll: impl FnMut() -> f32) -> Vec<String>` next to `wear`.
  - For `damage > 0`, `amount = max(1, (damage / 4.0) as u32)`.
  - It walks slots 36, 37, 38 and 39 (feet, legs, chest, head, in vanilla's order).
  - It skips an empty slot, a stack without durability (`recipes.durability`), the elytra, and any stack whose `minecraft:equippable` component sets `damage_on_hurt` to false.
  - It also skips a stack that resists the damage type, by the same rule that `minecraftoss_world::level::burning::resists` (`third_party/minecraftoss/world/src/level/burning.rs:145`) applies to dropped items.
  - Unbreaking spares each point with chance 2L/(5L+5); put its constants next to `UNBREAKING`. Then it calls `inventory.wear_tool(slot, lost)`.
  - It returns the ids of the pieces that broke.
- **`crates/minecraft_survival/src/survival.rs`:**
  - Add a field `armor_hits: Vec<(f32, &'static str)>` next to `hurts` (`:165`), not at the end of the struct, where M2 adds its hooks.
  - In `hurt` and `hurt_aloud`, after the armour bypass and only when the armour has a value: read `self.status.armor_damage(damage)` before `hurt_protected`. If the hit landed, push that amount with the cause's damage type.
  - Add `pub fn take_armor_hits(&mut self) -> Vec<(f32, &'static str)>`.
- **`crates/minecraft_survival/src/entities.rs`:** add `pub fn wear_armor(&mut self, inventory: &mut Inventory, damage: f32, damage_type: &str, infinite: bool) -> Vec<Sound>` next to `spend_boost` (`:1287`).
  - When `infinite` is set, it does nothing.
  - Otherwise it calls `durability::wear_armor` with `self.view.random`.
  - It pushes each broken id to `broken_items`, which shows the crumbs at the eye (`minecraft_world.rs:1258`, `minecraft_players.rs:1890-1897`).
  - It returns one `flint::break_sound` per broken piece. That sound's 0.8 volume and 0.8 + 0.4 pitch are already vanilla's.
- **`crates/render_anim/src/minecraft_players.rs` `struck` (`:753`):**
  - After the hits loop (`:863`), drain `survival.take_armor_hits()` through `entities.wear_armor(inventory, damage, kind, !player_mode.consumes_items())`.
  - Play the sounds the same way the boost's sounds are played (`:882-893`).
  - `struck` runs after `movement` for the host (`minecraft_world.rs:1253`) and for each friend (`minecraft_players.rs:1755`), so the frame's hazard hits are worn here too.

Commit 2: **"MW2 bullets, blasts and knives wear Minecraft armour"**
- **`crates/sim/src/gun_damage.rs`:** add `pub fn armor_damage(amount: i32, max_health: i32) -> f32`, the halved MW2 amount on the 20-point scale. `gun_damage_to_player` uses it for its `damage` (`:73`), so the formula exists in one place.
- **`crates/sim/src/voxel.rs`:**
  - Add `static ARMOR_HITS: Mutex<Vec<(u32, f32, GunHit)>>` next to `GUN_GUARDS` (`:503`).
  - Add `pub(crate) fn push_armor_hit(client, damage, kind)`, which only records for a client that has a guard, and `pub fn take_armor_hits(client: u32) -> Vec<(f32, GunHit)>`.
  - `forget_gun_guard` (`:514`) and `deactivate` (next to `guards.clear()`, `:936`) both clear the queue.
- **`crates/sim/src/script/host/players.rs`:**
  - `gun_verdict` also returns the pre-armour damage and the hit kind for a hit it scales; it already reads `max_health`.
  - `player_damage` reads the victim's MW2 health before `run_now` (`:76`). If the run succeeded and the health fell, it pushes the armour hit.
  - A hit refused by the totem, or refused by the damage callback, wears nothing.
- **`minecraft_players.rs` `struck`:** also drain `sim::voxel::take_armor_hits(client.0)`. Use the damage types `GUN_BULLET_DAMAGE`, `GUN_BLAST_DAMAGE` and `GUN_MELEE_DAMAGE` (`survival.rs:1336-1338`) for `Bullet`, `Explosion` and `Melee`.

**Tests**
- Vendored `player::survival::tests::armor_damage_is_the_hit_or_its_excess_inside_the_cooldown`.
- `durability::tests`:
  - `armour_loses_a_quarter_of_the_hit_and_at_least_one_point_per_piece`: 3 gives 1 per piece, 8 gives 2, 13.9 gives 3, 0 gives none, and an empty slot is skipped.
  - `the_elytra_and_an_unbreakable_piece_take_no_wear`.
  - `netherite_takes_no_wear_from_fire_but_from_a_bullet`: `minecraft:campfire` and `minecraft:lava` against `minecraft:arrow`.
  - `unbreaking_spares_armour_at_two_fifths_of_the_tool_rate`: with fixed rolls, Unbreaking III spares armour below 0.3 and a tool below 0.75.
  - `a_worn_out_piece_is_removed_and_named`.
- `survival::tests`:
  - `a_landed_mob_hit_queues_its_damage_before_armour`.
  - `bypassing_and_unarmoured_hits_queue_nothing`: magic, `Armor::default()`, an ender pearl.
  - `a_hit_inside_the_cooldown_queues_only_its_excess`.
  - `lava_and_cactus_queue_wear_and_burning_does_not`.
- `entities::tests::worn_out_armour_breaks_with_crumbs_and_a_sound_and_infinite_wears_nothing`.
- Sim `gun_damage::tests::armour_wear_takes_the_halved_mw2_amount_on_the_twenty_point_scale`: 30 at max health 100 gives 3, 135 gives 13.5, and 40 at max health 200 gives 2.
- Sim `voxel::tests::armour_hits_queue_per_guarded_client_and_go_with_the_guard`.
- Sim `damage::height_bands::a_gun_hit_on_a_guarded_player_queues_armour_wear_at_the_height_of_every_band`, using the band harness `crates/sim/src/damage/height_bands.rs` (`shoot` at `:175`, `frag` at `:194`):
  - a bullet and a frag on a guarded B queue one hit each, with the pre-armour damage;
  - an unguarded B queues nothing;
  - a hit inside the totem grace queues nothing.

**Owner's steps** (survival, Normal, by day, flat grass; run `/gamerule keep_inventory true` first). The kit's `+damage` pieces have 2 points left and show an almost empty red bar.
- **A-1.** Put on the four damaged leather pieces. Use a zombie egg 4 blocks away and let the zombie hit you without blocking.
  - After the first hit, all four pieces are still on.
  - After the second hit, all four break at once: you hear the break sounds, see leather crumbs in front of you, and the armour row above the hotbar is empty.
  - Kill the zombie with the sword. **Screenshot E3-G1** (the inventory with empty armour slots).
- **A-2.** Put on the four undamaged iron pieces. Let a zombie hit you three times, then kill it. Open the inventory: each piece shows a full-length green durability bar (3 points used). **Screenshot E3-G2.**
- **A-3.** Take the iron off. Put on the damaged golden helmet and the damaged netherite boots. Eat a golden apple, place the campfire, stand on it for two seconds and step off.
  - The golden helmet broke.
  - The netherite boots are still on, because netherite takes no wear from fire. **Screenshot E3-G3.**
- **A-4.** Take the boots off and put on the damaged elytra. Let a zombie hit you three times: the elytra stays, because vanilla never wears an elytra from hits. Kill the zombie.
- **A-5. Two windows** (`& scripts\mp-local.ps1 -Exe iw4l-survival.exe -World e3-armour -Fresh`, the kit in both).
  - H puts on the four damaged iron pieces.
  - F shoots H once with the M4 from 5 blocks: H's pieces stay on.
  - F shoots again: all four break in H's window, with the break sound. **Screenshot E3-G4** (H's inventory).
- **A-6.** H puts the undamaged iron back on.
  - F knifes H once with MW2's melee key: each of H's bars gets a little shorter. A knife wears 3 points, because MW2's 135 is 13.5 Minecraft damage before armour.
  - F throws a frag at H's feet: the bars shrink a little more.
- **A-7.** F puts on F's damaged leather, and a zombie hits F twice: F's pieces break in F's window, with the sound.

```
IW4L_MINECRAFT_KIT=minecraft:leather_helmet+damage=53,minecraft:leather_chestplate+damage=78,minecraft:leather_leggings+damage=73,minecraft:leather_boots+damage=63,minecraft:iron_helmet+damage=163,minecraft:iron_chestplate+damage=238,minecraft:iron_leggings+damage=223,minecraft:iron_boots+damage=193,minecraft:iron_helmet,minecraft:iron_chestplate,minecraft:iron_leggings,minecraft:iron_boots,minecraft:golden_helmet+damage=75,minecraft:netherite_boots+damage=479,minecraft:elytra+damage=430,minecraft:zombie_spawn_egg*8,minecraft:campfire,minecraft:diamond_sword,iw4l:m4,iw4l:rifle_ammo*64,iw4l:frag_grenade*16,minecraft:golden_apple*16,minecraft:cooked_beef*32,minecraft:cobblestone*64
```

24 stacks. The maximum durability from the 26.3 item catalog: leather 55, 80, 75 and 65; iron 165, 240, 225 and 195; golden helmet 77; netherite boots 481; elytra 432.

### Group `guns-targets` (M): bullets break lead knots, the knife stabs mobs (guns-16, guns-15, the guns-14 check)

**Gaps**
- **guns-16.** MW2 bullets pass through lead knots. `docs/round10/track-e2-log.md:511` left this for later; w3 owns only the knots' game events.
- **guns-15.** MW2's knife passes through mobs and hits the block behind them. `docs/ELDEN-RING-BOSSES.md:541` and `:562` put this with the later bosses, but it needs no Elden Ring data and covers every mob.
- **guns-14.** MW2 bullet impacts, blood and explosions in the Nether were never checked; this was round 9's optional A1-9 (`docs/round9/release-1-tests.md:141`). It is a test step only (T-10), with code only if the step fails.

**Cause, knots**
- **Bullets find targets only in `EntityView::shot_boxes`** (`crates/minecraft_survival/src/entity_view.rs:846-890`: mobs, fireballs, shulker bullets, dragon parts and item frames).
  - These boxes go to the sim each frame through `sim::voxel::set_mob_boxes`: for the host at `crates/render_anim/src/minecraft_world.rs:1505`, for other levels at `crates/render_anim/src/minecraft_levels.rs:537`.
  - Knots are not among them.
- **The punch already exists.**
  - `Entities::knot_target` and `punch_knot` (`crates/minecraft_survival/src/entities.rs:993-1008`) send `server.knot_action(player, pos, KnotAction::Punch)` (`crates/minecraft_terrain/src/server.rs:4844`, run at `:1863-1870`).
  - The knot box is `leash_render::knot_box` (`crates/minecraft_terrain/src/leash_render.rs:108`). The knots come from `view.world.knots()` (vendored `entities/src/world/leashes.rs:375`).
- **How hit keys are routed.**
  - `Entities::shoot` (`entities.rs:1532-1573`) and `Entities::blast` (`:1575`) route by the key's top byte: frames (`frame_of`, `:2309`), End targets, Margit, shulker bullets, fireballs, and then mobs.
  - Bytes in use: 0 to 23, 26 to 35, and the registry's 36 to 99 (decisions.md, "D0's shape"). Bytes 24 and 25 are free.
  - Bit 55 is `FLAT_HIT` (`entities.rs:2260`, `voxel.rs:257`).

**Cause, knife**
- **`fire_weapon_melee`** (`crates/sim/src/combat.rs:1500-1680`) traces only the world, glass and MW2 players. It never asks `voxel::mob_on_segment` (`crates/sim/src/voxel.rs:219`).
  - So a stab into a mob carries on to the block behind it (`push_shot` at `combat.rs:1596-1601`).
  - A stab at a mob with nothing solid behind it within reach finds no collider (`:1569-1571`) and does nothing.
- **Bullets do ask:** `combat.rs:1244-1263` calls `mob_on_segment` and then `push_mob_shot` (`voxel.rs:313`).
- **The host's side.** The host turns a `MobShot` into `Entities::shoot` (`minecraft_world.rs:1003-1008`, `minecraft_levels.rs:291-303`), which builds a `PlayerAttack` (`entities.rs:1559-1573`) with the default 0.4 knockback away from the shooter (vendored `entities/src/world/combat.rs:329-338`).
  - For a knife that is right for ordinary mobs.
  - But Margit would get `elden_boss::Source::Bullet` (`entities.rs:1541-1543`), and frames would get `FrameAction::Shot`.

**Cause, the Nether check (guns-14)**
- **No Nether-only gate in the code:**
  - MW2 FX are culled only by frustum and distance (`crates/render_frontend/src/adapters/fx/system.rs:1233-1240`, `:2764-2768`).
  - MW2 scene entities link into every cell in Minecraft worlds (`crates/render_frontend/src/adapters/anim/dyn_ent_brush.rs:356`, since 76c3565a).
  - Voxel hits use `STONE_SURFACE` in every level (`voxel.rs:19`).
- **The one suspect** (`docs/round9/track-a-log.md:25`): world marks build their `DpvsPlanes` from the stand-in map `mp_rust` (`crates/render_frontend/src/adapters/fx/world_mark.rs:193-229`), and the Nether band lies below that map (`track-a-log.md:20`).

**Vanilla 26.3**
- **Knots.**
  - `BlockAttachedEntity.isPickable` is true, and `Entity.canBeHitByProjectile` is `isAlive() && isPickable()`, so an arrow hits a knot.
  - `BlockAttachedEntity.hurtServer` kills the knot on any landed damage. It refuses only when the source is a `Mob` and `mob_griefing` is off.
  - `LeashFenceKnotEntity.dropItem` plays `LEAD_UNTIED`, and the tied mobs drop their leads.
  - `BlockAttachedEntity.ignoreExplosion` lets an explosion that `shouldAffectBlocklikeEntities` kill a knot too.
- **Knife.**
  - `Player.attack(Entity)` deals `player_attack` damage, and `LivingEntity.hurtServer` knocks the target back 0.4 away from the attacker.
  - Sprint knockback, criticals and the sweep need sprinting, falling or a sword, so the knife gets none of them.
  - `ItemFrame.hurtServer` from a player attack drops the framed item first, and the frame itself on the next hit.

**Plan**

Commit 1: **"MW2 bullets and blasts break lead knots"**
- **`entities.rs`:**
  - Add `pub(crate) const KNOT_KEY: u64 = 24 << 56;` next to `FRAME_KEY` (`:2259`).
  - Add `pub(crate) fn knot_key(pos: BlockPos) -> u64`. It packs the low 22 bits of x and z and the low 11 bits of y below bit 55, so `FLAT_HIT` stays clear.
  - Add `fn knot_of(key: u64, knots: &[BlockPos]) -> Option<BlockPos>` next to `frame_of` (`:2309`). It returns the knot whose key matches.
- **`entity_view.rs` `shot_boxes`:** after the frames (`:883-890`), add one box per `self.world.knots()` from `knot_box`, keyed `knot_key(pos)`.
- **`Entities::shoot` and `Entities::blast`:** a knot key sends `KnotAction::Punch`, checked before the frames.
  - `blast` uses `by_player.unwrap_or(u64::MAX)`, as it does for frames.
  - A knot that is already gone does nothing, and the fence stays.
  - The bullet stops at the knot, the same as at a mob (`combat.rs:1250-1262`).

Commit 2: **"MW2's knife stabs Minecraft mobs, Margit, item frames and lead knots"**
- **`voxel.rs`:**
  - Add `VoxelEvent::MobMelee { level, key, damage, from, attacker }` right after `MobShot` (`:172`).
  - Add `Level::mob_melee` next to `mob_shot` (`:339`).
  - Add `pub(crate) fn push_mob_melee` next to `push_mob_shot` (`:313`).
- **`combat.rs` `fire_weapon_melee`:**
  - When `world.publishes_snapshot() && crate::voxel::active()`, each melee trace (`:1533`) also asks `mob_on_segment(origin, stop)`. Here `stop` is the end of the trace's first world segment, or the end of the trace.
  - Ask before the "no collider" `continue` (`:1569`). Keep the nearest mob as a fraction of the trace, together with its point.
  - After the loop (`:1592`), when that mob is nearer than `best_hit`, or there is no `best_hit`:
    1. Draw `amount` the same way as now (`:1595`).
    2. Call `push_mob_melee(key, amount as f32, origin, Some(attacker.0))`.
    3. Push `MELEE_HIT` at the mob's point, with `other_entity_num` set to `ENTITYNUM_WORLD`. This plays the knife-in-body sound (`crates/audio/src/entity_events.rs:75`). Push no `MELEE_BLOOD`.
    4. Return, so there is no `push_shot` and no player damage.
  - A player who is nearer than the mob still takes the stab.
- **`entities.rs`:** add `pub fn stab(&mut self, player: u64, key: u64, damage: f32, from: [f64; 3], yaw: f32, creative: bool)` next to `shoot`.
  - A frame gets `FrameAction::Attack { infinite: creative }`, which is a punch.
  - A knot gets `KnotAction::Punch`.
  - Margit gets the `PlayerAttack` path. The registry turns that into `Source::Melee` (vendored `entities/src/world/families/bosses/elden_bosses.rs:195-217`).
  - End targets, shulker bullets, fireballs and other mobs get what `shoot` gives them.
  - Move `shoot`'s `PlayerAttack` tail (`:1559-1573`) into a private helper that both functions call.
  - The damage is `damage * HEALTH_SCALE`, the same as a bullet's: MW2's 135 to 139 becomes 27 to 27.8.
- **Host `minecraft_world.rs`:**
  - the partition (`:974-976`);
  - a `MobMelee` arm right after `MobShot` (`:1003-1008`) that calls `entities.stab` with the same credit and yaw;
  - the sound arm (`:1041`);
  - `event_level` (`:2168-2176`).
- **Other levels, `minecraft_levels.rs`:** the partition (`:259-264`) and an arm after `MobShot` (`:291-303`).
- **`minecraft_mining.rs:124`:** the no-op arm.
- **`gun_vibrations`** (`:2185-2196`) keeps its `_ => {}`: the hurt mob posts its own game event.

**Tests**
- `entities::tests::a_knot_key_keeps_the_flat_hit_bit_clear_and_finds_its_knot`: negative x and z, y at -64 and 319, and two knots 1 block apart.
- `entity_view::tests::lead_knots_are_shot_boxes_with_the_vanilla_knot_box`.
- `entities::tests::a_bullet_or_a_blast_on_a_knot_frees_its_mobs_and_leaves_the_fence`: uses the server and is gated on the game data, like E2's knot tests. Pattern: `server.rs` `punching_a_knot_drops_a_lead_for_each_tied_cow_and_leaves_the_fence`.
- `entities::tests::a_stab_on_a_frame_is_a_punch_and_on_margit_a_melee_hit`.
- Sim `voxel::tests::a_voxel_shot_carries_its_attacker_and_every_event_its_level` (`voxel.rs:2210`): add a `mob_melee` event and check its level.
- Sim `damage::height_bands::a_knife_stabs_a_mob_before_the_block_behind_it_at_the_height_of_every_band`:
  - a mob box one block in front of A gives one `MobMelee`, with A as the attacker and 135 to 139 damage, and no `Shot`;
  - without the box, the wall gets its `Shot` as before;
  - with B standing between A and the box, B is stabbed.
- `render_anim` `minecraft_world::tests` (`:2797-2830`): `event_level` covers `MobMelee`.

**Owner's steps** (survival, Normal, by day, flat grass; run `/gamerule keep_inventory true` first; stab with MW2's melee key while holding the M4)
- **T-1.** Put a lead on a cow and right-click an oak fence: a knot appears on the post. Stand 6 blocks off to the side, aim the M4 at the knot (the upper half of the post) and fire once.
  - The knot vanishes with the untie sound.
  - A lead drops by the cow, and the cow is free.
  - The fence is still there. **Screenshot E3-G5.**
- **T-2.** Tie the cow again and fire at the bottom third of the post: the bullet hits the fence, and the knot stays.
- **T-3.** `/gamemode creative`. Tie the cow and shoot the knot: the knot goes and a lead drops. Then `/gamemode survival`.
- **T-4.** Stand right in front of a cow, closer than 1.5 blocks, and stab it.
  - It dies at the first stab and drops beef: a stab does 27 and a cow has 10 health.
  - The block behind it shows no hit crack. **Screenshot E3-G6.**
- **T-5.** Eat a golden apple. Use the polar bear egg, walk up and stab the bear: it flashes red, slides back a little and turns on you. The second stab kills it (it has 30 health).
- **T-6.** Hang an item frame on a wall and put an apple in it. Stab it: the apple pops out. Stab it again: the frame drops.
- **T-7.** Tie a cow to the fence and stab the knot from the side: the knot goes, as with a punch.
- **T-8.** Run `/summon iw4l:margit ~ ~ ~6`. When he is next to you, stab him: his boss bar drops by about a thirtieth (27 of 800), counted as a melee hit like a sword. `/difficulty peaceful` removes him; then run `/difficulty normal`.
- **T-9.** Build a Nether portal from the obsidian and go through.
- **T-10. MW2 effects in the Nether** (round 9's A1-9). In the Nether:
  - shoot netherrack from 5 blocks;
  - throw a frag at a wall;
  - stab a piglin from its egg.
  The bullet impact puffs, the frag's fire and smoke, and the stab should look and sound as they do in the Overworld. **Screenshot E3-G7.** If anything is missing only in the Nether, take a screenshot in each world.
- **T-11. Two windows** (`& scripts\mp-local.ps1 -Exe iw4l-survival.exe -World e3-targets -Fresh`, the kit in both).
  - F stabs a cow next to H: it dies in both windows.
  - H ties a cow to a fence, and F shoots the knot: it goes in both windows and the lead drops. **Screenshot E3-G8** (F's window).

```
IW4L_MINECRAFT_KIT=iw4l:m4,iw4l:rifle_ammo*64,iw4l:frag_grenade*16,minecraft:lead*8,minecraft:cow_spawn_egg*8,minecraft:polar_bear_spawn_egg*2,minecraft:piglin_spawn_egg*2,minecraft:oak_fence*16,minecraft:item_frame*4,minecraft:apple*4,minecraft:obsidian*10,minecraft:flint_and_steel,minecraft:golden_apple*16,minecraft:cooked_beef*32,minecraft:cobblestone*64
```

15 stacks.

### Group `guns-flare` (L): Minecraft explosions destroy a tactical insertion, and it shows MW2's glowstick (guns-13, guns-11)

**Gaps**
- **guns-13.** Minecraft explosions leave a planted tactical insertion standing: creepers, TNT, end crystals, beds and anchors, and ghast and wither fireballs (`docs/round9/track-c-log.md:1101`).
- **guns-11.** Nothing marks a planted insertion.
  - The thrown `flare_mp` is the only flare that is ever drawn ("A glowstick model is left for later", `track-c-log.md:1105`).
  - The sim keeps its copy of the flare after the host has used it (`:1104`).

**Cause**
- **Only the sim's MW2 blast path destroys a flare.**
  - `push_voxel_blasts` (`crates/sim/src/damage.rs:281-284`) calls `voxel::blast_flares` (`crates/sim/src/voxel.rs:694-710`; 20 damage, `FLARE_DESTROYING_DAMAGE` at `:582`), which queues `Spent::FlareGone`.
  - The host's `settle_arsenal` then clears the flare in `guns::Binding` (`crates/minecraft_survival/src/guns.rs:463-464`).
- **Minecraft explosions run in the level server:**
  - `explode_creepers` (`crates/minecraft_terrain/src/server.rs:2028-2035`);
  - `explode_fireballs` (`:2045-2057`);
  - `explode_blasts` (`:2190-2214`), which covers crystals (`:3481-3486`) and beds and anchors;
  - `explode_tnt` (`:2216-2231`).
  Each one reaches the host in `Output.explosions` (`server.rs:4473`), which `Entities` only plays as a sound (`crates/minecraft_survival/src/entities.rs:1814-1817`). MW2's own blasts (`weapon_blast`, `server.rs:2233-2238`) are not in that list, so nothing is counted twice.
- **Planting records only a position:** `combat::plant_flare` (`crates/sim/src/combat.rs:1824-1840`) calls `voxel::plant_flare` (`voxel.rs:675-686`, `FLARES` at `:579`). MW2's glowstick comes from the GSC function `monitorTIUse` in `maps/mp/perks/_perkfunctions.gsc`, which never runs here (`track-c-log.md:1081`).
- **Using the flare.** The host uses it in `respawn_at_flare` (`crates/minecraft_survival/src/respawn.rs:117-134`), through `Binding::take_flare` (`guns.rs:238-240`). `voxel::forget_flare` (`voxel.rs:688`) is crate-private and runs only from `forget_arsenal`, on disconnect.

**Vanilla 26.3 and MW2**
- **`ServerExplosion.hurtEntities`** reaches every entity within twice the radius.
  - `ExplosionDamageCalculator.getEntityDamageAmount`: `d = sqrt(entity.distanceToSqr(center)) / (2r)`, `e = (1 - d) × seen`, and the damage is `(e² + e) / 2 × 7 × 2r + 1`.
  - `seen` is `ServerExplosion.getSeenPercent(center, entity)` over the entity's box.
- **The rule for the flare.** The flare is not a vanilla entity. E3 keeps MW2's rule (20 damage of MW2's 100 health, which is 4 of Minecraft's 20) and measures the damage with vanilla's formula.
- **Wind charges and firework bursts do not count.** Neither is in `Output.explosions`: vanilla's wind charge explosion damages no entity, and a firework burst hurts only living entities.
- **MW2 (`_perkfunctions.gsc`, `spawnGlowStick`):**
  - It spawns a `script_model` on the ground below `TISpawnPosition` and calls `setModel(level.spawnGlowModel[...])`.
  - `GlowStickTeamUpdater` spawns and triggers the glow effect `level.spawnGlow[...]` (`spawnFx`, `triggerFx`).
  - Both go when the insertion is used or destroyed.
  - The names are set in the init of `maps/mp/perks/_perks.gsc` in the owner's `common_mp.ff`. Read them the way track C read `_perkfunctions.gsc`. Expected, but not verified: `weapon_light_stick_tactical_green` and `misc/glow_stick_glow_green` (red versions for enemies).
- **Nearest vanilla marker:** the respawn anchor's glow (`RespawnAnchorBlock`).

**Plan**

Commit 1: **"Minecraft explosions destroy a planted tactical insertion"**
- **`entities.rs`:** add `pub level_blasts: Vec<(DVec3, f32)>` next to `broken_items` (`:68`; set up at `:517`). Push one entry per `output.explosions` blast (`:1814`).
- **`respawn.rs`:** add `pub fn flare_blast_damage(world: &impl minecraftoss_player::World, center: DVec3, radius: f32, feet: [f64; 3]) -> f32` next to `flare_stands` (`:109`).
  - It uses vanilla's formula, with `minecraftoss_entities::sight::seen_percent` (`third_party/minecraftoss/entities/src/sight.rs:163`) over a quarter-block cube resting on the flare's feet.
  - It returns 0 beyond twice the radius.
  - `ender_dragon::blast_damage` (`crates/minecraft_terrain/src/ender_dragon.rs:774-785`) has the same formula without exposure.
- **`voxel.rs`:** `blast_flares` (`:694`) becomes `pub`.
- **`crates/render_anim/src/minecraft_fork_items.rs`:** add `pub(crate) fn blast_flares(dimension, origin, scene, blasts: Vec<(DVec3, f32)>)` next to `flare_place` (`:557`). It makes one `sim::voxel::blast_flares` call per blast.
  - Its damage closure returns 0 for a position outside the level's band (`bands::holds`, as `flare_place` uses).
  - Otherwise it returns `flare_blast_damage` at `sim::voxel::to_block(origin, pos)`, times 5 to convert Minecraft's 20 health to MW2's 100.
  - The `FlareGone` it queues reaches the binding through `settle_arsenal`, as before.
- **Call sites:** call it with `std::mem::take(&mut entities.level_blasts)`:
  - for the host's level, after the MW2 blast loop (`minecraft_world.rs:1015-1019`);
  - for every other ticked level, after its blast loop (`minecraft_levels.rs:320-328`).

Commit 2: **"The sim forgets a tactical insertion once its player respawns at it"**
- **`guns.rs` `Binding` (`:226-231`):** add `flare_used: bool`, set by `take_flare` when it returns a flare, and `pub fn take_flare_used(&mut self) -> bool`.
- **`voxel.rs`:** `forget_flare` (`:688`) becomes `pub`.
- **After each settle** (host `minecraft_world.rs:1308-1310`, friend `minecraft_players.rs:1933-1935`): `if arsenal.take_flare_used() { sim::voxel::forget_flare(client) }`.
  - A later MW2 blast at the old spot then no longer queues a stale `FlareGone`.
  - Commit 3's glowstick disappears together with the flare.

Commit 3, after a check: **"A planted tactical insertion shows MW2's glowstick until it is used or destroyed"**
- **Check first.**
  - Read the model and FX names from the owner's fastfile.
  - Check whether a `script_model` with that model is drawn in a Minecraft world. The thrown `flare_mp` is drawn here, so its world model is the fallback.
  - If neither model draws, stop after commit 2 and write the finding in the log. The group is then done.
- **`voxel.rs`:** add `pub(crate) fn flares() -> Vec<(u32, [f32; 3])>` next to `forget_flare`.
- **New file `crates/sim/src/script/host/glowsticks.rs`:** a `Glowsticks` resource (client, spot, stick id, glow id) and `pub(crate) fn sync_glowsticks(world: &mut World)`.
  - It does nothing unless `voxel::active()` is true and the world publishes snapshots.
  - For a flare with no stick at its spot:
    1. Delete that client's old stick and glow.
    2. Spawn a `script_model` at the spot and set its model, as the `spawn` and `setmodel` natives do (`natives/engine.rs:825`, `:978`).
    3. Spawn a looping glow effect entity, as `spawnfx` and `triggerfx` do (`:1860-1914`). `entity_damage.rs:314-342` shows how to do this from Rust.
  - A stick whose flare is gone (used, blasted, replaced, or its player left) is deleted with its glow (`Runtime::delete_entity`, `host/entities.rs:382`).
- **Wiring:**
  - `host/mod.rs`: add `pub mod glowsticks;` after `pub mod entity_damage;`.
  - `script/mod.rs`: add `pub(crate) use host::glowsticks::sync_glowsticks;` next to `sync_engine_events` (`:39`).
  - The `step.rs` schedule: add `crate::script::sync_glowsticks` right after `crate::script::sync_engine_events` (`:53`).
- **Colour.** Every viewer sees one green stick and glow. Minecraft worlds are co-op, and MW2's red for enemies would need per-viewer hiding. Friends get the stick through MW2's snapshot, like any other script model and effect.

**Tests**
- `respawn::tests::a_flare_takes_vanilla_blast_damage_and_a_wall_shields_it`, with TNT (radius 4):
  - 3 blocks away in the open gives about 29.4;
  - 7.5 blocks away gives less than 4;
  - 9 blocks away gives nothing;
  - behind a solid wall gives 1.
- `render_anim` `minecraft_fork_items::tests::a_minecraft_blast_reaches_only_flares_in_its_own_band`: 0 for a position in another band, and five times the Minecraft damage in its own band. Split the closure into a pure `flare_damage_at` if that helps.
- `guns::tests::a_used_flare_is_reported_once`: `take_flare` sets the flag, while `FlareGone` and a new plant do not.
- Sim `voxel::tests` for `flares()` and the now public `forget_flare`.
- Sim `script::host::glowsticks::tests::a_planted_flare_has_one_glowstick_that_follows_a_replant_and_goes_with_the_flare`, set up like `item::arsenal_tests` (`crates/sim/src/item/arsenal_tests.rs:755-767`):
  - a plant gives one stick with the model and one glow at the spot;
  - a second plant moves them;
  - `forget_flare` and a blast remove them;
  - another client's stick stays.

**Owner's steps** (survival, by day, flat grass; run `/gamerule keep_inventory true` first; `/tp @s ~ ~ ~` shows the coordinates, since the F3 screen comes only with debug-screen in e3r)
- **F-1.** Place the bed and right-click it ("Respawn point set"). Walk 40 blocks away.
  - Open the inventory, move the frags from the lethal slot into the backpack, and shift-click the tactical insertions into the lethal slot.
  - Press G: the insertion is planted where you stand. Run `/tp @s ~ ~ ~` and note X and Z.
- **F-2. TNT in reach.** Place TNT 3 blocks from the spot, light it and run 10 blocks away. After the blast, `/kill`: you respawn at the bed. **Screenshot E3-G9.**
- **F-3. Out of reach.** Plant again. Place TNT 9 blocks from the spot, light it and stand clear. After the blast, `/kill`: you respawn at the insertion.
- **F-4. A wall.** Plant again. Build an obsidian wall 3 wide and 3 high, one block from the spot, and place TNT 3 blocks from the spot behind the wall. After the blast, `/kill`: you respawn at the insertion, because the wall shields it, like vanilla's exposure.
- **F-5. An end crystal.** Plant again. Put obsidian 4 blocks from the spot with the end crystal on it, walk 20 blocks away and shoot the crystal. After its blast, `/kill`: you respawn at the bed.
- **F-6. The glowstick.** Plant one: a small green glowing stick lies at the spot. Walk 20 blocks away: it still glows. **Screenshot E3-G10.**
- **F-7.** Plant another one 10 blocks further on: the first stick vanishes and the new one lies at your feet. `/kill`: you respawn at the stick, and it vanishes.
- **F-8.** Plant one and throw a frag onto it: it vanishes with the blast. Plant another and blow up TNT 3 blocks away: it vanishes too.
- **F-9.** Plant one, then save and quit, and load. There is no stick, and `/kill` takes you to the bed (insertions are not saved).
- **F-10. Two windows** (`& scripts\mp-local.ps1 -Exe iw4l-survival.exe -World e3-flare -Fresh`, the kit in both).
  - F plants an insertion: H sees F's green stick, and F sees H's. **Screenshot E3-G11** (H's window).
  - H lights TNT 3 blocks from F's spot. After the blast, F's stick is gone in both windows, and F's `/kill` sends F to F's spawn. **Screenshot E3-G12** (F's window).

Steps F-6 to F-10 need commit 3. If commit 3 was dropped, skip them and say so in the log.

```
IW4L_MINECRAFT_KIT=iw4l:tactical_insertion*8,iw4l:tactical_insertion*8,iw4l:frag_grenade*8,iw4l:m4,iw4l:rifle_ammo*64,minecraft:tnt*8,minecraft:flint_and_steel,minecraft:white_bed,minecraft:obsidian*24,minecraft:end_crystal*2,minecraft:golden_apple*8,minecraft:cooked_beef*16,minecraft:cobblestone*64
```

13 stacks.

### Decisions (for the log)

**Armour**
- **Gun armour wear uses its own sim queue, drained in `struck`.** The gap's note suggested `Spent` and `settle_arsenal` instead. `struck` already has the inventory, the RNG, the crumbs and the sounds for the host and every friend, and one drain serves both Minecraft and MW2 hits.
- **A gun hit wears armour only if the victim's MW2 health fell** after the damage callback. A hit refused by the totem or by the callback wears nothing, and neither does a gun hit fully stopped by Resistance V. Vanilla would still wear armour in that last case; note the difference.
- **Armour Unbreaking uses vanilla's armour rate**, 2L/(5L+5). Tools keep L/(L+1).

**Knots and the knife**
- **Knot keys:** byte 24, with x and z in 22 bits each and y in 11 bits, all below `FLAT_HIT`. A key is resolved by matching it against the view's knots, so wrapping at 2 million blocks cannot pick a nearby knot by mistake.
- **MW2 blasts break knots,** as vanilla's explosions do.
- **The knife on a mob:**
  - It sends `MELEE_HIT` (the knife-in-body sound) and no MW2 blood.
  - It uses the bullet's damage scale and vanilla's 0.4 knockback.
  - Margit takes it as a melee hit, and frames take it as a punch.
- **Pad aim assist** (`crates/net/src/client/runtime.rs:836`) sees knot boxes as it already sees item frames. Left as it is.

**The tactical insertion**
- **The blast rule:** MW2's 20 of 100 health is 4 Minecraft damage. It is measured with vanilla's explosion formula and exposure, over a quarter-block cube.
- **Exposure is measured after the blast.** The host sees an explosion after its blocks have broken, so a block the blast itself destroyed no longer shields the flare (vanilla measures before). A wall that survives the blast, such as obsidian, still shields it.
- **The sim forgets a used flare.** This replaces round 9's "the sim keeps its copy" (`track-c-log.md:1104`).
- **The glowstick is green for everyone,** because the game is co-op. MW2's red for enemies is left out.

### Left for later
- Vanilla arrows, tridents and thrown items breaking knots, and Minecraft explosions breaking knots (E2's list, `track-e2-log.md:511`). These are not MW2's.
- MW2's per-team glowstick colours.
- If T-10 shows a gap only in the Nether: the world marks' planes (`world_mark.rs:193-229`).

### Files shared
- **`crates/minecraft_survival/src/entities.rs`:** every track edits it. The new items go next to `spend_boost`, `shoot`, `FRAME_KEY`, `frame_of` and `broken_items`.
- **`crates/minecraft_survival/src/entity_view.rs` `shot_boxes`:** new mob families come in through `boxes()`, not here.
- **`crates/minecraft_survival/src/survival.rs`:** M2 also edits it (breath and the player-effect queue). The new field sits next to `hurts`.
- **Vendored `player/src/survival.rs`:** M2 may add effect hooks. The new method sits next to `hurt_protected`.
- **`crates/render_anim/src/minecraft_players.rs`** (`struck` and the friend settle), **`minecraft_world.rs`** and **`minecraft_levels.rs`:** every track edits them. The new arms sit next to `MobShot`.
- **The sim crate** (`combat.rs`, `voxel.rs`, `gun_damage.rs`, `script/host/players.rs`, `script/mod.rs`, `script/host/mod.rs`, `step.rs`, `damage/height_bands.rs`): no other round 10 track edits it.
- **`guns.rs`, `respawn.rs`, `minecraft_fork_items.rs` and `durability.rs`:** no other round 10 track edits them.

## E3-7. Explosions, withers, the dragon and fireballs

### The owner's findings

> **Explosions:** MW2 blasts (grenades, rockets, C4) still push no player. Blast Protection does not soften a push. TNT set off by a creeper's blast does not name the creeper. An end crystal blown up by a player's TNT does not name that player. A ghast fireball that a player sent back still hurts players on a direct hit with `pvp` off. Primed TNT is not saved. (`docs/round10/release-1-tests.md:395`)
>
> **`/tick freeze`:** [...] Respawn anchor and end crystal blasts wait for the next normal tick. (`:399`)
>
> **Withers:** the sky does not darken under a wither's bar. Skulls and fireballs leave no smoke trail. [...] A skull that kills a player does not heal its wither. Withers fly straight at their goal and stop at a wall until they break it or give up. [...] The blue armour is drawn see-through instead of glowing, so it looks a little darker than vanilla's. (`:400`)
>
> **The dragon and End crystals:** [...] A splash potion of harming or healing does not reach the dragon. A trident the sitting dragon shrugs off is not set on fire by its flames. (`docs/round10/release-6-tests.md`, Known limits; d0b's flying-navigation line there repeats the wither one)

### Shared points for this section

- **Base.** Paths and line numbers are as of `4301058a` (`minecraft-survival` before the e2b merge d565a3d2 and the d0b merge). Every group waits for e2b, which edits `server.rs` (crystals and the dragon hit by thrown things, bells, rockets), `end_crystal.rs`, `ender_dragon.rs`, `entity_view.rs` and `world/fireballs.rs`. Re-check every anchor before editing. `wither-flight` also waits for d0b (`flying.rs`), and `wither-armour` for M3's segment m3a (`lightning-drawn`'s additive pass).
- **Vanilla** facts were checked with javap on the 26.3 `client.jar` (`C:\Games\Mashup\2010-Rust-Rewrite-Mashup\iw4l-artifacts\minecraft-26.3\client.jar`, read with JDK 21's javap) and against its data this round. Port numbers and behaviour only, and read data at runtime.
- **Owned elsewhere, only called from here.** The additive draw pass and the charged creeper's swirl belong to M3 (`lightning-drawn`, `lightning-glow`, `docs/round10/track-m3.md:340-342`). `flying.rs` belongs to D0 d0b, and E3 only calls it. Game events of firework rockets, mob equipment and doors belong to W's w3. Do not edit M1-M3's hooks (decisions.md: "The M briefs give every cross-track hook one owner").
- **Protocol.** No group changes a `ToClient`/`ToServer` message or the codec: the wither bar's `darken_screen` already travels (`minecraft_net/src/messages.rs:689`), and the fireball and TNT tags are NBT inside existing messages. Never bump `minecraft_net::PROTOCOL`. If a commit does change one, note it under "## Protocol" in `docs/round10/track-e3-log.md`.
- **Multiplayer.** The host runs every player's hits, pushes and deaths (`render_anim/src/minecraft_players.rs` `struck` at `:753` and `settle` at `:1026` serve friends too), so each fix covers friends. Every owner's step list ends with a friend window.
- **Code rules.** New enum variants, struct fields and match arms go next to a related one, never at the end. No code comments of any kind. Change only the lines a fix needs. A new file must pass `rustfmt --check` on its staged content. The release-note lines this section fixes or corrects go under "State doc changes" in the log, not into `docs/`.

---

### Group `blast-push`

**Gaps:** explosions-2, explosions-1. Two commits.

**Cause**
- **MW2 craters push no player.** A crater is `VoxelEvent::Explosion` (`crates/sim/src/voxel.rs:169`, pushed from `sim/src/damage.rs:113` and `:433`). The host's level takes it at `render_anim/src/minecraft_world.rs:1015-1019` and friends' levels at `render_anim/src/minecraft_levels.rs:321-323`. From there it goes into `Entities::weapon_blast` (`minecraft_survival/src/entities.rs:1606`), then `Command::WeaponBlast` (`minecraft_terrain/src/server.rs:4239`, `:4764`, `:4965`), then `ServerSim::weapon_blast` (`:2233-2238`). That function only breaks blocks and truncates the effects.
- Mobs get MW2's own push through `VoxelEvent::MobBlast { impact }` (`damage.rs:294-304` → `entities.rs:1575-1604` → `ServerSim::mob_blast` `server.rs:684`). Players get nothing, and `crates/sim` has no player knockback at all.
- **Explosion pushes are never scaled.** `apply_explosion_by` (`third_party/minecraftoss/entities/src/world.rs:3260-3320`) gives a player `PlayerHitKind::Explosion { knockback: direction * impact }` (`:3301-3318`).
  - `route` puts that push into `out.pushes` unchanged (`entities.rs:344` through `explosion_push`, `:2046-2051`).
  - Pvp-spared blasts go to `players_pushed` (`spared_by_blasts_without_pvp`, `:2027-2039`, called at `:1741` and routed at `:316-320`). So do wind bursts (`ServerSim::wind_burst`, `server.rs:2356-2364`).
  - `minecraft_players.rs:949-956` launches every push as it is.
  - `protection.rs` reads only `burning_time` from `minecraft:attributes` (`:20-21`, `:291-306`, `Protection` at `:458-516`).
  - `out.pushes` also carries pushes vanilla never scales: a shield block (`server.rs:4999`), the dragon's wings (`:3175`) and ravager roars (`:5173`).

**Vanilla 26.3**
- `ServerExplosion.hurtEntities` (bytecode 361-422) pushes each entity by `normalize(eye - centre) × (1 - distance / (2 × radius)) × getSeenPercent × ExplosionDamageCalculator.getKnockbackMultiplier × (1 - LivingEntity.getAttributeValue(Attributes.EXPLOSION_KNOCKBACK_RESISTANCE))`, then calls `Entity.pushFromExplosion`.
  - A `Player` that is not a spectator and not flying in creative goes into `hitPlayers`, and that push is what the client applies.
  - Wind charges burst through the same method, so their push on a player is scaled too.
  - `EnderDragon.knockBack`, a shield's knockback and `Ravager.roar` are not explosions, and they are not scaled.
- `Attributes.EXPLOSION_KNOCKBACK_RESISTANCE` is `RangedAttribute(default 0, min 0, max 1)`.
- `enchantment/blast_protection.json` puts `explosion_knockback_resistance` under `minecraft:attributes` with operation `add_value`, linear base 0.15 and per level above the first 0.15, slots `armor`. `EnchantmentAttributeEffect.idForSlot` gives each slot its own modifier, so pieces add up: one Blast Protection IV piece is 0.6, and two reach the cap of 1.0 (no push).
- 26.3 has no MW2 blasts. The fork treats a crater as `Level::explode` (`weapon_blast`), so the crater's push on players is `hurtEntities`' push with the default calculator (multiplier 1). MW2 still does the damage.

**Plan**
1. **"Blast Protection softens the push of Minecraft explosions"**
   - `protection.rs`:
     - Constants `EXPLOSION_KNOCKBACK_RESISTANCE` (`minecraft:explosion_knockback_resistance`) and `ADD_VALUE` (`add_value`), next to `BURNING_TIME` and `ADD_MULTIPLIED_BASE`.
     - `ProtectionBook.blast_knockback: HashMap<String, Vec<Linear>>`, filled next to `burning` (`:291-306`).
     - `ProtectionBook::explosion_knockback_resistance(&worn)`, the sum over the worn pieces, next to `burning_time` (`:413`).
     - `Protection::explosion_knockback_resistance()`, clamped to 0..1, next to `burning_ticks` (`:510`).
   - `Survival::blast_push(push) -> DVec3`, next to `wear` (`survival.rs:357`): `push * (1 - resistance)`.
   - Explosion pushes get their own list, so other pushes stay unscaled:
     - `PlayerOutputs.blast_pushes` next to `pushes` (`entities.rs:184`), fed by `explosion_push` (`:344`).
     - `ServerSim.blast_pushes` and `Output.blast_pushes` next to `players_pushed` (`server.rs:80`, `:480`, `:4471`), with `take_blast_pushes` next to `take_players_pushed` (`:2348`) and taken at `:5172`.
     - `wind_burst` (`:2364`) fills the new list.
     - `spared_by_blasts_without_pvp` gets `&mut output.blast_pushes` (`entities.rs:1741`), routed like `players_pushed` (`:316`).
     - The player-removal retain (`:2304`) also covers `blast_pushes`.
   - `minecraft_players.rs:950`: chain `outputs.blast_pushes` mapped through `survival.blast_push` before `landed`.
2. **"MW2 grenades and rockets push Minecraft players like explosions"**
   - Vendored `world.rs`: add `pub fn blast_pushes(&self, world, blast: CreeperExplosion, players) -> Vec<(u64, DVec3)>` next to `blast_by_player` (`:3246`). It is `apply_explosion_by`'s player loop (`:3301-3318`: alive, not a spectator, the box by eye height, exposure, `players_pickable`) without the hurt. Share one private helper with that loop so both use the same numbers.
   - `ServerSim::weapon_blast` (`server.rs:2233`): after `level.explode`, extend `blast_pushes` with `self.mobs.blast_pushes(..)` for `CreeperExplosion { position: center, radius: power, powered: false }` over `self.blast_players`.
   - Mobs keep MW2's push. The crater hurts no one through this path.

**Tests**
- `protection::tests::blast_protection_resists_explosion_knockback_per_piece`, built from JSON like `fire_protection_shortens_burning_by_fifteen_percent_a_level` (`:619`): level I gives 0.15, IV gives 0.6, two IV pieces give 1.0 (capped), and a level I helmet plus level I boots give 0.3.
- `survival::tests::a_blast_push_shrinks_with_blast_protection`.
- `entities::tests`:
  - an explosion hit's push and a pvp-spared push land in `blast_pushes`;
  - `players_pushed` (the dragon or a shield block) stays in `pushes`.
- `server::tests::a_wind_burst_pushes_players_through_blast_pushes`.
- `server::tests::an_mw2_blast_pushes_players_away_and_hurts_none`. Use a power 3 crater:
  - a player 2 blocks east is pushed east and up, with no `PlayerHit`;
  - a player behind a 3-high stone wall gets nothing;
  - a spectator gets nothing;
  - a second player id (a friend) is pushed too.

**Owner's steps** (survival, Normal, by day, flat open ground, `/gamerule pvp true`; move the grenades, the RPG, its rockets and the wind charges into the hotbar)
1. **BP-1.** Throw a frag grenade so it lands 3 blocks in front of you and stand still. When it goes off, MW2 hurts you and you are thrown back and up, away from the crater. **Screenshot BP-1** in the air.
2. **BP-2.** Build a cobblestone wall 3 high and 5 wide, and stand right behind it. Throw a grenade over it so it lands 2 blocks beyond the wall. However much MW2 hurts you, you are not moved.
3. **BP-3.** Fire an RPG rocket into the ground 5 blocks ahead. You are pushed back.
4. **BP-4. Blast Protection.**
   - With no armour on, look straight down and throw a wind charge: you are launched high.
   - Put on the chestplate (Blast Protection IV) and do it again: you go up much less.
   - Add the leggings (also IV): the burst does not move you at all. **Screenshot BP-4**, standing on the ground in the burst's puff.
   - With only the boots (Blast Protection I): a little less than bare.
5. **BP-5.** Take the armour off. Light TNT with flint and steel, walk 5 blocks away and turn: you are thrown away from it. With the chestplate and leggings on, the same TNT hurts less and does not move you.
6. **BP-6. Friend window.** The friend stands 3 blocks from where the host's grenade lands, and is thrown back in both windows. With `/gamerule pvp false` the friend is still pushed. Vanilla pushes a spared player too. **Screenshot BP-6** (both windows).

```
IW4L_MINECRAFT_KIT=iw4l:frag_grenade*16,iw4l:rpg,iw4l:rocket*8,minecraft:wind_charge*64,minecraft:diamond_chestplate+blast_protection=4,minecraft:diamond_leggings+blast_protection=4,minecraft:iron_boots+blast_protection=1,minecraft:tnt*8,minecraft:flint_and_steel,minecraft:cobblestone*64,minecraft:totem_of_undying,minecraft:golden_apple*16,minecraft:cooked_beef*32
```

---

### Group `blast-owners`

**Gaps:** explosions-4, explosions-7, explosions-6. Three commits.

**Cause**
- **Crystals and the dragon lose the blast's player (explosions-4).**
  - `tick_end` calls `self.crystals.blast(center, radius, None)` and `dragon.blast(center, radius, None, ..)` for every blast of the tick (`server.rs:3079-3090`).
  - `self.explosions` (`:63`) holds only `CreeperExplosion` (pushed at `:2033`, `:2055`, `:2212`, `:2229`). So the player that `explode_blasts` knows (`BlastSource::Player`, `:2199`) and the one `explode_tnt` knows (`exploded.owner`, `:2222`) are dropped.
  - `crystal_destroyed` (`:3481-3502`) then sends the dragon at `destroyed.player.or(nearest)`, the nearest player.
  - `end_crystal.rs:368` `blast` and `ender_dragon.rs:1105` `blast` already take `player`.
- **Queued blasts wait under `/tick freeze` (explosions-7).**
  - Respawn points (`respawn_point_blast`, `:2041`) and destroyed crystals (`:3484`) queue into `self.blasts`. Only `explode_blasts` drains that queue, and `tick_mobs_with` skips it when frozen (`:1463-1465`).
  - `tick_ridden_mobs` returns early without a rider (`:1389-1394`).
  - `run_tick` runs `tick_end` only in normal ticks (`:5130-5141`), while `self.explosions` is taken every tick (`:5175`).
- **Primed TNT is not saved (explosions-6).**
  - `level_entity_tags` (`server_mobs.rs:2055-2100`) writes items, orbs and ominous item spawners and skips everything else. `load_level_entity` (`:2102-2140`) knows the same three.
  - The unload retain (`server.rs:1300`) drops only those three kinds from memory. Primed TNT therefore stays in memory while its chunk is unloaded, but is never written, and is gone after save and quit.
  - The state is `TntData { fuse, power, owner: Option<u64> }` (`world/src/level/entity.rs:46-52`), with `Entity::primed_tnt` (`:193`) and `tnt_data()` (`:256`).
  - Player UUIDs come from `ServerSim::player_uuid` (`server.rs:196`) and `EntityWorld::set_player_uuid` (`world.rs:4099`).

**Vanilla 26.3**
- **Crystals.** `EndCrystal.hurtServer` → `onDestroyedBy` → `EnderDragonFight.onCrystalDestroyed` → `EnderDragon.onCrystalDestroyed(level, crystal, pos, source)`.
  - The target is the player `source.getEntity()` when that is a `Player`. Otherwise it is `ServerLevel.getNearestPlayer(CRYSTAL_DESTROY_TARGETING, ...)`.
  - If the crystal was `nearestCrystal`, the head takes `hurt(head, damageSources().explosion(crystal, player), 10)`.
  - Then `phase.onCrystalDestroyed(crystal, pos, source, player)` runs (`DragonHoldingPatternPhase` strafes that player).
- **TNT credit.** `PrimedTnt.explode` uses `Explosion.getDefaultDamageSource` = `explosion(tnt, getIndirectSourceEntity(tnt))` = `PrimedTnt.getOwner`, so a player's TNT names the player.
- **Under freeze.** `RespawnAnchorBlock.explode` and `EndCrystal.hurtServer` call `ServerLevel.explode` at once, from the player's packet. `TickRateManager.isEntityFrozen` stops only entity ticks, so under `/tick freeze` the blast hurts, pushes and breaks at once. TNT it primes waits frozen.
- **TNT save.** `PrimedTnt.addAdditionalSaveData` writes:
  - `fuse` (short);
  - `block_state` (`BlockState.CODEC`, the TNT default `{Name:"minecraft:tnt",Properties:{unstable:"false"}}`);
  - `explosion_power` (float, only when it is not 4.0);
  - `owner` (`EntityReference.store`, a UUID int array).

  `readAdditionalSaveData` reads `fuse` (`getShortOr` 80), `block_state` (default TNT), `explosion_power` (default 4, clamped 0..128) and `owner`. The common entity data (Pos, Motion, OnGround, UUID) is saved too.

**Plan**
1. **"End crystals and the dragon blame the player whose TNT blew them up"**
   - `explosions: Vec<(CreeperExplosion, Option<u64>)>`. The four pushes give the owner: `None` at `:2033` and `:2055`, the `BlastSource::Player` player at `:2212`, and `exploded.owner` at `:2229`.
   - `take_explosions` maps back to `CreeperExplosion`.
   - The two tests that push into `server.explosions` (`:12696`, `:13008`) add `None`.
   - `tick_end` (`:3080`) passes the owner to `crystals.blast` and `dragon.blast`.
2. **"Minecraft respawn anchor and end crystal blasts go off under /tick freeze"**
   - Split `tick_end`'s blast loop (`:3080-3090`) into `fn blast_end_targets(&mut self)`, which `tick_end` calls.
   - In `run_tick`'s frozen branch (`:5140-5142`), after `tick_ridden_mobs`, call `sim.explode_blasts(&input.mob_players)` and then `sim.blast_end_targets()`.
   - Normal ticks keep their order.
3. **"Minecraft primed TNT is saved with its fuse and owner"**
   - Saving: a TNT arm in `level_entity_tags`, or a sibling function called next to it at `server.rs:1280`, so the existing test calls stay. It writes `id` `minecraft:tnt`, the common fields, `fuse` (short), `block_state` (the TNT default), `explosion_power` only when it is not 4.0, and `owner` (the owner's UUID ints through `player_uuid`).
   - Loading: a matching arm in `load_level_entity` (or a sibling at `:1252`) builds `Entity::primed_tnt(pos, motion, TntData { fuse, power, owner })`. The owner comes from a new `EntityWorld::player_with_uuid(uuid)` next to `set_player_uuid` (`world.rs:4099`).
   - The unload retain (`:1300`) also takes `tnt_data()` entities, because they are saved now.
   - Decision for the log: an owner who is not connected when the chunk loads becomes `None`. Vanilla keeps the UUID until it resolves, but a fuse lasts at most 80 ticks.

**Tests**
- `server::tests::a_players_tnt_that_breaks_a_crystal_sends_the_dragon_at_that_player`. On the dragon fight level, player 1 lights TNT beside a crystal (`ignite_tnt_by(.., Some(1))`) and player 2 stands nearer. The dragon's crystal reaction gets player 1. A creeper's blast still picks the nearest player.
- `server::tests::the_dragon_hurt_by_a_players_tnt_is_hurt_by_that_player`.
- `server::tests::a_respawn_anchor_blast_goes_off_in_a_frozen_tick`, through `run_ticks(.., false, 1)` (`:13367`): the anchor is air, one explosion is in the output, and the player hit is out.
- `server::tests::an_end_crystal_hit_in_a_frozen_tick_blows_up_at_once`.
- `server_mobs::tests::primed_tnt_round_trips_with_vanilla_field_names`: `fuse` is a short, `explosion_power` is absent at 4.0, and `owner` is an int array.
- `server::tests::reloaded_primed_tnt_keeps_its_fuse_and_blames_its_owner`. Light it, run 20 ticks, save the chunk and load it into a fresh sim with the same player UUID. It goes off 60 ticks later, credited to the player (`blast_by_player`).
- `server::tests::an_unloaded_chunk_takes_its_primed_tnt_along`.

**Owner's steps** (survival, Normal, by day, flat open ground in the Overworld; wear the kit's diamond armour)
1. **BO-1.** Run `/effect give @s minecraft:resistance 120 3` first: the anchor's blast (power 5, about 51 at 2 blocks) and the crystal's (power 6, about 56 at sword reach) kill through full diamond armour, and Resistance IV leaves about 4 hearts of each. Place a respawn anchor and right-click it with one glowstone to charge it. Type `/tick freeze`, step 2 blocks back and right-click the anchor with an empty hand. It blows up at once: a crater, and you are hurt (about 4 hearts) and thrown back. **Screenshot BO-1.**
2. **BO-2.** Eat a golden apple. Still frozen and with Resistance still on, place obsidian with an end crystal on it, and hit the crystal with the sword. It blows up at once, and you lose about 4 hearts. Then `/tick unfreeze`.
3. **BO-3. TNT across a save.**
   - Hatch three cows around a TNT block and type `/tick freeze`.
   - Light the TNT. It flashes but does not count down.
   - Save and quit, then load. The lit TNT is still there. **Screenshot BO-3.**
   - Type `/tick unfreeze` (also if the world loads unfrozen). About 4 seconds later it goes off, and cows killed by it drop experience orbs, because the blast is still yours.
4. **BO-4.** Light TNT, then at once `/tp ~ ~ ~400` and come back to a spot 10 blocks off with `/tp ~10 ~ ~-400` (out of the blast's reach). The TNT goes off only after you are back, with the time it had left.
5. **BO-5. Friend window, in the End.** Use a fresh world with the dragon alive.
   - Host: `/execute in minecraft:the_end run tp @a 0 70 0`.
   - The friend walks 15 blocks east of the portal.
   - The host places obsidian 12 blocks west of the portal with an end crystal on it. The host puts TNT next to it, lights it and runs 30 blocks north, so the friend is the nearer player.
   - When the crystal blows up while the dragon circles, the dragon turns and swoops at the host, not the friend, and shoots its fireball at the host. **Screenshot BO-5** (both windows).
   - If the dragon was perched or charging just then, try again with another crystal.

```
IW4L_MINECRAFT_KIT=minecraft:tnt*16,minecraft:flint_and_steel,minecraft:respawn_anchor*2,minecraft:glowstone*16,minecraft:end_crystal*8,minecraft:obsidian*16,minecraft:cow_spawn_egg*8,minecraft:cobblestone*64,minecraft:diamond_sword,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:totem_of_undying,minecraft:golden_apple*16,minecraft:cooked_beef*32
```

---

### Group `fireball-owners`

**Gaps:** explosions-5, explosions-10. Two commits.

**Cause**
- **A sent-back fireball ignores pvp (explosions-5).**
  - `PlayerHitKind::Fireball { velocity, ignite_ticks }` carries no player (`third_party/minecraftoss/entities/src/world.rs:623`).
  - `fireball_hit` sets `source` only for `Shooter::Mob`, so a direct hit from a fireball a player sent back has no attacker (`world/fireballs.rs:244-283`, the hit built at `:274-283`).
  - `spared_by_blasts_without_pvp` filters only `Explosion { by: Some(..) }` and `Fireworks { by }` (`minecraft_survival/src/entities.rs:2027-2039`). So the fireball's blast is spared with `pvp` off, but its direct hit is not.
  - Only large fireballs can be sent back (`FireballKind::redirectable`, `fireball.rs:53-55`).
- **A skull kill does not heal the wither (explosions-10).**
  - `wither_skull_strikes` heals the wither `SKULL_KILL_HEAL` only when a mob dies (`fireballs.rs:365-369`). For a player it just queues a `PlayerHit` (`:329-342`).
  - The main thread applies that hit later (`minecraft_players.rs:861`) and decides the death after the totem check (`settle`, `:1048-1100`).
  - Nothing reports a player's death back to the entity world: `TickInput.player_hurts` is always empty in production (`entities.rs:1687`).

**Vanilla 26.3**
- **Sent-back fireballs.** `LargeFireball.onHitEntity` hurts with `damageSources().fireball(this, owner)`. After `Projectile.deflect`/`setOwner`, the owner is the player who sent it back.
  - `ServerPlayer.hurtServer` refuses a source whose `getEntity()` is a player it may not harm (`canHarmPlayer`, the pvp rule; this includes the sender itself).
  - The death line is `death.attack.fireball` "%1$s was fireballed by %2$s". Type `minecraft:fireball`.
- **Skull kills.** `WitherSkull.onHitEntity`: with a `LivingEntity` owner it hurts with `witherSkull(this, owner)` for 8.
  - If `hurtServer` returned true and the target is no longer alive, the owner heals 5 (`LivingEntity.heal(5.0F)`). Otherwise `EnchantmentHelper.doPostAttackEffects` runs.
  - Players count like any `LivingEntity`. A totem keeps the target alive, so the wither does not heal.

**Plan**
1. **"A ghast fireball a player sent back spares players without pvp and names that player"**
   - `PlayerHitKind::Fireball { velocity, ignite_ticks, by: Option<u64> }` (`world.rs:623`, like `Fireworks { at, by }`).
     - `fireball_hit` sets `by` for a `Shooter::Player` owner.
     - `world/blazes.rs:500` sets `by: None`.
     - Patterns with `..` stay as they are.
   - `spared_by_blasts_without_pvp`: add `PlayerHitKind::Fireball { by: Some(by), .. } => !is_player(by),` next to the `Fireworks` arm.
   - `mob_hit`'s cause (`entities.rs:2099-2104`): add `PlayerHitKind::Fireball { by: Some(by), .. } => DamageCause::PlayerFireball { client }` next to the `PlayerFireworks` arm.
   - `death.rs`: add `DamageCause::PlayerFireball { client: u32 }` after `PlayerFireworks` (`:113`).
     - Its type is `minecraft:fireball` and its message `fireball`.
     - It joins the player-credit sites (`:449`, `:458`, `:548`) and gets a `by_player(.., "death.attack.fireball")` line after `:585`.
2. **"A Minecraft wither heals when its skull kills a player"**
   - Vendored: `EntityWorld::skull_killed_player(wither)`, the same `SKULL_KILL_HEAL` heal as `fireballs.rs:365-369`, for a live wither.
   - `server.rs`: `Command::SkullKill { wither: u64 }` next to `ShieldBlock` (`:4278`), its sender next to `shield_block` (`:4719`) and its arm next to `:4999`. `Entities::skull_killed(wither)` goes next to `shield_blocked` (`entities.rs:1278`).
   - `Survival::killed_by_skull() -> Option<u64>`: the wither's id when the latest combat entry is `DamageCause::Mob { id, attack: MobAttack::WitherSkull, .. }` and health is 0.
   - `minecraft_players.rs` `settle`: after `hold_off_death`, where `death` is decided (`:1075`), call `entities.skull_killed(id)` when the player died.
   - Nothing heals the wither on a totem save, on a death from the Wither effect (`DamageCause::Wither`), or on a death from the skull's own blast (`Explosion`).

**Tests**
- `entities::tests`:
  - with `pvp` off, a player-sent fireball's direct hit on another player, and on the sender, is dropped;
  - a ghast's own fireball still hits;
  - with `pvp` on, `mob_hit` gives `PlayerFireball { client }`.
- `death::tests::a_fireball_a_player_sent_back_names_that_player`: the line "%1$s was fireballed by %2$s" and kill credit to the player.
- Vendored: `fireball_hit` of a deflected fireball sets `by`.
- `survival::tests`:
  - a skull hit at 4 health gives `killed_by_skull() == Some(wither)`;
  - with a totem in hand, `None`;
  - a later death to the Wither effect, `None`.
- `server::tests::a_skull_kill_heals_its_wither_by_five`: capped at max health, and nothing for a dead wither.

**Owner's steps** (survival, Normal, by day, open ground; two windows, `scripts\mp-local.ps1`)
1. **FO-1. `pvp` off.**
   - `/gamerule pvp false`. Host and friend stand side by side, 2 blocks apart. The host uses a ghast egg 25 blocks ahead.
   - When a fireball comes at the host, the host turns to face the friend and hits the fireball with the sword as it arrives, so it flies straight into the friend.
   - The friend shows no red flash and loses no hearts, from the hit or its blast, and is only pushed. **Screenshot FO-1** (friend window, full hearts).
2. **FO-2. `pvp` on.** `/gamerule pvp true` and repeat.
   - The friend flashes red and loses hearts.
   - If the friend dies, the chat reads "<friend> was fireballed by <host>" and the kill feed names the host. **Screenshot FO-2.**
   - Then `/difficulty peaceful` and `/difficulty normal` to remove the ghast.
3. **FO-3. Skull kill.**
   - `/summon minecraft:wither ~ ~3 ~10 {Health:10f}`: its bar is almost empty and grows by one health a second.
   - Take your armour off. Let its black skulls hit you until one kills you. Drink milk between hits so the Wither effect does not kill you first.
   - The moment the skull kills you ("Sos was shot by a skull from Wither"), the bar jumps by about five seconds' worth of growth. **Screenshot FO-3** (the bar right after the death).
4. **FO-4.** The friend does the same: the bar jumps in both windows when the skull kills the friend. Afterwards, `/difficulty peaceful` and `/difficulty normal`.

```
IW4L_MINECRAFT_KIT=minecraft:ghast_spawn_egg*2,minecraft:iron_sword,minecraft:shield,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:milk_bucket,minecraft:milk_bucket,minecraft:milk_bucket,minecraft:cobblestone*64,minecraft:golden_apple*16,minecraft:cooked_beef*32
```

---

### Group `fireball-trails`

**Gaps:** explosions-9, explosions-15. Two small commits.

**Cause**
- **No smoke trail (explosions-9).**
  - Only the dragon's fireballs get a trail (`minecraft_survival/src/entity_view.rs:425-427` → `ambient.breath_trail`, `minecraft_terrain/src/ambient_particles.rs:1301-1303`).
  - Ghast and blaze fireballs and wither skulls live in the view's world (`self.world.fireballs()`, drawn at `entity_view.rs:1351-1379`) and add no particle.
  - The view's per-tick block already puffs `Kind::Smoke` over primed TNT the same way (`:151-158`).
- **Not heard when shot (explosions-15).**
  - `mob_tags_where` saves fireballs with `LeftOwner` but not `HasBeenShot` (`minecraft_terrain/src/server_mobs.rs:1926-1953`), and the load does not read it (`:1157-1167`). Shulker bullets are the same (`:1955-1985`, `:1169-1184`).
  - `Fireball` (`third_party/minecraftoss/entities/src/fireball.rs:91-99`) and `ShulkerBullet` (`shulker_bullet.rs:59-75`) have no such flag.
  - Neither posts `projectile_shoot`: `spawn_fireball` (`world/fireballs.rs:20-31`) and the tick (`tick_fireball`, `:188`; `tick_shulker_bullet`, `world/shulker_bullets.rs:217`) post no game event. W's thrown things (`server.rs:2250`), tridents (`:2330`) and arrows (`world.rs:3338`) already post one.

**Vanilla 26.3**
- **Trail.** `AbstractHurtingProjectile.tick` ends with `createParticleTrail()`: `level.addParticle(getTrailParticle(), x, y + 0.5, z, 0, 0, 0)` every tick.
  - `getTrailParticle` is `ParticleTypes.SMOKE`. `LargeFireball`, `SmallFireball` and `WitherSkull` do not override it, and `DragonFireball` overrides it with `DRAGON_BREATH`.
  - In water, `applyInertia` adds bubbles. Those are left out, and the log says so.
- **Heard when shot.** `Projectile.tick` posts `gameEvent(GameEvent.PROJECTILE_SHOOT, getOwner())` once, when `!hasBeenShot`, then sets the flag. For hurting projectiles this happens after the first move.
  - `Projectile.addAdditionalSaveData` writes `HasBeenShot` (and `LeftOwner`). `readAdditionalSaveData` reads `getBooleanOr("HasBeenShot", false)`, so a reloaded projectile is not announced twice.
  - The event's frequency is 3 (`game_events.rs:98`).

**Plan**
1. **"Minecraft fireballs and wither skulls leave a smoke trail"**
   - In `EntityView::advance`'s tick block, next to the primed TNT loop (`entity_view.rs:151-158`): for each alive fireball and skull in `self.world.fireballs()` near the viewer, call `self.ambient.add(Kind::Smoke, position + DVec3::Y * 0.5, DVec3::ZERO)`.
   - The dragon's fireballs keep their breath trail. Friends use the same view.
2. **"Minecraft fireballs, wither skulls and shulker bullets are heard when shot"**
   - `has_been_shot: bool` next to `left_owner` (`fireball.rs:99`), and the same on `ShulkerBullet`.
   - The first tick (`tick_fireball`, `tick_shulker_bullet`) posts one `self.game_event("minecraft:projectile_shoot", position, owner mob, None)` and sets the flag (W0 rule 1: one line per site). A player owner posts with no source.
   - Save `HasBeenShot` next to `LeftOwner` (`server_mobs.rs:1940`, and the bullet's tag) and load it next to `LeftOwner` (`:1165`, the bullet arm), defaulting to false.

**Tests**
- `entity_view::tests::fireballs_and_skulls_puff_one_smoke_a_tick`: the smoke sits 0.5 above each, and a dragon fireball still gets breath, not smoke.
- Vendored `world::fireballs::tests`:
  - a ghast's fireball posts one `projectile_shoot` with the ghast as source on its first tick and none after;
  - a fireball loaded with `has_been_shot` posts none;
  - the same for a shulker bullet.
- `server_mobs::tests::fireballs_and_shulker_bullets_keep_has_been_shot`: `HasBeenShot` is 1b on save, and a tag without it loads as not shot.
- `server::tests::a_blazes_shot_powers_a_sculk_sensor_with_three`, through `run_tick`.

**Owner's steps** (survival, Normal, by day, flat open ground; drink the fire resistance potion first)
1. **FT-1.** Build a cobblestone ring 2 high and use a blaze egg inside it, 12 blocks away. Each small fireball it shoots drags a short grey smoke trail. **Screenshot FT-1.**
2. **FT-2.** Use a ghast egg 25 blocks away. Its big fireballs trail smoke. **Screenshot FT-2.**
3. **FT-3.** Use a wither egg 20 blocks away. Its black and blue skulls trail smoke too. Then `/difficulty peaceful` and `/difficulty normal`.
4. **FT-4. Heard.**
   - Place a sculk sensor with a comparator out of it into a line of 15 redstone dust.
   - Use the blaze egg 5 blocks from the sensor and stand 10 blocks away on the other side. Each time the blaze opens fire, the line lights 3 dust.
   - Do the same with a shulker egg: its bullets also light 3.
5. **FT-5. Friend window.** The friend sees the same trails.

```
IW4L_MINECRAFT_KIT=minecraft:blaze_spawn_egg*2,minecraft:ghast_spawn_egg*2,minecraft:wither_spawn_egg*2,minecraft:shulker_spawn_egg*2,minecraft:sculk_sensor*4,minecraft:comparator*2,minecraft:redstone*32,minecraft:cobblestone*64,minecraft:potion@long_fire_resistance,minecraft:potion@long_fire_resistance,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:shield,minecraft:milk_bucket,minecraft:golden_apple*16,minecraft:cooked_beef*32
```

---

### Group `boss-darkening`

**Gaps:** explosions-8. One commit.

**Cause**
- The withers' `BossBarFlags.darken_screen` is set (`minecraft_survival/src/session.rs:881-892`) and sent to friends (`minecraft_net/src/messages.rs:689`), but nothing reads it.
  - `McBossBar` has only `world_fog` and `music` (`crates/frame/src/minecraft_ui.rs:297-307`).
  - The friend copies only `create_world_fog` and `play_music` (`render_anim/src/minecraft_remote.rs:853-858`).
  - The host's wither bars are made with `(false, false)` (`render_anim/src/minecraft_wither.rs:27`).
- `SkyInputs` has no darkening (`render_anim/src/minecraft_world.rs:1611-1627`).
- Both lightmap ports lack the boss term: the terrain one at `render_gpu/src/drawsurf/minecraft_world.rs:959-975` and the model one at `minecraft_light.rs:275-293`.
- `environment[1].w` is free. `present_sky` writes `put(right)` there (`minecraft_world.rs:1711`), and no shader reads it: W1 took `[0].w`, and the model pass owns `[3].w`.

**Vanilla 26.3**
- `GameRenderer.tick` returns when the tick rate does not run normally. Otherwise:
  - `bossOverlayWorldDarkeningO = bossOverlayWorldDarkening`;
  - if `Hud.getBossOverlay().shouldDarkenScreen()`, it adds 0.05f, capped at 1;
  - otherwise, while it is above 0, it subtracts 0.0125f.

  `bossOverlayWorldDarkening(partial)` is the lerp of the old and new values.
- `LightmapRenderStateExtractor` passes that value to `lightmap.fsh`, which runs `color = mix(color, color * vec3(0.7, 0.6, 0.6), BossOverlayWorldDarkeningFactor)` after the block light and before `color - vec3(DarknessScale)` and the clamp.
- `GameRenderer.extractCamera` also passes it to `FogRenderer.setupFog` → `computeFogColor`. After the void and darkness darkening, if the value is above 0: `r = lerp(f, r, r × 0.7)`, `g = lerp(f, g, g × 0.6)`, `b = lerp(f, b, b × 0.6)`, with no fog-type check.
- The sky texture itself is not darkened.
- Only the wither's bar sets `darkenScreen`. The dragon's bar sets fog and music.

**Plan: "A Minecraft wither's boss bar darkens the world like vanilla"**
- New `crates/minecraft_terrain/src/boss_darkening.rs` (`pub mod` next to `darkness` in `lib.rs:13`):
  - `BossDarkening { value, previous }` with `tick(darken)` (+0.05 capped at 1, else −0.0125 while above 0);
  - `factor(partial)`;
  - `advance(seconds, darken)`, like `Darkness::advance` (`darkness.rs:46`);
  - `fog(colour, factor)`.
- `McBossBar.darken: bool` after `world_fog`, default false. The host's wither bar sets it when it is created (`minecraft_wither.rs:27`). The friend copies `bar.flags.darken_screen` (`minecraft_remote.rs:853-858`).
- `SkyInputs.darken` after `world_fog` (`minecraft_world.rs:1594`): `ui.boss_bars.iter().chain(&ui.wither_bars).any(|bar| bar.darken)`, set for the host (`:1592`) and the friend (`minecraft_remote.rs:1695`).
- A `BossDarkening` next to each viewer's `darkness` (`minecraft_world.rs:98`, `minecraft_remote.rs:149`), advanced next to `darkness.advance` (`:1580`, `minecraft_remote.rs:1694`), only while the tick rate runs normally.
- `present_sky`:
  - after `camera_fog` (`:1703`), `sky.fog = boss_darkening::fog(sky.fog, factor)`;
  - `environment[1] = [right.x, right.y, right.z, factor]` in place of `put(right)`.
- Both lightmaps get the line before their darkness subtraction: `color = mix(color, color * vec3<f32>(0.7, 0.6, 0.6), environment.right.w);` in the terrain one and `mc_light.environment[1].w` in the model one. No comments in the WGSL either.

**Tests**
- `boss_darkening::tests::it_rises_a_twentieth_a_tick_and_falls_an_eightieth`: 20 ticks to 1, 80 ticks back to 0, capped, and the partial lerps.
- `boss_darkening::tests::the_fog_keeps_seventy_and_sixty_percent`: factor 1 gives `(0.7r, 0.6g, 0.6b)`, and factor 0.5 gives halfway.
- `render_gpu`: both shader sources run the boss mix before the darkness line (string tests like `minecraft_light.rs:481`), and the model pass still writes only `environment[3].w`.
- `minecraft_world::tests::a_withers_bar_darkens_the_environment_block`, and the dragon's bar does not.
- `minecraft_remote::tests::a_friend_under_a_withers_bar_darkens_too`.
- `minecraft_wither::tests`: the host's wither bar has `darken`.

**Owner's steps** (survival, Normal, by day, open ground with some grass, trees and a few torches)
1. **BD-1.** Take **Screenshot BD-1** (before).
2. **BD-2.** `/gamemode creative`, then use the wither egg 15 blocks away. Its purple bar appears.
   - Within about a second the ground, trees, mobs and torch-lit blocks turn darker and slightly red-brown.
   - The fog at the horizon darkens with them, and the sky overhead keeps its colour. **Screenshot BD-2.**
3. **BD-3.** `/difficulty peaceful`: the wither and its bar vanish, and the world brightens back over about 4 seconds. Then `/difficulty normal` and `/gamemode survival`.
4. **BD-4.** Build one from 4 soul sand and 3 skulls. The darkening starts as soon as its bar appears. Remove it with `/difficulty peaceful`, then `/difficulty normal`.
5. **BD-5. Friend window.** The friend stands 10 blocks from the host's wither and sees the same darkening. The friend then does `/tp ~ ~ ~300`: the bar goes and the friend's world brightens. **Screenshot BD-5** (both windows).
6. **BD-6.** In the End, `/execute in minecraft:the_end run tp @s 0 70 0`. The dragon's bar brings its fog but no darkening.

```
IW4L_MINECRAFT_KIT=minecraft:wither_spawn_egg*2,minecraft:soul_sand*8,minecraft:wither_skeleton_skull*6,minecraft:torch*32,minecraft:cobblestone*64,iw4l:m4,iw4l:rifle_ammo*64,iw4l:rifle_ammo*64,minecraft:netherite_sword,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:milk_bucket,minecraft:golden_apple*16,minecraft:cooked_beef*32
```

---

### Group `dragon-potions`

**Gaps:** explosions-19. One commit.

**Cause**
- The splash at `server.rs:2711-2714` (`crate::splash::splash(&mut self.mobs, ...)`, `splash.rs:108`) only visits `thrown::targets` and players.
- Lingering clouds' visitors are the same (`tick_clouds`, `server.rs:2258-2300` here, `:2301` after e2b).
- The dragon lives in `self.dragons` with its own `EndDamage` path. After e2b these are `attack`, `thrown` and `explosion`; `ender_dragon.rs` `hurt_part` (`:1032` on track-e2) and `hurts_the_dragon` have no magic or heal.
- e2b breaks potions on crystals and parts without effect (`docs/round10/track-e2-log.md:1399`).

**Vanilla 26.3**
- `ThrownSplashPotion.onHitAsPotion` takes every `LivingEntity` in the potion's box inflated by (4, 2, 4) that `isAffectedByPotions`.
  - The squared distance from the potion's box to the entity's box, inflated by `ProjectileUtil.computeMargin`, must be below 16.
  - The scale is 1 for the hit entity, otherwise `1 - sqrt(d) / 4` (the fork's `potion::splash_scale`).
- Instant effects run `MobEffect.applyInstantenousEffect`: harming is `hurtServer(indirectMagic(potion, owner), 6 << amplifier × scale)`, and healing is `heal(4 << amplifier × scale)` (`potions::instant_amount`, `potions.rs:393`).
- `EnderDragon.hurtServer` → `hurt(body, ..)`:
  - the phase's `onHurt` runs;
  - a non-head part takes `damage / 4 + min(damage, 1)`;
  - below 0.01 nothing happens;
  - damage counts only when `source.getEntity() instanceof Player` or the source is in `#always_hurts_ender_dragons`. Indirect magic is not in that tag, so a dispenser's potion does nothing.
- `EnderDragon.addEffect` returns false, so lasting effects never stick.
- `AreaEffectCloud` applies instant effects to living entities in its box the same way.

**Plan: "Splash and lingering harming and healing reach the Minecraft ender dragon"**
- At `server.rs:2714`, also test each dragon's own box (16 × 8) with `splash_scale`, for instant effects only:
  - harming → `dragon.hurt_part(BODY, &EndDamage::magic(amount, owner), ..)`;
  - healing → a new `EnderDragon::heal(amount)` (not while dying, capped at max health).
- `EndDamage::magic` goes next to `thrown`, with `DamageKind::Magic` next to `Fire`. It hurts only with a player owner, and the perched arrow rule does not apply to it.
- Lingering: the dragons join `tick_clouds`' visitors and take the instant effects through the cloud's existing amounts.
- Events go through `end_events`, so a potion kill counts as the player's.

**Tests**
- `ender_dragon::tests`:
  - magic from a player takes `damage / 4 + 1` off the body;
  - magic from no one does nothing;
  - the heal is capped.
- `server::tests`:
  - a Harming II splash 2 blocks from a perched dragon takes `(12 × scale) / 4 + 1`;
  - a Healing II splash heals it;
  - a lingering harming cloud under it hurts it on each reapplication;
  - no crystal is affected.

**Owner's steps** (fresh world with the dragon alive, survival, Normal)
1. `/execute in minecraft:the_end run tp @s 0 70 0`. Wait by the exit portal until the dragon lands on it.
2. **DP-1.** Throw a Harming II splash at it. It flashes red and its bar drops by about 2%. **Screenshot DP-1.**
3. **DP-2.** Throw a Healing II splash at it: the bar grows a little.
4. **DP-3.** Throw a lingering Harming II onto the portal while it sits. The bar drops again every second or so while it stays. **Screenshot DP-3.**
5. **DP-4.** A Harming I splash that breaks on its body while it flies also takes a little off.
6. **DP-5. Friend window.** The friend's harming splash hurts it the same way.

```
IW4L_MINECRAFT_KIT=minecraft:splash_potion@strong_harming*4,minecraft:splash_potion@strong_healing*2,minecraft:lingering_potion@strong_harming*2,minecraft:splash_potion@harming*2,minecraft:ender_pearl*16,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:shield,minecraft:cobblestone*64,minecraft:totem_of_undying,minecraft:golden_apple*16,minecraft:cooked_beef*32
```

That is 20 backpack slots (potions don't stack).

---

### Group `wither-flight` (after d0b, which is merged: c0366f3a)

**Gaps:** explosions-11. One commit: d0b's optional "Minecraft withers fly around walls".

**Cause**
- The wither steers straight at a single block with `wither_ai::FlightPath` (`third_party/minecraftoss/entities/src/wither_ai.rs:29-91`, held in `WitherState.flight` at `:98`).
- Its goals set and stop that path (`:204`, `:227`, `:278`, `:289`), and `wither::fly` (`wither.rs:204-246`) turns it into motion (`world/withers.rs:308-312`).
- There is no path search, so a wall between the wither and its goal stops it until `STUCK_CHECK_TICKS` (100) drops the node.
- d0b built `flying.rs` (on track-d0: `fly` `:52`, `plan_path` `:69`, `tick_path` `:94`, `FlyingMoveControl` `:273-300`) and left this commit open (`docs/round10/track-d0-log.md`, "Left for later"). It names the steps: a flying `WalkProfile` in the wither's `MonsterGoalContext`, goals that call `navigate_walk_to`, and `flying::tick_path` with `FlyingMoveControl::new(10, false)` in place of `FlightPath::tick` and `wither::fly`.

**Vanilla 26.3**
- The `WitherBoss` constructor sets `moveControl = new FlyingMoveControl(this, 10, false)`.
- `createNavigation` returns a `FlyingPathNavigation` with `setCanOpenDoors(false)` and `setCanFloat(true)`.
- `registerGoals`:
  - 0: `WitherDoNothingGoal`;
  - 2: `RangedAttackGoal(1.0, 40, 20.0)`, which `navigation.stop()`s when the target is within 20 and seen for 5 ticks and otherwise `moveTo(target, 1.0)`;
  - 5: `WaterAvoidingRandomFlyingGoal(1.0)`;
  - 6: `LookAtPlayerGoal`;
  - 7: `RandomLookAroundGoal`.
- `aiStep`'s direct approach toward its first target is separate and stays: it climbs to 5 above the target while not powered, with a 0.3 horizontal pull beyond 3 blocks.

**Plan**
- `wither_ai.rs`: give the wither's `MonsterGoalContext` a flying `WalkProfile` (`flying::fly`). `RangedAttackGoal` and the random flying goal call `navigate_walk_to` and stop the navigation (stroll with d0b's `hover_random_position` / `air_and_water_random_position`).
- `world/withers.rs:308-312`: `flying::tick_path` plus `FlyingMoveControl::new(10, false)` replace `FlightPath::tick` and `wither::fly`. The direct approach, the charge and the head code stay.
- Follow the d0b log's "How a family uses it". Do not edit `flying.rs`; if it lacks something, note it in the log and ask D0's owner (it is shared with M1 and M3).
- `FlightPath` goes only when nothing uses it. Its two tests (`wither_ai.rs:342`, `:353`) become equivalent tests on the new path. Do not drop them.

**Tests**
- `world::withers::tests::a_wither_flies_over_a_wall_to_a_player_it_cannot_see`: with `mob_griefing` off, an 8-high stone wall stands between it and the player, and within a set number of ticks it is on the player's side.
- `world::withers::tests::in_the_open_it_still_keeps_five_above_its_target`.
- The old straight-flight tests are reworked on the path.

**Owner's steps** (survival, Normal, by day, flat ground)
1. Type `/gamerule mob_griefing false`, so its hits and skulls break nothing.
2. Build a cobblestone wall 21 wide and 8 high, and stand at its middle on one side.
3. **WF-1.** Use the wither egg 10 blocks from the wall on the other side. It no longer hangs against the wall: it climbs over the top or flies round an end and comes at you. **Screenshot WF-1** as it crosses.
4. **WF-2.** Step out into the open. It hovers about five blocks above you again, as before.
5. Then `/difficulty peaceful`, `/difficulty normal` and `/gamerule mob_griefing true`.
6. **WF-3. Friend window.** The friend hides behind the wall instead and sees the same.

```
IW4L_MINECRAFT_KIT=minecraft:wither_spawn_egg*2,minecraft:cobblestone*64,minecraft:cobblestone*64,minecraft:cobblestone*64,minecraft:cobblestone*64,minecraft:netherite_sword,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:milk_bucket,minecraft:milk_bucket,minecraft:golden_apple*16,minecraft:cooked_beef*32
```

---

### Group `wither-armour` (after M3's `lightning-drawn`)

**Gaps:** explosions-13. One commit.

**Cause**
- The armour swirl goes into the translucent mesh with `ARMOR_ALPHA` 0.5 at full light (`minecraft_terrain/src/wither_render.rs:21`, `append_swirl` at `:286-299`, the call at `:349-359`).
- `append_withers` (`:301`) gets `&mut out.translucent` from `entity_view.rs:1207`.
- Every entity blend is `BlendState::ALPHA_BLENDING` today (`render_gpu/src/drawsurf/minecraft_world.rs:1675-1760`).
- M3's `lightning-drawn` adds the additive pass (`MobMeshes.additive`, a 7th entity mesh, `track-m3.md:317-318`, `:340`) for lightning and the charged creeper.

**Vanilla 26.3**
- `WitherArmorLayer` extends `EnergySwirlLayer`. `EnergySwirlLayer.submit` draws with `RenderTypes.energySwirl(texture, xOffset, yOffset)` in colour `0xFF808080` (0.5 per channel) at full light.
- `RenderPipelines` registers `pipeline/energy_swirl` with `ColorTargetState(BlendFunction.ADDITIVE)`, `DepthStencilState.DEFAULT` (`GREATER_THAN_OR_EQUAL` with depth write) and no cull.

**Plan: "The Minecraft wither's armour glows like vanilla's"**
- `append_withers` takes M3's additive mesh for the swirl in place of `translucent`, and `entity_view.rs:1207` passes `&mut out.additive`.
- The swirl's skin carries vanilla's colour 0.5 per channel at alpha 1, in place of `ARMOR_ALPHA`, at full light. Use whichever of the vertex colour or alpha M3's additive fragment scales by. The scroll (`swirl_shift`) and the cube growth stay.
- Decision for the log: M3's pass tests depth without writing it (built for lightning), while vanilla's energy swirl writes depth. Use M3's pass as it is.

**Tests**
- `wither_render::tests::a_powered_withers_swirl_goes_to_the_additive_mesh_at_half_grey`: nothing of it lands in the translucent mesh.
- An unpowered wither adds to neither mesh.

**Owner's steps** (survival, Normal)
1. **WA-1.** `/gamemode creative`, then `/summon minecraft:wither ~ ~3 ~10 {Health:10f}`. Its moving blue armour now glows: it brightens what is behind it instead of covering it, and the wither's dark body shows through bright blue light. **Screenshot WA-1.**
2. **WA-2.** `/time set midnight`: the armour is just as bright. **Screenshot WA-2.**
3. Then `/difficulty peaceful`, `/difficulty normal` and `/time set day`.
4. **WA-3. Friend window.** The friend sees the same glow.

```
IW4L_MINECRAFT_KIT=minecraft:wither_spawn_egg*2,minecraft:torch*16,minecraft:cobblestone*64,minecraft:netherite_sword,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*16,minecraft:cooked_beef*32
```

---

### Left out of this section

- **explosions-3**, creeper-lit TNT naming the creeper. Vanilla does not name it either. `Creeper.explodeCreeper` calls `discard()` right after `level.explode`, and `EntityReference.getEntity` returns null for a removed entity. So when that TNT goes off, `PrimedTnt.getOwner` is null, the source is `explosion(tnt, null)`, and the line is "X blew up", which is what the fork does. The release-1 limit line should be corrected in the log's "State doc changes".
- **explosions-14** and **explosions-18**: see the `left` list.

## E3-8. Doors, blocks, dispensers and redstone

The known gaps of this area come from `docs/round10/release-3-tests.md:660-678` (doors, fences and cats, dispensers), `release-4-tests.md:116`, `:786`, `:806`, `release-5-tests.md:125`, `release-1-tests.md:400` and `:414`, and `docs/MINECRAFT-SURVIVAL.md:357` ("A name or components on a placed block item are dropped, except on shulker boxes and banners"). Paths and lines are as of `minecraft-survival` 9c6efebd. Check them again before editing: e2b (merge d565a3d2), d0b, m2a and W's w3 merge around this track. Vanilla facts were checked with javap on the 26.3 `client.jar`, and ids against the 26.3 resource pack.

**Rules for every group here.** No code comments of any kind. When a vendored doc line goes stale, delete it with its line and write nothing new. Edits change only the lines they need, in the surrounding style. New enum variants, constants and match arms go next to a related one, never at the end of a shared list. No commit bumps `minecraft_net::PROTOCOL`. None of these groups changes a wire struct, so the log's "## Protocol" says "none". Decisions, the owner's steps and the kit go in `docs/round10/track-e3-log.md`.

**Owned elsewhere, do not touch:**
- M1-M3's hooks (`docs/round10/decisions.md`, "The M briefs give every cross-track hook one owner").
- M2's fish bucket arm in `dispenser.rs` (after `"water_bucket" | "lava_bucket"`, now `:110`), M1's hive arm after `"honeycomb"` (now `:112`), and both tracks' `OTHER_BEHAVIOURS` lines (`dispenser.rs:17-26`, `track-m2.md:2886`).
- M3's `golem_build.rs:191-199` (`try_copper_golem`) and the creaking heart.
- M1's `World` trait lines in `player/src/lib.rs:98-238`.
- W's game event helpers. Only call them, one line per site (W0 rule 1).

---

### Group `placed-facing` (M), three commits

**Gaps:** doors-5. The same cause also leaves the other look-facing, clicked-face and pillar blocks at their defaults; this group fixes those too.

**Cause.**
- The hand places blocks through `Player::place_target_with` (`third_party/minecraftoss/player/src/lib.rs:2044-2190`).
- Only these get a placement state there: hoppers (`:2094-2103`), logs and wood (`:2104-2111`, axis from the clicked face), furnaces (`:2140-2156`), slabs, stairs and rails.
- Everything else falls to `placement::block_entity_placement` (`placement.rs:45-136`). That has arms for ender chests, campfires, pots, lecterns, bells, ladders, levers and buttons, shulker boxes and beds, and nothing for comparators, repeaters, pistons, observers, dispensers, droppers, crafters, calibrated sculk sensors, barrels, pumpkins, workstations, rods, amethyst or other pillars.
- The block keeps its default state, which is facing north or axis y (`release-4-tests.md:116` worked around it with `/setblock`).
- The helpers already exist: `nearest(yaw, pitch)` (`placement.rs:716`, `Direction.orderedByNearest`, so `[0]` is `getNearestLookingDirection`), `horizontal(yaw)` (`:759`), `opposite` (`:867`), `name` (`:892`), and `water` from `source_water_at` (`:59-63`).

**Vanilla 26.3** (javap `getStateForPlacement`):
- `PistonBaseBlock`: `FACING = getNearestLookingDirection().getOpposite()`, `EXTENDED = false`.
- `DispenserBlock` (and `DropperBlock`, which extends it) and `BarrelBlock`: nearest looking, opposite.
- `ObserverBlock`: `getNearestLookingDirection().getOpposite().getOpposite()`, so the nearest looking direction itself.
- `DiodeBlock` (comparator): `getHorizontalDirection().getOpposite()`.
- `RepeaterBlock`: the same, plus `LOCKED = isLocked`. That is true when a side neighbour (clockwise or counter-clockwise of FACING) is a repeater or comparator with `POWERED` whose `FACING` equals the direction from this block to it (`DiodeBlock.getSignal` gives 0 unless powered and `FACING == direction`).
- `CalibratedSculkSensorBlock`: `SculkSensorBlock`'s state (waterlogged), then `FACING = getHorizontalDirection()` with no opposite. The input side is `pos.relative(FACING.getOpposite())` (`CalibratedSculkSensorBlockEntity$VibrationUser.getBackSignal`), so it faces the player.
- `CrafterBlock`: `front = nearest.getOpposite()`, and `top` is `horizontal.getOpposite()` when front is DOWN, `horizontal` when front is UP, else UP. `ORIENTATION = FrontAndTop.fromFrontAndTop(front, top)` and `TRIGGERED = hasNeighborSignal`.
- `CarvedPumpkinBlock` (also `jack_o_lantern`), `LoomBlock`, `StonecutterBlock`, `BeehiveBlock` (also `bee_nest`), `ChiseledBookShelfBlock`, `GlazedTerracottaBlock` and `VaultBlock`: `getHorizontalDirection().getOpposite()`.
- `AnvilBlock`: `getHorizontalDirection().getClockWise()`.
- `EndRodBlock`: `FACING = clickedFace`, but the opposite when the block at `clickedPos.relative(clickedFace.getOpposite())` is an end rod whose `FACING == clickedFace`.
- `LightningRodBlock` and `AmethystClusterBlock` (the three buds too): `FACING = clickedFace` and waterlogged.
- `RotatedPillarBlock` and `ChainBlock`: `AXIS = clickedFace.getAxis()` (chains also waterlogged).

**Plan.**
1. **"Minecraft pistons, observers, dispensers, droppers, repeaters, comparators, crafters and calibrated sculk sensors face the way they are placed"**
   - Arms in `block_entity_placement`'s match, after the `"lectern"` arm: `piston` and `sticky_piston`, `observer`, `dispenser` and `dropper`, `comparator`, `repeater`, `calibrated_sculk_sensor` and `crafter`, with the facings above.
   - The repeater's `locked` reads the two side neighbours through `world.block(step(pos, side))`, as the vanilla rule above says.
   - The crafter gets `orientation` `<front>_<top>` and `triggered=false`. Crafters do nothing in the fork, and the player crate's `World` has no neighbour signal (decision).
   - The calibrated sensor gets `waterlogged` from `water`.
2. **"Minecraft barrels, pumpkins, workstations, glazed terracotta and anvils face the way they are placed"**
   - Arms for `barrel` (nearest, opposite); `carved_pumpkin`, `jack_o_lantern`, `loom`, `stonecutter`, `beehive`, `bee_nest`, `chiseled_bookshelf`, `vault` and every `*_glazed_terracotta` (horizontal, opposite); and `anvil`, `chipped_anvil` and `damaged_anvil` (horizontal, clockwise; `clockwise` is at `placement.rs:878`).
3. **"Minecraft rods, amethyst, chains and pillar blocks point along the face they are placed on"**
   - Arms for `end_rod` (with the flip), every `*lightning_rod` (the copper and waxed variants too), and `amethyst_cluster` and the three `*_amethyst_bud`, all with `view.face` and `waterlogged`.
   - In `lib.rs:2104` the log and wood condition also takes:
     - the crimson and warped stems and hyphae (stripped too);
     - `hay_block`, `bone_block`, `basalt`, `polished_basalt`, `quartz_pillar` and `purpur_pillar`;
     - `deepslate` and `infested_deepslate`;
     - `bamboo_block` and `stripped_bamboo_block`;
     - `muddy_mangrove_roots` and the three `*_froglight`;
     - `iron_chain` and the copper chains (waterlogged from the clicked spot).
   - `creaking_heart` stays M3's.

**Tests.**
- `player::placement::tests`, with the existing `view(yaw, pitch, face)` and `state(..)` helpers:
  - `pistons_dispensers_and_droppers_face_the_player_and_up_when_looked_down_on` (yaw 180 level gives south; pitch 80 gives up; pitch -80 gives down; `extended=false`);
  - `an_observer_looks_where_the_player_looks`;
  - `repeaters_and_comparators_point_away_from_the_player`;
  - `a_repeater_beside_a_powered_side_repeater_is_placed_locked` (also: unpowered gives unlocked; a powered repeater behind it gives unlocked);
  - `a_calibrated_sensor_turns_its_input_to_the_player`;
  - `a_crafter_takes_front_and_top_from_the_look` (`south_up`, `up_north`, `down_south`);
  - `barrels_pumpkins_and_workstations_face_the_player` (all ids of commit 2);
  - `an_anvil_lies_across_the_view`;
  - `an_end_rod_on_an_end_rod_tip_points_back_into_it`;
  - `rods_and_amethyst_stick_out_of_the_clicked_face`.
- `player::tests::pillars_and_chains_take_the_axis_of_the_clicked_face` (lib.rs, through `place_target_with`).

**Owner's steps** (survival, by day, on a flat stone floor you place; stand 2 blocks from each spot and look at it, not steeply, unless a step says otherwise):
- **F-1.** Place a piston: its wooden pushing face points at you. Look steeply down at the floor beside you and place another: its face points up. Place one against the underside of a block above you: its face points down. Do the same with the sticky piston (green face). **Screenshot blk-1.**
- **F-2.** Place an observer, then a redstone lamp on the side facing you (its back). The observer's face (the eyes) looks away from you. Place a stone block in front of its face: the lamp behind it blinks once.
- **F-3.** A dispenser and a dropper: the hole faces you, and faces up when placed while you look steeply down.
- **F-4.** Build a line on the floor, placing each part while you look along the line away from the lever: lever, 2 redstone dust, a repeater, 2 dust, a comparator, 1 dust, a redstone lamp. Flip the lever: the lamp lights, because both parts point away from you. Before this build, every hand-placed repeater and comparator faced north. **Screenshot blk-2.**
- **F-5. Lock.**
  1. Place repeater B. Beside it to the right, place repeater A while looking at B's side, so A points into B.
  2. Feed A from behind with 1 dust and a lever, and switch the lever on.
  3. Break B and place it again, looking the same way as the first time: B shows its grey lock bar at once.
- **F-6.** Place a calibrated sculk sensor: its odd side (the input side) faces you. A crafter: its front grid faces you, and faces up when placed while you look down.
- **F-7.** Place a barrel: its lid faces you, and faces up when placed while you look down. A carved pumpkin looks at you. The loom, chiseled bookshelf and beehive face you. Place 4 white glazed terracotta as a 2x2 square, turning a quarter to the right before each: their patterns turn with you. An anvil lies across your view, its horn to your left or right. **Screenshot blk-3.**
- **F-8.**
  1. Click the side of a stone block with an end rod: it sticks out sideways. Click its tip with a second end rod: the second points back into the first.
  2. A lightning rod and an amethyst cluster clicked on a wall stick out of the wall.
  3. A hay bale, a quartz pillar and an iron chain clicked on the side of a block lie on their side, pointing at that block.
  **Screenshot blk-4.**
- **F-MP (two windows).** F places a piston and an observer while looking at H: in both windows the piston's face points at F and the observer looks away from F.

```
IW4L_MINECRAFT_KIT=minecraft:piston*4,minecraft:sticky_piston*4,minecraft:observer*4,minecraft:dispenser*4,minecraft:dropper*4,minecraft:repeater*8,minecraft:comparator*4,minecraft:calibrated_sculk_sensor*2,minecraft:crafter*2,minecraft:barrel*4,minecraft:carved_pumpkin*4,minecraft:loom,minecraft:chiseled_bookshelf,minecraft:beehive,minecraft:white_glazed_terracotta*8,minecraft:anvil,minecraft:end_rod*8,minecraft:lightning_rod*2,minecraft:amethyst_cluster*4,minecraft:hay_block*8,minecraft:quartz_pillar*8,minecraft:iron_chain*8,minecraft:lever*2,minecraft:redstone*32,minecraft:redstone_lamp*2,minecraft:stone*64
```
26 stacks. F-2 and F-4 need the redstone lamps; the jack o'lantern and the crimson stem, which follow the carved pumpkin's and the pillars' rules, made room for them.

---

### Group `cross-outlines` (S), one commit

**Gaps:** doors-7.

**Cause.**
- `outline_shapes` (`player/src/lib.rs:720-756`) gives fences `fence_outline` (`:707-718`, E2's 3188f4f8).
- Walls (`*_wall`), panes (`*_pane`) and bars (`iron_bars` and the 8 copper bars) fall through to `local_shapes` (`:815`), a full unit box. That box drives `Player::target` (`:1937`, the hand ray), `outline_bounds` (`:757`, the drawn outline and the crack overlay, `render_anim/src/minecraft_mining.rs:170`).
- `supports_rigid_top` (`:1016`) tests only `iron_bars` and `_pane`, so copper bars count as rigid.
- E2 left this for later (`docs/round10/track-e2-log.md:475`).

**Vanilla 26.3.**
- `IronBarsBlock` (panes extend it) is `CrossCollisionBlock(2, 16, 2, 16, 16)`. Its outline is `makeShapes`: post `Block.column(2, 0, 16)` (7/16 to 9/16) and, per connected side, `Block.boxZ(2, 0, 16, 0, 8)` turned by `Shapes.rotateHorizontal`.
- `WallBlock`'s constructor calls `makeShapes(16, 14)` for the outline. Post `Block.column(8, 0, 16)` when `up=true`. Low side `Block.boxZ(6, 0, 14, 0, 11)`, tall side `Block.boxZ(6, 0, 16, 0, 11)` (north: x 5-11, z 0-11), turned per side by `none`/`low`/`tall`.

**Plan: "Minecraft walls, panes and bars are aimed at by their posts and arms like vanilla"**
- `cross_outline(block)` (post plus the `true` sides, 2 wide) and `wall_outline(block)` (post when `up`, plus low or tall sides), as `fence_outline` is written, next to it.
- Two arms in `outline_shapes` next to the `_fence` arm (`:744`): `_pane` or `_bars`, and `_wall`. Check that `_wall` meets no other id: wall torches, signs, banners, fans and heads are matched earlier or end differently.
- `supports_rigid_top` (`:1016`) tests `ends_with("_bars")` in place of `== "iron_bars"`.
- Collision stays unchanged.

**Tests.**
- `player::tests::panes_and_bars_are_outlined_by_their_post_and_arms_and_the_ray_passes_beside_them`: a lone pane is the 2-pixel post; an east-west pane; copper bars with all four arms; a ray beside a lone pane reaches the stone behind.
- `player::tests::a_wall_is_outlined_by_its_post_and_low_or_tall_sides`: a lone post; a straight north-south wall with `up=false` and low sides, whose bounds are 5-11 by 0-14 by 0-16; tall sides reach 16.
- `player::tests::copper_bars_do_not_carry_a_rigid_block`.

**Owner's steps** (survival, by day):
- **O-1.** Place a stone block and a glass pane 1 block in front of it. Aim just beside the pane's thin middle post at the stone: the stone gets the outline and is mined, not the pane. **Screenshot blk-5.**
- **O-2.** Place a row of 3 panes: the middle one's outline is a thin full-height strip, not a cube. Do the same with iron bars and with copper bars.
- **O-3.** A single cobblestone wall's outline is its post, half a block wide. In a row of 3 walls, the middle one's outline is the lower straight section, a little below a full block and 6 pixels thick. Put a torch on it: the post and its outline come back. **Screenshot blk-6.**
- **O-4.** Mine the middle wall with your fist: the cracks cover only its outline.

```
IW4L_MINECRAFT_KIT=minecraft:glass_pane*16,minecraft:red_stained_glass_pane*16,minecraft:iron_bars*16,minecraft:copper_bars*8,minecraft:cobblestone_wall*16,minecraft:mossy_cobblestone_wall*8,minecraft:oak_fence*8,minecraft:stone*64,minecraft:torch*16,minecraft:diamond_pickaxe
```
10 stacks.

---

### Group `plate-bodies` (M), two commits

**Gaps:** doors-6, and the plate click that vanilla's `checkPressed` plays (the fork plays none for any presser).

**Cause.**
- `Level::entity_inside` (`third_party/minecraftoss/world/src/level/entity.rs:1294-1300`) presses plates only for the level's own entities (items, orbs).
- `plate_signal_strength` (`:1345-1373`) counts only `self.entities_in(..)`, so players and the entity crate's mobs never count.
- The scheduled release (`world/src/level/redstone.rs:833-837`) uses the same count.
- `plate_check_pressed` (`:1375-1397`) already posts `block_activate` and `block_deactivate` with a source, but plays no sound. No path plays `*_pressure_plate.click_on/off`, and `crates/minecraft_terrain/src/doors.rs:453-468` `release_sound` covers only buttons.
- The level already takes per-tick player data (`level/mod.rs:183-188` `players`, `living_players`, filled at `crates/minecraft_terrain/src/server.rs:5084`).
- W left players and mobs on plates out (`track-w-log.md:238`, `release-4-tests.md:786`).
- The player crate's `update_pressure_plate_entities` (`player/src/redstone.rs:238`) is test-only. Leave it.

**Vanilla 26.3.**
- `BasePressurePlateBlock.entityInside`: when `getSignalForState(state) == 0`, call `checkPressed(entity, ..)`.
- `getEntityCount(level, TOUCH_AABB.move(pos), class)` uses `EntitySelector.NO_SPECTATORS` and `!isIgnoringBlockTriggers`. `PressurePlateBlock.getSignalStrength` counts `Entity` (wood) or `LivingEntity` (stone, polished blackstone) by `BlockSetType.pressurePlateSensitivity`. `WeightedPressurePlateBlock` counts all entities up to its max.
- `checkPressed` sets the state with flag 2, updates the neighbours, then plays `BlockSetType.pressurePlateClickOn` or `pressurePlateClickOff` (`Level.playSound(null, pos, sound, BLOCKS)`, volume 1, pitch 1) and posts `BLOCK_ACTIVATE` or `BLOCK_DEACTIVATE` with the entity.
- `BlockSetType.<clinit>`:
  - `block.metal_pressure_plate.*`: iron, gold, copper;
  - `block.stone_pressure_plate.*`: stone, polished blackstone;
  - `block.cherry_wood_pressure_plate.*`, `block.bamboo_wood_pressure_plate.*`;
  - `block.nether_wood_pressure_plate.*`: crimson, warped;
  - `block.wooden_pressure_plate.*`: every other wood (pale oak and poplar too).

**Plan.**
1. **"Minecraft players and mobs press pressure plates"**
   - `level/mod.rs`: `pub bodies: Vec<entity::LevelBody>` right after `living_players` (`:188`), set up empty with it. `pub struct LevelBody { pub source: game_event::EventSource, pub min: [f64; 3], pub max: [f64; 3] }` goes next to the plate functions in `entity.rs`.
   - `entity.rs`:
     - a new `fn bodies_on_plates(&mut self)` beside `entity_inside`. For every body, it visits the blocks under its box shrunk by `1.0e-5` (as `check_inside_blocks` does at `:1243`). A `BasePressurePlateBlock` with `plate_signal == 0` gets `plate_check_pressed(pos, state, 0, Some(body.source))`;
     - one call line at the end of `tick_entities` (`:602`);
     - `plate_signal_strength` adds the bodies whose box meets `touch`. Every body is living, so stone plates count them too.
   - `server.rs` `run_tick`, the line after `:5084`: `sim.level.bodies` gets:
     - the alive, non-spectator `input.mob_players` (box 0.6 wide and 1.8 high from `position`, `EventSource::Player(id)`);
     - every `sim.mobs.mob_ids()` with `body_of(id)` (`world/leashes.rs:111`, its width and height, `EventSource::Mob(id)`).
   - The source makes W's sensors hear a player stepping on a plate (`block_activate`, frequency 10) through the existing level game event drain.
2. **"Minecraft pressure plates click when pressed and released"**
   - `doors.rs` `release_sound` (`:453`) also answers a `*_pressure_plate` whose `powered` or `power` goes from 0 to above 0 (`click_on`) or back (`click_off`). Volume 1, pitch 1, with the set from a `plate_set`: stone and polished blackstone give `stone`, the two weighted plates `metal`, otherwise `wood_set` (`:291`).
   - Both callers already play it for the host and its levels (`render_anim/src/minecraft_world.rs:1213`, `minecraft_levels.rs:494`), as they do for a button's click-off. Item presses click too now.

**Tests.**
- `minecraft_terrain` `server::tests`, gated on the 26.3 data with `door_world()` and `place()` (`server.rs:7059`, `:7184`), through `run_tick` or `run_ticks`:
  - `a_player_on_a_stone_plate_lights_the_lamp_beside_it_and_it_springs_back_20_ticks_after_they_leave`;
  - `a_stone_plate_ignores_a_dropped_item_but_a_cow_presses_it`;
  - `a_spectator_presses_no_plate`;
  - `a_light_weighted_plate_counts_players_mobs_and_items_together` (1 player gives 1; a player and 4 item entities of different items give 5);
  - `stepping_on_a_plate_posts_block_activate_with_the_player_as_source` (the level's game events, `EventSource::Player`).
- `doors::tests::pressure_plates_click_on_and_off_with_their_block_set_sound` (oak, cherry, crimson, stone, polished blackstone, light and heavy weighted; a power change from 3 to 5 gives no click).

**Owner's steps** (survival, by day, on a stone floor you place):
- **P-1.**
  1. Place a stone pressure plate with a redstone lamp right beside it. Walk onto the plate: it clicks and the lamp lights. **Screenshot blk-7.**
  2. Step off: about a second later it clicks again and the lamp goes out. Stand still on it: the lamp stays lit.
- **P-2.** Look at a second stone plate with a lamp and drop one cooked beef on it (Q): nothing, because stone plates feel only players and mobs. Pick the beef up. An oak plate with a lamp: the beef presses it, and so do you. A cherry plate sounds different.
- **P-3.** Hatch a cow and lead it onto the stone plate with wheat: the lamp lights while it stands there. **Screenshot blk-8.**
- **P-4.** Set an iron door in a 1-block wall with a stone plate on each side. Walk onto the plate: the door opens; step off: it shuts. `/difficulty hard`, `/time set night`, hatch a zombie on the far side: when it walks onto the plate, the iron door opens for it. **Screenshot blk-9.** Kill it, then `/time set day`.
- **P-5.** Run a line of 15 redstone dust out of a light weighted (gold) plate. Stand on the plate: only the first dust glows (strength 1). While you stand on it, drop 4 different items on the plate, one each of cooked beef, stone, wheat and a torch: the first 5 dust glow. The plate counts entities, and 4 of the same item dropped together merge into one stack entity (strength 2).
- **P-6.** `/gamemode spectator` and fly through the plate: nothing. `/gamemode survival`.
- **P-MP (two windows).** F walks onto the plate: H sees the lamp light and hears the click. H stands on it: F sees the lamp.

```
IW4L_MINECRAFT_KIT=minecraft:stone_pressure_plate*4,minecraft:oak_pressure_plate*2,minecraft:cherry_pressure_plate*2,minecraft:polished_blackstone_pressure_plate*2,minecraft:light_weighted_pressure_plate*2,minecraft:heavy_weighted_pressure_plate*2,minecraft:redstone_lamp*8,minecraft:redstone*64,minecraft:iron_door*2,minecraft:cow_spawn_egg*4,minecraft:wheat*16,minecraft:zombie_spawn_egg*4,minecraft:cooked_beef*32,minecraft:stone*64,minecraft:iron_sword,minecraft:torch*16
```
16 stacks.

---

### Group `dispenser-blocks` (M), three commits

**Gaps:** doors-12, doors-10, doors-11.

**Cause.**
- `Level::dispense_from` (`world/src/level/dispenser.rs:73-131`) pushes no level event. The empty branch (`:78-84`) posts only `block_activate`. The dropper branch (`:86-104`), `default_dispense` (`:134-142`), the buckets, honeycomb and bone meal (`:114-123`) play nothing. Only `dispense_rocket` (`:348-357`) plays its shoot sound.
- `crates/minecraft_terrain/src/level_events.rs:113-256` has no arm for 1000, 1001 or 2000.
- `carved_pumpkin` and `wither_skeleton_skull` are in `OTHER_BEHAVIOURS` (`:21`, `:25`), so they are unsupported and dispense nothing (`:124-127`).
- The builders exist. A carved pumpkin placed in the level builds a golem through `on_place` (`level/mod.rs:724-725` calls `golem_build.rs:191-199`). `Level::wither_skull_placed` (`wither_build.rs:130-170`) is called only by hand placement (`crates/minecraft_terrain/src/server.rs:2882`).
- The level's level events already reach the host and friends (`server.rs:5178`, `Output.level_events`).
- **Nobody can load a dispenser in game yet:** there is no dispenser, dropper or hopper screen (`crates/minecraft_survival/src/menus.rs:1976-1981`, `docs/round7/track-d-log.md:357`) and no `/item` command. The steps therefore use a world prepared in vanilla 26.3.

**Vanilla 26.3.**
- `DefaultDispenseItemBehavior.dispense` = `execute`, `playSound` (levelEvent 1000), `playAnimation` (levelEvent 2000, data `direction.get3DDataValue()`, which is the fork's `Direction::index`).
- `OptionalDispenseItemBehavior.playSound` gives 1000 on success and 1001 on failure, and still plays the animation.
- `DispenserBlock.dispenseFrom` and `DropperBlock.dispenseFrom` with no item: levelEvent 1001 (the dispenser also posts `BLOCK_ACTIVATE`) and no smoke. A dropper putting into a container (`HopperBlockEntity.addItem`) plays nothing.
- The behaviour classes (`DispenseItemBehavior.bootStrap`):
  - Default: the filled buckets (`$3`) and the empty bucket (`$4`).
  - Optional: bone meal `$5` (on growth also `levelEvent(1505, target, 15)`), the wither skull `$7`, the carved pumpkin `$8`, glass bottle `$9`, glowstone `$10`, honeycomb `$12`, shears, flint and steel.
  - Projectile: firework rockets, overridden to 1004 (the fork's own shoot sound), with the smoke.
- `LevelEventHandler.levelEvent`:
  - 1000: `block.dispenser.dispense`, BLOCKS, 1.0, 1.0;
  - 1001: `block.dispenser.fail`, 1.0, 1.2;
  - 2000: `shootParticles(data, pos, random, SMOKE)`. 10 times: `r = nextDouble*0.2+0.01`; position `pos + 0.5 + step*0.6 + step*0.01`, plus jitter `(nextDouble-0.5)*0.5` times stepZ on x, stepY on y and stepX on z, drawn in that order; motion `step*r + nextGaussian*0.01` per axis.
- `DispenseItemBehavior$8` (carved pumpkin): `target = pos.relative(FACING)`. If `isEmptyBlock(target)` and `CarvedPumpkinBlock.canSpawnGolem(level, target)` (snow base `" ","#","#"`, iron base `"~ ~","###","~#~"`, or the copper base, through `BlockPattern.find`): `setBlockAndUpdate(target, carved_pumpkin)` (whose `onPlace` builds the golem), `gameEvent(null, BLOCK_PLACE, target)`, shrink, success. Otherwise `EquipmentDispenseItemBehavior.dispenseEquipment`.
- `DispenseItemBehavior$7` (wither skull):
  - The condition: `isEmptyBlock(target)` and `WitherSkullBlock.canSpawnMob(level, target, stack)`, which needs the skull item, `target.y >= minY + 2`, not peaceful, and the base `"   ","###","~#~"` found.
  - Then: `setBlock(target, wither_skeleton_skull[rotation = RotationSegment.convertToSegment(facing)], 3)`. A vertical facing gives 0 (`SegmentedAnglePrecision.fromDirection`); horizontal ones give south 0, west 4, north 8, east 12.
  - Then `gameEvent BLOCK_PLACE`, `WitherSkullBlock.checkSpawn`, shrink. Otherwise the equipment fallback.

**Plan.**
1. **"Minecraft dispensers and droppers click, fail and puff smoke like vanilla"**
   - `dispenser.rs`: `pub const DISPENSE_EVENT: i32 = 1000`, `FAIL_EVENT = 1001`, `SMOKE_EVENT = 2000` and `BONE_MEAL_EVENT = 1505` next to `ROCKET_SHOOT_SOUND` (`:28`).
   - The empty branch (`:78-84`) pushes `(FAIL_EVENT, pos, 0)` for both blocks.
   - The dropper's no-container path pushes 1000 and the smoke.
   - `let count = stack.count;` before the match (`:109`). One call `self.dispense_effects(pos, &name, count, &result)` between the match's end and `container_set_item` (`:130`), so no arm line M1 or M2 anchors on changes. `dispense_effects`:
     - `firework_rocket` gets smoke only;
     - the optional names (`bone_meal`, `honeycomb`, `carved_pumpkin`, `wither_skeleton_skull`, plus `glass_bottle`, `glowstone`, `shears` and `flint_and_steel` for when they are ported) get 1000 when the stack shrank or changed, else 1001;
     - everything else gets 1000;
     - then `(SMOKE_EVENT, pos, facing.index())`.
     Unsupported items still return early and play nothing.
   - The bone meal arm (`:114-123`) pushes `(BONE_MEAL_EVENT, target, 15)` when something grew.
   - `level_events.rs`: `DISPENSER_DISPENSE`, `DISPENSER_FAIL` and `DISPENSER_SMOKE` taken from the world crate's numbers, as `ZOMBIE_ATTACK_WOODEN_DOOR` takes `door_goals`'. Their arms go right after the `LAVA_FIZZ` arm. The smoke uses a `shoot_smoke(base, data, random, &mut out)` with `Kind::Smoke` exactly as above.
2. **"Minecraft dispensers put carved pumpkins on golem bodies"**
   - `golem_build.rs`: `SNOW_GOLEM_BASE` and `IRON_GOLEM_BASE` (same kind and body, the head cell `' '`) after `GOLEMS` (`:34`), and `pub fn can_spawn_golem(pos, name_at) -> bool` beside `find` (`:137`). The copper base is M3's: if M3's `copper_golem_build` is on `minecraft-survival`, add its base as one line, otherwise log it for M3.
   - `dispenser.rs`: an arm `"carved_pumpkin" => self.dispense_pumpkin(pos, stack)` right after the bone meal arm. The earlier arm wins over `OTHER_BEHAVIOURS`; leave those lines.
   - `dispense_pumpkin`: air in front and `can_spawn_golem(target, &|at| self.name_if_loaded(at))` lead to `set_block_and_update(target, carved_pumpkin)`, then `block_game_event_from("minecraft:block_place", target, None, None)` (vanilla's site, one line), then shrink. Otherwise the stack stays unchanged (fail click from commit 1).
3. **"Minecraft dispensers place wither skulls and build withers"**
   - `wither_build.rs`: `BASE_ROWS = ["   ", "###", "~#~"]` next to `ROWS` (`:11`); `matches_at` (`:39`) takes the rows; `pub fn find_base(origin, cell_at)` beside `find` (`:62`); `wither_cell` (`:111`) becomes `pub(super)`.
   - `dispenser.rs`: an arm `"wither_skeleton_skull" => self.dispense_skull(pos, stack)` next to the pumpkin arm.
   - `dispense_skull`: air in front, `target.y >= min_y + 2`, `difficulty != 0` and `find_base` lead to: set the skull with `rotation` as above, the `block_place` game event line, `self.wither_skull_placed(target)`, then shrink.

**Tests.**
- `minecraft_terrain` `server::tests`, gated, with the dispenser setup of `a_dispenser_shoots_a_rocket_level_..` (`replace_block_item`, a redstone block beside):
  - `an_empty_dispenser_or_dropper_fails_with_1001_and_no_smoke`;
  - `a_dispenser_throwing_cobblestone_clicks_and_smokes_along_its_facing` (1000, then 2000 with data 5 facing east);
  - `a_dropper_into_a_chest_plays_nothing`;
  - `dispensed_bone_meal_sparkles_on_grass_and_fails_on_stone` (1505 with data 15 and 1000; 1001 and the meal kept);
  - `a_dispensed_rocket_smokes_and_keeps_its_own_shoot_sound`;
  - `a_dispenser_puts_a_pumpkin_on_two_snow_blocks_and_a_snow_golem_appears`;
  - `a_dispenser_finishes_an_iron_t_with_a_pumpkin`;
  - `a_pumpkin_facing_nothing_stays_in_the_dispenser`;
  - `a_dispenser_adds_the_third_skull_and_a_wither_appears`;
  - `a_skull_is_not_dispensed_on_peaceful_without_a_t_or_too_low`;
  - `the_dispensed_skull_turns_with_the_dispenser` (north gives 8, up gives 0).
- `level_events::tests::dispenser_clicks_fail_and_smoke_like_vanilla` (volumes, pitches, 10 smoke particles 0.61 out of the east face, moving east).
- `golem_build::tests::the_golem_bases_match_without_the_pumpkin`.
- `wither_build::tests::a_soul_sand_t_is_a_base_with_or_without_skulls`.

**Owner's steps.**

**Prepare the world in vanilla Minecraft 26.3** (this build has no dispenser screen):
1. Make a new world `e3-dispensers`: Superflat, Creative, Allow Commands on, Difficulty Normal.
2. Build these 8 stations 6 blocks apart, each with a lever on the dispenser's side, all switched off:
   - **A.** A dispenser on the ground, its hole facing east, with 16 cobblestone.
   - **B.** A dispenser facing east with a grass block in front at the same height, with 16 bone meal.
   - **C.** The same with a stone block in front, with 4 bone meal.
   - **D.** Two snow blocks stacked, and a dispenser facing down two blocks above the top snow block (its hole over the empty spot), with 2 carved pumpkins.
   - **E.** An iron golem T (4 iron blocks: 3 across, 1 below the middle, air under the arms and beside the head spot), with a dispenser facing down just above the empty head spot, holding 1 carved pumpkin.
   - **F.** A dispenser facing east into open air with 2 carved pumpkins.
   - **G.** A soul sand T (3 across on top of 1), a wither skeleton skull on the left and on the right top spot, and a dispenser facing down above the empty middle skull spot, holding 1 wither skeleton skull.
   - **H.** A dispenser facing east into open air with 1 wither skeleton skull.
3. Save and quit. Copy `%APPDATA%\.minecraft\saves\e3-dispensers` into `iw4l-artifacts\saves\`, start it single player in our game, then `/gamemode survival` and `/difficulty normal`.

Steps:
- **D-1** (any world, no preparation needed). Place a dispenser with a lever beside it and flip the lever on: a dry, higher click and no smoke. The same with a dropper.
- **D-2.** A: flip the lever: the normal dispense click, a puff of smoke out of the hole, and a cobblestone flies out east. Off and on again: the same. **Screenshot blk-10.**
- **D-3.** B: click, smoke, green sparkles over the grass, and grass or flowers grow. C: the higher fail click with smoke, and nothing grows.
- **D-4.** D: click, smoke, and the pumpkin lands on the snow and a snow golem appears at once. **Screenshot blk-11.** Flip it again: the fail click, because no snow is left, and the second pumpkin stays.
- **D-5.** E: an iron golem appears. **Screenshot blk-12.**
- **D-6.** F and H: the fail click with smoke; nothing comes out.
- **D-7.** G: the skull lands in the middle and the wither appears and grows. **Screenshot blk-13.** Then `/difficulty peaceful` removes it (`/kill` takes only players until kill-selectors in e3s), and `/difficulty normal`.
- **D-MP (two windows,** F beside H in the prepared world). H flips A: F hears the click and sees the smoke.

```
IW4L_MINECRAFT_KIT=minecraft:dispenser*4,minecraft:dropper*2,minecraft:lever*4,minecraft:cobblestone*64,minecraft:snow_block*4,minecraft:carved_pumpkin*4,minecraft:iron_block*8,minecraft:soul_sand*8,minecraft:wither_skeleton_skull*6,minecraft:diamond_sword,minecraft:golden_apple*16,minecraft:milk_bucket,minecraft:cooked_beef*32,iw4l:m4,iw4l:rifle_ammo*64
```
15 stacks (no bow: players cannot use one in the fork). The snow, iron, soul sand and skulls let you compare with a hand-built golem and wither.

---

### Group `placed-names` (M), three commits

**Gaps:** doors-14 (custom names), doors-15.

**Cause.**
- `crates/minecraft_survival/src/actions.rs:1599-1608` sends a placed stack's components only for banners. Shulker boxes keep their own path (`:1590-1598`, `Containers::place_shulker`, `crates/minecraft_terrain/src/containers.rs:913`).
- Chests, trapped chests, barrels, furnaces and brewing stands: the containers store owns `CustomName` (`OWNED_KEYS`, `containers.rs:41`) and shows it as the screen title (`menus.rs:331-334`). Placement never sets it, and `harvest`'s `kept` (`actions.rs:1763-1781`) carries no name, so a renamed chest loses its name.
- Level-owned block entities (hoppers, dispensers, droppers, enchanting tables, beacons): `components::implicit` (`world/src/level/components.rs:24-29`) and `dropped_components` (`:73-83`) know only banners.
- `harvest` reads the block entity from the stream (`actions.rs:1774-1779`). That copy arrives a tick or two after `Entities::block_entity_components` (`entities.rs:1096`), so a banner broken at once drops plain (`docs/round10/track-e1-log.md:986`).

**Vanilla 26.3.**
- `BlockItem.place` calls `updateCustomBlockEntityTag`, then `updateBlockEntityComponents`, then `BlockEntity.applyComponentsFromItemStack`, all in the placing call.
- `applyImplicitComponents`: `BaseContainerBlockEntity` reads `CUSTOM_NAME`, `LOCK` and `CONTAINER`; `EnchantingTableBlockEntity` reads `CUSTOM_NAME`; `BeaconBlockEntity` reads `CUSTOM_NAME` and `LOCK`.
- The 26.3 block loot tables `copy_components` `minecraft:custom_name` from `block_entity` for chest, trapped chest, barrel, furnace, brewing stand, hopper, dispenser, dropper, enchanting table and beacon (crafter: none).
- `ChestBlock$2$1.getDisplayName`: the first half's custom name, else the second half's, else `container.chestDouble`.

**Plan.**
1. **"Named Minecraft chests, barrels, furnaces and brewing stands keep their names when placed and broken"**
   - `containers.rs`, beside `place_shulker`:
     - `pub fn name_placed(&mut self, pos, stack)` sets or clears `names` from `minecraft:custom_name`. It makes the store entry its kind needs so `block_entity_tag` (`:797`) exports the name (`block_entity_tag` returns `None` without one), and marks it dirty;
     - `pub fn name_value(&self, pos) -> Option<&Value>`.
   - `actions.rs`: after the shulker branch, a branch for `Station::of` Chest, Barrel, Oven or Brewer.
   - In `harvest`, `kept` chains `{custom_name}` for those stations. It runs before `drop_blocks` (`:1797`) and `spill` (`:1800`), which drops the name.
   - `menus.rs:331`: the first cell's name, then the second's.
2. **"Named Minecraft hoppers, dispensers, droppers, enchanting tables and beacons keep their names"**
   - `implicit` gives `(CUSTOM_NAME, "CustomName")` for `minecraft:hopper`, `dispenser`, `dropper`, `crafter`, `enchanting_table` and `beacon`.
   - `dropped_components` gives `[CUSTOM_NAME]` for those minus the crafter.
   - The `is_banner` pair at `actions.rs:1599-1608` becomes a `components::takes_components(id)` (banners and these kinds).
3. **"A Minecraft banner broken right after it is placed keeps its patterns"**
   - `Entities` (`entities.rs:63`) gets `placed_components: HashMap<BlockPos, serde_json::Value>`, set in `block_entity_components`.
   - Any placement at a pos drops that pos's entry first (the write path at `actions.rs:1588`).
   - `harvest` uses the entry when `dropped_components(stored)` gives none, filtered by the kind's loot list (a `components::dropped_from_item(block_id, json)`), and removes it either way.

**Decisions (for the log).**
- Lock, container contents on chests and barrels, and `block_entity_data` are not applied (see `left`).
- A pending entry lives from its placement to the next placement or break at that block.

**Tests.**
- `containers::tests::a_named_chest_keeps_its_name_in_the_title_and_the_saved_tag`.
- `menus::tests::a_double_chest_shows_the_name_of_either_half`.
- `actions::tests::breaking_a_named_barrel_drops_it_with_its_name`.
- `components::tests::hoppers_dispensers_and_beacons_take_and_drop_a_custom_name` (pure `applied` and `dropped_components`; a crafter drops none).
- `actions::tests::a_banner_broken_before_the_server_answers_keeps_its_patterns`.
- `actions::tests::a_plain_banner_placed_where_a_patterned_one_was_drops_plain`.

**Owner's steps** (survival, by day):
- **N-1.**
  1. `/experience add @s 30 levels`. Place the anvil and rename a chest to `Tools`.
  2. Place it and open it: the title reads `Tools`. **Screenshot blk-14.**
  3. Break it: the dropped chest is still named `Tools` (hover it in the inventory). Place it again: still `Tools`.
- **N-2.** Do the same with a barrel (`Food`), a furnace (`Oven`), a smoker, a blast furnace, a brewing stand and a trapped chest. Place an unnamed chest beside the named one: the double chest opens as `Tools`.
- **N-3.** Rename a hopper `Feed`, a dispenser `Gun` and an enchanting table `Magic`. Place them and break them: each drops with its name. They have no screens in this build.
- **N-4.** Save and quit, and load: `Tools` still opens as `Tools`, and the hopper still drops as `Feed`.
- **N-5.** Take the striped banners and the Efficiency V diamond axe (it breaks a banner instantly). Five times: place a banner and break it at once. Every dropped banner keeps its red stripe. **Screenshot blk-15** (the hotbar tooltip).
- **N-MP (two windows).** F renames and places a chest; H opens it: the title shows F's name.

```
IW4L_MINECRAFT_KIT=minecraft:anvil,minecraft:chest*4,minecraft:trapped_chest*2,minecraft:barrel*2,minecraft:furnace*2,minecraft:smoker,minecraft:blast_furnace,minecraft:brewing_stand,minecraft:hopper*2,minecraft:dispenser*2,minecraft:dropper*2,minecraft:enchanting_table,minecraft:white_banner[banner_patterns=[{pattern:"minecraft:stripe_top",color:"red"}]]*8,minecraft:white_banner*4,minecraft:diamond_axe+efficiency=5,minecraft:diamond_pickaxe+efficiency=5,minecraft:cobblestone*64,minecraft:cooked_beef*16
```
19 stacks.

---

### Group `door-cracks` (S), one commit

**Gaps:** doors-4.

**Cause.**
- `EntityWorld::door_cracks` (`third_party/minecraftoss/entities/src/door_goals.rs:309-321`) lists only live mobs (health above 0) with a `door.crack`.
- Friends get diffs from it (`crates/minecraft_survival/src/session.rs:539-560`, -1 when an entry goes) and the host draws it (`crates/render_anim/src/minecraft_world.rs:1567`), so a crack vanishes the tick its mob dies or is removed.
- A stopping goal clears it (`door_goals.rs:227-229`), which is vanilla's -1 in `BreakDoorGoal.stop`.
- This was E2's decision 3 (`track-e.md` E-doors), listed in `release-3-tests.md:664`.

**Vanilla 26.3.**
- A dying mob is `isImmobile` (`LivingEntity.isDeadOrDying`), so neither `serverAiStep` nor `BreakDoorGoal.stop` runs, and no -1 is sent.
- `ClientLevel.removeBlockBreakingProgress` runs when `getGameTime() % 20 == 0`. It drops the `destroyingBlocks` entries whose `getUpdatedRenderTick` is more than 400 ticks old.
- A new progress from the same breaker id replaces the old one.

**Plan: "A Minecraft door crack stays up to 20 seconds after its mob dies like vanilla"**
- `door_goals.rs`:
  - `pub struct KeptCrack { pub mob: u64, pub pos: Pos, pub stage: u8, pub since: i64 }`.
  - `pub fn keep_door_cracks(&mut self)` on `EntityWorld`, with `self.game_time`:
    - each live crack updates or adds its mob's entry, and `since` restarts when the pos or stage changed;
    - an entry whose mob is alive but has no crack goes (stop);
    - an entry whose mob is dead or gone stays until a tick with `game_time % 20 == 0` finds it more than 400 ticks old.
  - `door_cracks()` returns the kept entries.
- Vendored `world.rs`: the field `kept_cracks` next to `level_events` (`:2598`), its default (`:2739`), and copied in `clone_where` (`:5826`), so the host's view and friends see it.
- `crates/minecraft_terrain/src/server.rs:1455`: `self.mobs.keep_door_cracks();` after the level event drain.
- `session.rs` and `render_anim` need no change.

**Tests.**
- `door_goals::tests::a_killed_breakers_crack_stays_until_400_ticks_after_its_last_change` (still there at 400; gone at the next multiple of 20).
- `door_goals::tests::a_stopped_break_still_clears_its_crack_at_once`.
- `door_goals::tests::a_removed_mobs_crack_is_kept_too`.
- `session::tests::a_friend_keeps_an_orphaned_crack_until_it_expires` (next to `a_friend_sees_a_zombies_door_crack_grow_and_clear`, `session.rs:3109`).

**Owner's steps** (release-3's door setup, `release-3-tests.md` "Zombies (in creative)": in creative the zombie leaves you alone and goes for the villager, so its path runs through the door):
- **Setup.** `/gamemode creative`, `/difficulty hard`, `/time set night`. Build a closed room of oak planks, 5 by 5 inside and 3 high, with a roof and one oak door in the middle of one wall. Inside, put 8 oak fences in a ring round the middle block and use a villager egg on that block. Open the door and stand just outside it with your back to it, looking straight away from the house.
- **K-1.**
  1. Run `/summon minecraft:zombie ^ ^ ^12 {CanBreakDoors:1b,PersistenceRequired:1b}`, 12 blocks ahead of you. It walks toward the villager: turn round and close the door before it gets there. If it walks in first, kill it, open the door and try again.
  2. When the cracks reach about half way, kill the zombie with the sword. The cracks stay on the door. **Screenshot blk-16**, just after it dies.
  3. They vanish about 20 seconds after the last crack grew. The door is not broken.
- **K-2.** Repeat. When the cracks show, `/difficulty normal`: they vanish at once, as before. `/difficulty hard`.
- **K-MP (two windows).** Repeat K-1 with F in creative too (`/gamemode creative Friend`), within 30 blocks: F sees the cracks stay after the kill and vanish with H's.

```
IW4L_MINECRAFT_KIT=minecraft:zombie_spawn_egg*4,minecraft:villager_spawn_egg*2,minecraft:oak_fence*16,minecraft:oak_door*4,minecraft:oak_planks*64,minecraft:oak_planks*64,minecraft:glass*16,minecraft:torch*16,minecraft:diamond_sword,minecraft:shield,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*8,minecraft:cooked_beef*16
```
16 stacks. If the villager dies or turns into a zombie villager, use a new villager egg.

### Files shared

- Vendored `player/src/lib.rs`: `place_target_with` (`:2104`) and `outline_shapes`. M1 owns the `World` trait lines `:98-238`.
- Vendored `world/src/level/mod.rs`: one field after `living_players`. M3 adds fields after `level_events` and `destroyed`.
- `entity.rs`: plate functions only.
- `dispenser.rs`: new arms after bone meal and one call before `container_set_item`. M1 and M2 anchor `:110` and `:112`.
- `golem_build.rs`: new constants and a function. M3 owns `:191-199`.
- `crates/minecraft_terrain/src/server.rs`: one line each at `:1455` and after `:5084`.
- `level_events.rs`: arms after `LAVA_FIZZ`.
- `doors.rs`: `release_sound`.
- `actions.rs`, `containers.rs`, `menus.rs`, `entities.rs`, `components.rs`.
- Vendored `door_goals.rs` and `world.rs`: one field, its default and its `clone_where` line.

## E3-9. Fireworks, projectiles and bells

The open gaps of this area come from E2's logs (`fw-burst`, `fw-launch`, `fw-elytra`, `bell-hits`, `ti4-end-targets`) and the known limits of the release test files. Segment e2b already brought firework shapes and crafting, elytra rockets, bells rung by projectiles, and snowballs and tridents hitting end crystals and the dragon. Everything below builds on it.

**Base.** Every file:line anchor is at **d565a3d2**, the e2b merge. It was read in `C:\Dev\Soscraft\mc-wt\track-e2` and reaches `minecraft-survival` by fast-forward before E3 starts. d0b and W's w3 merge at about the same time, so check each anchor again before editing. Vanilla facts were checked with javap on the 26.3 `client.jar` and against its data (`tags/damage_type`, `tags/entity_type`, `loot_table/blocks`).

| # | Group | Size | Gaps | Waits for |
|---|---|---|---|---|
| 1 | `registered-hits` | S | projectiles-8, -9 (damage type only), -10 | nothing |
| 2 | `rocket-end-targets` | S | projectiles-3 | W's w3 merged (segment e3k: w3's events-items edits `tick_rockets` and the rocket hits) |
| 3 | `elytra-rockets` | M | projectiles-12, -13 | W's w3 merged (segment e3k, the `Exploded` arm) |
| 4 | `angled-rockets` | M | projectiles-6 | nothing |
| 5 | `target-blocks` | M | projectiles-15 | W's w3 merged (it changes `ServerSim::projectile_hit_block`) |
| 6 | `projectile-breaks` | M | projectiles-16, -17 | `target-blocks`; W's w3; E3-12's `decorated-pots` (segment e3q, so a placed pot stores its sides) |

**Rules for this area**
- No group bumps `minecraft_net::PROTOCOL`. Only `angled-rockets` changes a wire struct, and it says so under "## Protocol" in the log.
- **M1's hook.** `elytra-rockets` adds a field to `PlayerControls` and the survival `PlayerView`. M1's `armadillos-live` (segment m1e) adds `sprinting` to the same structs next to `sneaking`. E3 puts `gliding` after `jumping` and never touches `sprinting`. The only shared lines are the one-line test literals at `crates/minecraft_survival/src/entities.rs:2578` and `:2668`; whichever track merges second keeps both fields.
- **M3's file.** M3 owns `world/equines.rs`. This area changes two single lines there, both in the llama spit code and away from the horse lines: `:724` (`registered-hits`) and `:706` (`target-blocks`). E3-0 lists every E3 line in M3's horse code.
- **M2's knockback hook.** `registered-hits` edits the family arms of `hurt_for_player` and `hurt_by_typed_blast`, where M2's m2e puts its family knockback hook (E3-0). Keep both sides at a merge.
- **Game events.** W is done after w3, so E3 adds the one game event its new block reactions need (`block_destroy` in `projectile-breaks`) itself. It goes through W's helpers, one line per site, as in W0 rule 1.

---

### Group `registered-hits`

**Gaps:**
- **projectiles-8:** a breeze is hurt by llama spit instead of letting it pass. No test pins that spit reaches registered mobs at all.
- **projectiles-9:** registered mobs hit by a player's thrown thing see `minecraft:player_attack`.
- **projectiles-10:** registered mobs caught in a firework burst see `minecraft:explosion` or `minecraft:player_explosion`.

**Cause**
- **Thrown hits (9).** In `third_party/minecraftoss/entities/src/world/combat.rs:494`, the family arm of `hurt_for_player` (`:357`) builds `families::Hurt::new(amount, "minecraft:player_attack", source)` and ignores the `kind` it is given.
  - `player_projectile_hurt` (`:340-355`) passes the projectile's type (`ProjectileHit.kind`: `minecraft:thrown`, `minecraft:wind_charge` or `minecraft:trident`), so only the old stores see it.
- **Firework bursts (10).** In `entities/src/world.rs:3078`, the family arm of `hurt_by_typed_blast` (`:3039`) builds its `Hurt` with `blaster.damage_type()` instead of `kind`. `firework_blast` (`:3109`) passes `minecraft:fireworks` (`:3117`), but only the old stores get it.
- **Llama spit (8).** In `world/equines.rs:651-727` (`tick_llama_spit`), the targets have come from `self.order` since 9eecb2ff, and `mob_body` covers the families, so spit does reach silverfish, breezes, rabbits and Margit.
  - A struck mob always goes to `mob_shot_mob` (`:724-726`, then `world.rs:3977`, then the `mob_hits_mob` family arm at `:4402`).
  - Nothing asks `families.deflect` here, the way arrows (`world.rs:6837`), fireballs (`world/fireballs.rs:227`) and shulker bullets (`world/shulker_bullets.rs:264`) do.
- **Families that read the damage type today:**
  - rabbits' and polar bears' `last_damage` (`families/land/rabbits.rs:538`, `land/polar_bears.rs:371`);
  - wardens (`deep_dark/wardens.rs:701`);
  - silverfish (`pests/silverfish.rs:231`);
  - Margit (`bosses/elden_bosses.rs:215`).

**Vanilla 26.3**
- **What a hit records.** `LivingEntity.hurtServer` stores `lastDamageSource` for every hit (javap: `putfield lastDamageSource`).
  - Its `resolveMobResponsibleForDamage` skips only `setLastHurtByMob`, and only for `#no_anger` (`mob_attack_no_aggro`) or a `wind_charge` source on `#no_anger_from_wind_charge`.
  - `resolvePlayerResponsibleForDamage` credits the player either way.
- **The anger half of projectiles-9 is not a gap.**
  - `HurtBySensor.doTick` puts `getLastDamageSource()` into `HURT_BY`.
  - `BreezeAi.initIdleActivity` has a second `StartAttacking`, whose target is the living entity of that `HURT_BY` source (`lambda$initIdleActivity$1`).
  - So in vanilla a player's wind charge does turn a breeze on the player. The tag only matters for `HurtByTargetGoal` mobs (skeletons, zombies, spiders, slimes), which are old stores that `player_projectile_hurt`'s `angers` already handles.
  - The fork's breeze does the same through `last_hurt` (`trial/breezes.rs:413`, read at `breezes/brain.rs:364-368`).
  - E2's merge note in decisions.md ("skip its anger on wind charges") is wrong. Keep the attacker.
- **Damage types:**
  - `Snowball`, `ThrownEgg` and `ThrownEnderpearl` use `thrown`;
  - `AbstractWindCharge` uses `wind_charge`;
  - `ThrownTrident` uses `trident`;
  - `FireworkRocketEntity.dealExplosionDamage` uses `DamageSources.fireworks` (`minecraft:fireworks`).
- **Spit passes through a breeze.**
  - `LlamaSpit.tick` (javap) takes `movement = getDeltaMovement()`, runs `hitTargetOrDeflectSelf(hit)`, then sets `setPos(pos + movement)` and `setDeltaMovement(movement.scale(0.99))` plus gravity. Both use the movement from before the hit test.
  - `Projectile.hitTargetOrDeflectSelf`: an entity whose `deflection(projectile)` is not `NONE` gets no `onHit`. It runs `deflect` only when that entity is not already `lastDeflectedBy`.
  - `Breeze.deflection` is `NONE` for `wind_charge` and `breeze_wind_charge`. For anything else it is `PROJECTILE_DEFLECTION` on `#deflects_projectiles` (the breeze): the `entity.breeze.deflect` sound, then `ProjectileDeflection.REVERSE` (yaw + 170 to 190, motion × -0.5).
  - For spit, `tick` overwrites that motion at once. So **the spit flies on through the breeze, the breeze takes nothing, and it whooshes once.**
  - Without a deflection, `LlamaSpit.onHitEntity` hurts with `DamageSources.spit(spit, llama)` for 1.0 and does not discard. Only `onHitBlock` does, so the fork's spit already flies on after a hit.

**Plan: two commits**
1. **"Registered Minecraft mobs learn which thrown thing or firework hurt them"**
   - `combat.rs:494`: `"minecraft:player_attack"` becomes `kind`. Melee still passes `minecraft:player_attack` (`:330`). The `.by(Some(Target::Player(player)), Some("minecraft:player"))` stays as it is.
   - `world.rs:3078`: `blaster.damage_type()` becomes `kind`. `hurt_by_blast` (`:3035`) passes `blaster.damage_type()` as `kind`, so TNT and creeper blasts keep theirs.
   - One changed line in each of D0's generic family arms.
2. **"Breezes let llama spit fly through them unhurt"**
   - `entities/src/llama_spit.rs:28`: `LlamaSpit` gains `deflected_by: Option<u64>` (vanilla `lastDeflectedBy`). It is `None` in `spit()` (`:72`) and in the two other literals: the load at `crates/minecraft_terrain/src/server_mobs.rs:1189` and the test at `crates/minecraft_survival/src/entity_view.rs:1843`.
   - `world.rs`, next to `mob_shot_mob` (`:3977`): `pub(super) fn spit_deflected(&mut self, spit: u64, target: u64) -> bool`. It returns true when `target` already turned this spit. Otherwise, when `self.families.deflect(target, "minecraft:llama_spit")` holds (which queues the breeze's deflect voice), it remembers `target` and returns true. The spit's position and motion stay as they are.
   - `equines.rs:724`: `} else {` becomes `} else if !self.spit_deflected(id, target) {`. This is the only line in M3's `equines.rs`. Players never deflect, because `target >= PLAYER_TARGET` keeps its own branch.

**Tests**
- Vendored `world::combat::tests::a_registered_mob_learns_a_snowball_a_trident_and_a_firework_burst`: a rabbit records each hit's damage type (or a polar bear, whichever state the test can read):
  - `minecraft:thrown` from `player_projectile_hurt` with a snowball hit;
  - `minecraft:trident` from a trident;
  - `minecraft:fireworks` from `firework_blast`;
  - still `minecraft:player_attack` from melee.
- `world::combat::tests::a_players_wind_charge_still_turns_a_breeze_on_the_player`: the breeze's `last_hurt` attacker is the player, and the player gets the kill credit. This pins vanilla.
- `llama_spit_hurts_a_silverfish_and_flies_on_through_a_breeze`, beside the existing spit tests:
  - spit aimed at a silverfish hurts it by 1;
  - spit aimed at a breeze leaves its health alone, and exactly one `entity.breeze.deflect` voice is queued over the whole flight;
  - the spit is still alive beyond the breeze and hurts a silverfish standing behind it.

**Owner's steps** (survival, Normal, by day, flat ground, diamond armour on)
- **RH-1. Wind charge.** Use a breeze egg 8 blocks away and throw a wind charge at it. It flashes, loses half a heart and keeps attacking you; in vanilla the breeze also goes for whoever hurt it. **Screenshot RH-1.**
- **RH-2. Spit through a breeze.**
  1. Run `/summon minecraft:breeze ~ ~ ~4 {NoAI:1b}`, then `/summon minecraft:llama ~ ~ ~8`.
  2. Throw a snowball over the breeze at the llama. The llama turns and spits at you.
  3. The spit passes through the breeze with the breeze's deflect whoosh, the breeze does not flash, and the spit flies on and hits you. **Screenshot RH-2**, with the spit past the breeze.
- **RH-3. Spit on a silverfish.**
  1. Kill the still breeze with the sword, then run `/summon minecraft:silverfish ~ ~ ~4 {NoAI:1b}`.
  2. Throw another snowball at the llama. Its spit makes the silverfish flash red and still reaches you. **Screenshot RH-3.**

```
IW4L_MINECRAFT_KIT=minecraft:breeze_spawn_egg*2,minecraft:llama_spawn_egg*2,minecraft:snowball*16,minecraft:wind_charge*16,minecraft:diamond_sword,minecraft:shield,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*8,minecraft:cooked_beef*16
```

12 stacks.

---

### Group `rocket-end-targets`

**Gap:** projectiles-3: firework rockets ignore end crystals and the ender dragon. Snowballs and tridents at end targets were e2b's group `ti4`; this group adds rockets.

**Cause**
- In `crates/minecraft_terrain/src/server.rs:1773-1779` (`tick_rockets`, `:1765`), a rocket's targets are `thrown::targets(&self.mobs)`, the item frames and the players. There are no end crystals and no dragon parts.
- `rocket_blast` (`:1819-1841`) hurts only `client_mobs::server_mobs` and players, never `self.dragons`.
- e2b's `end_crystal::end_targets` (`end_crystal.rs:234-265`) feeds only `tick_thrown` (`:2628`, `:2648`) and `tick_tridents` (`:2442`). E2's log (`fw-burst`, commit 2) says end crystals and the dragon's parts are not targets yet.

**Vanilla 26.3**
- **Hit test.** `FireworkRocketEntity.tick` takes `ProjectileUtil.getHitResultOnMoveVector(this, this::canHitEntity)`.
  - `Projectile.canHitEntity` takes any pickable entity, and `EndCrystal.isPickable` and `EnderDragonPart.isPickable` are true (e2b's `ti4` findings).
  - `onHitEntity` always runs `explode`.
- **`dealExplosionDamage`** (javap):
  - the strength is `5 + 2 × stars`, and 0 with no stars;
  - the attached rider takes the whole strength;
  - every other entity from `getEntitiesOfClass(LivingEntity.class, box.inflate(5))` with `distanceToSqr ≤ 25` (feet to rocket) and a clear `COLLIDER` clip to its feet, or to its feet plus half its height, takes `strength × sqrt((5 − distance) / 5)` through `DamageSources.fireworks(rocket, owner)`.
- **So:**
  - **The crystal stays whole**: `EndCrystal` and `EnderDragonPart` are plain `Entity`s.
  - **The dragon is hurt.** It is a `LivingEntity` 16 × 8 in size, positioned at its feet. `EnderDragon.hurtServer(level, source, damage)` calls `hurt(level, body, source, damage)`, and a part other than the head or neck takes `damage / 4 + min(damage, 1)`.
  - **Even an ownerless dispenser rocket hurts it**, because `minecraft:fireworks` is in `#is_explosion` and therefore in `#always_hurts_ender_dragons`.
  - **A rocket that bursts on the head more than 5 blocks from the dragon's feet does not hurt it.**
- **Knockback.**
  - `minecraft:fireworks` is not in `#no_knockback`.
  - `LivingEntity.dealDefaultKnockback` pushes 0.4 away from the rocket (`FireworkRocketEntity.calculateHorizontalHurtKnockbackDirection`, E2's `fw-burst` rule).
  - `EnderDragon.knockback` does nothing while `isSitting`, and otherwise runs `Mob.knockback`.

**Plan: one commit, "Minecraft firework rockets burst on end crystals and hurt the ender dragon"**
- **`tick_rockets`:** add `targets.extend(crate::end_crystal::end_targets(&self.crystals, &self.dragons));` after the frames line (`:1774`). Crystals are included; only wind charges skip them.
- **`end_crystal.rs`:**
  - `DamageKind::Fireworks` next to `Explosion` (`:21`);
  - `EndDamage::fireworks(amount, player, at)` next to `explosion` (`:63`), with `from: Some(at)`;
  - `destroyed_by` (`:133`) never sees this kind, because the burst skips crystals. Give it the `Explosion` rule.
- **`ender_dragon.rs`:**
  - `hurts_the_dragon` (`:771`) also accepts `Fireworks`;
  - `hurt_part`'s knockback filter (`:1057`) accepts `Attack | Fireworks`.
- **`rocket_blast`:**
  - in the same borrow block, compute `rocket.damage_to(&world, dragon.position, ender_dragon::HEIGHT)` for every dragon that is not removed;
  - after the block, send each hit through `self.end_hit(EndTarget::DragonPart { id, part: ender_dragon::BODY }, EndDamage::fireworks(damage, rocket.owner.player(), rocket.position))` (`:3525`), which also runs the dragon events;
  - the attached rocket's rule for its rider does not change.
- **W's w3.** This group runs in e3k, after w3's events-items, which posts a rocket's `projectile_land` when it hits a block or a mob (vanilla `Projectile.onHit`). The new crystal and dragon hit is a new site: add w3's rocket `projectile_land` line there too (W0 rule 3). If w3's line turns out to sit where it already covers every hit, add nothing and say so in the log.

**Tests** (`server::tests`, using e2b's `door_world` and `crystal_and_dragon_ahead`)
- `a_rocket_bursts_on_an_end_crystal_and_leaves_it_whole`: an ownerless `Rocket::shot` flying at the crystal bursts at its front. There is one burst, the crystal is still in the list, and no blast is queued.
- `a_rocket_bursting_by_the_dragon_hurts_its_body_by_a_quarter_plus_one`:
  - a one-star ownerless burst at the dragon's feet takes 2.75 (7 / 4 + 1);
  - a flying dragon is pushed 0.4 away from the burst, and a perched one is not pushed;
  - a burst on the head 6 blocks from its feet takes nothing;
  - a starless rocket bursts on a part and takes nothing.

**Owner's steps** (survival, Normal, flat ground, diamond armour on)

Run `/gamerule pvp false` first, so your own bursts spare you.

- **RE-1. Crystal.**
  1. Place obsidian on the ground and an end crystal on it, then step 3 blocks to the side.
  2. Hold the red rocket and right-click the top of the grass block that touches the obsidian, close to the obsidian (the crystal hangs half a block over it).
  3. The rocket rises into the crystal and bursts at once into the red ball. The crystal stays and keeps turning. **Screenshot RE-1.**
  4. Step 8 blocks back and throw a snowball at it: it explodes (e2b, for comparison).
- **RE-2. Dispenser** (in the prepared world `e3-rockets`, station R; this build has no dispenser screen, see "Prepare the world" below).
  1. Place obsidian on the ground 6 blocks east of station R's cobblestone block, and an end crystal on it.
  2. Stand aside and flip R's lever.
  3. The rocket flies level into the crystal and bursts, and the crystal stays. **Screenshot RE-2.**
- **RE-3. Dragon.**
  1. Run `/summon minecraft:ender_dragon ~ ~2 ~4 {NoAI:1b,Health:5f}`.
  2. Right-click the ground right under it with the three-star rocket. The rocket rises into the dragon and bursts, and the dragon flashes red. **Screenshot RE-3.**
  3. Wait two seconds and do it again: the dragon dies with its death beams.
- **RE-4. Plain rocket.** Summon another still dragon the same way and put a plain rocket under it. The rocket pops in grey puffs on the dragon, and the dragon does not flash.
- **RE-F. Friend** (two windows with `scripts\mp-local.ps1`, `/kit @a` from the host). The friend does RE-1. Both windows show the burst on the crystal, and the crystal stays.

**Prepare the world in vanilla Minecraft 26.3** (for RE-2 and `angled-rockets`' AR-1; nobody can load a dispenser in this build, and a hand-placed one faces north until `placed-facing`, as in E3-8's `dispenser-blocks`):
1. Make a new world `e3-rockets`: Superflat, Creative, Allow Commands on, Difficulty Normal.
2. Build two stations 20 blocks apart, each with a lever on the dispenser's side, switched off:
   - **R.** One cobblestone block on the ground with a dispenser on top of it, its hole facing east (its hole is then at the crystal's height), holding 2 red rockets: `/give @s minecraft:firework_rocket[fireworks={flight_duration:1,explosions:[{shape:"large_ball",colors:[I;11743532],has_trail:true}]}] 2`. Leave 10 blocks of open ground east of it.
   - **S.** A dispenser on the ground, its hole facing east, holding 4 plain firework rockets, with a straight lane of 15 blocks of open ground east of it.
3. Save and quit. Copy `%APPDATA%\.minecraft\saves\e3-rockets` into `iw4l-artifacts\saves\`, start it single player in our game, then `/gamemode survival` and `/difficulty normal`.

```
IW4L_MINECRAFT_KIT=minecraft:firework_rocket[fireworks={flight_duration:1,explosions:[{shape:large_ball,colors:[I;11743532],has_trail:1b}]}]*64,minecraft:firework_rocket[fireworks={flight_duration:0,explosions:[{shape:burst,colors:[I;15790320,11743532]},{shape:small_ball,colors:[I;2437522]},{shape:large_ball,colors:[I;14602026]}]}]*64,minecraft:firework_rocket*16,minecraft:end_crystal*4,minecraft:obsidian*8,minecraft:snowball*16,minecraft:cobblestone*64,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*8,minecraft:cooked_beef*16
```

13 stacks.

---

### Group `elytra-rockets`

**Gaps:**
- **projectiles-12:** an elytra rocket stays at the rider's hand after the rider stops gliding.
- **projectiles-13:** an elytra rocket that bursts early still boosts for its full time.

**Cause**
- **12.** `Rocket::follow` (`crates/minecraft_terrain/src/firework_rocket.rs:213-227`) always puts the attached rocket at feet + hand.
  - `tick_rockets` gets that hand from `hand_offset(self.mobs.player_controls(rider))` (`server.rs:1790`, `firework_rocket.rs:87-98`).
  - E2's log (`fw-elytra`) says the host does not know when a player stops gliding. It does: the sim player state holds `pm_flags::MC_GLIDING` for the host and every friend. `movement_iw4/src/glide.rs:102-104` clears it on landing, and `render_anim/src/minecraft_players.rs:702` reads it into `Actor.gliding`.
  - The flag is just never copied into the survival `PlayerView` (`crates/minecraft_survival/src/entities.rs:132-153`, filled at `render_anim/src/minecraft_players.rs:1367` for every player and at `render_anim/src/minecraft_world.rs:1141` for the host) or into `PlayerControls` (`third_party/minecraftoss/entities/src/controls.rs:2-14`, copied in `player_controls`, `entities.rs:155-170`).
- **13.**
  - The rocket's own `after_hit` (`firework_rocket.rs:229-256`) ends it on a hit.
  - The boost, however, lives in the MW2 player state: `movement_iw4/src/glide.rs:51-57` (`launch_rocket`) sets `ps.mc_rocket_time`, and `glide_move` (`:67-75`) spends it.
  - Nothing shortens that time when the rocket bursts early (E2's log, `fw-elytra` "Left for later").

**Vanilla 26.3** (`FireworkRocketEntity.tick`, the attached branch, javap)
- While `attachedToEntity.isFallFlying()`, the rocket adds `look × 0.1 + (look × 1.5 − motion) × 0.5` to the rider's motion and sits at the rider plus `getHandHoldingItemAngle(FIREWORK_ROCKET)`.
- Otherwise its offset is `Vec3.ZERO`: it sits at the rider's feet and gives no boost.
- Either way it takes the rider's motion and runs `getHitResultOnMoveVector`.
- The boost comes from the living rocket each tick, so it ends on the tick the rocket bursts (`onHitEntity` or `onHitBlock` calls `explode`, which calls `discard`).
- If the rider glides again while the rocket still lives, the boost resumes. The fork's timer keeps counting down while not gliding (`glide.rs:43-45`), which is the same.

**Plan: two commits**
1. **"An elytra rocket drops to its rider's feet when the rider stops gliding"**
   - `controls.rs`: `PlayerControls.gliding: bool` after `jumping` (`Default` is derived).
   - Survival `entities.rs`:
     - `PlayerView.gliding` after `jumping` (`:149`), copied in `player_controls` (`:168`);
     - the test literals gain `gliding: false`: `entities.rs:2386`, `:2578`, `:2668`, `:3288`, `:3336`, `:3387` and `actions.rs:2405`, `:2654`, `:2727`.
   - Set it from `ps.pm_flags & playerstate_iw4::pm_flags::MC_GLIDING != 0` after the `jumping` line of both literals: `minecraft_players.rs:1399` (every player, friends included) and `minecraft_world.rs:1158` (the host).
   - `firework_rocket.rs:87` `hand_offset`: `DVec3::ZERO` unless `controls.gliding`. `tick_rockets` (`:1790`) needs no change.
   - The two E2 elytra tests mark their rider as gliding: the `holding` helper of `server::tests::an_elytra_rocket_rides_its_player_and_goes_off_at_it_when_the_boost_ends` (`server.rs:16207`), and the view and controls of `actions::tests::a_gliding_players_rocket_rides_along_and_goes_off_at_the_player_when_the_boost_ends` (`actions.rs:2825`).
2. **"An elytra rocket that bursts early ends its rider's boost"**
   - **`server.rs`, the producer:**
     - a new `ServerSim.rocket_boosts: Vec<(u64, i32)>` next to `rocket_hits` (`:125`, initialised at `:525`);
     - in `tick_rockets`' `Exploded` arm (`:1805-1809`), an attached rocket that bursts before its lifetime is up pushes `(rider, left)`;
     - `left` is the most ticks any other living attached rocket of the same rider still has (`lifetime - life + 1`), or 0;
     - a new `Output.rocket_boosts` next to `rider_falls` (`:4564`), filled next to `take_rocket_hits` in `run_tick` (`:5230`);
     - if w3 has already added a game event line in that arm, put this line after it.
   - **Survival `entities.rs`:** `PlayerOutputs.rocket_boost: Option<i32>` (the smallest `left`) next to `rider_falls` (`:185`), routed in `PlayerQueues::route` like `rider_falls` (`:377-381`).
   - **`render_anim/src/minecraft_players.rs` `struck()`:** after the `rider_falls` loop (`:864-881`), call `authority.end_rocket_boost(client, ticks)`. It runs for the host and for every friend.
   - **`sim`:**
     - `carrier.rs` `SimWorld::end_rocket_boost` next to `launch_rocket` (`:306`);
     - `frame.rs` the same next to `:461`;
     - `movement_iw4/src/glide.rs` gets `pub fn end_rocket_boost(ps, ticks)`, which sets `ps.mc_rocket_time = ps.mc_rocket_time.min(ticks.max(0) * ROCKET_TICK_MS)`, next to `launch_rocket` (`:51`), re-exported next to it (`movement_iw4/src/lib.rs:71`).
   - **No wire change.** `mc_rocket_time` is already a replicated field (`net/src/transport/netfields.rs:367`), and friends predict from it as before.

**Tests**
- `firework_rocket::tests::the_hand_offset_holds_only_while_gliding`: zero when not gliding; the right hand, or the left with only an offhand rocket, while gliding.
- `server::tests::an_elytra_rocket_drops_to_its_riders_feet_when_the_rider_lands`: the controls stop gliding mid-boost. From that tick the view sits at the feet, the burst is at the feet, and the rider's `PlayerHit` `at` is the feet.
- `entities::tests::a_gliding_player_glides_in_the_mob_controls`.
- `server::tests::an_elytra_rocket_bursting_early_on_a_zombie_ends_its_riders_boost`:
  - the output carries `(rider, 0)`;
  - with a second attached rocket alive, it carries that rocket's ticks left;
  - a rocket that ends on time posts nothing.
- `entities::tests::rocket_boost_ends_reach_the_host_and_the_friend`, like `rider_falls_reach_the_host_and_the_friend` (`:3168`).
- `glide::tests::an_ended_rocket_boost_stops_at_once_and_never_grows` (cutting to 0 stops the push, cutting to 5 keeps 250 ms, and a cut never lengthens it); `frame` gets a matching test next to `an_elytra_lets_a_player_glide_and_a_rocket_lights_only_while_gliding` (`frame.rs:1325`).

**Owner's steps** (survival, Normal, by day, flat ground, `/gamerule pvp true`)

Put on the elytra (chest slot) and the rest of the diamond armour. Move the three rocket stacks to the hotbar: red (flight 1), green (flight 3, one star) and plain (flight 3, no stars).

- **EL-1. Landing.**
  1. Press F5 for third person.
  2. Pillar 10 blocks up with cobblestone, jump off, jump again to glide, then use the red rocket and dive straight down so you land within half a second.
  3. While you glide, the sparks trail from your right side. Once you stand, they come from your feet.
  4. The red ball bursts centred on your feet. You lose about 2 hearts (its 7 damage through the diamond helmet, leggings and boots: armour 12, toughness 6) and are not shoved sideways. **Screenshot EL-1.**
- **EL-2. Early burst.**
  1. Hatch 4 zombies in a row on flat ground.
  2. 35 blocks away, pillar up 15 blocks. Glide toward the zombies, coming down to their head height, and use a green rocket when they are about 12 blocks ahead.
  3. At the first zombie the rocket bursts into a green ball. The zombie flashes and is pushed, and you lose about 2 hearts.
  4. **Your push stops at that moment** and you coast, slowing down. **Screenshot EL-2.**
  5. For comparison, use a green rocket in open air: it pushes for about 2 to 2.5 s before it bursts.
- **EL-3. Plain rocket.** Do EL-2 with a plain flight-3 rocket. It pops in grey puffs at the first zombie, nobody is hurt, and your push stops there.
- **EL-F. Friend** (two windows, `/kit @a` from the host). The friend does EL-2. The friend's push stops at the zombie, and the host sees the green burst there and the friend slowing down. **Screenshot EL-F**, both windows.

```
IW4L_MINECRAFT_KIT=minecraft:elytra,minecraft:firework_rocket[fireworks={flight_duration:1,explosions:[{shape:large_ball,colors:[I;11743532],has_trail:1b}]}]*64,minecraft:firework_rocket[fireworks={flight_duration:3,explosions:[{shape:small_ball,colors:[I;4312372]}]}]*64,minecraft:firework_rocket[fireworks={flight_duration:3}]*64,minecraft:zombie_spawn_egg*8,minecraft:cobblestone*64,minecraft:cobblestone*64,minecraft:diamond_helmet,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*16,minecraft:totem_of_undying,minecraft:cooked_beef*16
```

13 stacks.

---

### Group `angled-rockets`

**Gap:** projectiles-6. A rocket shot at an angle (from a dispenser now, and later from a crossbow) is drawn as a flat sprite facing the camera. Vanilla draws it edge-on.

**Cause**
- `Rocket::view` (`crates/minecraft_terrain/src/firework_rocket.rs:292-303`) drops the `shot_at_angle` flag the rocket keeps (`:59`, set by `Rocket::shot` at `:167`, used for dispensed rockets in `server.rs:1851-1861`).
- `thrown::append_views` (`crates/minecraft_terrain/src/thrown.rs:665-692`) draws every `ThrownView` as a 0.5-block quad facing the camera.
- `ThrownView` (`thrown.rs:631-641`) has no field for the flag. It is serde and goes to friends in `EntitySnapshot.thrown_views` (`server.rs:353`, `:5347`; `crates/minecraft_survival/src/mob_sender.rs:185`).

**Vanilla 26.3**
- `FireworkEntityRenderer.extractRenderState` copies `FireworkRocketEntity.isShotAtAngle` and resolves the stack in `ItemDisplayContext.GROUND` (`ItemModelResolver.updateForNonLiving`).
- `FireworkEntityRenderer.submit` rotates by `CameraRenderState.orientation`. For a rocket shot at an angle it then rotates by `Axis.ZP` 180, `Axis.YP` 180 and `Axis.XP` 90, and submits the item.
- The camera's orientation is `Camera.setRotation`'s `rotationYXZ(π − yRot·π/180, −xRot·π/180, 0)`.
- So the extruded 3D rocket item is seen edge-on, as a thin horizontal strip across the screen, from every view. A rocket not shot at an angle faces the camera as now.

**Plan: one commit, "Minecraft rockets shot at an angle are drawn edge-on like vanilla"**
- **`thrown.rs`:** `ThrownView.shot_at_angle: bool` after `tint` (`:640`). `Thrown::view` (`:527`) sets `false`; `Rocket::view` takes `self.shot_at_angle`.
- **`firework_rocket.rs`:** `pub fn angled_pose(at: Vec3, forward: Vec3) -> Mat4`. It is the translation, times vanilla's camera orientation built from `forward` (the fork has no roll), times Z 180, Y 180 and X 90.
- **`crates/minecraft_survival/src/entity_view.rs:1442-1452`:**
  - the `thrown::append_views` filter also leaves out `shot_at_angle` rockets;
  - each angled rocket that `thrown::shown` (`thrown.rs:612`) lets through joins the held list (`:1489-1503`) as `HeldItem { pose: angled_pose(..), light, id: "minecraft:firework_rocket", display: HeldDisplay::Ground, first_tint: None }`;
  - `ItemVisuals::append_held_items` (`mesh.rs:1226`) applies the GROUND display transform of the extruded item model (`HeldDisplay::Ground`, `mesh.rs:1390-1399`).
- The sparks of the trail still come from every rocket view.
- **## Protocol** (log): `ThrownView` gains `shot_at_angle`, inside `EntitySnapshot.thrown_views`. The release step bumps `PROTOCOL`.

**Tests**
- `firework_rocket::tests::only_a_rocket_shot_at_an_angle_says_so_in_its_view`: a launched or attached rocket is `false`; `Rocket::shot` is `true`.
- `firework_rocket::tests::an_angled_rocket_is_seen_edge_on`: for several camera directions, the posed item's face normal is perpendicular to `forward`, and its long side runs along the camera's right axis.

**Owner's steps** (survival, by day, flat ground)
- **AR-1. Dispenser** (in the prepared world `e3-rockets` from `rocket-end-targets`, station S; this build has no dispenser screen).
  1. Stand 5 blocks to the side of S's lane, at the same height, and flip S's lever.
  2. The rocket flies level down the lane as a short, thin horizontal line of the rocket's coloured pixels, not as the flat rocket picture. **Screenshot AR-1.**
  3. Stand behind the dispenser looking down the lane, flip the lever off and on again: it is still a thin line.
- **AR-2. Clicked rocket.** Right-click the ground with a plain rocket. It rises showing the rocket picture facing you, as before.
- **AR-F. Friend.** In the friend's window the dispensed rocket is the same thin line. **Screenshot AR-F.**

```
IW4L_MINECRAFT_KIT=minecraft:firework_rocket*64,minecraft:firework_rocket[fireworks={flight_duration:1,explosions:[{shape:large_ball,colors:[I;11743532],has_trail:1b}]}]*64,minecraft:cobblestone*64,minecraft:cooked_beef*16
```

4 stacks.

---

### Group `target-blocks`

**Gap:** projectiles-15. There are no target blocks, and the projectile-to-block seam knows neither the projectile's kind nor its owner. **Waits for W's w3.** w3's `events-items` gives `ServerSim::projectile_hit_block` the shooting player as the bell's `block_change` source. Build on the shape it leaves.

**Cause**
- **The seam.** e2b's `ServerSim::projectile_hit_block(pos, face, at)` (`server.rs:1731-1737`), through `projectile_struck` (`:1739-1746`), only rings bells. Its callers:
  - mob projectiles (`:1748-1752`);
  - rockets (`:1810`, `RocketEvent::Block`);
  - tridents (`:2483`);
  - thrown things (`:2690` collects `(block, face, at)`, `:2761` calls it);
  - dragon fireballs (`:3303`).
- **The vendored hits.** They are bare `(Pos, DVec3, DVec3)` tuples (`entities/src/world.rs:2599`, `:3906`), pushed at:
  - arrows (`world.rs:6875`);
  - fireballs (`world/fireballs.rs:245`);
  - shulker bullets (`world/shulker_bullets.rs:282`);
  - witch potions (`world/witches.rs:424`);
  - llama spit (`world/equines.rs:706`).
- **The redstone side.** `world/src/level/redstone.rs` has no `TargetBlock` kind (class list `:87-122`, `Kind` `:40-73`). The sculk sensor's `power` property signal (`:293`) is the model.
- **Not edited.** The player crate's `RedstoneEngine` (`player/src/redstone.rs`: `connects_to_wire` `:1768`, `signal_from` `:1787`) runs only in its trace bins, not in the game.

**Vanilla 26.3**
- **Every projectile reaches it.** `Projectile.onHitBlock` calls `BlockState.onProjectileHit` for every projectile (e2b's `bell-hits` findings).
- **`TargetBlock.onProjectileHit`** (javap) calls `updateRedstoneOutput`:
  - `getRedstoneStrength`: `d = |frac(c) − 0.5|` per axis of the hit point. On a Y face it uses `max(dx, dz)`, on a Z face `max(dx, dy)`, on an X face `max(dy, dz)`. The strength is `max(1, ceil(15 × clamp((0.5 − d) / 0.5, 0, 1)))`.
  - The duration is 20 ticks for an `AbstractArrow` (arrows, spectral arrows, tridents) and 8 for anything else.
  - Only when `!hasScheduledTick(pos, block)` does `setOutputPower` run: `setBlockAndUpdate(power)` and `scheduleTick(duration)`.
- **The rest of the block:**
  - `tick` sets the power back to 0;
  - `ownSignal` is the power, a weak signal only (no `getDirectSignal`);
  - `isSignalSource` is true;
  - `onPlace` resets a target placed with power and no pending tick to 0 (flags 18).
- A `ServerPlayer` owner also gets `Stats.TARGET_HIT` and `TargetBlockTrigger`, which the fork lacks.

**Plan: one commit, "Minecraft target blocks light up when a projectile hits them"**
- **Vendored `world.rs`:**
  - `pub struct ProjectileBlockHit { pub pos, pub at, pub motion, pub kind: &'static str, pub owner: FireworkOwner }` next to `FireworkOwner` (`:1123`);
  - `projectile_block_hits` (`:2599`, `:2741`, `:5833`) and `take_projectile_block_hits` (`:3906`) carry it;
  - each push site fills it in:
    - arrows at `:6875`: `minecraft:arrow` or `minecraft:spectral_arrow`, owner `Mob(owner_id)`;
    - `fireballs.rs:245`: `fireball.kind.type_id()`, owner from `fireball.owner`'s `Shooter`;
    - `shulker_bullets.rs:282`: `minecraft:shulker_bullet`;
    - `witches.rs:424`: the potion's entity type;
    - `equines.rs:706`: `minecraft:llama_spit`, owner the llama. This is one line in M3's file.
  - `FireworkOwner { Mob, Player, Nobody }` already says who shot it. Reuse it rather than add a third owner type.
- **Terrain `server.rs`:**
  - `pub struct BlockShot { pub kind: &'static str, pub owner: crate::firework_rocket::RocketOwner }`;
  - `projectile_hit_block(pos, face, at, shot)` and `projectile_struck(.., shot)`;
  - if w3 added a shooter parameter, fold it into `shot.owner`;
  - the callers pass:
    - rockets: `minecraft:firework_rocket` and `rocket.owner`;
    - tridents: `minecraft:trident` and the thrower, or `Mob` for a drowned;
    - thrown things: the entity type of `t.item`, so blue and brown eggs are `minecraft:egg`, and `owner` is `Player(t.owner)`, or `Mob` for `MOB_THROWER` with `t.shooter`;
    - dragon fireballs: `minecraft:dragon_fireball` and `Mob(dragon)`.
  - On a target block it calls `Level::target_hit`, after the bell check.
- **Vendored `world/src/level/redstone.rs`:**
  - `Kind::Target` next to `SculkSensor` (`:66`), with `("TargetBlock", Kind::Target)` next to `SculkSensorBlock` (`:116`);
  - an `own_signal` arm with `SculkSensor` (`:293`) that reads `int_prop(state, "power")`;
  - a `redstone_tick` arm (`:840`) that sets the power to 0 through `set_block_and_update`;
  - an `on_place` arm (`:624`) with vanilla's reset;
  - `pub fn target_hit(&mut self, pos, at: [f64; 3], axis, arrow: bool) -> bool`, which applies the strength, the `has_block_tick_at` guard (`mod.rs:786`) and `schedule_block_tick_priority` (`mod.rs:775`) with 20 or 8.
  - Check that the target's states carry the registry's `signal_source` flag (`core/src/block.rs:453`). Wire connects through `is_signal_source` (`redstone.rs:241`, `generator/src/feature/update.rs:1177`).
- Players have no bow in the fork, so a player's arrow-like shots are tridents.

**Tests**
- Vendored `level::redstone::tests::a_target_block_signals_by_how_near_the_centre_it_is_hit`: 15 at the centre of each face, 1 at the edge, and the vanilla formula in between, on each axis.
- `a_target_block_holds_twenty_ticks_for_an_arrow_and_eight_for_a_snowball_and_ignores_hits_while_lit`: a second hit while lit changes nothing, a lamp beside it lights, and dust next to it carries the strength.
- `server::tests::a_snowball_on_a_target_block_lights_the_lamp_beside_it_for_eight_ticks`.
- `server::tests::a_skeletons_arrow_into_a_target_block_holds_it_twenty_ticks`.
- The e2b bell tests still pass with the new seam.

**Owner's steps** (survival, Normal, by day, flat ground, diamond armour on)
- **TG-1. Snowball.** Place a target block on the ground and a redstone lamp right beside it. From 6 blocks away, throw a snowball at the target's side facing you. The lamp lights and goes out after about 0.4 s. **Screenshot TG-1.**
- **TG-2. Bullseye.**
  1. On the target's far side, lay 10 redstone dust in a straight line away from it, with a lamp at the end.
  2. A snowball in the small red middle of the facing side lights the far lamp. One near the edge only lights the first few dust.
  3. **Screenshot TG-2**, with the far lamp lit.
- **TG-3. Trident.** Throw the loyalty trident at the target. The lamp beside it stays lit about 1 s, twice as long, and the trident comes back.
- **TG-4. Wind charge.** Throw a wind charge at the target. The lamp blinks (0.4 s) as the charge bursts.
- **TG-5. Skeleton.**
  1. `/time set night`. Stand behind the target with the shield raised and use a skeleton egg 12 blocks in front of it.
  2. Arrows that hit the target light the lamp for about 1 s each. **Screenshot TG-5.**
  3. `/time set day`.
- **TG-F. Friend.** The friend throws a snowball at the target. The lamp lights in both windows.

```
IW4L_MINECRAFT_KIT=minecraft:target*4,minecraft:redstone*64,minecraft:redstone_lamp*4,minecraft:snowball*16,minecraft:egg*16,minecraft:trident+loyalty=3,minecraft:wind_charge*16,minecraft:skeleton_spawn_egg*2,minecraft:cobblestone*64,minecraft:torch*16,minecraft:shield,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*8,minecraft:cooked_beef*16
```

17 stacks.

---

### Group `projectile-breaks`

**Gaps:**
- **projectiles-16:** projectiles do not break chorus flowers.
- **projectiles-17:** projectiles do not shatter decorated pots, and a cracked pot cannot drop its sherds.

Both extend the seam of `target-blocks`. **The group runs in segment e3q, right after E3-12's `decorated-pots`:** before that, placement stores no pot sides, so under vanilla's rule a placed pot would shatter into nothing.

**Cause**
- Neither branch reacts to a projectile on a chorus flower or a pot (`server.rs:1731`). The parts exist: the `projectiles_can_break_blocks` rule (`third_party/minecraftoss/world/src/game_rules.rs:149`) and `Level::destroy_block_drops` (`world/src/level/drops.rs:166`).
- `ServerSim::apply_game_rules` (`server.rs:3731`) does not read `projectiles_can_break_blocks`.
- `drop_resources` (`drops.rs:33-51`) builds `LootParams` without `this_entity`. The chorus flower's loot has an `entity_properties` "this" condition, so even a working destroy would drop nothing.
- `core/src/loot.rs:379` errors on any `minecraft:dynamic` entry. The 26.3 table `blocks/decorated_pot` uses one (`minecraft:sherds`) for a cracked pot.
- The pot's block entity keeps `sherds` as a compound (`core/src/block_entity.rs:229-233`).

**Vanilla 26.3** (javap)
- **`ChorusFlowerBlock.onProjectileHit`:** when `projectile.mayInteract(level, pos)` and `projectile.mayBreak(level, pos)`, it calls `level.destroyBlock(pos, true, projectile)`.
- **`DecoratedPotBlock.onProjectileHit`:** the same two gates, then `setBlock(state.setValue(CRACKED, true), 260)` and `destroyBlock(pos, true, projectile)`.
- **`Projectile.mayInteract`:** with a `Player` owner, `Entity.mayInteract` (true in the fork, which has no spawn protection); with another owner, the `mob_griefing` rule; with no owner, true.
- **`Projectile.mayBreak`:**
  - the entity type is in `#impact_projectiles`: `#arrows`, `firework_rocket`, `snowball`, `fireball`, `small_fireball`, `egg`, `trident`, `dragon_fireball`, `wither_skull`, `wind_charge` and `breeze_wind_charge`, but not pearls, potions or spit;
  - and the `projectiles_can_break_blocks` rule;
  - and `canBreakBlockInAdventureMode`.
- **`Level.destroyBlock(pos, true, entity)`** drops the loot with `THIS_ENTITY` set to the projectile and posts `GameEvent.BLOCK_DESTROY` with that entity and the old state.
- **`DecoratedPotBlock.getDrops`** gives the dynamic drop `SHERDS_DYNAMIC_DROP_ID` (`minecraft:sherds`). Its `lambda$getDrops$0` (javap) adds the pot's sides in the order left, back, front, right, each through `Optional.ifPresent`, so an empty side drops nothing. 26.3's `PotDecorations` is a record of four `Optional<ItemStackTemplate>` sides (`back`, `left`, `right`, `front`) with no `ordered()`. A block entity without `sherds` is `PotDecorations.EMPTY` and shatters into nothing; a plain pot drops 4 bricks only because the pot item's default `pot_decorations` (`PotDecorations.allBrick`) stores four explicit brick sides when it is placed (`decorated-pots`). The table's `alternatives` picks `dynamic` when `cracked=true`. E3-12's `decorated-pots` uses the same rule on the player's harvest side.
- A cracked pot breaks with the `decorated_pot_cracked` sound type (`block.decorated_pot.shatter`).

**Plan: two commits**
1. **"Snowballs, arrows and other impact projectiles break Minecraft chorus flowers"**
   - **`ServerSim`:**
     - read `tags/entity_type/impact_projectiles.json` once from `registries.datapack`, the way `game_events.rs:356` reads tags, resolving `#minecraft:arrows`. No list goes in code.
     - keep a `projectiles_break_blocks` flag, set in `apply_game_rules` next to `block_explosion_drop_decay` (`:3743`).
   - **Gates:**
     - `may_interact(owner)`: true for `Player` and `Nobody`; for `Mob`, `self.mobs.mob_griefing()`.
     - `may_break(kind)`: the tag and the rule.
     - The adventure-mode gate is left out (see "Left out").
   - **Vendored `drops.rs`:** `destroy_block_by_entity(pos, limit)` next to `destroy_block_drops` (`:166`), sharing its body through a private helper that sets `LootParams.this_entity = true`.
   - **`projectile_hit_block`:** on a chorus flower that passes both gates, call `destroy_block_by_entity` and add one `block_destroy` line through W's helper. Its source is the shot's owner, as W's helpers do for projectiles.
2. **"Projectiles shatter Minecraft decorated pots into their sherds"**
   - **`core/src/loot.rs`:** `LootParams.dynamic: Vec<(String, Vec<ItemStack>)>` (`:30-44`), and a `"dynamic"` arm in `create_items` (`:363-381`) that emits the named list. A missing name drops nothing, as vanilla does.
   - **`drops.rs` `drop_resources`:** for a `decorated_pot`, fill `minecraft:sherds` from its block entity's `sherds` compound in vanilla's order (left, back, front, right), each side only where present; no `sherds` gives an empty list. Read the block entity the way `comparator_output_at` does (`redstone.rs:248-252`).
   - **`projectile_hit_block`:** on a pot that passes both gates, set `cracked=true` (flags 260), then call `destroy_block_by_entity` and add the `block_destroy` line.
   - **Contents.** Check that the pot's held item, or its unrolled loot table, still drops through the container removal path (`block_entity.rs:365` lists the pot). If it does not, note it in the log and do not build it here.

**Tests**
- `server::tests::a_snowball_breaks_a_chorus_flower_and_a_pearl_or_a_potion_leaves_it`: it drops one `chorus_flower` item. With `projectiles_can_break_blocks` false it stays.
- `server::tests::a_skeletons_arrow_breaks_a_chorus_flower_only_with_mob_griefing`.
- Vendored `loot::tests::a_cracked_decorated_pot_drops_its_sides_in_vanilla_order`: a pot whose `sherds` hold `back = angler_pottery_sherd` and brick on the other three sides drops brick (left), the angler sherd (back), brick (front), brick (right); a pot with only `front` set drops just that one item; a pot without `sherds` drops nothing; an uncracked pot drops the pot item.
- `server::tests::a_snowball_shatters_a_decorated_pot_into_four_bricks`: the pot is placed with four brick sides, as `decorated-pots` stores a plain pot; an ender pearl leaves the pot whole.

**Owner's steps** (survival, Normal, by day, flat ground, diamond armour on)
- **PB-1. Chorus flower.**
  1. Place end stone on the ground and a chorus flower on top.
  2. From 6 blocks away, throw a snowball at it. It breaks with its break sound and drops a chorus flower item. **Screenshot PB-1.**
  3. Place another and throw an egg: the same.
  4. Throw an ender pearl and then a water splash at a new one: it stays (you teleport with the pearl).
- **PB-2. Rule.** Run `/gamerule projectiles_can_break_blocks false`: a snowball leaves the flower. Run `/gamerule projectiles_can_break_blocks true` again.
- **PB-3. Pot.**
  1. Place a decorated pot from the kit (since `decorated-pots` it stores four brick sides) and throw a snowball at it. It shatters with the pot-shatter sound and drops 4 bricks. **Screenshot PB-3.**
  2. The trident does the same and flies back.
  3. A water splash and an ender pearl leave a pot whole.
- **PB-4. Mobs.**
  1. `/time set night`. Put a chorus flower on end stone and a pot between you and a skeleton egg used 12 blocks away. Stand behind them with the shield raised.
  2. Arrows that hit them break the flower and shatter the pot.
  3. Run `/gamerule mob_griefing false` and place new ones: the arrows leave them whole.
  4. Run `/gamerule mob_griefing true` and `/time set day`.
- **PB-F. Friend.** The friend throws a snowball at a pot. Both windows see it shatter and the bricks drop.

```
IW4L_MINECRAFT_KIT=minecraft:chorus_flower*8,minecraft:end_stone*16,minecraft:decorated_pot*8,minecraft:snowball*16,minecraft:egg*16,minecraft:ender_pearl*8,minecraft:splash_potion@water,minecraft:splash_potion@water,minecraft:trident+loyalty=3,minecraft:wind_charge*16,minecraft:skeleton_spawn_egg*2,minecraft:cobblestone*64,minecraft:shield,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*8,minecraft:cooked_beef*16
```

19 stacks (splash potions don't stack).

---

### Left out (for the log and `MINECRAFT-SURVIVAL.md`)

See the "left" list of this area. The main ones:
- **projectiles-14**, two elytra rockets compounding their boosts.
- **The anger half of projectiles-9**, which matches vanilla already.
- **The adventure-mode gate** for projectile breaks.
- **Player-cracked pots and sherd drawing.**

## E3-10. Raids, illagers, villagers and trading

The release test files list six open gaps in this area: `docs/round10/release-1-tests.md:416` ("Evoker fangs give no knockback, and vexes ignore water and lava"), `:418` ("a ravager walks to the village itself even with a rider, raiders don't pick up banners, and there are no raid omen particles") and `docs/round10/release-3-tests.md:667` ("Cartographers don't sell explorer maps yet"). Four groups close five of them. raids-3, the ravager steered by its rider, is left out (see the area's Left list).

Paths and line numbers are as of 9c6efebd. Check them again before editing: e2b and d0b merge first, and M1 to M3 and W's w3 run alongside. Vanilla facts were checked with javap on the 26.3 `client.jar`, and data facts against the 26.3 data in that jar. **The vendored crates (`third_party/minecraftoss/*`) are edition 2021**, so let-chains are not allowed there: use nested `if let`. `crates/*` may use them.

**Order:**
1. `evoker-summons`
2. `omen-particles`
3. `raid-banners`
4. `cartographer-maps`, which waits for E2's e2b merge.

**Files shared with other tracks:**

| File | This area | Other tracks this round |
|---|---|---|
| `entities/src/world/illagers.rs` | `raid-banners` (in `tick_illager`); `evoker-summons` adds tests only | M1 adds `hiding: false` to the `MobCandidate` literals (`:317`, `:327`); W's w3 adds mob equipment events |
| `crates/minecraft_survival/src/entity_view.rs` | one line in the swirl loop (`:206-208`) | E2's e2b, D0 and M2 edit other functions |
| `crates/minecraft_terrain/src/ambient_particles.rs` | `Kind::RaidOmen` | everyone appends at the end of `Kind`, so a conflict is two lines |
| `crates/minecraft_terrain/src/server.rs` | one `Command` variant next to `LootTables`, one argument in `mob_swirls` | everyone |
| `crates/minecraft_terrain/src/maps.rs`, `crates/minecraft_survival/src/menus.rs` (`loot_explorer`) | `cartographer-maps` | E2's e2b (held maps, done and merging) |

No group touches a hook the M briefs own (decisions.md, "The M briefs give every cross-track hook one owner"). raids-3 would need `world/riding.rs`, whose horse arms belong to M3; it is left out anyway.

**Protocol.** Only `omen-particles` changes a wire struct: `SwirlView` travels in `EntitySnapshot.swirls` (`server.rs:368`, sent at `mob_sender.rs:212`). Write it under "## Protocol" in `docs/round10/track-e3-log.md` and do not bump `minecraft_net::PROTOCOL`.

**Rules for every commit:**
- No code comments of any kind.
- Edits change only the lines they need, in the surrounding style.
- New enum variants and match arms go next to related ones.
- Decisions go in the track log.

---

### Group `evoker-summons` (S, 2 commits): raids-1, raids-2

**Gaps:**
- **raids-1:** a fang bite pushes neither players nor mobs.
- **raids-2:** vexes fly through water and lava at air speed and never drown.

#### Cause
- **raids-1, the fangs:**
  - `third_party/minecraftoss/entities/src/world/evoker_fangs.rs:84-117` `tick_evoker_fangs`.
  - A bitten player gets `PlayerHit { kind: PlayerHitKind::Magic { at: position }, source: owner }` (`:92-97`).
  - A bitten mob only gets `self.hurt_mob_by_magic(victim, fangs::DAMAGE, owner)` (`:115-117`), with no `knock_back`.
- **raids-1, the player side:**
  - `crates/minecraft_survival/src/entities.rs:2066-2071` `mob_hit` fills `from` only for `Melee`, `Fireworks` and `Typed`.
  - `from` is what `Survival::hurt` keeps as the hit's source (`survival.rs:842`). `render_anim/src/minecraft_players.rs:1074-1115` hands it to `sim::voxel::push_player_damage`, which pushes the player away from that point.
  - `toward` is `None` for `Magic` (`:2081`). That is right: indirect magic passes shields (test `:2828-2848`).
- **raids-1, the model to copy:** the witch's harming potion already pushes mobs. `world/witches.rs:632-647` `potion_hurt` calls `knock_back(id, 0.4, at - body)` on an applied, full hit.
- **raids-2, the move:**
  - `world/vexes.rs:118-121` moves the vex with `position += velocity`.
  - It then always applies the air drags `vex.rs:31-32` (0.91 sideways, 0.98 up and down) and never samples `FluidFrame`.
- **raids-2, the base tick:**
  - `tick_vex` (`vexes.rs:293-372`) has no `base_tick_fluid` and no `living::breathe`. Illagers have both (`world/illagers.rs:576-597`).
  - The helpers exist:
    - `FluidFrame::sample` (`entities/src/fluid.rs:54`), with `in_water` and `in_lava` (`:41`, `:45`) and `lava_height` (`:8`).
    - `emissions::base_tick_fluid` (`world/emissions.rs:56`).
    - `living::breathe` (`world/living.rs:15`).
  - The vex's fire immunity is already right (`world/hazards.rs:27-39`).
- Nothing since 03c34c19 touched the fangs: `git log --grep fang` shows only that commit.

#### Vanilla 26.3 (javap)
- **`EvokerFangs.dealDamageTo(LivingEntity)`:**
  - With an owner: `hurt(damageSources().indirectMagic(fangs, owner), 6.0)`. The owner's allies are spared.
  - Without an owner: `magic()`.
- **`LivingEntity.hurtServer`** calls `dealDefaultKnockback(source, ..)` unless the damage type is in `#minecraft:no_knockback`. In 26.3 that tag lists `minecraft:magic` but not `minecraft:indirect_magic`, so only owned fangs push.
- **`LivingEntity.dealDefaultKnockback`:**
  - The fangs are not a `Projectile`, so the direction is `source.getSourcePosition()` (the direct entity, the fangs) minus the victim.
  - The push is `knockback(0.4000000059604645, fangs.x - x, fangs.z - z)`, scaled by knockback resistance.
  - Players go through the same `hurtServer`.
- **`Vex.tick`:**
  - It runs `noPhysics = true; super.tick(); noPhysics = false; setNoGravity(true)`.
  - `Entity` reads `noPhysics` only in `move`, `push`, `isInWall`, `isAffectedByBlocks` and `refreshDimensions`. So the fluid state, the entry splash and the air supply still update.
  - `Entity.move` under `noPhysics` returns before the step and swim sounds.
- **`LivingEntity.travel`** checks `shouldTravelInFluid`, then:
  - `travelInWater`: no input (`VexMoveControl` writes the motion), `move`, then `multiply(getWaterSlowDown() 0.8, 0.8, 0.8)`.
  - `travelInLava`: `move`, then `multiply(0.5, 0.8, 0.5)` when `isInShallowFluid(LAVA)`, otherwise `scale(0.5)`. Shallow means a lava height at most the jump threshold 0.4, which applies because the vex's eye height 0.51875 is at least 0.4.
  - No gravity terms (no gravity).
- **`LivingEntity.baseTick`** drains the air of a mob that is not in `#can_breathe_under_water` (the vex is not) and has its eyes in water. At -20 it hurts the mob by 2 with `drown`.

#### Plan
1. **"Minecraft evoker fangs knock back what they bite"**
   - `evoker_fangs.rs:115-117`:
     - Keep the result of `hurt_mob_by_magic`.
     - When `owner.is_some()` and the result is applied and full, call `self.knock_back(victim, f64::from(0.4_f32), position.x - body.position.x, position.z - body.position.z)`. Take the body from `self.mob_body(victim)` (`world/combat.rs:164`), as `witches.rs:640-645` does.
     - `knock_back` (`combat.rs:256`) already applies the iron golem's, ravager's, hoglin's and the registry families' resistance.
     - Once M2's m2e has merged, a bitten registry family takes this push through M2's family knockback hook (`Family::knockback`, E3-0); keep both sides at a merge.
     - Use nested `if`s.
   - `entities.rs:2066-2071`: add one arm, `PlayerHitKind::Magic { at } if hit.source.is_some() => Some(at.to_array()),`, before the catch-all. This is the same push path melee and firework hits use. `toward` (`:2081`) stays `None`.
2. **"Minecraft vexes slow down in water and lava and drown"**
   - `vex.rs`, next to the drags (`:31-32`):
     - Constants `WATER_DRAG` (`0.8_f32 as f64`), `LAVA_DRAG` (0.5), `SHALLOW_LAVA_VERTICAL_DRAG` (`0.8_f32 as f64`) and `FLUID_JUMP_THRESHOLD` (0.4).
     - A pure `pub fn drag(velocity: DVec3, fluid: FluidFrame) -> DVec3` with four cases: air, water, shallow lava and deep lava.
   - `vexes.rs` `tick_ai`:
     - Sample `FluidFrame::sample(world, body.position, body.width, body.height)` before the move (`:118`). Vanilla decides by the state `baseTick` set at the start of the tick.
     - Replace `:119-121` with `body.velocity = vx::drag(body.velocity, fluid)`.
     - The move stays free, with no collision.
   - `vexes.rs` `tick_vex`, after `entity.tick_count += 1` (`:317`):
     - `base_tick_fluid(&mut entity.vex.body, &*world, entity.tick_count == 1, &mut entity.random, &mut entity.voices, SOUNDS)`, with `const SOUNDS: MovementSounds = MovementSounds::monster(None);` as in `illagers.rs:7`. This gives the hostile entry splash.
     - Then the `living::breathe` block of `illagers.rs:586-597`, with `vx::EYE_HEIGHT` and `entity.hurt(2.0)`.
     - Both come before the AI, so a vex with `NoAI` drowns too, as in vanilla.
     - Do not add a `play_movement` call: a vex makes no step or swim sounds.

#### Tests
- `minecraft_survival`, `entities::tests::an_evokers_fangs_push_the_player_from_where_they_bit`, next to `:2828`:
  - An owned bite gives `from == Some(at)` and `toward == None`.
  - An ownerless bite gives `from == None`.
- Vendored `world::illagers::tests`, next to `fangs_bite_a_player_once_and_a_cow_but_spare_illagers` (`:1558`):
  - `owned_fangs_push_a_bitten_cow_away_from_them`:
    - A cow on the ground just -x of the fangs ends with velocity x < 0 and y = 0.4.
    - An iron golem bitten by the same line is hurt but keeps still.
  - `ownerless_fangs_hurt_without_a_push`.
- `vex::tests::a_vex_keeps_eight_tenths_of_its_motion_in_water_and_half_in_lava`, on the pure `drag`:
  - Air: (0.91, 0.98, 0.91).
  - Water: 0.8 on every axis.
  - Shallow lava: (0.5, 0.8, 0.5).
  - Deep lava: 0.5.
- `world::vexes::tests`, with a `Pool` world (water from y 60 to 66 over stone, a lava pool beside it):
  - `a_vex_drowns_under_water_and_not_in_the_air`: a vex with `NoAI` and its eyes under water first loses 2 health 320 ticks in, then 2 every 20 ticks. One above the water keeps 14.
  - `a_vex_charging_through_water_slows_and_still_passes_stone`.
  - `a_vex_in_lava_slows_and_neither_burns_nor_drowns`.
- The existing vex tests (for example `a_vex_flies_through_stone_hovers_and_charges_a_player`, `:464`) pass unchanged.

#### Owner's steps
Survival, Normal, by day, flat open ground, wearing the kit's diamond armour.
1. **E3R-1. Fangs push you.**
   - Use an evoker egg 12 blocks away and stand still.
   - When its line of fangs reaches you, you lose hearts and are pushed back a short hop away from the fang that bit you. Before this change there was no push.
   - Raise the shield and let it cast again: the fangs still bite through the shield, and still push.
   - **e3r01_fang_push**
2. **E3R-2. Fangs push mobs.**
   - Walk 30 blocks away. Use a villager egg, then an evoker egg 8 blocks from the villager, and watch from there.
   - When the fangs bite the villager, it flashes red and jumps away from them.
   - Use the iron golem egg next to the evoker. When the fangs bite the golem, it flashes but does not move.
   - **e3r02_fangs_on_mobs**
3. **E3R-3. Friend.**
   - Two windows (`scripts\mp-local.ps1`).
   - The friend stands in front of a new evoker. In the friend's window, the bite pushes the friend back.
   - **e3r03_friend_fang_push**
4. **E3R-4. A vex drowns.**
   - Stand on flat ground facing east and run `/fill ~1 ~-3 ~-1 ~3 ~-1 ~1 minecraft:water`: a pool 3 by 3 and 3 deep, full to the top, right in front of you (three buckets would fill only its bottom layer, and the vex below would float in air). You now stand on the middle of its west edge.
   - Still facing east, run both commands:
     - `/summon minecraft:vex ~2 ~-2 ~ {NoAI:1b}` (in the water)
     - `/summon minecraft:vex ~2 ~2 ~ {NoAI:1b}` (in the air above it)
   - After about 16 seconds, the vex in the water flashes red once a second with the hurt sound. It dies after about 7 hits. The vex in the air stays unhurt.
   - **e3r04_vex_drowns**
5. **E3R-5. A vex slows in water.**
   - Swim in the pool and use a vex egg on the ground 6 blocks away.
   - The vex charges you. As soon as it dives into the water it slows down sharply, and it takes much longer to reach you than over land.
   - **e3r05_vex_in_water**

```
IW4L_MINECRAFT_KIT=minecraft:evoker_spawn_egg*4,minecraft:vex_spawn_egg*4,minecraft:villager_spawn_egg*2,minecraft:iron_golem_spawn_egg,minecraft:shield,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:diamond_sword+sharpness=5,minecraft:golden_apple*16,minecraft:cooked_beef*32,minecraft:cobblestone*64,minecraft:cobblestone*64,minecraft:milk_bucket
```

---

### Group `omen-particles` (S, 1 commit): raids-5

**Gap.** A player with Raid Omen gives off faint red potion puffs instead of vanilla's raid omen sprite. Trial Omen has the same problem, with teal puffs.

#### Cause
- **Colours only:**
  - `crates/minecraft_survival/src/swirls.rs:12-25` `player_swirl` turns every `EFFECTS` entry into a colour through `potions::effect_color`. That includes Raid Omen and Trial Omen (`player_nbt.rs:36-37`), whose colours are 14565464 and 1484454 (`crates/minecraft_terrain/src/potions.rs:104-105`).
  - `crates/minecraft_terrain/src/spell_particles.rs:43-79` `SwirlView` holds colours only.
  - `swirl()` (`:200-223`) always spawns `SpellKind::Effect`.
- **The sprites exist already:**
  - `crates/minecraft_terrain/src/ambient_particles.rs` has `Kind::TrialOmen` (`:74`). Its arms are at `:636`, `:677`, `:739`, `:1341` and `:1400`, and its sprite list entry at `:937`.
  - The 26.3 pack has `textures/particle/raid_omen.png` (a 5x5 red skull) and `trial_omen.png` (a teal skull).
- **Where swirls are drawn:**
  - `crates/minecraft_survival/src/entity_view.rs:206-208` passes every swirl of the snapshot to `self.spell.swirl`.
  - Player swirls come from the host at `render_anim/src/minecraft_world.rs:1148` and are added at `entities.rs:1713`. Mob swirls come from `minecraft_terrain/src/server.rs:5246-5254`.
  - Friends get the same list in `EntitySnapshot.swirls` (`server.rs:368`, filtered at `mob_sender.rs:212`).

#### Vanilla 26.3 (javap)
- **`MobEffects.<clinit>`:**
  - `raid_omen` is `RaidOmenMobEffect(NEUTRAL, 14565464, ParticleTypes.RAID_OMEN)`.
  - `trial_omen` is `MobEffect(NEUTRAL, 1484454, ParticleTypes.TRIAL_OMEN)`.
  - So each one's particle is its own type, not a coloured `entity_effect`.
- **`LivingEntity.tickEffects`:**
  - Chance 1 in 4 a tick; 1 in 15 when invisible; times 5 when every effect is ambient.
  - It picks `Util.getRandom(particles)` at `getRandomX(0.5)`, `getRandomY()`, `getRandomZ(0.5)`, with speed (1, 1, 1).
- **The sprite:** `particles/raid_omen.json` points at the `raid_omen` sprite. Both omens use the spell particle provider, as `Kind::TrialOmen` already does in the fork.
- **Flags:** both omens are visible and not ambient:
  - `BadOmenMobEffect` gives `MobEffectInstance(RAID_OMEN, 600, amplifier)`.
  - `TrialSpawnerStateData` gives `MobEffectInstance(TRIAL_OMEN, duration, 0)`.
  - `/effect give` gives the same flags.

#### Plan
One commit: **"Minecraft players with Raid Omen or Trial Omen give off their omen particles"**
- `spell_particles.rs`:
  - `pub enum Omen { Raid, Trial }` (Copy, serde) next to `SwirlView`, with `pub fn kind(self) -> crate::ambient_particles::Kind`.
  - `SwirlView.omens: Vec<Omen>` after `colors`.
  - `of_effects` takes `omens: impl IntoIterator<Item = Omen>` after `effects`:
    - Each omen counts as a visible, non-ambient effect, so it clears `ambient`.
    - It returns a view when either list has an entry.
  - `swirl()` keeps its random draws in order:
    1. The chance.
    2. `next_int(colors.len() + omens.len())` in place of `next_int(colors.len())`.
    3. x, y and z.
  - A pick past the colours spawns no spell and returns `Some((omen, at))`. Otherwise it returns `None` as today. A view without omens draws exactly as before.
- `ambient_particles.rs`:
  - Append `Kind::RaidOmen` at the end of `Kind` (`:57-89`), as the file's convention says.
  - Add `| Kind::RaidOmen` beside `Kind::TrialOmen` in the arms at `:636`, `:739`, `:1341` and `:1400`.
  - Add the sprite name `"raid_omen"` next to `:677`, and add `"raid_omen"` to the sprite list at `:937`.
- `swirls.rs`: `EffectKind::RaidOmen` and `EffectKind::TrialOmen` go to the omens; every other effect stays a colour.
- `server.rs:5254` `mob_swirls`: pass no omens. The entities crate's mob effects have none.
- `entity_view.rs:206-208`: `if let Some((omen, at)) = self.spell.swirl(swirl) { self.ambient.add(omen.kind(), at, DVec3::ONE); }`.
- Struct literals gain `omens: Vec::new()` at `spell_particles.rs:410` (test) and `mob_sender.rs:546` (test).
- Protocol: write a "## Protocol" line in the log. No bump.

#### Tests
- `spell_particles::tests::an_omen_swirl_hands_back_its_sprite_inside_the_body`:
  - A view with one colour and `Omen::Raid` spawns spells for about half of its hits.
  - The other hits return the omen, at positions inside the body box.
  - The chance stays 1 in 4.
  - The existing colour-only tests pass unchanged.
- `spell_particles::tests::omens_alone_make_a_swirl_that_is_not_ambient`.
- `swirls::tests::raid_and_trial_omen_give_off_their_own_sprites`:
  - Raid Omen alone gives `omens == [Raid]` and no colours.
  - With poison added, the view has poison's colour and the omen.
- `ambient_particles::tests`:
  - The sprite test (`:2654`) also loads `raid_omen`.
  - `Kind::RaidOmen` coasts like `TrialOmen`.
- `mob_sender::tests::effect_swirls_go_only_as_far_as_their_body_is_tracked` also round-trips a swirl with an omen through the codec.

#### Owner's steps
Setup:
- Two windows (`scripts\mp-local.ps1`): host H and friend F.
- Survival, by day, on open ground at least 100 blocks from any village. A Raid Omen that runs out inside a village starts a raid.
- Stand 4 blocks apart, facing each other.

1. **E3R-6. Raid Omen.**
   - F runs `/effect give @s minecraft:raid_omen 30`.
   - In H's window, small red skulls rise from F's body instead of round red potion puffs.
   - **e3r06_raid_omen_skulls**
2. **E3R-7. Trial Omen and poison.**
   - F runs `/effect give @s minecraft:trial_omen 60`: teal skulls rise among the red ones.
   - F runs `/effect give @s minecraft:poison 20`: green round puffs mix in.
   - **e3r07_omens_and_poison**
3. F runs `/effect clear @s`: nothing rises from F any more.
4. **E3R-8. Your own skulls.**
   - H runs `/effect give @s minecraft:raid_omen 20` and looks straight down: red skulls rise around H's feet. F sees them on H.
   - **e3r08_own_skulls**
   - Then H runs `/effect clear @s`.
5. **E3R-9. The real path.**
   - H drinks an ominous bottle 70 blocks outside a village and walks in.
   - The Bad Omen icon turns into Raid Omen, and red skulls rise from H for the 30 seconds until the raid starts. F sees them too.
   - The Bad Omen itself still gives off plain puffs (see this area's Left list).
   - **e3r09_raid_omen_in_village**

```
IW4L_MINECRAFT_KIT=minecraft:ominous_bottle*2,minecraft:milk_bucket,minecraft:milk_bucket,minecraft:milk_bucket,minecraft:cooked_beef*32,minecraft:golden_apple*8
```

---

### Group `raid-banners` (M, 2 commits): raids-4

**Gap.** Raiders never pick up a dropped ominous banner, so a wave whose captain died stays without a captain.

#### Cause
- **The stand-in goal:**
  - `third_party/minecraftoss/entities/src/illager_ai.rs:146-159` `raider_goals` puts `NeverGoal { controls: [Move] }` at priority 1, where `ObtainRaidLeaderBannerGoal` belongs.
  - The name lists already name the goal (`:13`, `:33`, `:56`).
  - The ravager shares this selector (`ravager_ai.rs:28`, names at `:10`). The witch has its own stand-in (`monster_ai.rs:219`). For those two, doing nothing is right: `Ravager.canBeLeader` and `Witch.canBeLeader` return false.
- **Pieces that already exist:**
  - `World::items_in`, `item` and `take_item` (`player/src/lib.rs:105-114`), used by `villager_pick_up` (`world/villagers.rs:211-235`).
  - `is_ominous_banner` (`world/illagers.rs:33`, re-exported at `world.rs:36`) and `IllagerEntity::is_captain` (`:50`).
  - `Raider.wave`, `raid_id` and `patrol_leader` (`illager.rs:203-212`); `Illager.equipment` and `drop_chances` (`:224-225`); `HEAD` (`:79`).
  - Each raider's raid view (`world/illagers.rs:552-573`, through `raid_view`, `world/raiders.rs:45`).
  - `plan_walk_path` with `PlannedPath::reached` (`navigation.rs:115-120`), and `move_to_in` (`:449`).
  - The piglins' `spawn_at_location` (`world/piglins.rs:809`).
- **What the goal cannot see:**
  - Whether its wave's leader lives. `RaidView` (`raid.rs:24-34`) carries no leaders.
  - The terrain side's `leaders` map (`crates/minecraft_terrain/src/raids.rs:253`) only does bookkeeping: inserts at `:543` and `:925`, removals by `patrol_leader` at `:627-632` and `:947-950`.

#### Vanilla 26.3 (javap)
- **`Raider$ObtainRaidLeaderBannerGoal`** (flag MOVE):
  - **`cannotPickUpBanner`** is true when any of these holds:
    - The raider has no active raid, or its raid is over.
    - `!canBeLeader()`.
    - Its head already `ItemStack.matches` `Raid.getOminousBannerInstance`.
    - `Raid.getLeader(getWave())` is non-null and alive.
  - **`canUse`:**
    1. Fail when `cannotPickUpBanner`.
    2. Build a new unreachable cache that keeps only the entries whose time is still ahead of the game time.
    3. Scan `getEntitiesOfClass(ItemEntity, getBoundingBox().inflate(followRange, 8, followRange), ALLOWED_ITEMS)` in order. `ALLOWED_ITEMS`: no pickup delay, alive, and the ominous banner.
    4. Pursue the first item that is not cached and for which `getNavigation().createPath(item, 1)` can reach it.
    5. Cache an unreachable item until `gameTime + 600`.
  - **`start`:** `moveTo(path, 1.149999976158142)`.
  - **`tick`:** when the item is `closerThan(mob, 1.414)`, call `Raider.pickUpItem`.
  - **`canContinueToUse`:** the item is not removed, the path is not done, and not `cannotPickUpBanner`.
  - **`stop`:** forget the path and the item.
- **`Raider.pickUpItem`**, for the ominous banner, with an active raid and no leader for the raider's wave:
  1. Drop the current head item when `max(random.nextFloat() - 0.1, 0) < dropChance(HEAD)` (`spawnAtLocation`).
  2. `onItemPickup`.
  3. `setItemSlot(HEAD, banner)`.
  4. `take(item, count)`.
  5. `item.discard()`.
  6. `Raid.setLeader(wave, this)`: puts a new ominous banner on the head and sets the HEAD drop chance to 2.0.
  7. `setPatrolLeader(true)`.

#### Plan
1. **"Minecraft raiders walk to a dropped ominous banner when their wave has no captain"**
   - `raid.rs`: new `ObtainBannerGoal` beside `PathfindToRaidGoal`.
     - Constants: `BANNER_SPEED` 1.149999976158142, `BANNER_REACH` 1.414, `BANNER_SEARCH_HEIGHT` 8.0, `UNREACHABLE_BANNER_TICKS` 600.
     - Fields: the unreachable cache (`HashMap<i32, i64>`), the pursued item id, and the planned path.
     - `can_start`, as vanilla:
       - Require `ctx.illager.banner_wanted`.
       - Scan `world.items_in` over the box of `ctx.body` grown by `ctx.follow_range`, 8 and `ctx.follow_range`.
       - Keep items with `pickup_delay == 0` and `count > 0` that pass `crate::world::is_ominous_banner`. Build the `ItemStack` as `villagers.rs:229` does.
       - Plan with `plan_walk_path(&ctx.body, &mut ctx.navigation, world, &ctx.profile(), ctx.fluid, item.position, 1)` and require `reached()`.
     - `start_world`: `ctx.navigation.move_to_in(world, path, BANNER_SPEED, ctx.body.position)`.
     - `can_continue`: `world.item(id)` still exists, `!ctx.navigation.is_done()`, and `banner_wanted` still holds.
     - `tick_world`: within 1.414 of the item, set `ctx.illager.banner_pickup = Some(id)`.
   - `illager_ai.rs`:
     - `IllagerState.banner_wanted: bool` and `banner_pickup: Option<i32>`, next to `raid` and `near_raid` (`:139-140`).
     - `raider_goals` puts `crate::raid::ObtainBannerGoal::default()` at priority 1 in place of the `NeverGoal`.
     - The name lists do not change. Ravagers keep `banner_wanted` false. The witch's stand-in stays.
   - `world/raiders.rs`: `EntityWorld::wave_has_leader(raid, wave)` next to `raid_view` (`:45`): a living illager of that raid and wave with `patrol_leader`.
   - `world/illagers.rs` `tick_illager`, next to `near_raid` (`:558-573`): `banner_wanted` is true when the raid view is ongoing, the head is not an ominous banner, and `!wave_has_leader(raid_id, wave)`.
2. **"Minecraft raiders put on the ominous banner they reach and lead their wave"**
   - `world/illagers.rs`, after `tick_ai` (`:650`): take `banner_pickup` and run a new `raider_pick_up_banner(id, item, world)`:
     1. Check that the item is still there with no pickup delay, is an ominous banner, and `banner_wanted` still holds.
     2. Drop the old head item by the roll above, with the illager's own random, through `spawn_at_location` (`piglins.rs:809`, made `pub(super)`).
     3. `world.take_item(item.id, item.count)`.
     4. Set `equipment[HEAD]` to the banner (count 1), `drop_chances[HEAD] = 2.0` and `raider.patrol_leader = true`.
   - `crates/minecraft_terrain/src/raids.rs` needs no change:
     - Its `leaders` map is read nowhere else.
     - Deaths and departures already go by `patrol_leader`.
     - A load re-adopts a raider with `patrol_leader` (`:925`).
   - Friends see the banner through the look tags, which are re-checked every 10 ticks (`mob_tracking.rs:196`).
   - **Game event:** vanilla's `setItemSlot` posts `minecraft:equip` (`LivingEntity.onEquipItem`). W's w3 adds the mob equipment events. By W0 rule 3, whichever of W's w3 and this group merges second adds the one `EntityWorld::game_event("minecraft:equip", ..)` line after the head change, and writes it in its log.

#### Tests
Vendored entities. Use a test world with item entities like `piglins.rs`'s `Ground` (`:2640-2710`), and `set_raids` with an ongoing raid.
- `world::illagers::tests::a_raider_without_a_wave_captain_walks_to_a_dropped_ominous_banner_and_leads`:
  - A wave-1 pillager stands 6 blocks from a banner item.
  - Within 200 ticks the item is gone, the pillager's head holds an ominous banner, `drop_chances[HEAD] == 2.0`, and `is_captain()` holds.
- `no_raider_takes_the_banner_while_its_captain_lives_outside_a_raid_or_already_wearing_one`:
  - While a wave-1 captain lives, the other wave-1 pillager leaves the banner alone.
  - A wave-2 pillager whose wave has no captain takes it.
  - A pillager without a raid, one whose raid is over, and a raider already wearing a banner all leave it alone.
- `a_ravager_never_goes_for_a_banner`.
- `an_unreachable_banner_is_left_for_600_ticks`: a banner in a closed glass box is not pursued, and is not tried again before 600 ticks.
- `a_vindicator_drops_its_helmet_for_the_banner_by_its_drop_chance`: with drop chance 1.0 a helmet item spawns; with 0.0 none does.
- The existing illager, ravager and witch goal tests pass unchanged.

#### Owner's steps
Setup:
- Survival, Normal, by day.
- Go to a village: `/locate structure minecraft:village_plains`, then `/tp` there.
- Wear the kit's diamond armour.

1. Walk 70 blocks out of the village, drink an ominous bottle and walk back to the bell. The raid starts about 30 seconds after the Raid Omen.
2. **E3R-10. A new captain.**
   - In the first wave, find the captain (the pillager with the ominous banner on its head) and kill only it.
   - The Ominous Banner drops. Step 4 blocks back so you don't pick it up.
   - Within a few seconds another raider of that wave walks to the banner, takes it and wears it.
   - **e3r10_new_captain**
3. Kill the new captain. The banner always drops again, and the next raider of the wave takes it.
4. **E3R-11. A banner out of reach.**
   - Pick the banner up, build a 3-block cobblestone pillar, stand on it, drop the banner on top (Q) and jump off. No raider takes it, because none can reach it.
   - Break the pillar's top block from the side: the banner falls, and a raider takes it.
   - **e3r11_unreachable_banner**
5. **E3R-12. Friend.**
   - Two windows (`scripts\mp-local.ps1`). The friend watches step 2 or 3.
   - The banner appears on the new captain's head in the friend's window too.
   - **e3r12_friend_captain**

```
IW4L_MINECRAFT_KIT=minecraft:ominous_bottle*4,minecraft:diamond_sword+sharpness=5,minecraft:shield,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*16,minecraft:cooked_beef*32,minecraft:totem_of_undying,minecraft:cobblestone*64,minecraft:milk_bucket
```

---

### Group `cartographer-maps` (L, 3 commits): raids-6. Waits for E2's e2b merge

**Gap.** Cartographers never sell explorer maps. The trade is rolled and then discarded.

#### Cause
- **The trade data path:**
  - `third_party/minecraftoss/entities/src/trading.rs:542-543` makes `"exploration_map"` a no-op.
  - The trade's next modifier, `filtered` on `minecraft:map_id` with `on_fail: discard`, then drops the map, and `offer()` (`:398-440`) returns `None`.
  - `TradeContext` (`:183-186`) has no origin and no explorer. `offers_from` and `villager_offers` (`:282`, `:312`) take no position.
  - Their callers are `world/villagers.rs:165`, `world/merchants.rs:192`, `villager_brain/mod.rs:152` (through `OffersAccess`, `:133-146`, built at `villagers.rs:803` and `:1027`), `world/traders.rs:294` and `bin/check_villager_trades.rs:40`.
  - `TradeBook::from_jar` (`:219-266`) loads no `tags/worldgen/structure/`.
- **The chest side already works:**
  - The `Exploring` trait (`core/src/loot.rs:24-26`) and the `exploration_map` function (`:844-862`).
  - `LootExplorer` (`crates/minecraft_terrain/src/maps.rs:572-674`): 2 s search budget, claims.
  - `menus.rs:367-390` (`loot_explorer`, `keep_explored`).
  - The entities crate cannot see `Exploring`: its only vendored dependency is `minecraftoss-player` (`Cargo.toml`).
- **Two threads:**
  - Offers are made in the entity simulation on the world thread (`ServerSim`). It has the level's `WorldGen` (`server.rs:82`, `:415-417`).
  - The map store is `Globals.maps`, a plain `Mutex<MapStore>` (`crates/minecraft_survival/src/server.rs:46`, built at `render_anim/src/minecraft_world.rs:319`), which lives on the main thread.
  - The chest explorer takes `maps.next_id()` as its first id and inserts its maps later (`menus.rs:367-390`).
  - Ids only grow through `MapStore::free_id` and `insert` (`maps.rs:507-542`).
  - So a world-thread search must allocate ids from the same store, or the two threads hand out one id twice.

#### Vanilla 26.3 (javap and data)
- **`AbstractVillager.addOffersFromTradeSet`** builds `LootParams` with `ORIGIN = position()`, `THIS_ENTITY` and `ADDITIONAL_COST_COMPONENT_ALLOWED`. `VillagerTrade.getOffer` applies `given_item_modifier` as loot functions.
- **`ExplorationMapFunction.run`:**
  - An empty stack, or no `ORIGIN`, comes back unchanged.
  - Otherwise it calls `findNearestMapStructure(destination, BlockPos.containing(origin), search_radius, skip_existing_chunks)`.
  - A find gets a new `minecraft:map_id` (zoom, tracking, unlimited) and the `minecraft:map_decorations` entry `"+"` = `{type, x, z, rotation 180.0}`.
  - Defaults: decoration `minecraft:mansion`, zoom 2, radius 50, skip true.
  - The search runs synchronously on the server thread when the trade set is rolled: on the first `getOffers`, or on a level-up.
- **Data, cartographer level 2** (2 of 8 trades), each map for 8 emeralds and a compass, search radius 100:
  - Glass pane.
  - Five village maps, each gated by the villager's `villager/variant` (a plains cartographer can sell the Savanna and Taiga Village Maps).
  - Swamp Hut Map (variants taiga, snow, jungle).
  - Jungle Pyramid Map.
- **Level 3** (2 of 3):
  - Compass.
  - Ocean Monument Map: 13 emeralds and a compass, xp 10, max uses 12, destination `#on_ocean_monument_maps`.
  - Buried Trial Chambers Map: 12 emeralds and a compass.
- **Level 5** (both): the globe banner pattern, and the Woodland Mansion Map for 14 emeralds and a compass.
- Every map trade is `exploration_map`, then `filtered{minecraft:map_id}`, else `discard`.

#### Plan
1. **"Minecraft trades can roll explorer maps"** (vendored entities)
   - `trading.rs`:
     - `pub trait TradeMaps: Send + Sync + std::fmt::Debug { fn explore(&self, destination: &[String], origin: [i32; 3], radius: i32, skip_known: bool, zoom: u8) -> Option<(i32, [i32; 3])>; }`, the same shape as `Exploring`.
     - `pub struct TradePlace<'a> { pub origin: DVec3, pub maps: Option<&'a dyn TradeMaps> }`.
     - `TradeContext.place: Option<TradePlace<'a>>`, filled from a new last argument of `offers_from` and `villager_offers`.
     - `from_jar`'s slot list (`:229-236`) loads `tags/worldgen/structure/` into `structure_tags`.
   - The `"exploration_map"` arm (`:542-543`), when there is a place with maps:
     - Resolve `destination`: a `#tag` through `resolve_tag(&self.structure_tags, ..)`, or a single id.
     - Call `explore(&destination, origin floored, search_radius or 50, skip_existing_chunks or true, zoom or 2)`.
     - On a find: set `minecraft:map_id` = id, and add `"+": {type: decoration or "minecraft:mansion", x: f64, z: f64, rotation: 180.0}` to `minecraft:map_decorations`, as `core/src/loot.rs:844-862` does.
     - Otherwise leave the item unchanged, so `filtered` discards it as in vanilla.
   - `EntityWorld`:
     - `trade_maps: Option<Arc<dyn crate::trading::TradeMaps>>` next to `trades` (`world.rs:2634`), also in the constructor (`:2760`) and the copy (`:5847`).
     - `set_trade_maps` next to `set_trades` (`villagers.rs:148`).
   - Callers:
     - `villagers.rs:165` and `merchants.rs:192` pass the villager's position and `self.trade_maps.as_deref()`.
     - `OffersAccess` gains `place`, filled at `villagers.rs:803` and `:1027`, copied by `reborrow` (`:145`) and used at `:152`.
     - `traders.rs:294` (the wandering trader's sets have no explorer maps) and `bin/check_villager_trades.rs:40` pass `None`.
2. **"Minecraft maps found on the world thread share the map store"**
   - `Globals.maps` becomes `Arc<Mutex<MapStore>>`:
     - The field at `minecraft_survival/src/server.rs:46`.
     - The construction at `render_anim/src/minecraft_world.rs:319`.
     - `Default::default()` and every `.lock()` stay as they are.
   - `maps.rs` `LootExplorer::with_store(Arc<Mutex<MapStore>>)`. A store-backed `explore`:
     1. Reads the dimension's claims from the store at the start.
     2. Searches without holding the lock (the explorer's `search_time`).
     3. Under the lock, takes the id from `free_id()`, inserts the map and merges its claims with a new `Claims::merge` (union, keeping the larger reference count of each start).
     4. Returns that id.
   - The main thread (chests, empty maps, held maps) and the world thread then never hand out one id twice or lose a claim.
   - `menus.rs:367-374` `loot_explorer` builds the store-backed explorer. `keep_explored` (`:376-390`) keeps the timeout warnings. Its inserts find nothing for a store-backed explorer and stay as they are.
3. **"Minecraft cartographers sell explorer maps"**
   - `maps.rs`: `impl minecraftoss_entities::trading::TradeMaps for LootExplorer`, delegating to its `Exploring::explore`.
   - `crates/minecraft_terrain/src/server.rs`:
     - `Command::TradeMaps(Arc<dyn TradeMaps>)` next to `LootTables` (`:4225`).
     - Its arm next to `:4945`, calling `sim.mobs.set_trade_maps`.
     - `ServerHandle::set_trade_maps` next to `load_loot` (`:4655`).
   - `crates/minecraft_survival`:
     - `Entities::set_trade_maps` next to `set_game_rules` (`entities.rs:559`).
     - `start_level` (`server.rs:318-352`) gives each level `LootExplorer::new(level.terrain.stream.world_gen(), level.dimension.dimension_type(), 0).with_store(globals.maps.clone())`, which has its own world generation and dimension.
   - The search runs on the world thread when a cartographer's offers are made, as vanilla's does on the server thread. It stops after 2 s, like the chests'. Mobs pause meanwhile: write this decision in the log.

#### Tests
- **Commit 1:** vendored `trading::tests`, gated on `MINECRAFTOSS_ROOT` like the other trade-set tests, with a stub `TradeMaps` like core's `Charted` (`loot.rs:1036`):
  - The level-3 offers include `minecraft:ocean_monument_map` with the stub's `map_id` and the `"+"` `minecraft:monument` decoration at its target, with rotation 180.
  - The stub saw radius 100, skip true, zoom 2 and the floored origin.
  - `#minecraft:on_ocean_monument_maps` resolves to `["minecraft:monument"]`.
  - With no place, or with a stub that finds nothing, the map trade is dropped and the other offers still roll.
- **Commit 2:** `maps::tests::two_explorers_on_one_store_hand_out_different_ids`:
  - Two store-backed explorers on one store get ids `last + 1` and `last + 2`.
  - The store holds both maps, and `last_id` and the claims are updated and merged.
  - The existing maps tests pass unchanged.
- **Commit 3:** terrain `server::tests`, gated, with a 120 s search time as in E2's tests:
  - A `ServerSim` at seed 1234 with loot and a store-backed explorer, and a summoned level-3 cartographer.
  - Its offers include an Ocean Monument Map whose `map_id` is in the store.
  - The map's target equals `locate::nearest_structure(["minecraft:monument"], origin, 100)`.
  - Buying it gives that stack with its id and decoration.

#### Owner's steps
Setup: survival, by day, near a village.
1. **E3R-13. A level-2 cartographer.**
   - Build a 3x3 cobblestone pen two blocks high and put the cartography table in it.
   - Stand in the pen, run `/summon minecraft:villager ~ ~ ~ {VillagerData:{type:"minecraft:plains",profession:"minecraft:cartographer",level:2},Xp:10}` and climb out.
   - Open its trades. The screen can take up to 2 seconds to open while the maps are searched.
   - It shows two offers. Often one is a Savanna Village Map or a Taiga Village Map for 8 emeralds and a compass.
   - If neither offer is a map, summon another cartographer: each new one rolls the world's next offers.
   - Buy a map.
   - **e3r13_cartographer_offers**
2. **E3R-14. Holding the map.**
   - The coastline around the target is drawn, the target village is marked, and your own marker sits at the map's edge.
   - Walk towards the mark: your marker moves onto the map.
   - **e3r14_village_map**
3. **Level 3.** Summon one with `level:3`: it offers an Ocean Monument Map (13 emeralds and a compass), a Buried Trial Chambers Map (12 emeralds and a compass), or both. An offer is missing when no such structure lies within 100 chunks, as in vanilla.
4. **E3R-15. Level 5.**
   - Summon one with `level:5`: it offers a Woodland Mansion Map (14 emeralds and a compass) and the globe banner pattern. Buy the map and hold it.
   - **e3r15_mansion_map**
5. Two cartographers at the same place that both sell the same kind of village map point at two different villages.
6. Save and quit, then reload: the bought maps keep their pictures and marks.
7. **E3R-16. Friend.**
   - Two windows. The friend buys a map from another cartographer and holds it: the friend sees the picture and the marks too.
   - **e3r16_friend_map**

```
IW4L_MINECRAFT_KIT=minecraft:emerald*64,minecraft:emerald*64,minecraft:compass*16,minecraft:cartography_table,minecraft:cobblestone*64,minecraft:cobblestone*64,minecraft:cooked_beef*32,minecraft:golden_apple*8
```

## E3-11. Mobs, mob AI, spawning and the mob registry

Nine gaps from the release test files are in this area, plus two left without an owner by M1 (mobs-25 and sculk-29). Nine are planned below in six groups. Two are left out (see "Left out" at the end). The groups run in this order:

1. `mob-age-silent` (mobs-28, mobs-41)
2. `mob-shots-blame` (mobs-38, mobs-36, mobs-35)
3. `mob-floats-jumps` (mobs-27, mobs-37)
4. `structure-zombies` (mobs-9 without its jockey)
5. `wolf-armour` (mobs-23)
6. `rabbit-nibble` (mobs-25, sculk-29; segment e3z, after M1's m1c)

**Line numbers** are as of 44d96765 (the e2b merge). D0's d0b merge (c0366f3a) has since moved lines in `world.rs`, `walk_path.rs` and `navigation.rs`, so re-check every anchor before editing. Vanilla facts were checked this round with javap on the 26.3 `client.jar` and against its data.

**Rules for every group:**
- No code comments of any kind.
- Edits change only the lines they need, in the surrounding style. New files must pass `rustfmt --check`.
- New enum variants, match arms, struct fields and list entries go next to a related one, never at the end of a shared list.
- Never bump `minecraft_net::PROTOCOL`. No group here changes a message: Silent and wolf armour reach friends through the look tag (`mob_tracking::look`, `mob_tracking.rs:320`), which keeps every saved key outside `VOLATILE` (`:15`). If a commit does change one, add a line under "## Protocol" in the track log.

**Shared lines and their owners (decisions.md, "The M briefs give every cross-track hook one owner"):**
- M1 owns:
  - `PlayerControls.sprinting`, the survival `PlayerView` lines and `PlayerCandidate`;
  - the target-slot lines in `monster_ai.rs` (`:1287`, `:1309`, `:1948`, `:1965`, `:1971`) and `wolf_ai.rs:210`;
  - the panda "still" check in `monster_ai_step` (`world.rs:1064`).
- M2 owns:
  - `spawning.rs:1583` (the zombie nautilus jockey stub) and the water hooks;
  - `movement.rs` `travel_swimming` (after `:275`) and `move_self` (`:504-609`);
  - the sulfur cube hook in front of D0's arrow and blast arms, and the family knockback hook in front of D0's knockback arms (`mob_hits_mob`, `apply_arrow_hit`, `hurt_by_blast`, m2e `sulfur-cubes-kicks`).
- M3 owns `finalize_husk`, `travel_water_slowed`, the cow store's horse lines (the horse arm of `spawn_saved`, `server_mobs.rs:642`) and `world/equines.rs` (the horse's own fall `horse_fall`).
- M1 owns the rabbit family (`families/land/rabbits.rs`); `rabbit-nibble` adds only the two lines its own brief left for the second track to merge (W0 rule 3).

None of the groups below edits those lines. Where a group works next to one of them, the group says so.

---

### Group `mob-age-silent` (M): age locks survive a reload, silent mobs make no sound

**Gaps:** mobs-28, mobs-41.

**Cause: mobs-28, the age lock.**
- `spawn_saved` (`crates/minecraft_terrain/src/server_mobs.rs:600`) reads only `Age` (`:604`) and sets it at:
  - `:616` (cows and mooshrooms), `:642` (horses and llamas), `:693` (pigs), `:717` (striders);
  - `:765` (hoglins; the only `ForcedAge` read is at `:766`), `:784` (chickens), `:804` (sheep);
  - `:1053` (wolves, `set_age`), `:1084` (cats and ocelots), `:1273` (villagers).
- So the golden dandelion lock (`entities/src/animal.rs:42`, `mob_actions.rs:127`, `world/equines.rs:765`) and `ForcedAge` are lost on every reload.
- The save closure `age` (`:1565`) writes `Age` and `ForcedAge` but no `AgeLocked`. It is used at `:1644`, `:1654`, `:1715`, `:1733`, `:1738`, `:1790`, `:1797`, `:1809`, `:1821` and `:1854`.
- `mob_tags_where` (`:1526`) starts from the loaded tag, so two things go wrong:
  - a lock made in play is never written;
  - a stale `AgeLocked:1b` from the file survives an unlock.
- `Age.locked` already exists (`entities/src/age.rs:9`). Rabbits and polar bears save it correctly (`families/land/rabbits.rs:52`, `:70`).

**Cause: mobs-41, Silent.**
- `Silent` lives only in the saved tag: `KEPT` (`server_mobs.rs:2237`, `:2306`), plus two villager conversion reads (`server.rs:1518`, `:1540`).
- No store, `MobCore` or `EntityWorld` field holds it.
- `MobSound` (`entities/src/world.rs:672`) has no entity. `take_sounds` (`:5228`) turns each store's `voices` into sounds through `resolve_voices` (`:747`), 31 calls, so there is nothing to filter on.
- The warden heartbeat is client-side (`crates/minecraft_terrain/src/families/deep_dark/wardens.rs:200-212`, `HEARTBEAT_SOUND` `:19`) and sees only the `FamilyMob`.

**Vanilla 26.3:**
- **`AgeableMob.addAdditionalSaveData`:** when `canBeABaby()`, `putInt Age`, `putInt ForcedAge`, `putBoolean AgeLocked`.
- **`readAdditionalSaveData`:** `setAge(getIntOr("Age", 0))`, `forcedAge = getIntOr("ForcedAge", 0)`, `setAgeLocked(getBooleanOr("AgeLocked", false))`.
- **`AgeableMob.mobInteract`:** the golden dandelion toggles the lock (`canUseGoldenDandelion`, `setAgeLocked`).
- **`Entity.playSound`:** returns early on `isSilent()`. `LivingEntity`'s hurt, death and step sounds and `Mob.playAmbientSound` all go through it.
- **`Warden.tick`:** plays `WARDEN_HEARTBEAT` (client side) only when `!isSilent()`.

**Plan: three commits.**

1. **"Minecraft animals keep their golden dandelion lock and forced age through a reload"**
   - `server_mobs.rs`: add a small `fn load_age(age: &mut minecraftoss_entities::age::Age, tag: &Tag)`. It sets `forced` from `ForcedAge` (default 0) and `locked` from `AgeLocked` (default false).
   - Call it on the line after each `age.set(age)` / `set_age(age)` site listed above.
   - The horse and llama arm (`:642`) is M3's cow-store horse line: m3d and m3e add undead horses and camels to that arm's kinds, and they get the `load_age` line with it. Whichever of e3m and M3's m3d/m3e merges second keeps both sides.
   - The hoglin's `:766` line becomes that call.
   - The `age` closure (`:1565`) adds `("AgeLocked", Tag::Byte(i8::from(age.locked)))`, so every user writes it and an unlock overwrites a stale key.
   - The wandering trader's plain `Age` int (`:1693`) is not an `Age` and stays as it is.
2. **"Minecraft mobs saved or summoned as Silent make no sounds"**
   - `EntityWorld` gets a field `silent` (a set of ids) next to `uuids` (`world.rs:2644`). Initialise it in `new` (`:2765`) and copy it in `clone_where` (`:5857` area).
   - Add `pub fn set_silent(&mut self, id, on: bool)` and `pub fn is_silent(&self, id) -> bool`.
   - `spawn_saved` sets it in one line after the leash load (`server_mobs.rs:1350`), from `Silent` (default false).
   - `take_sounds` (`world.rs:5228`) first clears the voices of every silent mob in `self.order`, through `exposed_mut` (`world/pushing.rs:194`, made `pub(super)`). That dispatcher covers every store and every registry family.
   - Bats and vexes are not in `exposed_mut`, so they get two explicit lines.
   - Sounds pushed straight to `self.sounds` without a voice stay as they are; log which ones.
   - `/summon ... {Silent:1b}` goes through `summon_mob`, then `add_saved_entity`, then `spawn_saved` (`server.rs:1999-2012`), so it works without more changes.
3. **"Silent Minecraft wardens have no heartbeat"**
   - This is a root hook in its own commit (D0-3), so log it as one.
   - Add `MobCore.silent: bool` next to `no_ai` (`world/families.rs:302`), false in `MobCore::new` (`:316`).
   - `set_silent` mirrors the flag into the registry mob's core through `family_core_mut` (`:1343`).
   - The warden's `client_effects` (`wardens.rs:200`) skips the heartbeat when `mob.core.silent`.
   - A friend's mirror gets the flag from the look tag, because `spawn_tracked` goes through `spawn_saved`.

**Tests:**
- `server_mobs::tests::every_ageable_store_keeps_its_age_lock_and_forced_age`:
  - covers cow, mooshroom, horse, llama, pig, strider, hoglin, chicken, sheep, wolf, cat, ocelot and villager, each loaded from `{Age:-1200,ForcedAge:40,AgeLocked:1b}`;
  - after loading, `locked` and `forced == 40`, and `can_grow()` is false;
  - `mob_tags` writes `AgeLocked:1b` and `ForcedAge:40`;
  - a mob unlocked in play overwrites an original `AgeLocked:1b` with `0b`.
- `world::tests::a_silent_mob_makes_no_voices`:
  - covers a silent cow, bat, vex and silverfish (registry);
  - hurt each, let it step and roll its ambient sound: `take_sounds` holds nothing from them;
  - a normal cow's hurt sound is still there.
- `server_mobs::tests::silent_is_read_from_the_saved_tag_and_kept`: `{Silent:1b}` loads silent, and saving keeps `Silent:1b`.
- `mob_tracking::tests::a_friend_s_mirror_of_a_silent_mob_is_silent`.
- `families::deep_dark::wardens::tests::a_silent_warden_has_no_heartbeat`: the existing heartbeat test (`:480`) with `core.silent` yields no `HEARTBEAT_SOUND`.

**The owner's steps** (survival, Normal, by day, flat ground, inside a 9×9 oak fence pen):

1. Summon four babies:
   - `/summon minecraft:cow ~2 ~ ~2 {Age:-24000}`
   - `/summon minecraft:pig ~-2 ~ ~2 {Age:-24000}`
   - `/summon minecraft:chicken ~2 ~ ~-2 {Age:-24000}`
   - `/summon minecraft:wolf ~-2 ~ ~-2 {Age:-24000}`

   Right-click each baby with a golden dandelion: you hear the dandelion chime, and one dandelion is used each time. Right-click the calf with wheat: it does not eat, and the wheat stays at 32. **Screenshot mob-01** (the hotbar and the calf).
2. Save and quit, then load the world. Try the food again:
   - the calf with wheat;
   - the piglet with a carrot;
   - the chick with wheat seeds;
   - the puppy with raw beef.

   None of them eats, and the stacks stay full. Before this fix, the reloaded babies ate and grew. **Screenshot mob-02.**
3. Right-click the calf with a golden dandelion again: a different chime, the unlock. Then give it wheat: it eats, with green sparkles, and the wheat drops to 31. **Screenshot mob-03.**
4. `/summon minecraft:cow ~4 ~ ~ {Silent:1b}`. Stand next to it for half a minute: it never moos. Hit it twice with your hand: it flashes red but makes no hurt sound. Then `/summon minecraft:cow ~-4 ~ ~` and hit it: you hear that one. **Screenshot mob-04** (the silent cow flashing red).
5. Save and quit, then load: the silent cow is still silent.
6. `/summon minecraft:warden ~ ~ ~10 {Silent:1b,NoAI:1b}` and `/summon minecraft:warden ~ ~ ~-10 {NoAI:1b}`. Stand by each in turn: only the second one's heart thumps. **Screenshot mob-05.**
7. Friend (two windows): the friend stands by the silent cow and the silent warden and hears nothing from either, while the other warden thumps. **Screenshot mob-06-friend.**

```
IW4L_MINECRAFT_KIT=minecraft:golden_dandelion*8,minecraft:wheat*32,minecraft:wheat_seeds*32,minecraft:carrot*32,minecraft:beef*16,minecraft:oak_fence*64,minecraft:iron_sword,minecraft:cooked_beef*16
```

---

### Group `mob-shots-blame` (M): fireballs sent back blame the player, ownerless wind charges still hurt, breezes aim at riders

**Gaps:** mobs-38, mobs-36, mobs-35.

**Cause: mobs-38, fireballs sent back.**
- A player's knocked-back ghast or blaze fireball reaches `fireball_damages_mob` (`entities/src/world/fireballs.rs:400`). It passes `attacker = PLAYER_TARGET + player` (`:420`) to `apply_arrow_hit` (`world.rs:3555`).
- `apply_arrow_hit` stores `Target::Mob(owner)` for the stores below. That target resolves to no entity, so the mob never turns on the player:
  - iron golems (`:3613`), wolves (`:3625`), cats and ocelots (`:3632`), wandering traders (`:3639`);
  - cows, horses and llamas (`note_attacker`, `:3648`);
  - ghasts, blazes, snow golems, illagers, vexes, ravagers and withers (`:3673-3715`), and shulkers (`:3742`).
- `blasted_by_player` (`:3133`) repairs this afterwards, but only for the monster stores (`fireballs.rs:437`).
- Hoglins and piglins are already right, through `note_hoglin_hurt_by_owner` (`hoglins.rs:458`, `attacker_of` `:49`). Villagers are right too: `villager_hurt_by` (`world.rs:4119`) checks `PLAYER_TARGET`.
- The correct mapping is already written at `:3745` for the registry families.

**Cause: mobs-36, wind charges without their breeze.**
- `crates/minecraft_terrain/src/server.rs:2781-2794` handles each burst's hit.
- When the breeze is gone, `shooter` is `None` and `owner` is `MOB_THROWER`. The arm `(Some((id, None)), None) if owner != MOB_THROWER` (`:2788`) is skipped, so the mob takes nothing.
- Players are still hurt, without a source, as in vanilla (`:2787`).
- `thrown::hurt_by` (`thrown.rs:599`) always blames a player, through `player_projectile_hurt` (`world/combat.rs:340`).
- No public mob hurt has a damage type and no attacker. The closest one is `firework_blast` with `FireworkOwner::Nobody` (`world.rs:3109`) over `hurt_by_typed_blast` (`:3039`).
- That function's family arm passes `blaster.damage_type()` instead of its `kind` argument (`:3078`), so fireworks also reach registry mobs as `minecraft:explosion`.

**Cause: mobs-35, breezes and riders.**
- `world/families/trial/breezes/brain.rs:517` always aims at `target.height * 0.3`.
- The brain's `Ctx` (`:93-109`) knows only whether the breeze itself is a passenger (`:106`).
- The family builds `Ctx` at `breezes.rs:228-244`. `world.vehicle_of(Rider::Player(id))` (`world/riding.rs:182`) is in reach there. The test literal is at `brain.rs:838`.

**Vanilla 26.3:**
- **`Projectile.deflect`:** a player's hit makes the player the owner. `LivingEntity.hurtServer` then calls `setLastHurtByPlayer` and `setLastHurtByMob` with `source.getEntity()`, the player, so `HurtByTargetGoal`, `LlamaHurtByTargetGoal` and their kin target the player.
- **`AbstractWindCharge.onHitEntity`:** `hurtServer(level, damageSources().windCharge(this, owner), 1.0F)` runs even when `getOwner()` is null, then `explode(position())`.
- `wind_charge` is in `#is_projectile` (endermen dodge it) and in `#panic_causes`, like `explosion`.
- **`net.minecraft.world.entity.monster.breeze.Shoot.tick`:** aims at `target.getY(target.isPassenger() ? 0.8 : 0.3)`.

**Plan: three commits.**

1. **"Minecraft mobs hit by a fireball a player sent back turn on that player"**
   - In `apply_arrow_hit`, compute `by` once at the top: move the `:3745` line up next to `owner_undead` (`:3561`), or make `attacker_of` (`hoglins.rs:49`) `pub(super)` and call it.
   - Replace each `crate::monster_ai::Target::Mob(owner)` in the store arms with `by`. Nothing else changes.
   - Mob owners keep `Target::Mob(owner)` because `owner < PLAYER_TARGET`.
   - If M2's sulfur cube hook already sits at the top of the function, keep both lines.
   - Leave alone, and log: an ownerless fireball still passes its victim as owner (`fireballs.rs:421`, `None => victim`). Vanilla blames nobody there, and fixing it needs an optional owner through `apply_arrow_hit`.
2. **"Minecraft wind charges whose breeze is gone still hurt the mob they hit"**
   - `world/combat.rs`: add `pub fn ownerless_projectile_hurt(&mut self, id: u64, amount: f32, kind: &'static str, velocity: DVec3) -> Option<DamageResult>`, placed after `player_projectile_hurt`:
     - an enderman dodges, as at `:342-345`;
     - otherwise `hurt_by_typed_blast(key, amount, DVec3::ZERO, Blaster::Nobody, kind)`;
     - then `knock_back(id, 0.4, -velocity.x, -velocity.z)` on an applied full hit. Once M2's m2e has merged, a registry family takes it through M2's family knockback hook with `Knocker` None (E3-0).
   - ominous-shots (E3-14, e3t) later extends this same function and `hurt_unowned`; keep both easy to extend.
   - `combat` is a child of `world`, so it reaches the private method. Its `Explosion` source kind gives the same animal panic, because `wind_charge` is in `#panic_causes`.
   - The powered wither turns the charge away through `hurt_typed` (`withers.rs:181`).
   - `world.rs:3078`: registered-hits (E3-9, e3j) already makes the family arm pass `kind`, which also gives registry mobs vanilla's `minecraft:fireworks`. Check it is in and do not edit the line again.
   - `thrown.rs`: add `pub fn hurt_unowned(world, id, amount, kind, velocity) -> bool` next to `hurt_by`.
   - `server.rs`: a new arm `(Some((id, None)), None) => { crate::thrown::hurt_unowned(...); }` right after the guarded arm at `:2788`.
3. **"Minecraft breezes aim higher at riders"**
   - `brain::Ctx` gets `riders: &'a [Target]` next to `passenger` (`:106`).
   - `breezes.rs` builds it before `:228`: the players whose `world.vehicle_of(Rider::Player(p.id))` is `Some`.
   - `tick_shoot` (`:517`) aims at `target.height * if ctx.riders.contains(&target.target) { 0.8 } else { 0.3 }`.
   - The test literal (`:838`) gets `riders: &[]`.
   - The brain's shared `Living` struct (8 literals in hoglins, piglins and breezes) stays unchanged.

**Tests:**
- `world::tests::a_fireball_a_player_sent_back_turns_wolves_llamas_traders_and_golems_on_that_player`:
  - goes through `fireball_damages_mob(.., Some(Shooter::Player(p)), ..)`;
  - the wolf's `ai.state.hurt_by`, the llama's `horse.fight.hurt_by`, the trader's and the iron golem's are all `Target::Player(p)`;
  - a ghast-owned fireball still blames the ghast.
- `world::combat::tests::an_ownerless_wind_charge_hurts_a_cow_by_one_and_knocks_it_back`:
  - an enderman dodges;
  - a powered wither turns it away;
  - a silverfish takes it as `minecraft:wind_charge`.
- `world::tests::fireworks_reach_a_registry_mob_as_fireworks`.
- `server::tests::a_breeze_charge_still_hurts_the_mob_it_hits_after_its_breeze_is_gone`:
  - a real flight through `tick_thrown`: a breeze, its charge in flight, the breeze removed, a cow in the path;
  - the cow's health drops by 1 and no player is credited.
- `breezes::brain::tests::a_breeze_aims_at_eight_tenths_of_a_rider_and_three_tenths_of_a_walker`.
- `breezes::tests::a_player_on_a_pig_is_a_rider_to_the_breeze`.
- The existing wind charge and fireball tests pass unchanged.

**The owner's steps** (survival, Normal, by day, wide flat ground; full diamond armour, the shield in the offhand):

1. Hatch a llama 3 blocks in front of you and tie it to a fence with a lead. Hatch a ghast about 25 blocks away, past the llama and a little to the side.
2. When a fireball comes, look at the llama and hit the fireball with the sword: it flies back into the llama and bursts. The llama turns to you and spits at you (white spit puffs). Before this fix it spat at nothing. **Screenshot mob-07.**
3. Optional: do the same with a wild wolf tied the same way. If it survives the blast, it turns on you with red eyes and bites. **Screenshot mob-08.** Kill the ghast.
4. Hatch a pig, saddle it and ride it, steering with the carrot on a stick, 8 to 12 blocks from a breeze you hatch. The breeze's wind charges come at your chest and burst there, shoving you, instead of bursting low at the pig's legs. **Screenshot mob-09** (a burst at chest height).
5. Optional; this one is hard to time, and the unit test covers it. Hatch a cow 5 blocks in front of a second breeze, with you behind the cow. When the breeze inhales, shoot it with the M4 until it dies (players have no bow in the fork). A charge that is already flying still flashes the cow red and pushes it. Before this fix it only pushed.

```
IW4L_MINECRAFT_KIT=minecraft:ghast_spawn_egg*2,minecraft:llama_spawn_egg*2,minecraft:wolf_spawn_egg*2,minecraft:lead*4,minecraft:oak_fence*16,minecraft:diamond_sword+sharpness=5,minecraft:shield,minecraft:breeze_spawn_egg*2,minecraft:cow_spawn_egg*2,minecraft:pig_spawn_egg*2,minecraft:saddle,minecraft:carrot_on_a_stick,iw4l:m4,iw4l:rifle_ammo*64,minecraft:cobblestone*64,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*16,minecraft:cooked_beef*32
```

---

### Group `mob-floats-jumps` (M): low mobs float in thin water, jump boost works on mobs

**Gaps:** mobs-27, mobs-37.

**Cause: mobs-27, floating.**
- Every `FloatGoal` uses a fixed 0.4:
  - `third_party/minecraftoss/entities/src/monster_ai.rs:2064`, the shared goal. Silverfish, endermites, rabbits, polar bears, wolves, cats, ocelots, illagers, ravagers, traders and vexes use it.
  - `chicken_ai.rs:100`, `cow_ai.rs:177` (cows, and the horse and llama selectors at `:507` and `:556`), `pig_ai.rs:309` and `eat_block.rs:109` (sheep).
- The jump that follows has the same fixed threshold in `monster_ai_step` (`world.rs:1088`), `world/wolves.rs:181` and `world/felines.rs:265`.
- Chickens and pigs already pass the right threshold to `living_jump` (`world.rs:7074`, `:7348`).
- A puppy's goals see the adult eye height: `wolves.rs` `tick_ai` (`:118-132`) never sets `ai.state.eye_height`, which `MonsterAi::new` filled with `wolf::EYE_HEIGHT` (`monster_ai.rs:961`). Cats do set it (`felines.rs:156`).
- The right rule already exists twice: `rabbits/ai.rs:557` and `villager_brain/behaviors.rs:327`.
- The eye heights are at hand:
  - `MonsterGoalContext.eye_height` (`monster_ai.rs:499`), also the local `eye_height` in `monster_ai_step` (`world.rs:1029`);
  - `chicken_ai.rs:180`, `cow_ai.rs:257` (`species.eye(age)`), `pig_ai.rs:277`, `eat_block.rs:194`.

**Cause: mobs-37, jump boost.**
- `MobEffect` (`effects.rs:17-34`, `ALL` `:37`) has no `JumpBoost`. `PotionEffect::mob_effect` (`crates/minecraft_terrain/src/potions.rs:214`) and the save loader (`server_mobs.rs:1403`) map ids through `MobEffect::from_id`, so leaping splash and lingering potions, tipped arrows and saved effects all drop it.
- The mob jump is `0.42 * jump_factor` (`movement.rs:263`), with no boost.
- `Body::jump_power` (`movement.rs:642`) is vanilla's `getJumpPower(multiplier)` without the boost; ridden horses (`equines.rs:250`), ravagers (`ravagers.rs:227`) and rabbits (`rabbits/ai.rs:530`) use it.
- Per-tick effect state already reaches the body in one place, `Body::float_from` (`movement.rs:125`, levitation and slow falling). Every store calls it.
- The fall damage safe distance is a fixed 3 (`world/living.rs:104-109`).
- The breeze's long jump (`breezes/brain.rs:623-641`) takes `jump_vector` with no lift.

**Vanilla 26.3:**
- **`FloatGoal.canUse`:** `mob.isInFluidDeeperThan(mob.getFluidJumpThreshold(), fluid) || mob.isInLava()`. The fluid is `#entity_floatable`.
- **`Entity.getFluidJumpThreshold`:** `getEyeHeight() < 0.4 ? 0.0 : 0.4`.
- **`LivingEntity.isInFluidDeeperThan`:** `getFluidHeight(tag) > threshold`.
- **`LivingEntity.getJumpBoostPower`:** with `JUMP_BOOST`, `0.1 * (amplifier + 1)`, else 0.
- **`getJumpPower(m)`:** `JUMP_STRENGTH * m * getBlockJumpFactor() + getJumpBoostPower()`. `jumpFromGround` uses `getJumpPower()`.
- **`MobEffects.JUMP_BOOST`:** colour 16646020, with modifier `effect.jump_boost` `SAFE_FALL_DISTANCE +1.0 ADD_VALUE` per level.
- **`Breeze` `LongJump.calculateOptimalJumpVector`:** with `breeze.hasEffect(JUMP_BOOST)`, the chosen vector gets `+ (0, normalize().y * getJumpBoostPower(), 0)`.

**Plan: three commits.**

1. **"Minecraft silverfish, endermites, cats and baby animals float in thin water like vanilla"**
   - `fluid.rs`: add `pub fn jump_threshold(eye_height: f32) -> f64` after `floatable_height` (`:50`).
   - Each `FloatGoal` compares `floatable_height()` with `crate::fluid::jump_threshold(..)` of its eye height, at `monster_ai.rs:2064`, `chicken_ai.rs:100`, `cow_ai.rs:177`, `pig_ai.rs:309` and `eat_block.rs:109`.
   - Pass the same threshold to `living_jump` at `world.rs:1088` (M1's panda line `:1064` is nearby; leave it alone), `wolves.rs:181` and `felines.rs:265`.
   - `wolves.rs` `tick_ai`: add `ai.state.eye_height = eye;` after `:130`, as cats do.
   - Rabbits and villagers keep their own correct copies.
2. **"Minecraft mobs take jump boost from potions and tipped arrows like vanilla"**
   - `MobEffect::JumpBoost` goes after `SlowFalling` (`effects.rs:30`) in the enum, `ALL` (length 17) and `id` (`"minecraft:jump_boost"`).
   - `Body.jump_boost: Option<u8>` goes next to `levitation` and `slow_falling` (`movement.rs:77-78`, `:116-117`), and `float_from` sets it.
   - `jump_power` adds `0.1 * (amplifier + 1)`. `living_jump`'s power line (`:263`) becomes `self.jump_power(world, 0.42, 1.0)`, so ridden horses, ravagers and rabbits get the boost too.
   - `living::fall_damage` uses `3.0 + (amplifier + 1)`, and only the stores E3 may edit that land through it get the term. The horse's own fall (`horse_fall` in M3's `world/equines.rs`, which M3's camels reuse) and the registry's generic landing in `families.rs` (a shared root site, E3-2's "Left out") keep 3: log both as left for later, for M3 and for the registry's owner.
   - The swirl colour already exists (`potions.rs:78`).
3. **"Minecraft breezes leap higher with jump boost"**
   - `brain::Ctx` gets `jump_boost: f32` next to `levitating` (`:107`). `breezes.rs:242` fills it from `core.effects`; the test literal is `:852`.
   - In `tick_long_jump`, the found `leap` gets `+ DVec3::Y * leap.normalize().y * f64::from(ctx.jump_boost)` when the boost is above 0.

**Tests:**
- `fluid::tests::the_jump_threshold_is_zero_below_a_four_tenths_eye`: 0.13, 0.28125 and 0.35 give 0; 0.4, 0.68 and 1.74 give 0.4.
- `monster_ai::tests::float_goal_starts_in_thin_water_for_a_low_eye`: water height 0.22 starts it for an eye of 0.13 and not for 1.74; lava starts both.
- `world::tests::a_chick_a_piglet_a_kitten_a_silverfish_and_a_cat_bob_in_thin_water_but_adults_walk`: real ticks over a stand-in world of flowing water at height 0.22.
- `world::wolves::tests::a_puppy_s_goals_see_its_baby_eye`.
- `effects::tests::jump_boost_round_trips_its_id`.
- `movement::tests::jump_boost_adds_a_tenth_per_level_to_the_jump`: 0.42 becomes 0.52 at amplifier 0 and 0.62 at amplifier 1, through both `living_jump` and `jump_power`.
- `world::living::tests::jump_boost_raises_the_safe_fall_distance`: a 4.5-block fall hurts 1 without it and 0 at amplifier 0.
- `world::tests::a_leaping_splash_makes_a_slime_hop_higher`.
- `server_mobs::tests::a_mob_s_jump_boost_survives_a_save`.
- `breezes::brain::tests::jump_boost_lifts_the_long_jump_by_the_rise_times_the_power`.

**The owner's steps** (survival, by day, flat open ground; lay a 16×16 cobblestone floor walled 2 high):

1. Empty a water bucket in one corner. The water spreads 7 blocks; the last three blocks of it are thin.
2. In the thin water, 5 to 7 blocks from the corner, hatch a cat, a silverfish and an adult chicken, and run `/summon minecraft:chicken ~ ~ ~ {Age:-24000}`. Hatch a cow there too.
3. The cat, the silverfish and the chick keep bobbing up in the thin water, while the adult chicken and the cow walk through it. **Screenshot mob-10.** Kill the silverfish.
4. Hatch two slimes in the dry half. Throw a splash potion of strong leaping at one: light green swirls rise from it, and its hops go about twice as high as the other slime's. **Screenshot mob-11.**
5. Hatch a breeze in the pen and throw strong leaping at it: its long jumps rise clearly higher, often over the wall. **Screenshot mob-12.**
6. Save and quit, then load: the boosted slime still hops high until the effect runs out (about 1:30). **Screenshot mob-13.**
7. Friend (two windows): the friend sees the same high hops and the bobbing cat. **Screenshot mob-14-friend.**

```
IW4L_MINECRAFT_KIT=minecraft:water_bucket*2,minecraft:cobblestone*64,minecraft:cobblestone*64,minecraft:cat_spawn_egg*2,minecraft:silverfish_spawn_egg*2,minecraft:chicken_spawn_egg*2,minecraft:cow_spawn_egg*2,minecraft:slime_spawn_egg*4,minecraft:breeze_spawn_egg*2,minecraft:splash_potion@strong_leaping*4,minecraft:shield,minecraft:iron_sword,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:cooked_beef*32
```

---

### Group `structure-zombies` (M): ocean ruin drowned and zombie village villagers are finalized like zombies

**Gap:** mobs-9, without the zombie nautilus jockey (left for M2, see "Left out").

**Cause:**
- Ocean ruin drowned are made in the generator, which cannot reach the world crate's zombie code: `minecraftoss_world` depends on `minecraftoss_generator`, not the other way round.
- `generator/src/structure/kinds/ocean_ruin.rs:215-220` creates the drowned, sets `PersistenceRequired` and calls `feature::entities::finalize_spawn(ctx, &mut entity, false)`.
- `generator/src/feature/entities.rs:117-187` runs only `finalize_mob` (`:158`) for a drowned. So there is:
  - no Zombie part: `CanPickUpLoot`, the 5% baby, `CanBreakDoors`, equipment, enchantment draws;
  - no drowned weapon, so a ruin drowned never holds a trident or a fishing rod;
  - no 3% nautilus shell.
- The world crate does all of this, but only for natural spawns: `spawning.rs:1415` `finalize_zombie`, `:328` `drowned_weapon`, `:1603` `finalize_drowned`.
- **Correction to the gap list:** igloo template entities are not finalized in vanilla either (`IglooPieces$IglooPiece` sets no `setFinalizeEntities`).
- Jigsaw pieces are finalized: `SinglePoolElement.getSettings` sets `setFinalizeEntities(true)`, which the fork mirrors at `structure/jigsaw.rs:172` and `feature/template/mod.rs:399`. So the zombie villagers of zombie villages (`village/*/zombie/villagers/*.nbt`) get `Zombie.finalizeSpawn` in vanilla, while here `place_template_entity` (`entities.rs:191`, `:219`) gives them only the villager type and `finalize_mob`.
- The generator knows no difficulty: `Library.peaceful` (`feature/mod.rs:153`) is never set true. `finalize_piglin` (`entities.rs:253-258`) therefore uses peaceful's special multiplier of 0.

**Vanilla 26.3:**
- **`OceanRuinPieces$OceanRuinPiece.handleDataMarker("drowned")`:** calls `Drowned.finalizeSpawn(level, difficulty, STRUCTURE, null)`.
- **`Drowned.finalizeSpawn`** first calls `Zombie.finalizeSpawn`, which runs in this order:
  - `Monster`/`Mob` `finalizeSpawn` (the follow-range bonus and left-handed roll; this is `finalize_mob`);
  - `setCanPickUpLoot(nextFloat < 0.55 * special)`;
  - `ZombieGroupData(getSpawnAsBabyOdds = nextFloat < 0.05, canSpawnJockey true)`;
  - for a baby, `nextFloat < 0.05` looks for a free chicken within 5/3/5; failing that, `nextFloat < 0.05` makes a `JOCKEY` chicken;
  - `setCanBreakDoors(nextFloat < 0.1 * special)`;
  - `populateDefaultEquipmentSlots`. `Drowned` overrides it without calling super: `nextFloat > 0.9`, then `nextInt(16) < 10` gives a trident, else a fishing rod. A zombie villager gets `Zombie`'s: `Mob`'s armour roll at `0.15 * special`, then an iron weapon at 0.01, or 0.05 on Hard;
  - `populateDefaultEquipmentEnchantments` (one draw per filled slot);
  - the Halloween head (real date);
  - `handleAttributes` (on the zombie's own random).
- After that part, `Drowned.finalizeSpawn` continues: an empty offhand gets `NAUTILUS_SHELL` at `nextFloat < 0.03` with `setGuaranteedDrop`. Then, for `NATURAL` or `STRUCTURE` with a trident: `nextFloat < 0.5`, an adult and a biome outside `#more_frequent_drowned_spawns` (`#is_river`) give a persistent zombie nautilus jockey.
- **`ZombieVillager.finalizeSpawn`:** `finalizeVillagerType`, then `Zombie.finalizeSpawn`.

**Plan: two commits, all in `generator/src/feature/entities.rs`.** `ocean_ruin.rs` already calls `finalize_spawn`. `spawning.rs` is not touched (M2 and M3 own parts of it).

1. **"Minecraft zombies placed by structures are finalized like zombies"**
   - A new `fn finalize_zombie(tag: &mut Tag, random: &mut impl RandomSource) -> bool` returns whether a jockey chicken is wanted. It writes:
     - `CanPickUpLoot`, from a draw at special 0;
     - `IsBaby`;
     - `CanBreakDoors`, from a draw at special 0;
     - the equipment: drowned weapon, or the zombie villager's armour draw plus its 1% iron sword, spear or shovel by `nextInt(6)` as `spawning.rs:1467-1474`;
     - one enchantment draw per filled slot, in `finalize_piglin`'s order.
   - `finalize_spawn` calls it right after `finalize_mob` (`:158`) for `minecraft:drowned` and `minecraft:zombie_villager`, with `ctx.region.level_random()`.
   - For a baby, the first 5% roll finds no free chicken in a fresh structure, so it only draws. The second 5% roll creates a chicken with `create`, finalizes it with `finalize_mob`, and wraps the zombie as its `Passengers`, as `jockey_chicken` does.
   - Skip, and log as decisions:
     - special 0 and the non-Hard 1% weapon chance, because generation sees no difficulty;
     - no Halloween head (generation has no date);
     - no `handleAttributes`: it draws from the zombie's own random, so the level stream is unchanged, and the fork simulates neither reinforcements nor that knockback bonus.
2. **"Minecraft ocean ruin drowned carry nautilus shells like vanilla"**
   - For a drowned: an empty offhand gets `minecraft:nautilus_shell` at `nextFloat < 0.03`, with `drop_chances.offhand` 2.0, as `spawning.rs:1607-1611`.
   - Then a trident holder draws the 0.5 jockey roll. An ocean ruin is never in `#is_river`, so the biome test always passes. The nautilus itself is left for M2.
   - The world crate's loader already reads both hands, the drop chances and `IsBaby` (`server_mobs.rs:846-854`), so a ruin drowned throws its trident.

**Tests:**
- `feature::entities::tests::structure_drowned_roll_tridents_rods_shells_and_babies_at_vanilla_rates` (new test module):
  - 20 000 tags through `finalize_zombie` plus the shell step on a seeded random;
  - trident 6.25%, fishing rod 3.75%, shell 3% and baby 5%, each within one point;
  - `CanPickUpLoot` and `CanBreakDoors` are always present and `0b`.
- `feature::entities::tests::the_zombie_draws_come_in_vanilla_order`: a counting random sees loot, baby, (two jockey draws for a baby), doors, armour (zombie villager only), weapon (with `nextInt(16)` when it holds one), an enchantment draw per filled slot, shell, and the jockey draw for a trident holder.
- `feature::entities::tests::a_zombie_village_zombie_villager_is_finalized_like_a_zombie`: through `place_template_entity` with finalize on, the keys are present; a plain villager is unchanged.
- If a generated ocean ruin test already exists in `minecraft_terrain`, extend it so its drowned carry the keys.

**The owner's steps** (start a new world, so the ruins come from this build):

1. `/locate structure minecraft:ocean_ruin_warm` (or `minecraft:ocean_ruin_cold` in colder seas). `/tp` above it, then `/gamemode creative` to dive without drowning.
2. Look at its drowned. Run `/locate` again after flying about 400 blocks on, and check three or four ruins in all:
   - about one drowned in ten holds a trident or a fishing rod;
   - now and then one is a small baby;
   - rarely one holds a nautilus shell in its other hand.

   **Screenshot mob-15** (a drowned with a trident in a ruin).
3. `/gamemode survival` near a trident drowned: it throws tridents at you. **Screenshot mob-16.** If you found a shell holder, kill it: it always drops the shell.

Zombie village zombie villagers are too rare to find on purpose; the tests cover them.

```
IW4L_MINECRAFT_KIT=minecraft:diamond_sword+sharpness=5,minecraft:shield,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*16,minecraft:cooked_beef*32,minecraft:torch*64
```

---

### Group `wolf-armour` (L): wolves wear, wear out and show wolf armour

**Gap:** mobs-23.

M1 leaves wolf armour for later (`track-m1.md:1935`, `:3145`), and wolves are an old store, not an M family.

**Cause:**
- **Data:** `Wolf` (`entities/src/wolf.rs:132-163`) has no body slot. `hurt_from` (`world/wolves.rs:63-90`) has no armour absorption.
- **Interaction:** `wolf_interact` (`wolves.rs:345`) cannot equip or mend armour. The tame branch is `:362-398`: food at `:363`, collar dye at `:373`. `MobHit::Wolf` (`crates/minecraft_terrain/src/mob_actions.rs:362`) has no shears arm. The snow golem's shears arm (`:303`) is the model.
- **Saving:** load (`server_mobs.rs:1046-1081`) and save (`:1636-1650`) handle no `equipment.body`. `body_equipment` (`:1476`) already does it for horses (`:1770`).
- **Death:** the wolves' loop in `take_deaths` (`world.rs:4798`) drops nothing extra. `equines::horse_death_equipment` (`equines.rs:164`) is the model.
- **Drawing:** `append_wolves` (`crates/minecraft_terrain/src/wolf_render.rs:183`) draws no armour layer. The entity texture list (`mesh.rs:564-565`) has only the collars.
- **Already there:** crafting and dyeing wolf armour (`crafting.rs:740`), anvil repair with scutes (`stations.rs:434`), durability from data (`RecipeBook::durability`, `player/src/crafting.rs:564`), and W's `EntityWorld::game_event`.

**Vanilla 26.3:**
- **`Wolf.mobInteract`, tame branch, in order:**
  - food heals while hurt;
  - an owner's dye recolours the collar;
  - an item equippable in `BODY` (wolf armour), when the wolf `!isWearingBodyArmor()`, is owned and is not a baby: `setItemSlotAndDropWhenKilled(BODY, copyWithCount(1))` and `consume(1)`;
  - when `isInSittingPose()`, wearing armour, owned, `isDamaged()` and `isValidRepairItem` (armadillo scute): `shrink(1)` (even in creative), `WOLF_ARMOR_REPAIR` (`item.wolf_armor.repair`), and damage `max(0, damage - (int)(maxDamage * 0.125))`;
  - then `TamableAnimal`'s branch.
- **`Wolf.actuallyHurt`:** when `canArmorAbsorb` (body item is `WOLF_ARMOR` and the source is not in `#bypasses_wolf_armor`), the whole amount goes to `hurtAndBreak(ceil(amount), BODY)` and the wolf loses no health. When `Crackiness.WOLF_ARMOR.byDamage` changes, it plays `WOLF_ARMOR_CRACK` (`item.wolf_armor.crack`) and sends 20 `armadillo_scute` item particles at y+1, spread (0.2, 0.1, 0.2), speed 0.1.
- **`Wolf.getHurtSound`:** `WOLF_ARMOR_DAMAGE` (`item.wolf_armor.damage`) when the armour absorbs.
- **Breaking:** `item.wolf_armor.break`, and the slot empties.
- **`Crackiness.WOLF_ARMOR`:** low, medium and high at 0.95, 0.69 and 0.32 of the durability left.
- **`#bypasses_wolf_armor` (26.3 data):**
  - `#bypasses_invulnerability` (`out_of_world`, `generic_kill`);
  - `cramming`, `drown`, `dry_out`, `freeze`, `in_wall`, `indirect_magic`, `magic`, `outside_border`, `starve`, `thorns`, `wither`.
- **Equip and unequip:** the equip sound is `item.armor.equip_wolf`. `Mob.interact` with shears, when `canShearEquipment(player)` (`Wolf`: `isOwnedBy`) and the player is not sneaking, shears the armour off: `item.armor.unequip_wolf`, the item dropped at the passenger attachment, game event `shear`, and 1 damage to the shears. Javap `Mob.attemptToShearEquipment` during the work.
- **Durability:** wolf armour has 64 (`Item.Properties.wolfArmor(ArmorMaterials.ARMADILLO_SCUTE)`).
- **Saving:** `equipment.body` with `drop_chances.body` 2.0.
- **Client:**
  - `WolfArmorLayer` draws on adults only. Its model is `AdultWolfModel.createBodyLayer(new CubeDeformation(0.2F))`, 64×32 (`LayerDefinitions`).
  - The equipment asset `armadillo_scute` has `wolf_body` layers `armadillo_scute` and the dyeable `armadillo_scute_overlay`; an undyed overlay is not drawn.
  - Cracks use `textures/entity/wolf/wolf_armor_crackiness_{low,medium,high}.png`.

**Plan: three commits.**

1. **"Minecraft tame wolves wear wolf armour like vanilla"**
   - `Wolf` gets `body_item: Option<String>` and `body_components: Option<serde_json::Value>` next to `collar`, as `HorseState` keeps them.
   - In `wolf_interact`'s tame branch, before the dye `match` (the items never overlap, so the order is vanilla's in effect), add the equip:
     - the held stack's components are copied;
     - the stack is consumed;
     - an `item.armor.equip_wolf` voice;
     - game event `equip` through `EntityWorld::game_event`. Add this one line yourself, because W's w3 covers only sites that exist before it.
   - A new `EntityWorld::shear_wolf_armour(id, player) -> Option<ItemStack>` (owner only). A shears arm before the general `MobHit::Wolf` (`mob_actions.rs:362`), shaped like `:303-313`, does the rest: no sneaking, drop, sound, `shear` game event, and the shears worn by 1.
   - Saving: `body_equipment` in the wolf save, plus `drop_chances.body` 2.0. Loading reads `equipment.body` the way horses do (`server_mobs.rs:654-655`).
   - Death: `take_deaths` pushes the armour into `death.equipment` when `mob_drops`.
   - Friends get all of this from the look tag; no protocol change.
2. **"Minecraft wolf armour takes a wolf's hits, cracks and breaks, and armadillo scutes mend it"**
   - Add a `BYPASSES_WOLF_ARMOR` list of the ids above in `wolf.rs`, the way `NO_ANGER_FROM_WIND_CHARGE` (`world/combat.rs:40`) is kept.
   - In `hurt_from`, when the body item is `minecraft:wolf_armor` and `kind` is not in the list, use `damage.hurt_absorbed` (`health.rs:84`). Its absorb closure hands the hit to the armour through a `Cell` and returns 0, so the cooldown rules stay vanilla's.
   - The armour's `minecraft:damage` then grows by `ceil(amount)`, against max 64 or the stack's `minecraft:max_damage`.
   - When the crack level changes, push an `item.wolf_armor.crack` voice. The scute crumbs are logged as left unless a server-to-client item particle path for mobs exists by then (M1's `ClientFx.item_crumbs` is client-side).
   - At max damage: an `item.wolf_armor.break` voice, and the slot clears.
   - The hurt voice is `item.wolf_armor.damage` while armour absorbs.
   - Mending goes after the equip branch: sitting, owned, damaged, with a scute; `shrink 1`, an `item.wolf_armor.repair` voice, damage minus `(int)(max * 0.125)`.
   - Every hurt path (melee, arrows, blasts, hazards through `exposed!` in `hazards.rs:228`) already goes through `hurt_from`.
3. **"Minecraft wolves are drawn with their armour, its dye and its cracks"**
   - In `append_wolves`, for an adult with a body item, add a second pass over `ADULT` with every cube grown by 0.2:
     - `entity/equipment/wolf_body/armadillo_scute` (white);
     - `entity/equipment/wolf_body/armadillo_scute_overlay` tinted by `dyed_color` (`crate::body_gear::dye_in`, `armor_render::dye_tint`), drawn only when dyed;
     - the crack texture for its level.
   - Add the five textures to `mesh.rs` after `:565`.
   - The host and friends draw the same thing from the loaded components.

**Tests:**
- `world::wolves::tests`:
  - `an_owner_puts_wolf_armour_on_a_tame_adult_but_not_on_a_puppy_a_wild_wolf_or_another_player_s_wolf`;
  - `wolf_armour_takes_the_hit_and_the_wolf_keeps_its_health`;
  - `drowning_starving_magic_and_wither_pass_the_armour`;
  - `the_armour_cracks_at_95_69_and_32_percent_with_the_crack_sound`;
  - `the_armour_breaks_with_its_sound_and_the_next_hit_hurts_the_wolf`;
  - `a_scute_mends_an_eighth_only_while_the_wolf_sits`;
  - `an_armoured_wolf_s_hurt_sound_is_the_armour_s`;
  - `shears_take_the_armour_off_for_its_owner_only`;
  - `a_wolf_drops_its_armour_when_it_dies`.
- `server_mobs::tests::a_wolf_saves_and_loads_its_armour_with_damage_and_dye` (`equipment.body`, `drop_chances.body` 2.0).
- `mob_tracking::tests::a_friend_sees_wolf_armour_from_the_look_tag`.
- `mob_actions::tests::shears_on_an_armoured_wolf_drop_the_armour_and_wear_the_shears`.
- `wolf_render::tests::an_armoured_wolf_draws_plates_dye_and_cracks_and_a_puppy_none` (vertex counts per layer).

**The owner's steps** (survival, Normal, flat ground; full iron armour):

1. Hatch two wolves and tame both with bones (hearts, red collars).
2. Right-click the first with wolf armour: you hear it go on, and the wolf wears grey-brown scute plates over its body, head and legs. Right-click the second with the lime wolf armour: its plates have lime trim. **Screenshot mob-17.**
3. Hatch a third wolf and right-click it with armour without taming it: nothing happens.
4. `/time set 13000` and hatch four zombies 8 blocks away. The wolves fight them. The zombies' hits crack the plates, thin cracks first and then wider ones, with a cracking sound, and the wolves' tails stay up because their health is untouched. **Screenshot mob-18.** Kill the zombies.
5. Make the cracked wolf sit (right-click it with an empty hand), then right-click it with armadillo scutes. Each scute plays the mending sound, and the cracks shrink until they are gone. A standing wolf takes no scute. **Screenshot mob-19.**
6. Save and quit, then load: both wolves still wear their armour, the lime trim and any cracks. **Screenshot mob-20.**
7. Stand up straight (no crouch) and right-click the lime wolf with shears: the armour drops as an item with the unequip sound, and it is still lime when you pick it up. **Screenshot mob-21.** Put it back on.
8. Hatch more zombies and let them break the first wolf's armour: a breaking sound, and the plates vanish. From then on the wolf flashes red and its tail drops as it loses health. **Screenshot mob-22.**
9. Friend (two windows): the friend sees both wolves' plates, the lime trim and the cracks the same way, then tames a wolf and armours it. **Screenshot mob-23-friend.**

```
IW4L_MINECRAFT_KIT=minecraft:wolf_spawn_egg*4,minecraft:bone*64,minecraft:wolf_armor,minecraft:wolf_armor,minecraft:wolf_armor@lime,minecraft:armadillo_scute*16,minecraft:shears,minecraft:zombie_spawn_egg*8,minecraft:cobblestone*64,minecraft:torch*32,minecraft:iron_sword,minecraft:shield,minecraft:iron_helmet,minecraft:iron_chestplate,minecraft:iron_leggings,minecraft:iron_boots,minecraft:golden_apple*8,minecraft:cooked_beef*32
```

---

### Group `rabbit-nibble` (S): a rabbit's bite on a carrot shows crumbs and makes a vibration

**Gaps:** mobs-25, sculk-29. **Segment e3z; waits for M1's m1c merge** (foxes-hunt commit 1 adds `World::destroy_effect`).

**Cause**
- M1's m1a applies a rabbit's bite where the rabbit's tick drains `RabbitGoals.nibble`: `level.set_block(pos, crop)` (`third_party/minecraftoss/entities/src/world/families/land/rabbits.rs:499-501`, at 80aabd8e), with `crop` from `rabbits/ai.rs:319` `nibbled` (`None` at age 0, else age minus 1).
- It posts no game event and plays no level event. M1's log leaves both "for later" (`track-m1-log.md`, "Left for later": 2001 waits for foxes' `World::destroy_effect`; the `block_change` line goes to whichever of W and M1 merges second). Both W's helpers (w1, 81eaed3f) and m1a (a898c41c) have merged, and no M1 segment (m1b-m1i) or W group (w3) plans the lines.

**Vanilla 26.3** (javap `Rabbit$RaidGardenGoal.tick`)
- Age 0: `setBlock(pos, AIR, 2)`, then `destroyBlock(pos, true, rabbit)` on the air it just set, which does nothing. No event.
- Age above 0: `setBlock(pos, state with AGE - 1, 2)`, then `gameEvent(BLOCK_CHANGE, pos, Context.of(rabbit))`, then `levelEvent(2001, pos, Block.getId(state))` with the state before the bite.

**Plan: one commit, "Minecraft rabbits biting carrots show crumbs and make vibrations"**
- In `rabbits.rs:499-501`, read the carrot before `level.set_block(pos, crop)`. When `crop` is `Some` (a bite that took one age), after the set:
  - `world.game_event("minecraft:block_change", <block centre>, Some(id), None)` (W's helper, one line, the rabbit as source, no affected state as in `Context.of(rabbit)`);
  - `level.destroy_effect(pos, <the carrot before the bite>)` (M1's helper from m1c, one line; vanilla's level event 2001 data is the old state).
- Age 0 stays silent, as in vanilla.
- These are the two lines M1's brief leaves to the track that merges second (W0 rule 3); nothing else in M1's rabbit files changes.

**Tests** (vendored `rabbits/tests.rs`, beside the carrot raid tests)
- `a_rabbit_s_bite_posts_block_change_and_a_break_effect`: one bite on an age 7 carrot gives age 6, one `block_change` from the rabbit at the block centre and one destroy effect with the age 7 state.
- `eating_the_last_age_makes_no_event`: an age 0 carrot becomes air with no game event and no destroy effect.

**Owner's steps** (survival, by day, flat grass; as M1's carrot raid steps)
1. Hoe a 3 x 3 patch beside water, plant carrots and bone-meal them full grown. Place a sculk sensor 3 blocks from the patch with a comparator out of it into a line of 15 redstone dust.
2. Hatch two rabbits beside the patch and stand 10 blocks away, crouched and still.
3. When a rabbit bites a carrot, the carrot shrinks with a burst of crop crumbs and the crop break sound, and the dust line lights (`block_change`, frequency 11). Before this change the carrot shrank silently. **Screenshot mob-24.**
4. Friend (two windows): the friend sees the crumbs too.

```
IW4L_MINECRAFT_KIT=minecraft:rabbit_spawn_egg*4,minecraft:carrot*32,minecraft:diamond_hoe,minecraft:bone_meal*32,minecraft:water_bucket,minecraft:sculk_sensor*2,minecraft:comparator,minecraft:redstone*32,minecraft:cooked_beef*16
```
9 stacks.

---

### Left out (in the log and in MINECRAFT-SURVIVAL.md's limits)

- **mobs-34, the visibility factor.** See the left list.
- **mobs-9, the zombie nautilus jockey of ruin drowned.** This waits for M2's m2h.
- **Endermen ignore a carved pumpkin.** Found while researching mobs-34: `crates/minecraft_survival/src/entities.rs:1668` always builds `PlayerView` with `disguised: false`. It is not on this area's list; see the left list.

## E3-12. Items, tooltips, kits, crafting, effects and loot

This area closes items-1, items-2, items-5 to items-11, items-16 to items-22, items-24 to items-27 and items-29 of the known gaps in the release test files (`docs/round10/release-2-tests.md:560-562`, `release-4-tests.md:787`, `release-6-tests.md:420` and `:437`, and the earlier lists those files point to). items-4 is left out (see "Left out" at the end).

Paths and lines are as of `minecraft-survival` at 44d96765, which has e2b merged; its code tree is the same as d565a3d2's. D0's d0b merge (c0366f3a) has moved some lines since, and W's w3 and the M tracks will move more, so check them again before editing. Vanilla facts come from javap on the 26.3 `client.jar` and from the 26.3 data under `MINECRAFTOSS_ROOT` (`datapacks/local/minecraft-26.3/data/minecraft`).

**Rules for every group**
- No code comments of any kind.
- Edits change only the lines they need, in the surrounding style. New files pass `rustfmt --check` on their staged content.
- New enum variants and match arms go next to related ones, never at the end of a shared enum.
- Never bump `minecraft_net::PROTOCOL`. A wire change goes under "## Protocol" in `docs/round10/track-e3-log.md`. No group here should need one: mob names travel in the mob tags friends already get, `EffectView.kind` is a string (`minecraft_net/src/messages.rs:470-475`), and tooltips are built on each client.
- No Mojang data is committed. Rarities, tags, loot and recipes are read at runtime.
- Decisions go in the track log.

**Ownership this area respects**
- **M2** owns `EffectKind` and its `ALL` length line this round (`third_party/minecraftoss/player/src/survival.rs:36-82`; track-m2.md:93, :1204, :2733). Every new player effect therefore waits for M2's last change there, `nautiluses-riding` in segment m2i. That is group `trial-effects-players`.
- **M1** owns the loot-context rewrite: `EntityLootContext.damage_type` after `has_direct_attacker`, and `damage_source_properties` (`entities/src/loot.rs:19`, `:636-646`; track-m1.md:93, :2184). `tool-enchantments` only fills the existing `looting` field (`:18`) and edits the two enchanted arms (`:523`, `:628-633`).
- **M1's bees** (m1i `bees-hives`) own the hive storage in `world/src/level/beehive.rs`, the bees entering and leaving, and the release on break with its Silk Touch exception. `heads-hives` keeps only the heads; the silk-touched hives are group `hive-bees`, which runs in e3z after m1i and builds on M1's code.
- **W's w3** adds lead-knot game events next to `crates/minecraft_terrain/src/mob_actions.rs:163-179`. `name-tags` and `tool-enchantments` edit beside those lines, so if w3 has merged, merge `minecraft-survival` into the track before those groups.

---

### Group `firework-looks` (S), 2 commits

**Gaps:** items-7 (a firework star's icon stays grey) and items-8 (no firework tooltip lines). E2's `fw-craft` listed both under "Left for later".

**Cause**
- `crates/minecraft_terrain/src/interface.rs:81-84`: the `minecraft:firework` tint source falls back to its `default` (-7697782, grey), the same as `map_color`, `team` and `custom_model_data`.
- `crates/minecraft_terrain/src/item_tint.rs:28-33`: `of_stack_at` takes a colour only from `potions::stack_color` or `body_gear::dyed_color`. A star's `minecraft:firework_explosion` never reaches the inventory icon, the held item or a dropped item.
- `crates/render_anim/src/minecraft_inventory.rs:465-490` builds `McStack` with `lore: book_lines(..)` (`:992-1006`) and `effects: effect_lines(..)` (`:922`, the ominous bottle only). Nothing reads `minecraft:fireworks` or `minecraft:firework_explosion`.
- `crates/minecraft_terrain/src/firework.rs:40-124` already parses both: `Explosion::from_json`, `fireworks()`, and `flight_duration()` with the item default of 1.

**Vanilla 26.3**
- `net.minecraft.client.color.item.Firework.calculate`: it takes the colours of `FIREWORK_EXPLOSION` (none when the component is missing).
  - No colours: `defaultColor`.
  - One colour: `ARGB.opaque(c)`.
  - More: the integer average of red, green and blue, made opaque with `ARGB.color(r, g, b)`.
  - `firework_star.json` applies it to tint index 1, after a constant -1.
- `Fireworks.addToTooltip`:
  - When `flightDuration > 0`: a grey `item.minecraft.firework_rocket.flight_duration` line ("Flight Duration: %s").
  - Then each run of equal consecutive explosions goes through `addExplosionTooltip(explosion, count)`: grey `item.minecraft.firework_rocket.single_star` ("%s") or `.multiple_stars` ("%s x %s", count first) with the shape name, then the star's extra lines, each prefixed by two spaces (`lambda$addExplosionTooltip$0`: `Component.literal("  ")`).
- `FireworkExplosion.addToTooltip` (on a star) gives the grey shape name `item.minecraft.firework_star.shape.<shape>`, then `addAdditionalTooltip`, all grey:
  - the colours (when any), joined with `", "` (`appendColors`); each is `item.minecraft.firework_star.<dye>` when `DyeColor.byFireworkColor` finds one, else `item.minecraft.firework_star.custom_color` ("Custom");
  - `item.minecraft.firework_star.fade_to` + `CommonComponents.SPACE` + the fade colours (when any);
  - `item.minecraft.firework_star.trail` ("Trail");
  - `item.minecraft.firework_star.flicker` ("Twinkle").
- `Items.FIREWORK_ROCKET` has the default component `new Fireworks(1, List.of())`, so a plain rocket reads "Flight Duration: 1".

**Plan**
1. **"Minecraft firework stars take their colour on the icon like vanilla"**
   - `firework.rs`: `pub fn star_color(stack: &ItemStack) -> Option<u32>`. It returns None without `firework_explosion` colours, so a plain star keeps the grey default and the plain icon key. One colour gives that colour; more give vanilla's integer average.
   - `item_tint.rs:29-30`: add `.or_else(|| crate::firework::star_color(stack))` after `dyed_color`. The colour becomes part of the icon key, as for potions, so the inventory, the held item (`minecraft_hand.rs`) and dropped items all follow.
   - `interface.rs:81-84`: move `"minecraft:firework"` into the `"minecraft:potion" | "minecraft:dye"` arm, which uses the passed colour or else the default.
2. **"Minecraft firework rockets and stars list their flight and stars in the tooltip"**
   - `render_anim/src/minecraft_inventory.rs`: add `fn firework_lines(language, stack) -> Vec<String>` next to `book_lines`, built from `firework::fireworks` and a new `firework::explosion_of(stack)` for stars.
   - The `lore` field line chains `book_lines` and `firework_lines` (one changed line), so the lines are grey and sit before the enchantment lines, as vanilla's order has them.
   - Colour names come from the DyeColor firework colours in `third_party/minecraftoss/player/src/crafting.rs:60` (`DYE_COLORS`, which becomes `pub`).

**Tests**
- `item_tint::tests`:
  - a red large-ball star's key carries `~b3312c`;
  - a yellow (14602026, `0xDECF2A`) and blue (2437522, `0x253192`) star's key carries the integer average `~81805e` (red 259/2 = 129, green 256/2 = 128, blue 188/2 = 94; `Firework.calculate` uses `idiv`);
  - a plain star's key is `minecraft:firework_star`.
- `interface::tests`: the star model's tint 1 takes the passed colour, and tint 0 stays white.
- `render_anim minecraft_inventory::tests`:
  - the red star: "Large Ball", "Red", "Trail";
  - a two-colour star: "Star-shaped", "Yellow, Blue", "Fade to Lime", "Twinkle";
  - a 1234567 star: "Custom";
  - a plain rocket: "Flight Duration: 1";
  - three equal white small balls at flight 1: "Flight Duration: 1", "3 x Small Ball", "  White";
  - a two-star rocket shows both stars with indented lines;
  - `fireworks={flight_duration:0}` shows no lines.

**Owner's steps** (survival, by day, the kit below)
- **FL-1. Icons.** Open the inventory. The four star stacks are grey (plain), red, olive (the average of yellow and blue) and teal-green (custom 1234567). **Screenshot FL-1.**
- **FL-2. Star tooltips.**
  - Hover the red star: "Firework Star", then grey "Large Ball", "Red", "Trail".
  - The olive star: "Star-shaped", "Yellow, Blue", "Fade to Lime", "Twinkle".
  - The teal star: "Creeper-shaped", "Custom".
  - The plain star: the name only.
  - **Screenshot FL-2** (the olive star).
- **FL-3. Rocket tooltips.**
  - The plain rocket: "Flight Duration: 1".
  - The two-star rocket: "Flight Duration: 2", "Star-shaped", "  Yellow", "  Fade to Blue", "Creeper-shaped", "  Lime", "  Twinkle".
  - The white rocket: "Flight Duration: 1", "3 x Small Ball", "  White".
  - **Screenshot FL-3** (the two-star rocket).
- **FL-4. Held and dropped.** Hold the red star: it is red in your hand. Press Q: the dropped star is red on the ground.
- **FL-5. Crafted.** At the crafting table, gunpowder and lime dye make a lime star. Put it in the grid with blue dye: the star stays lime, and its tooltip gains "Fade to Blue". **Screenshot FL-5.**
- **FL-6. Friend** (two windows, `scripts\mp-local.ps1`, `/kit @a` from the host). The friend opens the inventory: the same colours and tooltips appear in the friend's window. **Screenshot FL-6**, the friend's window.

```
IW4L_MINECRAFT_KIT=minecraft:firework_star*4,minecraft:firework_star[firework_explosion={shape:large_ball,colors:[I;11743532],has_trail:1b}]*4,minecraft:firework_star[firework_explosion={shape:star,colors:[I;14602026,2437522],fade_colors:[I;4312372],has_twinkle:1b}]*4,minecraft:firework_star[firework_explosion={shape:creeper,colors:[I;1234567]}]*4,minecraft:firework_rocket*16,minecraft:firework_rocket[fireworks={flight_duration:2,explosions:[{shape:star,colors:[I;14602026],fade_colors:[I;2437522]},{shape:creeper,colors:[I;4312372],has_twinkle:1b}]}]*16,minecraft:firework_rocket[fireworks={flight_duration:1,explosions:[{shape:small_ball,colors:[I;15790320]},{shape:small_ball,colors:[I;15790320]},{shape:small_ball,colors:[I;15790320]}]}]*16,minecraft:gunpowder*16,minecraft:lime_dye*8,minecraft:blue_dye*8,minecraft:paper*8,minecraft:crafting_table
```

---

### Group `component-limits` (S), 3 commits

**Gaps:** items-9 and items-10 (the E1 E-kit decisions, track-e1-log.md:38-39; `release-2-tests.md:560`) and items-5.

**Cause**
- **items-9.** `third_party/minecraftoss/world/src/item_components.rs:142-163` `layer()` accepts any well-formed id through `identifier()` (`:112-140`).
  - `/give` checks only items and enchantments against `CommandRegistries` (`crates/minecraft_commands/src/server.rs:59-64`; `commands.rs:612-640`).
  - The kit checks only the item id (`kit.rs:85-88`; `minecraft_terrain/src/crafting.rs:510-513` `kit_components`).
  - An unknown pattern is kept and draws nothing. The registry is the data folder `data/minecraft/banner_pattern` (43 files, `special_models.rs:1636-1653`).
- **items-10.** `item_components.rs:39` sets `MAX_FLIGHT` to 255, and `:66-67` refuses other values with `in_range(number(flight), 0, MAX_FLIGHT)`. The test near `:323` expects "Value 256 outside of range [0:255]".
- **items-5.** Book text is counted in Unicode characters everywhere:
  - `crates/minecraft_survival/src/books.rs:51-53` checks pages and the title with `chars().count()`;
  - `render_anim/src/minecraft_inventory.rs:199` truncates with `chars().take(PAGE_LENGTH)`;
  - `hud/src/minecraft_book_edit.rs:211` counts room in chars;
  - the shared `hud/src/minecraft_edit.rs:64-79` `Edit::insert` counts and takes chars. Chat, creative search and recipe search use it too.

**Vanilla 26.3**
- `BannerPattern.CODEC` is `RegistryFileCodec.create(Registries.BANNER_PATTERN, DIRECT_CODEC)`.
  - An id outside the registry fails in `RegistryFileCodec.decode` with "Failed to get element <id> from registry minecraft:banner_pattern" (javap `lambda$decode$4`: `ResourceKey.identifier()`, then `registry()`).
  - `/give` then answers `arguments.item.component.malformed` for `minecraft:banner_patterns`.
  - Inline `{asset_id, translation_key}` patterns are direct and always pass.
- `Fireworks.CODEC` reads `flight_duration` with `ExtraCodecs.UNSIGNED_BYTE`, which is `Codec.BYTE.flatComapMap(...)` (`ExtraCodecs.<clinit>` :405-423). `Codec.BYTE` decodes any number through `Number.byteValue`, so 256 gives 0, 300 gives 44 and -1 gives 255.
- `ServerboundEditBookPacket.STREAM_CODEC`: pages are `stringUtf8(1024)` in a list of 100, and the title is `stringUtf8(32)`; `Utf8String` compares `String.length()`, which counts UTF-16 code units.
- `EditBox.insertText` and `MultilineTextField.truncateInsertionText` (through `StringUtil.truncateStringIfNecessary`) also count `String.length()`, and truncation never splits a surrogate pair. So an emoji outside the BMP counts as 2.

**Plan**
1. **"Minecraft item commands and kits refuse banner patterns the game does not have"**
   - Add `CommandRegistries.banner_patterns: Vec<String>` after `enchantments` (`server.rs:63`).
   - Fill it where the enchantments are filled (`render_anim/src/minecraft_world.rs:595`), from the file stems of `DataPaths::under(root).datapack/data/minecraft/banner_pattern`.
   - `give` (`commands.rs`, after the enchantment check at `:627-638`) refuses every layer whose `pattern` is an id outside the list, with `arguments.item.component.malformed`, `minecraft:banner_patterns` and the vanilla reason.
   - The kit loop (`kit.rs:85-88`) prints the same line and skips the entry.
   - An empty list, when no data was found, accepts everything, as `registered` (`server.rs:101`) does.
2. **"Minecraft item commands wrap a firework flight above 255 like vanilla"**
   - `item_components.rs:66-67` takes any number and wraps it like `byteValue` and `toUnsignedInt` (`(n as i8) as u8`).
   - The 0 default stays left out of the stored JSON, so 256 stores nothing.
   - `MAX_FLIGHT` goes if nothing else uses it. The kit uses the same parser, so it follows.
3. **"Minecraft book pages and titles count characters like vanilla"**
   - Add two helpers in `hud/src/minecraft_edit.rs`: the UTF-16 length, and a take-up-to-n-units that never splits a surrogate pair.
   - Use them in `Edit::insert` (`:64-79`) and in the book editor's `insert` (`minecraft_book_edit.rs:211`).
   - Also use them for the truncation in `render_anim/src/minecraft_inventory.rs:199` (render_anim keeps its own copy or reuses the hud one, whichever its dependencies allow) and for `books.rs:51-53` (`encode_utf16().count()`).
   - `TITLE_LIMIT` stays 15 in the editor (`minecraft_book_edit.rs:15`) and 32 on the server (`books.rs:10`), as in vanilla. Chat and search fields follow `EditBox` too.

**Tests**
- `minecraft_commands`:
  - `/give` of `banner_patterns=[{pattern:"minecraft:no_such",color:"red"}]` gives the malformed error with "Failed to get element minecraft:no_such from registry minecraft:banner_pattern";
  - `creeper`, `minecraft:border` and an inline pattern pass;
  - a kit entry with an unknown pattern is skipped with that line;
  - an empty registry accepts.
- `minecraftoss-world item_components`:
  - flight 256 stores no `flight_duration`; 300 stores 44; -1 stores 255; 255 stays;
  - a string is still malformed.
  - The test near `:323` is rewritten for the wrap.
- `books`:
  - 512 four-byte emoji on a page pass and 513 fail;
  - 16 emoji in the title pass and 17 fail.
- `hud`:
  - `Edit::insert` of "😀😀" into a field with 3 units left adds one emoji, not half of a second;
  - the book editor's title takes 7 emoji.

**Owner's steps** (survival, the kit below, after `firework-looks` for the tooltip)
- **CL-1. A pattern that doesn't exist.** `/give @s minecraft:white_banner[banner_patterns=[{pattern:"minecraft:no_such",color:"red"}]]`: a red error appears and nothing is given. It names `minecraft:banner_patterns` and says "Failed to get element minecraft:no_such from registry minecraft:banner_pattern". **Screenshot CL-1.** `/give @s minecraft:white_banner[banner_patterns=[{pattern:"creeper",color:"red"}]]` gives a banner; place it to see a red creeper face.
- **CL-2. Flight wraps.**
  - `/give @s minecraft:firework_rocket[fireworks={flight_duration:300}]` gives a rocket with no error. Its tooltip reads "Flight Duration: 44". **Screenshot CL-2.**
  - The same with `256`: no "Flight Duration" line.
  - With `-1`: "Flight Duration: 255".
- **CL-3. Emoji count twice.**
  - In Notepad, open the emoji panel with Win+. and insert 😀, then copy and paste until there are 10 of them. Copy the 10.
  - In the game, open a book and quill and press Ctrl+V on the page: 10 emoji go in (each may show as a box).
  - Click Sign and press Ctrl+V in the title: only 7 go in, which is 14 of the field's 15 units.
  - Sign the book. Its name is the 7 emoji. **Screenshot CL-3.**
- **CL-4. Friend.** The friend does CL-3 in the friend's window: again 7 go in, and the signed book keeps them on the host. **Screenshot CL-4**, the friend's window.

```
IW4L_MINECRAFT_KIT=minecraft:writable_book*4,minecraft:white_banner*4,minecraft:firework_rocket*16,minecraft:cooked_beef*16
```

---

### Group `item-rarity` (M), 2 commits

**Gap:** items-11 ("Tooltips have no rarity colours, so the ominous bottle's name is white", `release-2-tests.md:562`).

**Cause**
- `crates/hud/src/minecraft_hotbar.rs:52-58`: `name_color` returns `AQUA` for an enchanted stack and `WHITE` otherwise. Both the tooltip (`hud/src/minecraft_inventory.rs:894`) and the held item's name (`minecraft_hotbar.rs:222-226`) use it.
- `McStack` (`crates/frame/src/minecraft_ui.rs:33-52`) has no rarity. render_anim fills it at `minecraft_inventory.rs:465-490`.
- The `minecraft:rarity` component is never read. It is stored on ominous banners (`world/src/level/components.rs:11-17`).
- The item catalog (`crates/assets/data/minecraft/item-catalog-26.3.json.gz`, 588 sampled items) has no rarity field.

**Vanilla 26.3**
- `ItemStack.getRarity`: the `RARITY` component, default `COMMON`. When `isEnchanted()` (a non-empty `ENCHANTMENTS`; stored enchantments don't count), `COMMON` and `UNCOMMON` become `RARE`, and `RARE` becomes `EPIC`.
- `ItemStack.getStyledHoverName`: the hover name in `Rarity.color()` (white, yellow, aqua, light purple), plus `ITALIC` when `CUSTOM_NAME` is set.
- The defaults come from `Item$Properties.rarity` in `Items.<clinit>`: 115 `Rarity` references, 113 straight in the init and the rest in property lambdas such as `lambda$static$134` (UNCOMMON) handed to `registerBlock`. Examples:
  - EPIC: elytra, mace, heavy core, dragon egg, dragon head, command block.
  - RARE: trident, beacon, nether star, enchanted book, enchanted golden apple, wither skeleton skull.
  - UNCOMMON: totem of undying, heart of the sea, bottle o' enchanting, ominous bottle, creeper head, conduit, music discs.
  - COMMON: the golden apple (it has no rarity call).

**Plan**
1. **"Minecraft reads each item's rarity from the game jar"**
   - New `crates/minecraft_terrain/src/item_rarity.rs` with `ItemRarities::read_jar(jar)`. It runs `Items.<clinit>` through `class_file` the way `block_map_colors.rs` runs `Blocks` (its `Machine`, `:305-360`).
     - Each `putstatic Items.<FIELD>` takes its id from the `ItemIds`/`BlockItemIds` key loaded for it; `block_map_colors.rs:14` already resolves `BlockItemIds`.
     - Its rarity is the last `getstatic Rarity.<R>` followed by `Item$Properties.rarity` in its property chain, including inside the `Function`/`UnaryOperator` lambda it passes to `registerItem`/`registerBlock`.
   - `rarity(id) -> Rarity`, with Common as the default.
   - Loaded once in render_anim's inventory `publish` next to `GuiAssets::load` (`minecraft_inventory.rs:431-442`), from `minecraft_world::data_jar()`. Friends read their own jar.
   - No table is committed.
2. **"Minecraft item names take their rarity colour like vanilla"**
   - `McStack.rarity: u8` (0-3) after `named`. render_anim sets it from `minecraft:rarity` (`"common"` to `"epic"`) or from the table. When `stations::enchantments_of(s)` is not empty it follows `ItemStack.getRarity` (javap): COMMON and UNCOMMON become RARE (aqua), RARE becomes EPIC, EPIC stays.
   - `name_color` maps it to `WHITE`, `YELLOW`, `AQUA` or a new `LIGHT_PURPLE` (0xFFFF55FF, next to `AQUA` at `hud/src/minecraft_gui.rs:21`).
   - The italic still comes from `named`.

**Tests**
- `item_rarity::tests` (skipped without `MINECRAFTOSS_ROOT`, like the map colour test):
  - elytra epic; trident, enchanted book and wither skeleton skull rare (the skull comes through `registerBlock`);
  - totem and ominous bottle uncommon; golden apple and dirt common;
  - more than 100 non-common ids.
- `hud minecraft_hotbar::tests`: the four colours; enchanted common gives aqua, enchanted rare gives purple, enchanted epic stays purple.
- `render_anim`:
  - the ominous bottle is yellow;
  - an ominous banner (`minecraft:rarity` uncommon) is yellow;
  - an enchanted iron sword is aqua and an enchanted trident purple;
  - a renamed elytra is purple and italic.

**Owner's steps** (survival, the kit below)
- **RA-1. Base colours.** Open the inventory and hover each item:
  - elytra, heavy core and dragon egg: light purple;
  - trident, nether star, beacon, wither skeleton skull and the enchanted book: aqua;
  - totem of undying, bottle o' enchanting, ominous bottle and creeper head: yellow;
  - golden apple and diamond sword: white.
  - **Screenshot RA-1** (the elytra's tooltip).
- **RA-2. Enchanted.** The iron sword with Sharpness is aqua, the trident with Loyalty is light purple, the elytra with Unbreaking stays light purple, and the totem with Unbreaking is aqua. **Screenshot RA-2** (the trident).
- **RA-3. Held name.** Select the elytra in the hotbar: the name above the hotbar is light purple. Then select the totem: yellow.
- **RA-4. Renamed.** Throw bottles o' enchanting until you have 2 levels. Rename the trident "Spear" in the anvil: its tooltip shows "Spear" in italic aqua. **Screenshot RA-4.**
- **RA-5. Friend.** After `/kit @a`, the friend sees the same colours in the friend's window.

```
IW4L_MINECRAFT_KIT=minecraft:elytra,minecraft:totem_of_undying,minecraft:trident,minecraft:nether_star,minecraft:experience_bottle*16,minecraft:ominous_bottle,minecraft:creeper_head,minecraft:wither_skeleton_skull,minecraft:heavy_core,minecraft:beacon,minecraft:dragon_egg,minecraft:enchanted_book,minecraft:golden_apple*4,minecraft:diamond_sword,minecraft:iron_sword+sharpness=3,minecraft:trident+loyalty=3,minecraft:elytra+unbreaking=3,minecraft:totem_of_undying+unbreaking=1,minecraft:anvil
```

---

### Group `book-dye-recipes` (M), 2 commits

**Gaps:** items-2 (book cloning) and items-16 (dyeing armour in the grid; E1 left it out, track-e1-log.md:696). Both build on e2b's special recipe framework, which is now merged.

**Cause**
- `third_party/minecraftoss/player/src/crafting.rs:1239-1310` `parse_recipe` knows shaped and shapeless recipes plus e2b's four special kinds (`Kind` `:35-58`, parse arms `:1273-1300`). It drops `crafting_special_bookcloning` and the six `crafting_dye` recipes (leather helmet, chestplate, leggings and boots, leather horse armour and wolf armour; `dye` `#minecraft:dyes`, group `dyed_armor`).
- `special_result` (`:899-1069`) builds e2b's results. `transmuted` (`:1369`) is vanilla's `createWithOriginalComponents` with the result's count.
- Remainders are per item id only: `crafting_remainder` (`:635-641`) is used by `consume_crafting_ingredients` (`inventory.rs:614-656`). The source book can't stay.
- The dye mix exists as `apply_dyes` with the `DyeColor.getTextureDiffuseColor` table (`third_party/minecraftoss/entities/src/trading.rs:171-173`, `:583-600`, used by `set_random_dyes` `:495-502`). `entities` depends on `player`, so the player crate can't call it.
- Books carry `minecraft:written_book_content` with `generation` (`books.rs:6`). The tooltip shows it already (`render_anim minecraft_inventory.rs:1002-1004`).

**Vanilla 26.3**
- `BookCloningRecipe.matches`:
  - `ingredientCount() >= 2`;
  - every item is either the source or a material, else it fails. The source is a `written_book` with `WRITTEN_BOOK_CONTENT` whose generation is in `allowed_generations` (default `MinMaxBounds.Ints.between(0, 1)`); a second source fails. The material is `#book_cloning_target`, which is the writable book only;
  - it needs a source and at least one material.
- `BookCloningRecipe.assemble`: `TransmuteRecipe.createWithOriginalComponents(result, source, materials - 1)`, whose count is the result's 1 plus the extra, so one copy per writable book. Then `WRITTEN_BOOK_CONTENT = content.craftCopy()`, with the generation plus 1.
- `BookCloningRecipe.getRemainingItems`: an item's crafting remainder, else `copyWithCount(1)` of the stack that has `WRITTEN_BOOK_CONTENT`.
- `DyeRecipe`: exactly one target and at least one item with the `minecraft:dye` component; anything else fails. `assemble` calls `DyedItemColor.applyDyes` (the target's `dyed_color` averaged with each dye's texture colour, scaled back to the average brightness), then `createWithOriginalComponents`. It is a `NormalCraftingRecipe` shown in the book (group `dyed_armor`).

**Plan**
1. **"Minecraft written books are copied in the crafting grid like vanilla"**
   - `Kind::BookCloning { source, material, generations: (i64, i64) }` after `ShieldDecoration` (`:54-57`). It is parsed from `minecraft:crafting_special_bookcloning`, with `allowed_generations` `min`/`max`, default 0-1, next to the `:1297` arm.
   - A `special_result` arm after `ShieldDecoration` applies the rules above. The count is the number of writable books; the copy keeps the source's other components and its content with `generation + 1`.
   - Add `RecipeBook::crafting_remainders(grid, width, height) -> Vec<Option<ItemStack>>`:
     - for the matched book cloning recipe, the source slot keeps the source with count 1;
     - otherwise each slot gets `crafting_remainder(id)` as today.
   - `consume_crafting_ingredients` asks it once before its loop.
2. **"Minecraft leather armour and wolf armour are dyed in the crafting grid like vanilla"**
   - Move `apply_dyes` and the texture colour table to `player/src/crafting.rs` as `pub`. `trading.rs` imports them, and its uses keep their lines.
   - `Kind::Dye { target, dye }` is parsed from `minecraft:crafting_dye`. The dye name is found like `dye_color` (`:1324-1337`): the `minecraft:dye` component or the `<colour>_dye` id.
   - The result is `transmuted(result, target)` with `minecraft:dyed_color` set to `apply_dyes(current, dyes)`.
   - Recipe book entries are left for later (see Left out).

**Tests**
- `crafting::tests`, book cloning:
  - a source and 3 writable books give 3 copies with generation 1 and the same title and pages;
  - generation 1 gives 2; generation 2 gives nothing;
  - two sources fail; a source with a plain `minecraft:book` fails; a stick in the grid fails; a source alone fails.
- `inventory::tests`: taking the copy leaves the original in its slot (count 1) and uses one book and quill per slot. Shift-taking copies until the writable books run out.
- `crafting::tests`, dyeing:
  - a leather helmet and red dye give `dyed_color` 11546150;
  - a red helmet and yellow dye give the vanilla mix;
  - two dyes mix in one craft;
  - leather horse armour and wolf armour take dye;
  - two targets fail; a target alone fails; a non-dye fails;
  - the result keeps the target's name and damage.
- `trading`: `set_random_dyes`'s existing test still passes.

**Owner's steps** (survival, a crafting table, the kit below)
- **BR-1. One copy.** Put the "Kit" book and one book and quill in the grid: the result is a written book "Kit". Take it: the original stays in its slot and the book and quill is used. The copy's tooltip reads "Copy of original". **Screenshot BR-1.**
- **BR-2. Three copies.** The original with three books and quill (in three slots) gives 3 copies at once.
- **BR-3. Copy of a copy.** A copy with a book and quill gives "Copy of a copy". That one with a book and quill gives nothing. **Screenshot BR-3** (the empty result).
- **BR-4. Dyed helmet.** A leather helmet and red dye give a red helmet. Put it on: your MW2 body shows no helmet, but its icon is red. The red helmet with yellow dye gives an orange helmet. **Screenshot BR-4.**
- **BR-5. Two dyes.** A leather chestplate with blue and white dye gives a light blue chestplate.
- **BR-6. Horse armour.** Leather horse armour and lime dye give lime armour. Hatch a horse, tame it (keep mounting until hearts), and put the armour on: the horse wears lime armour. **Screenshot BR-6.**
- **BR-7. Wolf armour.** Wolf armour and blue dye give a blue wolf armour icon.
- **BR-8. Friend.** The friend copies the "Kit" book (from `/kit @a`) at the friend's table. The friend gets the copy and keeps the original. **Screenshot BR-8**, the friend's window.

```
IW4L_MINECRAFT_KIT=minecraft:written_book[written_book_content={title:"Kit",author:"Owner",pages:["Page one.","Page two."]}],minecraft:writable_book*6,minecraft:crafting_table,minecraft:leather_helmet,minecraft:leather_chestplate,minecraft:leather_horse_armor,minecraft:wolf_armor,minecraft:red_dye*8,minecraft:yellow_dye*8,minecraft:blue_dye*8,minecraft:lime_dye*8,minecraft:white_dye*8,minecraft:horse_spawn_egg*2,minecraft:saddle,minecraft:cooked_beef*16
```
20 stacks: a book and quill stacks to 1, so `writable_book*6` takes 6 slots (BR-1 uses 1, BR-2 3, BR-3 1 and tries a second). The old line had 33 stacks, and everything after the 27th (the dyes, the horse egg, the saddle, the food) never reached the backpack.

---

### Group `food-effects` (M), 2 commits

**Gaps:** items-6 (eating a suspicious stew applies nothing; `release-1-tests.md:404`) and items-17 (chorus fruit doesn't teleport; `release-4-tests.md:787`; W's log track-w-log.md:283 "Not in the fork").

**Cause**
- `crates/minecraft_survival/src/survival.rs:1047-1060`: when eating finishes, it calls `apply_consumed_food_effects` (`third_party/minecraftoss/player/src/food.rs:70-99`, by id only) and `drink_potion` (`survival.rs:928-934`, which reads `potion_contents` only).
  - Nothing reads `minecraft:suspicious_stew_effects`, though crafted, mooshroom and loot stews carry it as a list of `{id, duration?}` (47a945a3; `mooshroom.rs:148`; core `loot.rs:769-790`, where 160 is left out).
  - `PotionEffect::player_effect` (`minecraft_terrain/src/potions.rs:230-255`) maps 19 player effects; night vision, jump boost, blindness and saturation have none, and neither has wither, though players have `EffectKind::Wither` (and `/effect` maps it, `minecraft_survival/src/commands.rs:167`). So a wither rose stew would give nothing.
- **Chorus fruit:** `food.rs:96` returns true for `minecraft:chorus_fruit`, and `survival.rs:1051` drops the value. `Bite::Done` handling (`actions.rs:458-499`) only posts the eat events.
  - `try_random_teleport_target` (`player/src/lib.rs:1457-1496`) is never called, and it clamps y to -64..319 in every dimension.
  - Players are already moved by pearl landings: `entities.rs:722` `land_pearl` → `PlayerOutputs.pearl_landings` (`:180-182`) → `render_anim/src/minecraft_players.rs:926-947` (dismount, teleport, damage, sound). That covers host and friends.
  - Item cooldowns exist (`cooldowns.rs:14` `start`).

**Vanilla 26.3**
- `SuspiciousStewEffects.onConsume` calls `addEffect(new MobEffectInstance(effect, duration))` for each entry, with amplifier 0 and a default duration of 160.
- `TeleportRandomlyConsumeEffect.apply` (default diameter 16 and directional particles):
  - Up to 16 tries at x + (nextDouble − 0.5) × 16, y clamped from y + (nextDouble − 0.5) × 16 to [minY, minY + logicalHeight − 1], and z the same way as x.
  - `stopRiding` when a passenger.
  - `randomTeleport(x, y, z, true, BlockTags.CONSUMABLE_DOES_NOT_TELEPORT_TO)`. The tag `consumable_does_not_teleport_to` has no file in the 26.3 data, so it is empty.
  - On success: `GameEvent.TELEPORT` at the old position; `item.chorus_fruit.teleport` (PLAYERS) at the new one; level event 2017 at the old block with the packed difference (the trail); `resetFallDistance`; `resetCurrentImpulseContext`.
- `ItemStack.applyAfterUseComponentSideEffects` applies the chorus fruit's `use_cooldown` of 1.0 s (20 ticks) when eating finishes.

**Plan**
1. **"Minecraft suspicious stews give their effects when eaten"**
   - `potions.rs`: `pub fn stew_effects(components) -> Vec<PotionEffect>` reads the list (duration 160 by default, amplifier 0).
   - In `survival.rs`, one line after `drink_potion` (`:1052`) applies them through `apply_to_player` (`potions.rs:423`). Effects without a player kind are skipped, as for potions.
   - `player_effect` gets a `"wither" => EffectKind::Wither` row next to `"weakness"`. It is a row in `potions.rs`, not a change to M2's `EffectKind`. Potions and tipped arrows with a custom wither effect now apply it too, as in vanilla; log that.
2. **"Minecraft chorus fruit teleports the eater like vanilla"**
   - The finished bite reports the chorus request: `Bite::Done` gets a `teleport` flag from `apply_consumed_food_effects`'s result.
   - `actions.rs` (the `Bite::Done` arm) calls `entities.chorus_teleport(player, feet, dimension)`. It makes up to 16 draws from the host's entity random and checks each through `try_random_teleport_target`, whose y range then takes the dimension's minimum and logical height: Overworld -64..319, Nether 0..127, End 0..255.
   - On success it pushes `PlayerOutputs.chorus_landings` (beside `pearl_landings`). `minecraft_players.rs` handles it next to `:926`: dismount, teleport, clear the fall (`steps.air_peak = None`), no damage, the sound `minecraft:item.chorus_fruit.teleport` at the landing, and `Entities::game_event(player, TELEPORT, old)` (W's helper at `entities.rs:1077`, one line).
   - `cooldowns.start("minecraft:chorus_fruit", 20)` when eating finishes. Eating must not start while the cooldown is active: check the item-use path's `active` test.
   - If `level_events.rs` has no event 2017 drawing, the trail is logged as left.

**Tests**
- `potions::tests`: an allium stew gives fire resistance 60; a stew entry without `duration` gives 160; a wither rose stew gives `EffectKind::Wither`; a night vision stew gives nothing.
- `survival::tests`: eating the allium stew adds fire resistance.
- `minecraft_survival` chorus tests:
  - a target lies within ±8 on x and z;
  - Nether targets never go above 127;
  - a target in water or lava or inside a wall is refused;
  - a friend's landing goes to the friend's queue (like the test at `entities.rs:3128`);
  - after eating, the cooldown runs 20 ticks and blocks the next bite.

**Owner's steps** (survival, by day, flat ground, the kit below)
- **FE-1. Stews.** Craft stews from a bowl, a red mushroom, a brown mushroom and one flower each. A suspicious stew cannot be eaten on a full hunger bar, so before each stew run `/effect give @s minecraft:hunger 5 99` and wait until the bar is no longer full (as in `release-4-tests.md:113`).
  - Eat the allium stew: Fire Resistance shows at the top right for about 3 seconds. **Screenshot FE-1.**
  - The oxeye daisy stew: Regeneration.
  - The wither rose stew: Wither, with black hearts.
  - The lily of the valley stew: Poison, with green hearts.
  - The red tulip stew: Weakness.
  - The closed eyeblossom stew: Nausea, with a wobbling view.
  - The poppy stew (night vision) still does nothing; see Left out.
- **FE-2. Chorus fruit.** Eat a chorus fruit (it can be eaten even when full): you jump to a spot up to 8 blocks away with the chorus sound, and the fruit's hotbar slot shows the white cooldown sweep for a second. Eat again at once: nothing happens until the sweep ends. **Screenshot FE-2.**
- **FE-3. Riding.** Saddle a pig, ride it, and eat a chorus fruit: you leave the pig and land nearby.
- **FE-4. Walls.** Stand in a 3×3 room walled with stone, with a roof: you never land inside the stone. You land in the room or outside it on solid ground.
- **FE-5. Friend.** The friend eats a chorus fruit: both windows show the friend jumping. The host does the same: both windows show the host jumping. **Screenshot FE-5**, both windows.

```
IW4L_MINECRAFT_KIT=minecraft:bowl*16,minecraft:red_mushroom*16,minecraft:brown_mushroom*16,minecraft:allium*4,minecraft:oxeye_daisy*4,minecraft:wither_rose*4,minecraft:lily_of_the_valley*4,minecraft:red_tulip*4,minecraft:closed_eyeblossom*4,minecraft:poppy*4,minecraft:crafting_table,minecraft:chorus_fruit*64,minecraft:milk_bucket,minecraft:saddle,minecraft:pig_spawn_egg*2,minecraft:carrot_on_a_stick,minecraft:stone*64,minecraft:cooked_beef*16
```

---

### Group `tool-enchantments` (M), 2 commits

**Gaps:** items-18 (Unbreaking on shears used on mobs; E2's lead-knot decision track-e2-log.md:543 left it) and items-29 (Looting does nothing).

**Cause**
- **items-18.** `crates/minecraft_terrain/src/mob_actions.rs` calls `actor.inventory.wear_tool` directly, and `inventory.rs:381` `wear_tool` ignores enchantments. The sites are:
  - `:98`, a weapon hitting a mob;
  - `:172`, cutting a mob's lead;
  - `:223`, sheep;
  - `:311`, a snow golem;
  - `:323`, a mooshroom.
  - The same gap was found at `crates/minecraft_survival/src/pvp.rs:231-238` `wear_weapon` (a player hitting a player, called at `actions.rs:418`).
  - `durability::wear` (`minecraft_terrain/src/durability.rs:8-23`) applies Unbreaking, but only knot shears (`actions.rs:810`) and tridents (`trident.rs:85`) use it.
  - `Actor` (`mob_actions.rs:50-58`) has no random, and it is built in F's and M1's family tests (`elden_bosses.rs:211`, `polar_bears.rs:275`, `rabbits.rs:403`), so it should not change.
- **items-29.**
  - `third_party/minecraftoss/entities/src/loot.rs:523` handles `enchanted_count_increase` only when `context.looting == 0`. Any other level falls to `_ => return None` (`:535`), which voids the roll.
  - `random_chance_with_enchanted_bonus` (`:628-633`) returns None when `looting > 0`.
  - Nothing sets `EntityLootContext.looting` (`:18`): `death_of` (`world.rs:663-666`) builds the context without it.
  - Deaths are taken right after a player's hit in `mob_actions::attack` (`:102`) and `sweep` (`:116-121`), and once per tick in `server.rs:1917`.

**Vanilla 26.3**
- `Sheep.mobInteract`, `SnowGolem`, `MushroomCow`, `Entity`'s lead shearing and `ItemStack.hurtEnemy` all call `ItemStack.hurtAndBreak(1, ...)` → `processDurabilityChange` → `EnchantmentHelper.processDurabilityChange`. Unbreaking spares each point with chance level / (level + 1).
- `EnchantedCountIncreaseFunction.run`:
  - level = `EnchantmentHelper.getEnchantmentLevel(enchantment, ATTACKING_ENTITY)`, the killer's main hand at the kill;
  - level 0 returns the stack unchanged, without drawing;
  - otherwise `grow(Math.round(level × count.getFloat(ctx)))`, then `limitSize(limit)` when there is a limit.
- In the 26.3 tables: 50 entity tables use it; 76 counts are `uniform(0, 1)`, 3 of them with `limit: 1`, and one is `uniform(1, 2)`.
- `LootItemRandomChanceWithEnchantedBonusCondition`: at level > 0 the chance is `enchanted_chance.calculate(level)`, else `unenchanted_chance`. All 11 tables use `linear` (`base + per_level_above_first × (level − 1)`), for example the wither skeleton's skull at 0.035 + 0.01 per level.

**Plan**
1. **"Unbreaking spares Minecraft shears and weapons used on mobs and players"**
   - The five `mob_actions.rs` sites wear through `crate::durability::wear(actor.inventory, selected, n, || world.tool_roll())`.
   - `EntityWorld::tool_roll()` reads a new `LegacyRandom` field next to `projectile_seed_random` (`world.rs:2578`, made at `:2725`), so no existing random stream shifts.
   - `pvp::wear_weapon` takes a roll from its caller (`actions.rs:418`, the entities random).
   - Creative still wears nothing.
2. **"Looting adds to Minecraft mob drops like vanilla"**
   - `death_remains(world, loot, looting: u32)` (`mob_actions.rs:65`) sets `context.looting` on the deaths it takes whose attacker is `minecraft:player` with a direct hit.
     - `attack` passes `melee::level(held, "minecraft:looting")` (`melee.rs:12`).
     - `sweep` gets the sweeping player's level from its caller (`server.rs:602`).
     - `server.rs:1917` passes 0.
   - `entities/src/loot.rs:523`: the arm handles every level with vanilla's rounding and limit.
   - `:628-633`: the `linear` `enchanted_chance` at level > 0.

**Tests**
- `durability`/`mob_actions::tests`:
  - Unbreaking III shears over 400 seeded wear calls keep close to three quarters of the points;
  - plain shears lose 1 each;
  - creative keeps all;
  - the snow golem, mooshroom and lead-cutting paths use the roll;
  - a sword with Unbreaking hitting a mob wears less.
- `pvp::tests`: `wear_weapon` with Unbreaking.
- `entities loot::tests`:
  - with a seeded random, a zombie's flesh at Looting III can reach 5 and at Looting 0 never goes past 2;
  - a `limit: 1` table never goes past 1;
  - Looting 0 draws nothing extra (the sequence after the roll is unchanged);
  - the wither skeleton skull chance is 0.055 at Looting III.
- `mob_actions::tests`: a Looting sword's kill puts the level into the death's context; a burning death and a gun kill get 0.

**Owner's steps** (survival, by day, flat grass, the kit below)
- **TE-1. Shears.**
  - Hatch 16 sheep. Shear 8 with the plain shears, which have 8 uses left: they break on the eighth sheep.
  - Wait for the wool to grow back (feed them wheat to speed it up), or hatch new sheep. Shear 8 with the Unbreaking III shears: they are still whole, and their bar barely moved. **Screenshot TE-1.**
- **TE-2. Golem, mooshroom and lead.** With the Unbreaking shears, shear a snow golem's pumpkin off, shear a mooshroom, and cut a cow's lead tied to a fence. The bar moves at most a pixel.
- **TE-3. Swords.**
  - Hatch zombies and hit them with the plain worn iron sword: it breaks after 5 hits.
  - The worn Unbreaking III iron sword lasts far longer, usually 15 to 20 hits. **Screenshot TE-3.**
- **TE-4. Looting.**
  - Kill 8 cows with the plain iron sword: each drops at most 3 beef and 2 leather.
  - Kill 8 cows with the Looting III sword: several drop 4 to 6 beef, or 3 to 5 leather.
  - Kill zombies with each sword: the plain one gives at most 2 rotten flesh, and Looting III up to 5.
  - **Screenshot TE-4**, the Looting cow drops on the ground.
- **TE-5. Friend.** The friend kills cows with the Looting sword: the friend gets the bigger drops too.

```
IW4L_MINECRAFT_KIT=minecraft:shears+damage=230,minecraft:shears+unbreaking=3+damage=230,minecraft:sheep_spawn_egg*16,minecraft:wheat*32,minecraft:snow_golem_spawn_egg*2,minecraft:mooshroom_spawn_egg*2,minecraft:lead*4,minecraft:oak_fence*8,minecraft:iron_sword+damage=245,minecraft:iron_sword+unbreaking=3+damage=245,minecraft:iron_sword,minecraft:iron_sword+looting=3,minecraft:cow_spawn_egg*16,minecraft:zombie_spawn_egg*16,minecraft:cooked_beef*16
```

---

### Group `name-tags` (L), 3 commits

**Gap:** items-1. No round 10 track builds the item: M1-1 (track-m1.md:369, :3126) and M3 (track-m3.md:1791) only name it.

**Cause**
- No name tag code exists: `rg name_tag` over the crates and `third_party/minecraftoss` finds nothing.
- `mob_actions.rs:156-401` `interact` runs `leash_interact` first (`:163`), then the per-species match. Name tags have no arm.
- `CustomName` survives only as kept NBT:
  - mob tags start from the loaded tag (`server_mobs.rs:1526-1560`, `ServerSim.mob_tags`, `server.rs:51`);
  - conversions keep it (`server_mobs.rs:2237`, `:2301`);
  - it is read per family at load: Toast in `families/land/rabbits.rs:25-57`, Johnny in `illager_save.rs:33-111`.
- The saved `PersistenceRequired` comes from each store's flag (`server_mobs.rs:1546`). `check_despawn` (`world.rs:5956`, `riders` at `:5963`) and `census` (`:5895-5905`) read the per-store flags.
- The fork draws no mob names (track-m1-log.md:193). `death.rs:686` names the killer's type (`entity_text`).
- Friends already receive the full mob tag: `MobEvent::Add`/`Look` → `ToClient::MobAdd`/`MobLook` (`mob_sender.rs:65-84`). The friend re-spawns the mob from it (`remote.rs:38-46`, `:448-454`). `CustomName` is not in `VOLATILE` (`mob_tracking.rs:15-52`), so a new name sends a `Look`.
- The host's view is a clone of the server's `EntityWorld` (`entities.rs:1836-1840`).
- `ServerMob` (`client_mobs.rs:31`) is `Copy` and on the wire, so it must not carry names.
- The anvil already renames items (`stations.rs:1087`).

**Vanilla 26.3**
- `Mob.interact` calls `checkAndHandleImportantInteractions` first. A `NAME_TAG` there goes before the leash and shears handling in `LivingEntity`/`Entity.interact` and before `mobInteract`.
- `NameTagItem.interactLivingEntity`: if the stack has `minecraft:custom_name` and `target.getType().canSerialize()`, then on the server, while the target is alive, it calls `setCustomName(name)` and `Mob.setPersistenceRequired()`, and shrinks the stack by 1 (creative keeps it). It returns `SUCCESS`, else `PASS`.
- A mob named by a command still despawns: `Mob.requiresCustomPersistence` is only passenger or leashed.
- Drawing:
  - `MobRenderer.shouldShowName`: `LivingEntityRenderer.shouldShowName` (not the camera entity, not a vehicle, HUD not hidden), and `CustomNameVisible`, or a custom name on the mob under the crosshair (`crosshairPickEntity`).
  - `EntityRenderer.extractNameTags`: only within 64 blocks (32 when discrete).
  - The tag sits at the `NAME_TAG` attachment plus 0.5. The text is 0.025 blocks a pixel on a 25% black box, faint white through walls and full white where visible.
- Death messages use `getDisplayName`, which is the custom name.

**Plan**
1. **"Minecraft name tags name mobs and keep them from despawning"**
   - `mob_actions::interact`: a name tag arm before `leash_interact` (`:163`). With `minecraft:custom_name` on the held stack and the mob alive, it sets `outcome.named = Some((id, name_json))`, uses one tag unless `infinite`, and is handled. Without a name it passes.
   - `ServerSim::mob_action` (`server.rs:565`) writes `CustomName` (`item_nbt::json_tag`) into `self.mob_tags` (creating the entry), calls `self.mobs.set_custom_name(id, text)`, and calls `self.mobs.require_persistence(id)`.
   - Vendored `EntityWorld`:
     - `custom_names: HashMap<u64, String>` with `set_custom_name`/`custom_name`;
     - `kept: HashSet<u64>` with `require_persistence`/`persistence_required`;
     - `check_despawn`'s `riders` line (`:5963`) and `census`'s `persistent` (`:5904`) take `kept` into account;
     - `mob_tags_where` saves `PersistenceRequired` as `persistent || world.persistence_required(id)` (`server_mobs.rs:1546`).
   - `spawn_saved` (`server_mobs.rs:600`) sets the custom name from a loaded `CustomName`, read as a string or a component's `text`, like `rabbits.rs:25-57`. The host's loads and the friend's `spawn_tracked` both get it.
2. **"Named Minecraft mobs show their name when looked at"**
   - render_anim lists, each frame, the named mobs of the view's `EntityWorld`: the host's clone, or the friend's `level.view.world`.
   - It shows a mob that is either `CustomNameVisible` in its tag or the mob under the crosshair (the same pick as attacking and using items on mobs), within 64 blocks.
   - Each label sits at the drawn mob's feet plus its height plus 0.5.
   - `MinecraftUi.mob_names` (`frame/src/minecraft_ui.rs`, next to `wither_bars`) carries them to a new `hud/src/minecraft_mob_names.rs`. It projects the label like `minecraft_held_map::to_screen` (`:64-76`) and draws the Minecraft font at 0.025 blocks a pixel on a `0x3F000000` box: white where a block ray from the eye reaches the label, `0x20FFFFFF` where blocks hide it.
3. **"Minecraft death messages use a named mob's name"**
   - `death_text` (`death.rs:677-697`) gets a `named: impl Fn(u64) -> Option<String>` lookup next to `present`. Where it pushes `entity_text(type_id)` (`:686`), it pushes the name of the killing mob (`DamageCause::Mob { id, .. }`) when it has one.
   - The callers pass the view world's `custom_name`. The kill feed follows the same line.

**Tests**
- `mob_actions::tests`:
  - a named tag on a cow names it and leaves 7 of 8 tags; creative leaves 8;
  - a plain tag passes, and the lead code still runs for a lead;
  - a tag on a dying mob does nothing;
  - the tag is handled before `leash_interact`.
- `server::tests`:
  - the named cow saves `CustomName` and `PersistenceRequired:1b` and loads back named;
  - a named zombie 150 blocks from every player is not despawned, while an unnamed one is;
  - a `/summon` with `CustomName` keeps despawning, as in vanilla.
- `mob_tracking`/`entity_view`:
  - naming sends a `MobEvent::Look`;
  - a friend's spawned mob has the name.
- `hud`: a label's projected size halves at twice the distance; a hidden label is faint.
- `death::tests`: "Host was slain by Zed" for a named zombie and "Host was slain by Zombie" without a name.

**Owner's steps** (survival, a flat world, the kit below)
- **NT-1. Name tags.** Throw bottles o' enchanting until you have a few levels. The anvil renames the whole stack in its slot, so put a single tag in it: hold the stack and right-click the anvil's left slot once. Rename that one tag "Bob". Do the same with two more single tags: "Zed" and "Star". 13 plain tags stay in the stack.
- **NT-2. Name a cow.**
  - Hatch a cow and right-click it with the "Bob" tag: the tag is used.
  - Look at the cow from 2 blocks: "Bob" shows above it in white on a dark box. Look away: the name goes.
  - Stand behind a block wall and look at it through the wall: the name is faint.
  - **Screenshot NT-2.**
- **NT-3. A plain tag.** Right-click a cow with an unrenamed tag: nothing happens and the tag stays.
- **NT-4. Kept.**
  - Name a zombie with the "Zed" tag (use it at night, or hatch the zombie inside a dark hut) and hatch an unnamed zombie beside it.
  - `/tp @s ~150 ~ ~`, wait 30 seconds, and `/tp @s ~-150 ~ ~`: Zed is still there and the unnamed zombie is gone.
  - Save and quit, then load the world: Zed and Bob still carry their names.
  - **Screenshot NT-4.**
- **NT-5. Death message.** Take off your armour and let Zed kill you: the chat reads "<your name> was slain by Zed". **Screenshot NT-5.**
- **NT-6. Before the lead.** Hold the "Star" tag and right-click a horse: the horse is named and you don't mount it.
- **NT-7. Friend.**
  - The friend looks at Bob from 2 blocks: "Bob" shows in the friend's window.
  - The friend renames one single tag "Pinky" in an anvil and names a pig with it: the host sees "Pinky" when looking at it.
  - **Screenshot NT-7**, both windows.

```
IW4L_MINECRAFT_KIT=minecraft:name_tag*16,minecraft:anvil,minecraft:experience_bottle*64,minecraft:cow_spawn_egg*4,minecraft:pig_spawn_egg*4,minecraft:zombie_spawn_egg*8,minecraft:horse_spawn_egg*2,minecraft:lead*4,minecraft:iron_sword,minecraft:shield,minecraft:oak_planks*64,minecraft:torch*16,minecraft:cooked_beef*32
```

---

### Group `heads-hives` (S), 1 commit

**Gaps:** items-24 (heads lose their name and profile). The hives half (items-27) is its own group, `hive-bees` (segment e3z, after M1's m1i), because M1's bees build the hive storage and break handling it changes. The group keeps its key so the run specs and E3-0 stay stable.

**Cause**
- Placement applies item components to the block entity only for banners: the `is_banner` filter at `crates/minecraft_survival/src/actions.rs:1633-1642` (at 44d96765). placed-names (E3-8, e3j) turns it into `components::takes_components(id)` for banners and the named containers, so re-read that helper before editing.
- `third_party/minecraftoss/world/src/level/components.rs:24-29` `implicit()` maps only banners (plus placed-names' kinds), and `:73-83` `dropped_components` returns None for every other kind. So the harvest (`actions.rs:1797-1814`) and the explosion path (`Level::keep_block_entity_components`, `:104-117`) give a head nothing.
- core `block_entity.rs:258-264` already saves a skull's `profile`, `note_block_sound` and `custom_name`.

**Vanilla 26.3**
- `SkullBlockEntity.applyImplicitComponents`/`collectImplicitComponents` carry `profile`, `note_block_sound` and `custom_name`. `blocks/player_head.json` copies all three from `block_entity`; the other heads' tables (`creeper_head.json`, `zombie_head.json` and so on) copy only `minecraft:custom_name`.

**Plan**
1. **"Minecraft heads keep their name and player heads their profile when placed and broken"**
   - `components.rs`: `implicit("minecraft:skull")` maps `minecraft:profile`/`profile`, `minecraft:note_block_sound`/`note_block_sound` and `minecraft:custom_name`/`custom_name` (snake case, as core saves it).
   - `dropped_components` gets a `SKULL_LOOT` list with those three. A non-player head never carries a profile, so its drop matches its table.
   - Placement: add every head (wall forms through `standing`, `:85`) to placed-names' `components::takes_components`, with the placed block matching it. No second `keeps_components` helper (E3-0, "Placed components").
   - Explosions follow through `keep_block_entity_components`.

**Tests**
- `components::tests`: a skull entity's `applied`, `collected` and `dropped_components`.
- `minecraft_survival actions` tests:
  - a renamed creeper head placed and mined drops with its name;
  - a player head keeps `profile` and `custom_name`;
  - a TNT blast keeps a head's name;
  - a banner's and a named chest's drops are unchanged.

**Owner's steps** (survival, the kit below)
- **HH-1. Named heads.** Throw bottles until you have levels. Rename the creeper head "Boom" and the player head "Alex" in the anvil. Place both and mine them with the pickaxe: they drop as "Boom" and "Alex" (italic). **Screenshot HH-1.**
- **HH-2. Blown up.** Place "Boom" again, put TNT 2 blocks away and light it from a distance: "Boom" drops and keeps its name.
- **HH-6. Friend.** The friend renames and places a head, and the host mines it: the name is kept. **Screenshot HH-6.**

```
IW4L_MINECRAFT_KIT=minecraft:creeper_head*2,minecraft:zombie_head*2,minecraft:player_head*2,minecraft:skeleton_skull*2,minecraft:anvil,minecraft:experience_bottle*32,minecraft:tnt*4,minecraft:flint_and_steel,minecraft:diamond_pickaxe,minecraft:cobblestone*64,minecraft:cooked_beef*16
```

---

### Group `decorated-pots` (M), 3 commits

**Gaps:** items-25 (pots lose their sherds) and items-26 (pots never shatter). Commit 1 adds the pot recipe, which the fork also lacks (`crafting_decorated_pot` is not parsed), so a sherd pot can be made in survival.

**Cause**
- Placement (`actions.rs:1633-1642`, which placed-names turns into `components::takes_components`) and `dropped_components` (`components.rs:73-83`) know only banners, placed-names' containers and heads-hives' heads, so `pot_decorations` is neither stored nor dropped. core `block_entity.rs:229-232` saves `sherds` when present.
- The fork never sets `cracked=true`: placement writes `false` (`player/src/placement.rs:86-91`). So a pot broken with a pickaxe drops a whole pot.
- A pot that is already cracked reaches the `minecraft:dynamic` entry, which `player/src/loot.rs:297-318` `entry()` doesn't know (`_ => None`), so it drops nothing.
- `parse_recipe` (`crafting.rs:1239`) drops `recipe/decorated_pot.json` (`crafting_decorated_pot`). Only `decorated_pot_simple.json` (shaped, four bricks) works, and it gives a pot with no component.
- Pots draw plain sides (`special_models.rs:1082`).

**Vanilla 26.3**
- `DecoratedPotRecipe`:
  - `matches` needs a 3×3 input with exactly 4 items: back at (1,0), left at (0,1), right at (2,1) and front at (1,2), each in `#decorated_pot_ingredients`.
  - `assemble` sets `POT_DECORATIONS` to `(back, left, right, front)`, each `Optional.of` its item.
- The decorated pot item's default `POT_DECORATIONS` is a delayed component (four bricks, `PotDecorations.allBrick`). A block entity without `sherds` is `PotDecorations.EMPTY` (all sides empty). The codec fields are `back`, `left`, `right` and `front`.
- `DecoratedPotBlock.playerWillDestroy`: when the main hand is in `#breaks_decorated_pots` (swords, axes, pickaxes, shovels, hoes, the trident, the mace) and has no `#prevents_decorated_pot_shattering` enchantment (Silk Touch), it sets `CRACKED=true` first.
- `getDrops` adds the dynamic drop `minecraft:sherds`. Its lambda drops left, back, front and right, each only where present, so an EMPTY pot shatters into nothing.
- `getSoundType` of a cracked pot is `DECORATED_POT_CRACKED`, whose break sound is the shatter.
- `blocks/decorated_pot.json`: the alternatives are the dynamic `minecraft:sherds` when `cracked=true`, else the pot with `copy_components` `minecraft:pot_decorations`.

**Plan**
1. **"Minecraft decorated pots are crafted from sherds like vanilla"**
   - `Kind::DecoratedPot { back, left, right, front }` is parsed from `minecraft:crafting_decorated_pot`, with an arm next to the other special kinds.
   - Its match needs the grid's positions: `special_result` takes the grid width from `matching` (`:839`), and the bounding box must be 3×3 with 4 items.
   - The result is `minecraft:pot_decorations {back, left, right, front}`. It is left out when all four are bricks, so the pot stacks with the simple recipe's pot, as vanilla's default does.
2. **"Minecraft decorated pots keep their sherds when placed and broken"**
   - `implicit("minecraft:decorated_pot")` maps `minecraft:pot_decorations`/`sherds`. `dropped_components` gets `POT_LOOT`, whose all-brick decorations drop no component.
   - Add the pot to placed-names' `components::takes_components` (E3-0, "Placed components"). A pot item without the component (and without `!minecraft:pot_decorations`) stores four bricks, the item default `PotDecorations.allBrick`. projectile-breaks (E3-9), which runs right after this group in e3q, relies on that.
3. **"Minecraft decorated pots shatter into their sherds under a tool"**
   - In the harvest (`actions.rs`, where `broken` is built before the drops, about `:1765-1795`): a decorated pot mined by hand with a `#breaks_decorated_pots` tool without Silk Touch becomes `cracked=true` before its sound and loot. The cracked state's sound type comes from the state catalog (`scene.rs:356`).
   - `player/src/loot.rs` `entry()` gets a `minecraft:dynamic` arm that returns a one-item placeholder named by the dynamic id. `drop_blocks` (`entities.rs:804-835`) replaces a `minecraft:sherds` placeholder with the pot's `sherds` sides from the kept components, in vanilla's order (left, back, front, right), each only where present.

**Tests**
- `crafting::tests`:
  - four sherds in the diamond give their decorations in back, left, right, front order;
  - four bricks give a pot with no component;
  - a 2×2 shape or a fifth item fails.
- `components`/`actions` tests:
  - a sherd pot placed and mined by hand keeps its decorations;
  - a plain pot placed stores four bricks and drops a plain pot.
- `actions`/`entities` tests:
  - a pickaxe shatters the sherd pot into its four sherds in order;
  - a plain pot shatters into 4 bricks;
  - Silk Touch, a stick or a hand drop the pot;
  - a hoe shatters it;
  - a generated pot without `sherds` shatters into nothing;
  - TNT drops the pot whole with its sherds;
  - a friend's mining shatters the same way.

**Owner's steps** (survival, a crafting table, the kit below)
- **DP-1. Craft.** Put the angler sherd at the top middle, the archer at the left middle, the arms up at the right middle and the blade at the bottom middle: a Decorated Pot comes out. The sides are not drawn yet; see Left out. **Screenshot DP-1.**
- **DP-2. Kept.** Place it and mine it with your hand: the pot drops. Place it again.
- **DP-3. Shatter.** Mine it with the plain diamond pickaxe: it breaks with a shatter sound and drops the four sherds. **Screenshot DP-3.**
- **DP-4. Bricks.** Craft a pot from 4 bricks, place it and mine it with the pickaxe: 4 bricks drop.
- **DP-5. Silk Touch and a stick.** Pots mined with the Silk Touch pickaxe or with a stick in hand drop whole. With the diamond hoe or sword they shatter.
- **DP-6. TNT.** Light TNT 2 blocks from a sherd pot: the pot drops whole. Place it and shatter it: the four sherds come back.
- **DP-7. Friend.** The friend shatters a sherd pot: both windows show the four sherds dropping. **Screenshot DP-7.**

```
IW4L_MINECRAFT_KIT=minecraft:angler_pottery_sherd*4,minecraft:archer_pottery_sherd*4,minecraft:arms_up_pottery_sherd*4,minecraft:blade_pottery_sherd*4,minecraft:brick*16,minecraft:crafting_table,minecraft:diamond_pickaxe,minecraft:diamond_pickaxe+silk_touch=1,minecraft:diamond_hoe,minecraft:diamond_sword,minecraft:stick*4,minecraft:tnt*4,minecraft:flint_and_steel,minecraft:cobblestone*64,minecraft:cooked_beef*16
```

---

### Group `trial-effects` (L), 3 commits

**Gaps:** items-19 (wind charged), items-20 (weaving; also `docs/MINECRAFT-SURVIVAL.md:420`) and items-21 (oozing), on mobs. The player side and infested are in `trial-effects-players`.

**Cause**
- `MobEffect` (`third_party/minecraftoss/entities/src/effects.rs:17-54`, `ALL` at `:37`) has no `wind_charged`, `weaving`, `oozing` or `infested`. So `PotionEffect::mob_effect` (`potions.rs:214`, through `from_id` at `effects.rs:78`) returns None, and splash potions, lingering clouds and the ominous vault clouds drop them.
- The potions (`potions.rs:64-67`) and the HUD rows (`hud/src/minecraft_effects.rs:68-71`) already exist.
- Dead mobs leave their stores in the `retain` block at `world.rs:8196-8239` once `death_ticks` reaches 20. Nothing runs on that removal.
- A wind burst exists (`server.rs:2387` `wind_burst` with `wind::Charge` `Player | Breeze`, `wind.rs:18-50`).
- The cobweb slowdown is a fixed `BlockEffect::Stuck(0.25, 0.05, 0.25)` (`inside_blocks.rs:493`).
- The slime store has sizes (`world.rs` slimes).

**Vanilla 26.3**
- `LivingEntity.remove` (KILLED or DISCARDED) → `triggerOnDeathMobEffects` → each effect's `onMobRemoved`, then the effects are cleared. A mob is removed at the end of its 20-tick death (`tickDeath`), with the poof.
- `WindChargedMobEffect.onMobRemoved` (KILLED): `ServerLevel.explode(entity, null, AbstractWindCharge.EXPLOSION_DAMAGE_CALCULATOR, x, y + height / 2, z, 3.0 + nextFloat() × 2.0, false, TRIGGER, GUST_EMITTER_SMALL, GUST_EMITTER_LARGE, ..., BREEZE_WIND_CHARGE_BURST)`. That is power 3 to 5.
- `WeavingMobEffect.onMobRemoved` (KILLED, and a player or `mob_griefing`): `spawnCobwebsRandomlyAround`.
  - It takes `maxCobwebs = randomBetweenInclusive(2, 3)` (`MobEffects lambda$static$0`) from `BlockPos.randomInCube(random, 15, pos, 1)`.
  - A spot counts when its block can be replaced, the block below has a sturdy up face, and the block below wasn't already chosen.
  - It places cobwebs (`setBlockAndUpdate`) and level event 3018 at each.
- `WebBlock.entityInside`: `makeStuckInBlock(0.5, 0.25, 0.5)` for an entity with WEAVING, else `(0.25, 0.05, 0.25)`.
- `OozingMobEffect.onMobRemoved` (KILLED):
  - `spawnedCount` is 2 (`lambda$static$1`), capped by `numberOfSlimesToSpawn`: clamp of `max_entity_cramming` minus the slimes within `RADIUS_TO_CHECK_SLIMES` 2, between 0 and the count.
  - It spawns size-2 slimes (`SLIME_SIZE`) at (x, y + 0.5, z) with a random yaw (`spawnSlimeOffspring`, `EntitySpawnReason.TRIGGERED`).

**Plan**
1. **"Minecraft mobs carry the trial potion effects and burst when they die wind charged"**
   - `MobEffect` gets `WindCharged`, `Weaving`, `Oozing` and `Infested` next to the harmful ones (after `Wither`), in `ALL` and `id()`. `Infested` is stored here and acts in `trial-effects-players`. Saving goes through the existing `active_effects`.
   - Vendored `EntityWorld`: before the dead-mob `retain` block (`world.rs:8196`), collect each mob whose death ends this tick and that carries one of these effects (`mob_effects`, `world/witches.rs:575`) into `removed_with_effects: Vec<(DVec3, f32 height, Vec<MobEffect>)>`. `take_removed_with_effects()` hands them over after the mob tick.
   - The server drains them where the deaths are presented (`server.rs:1917`): wind charged goes through `wind_burst` at the centre with a third `Charge` kind whose radius is the rolled power and whose sound is the breeze burst.
2. **"Minecraft mobs that die weaving leave cobwebs, and weaving mobs push through cobwebs"**
   - The server places 2-3 cobwebs by vanilla's rule through the level, when `mob_griefing` is on, with a block change per web.
   - `inside_blocks.rs:493` uses `(0.5, 0.25, 0.5)` for a mob with Weaving, whose effects are read where the mob's inside-blocks pass runs.
3. **"Minecraft mobs that die oozing leave slimes"**
   - The server spawns `min(2, cramming − slimes within 2)` size-2 slimes through the slime store at y + 0.5, each with a random yaw.

**Tests**
- `effects::tests`: ids round-trip.
- `potions::tests`: a splash of oozing gives a mob the effect.
- `server_mobs` save test: a zombie with Weaving saves and loads it.
- `world`/`server` tests:
  - a weaving zombie killed on flat ground leaves 2 or 3 cobwebs within one block, none floating, after 20 ticks and not before;
  - with `mob_griefing` off it leaves none;
  - an oozing zombie leaves 2 size-2 slimes, or 1 when cramming 24 already has 23 slimes within 2 blocks;
  - a wind-charged zombie's death pushes a player standing 2 blocks away;
  - a discarded (despawned) mob triggers nothing.
- `inside_blocks::tests`: the weaving speed factor.

**Owner's steps** (survival, by day, flat grass, the kit below)
- **TP-1. Weaving.**
  - Hatch 3 zombies close together and throw the weaving splash at them: their swirls change colour.
  - Kill one: when its body poofs, 2 or 3 cobwebs appear where it died, on the ground. **Screenshot TP-1.**
  - `/gamerule mob_griefing false`, then repeat: no cobwebs. `/gamerule mob_griefing true`.
- **TP-2. Through webs.** Splash weaving on a zombie and lure it through a cobweb line next to an unsplashed zombie: the weaving zombie gets through clearly faster.
- **TP-3. Oozing.** Splash oozing on a zombie and kill it: when its body poofs, two medium slimes pop out. **Screenshot TP-3.**
- **TP-4. Wind charged.** Splash wind charged on a zombie, stand 2 blocks from it and kill it: when its body poofs, a gust bursts with the breeze sound and throws you back. **Screenshot TP-4.**
- **TP-5. A cloud.** Throw the lingering oozing potion on 4 cows and kill them inside the cloud: slimes come out of each body.
- **TP-6. Friend.** The friend kills a weaving zombie: cobwebs appear in both windows. **Screenshot TP-6.**

```
IW4L_MINECRAFT_KIT=minecraft:splash_potion@wind_charged*3,minecraft:splash_potion@weaving*3,minecraft:splash_potion@oozing*3,minecraft:lingering_potion@weaving*2,minecraft:lingering_potion@oozing*2,minecraft:zombie_spawn_egg*16,minecraft:cow_spawn_egg*8,minecraft:iron_sword+sharpness=5,minecraft:shield,minecraft:cobweb*8,minecraft:cobblestone*64,minecraft:milk_bucket,minecraft:cooked_beef*32
```

(Potions don't stack, so the kit fills 23 slots.)

---

### Group `trial-effects-players` (M), 2 commits

**Gaps:** items-22 (infested; `release-6-tests.md:437`) and the player half of items-19 to items-21.

**Waits for:** M2's segment m2i merged, because `nautiluses-riding` is M2's last change to `EffectKind` and its `ALL` length. It also needs D0's d0b merged (c0366f3a) for spawning silverfish.

**Cause**
- The player's `EffectKind` (`player/src/survival.rs:36-82`) lacks the four. So `player_effect` (`potions.rs:230-255`) returns None: drinking the potions, splashes and clouds do nothing to players.
- The player's cobweb hold (`player/src/lib.rs`) is fixed.
- Silverfish are registered (`entities/src/world/families/pests/silverfish.rs`), and d0b brings their release from infested blocks. Nothing spawns them when an entity with Infested is hurt.

**Vanilla 26.3**
- A dead player is removed with `RemovalReason.KILLED` when it respawns (`ServerGamePacketListenerImpl` → `PlayerList.respawn(player, false, KILLED)`), so wind charged, weaving and oozing trigger at the death spot then.
- Weaving skips the `mob_griefing` check for players.
- `InfestedMobEffect.onMobHurt`:
  - on `nextFloat() <= 0.1` (`chanceToSpawn`) it spawns `randomBetweenInclusive(1, 2)` silverfish (`lambda$static$2`) at (x, y + height / 2, z);
  - each gets a random yaw and the velocity `lookAngle × 0.3 × (1, 1.5, 1)` turned by a random angle in ±π/2 (`Mth.randomBetween(-1.5707964, 1.5707964)`);
  - each plays `entity.silverfish.hurt`;
  - this happens for mobs and players.

**Plan**
1. **"Minecraft players carry the trial potion effects like mobs"**
   - `EffectKind` gets `WindCharged`, `Weaving`, `Oozing` and `Infested` after `Darkness`, in `ALL` (its length +4 over M2's), `player_nbt.rs:18` `EFFECTS`, `/effect` (`commands.rs:173`, `:2687`) and `player_effect`.
   - The survival death record keeps the dead player's wind charged, weaving and oozing. When the player respawns, the host sends them to the server queue that `trial-effects` drains, at the death position (players skip the `mob_griefing` check for webs).
   - The player's cobweb hold uses `(0.5, 0.25, 0.5)` with Weaving.
2. **"Infested Minecraft mobs and players let silverfish out when hurt"**
   - On a full hurt of a mob with `MobEffect::Infested` (the hurt path in `entities/src/health.rs` / `world/combat.rs`) or of a player with `EffectKind::Infested` (`minecraft_survival/src/survival.rs`'s hurt): roll 0.1, then spawn 1-2 silverfish through d0b's silverfish spawn with vanilla's push and sound.

**Tests**
- `player_nbt`: the four effects save and load.
- `potions::tests`: drinking oozing gives the player the effect.
- `minecraft_survival` tests:
  - a player who dies with Oozing respawns and two slimes appear at the death spot;
  - Weaving leaves webs even with `mob_griefing` off.
- Seeded tests: an infested zombie hit 40 times lets out silverfish about 4 times, 1 or 2 each; the same for a player hit by a zombie.

**Owner's steps** (survival, by day, flat grass, the kit below)
- **TQ-1. Oozing on you.** Drink the oozing potion: its icon shows. `/tp @s ~ ~30 ~` and fall. Click Respawn: two medium slimes are where you died. **Screenshot TQ-1.**
- **TQ-2. Weaving on you.** Drink weaving and walk through the cobweb line: you are slowed less than before. Then die from a fall: cobwebs appear around the spot.
- **TQ-3. Infested on you.** Drink infested and let a zombie hit you about 20 times (the shield helps): now and then 1 or 2 silverfish burst out of you with a hiss. **Screenshot TQ-3.**
- **TQ-4. Infested mobs.** Splash infested on 3 zombies and hit them: silverfish come out of some hits. **Screenshot TQ-4.**
- **TQ-5. Friend.** The friend drinks wind charged and dies by a fall near the host. On the friend's respawn, a gust bursts at the spot in both windows.

```
IW4L_MINECRAFT_KIT=minecraft:potion@wind_charged*2,minecraft:potion@weaving*2,minecraft:potion@oozing*2,minecraft:potion@infested*2,minecraft:splash_potion@infested*3,minecraft:zombie_spawn_egg*16,minecraft:cobweb*8,minecraft:milk_bucket,minecraft:iron_sword,minecraft:shield,minecraft:cobblestone*64,minecraft:cooked_beef*32
```

---

### Group `hive-bees` (S), 1 commit, segment e3z after M1's m1i

**Gaps:** items-27 (silk-touched nests and hives drop nothing; M1's bee brief, track-m1.md:3020 and :3158, leaves "bees kept in a silk-touched nest" to shared loot and placement work). This was `heads-hives`' second commit. It moved here because M1's m1i `bees-hives` builds `world/src/level/beehive.rs` (the occupants in the hive tag, release and anger on break with a Silk Touch exception, the honey level, the comparator), and this commit changes the harvest, loot and placement of the same blocks.

**Waits for:** M1's segment m1i merged. Re-read `beehive.rs`, M1's break handling (`empty_hive`, its Silk Touch branch) and M1's tests before editing.

**Cause**
- `player/src/loot.rs:431` `_ => return None` catches `minecraft:copy_state`, which voids the silk-touch entry of `blocks/beehive.json` and the pool of `blocks/bee_nest.json`. `:337-341` lets `copy_components` through without copying.
- `drop_blocks` (`entities.rs:804-835`) replaces the drop's components with the kept ones.
- Nothing applies a `minecraft:block_state` component on placement.
- core `block_entity.rs:223-228` saves a hive's `bees`; M1's `beehive.rs` keeps the occupants in that vanilla tag.

**Vanilla 26.3**
- `blocks/bee_nest.json`: a pool with the `minecraft:tool/can_silk_touch` condition; its item has `copy_components` (`minecraft:bees`, from `block_entity`) and `copy_state` (`honey_level`).
- `blocks/beehive.json`: alternatives; the silk-touch child has both functions, and the other child is a plain beehive.
- `CopyBlockState.run` sets `BLOCK_STATE` to the listed properties of the broken state when the block matches.
- `BeehiveBlockEntity.collectImplicitComponents`/`applyImplicitComponents` carry `bees`. `BlockItem.updateBlockStateFromTag` sets each listed property the placed block has.
- `BeehiveBlock.playerDestroy`: with a tool in `#prevents_bee_spawns_when_mining` (Silk Touch) the occupants stay in the block entity and go into the item; otherwise they are released angry (M1's code).

**Plan: one commit, "Silk-touched Minecraft bee nests and beehives keep their bees and honey"**
- `player/src/loot.rs` `apply_modifiers` gets a `minecraft:copy_state` arm: when `block` matches, it writes the listed properties of `context.block` into `minecraft:block_state` (string values).
- `implicit("minecraft:beehive")` maps `minecraft:bees`/`bees` for placement, in the same occupant form M1's `beehive.rs` reads. Hives stay out of `dropped_components`, so explosions and breaks without Silk Touch keep no bees, as their loot copies none.
- The harvest (`actions.rs:1797-1814`) adds a hive's `minecraft:bees` (from `components::collected`, read before M1's break handling empties it) to `kept` only when the breaking tool has Silk Touch; M1's Silk Touch branch already releases nobody.
- `drop_blocks` merges the kept components into the drop (`minecraft_terrain::crafting::with_components`, `crafting.rs:519`) instead of replacing them. Banners and heads are unchanged by that.
- Placement: add hives and nests to placed-names' `components::takes_components`, and apply the held item's `minecraft:block_state` to the placed state: each listed property the block has.
- M1's brief says the bees are lost (`track-m1.md:3020`, `:3158`, owner step 11): write the new wording under "State doc changes" in the log for the orchestrator; never edit M1's brief.

**Tests**
- `components::tests`: a hive's `bees` are applied but not dropped.
- `player loot::tests`: `copy_state` on a `bee_nest[honey_level=5]` gives `{"honey_level":"5"}`.
- `entities`/`actions` tests:
  - a silk-touched nest with two bees drops one nest with `block_state` and both bees, and no bee is released;
  - a plain pickaxe on a nest drops nothing;
  - a hive without Silk Touch drops a plain hive (M1's release test still passes);
  - placing the nest restores `honey_level` 5 and the entity's `bees`, which M1's ticker then lets out;
  - a banner's and a head's drops are unchanged.

**Owner's steps** (survival, the kit below)
- **HH-3. A full nest.**
  - `/setblock ~ ~ ~2 minecraft:bee_nest[honey_level=5]`: the nest's front shows dripping honey.
  - Mine it with the Silk Touch pickaxe: it drops a Bee Nest. Before this change it dropped nothing.
  - Place it: the front still drips honey. **Screenshot HH-3.**
  - Mine it with the plain pickaxe: nothing drops, as in vanilla.
- **HH-4. Hives.**
  - `/setblock ~ ~ ~2 minecraft:beehive[honey_level=5]`, then mine it with Silk Touch: placed again, it is still full.
  - Do the same with the plain pickaxe: a beehive drops, and placed it is empty.
  - Use a glass bottle on the full one: honey bottle.
- **HH-5. Bees.** Hatch two bees near flowers beside a placed hive and wait until they fly into it (at night, or after pollinating). Silk-touch the hive: no bee comes out. Place it near flowers and wait: the two bees come out by day. Break a hive with bees inside using the plain pickaxe: the bees come out angry (M1).

```
IW4L_MINECRAFT_KIT=minecraft:diamond_pickaxe+silk_touch=1,minecraft:diamond_pickaxe,minecraft:bee_nest*2,minecraft:beehive*2,minecraft:bee_spawn_egg*4,minecraft:poppy*32,minecraft:dandelion*32,minecraft:glass_bottle*8,minecraft:cobblestone*64,minecraft:cooked_beef*16
```
10 stacks.

---

### Left out (log them in `docs/round10/track-e3-log.md` and the release notes)

These are listed in the `left` field.

## E3-13. Screens, settings, commands and statistics

This area closes the screen and command gaps from the release test files: the recipe book's keys, its bounce and its furnace version, the F3 screen, `/setworldspawn`, `/item replace` and `/kill` with entity selectors. No statistics gap is open in this area. The effect timers' tick rate (screens-4) is left out on purpose (see "Left out" at the end of this section).

Paths and lines are as of 9c6efebd (`minecraft-survival` after W's w2 merge). Check them again before editing, because e2b's merge, d0b, w3 and M1-M3 land in the meantime. Vanilla facts were checked with javap on the 26.3 `client.jar` (`C:\Games\Mashup\2010-Rust-Rewrite-Mashup\iw4l-artifacts\minecraft-26.3\client.jar`) and its `assets/minecraft/lang/en_us.json`.

### Shared points for this area

- **Build order:** `setworldspawn`, `book-keys`, `debug-screen`, `item-replace`, `recipe-bounce`, `kill-selectors`, `oven-book`. The three book groups edit the same files (`crates/hud/src/minecraft_recipe_book.rs`, `crates/frame/src/minecraft_recipe_book.rs`, `crates/render_anim/src/minecraft_recipe_book.rs`) and run in this order. The three command groups share `crates/minecraft_commands/src/{args,commands,server,suggest}.rs` and `crates/minecraft_survival/src/commands.rs`.
- **Other tracks.** None of these files or hooks belongs to M1-M3 (decisions.md "The M briefs give every cross-track hook one owner").
  - M2 adds effect name rows to `crates/minecraft_survival/src/commands.rs` (near `:169-172`) and M3 adds the simulated kinds at `:147`. E3 adds only new `CommandWorld` methods in the `impl` (`:255` on), new `CommandJob` variants next to `Edit` (`:868`) and their arms in `answer_jobs` (`:995`).
  - `kill-selectors` puts its new walk over the vendored mob stores in a new file. It edits vendored `entities/src/world.rs` only by one `mod` line, and the registry families (D0, M1-M3, F) come in through the registry's own `hurt_family` (`world/families.rs:1359`).
  - E3 adds no game events. W's w3 adds the missing ones, and a mob killed by `/kill` dies through the same `take_deaths` path as any other death.
- **Protocol:** only `oven-book` changes the wire (`RecipeBookView` and `ToServer::RecipeBookSettings`). Note it under "## Protocol" in `docs/round10/track-e3-log.md` and never bump `minecraft_net::PROTOCOL`.
- **State docs:** a track run does not edit CLAUDE.md, `docs/MINECRAFT-SURVIVAL.md`, `docs/MULTIPLAYER.md`, `docs/PROGRESS.md` or `docs/round10/decisions.md`. New wording goes under "State doc changes" in the log:
  - `docs/MINECRAFT-SURVIVAL.md:400` (commands: `@e`, `@n` and selector options for `/kill`, `/setworldspawn`, `/item replace`).
  - `:405` (GUI gaps: the recipe book exists and the furnaces have one).
  - The test files' note "the game has no F3 screen" (`docs/round10/release-3-tests.md:113`, `release-4-tests.md:114`, `release-5-tests.md:13`).
- **Code rules:**
  - No code comments of any kind. Edits change only the lines they need, in the surrounding style. New files must pass `rustfmt --check`.
  - New enum variants and match arms go next to a related one. The exceptions are `INPUT_COMMAND_NAMES` and `McAction` in `debug-screen`: command ids are array positions (`crates/input_iw4/src/names.rs:91` `MINECRAFT_COMMAND_FIRST`), so the new action is appended.

---

### Group `setworldspawn` (S, 1 commit)

**Gap:** screens-11, no `/setworldspawn`. A friend's compass could not be shown following a moved spawn (`docs/round10/release-2-tests.md:556`).

**Cause**
- There is no node: the list at `crates/minecraft_commands/src/commands.rs:183-195` has `crate::spawnpoint::node()` (`:191`) but no world spawn command. The BlockPos and Rotation argument kinds and the angle helpers already exist in `crates/minecraft_commands/src/spawnpoint.rs:67-80` (`node`), `:82` (`angle`), `:19` (`wrap_degrees`) and `:61` (`in_spawnable_bounds`).
- Everything around it exists:
  - The spawn lives in `Globals.spawn` (`crates/minecraft_survival/src/server.rs:42`) and is written to `level.dat` by `save.rs:190`.
  - `session.rs:384` `send_spawn` resends a changed spawn to every friend. The test at `session.rs:2456-2459` already asserts "/setworldspawn reaches everyone".
  - Respawns read the spawn at `respawn.rs:51` (`level_spawn`), `:94` and `:143`, and other dimensions are supported (`save.rs:536` test, `respawn.rs:173`).
- The respawn search uses the stream's own copy: `crates/minecraft_terrain/src/terrain.rs:984` `world_spawn`, set at load (`:1068-1071`) and read by `respawn_position_at` (`:1193`) and `exact_spawn_position` (`:1197`).
- The command `Host` (`crates/minecraft_survival/src/commands.rs:39-58`, built at `:1074` and `:1233`) has no access to `Globals.spawn`.
- The MW2 band origins are fixed from the spawn at load (`crates/render_anim/src/minecraft_world.rs:576-580`, `bands.rs:47`). They must stay fixed for the session.

**Vanilla 26.3** (javap `SetWorldSpawnCommand.register` and `setSpawn`, `LevelData$RespawnData.of`, `ServerLevel.setRespawnData`)
- `setworldspawn` needs `LEVEL_GAMEMASTERS`. The forms are `setworldspawn`, `setworldspawn <pos>` and `setworldspawn <pos> <rotation>`.
  - Without a position, it uses `BlockPos.containing(source position)`. Without a rotation, it uses `WorldCoordinates.ZERO_ROTATION`.
  - `<pos>` goes through `BlockPosArgument.getSpawnablePos`, which fails with `argument.pos.outofbounds` like `/spawnpoint`.
- `RespawnData.of(level.dimension(), pos, yaw, pitch)` wraps the yaw (`Mth.wrapDegrees`) and clamps the pitch to ±90.
  - It works in the source's own dimension, whichever that is. The lang still has `commands.setworldspawn.failure.not_overworld`, but the command never uses it.
  - `ServerLevel.setRespawnData` goes to `MinecraftServer.setRespawnData`, which broadcasts the new default spawn.
- The reply `commands.setworldspawn.success` gets six arguments (x, y, z, yaw, pitch, dimension id) and goes to operators. The 26.3 English text has four placeholders, "Set the world spawn point to %s, %s, %s [%s]", so players read "Set the world spawn point to 100, 70, 100 [90.0]".

**Plan.** Commit "Minecraft /setworldspawn moves the world spawn like vanilla".
- `crates/minecraft_commands/src/spawnpoint.rs`:
  - `pub(crate) fn world_node()` under `node()`, with `pos` and `rotation` the way `spawnpoint` reads them, reusing `angle`, `wrap_degrees`, `in_spawnable_bounds` and `java_float`.
  - The reply with all six arguments and `send_command_feedback` as `/spawnpoint` does.
  - Register it after `crate::spawnpoint::node()` (`commands.rs:191`).
  - Add the English fallback next to the spawnpoint ones (`server.rs:1194`).
  - Add "setworldspawn" to the root completion list (`suggest.rs:842`) and "/setworldspawn [<pos>]" to the help test (`commands.rs:1025`).
- `CommandWorld::set_world_spawn(&mut self, respawn: &Respawn) -> bool` (default `false`) next to `set_respawn` (`crates/minecraft_commands/src/server.rs:165`). Implement it in the test `FakeWorld` (next to `:919`).
- `crates/minecraft_survival/src/commands.rs`:
  - `Host` gains `pub spawn: &'a mut RespawnData` next to `day`. Fill it at `:1074` and `:1233` from `globals.spawn`.
  - `set_world_spawn` builds `crate::save::world_spawn(dimension, pos)` with yaw and pitch (`save.rs:253`). When that dimension's level is loaded, it sets `level.terrain.stream.world_spawn` (`terrain.rs:984`), so the respawn search moves with it.
  - Nothing else changes: the next autosave writes `level.dat`, `send_spawns` tells the friends, and the band origins stay as loaded.

**Tests**
- `spawnpoint::tests` (commands crate):
  - `setworldspawn_takes_the_block_under_the_source_and_zero_rotation`.
  - `setworldspawn_wraps_the_yaw_and_clamps_the_pitch` (`100 70 100 450 120` gives yaw 90 and pitch 90).
  - `setworldspawn_outside_the_bounds_is_refused`.
  - The English reply "Set the world spawn point to 100, 70, 100 [90.0]".
  - A non-operator gets the unknown-command error.
- `commands::tests` (survival):
  - `setworldspawn_moves_the_spawn_the_respawn_search_and_the_save` (`Globals.spawn`, the stream's `world_spawn`, and a `level.dat` round trip through `save.rs`).
  - `setworldspawn_in_the_nether_sends_respawns_there` (`respawn::respawn_dimension`).
  - `the_band_origins_do_not_move`.

**Owner's steps** (survival, Normal, by day on open ground; read positions with `/tp @s ~ ~ ~`)
1. Walk about 40 blocks away from where you joined. Run `/setworldspawn`. The chat says "Set the world spawn point to X, Y, Z [0.0]" with your block position. Hold the compass and walk around that spot: the needle turns to it. **scr01_setworldspawn**
2. `/kill`. You respawn within about 10 blocks of the new spot (default `respawn_radius`), facing south.
3. `/setworldspawn 100 70 100 90 0`. The chat says "Set the world spawn point to 100, 70, 100 [90.0]". `/kill`: you respawn near 100 70 100 facing west.
4. Save and quit, then load the world and `/kill`: you respawn near 100 70 100 again.
5. Place the bed, sleep or click it to set your spawn point, and `/kill`: the bed wins over the world spawn, as in vanilla. Break the bed.
6. Build and light a Nether portal and go through. In the Nether run `/setworldspawn`, then `/kill`: you respawn in the Nether near that spot. **scr02_setworldspawn_nether** Go back through the portal and run `/setworldspawn` in the overworld again.
7. Friend window: the friend holds a compass while the host runs `/setworldspawn` somewhere new. The friend's needle turns to the new spot. The friend runs `/kill` (the host can use `/kill <friend name>`) and respawns there. **scr03_friend_compass**

```
IW4L_MINECRAFT_KIT=minecraft:compass,minecraft:obsidian*14,minecraft:flint_and_steel,minecraft:white_bed,minecraft:cooked_beef*32,minecraft:stone*64
```

---

### Group `book-keys` (S, 1 commit)

**Gap:** screens-2, the recipe book's keys: "The book has no furnace version, no keyboard navigation and no bounce for new recipes" (`docs/round10/release-1-tests.md:404`). The gamepad part of screens-2 is left out (see "Left out").

**Cause**
- `crates/hud/src/minecraft_recipe_book.rs:212` `input` handles only the mouse, typing in a focused search box (`:364` `type_query`) and the chat key focusing it (`:262-263`).
- Escape always closes the whole screen: `crates/hud/src/minecraft_inventory.rs:253-254` flips `ui.inventory_open` on `KeyCode::Escape`. That happens even in narrow mode (`narrow()` `:48`, `covered()` `:60`), where the book covers the inventory.
- Nothing re-places the last recipe. `McRecipeBook.last_placed` (`crates/frame/src/minecraft_recipe_book.rs:73`, set in `place()` `:119-131`) is the ghost guard, and `clear_ghost()` (`:114`) clears it whenever the grid is touched, so it can't serve as vanilla's `lastRecipe`.
- The decision "The book is mouse only" is at `docs/round9/track-c-log.md:627`.

**Vanilla 26.3** (javap `RecipeBookComponent.keyPressed`, `mouseClicked`, `isOffsetNextToMainGUI`, `KeyEvent.isSelection`)
- `keyPressed` first clears `ignoreTextInput`. It returns false when the book is hidden or the player is a spectator.
- Escape when `!isOffsetNextToMainGUI()` (`xOffset != 86`, the narrow layout) calls `setVisible(false)` and returns true. In the wide layout Escape falls through and the screen closes.
- The search box gets the key next. While it `capturesInput()`, every key but Escape is eaten.
- The chat key focuses an unfocused search box and sets `ignoreTextInput`, so the "t" is not typed.
- `isSelection()` means Enter (40), Space (44) or keypad Enter (88). When `lastRecipeCollection` and `lastRecipe` are set, it plays `AbstractWidget.playButtonClickSound` and calls `tryPlaceRecipe(lastRecipeCollection, lastRecipe, hasShiftDown())`.
- `lastRecipe` and `lastRecipeCollection` are written only in `mouseClicked`, after a click on a recipe button whose `tryPlaceRecipe` returned true. The component is rebuilt when the screen opens, so they start empty. `tryPlaceRecipe` refuses an uncraftable recipe equal to `lastPlacedRecipe`, which is the existing `place()` rule.

**Plan.** Commit "Minecraft recipe book takes Escape, Enter and Space like vanilla".
- `frame::McRecipeBook` gains `last_recipe: Option<McRecipeOption>` next to `last_placed`. It is cleared in `reopen()` (`:102`) but not in `clear_ghost()`. `click()` (`hud minecraft_recipe_book.rs:290`) sets it when `book.place(..)` returned `Some`, in both the button branch (`:327-334`) and the overlay branch (`:307-315`).
- `BookFrame` gains `select: bool`, true when Enter, keypad Enter or Space was just pressed. `input()` handles it after the chat-key branch and only when the search box is not focused.
  - The current option with the stored id is looked up in `book.buttons` for a fresh `craftable` flag. If it is not shown, the stored copy is used.
  - Then `clicks.extend(book.place(&option, frame.shift))`, and when that gives a click, `ui.ui_sounds.push(crate::minecraft_book::BUTTON_SOUND)`.
  - In narrow mode the book closes as a click does.
- Escape: `pub(crate) fn takes_escape(ui, gui) -> bool` returns `has_book && open && narrow && !spectator`.
  - In `crates/hud/src/minecraft_inventory.rs:253`, an Escape that the book takes sets `ui.recipe_book.open = false` instead of closing the screen.
  - The GUI size is not known yet at `:253`, so keep last frame's `[space.width, space.height]` in `InventoryInput` (next to `book`, `:88`) and use it. The book was drawn with that size.
- A focused search box keeps eating Space and Enter (`type_query` types the space). Escape while it is focused follows the narrow and wide rule above.

**Tests** (`crates/hud/src/minecraft_recipe_book.rs` and `minecraft_inventory.rs`)
- `escape_hides_only_a_narrow_book`: narrow and open, so the book closes and the inventory stays open. Wide, so the screen closes.
- `enter_and_space_place_the_last_recipe_again_and_shift_places_all`: a `PlaceRecipe` with `all` following Shift, plus the click sound.
- `enter_does_nothing_before_a_recipe_was_clicked_and_after_reopening`.
- `a_focused_search_box_takes_space_as_text_and_ignores_enter`.
- `an_uncraftable_last_recipe_is_not_sent_twice`: the ghost guard still holds for Enter.

**Owner's steps** (survival; a normal 1920x1080 window for steps 1-3)
1. Press E and click the recipe book button. Click the oak planks recipe: 1 log goes into the 2x2 grid. Press Enter: a second log goes in. Press Space: a third. Hold Shift and press Enter: the grid fills to a full stack. **scr04_book_enter**
2. Open a crafting table, open the book and click the chest recipe: the planks go in. Take the chest out, then press Enter: the chest recipe goes in again.
3. Click the search box (or press T) and type "stick": Space and Enter only type into the box, and Escape closes the screen.
4. Make the window narrow (windowed, about 1280x960, or use the second window from `scripts\mp-local.ps1`). The book now covers the inventory when opened. Press Escape: only the book closes and the inventory stays. Press Escape again: the inventory closes. **scr05_book_escape_narrow**
5. Friend window: the friend repeats steps 1 and 4.

```
IW4L_MINECRAFT_KIT=minecraft:oak_log*64,minecraft:cobblestone*64,minecraft:iron_ingot*32,minecraft:stick*32,minecraft:crafting_table,minecraft:cooked_beef*16
```

---

### Group `debug-screen` (M, 2 commits)

**Gap:** screens-5, no F3 screen. The test steps work around it with `/tp @s ~ ~ ~` (`docs/round10/release-5-tests.md:13`, `release-3-tests.md:113`, `release-4-tests.md:114`), and round 9's steps still say "press F3" (`docs/round9/release-1-tests.md:61`, `:76`, `:184`; `release-2-tests.md:140`).

**Cause**
- Nothing handles F3 (`crates/console/src/binds.rs:738` only names the key).
- The Minecraft actions are `Inventory`, `Drop`, `SwapHands`, `Chat`, `Command`, `Perspective` and `Skate` (`crates/frame/src/minecraft_ui.rs:450-486`). Their bits are a `u8` (`McActionInput`, `:489-530`) with exactly one bit left.
- The default binds are at `crates/console/assets/default_controls.cfg:29-35`. The command names are at `crates/input_iw4/src/names.rs:79-86` (`mc_skate` is id 84, pinned at `:175-179`).
- Older settings files get new keys by header version: `crates/console/src/user_settings.rs:343` writes `v6`, and `:500-528` migrate older headers; `binds.rs:374` `add_skate_key` is the model. The settings page lists the Minecraft keys in `crates/ui/menus/settings.json:148-163`, with the skate key row at y 168.
- The data exists on both sides:
  - The host: feet and view angles in `crates/render_anim/src/minecraft_world.rs` near `:1269-1277`, `minecraft_yaw` at `:444`, the targeted block from `aim.target` at `:927`, the terrain light (`minecraft_terrain::lighting::SkyLight` `get` and `get_block`, used in `leash_render.rs:221`) and biomes (`terrain.rs:240` `biome_name`).
  - The friend: the same in `crates/render_anim/src/minecraft_remote.rs` near `:1096` (aim) and `:1301` (UI fill).

**Vanilla 26.3** (javap `Options.keyDebugOverlay`, `DebugScreenOverlay.extractLines`, `DebugScreenEntries.<clinit>`, `DebugEntryPosition`, `DebugEntrySectionPosition`, `DebugEntryBiome`, `DebugEntryLight`, `DebugEntryVersion`, `DebugEntryFps`, `DebugEntryLookingAt$BlockStateInfo`)
- F3 is the rebindable `key.debug.overlay`. The overlay works over screens unless an edit box is taking input.
- **Drawing.** `extractLines(lines, left, screenWidth)`, for each non-empty line i:
  - Line height 9. `x = left ? 2 : screenWidth - 2 - width`, `y = 2 + 9i`.
  - `fill(x - 1, y - 1, x + width + 1, y + 8, 0x90505050)`, then the text at (x, y) in `0xFFE0E0E0` with no shadow.
  - Empty lines leave a gap.
- **The 26.3 default profile** (`DebugScreenEntries.PROFILES`, `DEFAULT`): `3d_crosshair`, `game_version`, `tps`, `fps`, `memory`, `system_specs`, `player_position`, `player_section_position`, `simple_performance_impactors`. `biome`, `light_levels` and `looking_at_block_state` are optional entries that players switch on in the debug options screen (F3+F6).
- **Formats:**
  - Version: `Minecraft %s (%s/%s)`.
  - FPS: `%d fps T: %s%s @%sHz`.
  - Position:
    - `XYZ: %.3f / %.5f / %.3f`
    - `Block: %d %d %d`
    - `Chunk: %d %d %d [%d %d in r.%d.%d.mca]`, with the section x y z, then chunk x & 31 and chunk z & 31, then the region chunk x >> 5 and z >> 5.
    - `Facing: %s (%s) (%.1f / %.1f)`, with the direction name, "Towards negative Z" / "Towards positive Z" / "Towards negative X" / "Towards positive X", then yaw and pitch.
  - Section: `Section-relative: %02d %02d %02d`.
  - Biome: `Biome: <id>`.
  - Light: `Client Light: %d (%d sky, %d block)`.
  - Targeted block, in the right column: the underlined title `Targeted Block: x, y, z`, then the block id, then one `name: value` line per property.

**Plan**
1. **"Minecraft debug screen key is a rebindable control, F3 by default"**
   - `McAction::Debug`, appended after `Skate` with command `mc_debug`, in `ALL` (length 8). Append `"mc_debug"` to `INPUT_COMMAND_NAMES` (length 86, id 85) and update the id range tests (`names.rs:160-185`).
   - Add `bind F3 mc_debug` to `default_controls.cfg`.
   - `binds.rs`: `DEBUG_COMMAND` and `add_debug_key()` modelled on `add_skate_key`: F3 is added only when it is free in both layers and `mc_debug` is unbound. Exclude it from `apply_minecraft_defaults` the way skate is (`:366-369`). Update the defaults test (`:843-861`, 8 binds).
   - `user_settings.rs`: write `// IW4L user settings v7`, call `add_debug_key()` for headers v1 to v6, and extend the tests the way 1daef351 did for the skate key.
   - Add a "Debug Screen" bind row at y 188 in `crates/ui/menus/settings.json`, plus its entry in `MINECRAFT_KEYS` (`crates/ui/src/menu.rs:197`), as 1daef351 did.
   - `MinecraftUi.debug_screen: bool` next to `camera_overlay` (`minecraft_ui.rs:282`). `crates/hud/src/minecraft_inventory.rs` toggles it on `mc(McAction::Debug)` when the console and menus don't block (`blocked`, `:234`), no text box is typing (`typing`, `:250`) and the chat is closed. As in vanilla, it toggles with screens open too.
2. **"Minecraft F3 screen shows the version, position, facing, biome, light and the targeted block"**
   - Frame `McDebug { left: Vec<String>, right: Vec<String> }`, and `MinecraftUi.debug: Option<McDebug>` next to `debug_screen`.
   - New `crates/render_anim/src/minecraft_debug.rs` with a pure `lines(input: DebugInput) -> McDebug`. `DebugInput` holds feet, yaw, pitch, fps, sky and block light, sky darkening, biome id and the targeted block id with its properties.
   - Left column:
     - `Minecraft 26.3 (26.3/soscraft)` and `<fps> fps`, then an empty line.
     - `XYZ`, `Block`, `Section-relative`, `Chunk`, `Facing` in vanilla's formats, using vanilla's yaw (`minecraft_yaw`, wrapped to -180..180) for the direction and the numbers.
     - An empty line, then `Client Light: max(sky - darkening, block) (S sky, B block)`, with the darkening as in `minecraft_terrain::placing::raw_brightness` (`placing.rs:115`).
     - `Biome: minecraft:<id>`.
   - Right column, when a block is targeted: `Targeted Block: x, y, z`, the id, and the `name: value` property lines.
   - The host fills `ui.debug` next to `ui.effects` (`minecraft_world.rs:1277`) and the friend next to `minecraft_remote.rs:1301`, only while `ui.debug_screen` is on, from its own player, level, aim and light. FPS is counted per second in the hud from frame times, like vanilla's `Minecraft.fps`.
   - New `crates/hud/src/minecraft_debug.rs`: `draw(p, &McDebug, width)` with vanilla's geometry and colours. It is called in `crates/hud/src/minecraft_inventory.rs` right after `minecraft_hotbar::draw_hud` (`:360`) and registered in `crates/hud/src/lib.rs` next to `minecraft_credits`.

**Decisions (for the log)**
- The fork shows a fixed list: the default profile entries it can fill (version, fps, position, section position) plus biome, client light and the targeted block from vanilla's optional entries. The debug options screen (F3+F6) is left out, and these are the lines the test steps need.
- No TPS, memory, system specs or server lines.

**Tests**
- `input_iw4`: `mc_debug_is_command_85`.
- `frame`: `every_minecraft_action_has_its_own_command` covers `Debug`, and `minecraft_action_presses_last_until_cleared` covers bit 7.
- `console`:
  - The defaults test with F3.
  - `older_settings_get_the_debug_key_on_a_free_f3` (v1 to v6).
  - `an_f3_bound_in_the_mw2_layer_keeps_the_debug_key_unbound`.
  - `a_rebound_debug_key_survives_a_restart`.
  - The menu dvar `ui_bind_mc_debug` is "f3".
- `render_anim::minecraft_debug::tests`:
  - `the_position_lines_match_vanilla`: feet (-12.3, 64.0, 40.5) give `XYZ: -12.300 / 64.00000 / 40.500`, `Block: -13 64 40`, `Section-relative: 03 00 08` and `Chunk: -1 4 2 [31 2 in r.-1.0.mca]`.
  - `facing_names_the_direction_and_axis`: yaw 0 gives "south (Towards positive Z)", 90 "west (Towards negative X)", 180 "north (Towards negative Z)" and -90 "east (Towards positive X)".
  - `the_targeted_block_lists_its_properties`.
  - `client_light_subtracts_the_sky_darkening`.
- `hud::minecraft_debug::tests`:
  - `lines_have_the_vanilla_background_and_colour`.
  - `the_right_column_is_right_aligned`.
  - `empty_lines_leave_a_gap`.
- `hud::minecraft_inventory`: `the_debug_key_toggles_but_not_while_typing`.

**Owner's steps** (survival, by day)
1. Press F3. Grey-backed white text shows at the top left:
   - "Minecraft 26.3 (26.3/soscraft)" and the fps.
   - Then XYZ, Block, Section-relative, Chunk and Facing.
   - Then Client Light and Biome.

   Run `/tp @s ~ ~ ~`: the chat's position matches the XYZ line. **scr06_f3_left**
2. Turn to face north: Facing shows "north (Towards negative Z)". Face east: "east (Towards positive X)".
3. Place an oak stair and look at it. The right column shows "Targeted Block: x, y, z", "minecraft:oak_stairs" and its properties (facing, half, shape, waterlogged). **scr07_f3_targeted_block**
4. Place a torch on the ground and stand next to it at night or in a cave: the block light in "Client Light" goes up near the torch. Stand in a desert or on a beach: Biome changes.
5. Press E with F3 on: the text stays over the inventory. Open the chat and type "f3text": typing does not toggle it. Press F3 again with the chat closed: the text goes.
6. In the MW2 settings, Minecraft page: the new row "Debug Screen" shows F3. Rebind it to F4, then F4 toggles the screen. Restart the game: it is still F4. Rebind it back to F3.
7. Friend window: F3 shows the friend's own position and biome, matching the friend's `/tp @s ~ ~ ~`. **scr08_f3_friend**

```
IW4L_MINECRAFT_KIT=minecraft:oak_stairs*16,minecraft:torch*32,minecraft:stone*64,minecraft:water_bucket,minecraft:cooked_beef*16
```

---

### Group `item-replace` (M, 2 commits)

**Gap:** screens-12, no `/item`. Banners and other items can't be put on the head (`docs/round10/track-e1-log.md:1056`), and the only way to put items in is `/give`.

**Cause**
- There is no `item` literal in `crates/minecraft_commands` (node list `commands.rs:183-195`) and no slot argument kind in `args.rs` (`Arg` `:93-118`, `ArgKind::parse` `:120` on).
- The only item path is `/give`: `commands.rs:157-170`, `CommandWorld::give` (`server.rs:120`), implemented at `crates/minecraft_survival/src/commands.rs:371`.
- The worn head already draws any item with its look (track E1 e1b), and `Inventory.slots` follows vanilla's indices:
  - 0-35 for the hotbar and backpack.
  - Armour at 36-39 (`crates/minecraft_survival/src/digging.rs:10` `FEET`).
  - The offhand at 40 (`offhand.rs:6`).
- Block containers live in each level's `Containers` (`crates/minecraft_survival/src/level.rs:22`; chests, barrels and shulker boxes, ovens, brewers; `crates/minecraft_terrain/src/containers.rs:603`). The menus reach their cells through `containers.cells(opened)` (`menus.rs:592`).

**Vanilla 26.3** (javap `ItemCommands.register`, `setItem`, `BlockItemAccessor`, `EntityItemAccessor`, `SlotsArgument`, `SlotRanges`, `Player.getSlot`)
- The syntax is `item replace (block <pos> | entity <targets>) <slots> with <item> [<count>]`. `count` is `IntegerArgumentType.integer(1, 99)`, and the stack is `ItemInput.createItemStack(count)` with no stack size check.
- The other forms are `from <source> <sourceSlots> [<modifier>]`, `item fill`, `item override` and `item modify` (loot item functions).
- **Slots** (`SlotRanges`): `contents`, `container.0-53`, `hotbar.0-8`, `inventory.0-26` (index 9 + N), `enderchest.0-26` (200 + N), `mob.inventory.0-7`, `horse.0-14`, `weapon`, `weapon.mainhand`, `weapon.offhand`, `armor.head`, `armor.chest`, `armor.legs`, `armor.feet`, `armor.body`, `saddle`, `horse.chest`, `player.cursor`, `player.crafting.0-3`. Each family also has a `.*` range (`container.*`, `armor.*` and so on).
  - An unknown name is `slot.unknown` ("Unknown slot '%s'").
- **A player's slots** (`Player.getSlot`):
  - 0-35 are the inventory (so `container.N` below 36 works).
  - 200 + N is the ender chest.
  - 499 is the cursor and 500-503 the crafting grid.
  - The rest go through `LivingEntity.getSlot` (equipment: mainhand, offhand, armour).
- **Blocks:** the block entity must be a `Container`, or the error is `commands.item.target.not_a_container` ("Target position %s, %s, %s is not a container"). A slot outside the container is `commands.item.target.no_such_slot` ("The target does not have slot %s").
- **Replies** (26.3 en_us):
  - "Replaced %s slot(s) at %s, %s, %s with %s" (`commands.item.block.replace.success.known_item`).
  - "Replaced %s slot(s) on %s with %s" (`commands.item.entity.replace.success.single.known_item`).
  - "Replaced slot(s) on %s entities with %s" (`...multiple.known_item`).
  - The failure "No targets accepted item %s into specified slots" (`commands.item.target.failed.known_item`).
  - Check the exact key choice in `EntityItemAccessor.getReplaceSuccess` and `BlockItemAccessor.getReplaceSuccess`.

**Plan**
1. **"Minecraft commands read item slots like vanilla"**
   - `ArgKind::Slots` and `Arg::Slots(Vec<u16>)` next to `Item`. The parse reads a slot name or `.*` range from vanilla's table above, with vanilla's indices; unknown names give `slot.unknown` with the name.
   - Suggestions in `suggest.rs` list the names, as the item and block arguments do.
2. **"Minecraft /item replace fills player and container slots"**
   - New `crates/minecraft_commands/src/item.rs`: `node()` for `item replace block <pos> <slots> with <item> [<count>]` and `item replace entity <targets: players> <slots> with <item> [<count>]`, the replies and the errors above.
     - The block position goes through the existing loaded-position checks (`blocks.rs:420-426`).
     - Register it in `commands.rs` in alphabetical order next to `give`, and add it to the help and suggest lists.
   - `CommandWorld::replace_player_slots(player, slots, item, count) -> u32` and `replace_block_slots(dimension, pos, slots, item, count) -> Result<u32, SlotError>`, with defaults, next to `give`.
   - `crates/minecraft_survival/src/commands.rs`: build the stack like `give` does (`:371-385`, components and the full clip included).
     - Players: 0-35 go to `inventory.slots[N]`, `armor.feet..head` to 36-39, `weapon.offhand` to 40, `weapon.mainhand` to the selected hotbar slot, enderchest to `player.ender`, the cursor and crafting to `inventory.cursor` and `inventory.crafting`.
     - Blocks: the level's `Containers` cells for chests, barrels, shulker boxes, ovens (0 input, 1 fuel, 2 result) and brewers, marked dirty so they save and show to an open screen.
   - Vanilla allows counts above the max stack (up to 99). Check that the fork's click and merge code tolerates `count > max`. If it does not, cap at `max` and record that in the log.

**Tests**
- `args::tests::slots_parse_like_vanilla`: `hotbar.8` gives 8, `inventory.0` 9, `armor.head` the head index, `container.*` 54 slots. `hotbar.9` and `foo` give `slot.unknown`.
- `item::tests`:
  - `replace_entity_puts_the_stack_in_the_slot_and_replies` (`FakeWorld`).
  - `replace_block_needs_a_container`.
  - `a_furnace_has_no_container_30`.
  - `the_count_is_one_to_ninety_nine`.
- Survival `commands::tests`:
  - `replace_slots_map_vanilla_indices_to_the_inventory` (head 39, offhand 40, mainhand the selected slot, `inventory.0` slot 9, `enderchest.0`).
  - `a_chest_and_a_furnace_take_items_by_command` (saved and seen in the open menu).

**Owner's steps** (survival)
1. `/item replace entity @s armor.head with minecraft:white_banner[banner_patterns=[{pattern:"minecraft:creeper",color:"black"}]]`. The chat says "Replaced 1 slot(s) on <your name> with [White Banner]". Press F5 twice: the banner with the creeper face sits on your head. **scr09_item_head_banner**
2. `/item replace entity @s weapon.offhand with minecraft:shield`: a shield appears in the offhand. `/item replace entity @s hotbar.8 with minecraft:diamond 5`: 5 diamonds in the last hotbar slot. `/item replace entity @s inventory.0 with minecraft:stone 64`: stone in the first backpack slot (E to check).
3. Place the chest and stand on it. `/item replace block ~ ~-1 ~ container.0 with minecraft:emerald 3` says "Replaced 1 slot(s) at x, y, z with [Emerald]". Open the chest: 3 emeralds in the first slot.
4. Place the furnace and stand on it. `/item replace block ~ ~-1 ~ container.0 with minecraft:raw_iron 8`, then `/item replace block ~ ~-1 ~ container.1 with minecraft:coal 2`. The furnace lights and smelts. Open it: iron ingots come out. **scr10_item_furnace**
5. Errors:
   - `/item replace block ~ ~-1 ~ container.30 with minecraft:stone` on the furnace says "The target does not have slot container.30".
   - Standing on grass, `/item replace block ~ ~-1 ~ container.0 with minecraft:stone` says "Target position x, y, z is not a container".
   - `/item replace entity @s hotbar.9 with minecraft:stone` says "Unknown slot 'hotbar.9'".
6. Friend window: the host runs `/item replace entity <friend name> armor.head with minecraft:carved_pumpkin`. The friend's view gets the pumpkin blur, and the host sees the pumpkin on the friend's head. **scr11_item_friend_pumpkin**

```
IW4L_MINECRAFT_KIT=minecraft:chest,minecraft:barrel,minecraft:furnace,minecraft:coal*16,minecraft:cooked_beef*16
```

---

### Group `recipe-bounce` (M, 2 commits)

**Gap:** screens-3, newly unlocked recipes and their tabs do not bounce ("no bounce for new recipes", `docs/round10/release-1-tests.md:404`; `docs/round9/track-c-log.md:679`).

**Cause**
- The data exists:
  - The server keeps vanilla's toBeDisplayed (`crates/minecraft_survival/src/recipe_book.rs:28` `highlight`, `:62` `pending`, `:97` `seen`) and sends it to friends (`crates/minecraft_net/src/messages.rs:342` `RecipeBookView.highlight`).
  - The friend keeps it (`crates/render_anim/src/minecraft_recipe_book.rs:203` `FriendBook.highlight`) and reports what was shown (`:222` `outgoing`, `ToServer::RecipeSeen`).
  - The host reports through `crates/render_anim/src/minecraft_world.rs:1393` `follow_screen(.., &inventory_ui.take_seen())`.
- The client never gets the highlight set:
  - `RecipeBookUi::build` (`render_anim/src/minecraft_recipe_book.rs:61`) takes none.
  - `frame::McRecipeButton` (`crates/frame/src/minecraft_recipe_book.rs:16`) and `McRecipeBook.tabs` (`:76`) have no "new" flag.
- `crates/render_anim/src/minecraft_inventory.rs:600-602` marks every built button as seen, also when the book is closed and only a ghost is built (`build` runs for `state.open || state.ghost.is_some()`, `render_anim minecraft_recipe_book.rs:69`).
- The draw (`crates/hud/src/minecraft_recipe_book.rs:491`, tabs `:497-517`, buttons `:545-570`) has no animation, and the `Painter` (`crates/hud/src/minecraft_gui.rs:195-260`) has no scale.

**Vanilla 26.3** (javap `RecipeButton.init`, `RecipeButton.extractWidgetRenderState`, `RecipeBookTabButton.startAnimation`, `RecipeBookTabButton.extractContents`, `GameRenderer` passing `DeltaTracker.getGameTimeDeltaTicks` to screens)
- **Buttons.** `RecipeButton.init(collection, ...)`: when any selected entry `ClientRecipeBook.willHighlight`, it calls `page.recipeShown(id)` for it (which removes the highlight and sends the seen packet) and sets `animationTime = 15.0f`.
  - The page reuses its 20 buttons, so a re-init without a new recipe keeps a running animation.
  - While `animationTime > 0`, the button is scaled by `1 + 0.1·sin(animationTime / 15 · π)` about its centre, and `animationTime` drops by the frame time in ticks. It lasts 15 ticks (0.75 s) and peaks at 1.1.
- **Tabs.** `RecipeBookTabButton.startAnimation(book, filtering)` sets `animationTime = 15.0f` when any recipe of the tab's categories (craftable ones when filtering) `willHighlight`. The component calls it from `updateTabs` (book shown, tab or filter changed) and `recipesUpdated`.
  - `extractContents` scales Y only, by the same formula, about (x + 8, y + 12).

**Plan**
1. **"Minecraft recipe book knows which shown recipes are new"**
   - `RecipeBookUi::build` takes `highlight: &HashSet<String>` (hashed into the rebuild key).
   - `Choice` gains `new: bool` (in the highlight set), and `Built` gains `tab_new: [bool; MC_RECIPE_TABS]` (the tab has a selected recipe, craftable if filtering, in the set).
   - `frame::McRecipeButton.new: bool`. `McRecipeBook` gains `tab_new` and `built: u64`, a counter bumped on every rebuild that the hud reads as vanilla's re-init.
   - The host passes `player.recipe_book.pending(&player.inventory)` (`recipe_book.rs:62`), kept on the inventory UI next to `unlocked`. The friend passes `FriendBook.highlight`, with a getter.
   - `seen` holds only the new ids of the built page, and only while `ui.recipe_book.open`.
2. **"Minecraft recipe book bounces new recipes and their tabs"**
   - `hud/src/minecraft_gui.rs`: `Painter::scale_since(&mut self, from: usize, centre: [f32; 2], scale: [f32; 2])` scales the quads pushed since `from` about a GUI-space centre, mapped through `space.vertex`.
   - `BookInput` gains `bounce: [f32; MC_RECIPES_PER_PAGE]`, `tab_bounce: [f32; MC_RECIPE_TABS]` and the last seen `built`.
     - When `built` changes, every button with `new` gets 15 and the others keep their time.
     - When the book opens, the tab or filter changes, or a rebuild turns a tab's `tab_new` on, that tab gets 15.
   - The countdown is `dt · 20` per frame (`ticks(now)` exists at `:40`).
   - `draw`: remember `p.quads.len()` before each button (sprite and items) and each tab, then `scale_since` with vanilla's formulas.

**Tests**
- `render_anim minecraft_recipe_book::tests`:
  - `new_recipes_are_marked_until_seen`.
  - `a_tab_with_a_new_recipe_is_marked_and_filtering_counts_only_craftable_ones`.
  - `only_new_shown_recipes_are_reported_and_only_while_the_book_is_open`.
- `hud minecraft_recipe_book::tests`:
  - `a_new_button_bounces_for_fifteen_ticks` (scale 1.1 at 7.5 ticks, 1.0 at 0 and 15).
  - `a_rebuild_without_new_recipes_keeps_a_running_bounce`.
  - `a_tab_bounces_in_y_about_8_12`.
- `hud minecraft_gui::tests::scaled_quads_scale_about_the_centre`.

**Owner's steps** (survival, a fresh world)
1. Press E and open the recipe book. The buttons of the iron recipes the kit unlocked (iron tools and armour, the bucket and others) swell and shrink once, and the tabs that hold them (the sword tab, the building tab) bounce up and down once. Take a short recording or screenshots in a row. **scr12_book_bounce**
2. Close and reopen the book: nothing bounces any more.
3. `/give @s minecraft:copper_ingot 8`, then open the book: only the copper recipes and their tabs bounce. Turn to the page that holds them: they bounce when shown.
4. Save and quit before opening the book after a new unlock (`/give @s minecraft:gold_ingot 8`, then quit), then load: the gold recipes still bounce the first time they show.
5. Friend window: the friend runs step 3 with `/give @s minecraft:copper_ingot 8` (as operator): the same bounce in the friend's book. **scr13_book_bounce_friend**

```
IW4L_MINECRAFT_KIT=minecraft:crafting_table,minecraft:oak_log*16,minecraft:cobblestone*32,minecraft:iron_ingot*8,minecraft:cooked_beef*16
```

---

### Group `kill-selectors` (L, 3 commits)

**Gap:** screens-10, `/kill @e` and selector options.
- `docs/MINECRAFT-SURVIVAL.md:400` says "no `@e`, `@n` or selector options, no mob targets".
- The test steps work around it with `/difficulty peaceful` (`docs/round10/track-d0-log.md:427`, `:567`) or kill by hand (`track-e1-log.md:702`).

**Cause**
- `crates/minecraft_commands/src/args.rs:290-340` `players()` refuses `@e` and `@n` with `argument.entity.selector.not_allowed` and any `[` with `argument.entity.options.unknown`. Tests pin both at `args.rs:930-939` and `suggest.rs:1004`.
- `Selector` (`args.rs:84-90`) has no options. `Ctx::targets` (`server.rs:311-340`) resolves `@p` to the source.
- The kill node is `commands.rs:153-156` with the players argument, and the handler is at `:558`. `CommandWorld` has only `kill(PlayerId)` (`server.rs:115`).
- The mobs are reachable on each level's server thread:
  - `EntityWorld::mob_ids` (vendored `entities/src/world.rs:5341`), `entity_type` (`:4905`), `body_mut` (`:5051`) and `damage_state` (`:5115`).
  - The saved NBT with `CustomName` and `Tags`: `minecraft_terrain::server_mobs::mob_tags_where` (`crates/minecraft_terrain/src/server_mobs.rs:1526`, kept keys `:2237`).
  - Level entities (items, experience orbs, falling blocks, primed TNT, ominous item spawners) are in vendored `world/src/level/entity.rs` (`kill_tagged` `:1727`).
- There is no generic kill. Each store has its own hurt, and the walks to copy are `hurt_mob_by_magic` (`world/witches.rs:724`) and the registry's `hurt_family` (`world/families.rs:1359`).
- Deferred command answers exist: `run_job` with `CommandJob` (`crates/minecraft_survival/src/commands.rs:626-641`, `:868`), finished in `answer_jobs` (`:995`).

**Vanilla 26.3** (javap `KillCommand.register` and `kill`, `EntitySelectorParser`, `EntitySelectorOptions`, `EntityArgument`, `LivingEntity.kill`, the damage type tags in the jar)
- `kill [targets: EntityArgument.entities()]` calls `Entity.kill(level)` on each target.
  - For a living entity that is `hurtServer(genericKill, Float.MAX_VALUE)`. `minecraft:generic_kill` is in `#bypasses_invulnerability`, `#bypasses_armor`, `#bypasses_resistance` and `#no_knockback`, so drops and death are as usual.
  - Anything else is discarded.
- Replies: `commands.kill.success.single` "Killed %s" (the display name) or `.multiple` "Killed %s entities". No target is `argument.entity.notfound.entity` "No entity was found".
- **Selectors:** `@e` (all entities), `@n` (nearest entity), `@s`, `@p`, `@a`, `@r`.
  - Options: `type` (with `!` and `#tag`), `name` (with `!`), `tag` (with `!`, repeatable), `distance` (`..5`, `2..`, `2..5`), `limit`, `sort` (`nearest`, `furthest`, `random`, `arbitrary`), and `x`, `y`, `z`, `dx`, `dy`, `dz`.
  - Errors:
    - `argument.entity.options.unknown`, `.valueless`, `.unterminated` and `.inapplicable`.
    - `.type.invalid`, `.limit.toosmall`, `.distance.negative` and `.sort.irreversible`.
  - A player-only argument given a selector that may include entities is `argument.player.entities` ("Only players may be affected by this command, but the provided selector includes entities").
  - `@e` includes players, so `/kill @e` kills the source too.

**Plan**
1. **"Minecraft commands parse entity selectors and their options"**
   - `args.rs`: `EntitySelector { kind: Me | Nearest | All | Random | Entities | NearestEntity | Name, options }` with `SelectorOptions { types: Vec<(String, bool)>, names, tags, distance: Option<(f64, f64)>, limit, sort, at: [Option<f64>; 3], size: [Option<f64>; 3] }`.
   - `ArgKind::Entities { single }` and `Arg::Entities(EntitySelector)` next to `Players`. One option parser is shared with player arguments, so `@a[distance=..10,limit=2,sort=nearest]` works for every player command.
   - Player arguments given `@e` or `@n` (without `type=player`) fail with `argument.player.entities`. Update the two pinned tests to vanilla's key and text.
   - Entity types are checked against `CommandRegistries.entities`. Suggestions list the option names after `[` and the values for `sort` and `type`.
   - `Ctx::targets` applies `distance`, `limit`, `sort`, `name`, `tag` and the box to players, with `ctx.world.pose`. `@p` becomes the true nearest.
2. **"Minecraft levels kill the mobs and items a selector finds"**
   - New vendored `third_party/minecraftoss/entities/src/world/kills.rs` (one `mod kills;` line next to the other `world/` modules): `EntityWorld::kill(id) -> bool`.
     - It is `hurtServer` with `"minecraft:generic_kill"` and `f32::MAX` through each store's own hurt, walked like `hurt_mob_by_magic` but with no anger and no knockback, plus the registry arm through `hurt_family`.
     - Count the new vendored file in the log.
   - Vendored `world/src/level/entity.rs`: `kill(id)` next to `kill_tagged`, for level entities.
   - `ServerSim` gets a selector job. It walks `mob_ids` and the level entities and builds candidates (id, type, position, the `CustomName` and `Tags` from the saved NBT). It filters by type, name, tag, distance and box from the source position, sorts and limits, kills the matches, and returns `(count, first display name)`. The display name is the custom name or `entity.minecraft.<type>`.
   - Not included: projectiles and the ender dragon and end crystals (left out).
3. **"Minecraft /kill takes @e, @n and selector options like vanilla"**
   - The kill node takes `ArgKind::Entities { single: false }`.
   - `CommandWorld::kill_entities(source, &EntitySelector, dimension) -> Answer<..>` next to `kill`:
     - Players that match are killed at once on the main thread.
     - The mobs and items go to the source dimension's level as a job (`CommandJob::Killed` next to `Edit`), answered in `answer_jobs` with the combined count.
     - The reply is `commands.kill.success.single` or `.multiple`, or `argument.entity.notfound.entity` when nothing matched.
   - `/kill` with no target and with player selectors keeps today's path.

**Tests**
- `args::tests`:
  - `entity_selectors_parse_with_options`.
  - `selector_option_errors_match_vanilla` (every error key above).
  - `player_arguments_refuse_entity_selectors_with_vanilla_s_error`.
  - `player_options_filter_and_sort_players`.
- `suggest::tests::option_names_follow_the_bracket`.
- Vendored `entities` tests:
  - `kill_hurts_every_store_with_generic_kill` (a zombie, a cow, a villager and a registry family each die and come out of `take_deaths` with loot).
  - `kill_ignores_invulnerable_and_armour`.
- `minecraft_terrain server::tests::a_selector_job_kills_matching_mobs_and_items` (type, `!`, name, tag, distance, limit and sort).
- Survival `commands::tests`:
  - `kill_at_e_counts_players_and_mobs_together`.
  - `kill_with_no_match_says_no_entity_was_found`.

**Owner's steps** (survival, Normal; run `/effect give @s minecraft:resistance 600 4` first)
1. Fence a pen about 8 blocks away. Hatch 4 cows and 4 sheep inside it. Drop 5 dirt with Q outside it, next to you.
2. `/kill @e[type=minecraft:cow]`. The chat says "Killed 4 entities". The cows flash red, fall over and drop beef and leather, and the sheep stay. **scr14_kill_cows**
3. `/kill @n[type=sheep]` says "Killed Sheep" and the nearest sheep dies. `/kill @e[type=sheep,limit=2,sort=furthest]` kills the two furthest sheep.
4. `/summon minecraft:zombie ~4 ~ ~ {CustomName:"Bob"}` then `/kill @e[name=Bob]` says "Killed Bob". `/summon minecraft:zombie ~4 ~ ~ {Tags:["e3"]}` twice, then `/kill @e[tag=e3]` says "Killed 2 entities".
5. `/kill @e[type=item,distance=..6]`: the dirt, beef and leather near you vanish and the chat counts them. **scr15_kill_items**
6. `/kill @e[type=!player,distance=..30]`: everything else near you dies and you live. `/kill @e[type=pig]` with no pigs says, in red, "No entity was found".
7. Errors:
   - `/kill @e[foo=1]` says "Unknown option 'foo'".
   - `/kill @e[limit=0]` says "Limit must be at least 1".
   - `/kill @e[type=minecraft:nope]` says "Invalid or unknown entity type 'minecraft:nope'".
   - `/give @e minecraft:stone` says "Only players may be affected by this command, but the provided selector includes entities".
8. Type `/kill @e[` without sending: the suggestions list the options. **scr16_kill_suggestions**
9. Friend window: hatch two cows next to the friend and one next to the host, about 40 blocks apart. The friend (operator) runs `/kill @e[type=cow,distance=..10]`: only the cows near the friend die. **scr17_kill_friend**
10. `/kill @e` kills you too (vanilla). Respawn.

```
IW4L_MINECRAFT_KIT=minecraft:cow_spawn_egg*8,minecraft:sheep_spawn_egg*8,minecraft:zombie_spawn_egg*4,minecraft:oak_fence*64,minecraft:oak_planks*64,minecraft:dirt*16,minecraft:torch*16,minecraft:cooked_beef*16
```

---

### Group `oven-book` (L, 3 commits)

**Gap:** screens-1, the furnace, blast furnace and smoker recipe books ("The book has no furnace version", `docs/round10/release-1-tests.md:404`; "the furnace books are not built", `docs/round9/track-c-log.md:627`).

**Cause**
- The book is built for the inventory and the crafting table only:
  - `crates/hud/src/minecraft_recipe_book.rs:44` `has_book` matches `Inventory | CraftingTable`, and `:69` `button` has their two positions only.
  - `crates/render_anim/src/minecraft_recipe_book.rs:34` `grid_of` returns `None` for every other screen.
- The tabs are the crafting ones: `crates/minecraft_terrain/src/recipe_book.rs:12-56` (`Tab`, `SEARCHED`, `of_category`, `icons`) and `crates/frame/src/minecraft_recipe_book.rs:3` (`MC_RECIPE_TABS = 5`). `known()` (`recipe_book.rs:101`) walks only `crafting_recipes()`.
- Placing is crafting only: `crates/minecraft_survival/src/recipe_book.rs:122` `place` calls `Inventory::place_known_crafting_recipe` (vendored `player/src/inventory.rs:234`), and `menus.rs:582` `book_grid` accepts only the inventory and the table (used at `:623` and `:999`).
- The six oven flags are saved (`recipe_book.rs:8` `SETTINGS`), but only `OPEN` and `FILTERING` (`:18-19`) are used. `RecipeBookView` (`messages.rs:342`) and `ToServer::RecipeBookSettings` (`messages.rs:66`, codec `:219`, `:368`, host `render_anim/src/minecraft_host.rs:199`) carry one pair.
- What already exists:
  - The smelting recipes, their categories and unlocks: vendored `player/src/crafting.rs:707` `cooking_recipes` (id, category, group, result), `:733` `cooking_display_ingredient`, `:285` `has_recipe`.
  - The fuel table (`:567-580`) and the oven screen (`crates/frame/src/minecraft_ui.rs:77` `McScreen::Furnace(McOven)`, slots `hud/src/minecraft_screens.rs:191-193`: input `Container(0)`, fuel `Container(1)`, `Output`).
  - The oven's cells in `Containers` (`containers.rs:905` `oven`).

**Vanilla 26.3** (javap `FurnaceScreen`, `BlastFurnaceScreen`, `SmokerScreen` `<clinit>`; `AbstractFurnaceScreen.getRecipeBookButtonPosition`; `FurnaceRecipeBookComponent`; `SmeltingRecipe`, `BlastingRecipe` and `SmokingRecipe` `recipeBookCategory`; `AbstractFurnaceMenu.handlePlacement`; `ServerPlaceRecipe.placeRecipe`)
- **Tabs, with their icons:**
  - Furnace: search (compass), `FURNACE_FOOD` (porkchop), `FURNACE_BLOCKS` (stone), `FURNACE_MISC` (lava bucket and emerald).
  - Blast furnace: search, `BLAST_FURNACE_BLOCKS` (redstone ore), `BLAST_FURNACE_MISC` (iron shovel and golden leggings).
  - Smoker: search, `SMOKER_FOOD` (porkchop).
- **Categories:**
  - Smelting: food to food, blocks to blocks, misc to misc.
  - Blasting: blocks to blocks, everything else to misc.
  - Smoking: always food.
  - Each search tab covers its own book's categories.
- **The button** is at (leftPos + 20, height/2 - 49). The book moves the screen like the crafting book does.
- **The filter button** uses `recipe_book/furnace_filter_enabled`, `_disabled` and their `_highlighted` variants. Its tooltips are `gui.recipebook.toggleRecipes.smeltable`, `.blastable` and `.smokable`, against "all".
- **The ghost** (`fillGhostRecipe`): the result in the result slot, the ingredient (cycling its choices) in slot 0, and, only when the fuel slot is empty, the fuels (`FurnaceRecipeDisplay.fuel`, cycling) in slot 1. `isCraftingSlot` covers slots 0 to 2.
- **Placing** (`AbstractFurnaceMenu.handlePlacement`, then `ServerPlaceRecipe.placeRecipe` on a 1x1 grid): the rules of the crafting placement.
  - A different input goes back to the inventory first.
  - The amount is the biggest stack the player has with Shift; otherwise it is one more than now when the recipe already matches, else 1.
  - A missing ingredient leaves a ghost.
- **Flags:** the open and filtering flags are per `RecipeBookType`: crafting, furnace, blast furnace and smoker (`isFurnaceGuiOpen`, `isFurnaceFilteringCraftable` and so on).

**Plan**
1. **"Minecraft furnaces, blast furnaces and smokers have their recipe book tabs"**
   - `minecraft_terrain/src/recipe_book.rs`:
     - `Tab` gains `FurnaceFood`, `FurnaceBlocks`, `FurnaceMisc`, `BlastBlocks`, `BlastMisc` and `SmokerFood` after `Redstone`, with vanilla's icons.
     - `pub enum Book { Crafting, Furnace, BlastFurnace, Smoker }` with `tabs() -> &'static [Tab]` (at most 5, so `MC_RECIPE_TABS` stays) and `searched()`. `Tab::ALL`, `at` and `in_tab` take the book.
     - `Tab::of_cooking(kind, category)`.
     - `known_cooking(book, kind, unlocked) -> Known`: one-cell entries whose `table` and `small` are the ingredient's item choices.
   - Vendored `player/src/crafting.rs`: `cooking_candidates(kind, id) -> Option<Vec<String>>` next to `cooking_display_ingredient` (`:733`), expanding tags as `crafting_grid_candidates` does. Count it as a patched file in the log.
2. **"Minecraft furnace screens show the recipe book with its ghost and fuel"**
   - hud:
     - `has_book` and `button` take `McScreen::Furnace(_)` at `[origin[0] + 20, gui[1] / 2 - 49]`, and `origin`, `covered` and `panel` follow.
     - The filter sprites and tooltip follow the oven.
     - The tab icons come from `book.tab_icons` as now.
     - `clears_ghost` takes the screen and covers `Container(0..=2)` and `Output` on ovens.
     - `draw_ghost` draws the oven result like the inventory result.
   - render_anim:
     - `grid_of` becomes `book_of(screen) -> Option<(Book, ..)>`.
     - `RecipeBookUi::build` counts the inventory plus the oven's input. Its ghost is `Container(0)` with the ingredient choices, `Container(1)` with every `is_fuel` item in item order when the fuel slot is empty, and `Output` with the result.
   - Flags per book:
     - `minecraft_survival/src/recipe_book.rs`: `crafting()` and `set_crafting()` become `flags(book)` and `set_flags(book, ..)` on `settings[2k]` and `settings[2k + 1]`.
     - `follow_screen` and `follow_friend` take the book of the open screen and reload the pair when the book type changes.
     - `RecipeBookView.settings: [bool; 8]` replaces `open` and `filtering`.
     - `ToServer::RecipeBookSettings` gains `book: u8`, with its codec, the host handler (`minecraft_host.rs:199`) and `FriendBook`.
   - Wire change: note it under "## Protocol".
3. **"Minecraft furnace recipe books put the ingredient in like vanilla"**
   - Vendored `player/src/inventory.rs`: `place_cooking_recipe(&mut self, candidates, input: &mut Option<ItemStack>, matches, use_max_items) -> RecipePlacement` next to `place_known_crafting_recipe` (`:234`), with the same amount rules for one cell.
   - `crates/minecraft_survival/src/recipe_book.rs` `place_cooking(inventory, containers, opened, kind, recipe, all)` refuses recipes that aren't unlocked, as `place` does.
   - `menus.rs`: `book_grid` (`:582`) also accepts `Open::Block` with `Station::Oven(kind)`, so `valid` (`:623`) passes. The placement branch (`:999`) calls `place_cooking` on the oven's cells and marks them dirty, so the oven ticks and saves.
   - Friends use the same `MenuAction::PlaceRecipe`, so the click needs no wire change.

**Tests**
- `minecraft_terrain recipe_book::tests`:
  - `cooking_categories_map_to_vanilla_tabs`.
  - `an_oven_book_lists_only_its_unlocked_recipes`.
  - `the_furnace_search_tab_covers_food_blocks_and_misc`.
- `render_anim`:
  - `an_open_furnace_book_builds_its_tabs_and_buttons`.
  - `the_furnace_ghost_has_ingredient_fuel_and_result` (no fuel ghost when the fuel slot is full).
  - `each_book_type_keeps_its_own_flags`.
- `hud`:
  - `the_furnace_book_button_sits_at_left_plus_20`.
  - `the_oven_filter_uses_the_furnace_sprites_and_tooltips`.
- Vendored `inventory`: `cooking_placement_moves_one_then_one_more_and_shift_a_stack`.
- Survival `recipe_book::tests`:
  - `a_different_input_goes_back_first`.
  - `a_missing_ingredient_leaves_a_ghost`.
  - `the_eight_flags_save_and_load_under_vanilla_names`.
- `minecraft_net codec`: round trips for the new `RecipeBookView.settings` and `RecipeBookSettings.book`.

**Owner's steps** (survival; place the furnace, smoker and blast furnace in a row)
1. Open the furnace. A green recipe book button sits at the left of the furnace window. Click it: the book opens beside the furnace with four tabs: compass (all), porkchop (food), stone (blocks), lava bucket and emerald (misc). **scr18_furnace_book**
2. Hover the filter button: "Showing Smeltable" when on, "Showing All" when off.
3. Click the iron ingot recipe: one raw iron moves into the input slot. Click it again: a second one goes in. Hold Shift and click: all of your raw iron (32) goes in. Put coal in the fuel slot: it smelts.
4. Take everything out of the furnace. Click the glass recipe: the sand goes in.
   - Then click a recipe you have no ingredient for. Turn the filter off ("Showing All") to see it: the smooth stone recipe shows once you have smelted stone; until then use the charcoal recipe with the logs moved out of your inventory into a chest.
   - The input slot shows a red-tinted ingredient, the empty fuel slot cycles through fuels (coal, logs, planks and others), and the result slot shows the result. **scr19_furnace_ghost**
5. Open the smoker: the book has two tabs, compass and porkchop, and lists only food (beef, potato, kelp). Open the blast furnace: compass, redstone ore (blocks) and iron shovel with golden leggings (misc), with only ores and metal items.
6. Open the book in the furnace and close it in the smoker. Reopen each: the furnace's book is open, the smoker's is closed. The crafting book in the inventory keeps its own state. Save and quit, then load: all three remember.
7. Press Enter after clicking a furnace recipe: it goes in again (from `book-keys`). Newly unlocked smelting recipes bounce (from `recipe-bounce`): pick up the `raw_gold` stack again after dropping it with Q if needed.
8. Friend window: the friend does steps 1, 3 and 6 on its own furnace. Its flags are its own, and the host's furnace screens are unchanged. **scr20_furnace_book_friend**

```
IW4L_MINECRAFT_KIT=minecraft:furnace,minecraft:smoker,minecraft:blast_furnace,minecraft:raw_iron*32,minecraft:raw_gold*16,minecraft:beef*32,minecraft:potato*32,minecraft:sand*64,minecraft:cobblestone*64,minecraft:oak_log*32,minecraft:coal*32,minecraft:kelp*16,minecraft:chest
```

---

### Left out (write in the log and in "State doc changes")

- **screens-4, effect times under `/tick rate`.** The fork runs the player's timers, effects included, at 20 a second at any tick rate (`docs/MINECRAFT-SURVIVAL.md:402`). So `minecraft_effects.rs:22` and the bottle line in `render_anim/src/minecraft_inventory.rs:929-943` dividing by 20 show the real time left. Vanilla's divide by `TickRateManager.tickrate()` only matches a world whose player effects follow `/tick`.
- **The book on a gamepad** (part of screens-2). Vanilla Java has no gamepad book to copy, and the fork's pad pointer is its own design.
- **F3 extras:** the F3 key combos (hitboxes, chunk borders, advanced tooltips, copy location and others), the debug options screen (F3+F6), the profiler and charts, the server-side lines and `reduced_debug_info`.
- **Selectors beyond `/kill`:**
  - `@e` in `/tp`, `/effect`, `/tag`, `/item` and `/execute as`.
  - Killing projectiles (arrows, thrown items, tridents, fireballs, potions), the ender dragon and end crystals.
- **`/item` beyond `replace ... with`:** `item modify` (loot item functions), `from` copies, `fill` and `override`, mob, horse and villager slots, and dispensers, hoppers and other block entities outside `Containers`.

## E3-14. Saving, maps, trial chambers, vaults and compasses

This section covers the known limits of `docs/round10/release-2-tests.md` (Trial spawners, Vaults, Lodestones and compasses) and the maps the E brief left for later (`docs/round10/track-e.md:1592-1598`). Code was checked on `minecraft-survival` at 9c6efebd. Map code was checked on `track-e2` at dc9d6541 (segment e2b, not merged yet). Vanilla facts come from javap on the 26.3 `client.jar` and from the 26.3 data in `datapacks/local/minecraft-26.3` under `MINECRAFTOSS_ROOT`.

### Shared points for this area
- **Order and waits.** The groups run in the order below.
  - `trial-arrows` and `ominous-shots` edit `ServerSim::drop_ominous_items` (`crates/minecraft_terrain/src/server.rs:3794-3829`). W's w3 adds the ominous item spawner's game events there, so these groups start after w3 is merged.
  - `ominous-shots` and `thrown-saved` edit `tick_thrown` (`server.rs:2572-2747`), which D0's d0b also edited (endermites from pearls; merged in c0366f3a, so the lines have moved).
  - `ominous-shots` builds on E3-11's `mob-shots-blame` (e3m: `ownerless_projectile_hurt`, `thrown::hurt_unowned`, the burst arm for a charge whose breeze is gone) and E3-9's `registered-hits` (e3j: the family arm of `hurt_by_typed_blast` passes `kind`). See E3-0, "Ownerless projectile hits".
  - `map-banners` and `map-frames` build on e2b's map code (`MapStore::tick_carried_on`, `Held`, `MapMark`, the held map picture). Their anchors are `track-e2` lines; re-check them after e2b merges.
- **Test aid.** Several owner steps summon an ominous item spawner: `/summon minecraft:ominous_item_spawner <pos> {spawn_item_after_ticks:<n>L,item:{...}}`.
  - The entity catalog lists the type, and `server_mobs.rs:2135` loads it from a tag, so `ServerSim::summon` (`server.rs:1968`) should already make one.
  - `trial-arrows` pins this with a server test. If `/summon` refuses the entity, that group starts with its own commit, "Minecraft /summon makes ominous item spawners".
- **Protocol.** `map-banners` 2 (`MapMark.name`) and `map-frames` 1 (`FrameView.map`) change messages that friends decode.
  - Each adds a line under "## Protocol" in `docs/round10/track-e3-log.md`.
  - Never bump `minecraft_net::PROTOCOL`.
- **Hooks other tracks own.** No group touches any of these:
  - `PlayerControls`: M1 owns `PlayerControls.sprinting`.
  - `EffectKind`: M2's m2i adds `DolphinsGrace` and grows `ALL`.
  - M2's player-effect queue.
  - M1's loot-context rewrite (`loot.rs` `EntityLootContext`).
- **Vendored files.** The log counts the patched vendored files: `minecraftoss-entities` `world.rs`, `world/combat.rs` and `skeleton.rs`. No new vendored files.
- **Builds.** `check -p` and `test -p` on `minecraftoss-entities`, `minecraft_terrain` and `minecraft_survival`, plus `minecraft_net`, `hud` and `render_anim` for the map groups. Run `check -p launcher` before each commit that touches game code.

---

### Group `trial-arrows` (gaps world-1, world-3; after W's w3)

**Cause**
- **world-1: the effect level is dropped.**
  - `trial_spawner::arrow_effect` (`crates/minecraft_terrain/src/trial_spawner.rs:1159-1172`) keeps `(EffectKind, u32)` and drops `PotionEffect.amplifier` (`potions.rs:156-162`).
  - Every carrier lacks the amplifier: `ArrowEntity.effect` (`third_party/minecraftoss/entities/src/world.rs:376`), `PlayerHitKind::Arrow` (`:619`), `spawn_effect_arrow` (`:3357-3363`) and `SkeletonKind::arrow_effect` (`skeleton.rs:69-76`).
  - `mob_hit` sets amplifier 0 for an arrow (`crates/minecraft_survival/src/entities.rs:2108`).
  - The player side already takes an amplifier: `Hit` (`entities.rs:40`), `Survival::struck` (`survival.rs:912-921`) and `Typed.effect` (`world.rs:631`).
  - Result: the ominous table's strong slowness arrow slows at level I. In 26.3 that arrow is a plain `minecraft:arrow` with `set_potion strong_slowness`: Slowness IV, 400 ticks, duration scale 1.0.
- **world-3: no arrow pickup.**
  - `ArrowEntity` (`world.rs:371-378`) has no stack and no pickup mode.
  - The ominous arrow is spawned ownerless (`server.rs:3810-3814`). It sticks in the ground and vanishes at the end of its life.
  - Only tridents can be picked up: `touched_by` and `taken_by` (`trident.rs:603-622`), `pickup_tridents` (`server.rs:2461-2482`), called from the pickup loop (`server.rs:5201-5208`).

**Vanilla 26.3**
- `Arrow.doPostHurtEffects` runs `getPotionContents().forEachEffect(..., getPotionDurationScale())`. Each `MobEffectInstance` keeps its amplifier and goes to `LivingEntity.addEffect(instance, effectSource)`.
- `ArrowItem.asProjectile` (javap) builds `new Arrow(level, x, y, z, stack.copyWithCount(1), null)` and sets `pickup = AbstractArrow$Pickup.ALLOWED`. A mob's arrow stays DISALLOWED.
- `AbstractArrow.playerTouch` and `tryPickup` (javap):
  - The arrow is taken on the server when `(isInGround() || isNoPhysics()) && shakeTime <= 0`.
  - `ALLOWED` adds `getPickupItem()` (the stack with its components). `CREATIVE_ONLY` needs `hasInfiniteMaterials`.
  - Then `player.take(this, 1)` and the arrow is discarded.
  - The touch box is the player's box inflated by (1.0, 0.5, 1.0), as `trident.rs:603-607` already does.

**Plan**
1. "Minecraft arrows from ominous trial spawners keep their effect's level"
   - `world.rs`: `ArrowEntity.effect`, `PlayerHitKind::Arrow.effect` and the parameter of `spawn_effect_arrow` become `Option<(EffectKind, u32, u8)>`, the same shape as `Typed.effect`.
   - `skeleton.rs:69-76`: strays, bogged and parched return amplifier 0.
   - `trial_spawner.rs:1159-1172`: return the amplifier as a `u8`, clamped to 0-255.
   - `minecraft_survival/src/entities.rs:2108`: `PlayerHitKind::Arrow { effect, .. } => effect`.
   - Only the first effect still lands (world-2 is in Left).
2. "Minecraft arrows from ominous trial spawners can be picked up"
   - `world.rs`:
     - `ArrowEntity.pickup: Option<ItemStack>` (`minecraftoss_player::inventory::ItemStack`). `spawn_arrow` (`:3335-3349`) sets `None`, so every mob arrow keeps vanilla's DISALLOWED.
     - `spawn_dispensed_arrow(arrow, effect, stack)` next to `spawn_effect_arrow`.
     - `EntityWorld::pickup_arrows(bounds, take) -> Vec<(u64, DVec3, ItemStack)>`. It takes an arrow that is alive, in the ground (`Arrow.in_ground`) and has `shake_time <= 0`, whose box meets the inflated player box, and that `take` accepts. Remove it from `arrows` and `order`.
   - `server.rs`:
     - `:3813`: the ominous arrow uses `spawn_dispensed_arrow` with the item's stack at count 1, components kept.
     - `ServerSim::pickup_arrows` next to `pickup_tridents`, called once after it in the pickup loop (`:5201-5208`) with the same `take` and stack-size clamp.
     - It returns `PickedStack`s. Arrows have no i32 view id, so check how the client animates `out.picked` (`minecraft_survival/src/entities.rs:1725-1750`). If no view matches, the arrow pops with the pickup sound and no flying item; log that.
   - Friends: a friend's pickup goes through the same loop. The mirrored arrow vanishes with the entity world copy.

**Tests**
- `minecraftoss-entities`:
  - `world::tests::an_arrow_without_an_owner_hits_a_player_with_no_source` (`:8838`) changes to `(EffectKind::Poison, 900, 0)`.
  - New `world::tests::a_dispensed_arrow_in_the_ground_is_picked_up_and_a_skeleton_s_is_not`: the stack comes back with its components; an arrow that is shaking or still flying is not taken; a refusing `take` leaves the arrow in place.
- `minecraft_terrain`:
  - `trial_spawner::tests::ominous_items_fly_with_vanillas_dispense_power_and_carry_their_potion` (`:2538`): strong slowness gives `(Slowness, 50, 3)` on a tipped arrow and `(Slowness, 400, 3)` on a plain arrow.
  - New `trial_spawner::tests::a_custom_arrow_effect_keeps_its_amplifier`: a `custom_effects` weakness with amplifier 2.
  - New `server::tests::summon_makes_an_ominous_item_spawner_that_drops_its_item`: through `ServerSim::summon` with `item` and `spawn_item_after_ticks`.
  - New `server::tests::a_player_picks_up_the_arrow_an_ominous_spawner_shot`: queued on `level.ominous_drops`, ticked until it sticks, then picked up with its `potion_contents`.
- `minecraft_survival`:
  - The arrow test at `entities.rs:2926` takes amplifier 0.
  - New `entities::tests::an_arrow_s_effect_reaches_the_player_with_its_level`: `mob_hit` gives `(Slowness, 400, 3)`.

**Owner's steps** (survival, Normal, by day, flat open ground)
1. Run `/summon minecraft:ominous_item_spawner ~ ~3 ~ {spawn_item_after_ticks:60L,item:{id:"minecraft:arrow",count:1,components:{"minecraft:potion_contents":{potion:"minecraft:strong_slowness"}}}}` and stand still.
   - After 3 s a small arrow grows above you and shoots down into you.
   - Open the inventory: the effect list shows Slowness IV at about 0:20, and walking forward you barely move. **e3w01_slowness_iv**
2. Drink milk. Run step 1 again with `{potion:"minecraft:poison"}`: Poison (level I) for about 0:45, as before.
3. Drink milk. Run step 1 again with `components:{"minecraft:potion_contents":{custom_effects:[{id:"minecraft:weakness",amplifier:2b,duration:200}]}}`: Weakness III for 0:10.
4. Run `/summon minecraft:ominous_item_spawner ~3 ~3 ~ {spawn_item_after_ticks:60L,item:{id:"minecraft:arrow",count:1}}` and stay where you are.
   - The arrow sticks in the ground 3 blocks away.
   - Walk onto it: it goes into your inventory with the pickup pop. **e3w02_arrow_picked**
5. Do step 4 again with the poison arrow from step 2. When you pick it up it takes its own slot and does not join the plain arrow, because it keeps its potion.
6. Use a skeleton egg 15 blocks away and let it shoot until a few arrows stick in the ground near you. Kill it and walk over its arrows: they are not picked up.
7. Friend: the host runs step 4 standing next to the friend. The friend walks onto the arrow and gets it. **e3w03_friend_arrow**

```
IW4L_MINECRAFT_KIT=minecraft:skeleton_spawn_egg*2,minecraft:diamond_sword+sharpness=5,minecraft:shield,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:milk_bucket,minecraft:milk_bucket,minecraft:milk_bucket,minecraft:golden_apple*8,minecraft:cooked_beef*32,minecraft:cobblestone*64,minecraft:torch*32
```

---

### Group `ominous-shots` (gaps world-4, world-5; after W's w3 and D0's d0b)

**Cause**
- **world-4: projectiles with no thrower hurt nothing.**
  - In `tick_thrown`, a hit by a thing with no thrower is dropped: `None if t.owner == crate::thrown::MOB_THROWER => {}` (`server.rs:2638`).
  - A wind charge's burst hurts a mob only when `owner != MOB_THROWER` (`:2726`).
  - `thrown::hurt` and `hurt_by` (`thrown.rs:584-610`) go through `player_projectile_hurt` (`world/combat.rs:340-355`), which needs a player. mob-shots-blame (e3m) adds `ownerless_projectile_hurt` and `thrown::hurt_unowned` beside them, but only its wind charge burst arm uses them.
  - `mob_hits_mob` (`world.rs:4261`) returns None without an attacker type.
  - `Thrown::dispensed` (`thrown.rs:421-444`) and `Thrown::resumed` (`:446-473`) set the owner to `MOB_THROWER`.
  - A trident whose thrower left has `UNKNOWN_OWNER` (`trident.rs:36`, the same `u64::MAX`). It goes through `hurt_by` with that id as a player (`server.rs:2442`), so it blames a player `u64::MAX`.
- **world-5: tridents and rockets drop as items.**
  - `dispense_config` (`trial_spawner.rs:1121-1135`) has no trident or firework rocket, so `drop_ominous_items` drops both as items (`server.rs:3797-3802`).
  - `Rocket::shot` (`firework_rocket.rs:112`; `DISPENSE_POWER` 0.5 and `DISPENSE_UNCERTAINTY` 1.0 at `:23-24`) already serves dispensers (`server.rs:1820-1829`).
  - Tridents have only `ThrownTrident::launch` (`trident.rs:250`, for a player) and `thrown_by_mob` (`:309-327`, `Pickup::Disallowed`).

**Vanilla 26.3**
- Hits:
  - `Snowball.onHitEntity`: `hurt(damageSources().thrown(this, owner), entity instanceof Blaze ? 3 : 0)`.
  - `ThrownEgg.onHitEntity` hurts for 0.
  - `AbstractWindCharge.onHitEntity`: `hurtServer(level, windCharge(this, owner as LivingEntity or null), 1.0)`.
  - `ThrownTrident.onHitEntity`: 8 damage from `trident(this, owner or this)`.
  - With no owner, `DamageSource.getEntity()` is null: no `setLastHurtByMob` and no player credit. The hurt flash and the 0.4 knockback along the projectile's motion still happen.
- Ominous drops:
  - `OminousItemSpawner.spawnItem` sends a `ProjectileItem` through `spawnProjectile`, then `ProjectileItem.asProjectile` and `Projectile.spawnProjectileUsingShoot`, downwards at the item's `DispenseConfig`.
  - `TridentItem.asProjectile` builds `ThrownTrident(level, x, y, z, stack.copyWithCount(1))` with `pickup = ALLOWED` and the default config (power 1.1, uncertainty 6.0).
  - `FireworkRocketItem.asProjectile` builds `FireworkRocketEntity(level, stack, x, y, z, true)`. Its `createDispenseConfig` is power 0.5, uncertainty 1.0.
  - A loyal trident with no owner never returns: `ThrownTrident.tick` needs an owner.

**Plan**
1. "Minecraft snowballs, eggs, wind charges and tridents without a thrower hurt what they hit"
   - **Builds on e3m and e3j; adds no second ownerless helper.** mob-shots-blame (E3-11, e3m) added `EntityWorld::ownerless_projectile_hurt(id, amount, kind, velocity)` in `world/combat.rs` (enderman dodge, `hurt_by_typed_blast(key, amount, DVec3::ZERO, Blaster::Nobody, kind)`, the 0.4 knockback on an applied full hit), `thrown::hurt_unowned` next to `hurt_by`, and the server arm `(Some((id, None)), None)` for a wind charge burst whose breeze is gone. registered-hits (E3-9, e3j) already made the `Family` arm of `hurt_by_typed_blast` (`world.rs:3078`) pass `kind`: do not edit that line again.
   - `world.rs`:
     - `hurt_by_typed_blast` (`:3037-3083`) takes `source: DamageSourceKind` instead of fixing `Explosion` at `:3039`.
     - `hurt_by_blast` (`:3034`) and `firework_blast` (`:3115`) pass `DamageSourceKind::Explosion`.
   - `world/combat.rs` `ownerless_projectile_hurt`:
     - after its enderman dodge, the closed shulker refusing a trident, as in `player_projectile_hurt` (`:347`);
     - it passes `DamageSourceKind::Projectile`, as `player_projectile_hurt` does (both make animals panic, `DamageSourceKind::panics`);
     - still no blame, no credit, no `note_hoglin_hurt`. The child module can see the private `Blaster`.
   - `thrown.rs` `hurt_by` (`:599-610`): when `player == MOB_THROWER` (the same `u64::MAX` as `trident::UNKNOWN_OWNER`), call `hurt_unowned`; otherwise unchanged.
   - `server.rs`:
     - `tick_thrown`: remove the `None if t.owner == crate::thrown::MOB_THROWER => {}` arm (`:2696` at 80aabd8e), so an ownerless hit reaches `hurt_by` and so `hurt_unowned`.
     - The wind charge bursts: fold the guarded arm `(Some((id, None)), None) if owner != MOB_THROWER` (`:2796` at 80aabd8e) and e3m's unguarded arm into one `(Some((id, None)), None)` arm that calls `hurt_by`, which now routes itself. No duplicate arm.
     - The ownerless trident (`:2442`) takes the same path through `hurt_by`.
   - M2's family knockback hook (E3-0): once m2e has merged, the knockback of `ownerless_projectile_hurt` on a family goes through it with `Knocker` None; keep both sides at a merge.
2. "Minecraft ominous trial spawners shoot tridents and firework rockets"
   - `trial_spawner.rs:1121-1135`: `"minecraft:trident"` joins the `(1.1, 6.0)` arm. `"minecraft:firework_rocket"` gets `(DISPENSE_POWER, DISPENSE_UNCERTAINTY)` from `firework_rocket.rs`, next to the charges.
   - `trident.rs`: `ThrownTrident::dispensed(stack, position, velocity)` next to `thrown_by_mob`.
     - Owner `UNKNOWN_OWNER`, no `owner_uuid`, no `mob_owner`, `Pickup::Allowed`.
     - Loyalty and foil come from the stack; the count is 1.
     - `taken_by` then lets anyone take it, since no owner is present.
   - `server.rs` `drop_ominous_items`, next to the arrow arm:
     - A `"minecraft:trident"` arm: velocity from the same `shoot`, then `throw_trident`.
     - A `"minecraft:firework_rocket"` arm: `Rocket::shot(RocketOwner::Nobody, from, NEG_Y, power, uncertainty, crate::firework::fireworks(&stack), &mut random)`, with the stack built as `shoot_dispensed_rockets` builds it (`:1824-1826`).
     - Keep w3's game event lines where they are.

**Tests**
- `minecraftoss-entities`, `world::combat::tests`:
  - `an_ownerless_snowball_pushes_a_cow_and_blames_nobody`: health unchanged, pushed along the motion, no attacker remembered, no kill credit.
  - `an_ownerless_trident_is_refused_by_a_closed_shulker` (e3m's test already covers the wind charge and the enderman).
  - `an_ownerless_snowball_hurts_a_blaze_by_three`.
  - `an_ownerless_hit_is_a_projectile_hit`: the cow panics, as from a player's snowball.
- `minecraft_terrain`:
  - `server::tests::a_snowball_from_an_ominous_spawner_hurts_a_blaze_by_three`: a `Thrown::dispensed` through `tick_thrown`.
  - `server::tests::a_dispensed_wind_charge_hurts_the_mob_it_hits` (`Thrown::dispensed`, a direct hit through `tick_thrown`); e3m's `a_breeze_charge_still_hurts_the_mob_it_hits_after_its_breeze_is_gone` still passes with the folded arm.
  - `server::tests::a_trident_whose_thrower_left_hurts_a_mob_without_blaming_anyone`.
  - The trial spawner dispense test adds the trident and the rocket.
  - `trident::tests::a_dispensed_trident_has_no_owner_and_anyone_may_pick_it_up`.
  - `server::tests::an_ominous_item_spawner_shoots_a_trident_and_a_rocket_down`: no item entity appears, the trident sticks and a player picks it up, and the rocket flies down and bursts.
- The existing tests of `player_projectile_hurt`, `ownerless_projectile_hurt`, fireworks and creeper blasts pass unchanged.

**Owner's steps** (survival, Normal, by day, flat ground)

Setup: build a glass pen one block wide and 3 high, with one side open. Before step 3, run `/effect give @s minecraft:fire_resistance 600`.

1. Stand in the pen and run `/summon minecraft:ominous_item_spawner ~ ~5 ~ {spawn_item_after_ticks:200L,item:{id:"minecraft:snowball",count:1}}`. Step out, use a cow egg on the pen floor and close the pen with glass.
   - After 10 s the snowball breaks on the cow with white crumbs. The cow flashes red and keeps its health. Before this change, nothing happened to it. **e3w04_cow_snowball**
2. Do step 1 again with `item:{id:"minecraft:wind_charge",count:1}` over a new cow: the charge bursts on it, and the cow flashes red and loses half a heart.
3. Kill the cow and put a blaze in the pen (blaze egg). Summon three spawners over it, each with a snowball, with `spawn_item_after_ticks` of 200, 240 and 280: every snowball flashes the blaze red and takes 1.5 hearts. **e3w05_blaze_snowballs**
4. Away from the pen, run `/summon minecraft:ominous_item_spawner ~3 ~6 ~ {spawn_item_after_ticks:60L,item:{id:"minecraft:trident",count:1}}`.
   - A trident shoots down and sticks in the ground. Before this change, a trident item fell. **e3w06_trident_stuck**
   - Walk onto it: it goes into your inventory.
5. Put a trident spawner over a new penned cow: the cow flashes and loses 4 hearts.
6. Run `/summon minecraft:ominous_item_spawner ~8 ~6 ~ {spawn_item_after_ticks:60L,item:{id:"minecraft:firework_rocket",count:1,components:{"minecraft:fireworks":{flight_duration:1b,explosions:[{shape:"large_ball",colors:[I;16711680]}]}}}}` and stay where you are: a rocket shoots down with its sparks and bursts red at the ground. **e3w07_rocket_down**
7. Friend: the friend watches steps 4 and 6 from 10 blocks away. The trident flies and sticks, the rocket flies and bursts, and the friend can pick up the trident. **e3w08_friend_trident**

```
IW4L_MINECRAFT_KIT=minecraft:cow_spawn_egg*8,minecraft:blaze_spawn_egg*2,minecraft:glass*64,minecraft:cobblestone*64,minecraft:diamond_sword+sharpness=5,minecraft:shield,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*8,minecraft:cooked_beef*32,minecraft:milk_bucket
```

---

### Group `trial-clock` (gaps world-8, world-6)

**Cause**
- **world-8: an empty level's clock falls behind.**
  - A level takes the world's game time only when it opens (`start_level`, `crates/minecraft_survival/src/server.rs:342`). After that it counts its own ticks.
  - `plan_ticks` (`:394-399`) advances the world's clock every batch, but only levels with players get the batch.
  - `close_empty_levels` (`:604-635`) closes an empty level after `EMPTY_TIME` (300 ticks) when the world has a save, and a reopened level takes the world's time.
  - A level reoccupied within 300 ticks, or any level of a world without a save (`keeps_empty_levels`), keeps its lag.
  - On return the weather is resynced (`:613-619`), but the time is not.
  - Trial spawner cooldowns and vault timers run on the level's time.
- **world-6: every player counts as standing.**
  - `item_spot` uses `PLAYER_HEIGHT` 1.8 for every player (`trial_spawner.rs:48`, `:947`).
  - `TrialPlayer` (`:592-600`) has no height. Its `eye` comes from `PlayerCandidate.eye_height` (`server.rs:3722`), which is the constant `EYE_HEIGHT` (`minecraft_survival/src/entities.rs:462`).
  - `PlayerControls.sneaking` reaches the server tick (`TickInput.controls`, `server.rs:4388`) but not `tick_trial_spawners` (`:3705`, called at `:5103`).

**Vanilla 26.3**
- One game time for every dimension: `ServerLevel.tickTime` sets the world's game time once per server tick, and the other dimensions read it through `DerivedLevelData`, players or not.
- `TrialSpawnerState.calculatePositionAbove` (javap) uses `position().relative(UP, getBbHeight() + 2 + random.nextInt(4))`, then clips.
- `PlayerDetector` (javap) checks sight to `Player.getEyePosition()`.
- `Avatar.POSES` (javap):

  | Pose | Height | Eyes |
  |---|---|---|
  | Standing | 1.8 | 1.62 |
  | Crouching | 1.5 | 1.27 |
  | Swimming, fall flying, spin attack | 0.6 | 0.4 |

**Plan**
1. "Minecraft levels take the world's game time back when a player returns"
   - `minecraft_survival/src/server.rs:612-622`: when an occupied level had `empty_ticks > 0`, call `entities.set_game_time(self.globals.game_time())` (`entities.rs:566`) for every dimension. This sits beside the weather resync, which stays limited to the dimensions with weather.
2. "Minecraft trial spawners see a crouching player's eyes and drop items above their head"
   - `trial_spawner.rs`:
     - `TrialPlayer.height` next to `eye`; `item_spot` uses it.
     - Standing (1.8, 1.62) and crouching (1.5, 1.27) constants replace `PLAYER_HEIGHT`.
     - The test helpers (`:2008`, `:2262`) stand.
   - `server.rs`:
     - `tick_trial_spawners(players, controls, omens)`: a player whose controls are sneaking gets height 1.5 and eyes at 1.27.
     - The caller (`:5103`) passes `&input.controls`; the tests (`:11527`, `:11549`) pass `&[]`.
   - MW2 prone also sends `sneaking` (`render_anim/src/minecraft_players.rs:1387-1389`, `minecraft_world.rs:1141`), so a prone player counts as crouching. Swimming and gliding are in Left.

**Tests**
- `minecraft_survival`:
  - `server::tests::a_level_a_player_returns_to_takes_the_world_s_game_time_back`, next to the weather test at `:2460`. Steve goes to the Nether, the world's clock moves 100 ticks while the Overworld does not tick, and Steve returns within `EMPTY_TIME`: the Overworld's `game_time()` equals `globals.game_time()`.
  - The same for the End in a world without a save, as in `:2441`.
- `minecraft_terrain`:
  - `trial_spawner::tests::an_item_appears_higher_above_a_standing_player_than_a_crouching_one`: the `top` handed to `spot_above` is feet + height + 2 + 0..3.
  - `server::tests::a_crouching_player_behind_a_wall_is_not_seen_by_a_trial_spawner`: a 1.5-block wall hides eyes at 1.27 but not at 1.62.

**Owner's steps** (survival, Normal, by day, flat open ground)
1. Crouch test:
   - In creative, place a block and put a trial spawner on top of it. Use a zombie egg on the spawner.
   - Walk 8 blocks straight away from it.
   - On the ground one block in front of you, between you and the spawner, place a cobblestone with a bottom cobblestone slab on top of it. Stand right behind this wall.
   - Crouch (MW2 crouch, C) and run `/gamemode survival`. The spawner stays waiting (dim, its mob turning) for 5 s.
   - Stand up: it wakes. **e3w09_crouch_hidden**
2. Win that trial. Place a second trial spawner and use a zombie egg on it. Drink an ominous bottle and walk into its sight until it turns blue. Crouch and stay still: items still appear 3 to 6 blocks above you or a zombie and fall. **e3w10_item_above_crouching**
3. Optional, the shared clock:
   - Run `/setblock ~3 ~ ~ minecraft:trial_spawner{target_cooldown_length:400,normal_config:{spawn_potentials:[{weight:1,data:{entity:{id:"minecraft:zombie"}}}],total_mobs:1f,simultaneous_mobs:1f}}` and build a nether portal 5 blocks away.
   - Win its one-zombie trial and go through the portal at once. Stay 10 s in the Nether (less than 15 s, so the Overworld stays open), then come back.
   - The spawner wakes again 20 s after its reward, counting the time in the Nether. Before this change, it took about 10 s longer.

```
IW4L_MINECRAFT_KIT=minecraft:trial_spawner*3,minecraft:zombie_spawn_egg*4,minecraft:ominous_bottle*2,minecraft:cobblestone*64,minecraft:cobblestone_slab*16,minecraft:obsidian*14,minecraft:flint_and_steel,minecraft:diamond_sword+sharpness=5,minecraft:shield,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*8,minecraft:cooked_beef*32,minecraft:torch*32
```

---

### Group `vault-looks` (gaps world-10, world-12)

**Cause**
- **world-10: the ejection count survives a load.**
  - `ServerData::read` takes `total_ejections_needed` back from the tag (`crates/minecraft_terrain/src/vault.rs:164-167`).
  - After a load, `ejection_progress` (`:200-206`) therefore continues from the saved total, and the pitch (`0.8 + 0.4 * progress`, `:452-456`) keeps rising.
  - `tick_vaults` (`:702-726`) and `insert_key` (`:728-761`) re-read the vault from its block entity tag every time, so the runtime value cannot simply be dropped from the tag.
- **world-12: stacked copies are scattered.**
  - `cluster_offsets` (`vault_display.rs:133-150`) moves each extra copy by up to `CLUSTER_SPREAD` 0.15 on x and y (0 on z for flat items). It is seeded from the item name and the count.
  - `append_vault_items` (`mesh.rs:1315-1331`) applies these offsets.
  - Dropped items do the same with their own bit hash (`mesh.rs:1042-1065`).

**Vanilla 26.3**
- **Ejection count:**
  - `VaultServerData.set` (javap) copies `stateUpdatingResumesAt`, `itemsToEject` and `rewardedPlayers`, but not `totalEjectionsNeeded`. `VaultBlockEntity.loadAdditional` applies the loaded data through `set`, so after every load the total is 0.
  - The codec still writes `total_ejections_needed`, and leaves it out when it is 0.
  - `ejectionProgress = 1 - Mth.inverseLerp(items left, 1, total)`. With a total of 0 this equals the number of items left.
  - So after a load the pitch is 0.8 + 0.4 × items left (the sound engine caps it at 2.0) and falls with each item.
- **Stacked copies:** `ItemEntityRenderer.renderMultipleFromCount` (javap), which `VaultRenderer` also uses:
  - The random is seeded with `ItemClusterRenderState.getSeedForItemStack`: `Item.getId(item) + damage`, or 187 for an empty stack.
  - A model is flat when `getZsize() <= 0.0625`, where the box is `ItemStackRenderState.getModelBoundingBox`: the layers' extents through their display transform.
  - **Flat models:**
    1. Move z by `-(1.5 × zsize × (n - 1)) / 2` and draw copy 0.
    2. For each further copy, step z by `1.5 × zsize`, then draw the copy with x and y offsets of `(nextFloat × 2 - 1) × 0.15 × 0.5` and no z offset.
  - **Other models:** every copy after the first gets `(nextFloat × 2 - 1) × 0.15` on x, y and z.
  - `getRenderedAmount`: 1 copy for 1 item, 2 up to 16, 3 up to 32, 4 up to 48, else 5.

**Plan**
1. "Minecraft vaults forget the ejection count on load like vanilla"
   - `vault.rs`: `Vaults` (`:529-533`) gains `ejections: HashMap<(i32, i32, i32), i32>` beside `fail_times`.
   - `tick_vaults` and `insert_key` set `vault.server.total_ejections_needed` from this map right after `Vault::read` and before `before` is cloned; a missing entry counts as 0. After the call they store the new total back, removing the entry at 0.
   - `tick_vaults` drops the entries of vaults it no longer lists, so a vault whose chunk reloads starts from 0, like vanilla's reloaded block entity.
   - `ServerData::read` and `to_tag` keep the tag field.
2. "Minecraft stacked items in vaults and on the ground are drawn like vanilla"
   - `vault_display.rs`: `cluster_offsets(seed, count, depth: Option<f32>)` with vanilla's steps; `Some(zsize)` marks a flat model.
     - The seed is the item's raw registry id plus its damage when the mesh builder can reach the item registry.
     - Otherwise it is a stable hash of the id without the count. Log which one is used.
   - `mesh.rs`:
     - `ItemVisual` (`:990-1001`) adds `depth`: `max.z - min.z` after the ground transform.
     - `append_vault_items` (`:1322`) and the dropped item copies (`:1042-1065`) both use `cluster_offsets`, applied after the spin and before the ground transform, as now.

**Tests**
- `vault::tests::after_a_load_the_eject_pitch_starts_high_and_falls`: 3 items left, a total of 3 in the tag and a fresh `Vaults` give 2.0, 1.6, 1.2.
- `vault::tests::without_a_load_the_eject_pitch_rises`: 0.8, 1.0, 1.2.
- The tests at `:956` and `:998` still pass.
- `vault_display::tests::flat_copies_stack_a_layer_and_a_half_apart`: 3 copies with z size 0.03125 sit at z −0.046875, 0 and 0.046875; copy 0 at x = y = 0; |x| and |y| at most 0.075.
- `vault_display::tests::block_copies_scatter_on_three_axes`.
- `vault_display::tests::the_same_item_gives_the_same_cluster`.

**Owner's steps** (survival, by day)
1. Place a vault and stand 3 blocks from it. Watch the item inside: a stack of emeralds, arrows or wind charges shows its copies one behind the other, a little apart and slightly shifted sideways, like a fanned deck. A block item shows as a loose clump. **e3w11_vault_stack**
2. Drop the 16 emeralds, the 32 arrows and the 64 cobblestone as whole stacks (Ctrl+Q). Emeralds and arrows lie in neat layers; cobblestone lies in a loose clump. **e3w12_dropped_stacks**
3. Friend: the friend sees the same shapes. **e3w13_friend_stacks**
4. Listen:
   - Place a second vault and open it with a trial key. As soon as the first item pops out, press Esc and choose Save and Quit to Title.
   - Load the world. The remaining items pop out highest pitch first, each one lower than the one before (vanilla does this after a load), and the shutter closes after the last.
   - Open a third vault without saving: the pitches rise as before.

```
IW4L_MINECRAFT_KIT=minecraft:vault*3,minecraft:trial_key*6,minecraft:ominous_trial_key*3,minecraft:emerald*16,minecraft:arrow*32,minecraft:cobblestone*64,minecraft:diamond_pickaxe+efficiency=5,minecraft:cooked_beef*32
```

---

### Group `lodestones-unloaded` (gap world-13)

**Cause**
- `lodestones_gone` (`server.rs:3783-3792`) counts a target as gone only when `self.level.chunk(at.chunk()).is_some()` and the block there differs. So a lodestone in a chunk the level does not hold always counts as present, even at a place that never had one.
- The fork's points of interest live only in memory (`third_party/minecraftoss/entities/src/poi.rs`):
  - `load_pois` (`server.rs:936-949`) fills them from a loading chunk.
  - `unload_chunk` drops them (`:2768`).
  - The `poi` folder is created but never written (`world/src/level_storage.rs:18`, `minecraft_survival/src/save.rs:504`).
- `ServerSim.storage` (`server.rs:54`) can read a saved chunk with `ChunkStorage::load` (`third_party/minecraftoss/world/src/storage.rs:79-90`), which also sees chunks that are saved but not yet flushed.

**Vanilla 26.3**
- `LodestoneTracker.tick(ServerLevel)`:
  - A target in another dimension stays.
  - `!level.isInWorldBounds(pos)` or `!level.getPoiManager().existsAtPosition(PoiTypes.LODESTONE, pos)` removes the target. `tracked` stays true, so the needle spins.
- `PoiManager.existsAtPosition` loads the POI section from the `poi` region files when it is not in memory. A place that never had a lodestone has no record.

**Plan.** One commit: "Minecraft lodestone compasses check lodestones in unloaded chunks".
- `server.rs`:
  - New `ServerSim.saved_lodestones: HashMap<ChunkPos, Vec<BlockPos>>`.
  - A helper `lodestones_in(chunk)` scans a chunk's sections for the lodestone state with the POI table, as `load_pois` does.
  - `unload_chunk` (`:2761`) records the chunk's lodestones before `remove_chunk`, even an empty list, so the answer never waits on the chunk map's save.
  - `load_chunk` (`:912`) removes the chunk's entry.
- `lodestones_gone` becomes `&mut self` (caller `:5098`):
  - In a loaded chunk it checks the block as now.
  - In an unloaded chunk it asks `saved_lodestones`.
  - For a chunk with no entry it reads the chunk once through `storage.load` and caches the list. No saved chunk, or no storage, means no lodestone, as vanilla has no record there.
- The out-of-bounds check stays first. Porting vanilla's `poi` files is not needed.

**Tests**
- `server::tests::a_broken_lodestone_is_reported_only_when_its_chunk_is_loaded` (`:12143`): the `far` target in a never-loaded chunk with no storage is now gone. Rename the test `a_lodestone_target_is_gone_where_no_lodestone_is_loaded_or_saved`.
- New `server::tests::a_lodestone_in_an_unloaded_chunk_counts_as_its_chunk_left_it`: placed then unloaded, it is kept; loaded, broken and unloaded, it is gone.
- New `server::tests::a_lodestone_in_a_saved_chunk_is_read_once_from_the_region_file`: with a temporary `ChunkStorage`.
- `minecraft_survival` `entities::tests::each_players_compass_forgets_a_lodestone_that_is_not_there_in_a_loaded_chunk` (`:3405`) still passes.

**Owner's steps** (survival, by day; the fork's `/give` takes no `minecraft:lodestone_tracker` component, `minecraft_commands/src/args.rs:391-416`, so a compass aimed at a place that never had a lodestone is covered by the tests, and step 1 makes the unloaded-chunk case by hand)
1. A lodestone broken while its compass is far away.
   - `/gamemode creative`. Place a lodestone and lock a compass on it.
   - `/tp @s ~1500 ~40 ~`, fly down, place a chest on the ground and put the compass in it (a compass in a chest is not checked, as in vanilla).
   - `/tp @s ~-1500 ~ ~` back to the lodestone and break it.
   - `/tp @s ~1500 ~ ~` back to the chest (the lodestone's chunk is unloaded and saved without it) and take the compass: within a second its needle spins at random. Its name stays "Lodestone Compass". Before this change it kept pointing back toward the broken lodestone. **e3w14_compass_spins**
   - `/gamemode survival`.
2. Place a lodestone and lock a compass on it. Run `/gamemode creative` and `/tp @s ~1500 ~40 ~`, and hover there: the locked compass still points back toward the lodestone. **e3w15_compass_far**
3. Save and Quit there, then load. The lodestone's chunk has not been loaded since, and the compass still points back.
4. Run `/tp @s ~-1500 ~ ~` and `/gamemode survival`, then break the lodestone: the needle starts spinning at once, as before.
5. Friend: the friend does step 1 with their own lodestone, compass and chest, and their needle spins too. **e3w16_friend_compass**

```
IW4L_MINECRAFT_KIT=minecraft:lodestone*4,minecraft:compass*4,minecraft:chest*2,minecraft:diamond_pickaxe+efficiency=5,minecraft:cobblestone*64,minecraft:cooked_beef*32,minecraft:torch*32
```

---

### Group `thrown-saved` (gap world-23; after D0's d0b)

**Cause**
- `save_chunk_entities` (`server.rs:1276-1305`) saves only what `breeze_charge::to_tag` accepts (`:1282`), and `breeze_charge.rs:40-63` refuses everything except the breeze charge.
- An unload takes out only breeze charges (`:1288`). Other thrown things keep flying in unloaded chunks and are lost on save.
- `add_saved_entity` (`:1242-1251`) loads only tridents and breeze charges, so a vanilla-saved snowball goes dormant.
- `Thrown::resumed` (`thrown.rs:446-473`) resumes a flight with owner `MOB_THROWER`. Mob shooters are found again each tick from `shooter_uuid` (`server.rs:2577-2579`), but players are not. Tridents already do this for players at `:2403-2405`.

**Vanilla 26.3**
- `ThrowableItemProjectile.addAdditionalSaveData` (javap) writes `Item`, on top of `Projectile`'s `Owner`, `LeftOwner` and `HasBeenShot` and the entity's `Pos`, `Motion` and `Rotation`. `readAdditionalSaveData` restores them.
- Entity ids: `minecraft:snowball`, `minecraft:egg` (the item keeps a blue or brown egg), `minecraft:splash_potion`, `minecraft:lingering_potion`, `minecraft:experience_bottle`.
- A player's `minecraft:wind_charge` is a hurting projectile with no `Item`, saved with `acceleration_power` like the breeze charge.
- Ender pearls are saved with their owner (`ServerPlayer.saveEnderPearls`), not with the chunk.

**Plan.** One commit: "Minecraft snowballs, eggs, potions, bottles and wind charges in flight are saved".
- `thrown.rs`: `to_tag(thrown, owner) -> Option<Tag>` and `from_tag(tag, random) -> Option<Thrown>`, next to `resumed`.
  - The entity id follows the kind.
  - Item projectiles write `Item` (`{id, count: 1, components}` through `item_nbt`, as dropped items are saved).
  - The flight fields are written as `breeze_charge::to_tag` writes them.
  - A pearl gives `None`.
  - `breeze_charge::to_tag` and `from_tag` keep their entity id and call the shared writer and reader.
- `server.rs`:
  - `:1282` uses `thrown::to_tag` with the owner's UUID: the mob shooter's, or a player owner's `uuid_of(PLAYER_TARGET + owner)`.
  - The unload at `:1288` keeps only what `to_tag` refuses, the pearls.
  - `add_saved_entity` tries `thrown::from_tag` next to the breeze charge (`:1247`).
- `tick_thrown`: a resumed thing whose `shooter_uuid` belongs to an online player gets that player as owner, as tridents do at `:2403-2405`. A new `Thrown::set_player_owner` also sets the flight's owner id for the left-owner check.

**Tests**
- `thrown::tests::every_thrown_item_but_a_pearl_saves_and_loads_its_flight`: a snowball, a blue egg, a poison splash potion with its components, a lingering potion, an experience bottle and a wind charge each keep their position, motion, rotation, item and components; a pearl gives `None`.
- `server::tests::a_snowball_in_flight_is_saved_with_its_chunk_and_flies_on`:
  - Unloading the chunk saves the snowball with `minecraft:snowball`, `Item` and the player as `Owner`.
  - After loading, it flies on and lands, and the player owns it again.
- `server::tests::a_vanilla_saved_splash_potion_loads_with_its_potion`.
- `breeze_charge::tests::a_breeze_charge_in_flight_saves_and_loads_with_its_owner` passes unchanged.

**Owner's steps** (survival, by day, flat ground)
1. Look up at about 45°, throw a poison splash potion, and at once press Esc and choose Save and Quit to Title. Load the world: the bottle is in the air where it was, flies on and breaks with its green splash. **e3w17_potion_after_load**
2. The same with an experience bottle: after the load it breaks and orbs drop.
3. The same with a wind charge thrown upward: after the load it flies on and bursts where it lands.
4. The same with a snowball thrown at a wall 20 blocks away: after the load it flies on and breaks on the wall. **e3w18_snowball_after_load**
5. The same with an ender pearl: after the load the pearl is gone and you stay where you were. Vanilla keeps pearls with the player; this is left for later.

```
IW4L_MINECRAFT_KIT=minecraft:splash_potion@poison*2,minecraft:lingering_potion@poison,minecraft:snowball*16,minecraft:egg*16,minecraft:experience_bottle*16,minecraft:wind_charge*16,minecraft:ender_pearl*16,minecraft:cobblestone*64,minecraft:cooked_beef*32
```

---

### Group `map-banners` (gap world-19 and the banners half of world-18; after E2's e2b)

**Cause**
- `MapData.banners` and `frames` are raw `Vec<Tag>`: `crates/minecraft_terrain/src/maps.rs:90-91` on track-e2, `:53-54` on minecraft-survival. They are read at `:303-304`, written back unchanged at `:259`, and nothing creates or updates them.
- Using a map on a banner does nothing.
- e2b's fill-in (`MapStore::fill_in`, `maps.rs:1219`) skips `checkBanners` (`track-e2-log.md:1311`).
- `MapMark` (`crates/minecraft_net/src/messages.rs:374-379`) has no name (log `:1183`).
- A banner's colour and custom name are already read for drawing in `crates/minecraft_terrain/src/banner_look.rs`.

**Vanilla 26.3**
- `MapItem.useOn`: on a block in `#banners`, the server calls `MapItemSavedData.toggleBanner(level, pos)`. A false result is `InteractionResult.FAIL`.
- `toggleBanner` (javap):
  - The banner's map pixel must lie within −63..63.
  - `MapBanner.fromWorld` gives `pos`, `color` and `name`, or null when there is no banner block entity.
  - If the same banner is already marked, it is removed with its marker.
  - Otherwise, unless `isTrackedCountOverLimit(256)`, it is added with `addDecoration(banner_<color>, id, x, z, 180, name)`.
- `checkBanners(BlockGetter, x, z)` (javap) runs from `MapItem.update` for each filled column. It removes a marked banner in that column whose `fromWorld` no longer equals it: gone, recoloured or renamed.
- The codec saves `banners` as `MapBanner` entries: `pos`, `color` (default white) and an optional `name`. Reading them back adds their markers.
- `MapRenderer.render` (javap) draws a marker's name centred 4 map pixels below it, at scale `clamp(25 / width, 0, 6/9)`, on a 0x80000000 background.

**Plan**
1. "Minecraft maps mark banners like vanilla"
   - `maps.rs`:
     - A typed `MapBanner { pos, color, name }` beside `MapData`, read from and written to the codec's `banners` list. `frames` stays raw until `map-frames`.
     - `MapStore::toggle_banner(id, pos, banner)` and `check_banners(id, x, z, banner_at)` add and remove `banner_<color>` markers in `Held` through e2b's `addDecoration` port, at rotation 180.
     - Loading a map adds its banner markers.
     - `fill_in` calls `check_banners` for each column it fills.
   - `minecraft_survival`: using a filled map on a block in `#minecraft:banners` toggles the banner through the map store. This goes in the item-on-block path in `actions.rs`, where e2b's map use sits. The colour and custom name come from the block entity (`banner_look.rs`); a refused toggle is a failed use.
2. "Minecraft map banner markers show their names, friends too"
   - `minecraft_net/src/messages.rs`: `MapMark.name: Option<String>` (a "## Protocol" line).
   - The held map picture (`crates/hud/src/minecraft_held_map.rs` and `minecraft_terrain/src/map_picture.rs`) draws each name with the GUI font, as `MapRenderer` does.

**Tests**
- `maps::tests::a_map_used_on_a_banner_marks_it_and_a_second_use_removes_it`: also covers an out-of-range banner and a missing banner, both refused.
- `maps::tests::a_gone_or_recoloured_banner_leaves_the_map_when_its_column_fills_in`.
- `maps::tests::banner_markers_are_saved_in_the_banners_list_and_come_back`, with vanilla's field names.
- `minecraft_net` `codec::tests::every_message_to_the_client_round_trips`, with a named `MapMark`.
- `hud` `minecraft_held_map::tests::a_named_marker_draws_its_name_under_it`.

**Owner's steps** (survival, by day)
1. Right-click an empty map to make a map of where you stand. Place a red banner 10 blocks away and right-click it with the map.
   - A red banner marker appears on the map where the banner stands. **e3w19_banner_marker**
   - Right-click again: the marker goes. Once more: it comes back.
2. Run `/experience add @s 30 levels`, place the anvil and rename a white banner "Home" in it (the fork's `/give` takes no `minecraft:custom_name` component). Place the banner within the map's area and right-click it with the map: a white marker appears with "Home" under it. **e3w20_banner_name**
3. Break the red banner and walk around its spot with the map in hand: the red marker vanishes as the map fills that place again.
4. Save and Quit, then load: the "Home" marker and its name are still on the map.
5. Friend: drop the map for the friend. The friend picks it up and sees the white marker with "Home". **e3w21_friend_banner_name**

```
IW4L_MINECRAFT_KIT=minecraft:map*4,minecraft:red_banner*4,minecraft:blue_banner*4,minecraft:white_banner*4,minecraft:anvil,minecraft:diamond_axe,minecraft:cobblestone*64,minecraft:cooked_beef*32
```

---

### Group `map-frames` (gap world-20 and the frames half of world-18; after E2's e2b; last and optional)

**Cause**
- On track-e2, `MapStore::tick_carried_on` (`maps.rs:1165-1190`) ports `tickCarriedBy` without item frames (`track-e2-log.md:1139`, `:1150`).
- Item frames (`crates/minecraft_terrain/src/item_frame.rs:113-126`) hold a map as an ordinary item:
  - Nothing ticks the map.
  - `append_frames` (`mesh.rs:1275`) draws the `filled_map` item model.
  - `FrameView` (`item_frame.rs:155-168`), which friends get in the entity snapshot, has no map id.
- Map pictures reach friends only for maps they carry (e2b's `Sessions::send_maps`).

**Vanilla 26.3**
- `ServerEntity.sendChanges` (javap): every 10 ticks, a frame holding a map with saved data calls `tickCarriedBy(player, stack, frame)` for every player in the level and sends each player the update.
- `tickCarriedBy` with a frame and `trackingPosition` (javap):
  - Adds `MapFrame(pos, direction.get2DDataValue() × 90, frame id)` under the key `MapFrame.frameId(pos)`; a different frame at the same place loses its marker.
  - Calls `addDecoration(FRAME, "frame-<id>", x, z, direction × 90, null)`.
- `removedFromFrame(pos, id)`, called from `ItemFrame.removeFramedMap`, removes both.
- The codec saves `frames` with `pos`, `rotation` and `entity_id`.
- `ItemFrameRenderer.submit` (javap), map branch:
  - It uses the `map=true` frame model.
  - Transform, in order: `translate(0, 0, 0.4375)`, `rotZ(rotation % 4 × 2 × 45°)`, `rotZ(180°)`, `scale(0.0078125)`, `translate(−64, −64, 0)`, `translate(0, 0, −1)`.
  - Light is 15728850 in a glow frame.
  - It calls `MapRenderer.render(..., true, light)`, which skips player markers.

**Plan**
1. "Minecraft maps in item frames mark their frame like vanilla"
   - `item_frame.rs`: `FrameView.map: Option<i32>`, taken from the framed stack's `minecraft:map_id` (a "## Protocol" line).
   - `maps.rs`:
     - A typed `MapFrame { pos, rotation, entity_id }` in the codec's `frames` list.
     - `MapStore::tick_framed(frames, carriers, game_time)` runs every 10 ticks per level.
     - `removed_from_frame(id, pos, frame)` runs when a map leaves its frame or the frame breaks: `frame_action` and `frame_broken`, `server.rs:3864-3890`, reported in the output.
   - `minecraft_survival/src/maps.rs` `tick` takes each level's framed maps from its frame views, and framed map pictures go to the players who track the frame.
2. "Minecraft item frames show the map they hold, friends too"
   - The world atlas reserves map slots, for example 16 slots of 128 × 128.
   - A framed map in view takes a slot. Its picture (colours plus frame and banner markers, without player markers or background) is written as an `AtlasPatch` (`mesh.rs:235-238`) whenever the picture changes.
   - `append_frames` draws the `map=true` frame model and the picture quad with vanilla's transform.
   - The host takes pictures from `Runtime.map_cache`, a friend from `RemoteWorld.maps`.
   - If the atlas has no room, ship commit 1 alone and log this commit as left.

**Tests**
- `maps::tests::a_framed_map_marks_its_frame_facing_its_wall_and_forgets_it_when_taken_out`.
- `maps::tests::frame_markers_are_saved_in_the_frames_list_and_come_back`.
- `maps::tests::a_second_frame_at_the_same_place_replaces_the_marker`.
- `item_frame::tests::a_frame_view_carries_its_map_id`.
- `mesh::tests::a_framed_map_is_drawn_flat_in_the_frame_turned_by_quarter_turns`.

**Owner's steps** (survival, by day)
1. Right-click an empty map. Hang an item frame on a wall and put the map in: the frame shows the whole picture, flush on the wall. **e3w22_framed_map** Right-click the frame: the picture turns a quarter each time.
2. Run `/give @s minecraft:filled_map[minecraft:map_id=0]`. The framed map's tooltip shows its number, and a fresh world's first map is #0. Hold the new copy: a green frame marker sits where the frame hangs, pointing the way the frame faces. **e3w23_frame_marker**
3. Punch the map out of the frame: the green marker leaves the held copy. Put the map back.
4. Save and Quit, then load: the picture is back in the frame and the marker is back on the held copy.
5. At night, put a second map in a glow item frame: its picture is fully bright.
6. Friend: the friend sees both pictures in their frames without carrying a copy. **e3w24_friend_framed_map**

```
IW4L_MINECRAFT_KIT=minecraft:map*4,minecraft:item_frame*8,minecraft:glow_item_frame*4,minecraft:oak_planks*64,minecraft:cobblestone*64,minecraft:diamond_axe,minecraft:cooked_beef*32,minecraft:torch*32
```

---

### Commit groups (this area)

| Order | Key | Waits for | Commits |
|---|---|---|---|
| 1 | `trial-arrows` | W w3 | 1. Minecraft arrows from ominous trial spawners keep their effect's level. 2. Minecraft arrows from ominous trial spawners can be picked up. |
| 2 | `ominous-shots` | W w3 (D0 d0b is merged) | 1. Minecraft snowballs, eggs, wind charges and tridents without a thrower hurt what they hit. 2. Minecraft ominous trial spawners shoot tridents and firework rockets. |
| 3 | `trial-clock` | - | 1. Minecraft levels take the world's game time back when a player returns. 2. Minecraft trial spawners see a crouching player's eyes and drop items above their head. |
| 4 | `vault-looks` | - | 1. Minecraft vaults forget the ejection count on load like vanilla. 2. Minecraft stacked items in vaults and on the ground are drawn like vanilla. |
| 5 | `lodestones-unloaded` | - | 1. Minecraft lodestone compasses check lodestones in unloaded chunks. |
| 6 | `thrown-saved` | D0 d0b (merged) | 1. Minecraft snowballs, eggs, potions, bottles and wind charges in flight are saved. |
| 7 | `map-banners` | E2 e2b | 1. Minecraft maps mark banners like vanilla. 2. Minecraft map banner markers show their names, friends too. |
| 8 | `map-frames` | E2 e2b | 1. Minecraft maps in item frames mark their frame like vanilla. 2. Minecraft item frames show the map they hold, friends too. |

### Left for later (this area)
These go to the log's "Left for later":
- world-2: several effects per arrow.
- world-7: dropping preserved equipment.
- world-11: luck in vaults.
- world-16: the explorer search off the frame.
- Ender pearls in the player file.
- Swimming and gliding heights.
- Arrow effects on mobs.
- Saving stuck arrows.

The reasons are in the E3 summary.

## E3-15. Particles, models and drawing

This section closes 16 of the 22 open drawing gaps of the release test files in eight groups. The other six are under "Left out" at the end.

**Where things stand.** Anchors are at `minecraft-survival` 4301058a. The held map files (`crates/hud/src/minecraft_held_map.rs`, `crates/render_anim/src/minecraft_held_map.rs`, `McHeldMap` in `crates/frame/src/minecraft_ui.rs`) exist only in E2's merge d565a3d2 (`C:\Dev\Soscraft\mc-wt\track-e2`), so the last two groups wait for that merge. Check each anchor again before editing, because D0, W, M1, M3 and later M2 merge in the meantime.

**How vanilla was checked.** Every vanilla fact comes from `javap -c -p` on the 26.3 `client.jar` (`C:\Games\Mashup\2010-Rust-Rewrite-Mashup\iw4l-artifacts\minecraft-26.3\client.jar`, read with the JDK 21 `javap` under `C:\Program Files\Eclipse Adoptium\jdk-21.0.8.9-hotspot\bin`) and from the 26.3 `assets/minecraft/items/*.json` in that jar.

**Rules for every commit:**
- No code comments of any kind. Edits change only the lines they need, in the surrounding style.
- New `ambient_particles::Kind` variants are appended at the end, as that enum's convention says. Every other new variant or match arm goes next to a related one.
- New files must pass `rustfmt --check`. Never commit a Mojang file.
- Never bump `minecraft_net::PROTOCOL`. Note any wire or wire-content change under "## Protocol" in the track log.

**Files the M tracks guard:**
- `entity_view.rs` and `mob_tracking.rs` are on the M briefs' never-edit lists for registered families. E3 edits only the arms of existing, unregistered kinds and the glow pass.
- The terrain root `families.rs` (the `draw_glowing` callback, group `draw-mob-held`) is a hook change. It goes in its own clearly named commit and is listed in the log, because M1 and M3 add their own hooks to that file.
- Vendored `world/equines.rs` belongs to M3 for the undead horses and camels. E3 adds one line in `breed_horses` only.

### Group `draw-particles` (M), three commits

**Gaps:** drawing-12, drawing-13, drawing-6.

**Cause:**
- `crates/minecraft_terrain/src/ambient_particles.rs:1438-1439` builds `right` and `up` from the camera forward for every kind, so every quad fully faces the camera. `Kind::TrialSpawnerDetection` and `TrialSpawnerDetectionOminous` (`:1458`) have no other mode (decision in `track-d-log.md:248`).
- `crates/minecraft_terrain/src/level_events.rs:374` pushes `(Kind::SculkCharge, at, motion)` with no roll. The `Particle` struct (`ambient_particles.rs:92-110`) has no roll, and the mesher never turns a quad in its plane (decision in `track-w-log.md:463`).
- `crates/minecraft_terrain/src/spell_particles.rs` knows nothing about scoping. `swirl` (`:200-222`) sets a fixed alpha, and `tick` (`:274-283`) only moves the particles. `crates/minecraft_survival/src/entity_view.rs:197-209` ticks clouds, swirls and spells with no eye or scoping input, although the host knows its scoping (`crates/render_anim/src/minecraft_world.rs:1476`) and so does the friend (`crates/render_anim/src/minecraft_remote.rs:1318`).

**Vanilla 26.3:**
- **Detection sparks.** `TrialSpawnerDetectionParticle.getFacingCameraMode` returns `SingleQuadParticle$FacingCameraMode.LOOKAT_Y`. Its lambda sets the quaternion to `(0, camera.rotation().y, 0, camera.rotation().w)`. JOML's `transform` divides by the norm, so the quad only turns around Y: right is horizontal and up is world Y. `LOOKAT_XYZ` copies the full rotation.
- **Sculk charges.** `LevelEventHandler.levelEvent` 3006 (offsets 2938-3122): with `data & 63 == 0` it puts particles on all six faces with roll `3.1415927` on `Direction.DOWN` only. Otherwise it puts them on the faces from `MultifaceBlock.unpack` with roll π on `Direction.UP` only. `SingleQuadParticle.extract` applies `rotateZ(lerp(partial, oRoll, roll))` after the facing mode. The sculk charge particle keeps its roll.
- **Spell particles.** `SpellParticle.isCloseToScopingPlayer` is true when the local player's eye is within `distanceToSqr <= 9.0` of the particle, the camera is first person and `LocalPlayer.isScoping`.
  - The constructor then calls `setAlpha(0)`.
  - `tick` sets `alpha = 0` while that holds, and otherwise `alpha = lerp(0.05, alpha, originalAlpha)`.
  - `setAlpha` also stores `originalAlpha`, and `SpellParticle$MobEffectProvider` calls `setAlpha(option.getAlpha())` after the constructor.
  - The check covers every spell particle near the scoping player, not only that player's own.

**Plan:**

1. **"Minecraft trial spawner detection sparks turn only around the vertical like vanilla"**
   - `ambient_particles.rs`: add `fn faces_y_only(self) -> bool` next to `translucent()`. It is true for `TrialSpawnerDetection` and `TrialSpawnerDetectionOminous`.
   - In `append_mesh` (`:1438-1439`), such a particle uses `right` as now, which is already horizontal, and `Vec3::Y` as up. MW2's pitch stops short of ±90°, so the horizontal part of `forward` is never zero. Keep `normalize_or_zero`.
2. **"Minecraft sculk charges under a block and on top of a vein are drawn upside down like vanilla"**
   - `ambient_particles.rs`: `Kind::SculkChargeRolled`, appended at the end of `Kind`. It shares every `SculkCharge` arm (constructor, tick, sprite, size, the full-bright light at `:1449`, layer) as `Kind::SculkCharge | Kind::SculkChargeRolled`.
   - The mesher turns a particle's corners by its kind's roll: π for this kind, so corner `(x, y)` becomes `(−x, −y)` and the sprite is upside down.
   - `level_events.rs:374`: push `SculkChargeRolled` when `(faces == 0 && y == -1) || (faces != 0 && y == 1)`. `FACES` (`:30`) is in vanilla's `Direction` order: index 0 is down, 1 is up.
   - Friends draw level events through the same `level_event`, so nothing goes on the wire.
3. **"Minecraft effect particles near a player looking through a spyglass fade out like vanilla"**
   - `spell_particles.rs`:
     - `Spell` gets `original_alpha`, and `SpellParticles` gets `scope: Option<DVec3>` with `pub fn set_scope(&mut self, eye: Option<DVec3>)`.
     - `spawn` sets alpha 0 when the scope eye is within 3 blocks (distance² ≤ 9) of the spawn point.
     - The swirl's alpha line (`:217-219`) sets both `alpha` and `original_alpha`, as vanilla's `setAlpha` does.
     - `tick` (`:274-283`) runs after the move. It sets alpha 0 while the particle is close and otherwise adds `(original_alpha − alpha) × 0.05`.
   - `entity_view.rs`: a `pub scope_eye: Option<DVec3>` field, handed over with `self.spell.set_scope(self.scope_eye)` before the cloud and swirl loop (`:197`).
   - **Host:** next to `ui.scoping` (`minecraft_world.rs:1476`), set `entities.view.scope_eye = (ui.scoping && !mc_detached).then(|| DVec3::from_array(eye))`.
   - **Friend:** next to `minecraft_remote.rs:1318`, the same from the friend's eye and its detached-camera flag. Vanilla checks only the local player, in first person.

**Tests:**
- `ambient_particles::tests::trial_spawner_sparks_turn_only_around_the_vertical`: with the camera looking 80° down, a detection spark's two upper corners lie straight above its lower ones in world Y, while a `Smoke` quad tilts.
- `ambient_particles::tests::a_rolled_sculk_charge_draws_its_sprite_upside_down`: at the same spot, its corner uvs are those of `SculkCharge` turned by half a turn.
- `level_events::tests::charges_under_a_full_block_and_on_a_vein_s_top_are_rolled`:
  - data 0 with charge 1: exactly the down-face particles are rolled;
  - faces up and north: only the up-face ones are rolled.
- `spell_particles::tests::spell_particles_by_a_scoping_eye_vanish_and_fade_back`: within 3 blocks the alpha is 0 at spawn and while scoping. After the scope ends it rises by 5 % of the gap each tick. A particle 4 blocks away keeps its alpha, and a swirl keeps its ambient alpha as its original.
- `entity_view::tests::a_scoping_viewer_hides_the_swirl_at_its_eye`.

**Owner's steps** (survival, Normal, by day, flat ground, the kit below):

1. **Sparks from above.**
   1. Dig a pit 1 wide and 2 deep, and put the trial spawner at its bottom.
   2. Stand at the pit's edge and use a zombie spawn egg on the spawner. With the detect sound, orange-gold sparks rise out of the pit.
   3. Look straight down into it: the sparks are thin upright slivers seen from above. Before this change, they were flat squares facing you. **e3d01_sparks_from_above**
2. **Sparks from below.**
   1. Build a cobblestone pillar 4 blocks high and put the second trial spawner on top, standing on a block beside it.
   2. Climb down, stand next to the pillar's foot and look straight up while a friend or a second egg wakes it (break and replace the spawner if it already detected you). The sparks stay upright and do not tilt towards you. **e3d02_sparks_from_below**
3. **Sculk charges.**
   1. Build a stone shelf two blocks above the ground, 3 by 3, on one pillar. Place a sculk catalyst on top of it and stand under the shelf's edge.
   2. Hatch cows on the shelf and kill them there with the sword. Sculk spreads with blue charge sparks.
   3. Look up at the shelf's underside: the sparks that crawl there are the same sprite as those on top, turned upside down. **e3d03_sculk_underside**
4. **Spyglass.**
   1. `/effect give @s minecraft:regeneration 60 1`. Swirls rise from your body into view.
   2. Hold the spyglass and right-click. The swirls in front of the lens vanish, and those further than 3 blocks stay.
   3. Release: the swirls near you fade back in over about a second. **e3d04_scope_swirls**, while scoping.
   4. Throw a lingering potion of regeneration 2 blocks in front of you and scope over the cloud: the cloud particles within 3 blocks of your eye vanish, and the far edge stays.
5. **Two windows.** F scopes inside the cloud: F's view hides the near particles, and H's view shows them all. **e3d05_scope_friend**, both windows.

```
IW4L_MINECRAFT_KIT=minecraft:trial_spawner*2,minecraft:zombie_spawn_egg*4,minecraft:sculk_catalyst*2,minecraft:cow_spawn_egg*8,minecraft:stone*64,minecraft:cobblestone*64,minecraft:spyglass,minecraft:lingering_potion@regeneration*2,minecraft:diamond_sword,minecraft:diamond_shovel,minecraft:shield,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:torch*16,minecraft:cooked_beef*16
```

16 stacks.

### Group `draw-mob-held` (M), three commits

**Gaps:** drawing-21, drawing-11, drawing-26.

**Cause:**
- **Skeleton bows.**
  - `crates/minecraft_terrain/src/skeleton_render.rs:172-178` pushes `"minecraft:bow"` or `main_hand` by plain id.
  - `crates/minecraft_terrain/src/mob_tracking.rs:359-366` sends only `bow.aggressive`, and `show_on` (`:639-642`) sets only that.
  - The vendored `BowState` already has `using_item` and `ticks_using_item` (`third_party/minecraftoss/entities/src/monster_ai.rs:473-490`). `MobShow.using_item: Option<i32>` exists (`mob_tracking.rs:74`), so nothing new goes on the wire.
  - The walker already reads `use_duration` (`special_models.rs:550`). Illagers already key their crossbows this way (`illager_render.rs:382-386` `hand_key`).
- **Drowned tridents.**
  - `crates/minecraft_terrain/src/zombie_render.rs:105-108` pushes the drowned's main-hand item by plain id, although `raises_trident` (`:183-185`) knows it is aiming.
  - `ItemVisuals::model` (`crates/minecraft_terrain/src/mesh.rs:959-967`) reads the right and left displays with `using_item` false (`model.rs:194`, `:213-216`), so a keyed using state could not reach `trident_throwing`'s display anyway.
- **Outlines of held items.** `crates/minecraft_survival/src/entity_view.rs:1217` and `:1227` drop the held items of glowing illagers and witches (`let _ =`). `families::draw_glowing` (`:1231`, `crates/minecraft_terrain/src/families.rs:97-107`, `:384-400`, `glowing_of` `:548-575`) hands only the body to `glow_render::append_glowing`. Held items are meshed only into `out.items` (`entity_view.rs:1486-1500`).

**Vanilla 26.3:**
- **Bows.** `RangedBowAttackGoal.tick` calls `startUsingItem` and fires when `getTicksUsingItem >= 20`. `items/bow.json` is a `using_item` condition whose `on_true` is a `use_duration` `range_dispatch` (scale 0.05): `bow_pulling_0` from the start, `_1` at 0.65 (13 ticks) and `_2` at 0.9 (18 ticks).
- **Tridents.** `Drowned$DrownedTridentAttackGoal.start` calls `setAggressive(true)` and `startUsingItem(MAIN_HAND)`, and `stop` undoes both. `items/trident.json`'s fallback is a `using_item` condition: `on_true` is the special trident with base `item/trident_throwing`, whose `thirdperson_righthand` display is rotation `[0,90,180]`, translation `[8,-17,9]` (left hand `[8,-17,-7]`). `trident_in_hand` is the resting one.
- **Outlines.** `ItemInHandLayer.submitArmWithItem` passes `ArmedEntityRenderState.outlineColor` into `ItemStackRenderState.submit` (offsets 248-251), so held items join the body in the outline target.

**Plan:**

1. **"Minecraft skeletons, strays, bogged and parched draw their bows back like vanilla"**
   - `mob_tracking.rs:359-366`: `using_item: e.bow.using_item.then_some(e.bow.ticks_using_item)`.
   - `show_on` `:639-642`: set `e.bow.using_item = show.using_item.is_some()` and `e.bow.ticks_using_item = show.using_item.unwrap_or(0)`.
   - `skeleton_render.rs:172-178`: key the bow with `crate::item_look::held_key(&ItemStack::new("minecraft:bow", 1), entity.bow.using_item.then_some(entity.bow.ticks_using_item))`. Other items keep their plain id.
2. **"Minecraft drowned hold their tridents in the throwing pose while aiming like vanilla"**
   - `zombie_render.rs:105-108`: when `raises_trident(entity)`, key the main hand as `item_look::keyed(item, &ItemLook { use_ticks: Some(0), ..ItemLook::default() })`.
   - `mesh.rs:959-967`: read both hand displays with `look.state().using`. The right hand uses `model::item_display_transform_using(.., "thirdperson_righthand", using)`. The left hand uses a `using` variant of `item_third_person_transform`, added beside it in `model.rs:213`.
   - The hand geometry (`resolve_item_model_in`) is the same special trident in both states, and the key's use suffix keeps the two cached apart.
   - Players' F5 trident charge is not part of this (the MW2 body keeps its gun hold; `MINECRAFT-SURVIVAL.md` Limitations).
3. **"Glowing Minecraft mobs outline the items they hold"**
   - `entity_view.rs:1216-1229`: keep each glowing illager's and witch's returned held items. Mesh them into their own `ChunkMesh` with `self.items.append_held_items(..)`, and pass that mesh to `glow_render::append_glowing` as a separate hull, so the body's hull keeps its own centre and scale.
   - `families.rs`: the `each` callback of `draw_glowing` becomes `FnMut(u64, &ChunkMesh, &[HeldItem])` in the macro (`:97-107`), the root (`:384-400`) and `glowing_of` (`:548-575`, which already collects `held`).
   - The families' held items get the same separate hull. To keep the borrow checker happy, collect the held lists first, then mesh them.
   - Log the `draw_glowing` change as a hook change.

**Protocol:** none. `MobShow.using_item` already exists.

**Tests:**
- `mob_tracking::tests::a_skeleton_drawing_its_bow_shows_its_use_ticks_to_friends`: `shows` carries the ticks, and `show_on` puts them on a friend's skeleton.
- `skeleton_render::tests::a_drawing_skeleton_holds_the_pulling_bow_models`: with a real pack, skipped without one: at 0, 13 and 18 ticks the keys resolve to `bow_pulling_0`, `_1` and `_2`, and to the plain bow when the skeleton is not drawing.
- `zombie_render::tests::an_aiming_drowned_keys_its_trident_as_used`.
- `mesh::tests::a_using_trident_takes_the_throwing_display_in_either_hand` (real pack, skipped without): its right display differs from the resting one by `trident_throwing`'s rotation and translation.
- `entity_view::tests::a_glowing_pillager_outlines_its_crossbow`: the glow mesh gains a second hull around the hand, and a pillager that is not glowing adds nothing.
- The existing glow tests still pass.

**Owner's steps** (survival, Normal, `/time set midnight`, flat ground lit by a few torches, full diamond armour and the shield):

1. **Bows.** Use a skeleton egg 10 blocks away. Each time it aims, its bow string pulls back in three steps over about a second, then it shoots. **e3d06_skeleton_pull**, mid-draw. Do the same with the stray, the bogged and the parched eggs.
2. **Drowned.** `/summon minecraft:drowned ~6 ~ ~ {equipment:{mainhand:{id:"minecraft:trident",count:1}}}`. When it aims at you it raises the trident over its shoulder, point forward, then throws. Before this change it raised the resting trident. **e3d07_drowned_throw**
3. **Outlines.**
   1. Place the bell. Hatch two pillagers, one vindicator and one witch within 10 blocks, then right-click the bell's side.
   2. About 2 seconds later they glow for 3 seconds, and the pillagers' crossbows and the vindicator's axe have the white outline too.
   3. Step behind a cobblestone wall: the outlines of the bodies and the weapons show through it. **e3d08_glow_held**
4. **Two windows.**
   - F stands 10 blocks from the skeleton: F sees the same three-step draw.
   - H rings the bell: F sees the weapons outlined. **e3d09_glow_held_friend**, F's window.

```
IW4L_MINECRAFT_KIT=minecraft:skeleton_spawn_egg*2,minecraft:stray_spawn_egg,minecraft:bogged_spawn_egg,minecraft:parched_spawn_egg,minecraft:bell,minecraft:pillager_spawn_egg*2,minecraft:vindicator_spawn_egg,minecraft:witch_spawn_egg,minecraft:cobblestone*64,minecraft:shield,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:diamond_sword,minecraft:golden_apple*8,minecraft:torch*16,minecraft:cooked_beef*16
```

19 stacks.

### Group `draw-dyed` (S), two commits

**Gaps:** drawing-16, drawing-17.

**Cause:**
- **Wolf armour.**
  - `special_models.rs:627-636` picks `on_true` only for `minecraft:using_item` with `state.using`.
  - `crates/minecraft_terrain/src/interface.rs:38-41` `item_model_leaf` always takes `on_false`, and its test (`:155-169`) pins the undyed `wolf_armor`.
  - The dye colour is already in the item key (`item_tint::of_stack_at`, e1 log line 610), but `ItemState` and `ItemLook` (`item_look.rs:51-66`) carry no "has `dyed_color`".
- **Frames.** `item_frame.rs:372-375` `view()` fills `FrameView.potion` from `potions::stack_color` only, while held items key `item.potion.or(item.dye)` (e1 log line 613).

**Vanilla 26.3:**
- `items/wolf_armor.json` is a `minecraft:has_component` condition on `minecraft:dyed_color`. Its `on_true` is `item/wolf_armor_dyed` with tints `[constant -1, dye default 0]`.
- `HasComponent.get` returns `ItemStack.hasNonDefault(type)` with `ignore_default`, else `ItemStack.has(type)`. No item has a default `dyed_color`, so both are "the stack has it".
- `ItemFrameRenderer` submits the framed stack with all its components, so dye tints show.

**Plan:**

1. **"Dyed Minecraft wolf armour shows its colour like vanilla"**
   - `item_look.rs`:
     - `ItemLook.dyed: bool`, set in `of()` when `minecraft:dyed_color` is present, written by `suffix()` as `dyed=1` and read back by `parse`;
     - `ItemState.dyed`, false in `plain()` and set by `state()`.
   - `special_models.rs:627-636`: also pick `on_true` when `property` is `minecraft:has_component`, `component` is `minecraft:dyed_color` and `state.dyed`.
   - `interface.rs`:
     - `item_model_leaf_in(value, look)` takes `on_true` for that condition when `look.dyed`, and `item_model_leaf(value)` calls it with the default look;
     - `item_look_tints` (`:62`) takes its tints from `item_model_leaf_in(model, look)`.
     - The pinned test now expects `[[255,255,255], dye]` for a dyed look and still the plain model for an undyed one.
   - Icons, both hands, F5, friends (`GearItem.look`), the ground (`of_stack`) and frames (`FrameView.look`) all carry the look already.
2. **"Minecraft item frames show dyed items in their colour"**
   - `item_frame.rs:372-375`: `.and_then(|stack| crate::potions::stack_color(stack.components.as_ref()).or_else(|| crate::body_gear::dyed_color(stack)))`.
   - The serde `FrameView` and the friends' wire stay the same.

**Protocol:** none (look and colour content only; friends run the same build).

**Tests:**
- `item_look::tests::a_dyed_stack_s_look_says_so_and_round_trips`.
- `special_models::tests::a_has_component_condition_on_dyed_color_picks_on_true_when_dyed`.
- `interface::tests::dyed_and_firework_tints_use_their_default_colour_through_selects`, updated as above.
- `item_icons::tests::a_red_wolf_armour_icon_is_red_on_its_dye_layer` (real pack, skipped without): it differs from the plain icon, and its layer-1 pixels are the sprite times #B02E26.
- `item_frame::tests::a_framed_dyed_leather_piece_carries_its_dye_in_the_colour_slot`: a red leather chestplate gives `Some(0xB02E26)`, and a potion keeps its potion colour.

**Owner's steps** (survival, by day):

1. **Inventory.** The red and the blue wolf armour icons show a red and a blue band. The plain one is unchanged. **e3d10_wolf_armour_icons**
2. **Hand and ground.** Hold the red wolf armour: red in first person, and in F5 in your body's hand. Press Q: red on the ground.
3. **Frames.**
   1. Build a plank wall and hang four item frames on it.
   2. Put in the red wolf armour, the blue leather chestplate, the lime leather horse armour and the yellow leather helmet. Each shows its colour. Before this change, the leather pieces were brown and the wolf armour plain. **e3d11_dyed_frames**
   3. Do the same in a glow item frame.
4. **Two windows.** F sees the dyed items in H's frames and in H's hand. **e3d12_dyed_friend**, F's window.

```
IW4L_MINECRAFT_KIT=minecraft:wolf_armor@red,minecraft:wolf_armor@blue,minecraft:wolf_armor,minecraft:leather_chestplate@blue,minecraft:leather_horse_armor@lime,minecraft:leather_helmet@yellow,minecraft:item_frame*8,minecraft:glow_item_frame*2,minecraft:oak_planks*32,minecraft:cooked_beef*8
```

10 stacks.

### Group `draw-birth-hearts` (S), one commit

**Gap:** drawing-22.

**Cause:**
- The vendored birth sites record no entity event 18:
  - chickens: `third_party/minecraftoss/entities/src/world.rs:7111-7131`;
  - pigs `:7382`, sheep `:7699`, cows and mooshrooms `:8100-8130`;
  - horses, donkeys, mules and llamas: `world/equines.rs:989` `breed_horses`;
  - striders: `world/striders.rs:656`; hoglins: `world/hoglins.rs:628`.
- Wolves (`wolves.rs:309`), cats (`felines.rs:649`) and rabbits (`families/land/rabbits.rs:319`) do record it.
- The route to hearts exists: `take_entity_events`, then `mob_particles::mob_puffs` (event 18 is 7 hearts at lift 0.5 for every type but villagers), then the snapshot and `ToClient::Bursts` (e1 log line 165).

**Vanilla 26.3:** `Animal.finalizeSpawnChildFromBreeding` resets both parents' love and then calls `ServerLevel.broadcastEntityEvent(this, 18)` once, on the parent the birth is worked from (offsets 42-53). `Animal.handleEntityEvent(18)` draws 7 `HEART`.

**Plan:**
- **"Minecraft farm animals, horses, striders and hoglins show hearts when a baby is born"**
  - One line `self.broadcast_entity_event(<parent id>, 18)` (`world.rs:4150`), using the store's id of the breeding mob (`index`), after the love reset at each site above.
  - Not camels (M3 owns their breeding).
  - The `breed_horses` line is the only one in `equines.rs`. Re-check it after merging `minecraft-survival`, where M3 may have moved it.

**Protocol:** none.

**Tests:**
- Vendored entities tests: `a_birth_puffs_love_hearts_on_the_breeding_parent`. Run it for a chicken, pig, sheep, cow and mooshroom pair, a tame horse pair, a strider pair and a hoglin pair, each set in love and ticked to the birth. `take_entity_events` holds exactly one `(parent, 18)` from the birth tick.
- `minecraft_terrain::server::tests::a_calf_s_birth_shows_hearts_in_the_snapshot`.

**Owner's steps** (survival, by day, a fenced 8 by 8 pen):

1. Hatch two cows and feed each wheat. Love hearts show (as before), and a few seconds later a calf appears while one parent puffs 7 hearts again. **e3d13_birth_hearts**
2. Do the same with pigs (carrots), sheep (wheat), chickens (seeds) and mooshrooms (wheat).
3. `/summon minecraft:horse ~2 ~ ~ {Tame:1b}` twice, and feed each a golden carrot: a foal and the hearts.
4. Hatch two striders and feed each warped fungus. They shiver on land but breed: a baby and the hearts.
5. **Two windows.** F stands 10 blocks away and sees the birth hearts.

```
IW4L_MINECRAFT_KIT=minecraft:cow_spawn_egg*2,minecraft:pig_spawn_egg*2,minecraft:sheep_spawn_egg*2,minecraft:chicken_spawn_egg*2,minecraft:mooshroom_spawn_egg*2,minecraft:strider_spawn_egg*2,minecraft:wheat*64,minecraft:carrot*32,minecraft:wheat_seeds*32,minecraft:warped_fungus*8,minecraft:golden_carrot*8,minecraft:oak_fence*64,minecraft:oak_fence_gate,minecraft:cooked_beef*8
```

14 stacks.

### Group `draw-ride-heads` (S), one commit

**Gap:** drawing-3.

**Cause:**
- `crates/render_anim/src/anim/minecraft_glide.rs:66-74` stops the walk animation while `input.riding` (`MC_RIDING`), and `minecraft_gear.rs:534` takes `head_swing = glide.walk_position(body.client)` only.
- So a mounted player's worn dragon jaw and piglin ears rest.
- A rider gets no velocity: `sim::frame::ride` sets `ps.velocity` to zero and moves `ps.origin` to the seat (`crates/sim/src/frame.rs:465-480`).

**Vanilla 26.3:** `LivingEntityRenderer.extractRenderState` (offsets 145-186) sets `wornHeadAnimationPos` to the vehicle's `walkAnimation.position(partial)` when `getVehicle()` is a `LivingEntity`, and otherwise to the entity's own (resting) `walkAnimationPos`.

**Plan:**
- **"Worn Minecraft heads move with the gait of the mount a player rides"**
  - Every vehicle a player can ride in the fork is a living mob: `server_mobs.rs:497` `VEHICLES` plus the registry's vehicles. The seat moves exactly as the mount does, so the mount's walk is the walk of the rider's seat.
  - `minecraft_glide.rs`:
    - `GlideInput` gets `origin: [f64; 3]` (from `ps.origin`, in blocks through `sim::voxel::BLOCK`).
    - `BodyGlide` gets `ride_walk: WalkAnimation` and `ride_from: Option<[f64; 3]>`.
    - On each walk tick while riding and alive, `ride_walk.update` takes the horizontal distance the seat moved since `ride_from`, split over this frame's ticks.
    - When not riding, `ride_from` is cleared and `ride_walk` stops.
    - `head_swing()` returns `ride_walk`'s position while riding and `walk_position()` otherwise.
  - `minecraft_gear.rs:534`: `head_swing: glide.head_swing(body.client)`.
  - Host and friends both run `step_body_glides` on the presented snapshot, so both see it. A flying mount (M3's happy ghast) would add vertical motion in vanilla; log that this one uses the horizontal walk.

**Protocol:** none.

**Tests:**
- `minecraft_glide::tests::a_rider_s_worn_head_follows_the_mount_s_gait`: a seat moving 0.2 blocks a tick advances the head swing like walking.
- `a_still_mount_lets_the_head_rest`.
- `getting_off_returns_to_the_body_s_own_walk`.
- `minecraft_gear::tests::a_mounted_body_s_head_swing_comes_from_its_ride`.

**Owner's steps** (survival, by day):

1. Put the dragon head in your helmet slot.
2. `/summon minecraft:horse ~2 ~ ~ {Tame:1b}`, right-click it with a saddle, and ride. In F5, when the horse walks the dragon's jaw opens and shuts with its steps. Standing still it rests. Before this change it stayed shut on the horse. **e3d14_dragon_head_riding**
3. Swap to the piglin head: its ears flap with the gait.
4. Hatch the strider on a lava pool (two lava buckets in a cobblestone pit) and ride it with the warped fungus on a stick: the same.
5. **Two windows.** F watches H ride and sees the jaw move. **e3d15_ride_head_friend**, F's window.

```
IW4L_MINECRAFT_KIT=minecraft:dragon_head,minecraft:piglin_head,minecraft:saddle,minecraft:saddle,minecraft:golden_carrot*16,minecraft:strider_spawn_egg,minecraft:warped_fungus_on_a_stick,minecraft:lava_bucket,minecraft:lava_bucket,minecraft:cobblestone*64,minecraft:cooked_beef*8
```

11 stacks.

### Group `draw-compass` (L), three commits

**Gaps:** drawing-15, drawing-14.

**Cause:**
- **The first frame (drawing-15).**
  - `crates/minecraft_terrain/src/dials.rs:144-156` `watch()` only registers lodestone keys.
  - `step()` (`:162-193`) returns early when no tick passed and keys exist (`:169-171`).
  - `key()` (`:136-142`) falls back to the plain item's key for a lodestone key not stepped yet, so a new lodestone compass shows the plain needle for its first frame (d log line 514).
- **Everyone else's compasses (drawing-14).** One dial state per item key is stepped only with the local player's `DialContext` (`minecraft_world.rs:921`, `minecraft_remote.rs:1336`), and only icons and the first-person hand read it (`minecraft_world.rs:1498`; d log line 515). Everything else keys the plain compass, whose default model is the needle at rest:
  - other players' held items (`body_render.rs:23-40` `held_items` with `GearItem::key`, through `anim/minecraft_gear.rs`);
  - frames (`item_frame.rs:372-379` keys the plain `stack.id`, composed in `mesh.rs:1275-1296`);
  - dropped items (`mesh.rs:1022` `of_stack`, inside `EntityView::meshes` at `entity_view.rs:1383`).
- `GearItem` (`body_gear.rs:10-27`) and `FrameView` carry no lodestone target.

**Vanilla 26.3 (`CompassAngleState`):**
- `calculate(stack, level, seed, owner)` uses `getRandomlySpinningRotation(seed, gameTime)` without a valid target in the owner's dimension (at least 1e-5 away). That is the shared `noTargetWobbler`, stepped once per game tick, plus `hash(seed) / 2.1474836E9` with `hash(i) = i * 1327217883`.
- Otherwise it uses `getRotationTowardsCompassTarget`. Only for the local player while the tick rate runs normally does the shared `wobbler` follow `0.5 − (yaw − 0.25)`. For every other owner the result is `positiveModulo(0.5 − (visualYaw − 0.25 − angle), 1)` with no wobble, where `angle = atan2(dz, dx) / 2π` from the owner's position to the block centre and `visualYaw = positiveModulo(getVisualRotationYInDegrees / 360, 1)`.
- The owner's visual yaw:
  - a living owner: `yBodyRot`;
  - `ItemFrame`: `wrapDegrees(180 + facing.get2DDataValue × 90 + rotation × 45 + (vertical ? 90 × step : 0))`;
  - `ItemEntity`: `180 − getSpin(age + 0.5, bobOffs) / 2π × 360`, where `getSpin = age / 20 + bobOffs`.
- `CompassTarget`: `SPAWN` is the level's spawn and `LODESTONE` comes from the stack. `RECOVERY` is the owner's last death, which the client knows only for the local player, so it spins elsewhere.

**Plan:**

1. **"A new Minecraft lodestone compass shows its needle on the first frame"**
   - `dials.rs` `step`: when no tick passed, still fill every watched key that has no entry yet, at the current `self.tick`. There is no tick advance, so no needle moves twice in one tick (`Needle::update` ignores a repeated tick).
2. **"Minecraft compasses in item frames and on the ground point at their targets"**
   - `dials.rs`:
     - `pub struct DialOwner { pub feet: [f64; 3], pub yaw: f32, pub seed: i32 }`.
     - `pub fn owner_key(&mut self, key: &str, owner: &DialOwner, packs: &PackStack, context: &DialContext) -> Option<String>`: for a drawn-item key whose item (after `item_tint::split`, `item_look::split` and `item_of`) is a compass or recovery compass, it returns the key with the dial model put in by `keyed` (look and colour kept), from the owner formulas above.
     - The lodestone target comes from the key's `@lodestone:` part. Without a target, the spinning branch reads the shared spinning needle (`needles["compass:<target>"].1`, stepped by `step`) plus the seed term. Clocks keep `key()`.
     - Thread an `owner: Option<&DialOwner>` through the private `leaf`, `value` and `compass` functions.
   - **Frames:** `FrameView` keeps `item` as the plain id. The lodestone part (`dials::stack_key` after the id) goes in front of `look`, so `item + look` equals `of_stack`'s shape. `ItemLook::parse` ignores a segment without `=`, and `item_look::split` and `dials::split` already take that shape apart.
   - `EntityView::meshes` (`entity_view.rs:937`) gets `dials: Option<(&mut Dials, &DialContext)>` from both callers: the host at `minecraft_world.rs:1518` (`inventory_ui.dials` and the context of `:921`) and the friend at `minecraft_remote.rs:2015` (`remote.dial_context`).
   - Before `append_frames` (`:1415`), each compass frame's key is rewritten with `owner_key`. Owner: the frame's position, the frame yaw formula above, seed = the frame id. Do the same for dropped items before `ItemVisuals::mesh` (`:1383`): the yaw from `age` and `bob_offset` (`mesh.rs:1039-1041`), seed = the item's entity id. Confirm both seeds with javap on `ItemFrameRenderer` and `ItemEntityRenderer`.
3. **"Minecraft compasses held by other players point at their targets"**
   - `GearItem::of` (`body_gear.rs:19-27`) puts the lodestone part of `dials::stack_key(stack)` in front of `look`, so `GearItem::key()` equals `item_tint::of_stack(stack)`. There is no new field.
   - `gear_meshes` (`minecraft_world.rs:2308`, `:2310`) gets the dials and context.
   - Bodies other than the viewer's own key held compasses through `owner_key`. Owner: the body's feet, `minecraft_yaw` of its view yaw for `yBodyRot`, seed = the client number.
   - The viewer's own F5 body and inventory character keep `dials.key()`, the local wobbling needle, as vanilla does for the local player.
   - Recovery compasses held by others spin.

**Protocol:** no layout change. `GearItem.look` and `FrameView.look` gain the lodestone part as content. Note it under "## Protocol"; friends run the same build.

**Tests:**
- `dials::tests::a_new_lodestone_compass_shows_its_needle_on_its_first_frame`.
- `a_framed_compass_points_at_spawn_from_its_facing_without_wobble`: four facings, plus `rotation` 2.
- `a_dropped_compass_needle_turns_against_its_spin`.
- `another_player_s_compass_points_from_their_body`: turning the owner keeps the world direction.
- `a_recovery_compass_away_from_the_viewer_spins_with_its_seed` and `a_compass_in_another_dimension_spins`.
- `body_gear::tests::a_held_lodestone_compass_keys_like_a_dropped_one`.
- `item_frame::tests::a_framed_lodestone_compass_view_carries_its_target`.
- An `entity_view` test: a framed compass is drawn with a dial model, not the plain compass.

**Owner's steps** (survival, Overworld, by day):

1. Walk about 50 blocks east of spawn. Build a 3-high plank pillar, hang item frames on its four sides, and put a compass in each. Every needle points towards spawn, so frames on opposite sides point opposite ways relative to their frames. Before this change all four showed the needle at rest. **e3d16_framed_compasses**
2. Turn one framed compass with right-clicks: its needle keeps pointing at spawn.
3. Place a lodestone 20 blocks away and right-click it with a compass. In your hand the needle points at it at once. Put that compass in a frame: it points at the lodestone. Break the lodestone: the framed needle spins.
4. Drop a compass with Q: while it spins on the ground, its needle keeps turning back towards spawn.
5. Put the recovery compass in a frame: it spins.
6. **Two windows.**
   - F holds a compass and turns slowly. In H's window, the needle in F's hand keeps pointing towards spawn. **e3d17_friend_compass**, both windows.
   - F binds a compass to H's lodestone: H sees F's needle point at it.

```
IW4L_MINECRAFT_KIT=minecraft:compass*4,minecraft:lodestone*2,minecraft:recovery_compass,minecraft:clock,minecraft:item_frame*8,minecraft:glow_item_frame*2,minecraft:oak_planks*64,minecraft:cobblestone*32,minecraft:diamond_pickaxe,minecraft:cooked_beef*8
```

10 stacks.

### Group `draw-hand` (M), three commits, after E2's merge

**Gaps:** drawing-7, drawing-30, drawing-29.

**Cause:**
- **The HUD toggle.** `crates/render_anim/src/minecraft_world.rs:1484` filters only the item activation by `UiDraw`. `:1485-1486` (`main_stack`, `offhand_stack`) are filtered by detach and scoping but not by `ui_draw`, so the host's held items stay with the HUD hidden. A friend's are gated by `HandGate.items`, which includes `ui_shown` (`minecraft_remote.rs:1845-1847`, set at `:681`).
- **No lag behind the view.** `crates/render_anim/src/minecraft_hand.rs:168` `held_hand_mesh` poses from the current view only, and no lagging yaw or pitch is kept on the host (`minecraft_world.rs:1487-1500`) or the friend (`minecraft_remote.rs` `show_hand`). The held map uses the view's own pitch (E2 log line 1223; `hud/src/minecraft_held_map.rs:27` `pose` in `mc-wt/track-e2`).
- **No hand height.** `minecraft_hand.rs:44-46` `item_pose` takes `inverse_height`, but every caller passes `0.0` (`:196`), and there is no per-hand height state.

**Vanilla 26.3:**
- **The HUD toggle.** `GameRenderer.renderItemInHand` returns before `FirstPersonHandsAndItemsRenderer.submitHandsWithItems` when the camera is panoramic, there is no player, the camera is not first person, the camera entity sleeps, `isHudHidden`, or the game mode is spectator (offsets 1-64).
- **The lag.**
  - `LocalPlayer.applyInput` once a tick: `yBobO = yBob`, `xBobO = xBob`, `xBob += (xRot − xBob) × 0.5`, `yBob += (yRot − yBob) × 0.5`.
  - `submitHandsWithItems` rotates the whole hands pose by `(viewXRot − lerp(xBobO, xBob)) × 0.1` degrees around X, then `(viewYRot − lerp(yBobO, yBob)) × 0.1` around Y.
- **The height (`FirstPersonHandsAndItems.tick`).**
  - The old heights are copied first.
  - A shown stack is swapped at once when `ItemStack.matchesIgnoringComponents` holds or the item model's `shouldPlaySwapAnimation` is false (its `hand_animation_on_swap`).
  - The targets are `swap³` for the main hand while the shown stack is the held one, else 0. `swap` is `getItemSwapScale(1) = clamp((itemSwapTicker + 1) / attackDelay, 0, 1)`, and the ticker resets on an attack and on a main-hand change. The offhand target is 1 while its shown stack matches, else 0.
  - Each hand moves by at most ±0.4 a tick, and the shown stack becomes the held one once its height is below 0.1.
  - `itemUsed(hand)` zeroes that hand's height. `Minecraft.startUseItem` calls it after every successful `useItem`, and after a successful `useItemOn` whose stack count changed (or with infinite materials).
  - The renderer uses `1 − lerp(old, height)` as `inverseArmHeight`, so the item drops by `0.6 ×` that (`applyItemArmTransform`).

**Plan:**

1. **"Minecraft held items hide with the HUD like vanilla"**
   - `minecraft_world.rs:1485-1486`: both filters also require `ui_draw.as_ref().is_none_or(|draw| draw.0)`, through a small pure `hand_items_shown(detached, scoping, ui_shown)` with a test.
   - With the HUD hidden, the HUD's held map is not drawn either.
2. **"The Minecraft first-person hand lags behind the view like vanilla"**
   - `minecraft_hand.rs`:
     - `HandBob { x, y, x_o, y_o }` with `tick(pitch, yaw)` (the 0.5 ease) and `turn(partial, pitch, yaw) -> Mat4`.
     - Unwrap the yaw: add the wrapped difference to a running yaw, so turning past ±180° never swings the hand 36°.
     - Snap it on the first tick and after a teleport or level switch.
   - `Held` gets `bob: Mat4`, applied in front of every posed item (not the totem activation).
   - **Host:** ticked with the hand clock once per client tick (`minecraft_world.rs:1487-1500`).
   - **Friend:** in `show_hand` with its own view. Signs follow the fork's view space (x right, y up, z back; MW2 pitch is positive looking down like `xRot`; `minecraft_yaw` for `yRot`).
   - **Held map:** `McHeldMap` gets the two angles and `hud` `pose()` applies the same turn.
3. **"The Minecraft first-person hand dips when items are switched, used or swung like vanilla"**
   - `minecraft_hand.rs`: `HandHeights` with both heights, their old values and the shown stacks, `tick(...)` by the vanilla rules above, and `used(hand)`.
   - The main-hand target uses the fork's attack strength (`minecraft_survival::survival::attack_strength(hand.attack_ticks, speed)`, `actions.rs:382`, cubed). If `attack_ticks` does not reset on a main-hand change, the dip still comes from the shown-stack mismatch.
   - The drawn stack is the shown one, not the inventory's.
   - `item_pose(display, swung, 1 − lerp(old, height), arm)` at `:196`, and the held map's pose gets the same drop.
   - `used(hand)` is called by the host where an item use succeeds (eating or drinking starts, throwing, buckets, spyglass) and where a placement changed the stack count or the player is creative, as `startUseItem` does.
   - The friend keeps its own `HandHeights` from its inventory, swing and use.

**Protocol:** none. The hand is drawn locally on each side.

**Tests:**
- `minecraft_world::tests::held_items_hide_with_the_hud_scoping_and_the_detached_camera`.
- `minecraft_hand::tests`:
  - `the_hand_lags_half_way_to_the_view_each_tick_and_turns_a_tenth_of_the_gap`;
  - `turning_past_one_eighty_does_not_swing_the_hand`;
  - `switching_items_dips_the_hand_and_raises_the_new_one` (the old stack is drawn until below 0.1);
  - `an_attack_lowers_the_main_hand_with_the_cube_of_its_strength`;
  - `the_offhand_dips_only_on_a_change`;
  - `a_matching_stack_with_new_components_swaps_at_once`;
  - `a_used_item_drops_its_hand`.
- `hud` `minecraft_held_map::tests::the_map_lags_and_dips_with_the_hand`.

**Owner's steps** (survival, by day):

1. Hold the diamond sword and press F1: the HUD and the sword vanish. F1 again: both are back. **e3d18_f1_hand**
2. Turn the view quickly left and right, then up and down: the sword trails the view a little and settles back into place. Spin a full turn: no jump at any point.
3. Scroll from the sword to the pickaxe: the sword drops out of view and the pickaxe rises, about a quarter second. **e3d19_swap_dip**, mid-swap.
4. Left-click the air with the sword: it dips and rises back over about 0.6 s. With the axe the rise takes about 1 s. Clicking fast keeps it low.
5. Eat beef and place a cobblestone: the hand dips and rises.
6. Hold an empty map and right-click: you get a filled map. Turn and switch with it: it lags and dips like the sword.
7. **Two windows.** F does steps 1 to 4 in F's own view.

```
IW4L_MINECRAFT_KIT=minecraft:diamond_sword,minecraft:diamond_pickaxe,minecraft:diamond_axe,minecraft:map*4,minecraft:shield,minecraft:torch*16,minecraft:cobblestone*32,minecraft:cooked_beef*16
```

8 stacks.

### Group `draw-map-light` (S), one commit, after E2's merge

**Gap:** drawing-27.

**Cause:** in `mc-wt/track-e2`, `crates/hud/src/minecraft_held_map.rs:86-101` `map_quads` draws with colour `[1.0; 4]`, at full brightness. E2's log line 1222 and its MINECRAFT-SURVIVAL line 127 say so. The map is a HUD draw, so it never passes the world's lightmap.

**Vanilla 26.3:** `FirstPersonHandsAndItemsRenderer.renderMap` passes its packed light (`iload_3`) to `MapRenderer.render(state, pose, collector, active, light)` and draws the background with `RenderTypes.text`, so the map is lit by the lightmap at the player, like the held items.

**Plan:**
- **"Held Minecraft maps are lit by the light where the player stands"**
  - **CPU lightmap.** A CPU port of the terrain `lightmap()` that the hand pipeline applies (`crates/render_gpu/src/drawsurf/minecraft_world.rs:957-971`, used by `hand_vertex`). Put it in `render_anim` beside where the environment array is filled, as `fn lightmap(environment: &[[f32; 4]; 16], sky: f32, block: f32) -> [f32; 3]`, using these indices of the `Environment` struct (`:908-916`):
    - `light` 6 and `ambient` 14;
    - `block_tint` 15;
    - the darkness lane `forward.w` (0).
    - It clamps, then mixes the gamma by `cloud.w` (10).
  - **Light sample.** It is taken at the cell the held items are lit at: the host's `eye_light_at` (`minecraft_world.rs:1497`) and the friend's equivalent.
  - **Carry it to the map.** `McHeldMap` gets `light: [f32; 3]`, and `map_quads` colours both the background and the picture with it.

**Protocol:** none.

**Tests:**
- `the_cpu_lightmap_matches_the_shader_formula`: full daylight, a torch at night, darkness subtracting, the brightness mix.
- `a_held_map_in_the_dark_is_drawn_dark` (`McHeldMap.light`).
- `hud` `minecraft_held_map::tests::map_quads_carry_the_light`.

**Owner's steps** (survival):

1. Hold an empty map, right-click, and hold the filled map with the sword in the offhand. By day it looks as before.
2. `/time set midnight` in the open: the map is as dim as the sword beside it. Before this change it glowed. **e3d20_map_night**
3. Stand by a torch: warm and bright. In a dark cave: nearly black. **e3d21_map_cave**
4. **Two windows.** F sees the same in F's own view.

```
IW4L_MINECRAFT_KIT=minecraft:map*4,minecraft:torch*32,minecraft:diamond_sword,minecraft:diamond_pickaxe,minecraft:cobblestone*64,minecraft:cooked_beef*8
```

6 stacks.

### Left out

Six gaps stay open: drawing-1, drawing-5, drawing-10, drawing-19, drawing-20 and drawing-24. Each one's reason is in this brief's `left` list.

### Combined kit for this section

One line for a quick look at most of it (27 backpack slots). Each group's own kit is the full one.

```
IW4L_MINECRAFT_KIT=minecraft:trial_spawner,minecraft:zombie_spawn_egg*2,minecraft:sculk_catalyst,minecraft:cow_spawn_egg*4,minecraft:spyglass,minecraft:skeleton_spawn_egg,minecraft:bell,minecraft:pillager_spawn_egg*2,minecraft:wolf_armor@red,minecraft:leather_chestplate@blue,minecraft:item_frame*8,minecraft:wheat*32,minecraft:dragon_head,minecraft:saddle,minecraft:golden_carrot*8,minecraft:compass*4,minecraft:lodestone,minecraft:map*4,minecraft:diamond_sword,minecraft:diamond_pickaxe,minecraft:shield,minecraft:diamond_chestplate,minecraft:torch*32,minecraft:oak_planks*64,minecraft:cobblestone*64,minecraft:cooked_beef*16
```

27 stacks.

## E3-16. Sculk, vibrations, darkness and the warden

The known limits of track W (darkness, vibrations, sculk, the warden): `docs/round10/release-4-tests.md:780-805` (shriekers, vibrations not heard yet, vibrations that differ, catalysts) and `docs/round10/release-6-tests.md:393-399` (the warden). Eleven groups close 36 of the area's 38 open gaps; sculk-42 and sculk-57 are left (end of this section).

### Shared points

- **Anchors** are at `minecraft-survival` 44d96765 (E2's e2b merged). d0b's merge (silverfish in stone, endermites, flying navigation) and W's w3 move lines in `third_party/minecraftoss/entities/src/world.rs`, `crates/minecraft_terrain/src/server.rs` and `server_mobs.rs`: check every anchor again before editing.
- **W0's rules hold** (`docs/round10/track-w.md` W0): one line per call site through the existing helpers, after the state change it reports. Helpers: `Level::game_event` / `game_event_from` / `block_game_event_from` / `projectile_game_event` (`third_party/minecraftoss/world/src/level/game_event.rs:47-114`), `EntityWorld::game_event` (`third_party/minecraftoss/entities/src/world/game_events.rs:13`), a mob's own `Body.game_events` queue (`third_party/minecraftoss/entities/src/movement.rs:79`, drained with the mob as source by `take_game_events` `game_events.rs:29`) and `Entities::game_event` (`crates/minecraft_survival/src/entities.rs:1077`). A player as entity-world source is `PLAYER_TARGET + id` (`ServerSim::mob_or_player`, used at `server.rs:1040`). Frequencies come from `crates/minecraft_terrain/src/game_events.rs:60` `EVENTS`; nothing is added to that table. Before each line, find the vanilla call with javap and write it in the log; where vanilla posts nothing, post nothing.
- **Ownership** (`docs/round10/decisions.md`, "The M briefs give every cross-track hook one owner"): leash events are W's w3; the family actions of armadillos, camels, sniffers, turtles, nautiluses, creakings and sulfur cubes are M1-M3's. E3 adds no family hook and no ambient hook. Where a group extends a struct another track also extends (`EffectGrant`, M2), the group says so.
- **Protocol:** `EffectView` (group `sculk-darkness-icon`) and `VibrationView` (group `sculk-visuals`) change what friends decode. Never bump `minecraft_net::PROTOCOL`; add a line under "## Protocol" in `docs/round10/track-e3-log.md`.
- **Style:** no code comments of any kind; edits change only the needed lines; new enum variants and match arms next to related ones; new files pass `rustfmt --check`; new vendored files are counted in the log; cargo only through `scripts/cargo-slot.ps1`.

**The sensor rig** (from `docs/round10/release-4-tests.md:120-163`). Survival, by day, flat open ground; stand still and run:

```
/fill ~-2 ~-1 ~-2 ~2 ~-1 ~22 minecraft:stone
/fill ~-2 ~ ~-2 ~2 ~3 ~22 minecraft:air
/setblock ~ ~ ~4 minecraft:sculk_sensor
/setblock ~1 ~ ~4 minecraft:redstone_lamp
/setblock ~ ~ ~5 minecraft:comparator[facing=north]
/fill ~ ~ ~6 ~ ~ ~20 minecraft:redstone_wire
```

The block you stood on is **the start**, 4 blocks from the sensor. When the sensor hears something its lamp lights for 1.5 s and the comparator lights as many dust as the frequency: 1 step or flap, 2 landing or bounce, 3 throw or shot, 4 entity action, 5 dismount, 6 interact, 7 hurt, 8 eat, 9 close, 10 open, 11 block change, 12 break or fluid pickup, 13 place or fluid place, 14 entity place, 15 death or explosion. Wait 2 s between actions and move crouched.

**The calibrated rig** (a frequency filter): `/setblock ~ ~ ~4 minecraft:calibrated_sculk_sensor[facing=north]`, `/setblock ~1 ~ ~4 minecraft:redstone_lamp`, then n dust behind it with `/fill ~ ~ ~5 ~ ~ ~<4+n> minecraft:redstone_wire` and a lever placed by hand on the floor right after the last dust, flipped on. The sensor then hears only frequency 16 - n (n = 9 for 7, n = 11 for 5).

**Order:** `sculk-darkness-icon`, `sculk-warden`, `sculk-visuals`, `sculk-thrower`, `sculk-sources`, `sculk-projectiles`, `sculk-mob-events` (after W's w3), `sculk-deaths`, `sculk-saves`, `sculk-blocks`, `sculk-pots-shelves`.

---

### Group `sculk-darkness-icon`

**Gaps:** sculk-1 (`release-4-tests.md:781`, `release-6-tests.md:399`).

#### Cause
- Player effects keep only duration and amplifier: `third_party/minecraftoss/player/src/survival.rs:87` `EffectInstance` (its one literal is in `add_effect` `:280-292`, which replaces only on a higher amplifier or a longer time).
- `crates/minecraft_terrain/src/warden_tracker.rs:177` `EffectGrant { effect, ticks, amplifier, display_limit }` has no visibility. `SHRIEK_DARKNESS` (`:21`) serves the shrieker (`server.rs:1143`) and the warden's pulse (`server.rs:1155`). `crates/minecraft_survival/src/survival.rs:283` `take_effect` calls `add_effect`.
- `crates/minecraft_survival/src/views.rs:101-114` `effect_views` sends `ambient: false` and no icon flag; `EffectView` (`crates/minecraft_net/src/messages.rs:470`) and `McEffect` (`crates/frame/src/minecraft_ui.rs:144`) have none; `mc_effects` (`crates/render_anim/src/minecraft_world.rs:2268`, host `:1288`, friend `minecraft_remote.rs:1314`); `crates/hud/src/minecraft_effects.rs:154` `draw_effects` lays out every effect (`layout` `:121`).
- `crates/minecraft_survival/src/player_nbt.rs:655-672` writes `show_particles` and `show_icon` 1 for every effect; the load (`:613-627`) reads neither.
- `/effect give <targets> <effect> <seconds> <amplifier> <hideParticles>` parses the flag (`crates/minecraft_commands/src/commands.rs:52`) but `give_effect` (`:787`) ignores it; `effect_give` (trait `crates/minecraft_commands/src/server.rs:122`, impl `crates/minecraft_survival/src/commands.rs:418`) has no flag.

#### Vanilla 26.3 (javap)
- `Warden.applyDarknessAround`: `new MobEffectInstance(DARKNESS, 260, 0, false, false)`, then `MobEffectUtil.addEffectToPlayersAround(level, source, pos, radius, instance, 200)`. `SculkShriekerBlockEntity` (its response) and `Warden.customServerAiStep` call it.
- `MobEffectInstance.<init>(Holder, int, int, boolean ambient, boolean visible)` passes `visible` as `showIcon` too. `MobEffectInstance.update` copies `visible` and `showIcon` from the newer instance every time it runs, even when amplifier and time stay.
- `Hud.extractEffects` skips an effect when `!showIcon()`. `EffectsInInventory` has no `showIcon` check, so the list beside the inventory still shows it.
- `EffectCommands.giveEffect`: `visible = !hideParticles` into the same constructor.
- Saved effects (`MobEffectInstance$Details`): `show_particles` (default true) and `show_icon` (default `show_particles`).

#### Plan
**Commit "Minecraft darkness from wardens and shriekers hides its effect icon like vanilla"**
- Player crate: `EffectInstance` gains `visible` and `show_icon`. `add_effect` keeps its signature (both true). A new `add_effect_shown(kind, duration, amplifier, visible, show_icon)` beside it applies `update`'s flag copy even when the slot keeps its time.
- `EffectGrant` gains `visible: bool` right after `display_limit`; `SHRIEK_DARKNESS` gets `false`; `take_effect` calls `add_effect_shown`. **M2 also extends `EffectGrant`** (`any_mode`, `cue`: `docs/round10/track-m2.md:1215`, `:1532`): whichever track merges second adds the other's field to its literals.
- `EffectView` and `McEffect` gain `hidden: bool` (false shows, so `Default` keeps icons). `effect_views` and `mc_effects` carry it. `layout`/`draw_effects` skip hidden effects; `listed` (`minecraft_effects.rs:230`) keeps them. Protocol note.
- `player_nbt.rs`: write both flags from the instance; read them on load (`show_icon` defaulting to `show_particles`).
- `/effect give`: `effect_give` gains `visible` and passes `!hideParticles` to `add_effect_shown`. The fork draws no effect swirls on players, so `visible` only travels into the save.

#### Tests
- player crate: `a_newer_grant_sets_the_flags_even_when_its_time_is_shorter`.
- `warden_tracker::tests::shriek_darkness_is_hidden`; `survival::tests::a_shrieker_grant_hides_the_icon`.
- `views` and `hud`: `hidden_effects_are_listed_beside_the_inventory_but_not_in_the_hud`.
- `player_nbt::tests::a_hidden_effect_saves_show_icon_zero_and_loads_hidden`.
- `minecraft_commands`: `hide_particles_hides_the_icon`; `minecraft_net` codec: `EffectView.hidden` round trip.

#### Owner's steps (survival, Normal, flat open ground)
1. `/setblock ~ ~-1 ~3 minecraft:sculk_shrieker[can_summon=true]` and step on it: rings, a warden sound, the view darkens and pulses, and **no darkness icon** shows at the top right. Press E: Darkness is listed beside the inventory with its time. **e3s01_darkness_no_icon**
2. `/effect give @s minecraft:haste 120 0 true`: you mine faster, no icon; E lists Haste. Then `/effect give @s minecraft:haste 30 0`: the icon appears (the newer, visible grant takes over) and Haste keeps its 2 minutes. **e3s02_hidden_haste** (Players have no Speed effect in the fork: `EffectKind` and `/effect` have no `speed`, so the command would fail.)
3. `/effect clear @s`, `/effect give @s minecraft:haste 120 0 true`, Save and Quit, load: still no icon; E lists it.
4. Use a warden egg 15 blocks away: within 6 s its darkness comes, without an icon. `/difficulty peaceful` removes it, then `/difficulty normal`.
5. **(Two windows)** The friend steps on the shrieker (after its 10 s cooldown): the friend's view darkens with no icon, and the friend's E lists Darkness. **e3s03_friend_no_icon**

```
IW4L_MINECRAFT_KIT=minecraft:warden_spawn_egg*2,minecraft:sculk_shrieker*2,minecraft:milk_bucket,minecraft:milk_bucket,minecraft:shield,minecraft:diamond_chestplate+protection=4,minecraft:diamond_helmet,minecraft:golden_apple*8,minecraft:totem_of_undying,minecraft:cobblestone*64,minecraft:cooked_beef*32
```

---

### Group `sculk-warden`

**Gaps:** sculk-50, sculk-51, sculk-53, sculk-54, sculk-55, sculk-56 (`release-6-tests.md:395`, `:398`). **Waits for M1's m1d merge** (segment e3w): commit 2 builds on goats-ram's `MeleePush.blocked`.

#### Cause
- **Follow range:** the family uses `FOLLOW_RANGE` 24.0 (`third_party/minecraftoss/entities/src/world/families/deep_dark/wardens.rs:42`) in `walk_profile` (`:233`; each warden keeps its own `Warden.profile`, `:197`), the sensor scan (`wardens/brain.rs:340`) and the look follow (`brain.rs:568`). `ServerSim::summon_warden` draws and drops the bonus and the handedness (`crates/minecraft_terrain/src/server.rs:1182-1183`), and the summoned tag (`:1194-1197`) has neither. Eggs and `/summon` run `finalize_mob` (`third_party/minecraftoss/core/src/entity_data.rs:205`), which adds the `random_spawn_bonus` modifier but writes **base 16** when the tag has no `follow_range` (`:232`); the family never reads the tag's attribute.
- **Shield push:** `MeleePush::landing` (`crates/minecraft_survival/src/impulse.rs:45-55`) throws on `hurt || shielded`, written for the hoglin. The warden's `strike` (`wardens.rs:543-576`) sends `PlayerHitKind::Melee { throw }` (`third_party/minecraftoss/entities/src/world.rs:615`). `melee_push` (`crates/minecraft_survival/src/entities.rs:2057`) is called from `mob_hit` (`:2118`), which already has the attacker's `type_id`.
- **Slowdown:** `strike` multiplies its own x and z velocity by 0.6 after every swing (`wardens.rs:572-576`), `ravager_strikes` before every swing (`third_party/minecraftoss/entities/src/world/ravagers.rs:347-349`). Player hits are applied later on the survival side (`crates/render_anim/src/minecraft_players.rs:861` `survival.struck`); only a shield block comes back (`Entities::shield_blocked` `entities.rs:1282`, `Command::ShieldBlock` `server.rs:4353`, handle `:4794`, dispatch `:5079`).
- **Sonic boom on mobs:** `wardens.rs:617` calls `EntityWorld::mob_hits_mob` (`world.rs:4267`), which applies each store's armour (zombies `:4279-4280`, slimes `damage_after_armor`, the `hurt_armored` arms), types every hurt `minecraft:mob_attack` (the family arm's `Hurt::new` near `:4402`) and calls the witch with `resisted` false (`:4320`; `WitchEntity::hurt` `third_party/minecraftoss/entities/src/world/witches.rs:83`, the 15 % in `witch.rs:167-171`).

#### Vanilla 26.3 (javap)
- `Mob.finalizeSpawn`: `random_spawn_bonus` = `triangle(0, 0.11485)` as `ADD_MULTIPLIED_BASE` on `FOLLOW_RANGE`, then `setLeftHanded(nextFloat() < 0.05)` (saved `LeftHanded`). `Warden.createAttributes`: follow range 24. The warden's `NearestLivingEntitySensor` box and brain range checks use `getAttributeValue(FOLLOW_RANGE)`. `WardenModel` has no item-holding arms, so handedness changes nothing drawn.
- `Mob.doHurtTarget` calls `causeExtraKnockback` only when `target.hurtServer` succeeded; `LivingEntity.causeExtraKnockback`: knockback above 0 knocks the target back, then the attacker's own velocity × (0.6, 1, 0.6). `Hoglin.blockedByItem` and `Zoglin.blockedByItem` call `HoglinBase.throwTarget`; `Warden` has no `blockedByItem`; `Ravager.blockedByItem` stuns.
- `SonicBoom.tick`: `target.hurtServer(level, damageSources().sonicBoom(warden), 10)`. `minecraft:sonic_boom` is in `#bypasses_armor`, `#bypasses_enchantments` and `#witch_resistant_to` (26.3 data pack tags); `Witch.getDamageAfterMagicAbsorb` cuts `#witch_resistant_to` to 15 %.

#### Plan
1. **"Summoned Minecraft wardens keep vanilla's spawn bonus and handedness, and every warden uses its own follow range"**
   - `summon_warden`: the attempt closure returns its two draws with the spot. The tag gets `attributes: [{id: minecraft:follow_range, base: 24.0, modifiers: [{id: minecraft:random_spawn_bonus, amount: 0.11485 × draw, operation: add_multiplied_base}]}]` and `LeftHanded`.
   - The family reads the tag's follow range when it hatches a warden: base 24 plus 24 × the sum of `add_multiplied_base` modifiers (a saved base of 16 from `finalize_mob` is ignored and saved back as 24), into `Warden.profile.follow_range`. `brain.rs:340` and `:568` read the warden's value instead of the constant.
2. **"Minecraft warden and ravager blows push and slow like vanilla around shields"**
   - **No new `MeleePush` field.** M1's m1d (goats-ram) adds `MeleePush.blocked: f64`, the share of `throw` a raised shield lets through (1.0 by default), scales the throw by it in `landing` when shielded (`impulse.rs:48-49`), sets 0.5 in `melee_push`'s `Typed` arm for the goat's ram and gives every literal `blocked: 1.0`. `landing` stays as M1 wrote it.
   - `melee_push` (`entities.rs:2057`) gains the attacker's `type_id` from `mob_hit` (`:2118`). Its `Melee` arm sets `blocked` 0.0 unless `type_id` is `minecraft:hoglin` or `minecraft:zoglin` (1.0, vanilla's `blockedByItem` throw). M1's 0.5 for the goat's `Typed` ram stays.
   - New `Command::MeleeLanded { attacker }` beside `ShieldBlock`, sent by `Entities::melee_landed` (beside `shield_blocked`) from `minecraft_players.rs` right after `survival.struck` when it hurt and the cause is `DamageCause::Mob { attack: MobAttack::Melee, .. }` (the pattern of `:806-812`). `EntityWorld::melee_landed(id)` slows a warden or a ravager (attack knockback 1.5 each) by (0.6, 1, 0.6). It arrives a tick or more after the swing; note that in the log.
   - `strike` and `ravager_strikes` keep their own slowdown only after an applied hit on a mob.
3. **"Minecraft sonic booms pierce mob armour and witches resist them"**
   - `mob_hits_mob`'s body takes a hit type (`damage_type`, `bypasses_armor`); `mob_hits_mob` stays as the `minecraft:mob_attack` wrapper, so its other callers are untouched. M2's m2e puts its family knockback hook in front of `mob_hits_mob`'s knockback (E3-0): keep both sides at a merge. Each armour line skips armour for a bypassing hit; the `hurt_from` and family `Hurt` types use the given type (with `bypasses_armor`); the witch arm passes `resisted` when the type is in `#witch_resistant_to` (the list `witch.rs` already uses). The boom calls it with `minecraft:sonic_boom`.

#### Tests
- `server::tests::a_summoned_warden_saves_its_spawn_bonus_and_handedness` (seeded level random: the modifier equals the drawn triangle, `LeftHanded` matches the draw).
- entities: `a_wardens_follow_range_comes_from_its_tag` (21.24 to 26.76), `an_egg_tag_with_base_sixteen_still_gives_twenty_four_plus_its_bonus`.
- `impulse::tests::only_hoglins_and_zoglins_throw_through_a_shield`: a shielded warden or zombie melee push is `None`, a hoglin's or zoglin's throws in full, and M1's goat ram still lets half through.
- entities: `a_landed_blow_slows_the_warden_and_a_blocked_one_does_not`, `a_ravager_slows_only_after_a_landed_blow`; server: `melee_landed_reaches_the_entity_world`.
- entities: `a_sonic_boom_ignores_a_zombies_armour`, `a_witch_takes_fifteen_percent_of_a_sonic_boom`, `plain_melee_still_meets_armour`.

#### Owner's steps (survival, Normal, flat open ground at dusk; full diamond armour, the shield in the offhand)
1. Use a warden egg 12 blocks away. Raise the shield toward it as it swings: the blow is blocked, the shield goes down for 5 s, and **you are not pushed back** (before: thrown back). **e3s04_warden_shield**
2. Let it hit you without the shield: you are thrown back as before, and the warden stops pushing forward for a moment after each landed hit. Eat golden apples; `/difficulty peaceful` ends it, then `/difficulty normal`.
3. Run `/fill ~ ~ ~14 ~ ~2 ~14 minecraft:cobblestone` and `/summon minecraft:zombie ~ ~3 ~14 {equipment:{head:{id:"minecraft:diamond_helmet",count:1},chest:{id:"minecraft:diamond_chestplate",count:1},legs:{id:"minecraft:diamond_leggings",count:1},feet:{id:"minecraft:diamond_boots",count:1}}}`: a zombie in full diamond on a 3-high pillar. Use a warden egg beside the pillar and walk crouched 20 blocks away. The zombie's noise angers the warden; it cannot reach the zombie and fires sonic booms: **two booms kill it** (before: about ten). **e3s05_boom_armour**
4. Same with `/summon minecraft:witch ~ ~3 ~14` on a fresh pillar: the witch survives many booms, healing between them (before: three killed it).

```
IW4L_MINECRAFT_KIT=minecraft:warden_spawn_egg*4,minecraft:shield,minecraft:diamond_helmet,minecraft:diamond_chestplate+protection=4,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*16,minecraft:totem_of_undying,minecraft:cobblestone*64,minecraft:milk_bucket,minecraft:cooked_beef*32
```

---

### Group `sculk-visuals`

**Gaps:** sculk-3 (`release-4-tests.md:783`), sculk-49 (`release-6-tests.md:397`).

#### Cause
- `crates/minecraft_terrain/src/shriek_particles.rs:27-41` `Ring::tick` adds the rise to `position.y` with no collision; `ShriekRings::tick` (`:95`) takes no scene; `crates/minecraft_survival/src/entity_view.rs:903` calls it (host and friend share `EntityView`). `poof_particles.rs:134-155` already gathers the collision boxes around a particle from a `Scene`, and `:56-90` has vanilla's collide-and-stop.
- `VibrationView { from, to, ticks }` (`crates/minecraft_terrain/src/vibration_particles.rs:13-17`) has a fixed target; `Vibrations::tick` takes `listener.position()` (the warden's eye) once, when it picks the vibration and when it reloads it (`crates/minecraft_terrain/src/vibrations.rs:477-496`); `User::particle` (`:279`), `Hearing::particle` (`server.rs:4247`); friends get the ripple through `mob_sender` (`crates/minecraft_survival/src/mob_sender.rs:199`).

#### Vanilla 26.3 (javap)
- `ShriekParticle.<init>`: quad 0.85, delay, lifetime 30, gravity 0, `xd` 0, `yd` 0.1, `zd` 0; `hasPhysics` stays true. `Particle.move` collides its 0.2 box (`Entity.collideBoundingBox`) and sets `stoppedByCollision` when the asked vertical motion was cancelled; a stopped particle no longer moves.
- `Warden$VibrationUser.<init>`: `new EntityPositionSource(warden, warden.getEyeHeight())`. `VibrationSignalParticle.tick` reads the source's position every client tick and lerps toward it by `1 / (lifetime - age)`; when the position is empty (the warden is gone) the particle is removed.

#### Plan
1. **"Minecraft shriek rings stop under a ceiling"**: `ShriekRings::tick(scene)` gathers boxes like `poof_particles.rs:134-155`; `Ring::tick(boxes)` moves the 0.2 box up only (a ring has no x or z motion), clips the rise at the lowest box above it and sets a `stopped` flag when the asked rise was cut below 1e-5; a stopped ring keeps aging and fading in place. `entity_view.rs:903` passes the scene.
2. **"Minecraft vibration ripples follow the warden that hears them"**: `User::particle` and `VibrationView` gain `follow: Option<u64>` (the listener's `entity`; Protocol note). `VibrationSignals::tick` takes a closure that gives a mob's eye; `EntityView` passes one from its world (host) or its shown mobs (friend). A ripple whose warden is missing is removed, as vanilla's. `mob_sender` keeps `follow`.

#### Tests
- `shriek_particles::tests::a_ring_stops_under_a_ceiling_and_fades_there`, `a_ring_under_open_sky_rises_as_before`.
- `vibration_particles::tests::a_ripple_follows_its_wardens_eye`, `a_ripple_whose_warden_is_gone_disappears`; codec round trip of `VibrationView.follow`; `mob_sender::tests` keeps it for friends.

#### Owner's steps (survival by day; step 3 in creative)
1. `/setblock ~ ~-1 ~3 minecraft:sculk_shrieker` (a plain shrieker shrieks without warning) and `/fill ~-1 ~2 ~2 ~1 ~2 ~4 minecraft:glass` (a glass roof 2 blocks over it). Step on the shrieker: the rings rise, stop flat under the glass and fade there (before: they passed through it). **e3s06_shriek_ceiling**
2. `/fill ~-1 ~2 ~2 ~1 ~2 ~4 minecraft:air`, wait 5 s, step on it again: the rings rise high as before.
3. `/gamemode creative` (a warden ignores creative players). Use a warden egg 10 blocks away and throw snowballs onto the ground near it while it walks: each ripple bends to follow its moving head (before: it flew to where the head was). **e3s07_ripple_follows** **(Two windows)** the friend sees the same.

```
IW4L_MINECRAFT_KIT=minecraft:sculk_shrieker*2,minecraft:glass*16,minecraft:warden_spawn_egg*2,minecraft:snowball*16,minecraft:cobblestone*64
```

---

### Group `sculk-thrower`

**Gaps:** sculk-2 (`release-4-tests.md:782`).

#### Cause
- Level items have no thrower: `third_party/minecraftoss/world/src/level/entity.rs:29` `ItemData { stack, pickup_delay, age, health }` (its one literal is `Entity::item` `:187`). An item on a shrieker only records `unsupported` (`step_on` `:1280-1290`).
- The player's items live first in the client view (`third_party/minecraftoss/player/src/items.rs:15` `ItemEntity`, `toss` `:233`) and reach the server through `crates/minecraft_survival/src/entities.rs:851-866` `server.spawn_item` → `Command::SpawnItem` (`server.rs:4294`, handle `:4707`, dispatch `:5019`), with no player. Hand throws: `clicked.thrown` (`crates/render_anim/src/minecraft_inventory.rs:373` pending drops, `:378-382` Q and Ctrl+Q, click-outside throws), thrown at `minecraft_world.rs:1442`, `:2545` (`minecraft_inventory::throw` `:1030`, `Thrower` `:147`) and for friends at `minecraft_players.rs:2214`.
- `event_source` maps an item to `Source::Item { id, dampens }` (`server.rs:1223-1235`, enum `crates/minecraft_terrain/src/game_events.rs:167`); `Hearing::player_behind` (`server.rs:4161-4169`) returns `None` for it. The item NBT write and read (`crates/minecraft_terrain/src/server_mobs.rs:2081-2083`, `:2115-2120`) have no `Thrower`.

#### Vanilla 26.3 (javap)
- `LivingEntity.createItemStackToDrop(stack, randomly, includeThrower)` calls `ItemEntity.setThrower` when the last flag is true. `ServerPlayer.drop(boolean)` (Q), `AbstractContainerMenu` throws and the creative drop pass true; `ServerPlayer.handleExtraItemsCreatedOnUse` and `Inventory`'s drops pass false. Saved as `Thrower` (UUID int array).
- `SculkShriekerBlockEntity.tryGetPlayer` accepts a `ServerPlayer`, a controlling passenger, `Projectile.getOwner`, or `ItemEntity.getOwner` (the thrower). `SculkShriekerBlock.stepOn` calls `tryGetPlayer` for any entity, items included.

#### Plan
**Commit "Minecraft items remember who threw them, so shriekers answer the thrower"**
- `ItemData.thrower: Option<u128>` (UUID), saved as `Thrower` and read back in `server_mobs.rs`.
- The client `ItemEntity` gains `thrower: Option<u64>`. Q, Ctrl+Q and click-outside throws carry the player (host, and friends at `minecraft_players.rs:2214`); pending drops (`minecraft_inventory.rs:373`), leftovers, death drops and closed-menu returns carry none, as vanilla's false. `Command::SpawnItem` gains `thrower: Option<u64>`; the server stores the player's UUID (`mobs.uuid_of(PLAYER_TARGET + id)`).
- `Source::Item` gains `thrower: Option<u64>`, resolved in `event_source` from the UUID against the players; `player_behind` returns it for items.
- `step_on` on a shrieker: an item with a thrower queues `(pos, uuid)` in a new level list instead of `unsupported`; `ServerSim::hear_game_events` resolves it and adds it to `stepping` (`server.rs:1023`).

#### Tests
- player crate: `a_q_drop_carries_its_thrower_and_a_pending_drop_none`.
- world level: `an_item_with_a_thrower_on_a_shrieker_queues_a_step`.
- `server_mobs::tests::an_items_thrower_round_trips_as_a_uuid_int_array`.
- server: `a_thrown_item_on_a_shrieker_shrieks_for_its_thrower`, `a_sensor_clicked_by_a_thrown_item_makes_a_shrieker_shriek`, `a_block_drop_never_shrieks`.

#### Owner's steps (survival, Normal, the sensor rig)
1. `/setblock ~-1 ~ ~4 minecraft:sculk_shrieker` (beside the sensor; it cannot summon). At the start, crouched, press Q once facing the sensor: the cobblestone lands on the stone, the sensor lights 2, and **the shrieker beside it shrieks** (before: it stayed quiet). **e3s08_item_click_shriek**
2. `/setblock ~ ~-1 ~-3 minecraft:sculk_shrieker[can_summon=true]` behind the start. Stand 2 blocks from it and press Q so the cobblestone lands on it: it shrieks, darkness comes and a warden sound plays (your warning rose). **e3s09_item_on_shrieker**
3. Leave the item there and Save and Quit; load. 10 s after the last shriek it shrieks again for you (the item kept its thrower). Pick the item up now: left there, it keeps warning you every 10 s until a warden comes, as in vanilla.
4. **(Two windows)** The friend presses Q onto the summoning shrieker: the friend gets the darkness, the host does not (unless within 16 blocks).

```
IW4L_MINECRAFT_KIT=minecraft:cobblestone*64,minecraft:sculk_shrieker*2,minecraft:milk_bucket,minecraft:milk_bucket,minecraft:cooked_beef*32
```

---

### Group `sculk-sources`

**Gaps:** sculk-35, sculk-36, sculk-37 (`release-4-tests.md:794-796`).

#### Cause
- `menus::recheck_lids` (`crates/minecraft_survival/src/menus.rs:1890-1901`) calls `lid_effects` with `None`, and `lid_effects` (`:1915-1917`) posts only with a player. The survival side's only queue is `PlayerEvent { player, .. }` (`crates/minecraft_terrain/src/game_events.rs:241`, `Entities::game_event` `entities.rs:1077`); `hear_game_events` makes the player the source (`server.rs:1004-1010`).
- `crates/minecraft_survival/src/actions.rs:1235` posts `item_use_event` (`game_events.rs:141-154`: `fluid_pickup`/`fluid_place` on a cauldron) from the player, so shriekers answer and wardens get a suspect.
- `Survival::observe` (`crates/minecraft_survival/src/survival.rs:408-427`) counts `hurts` and adds hurt exhaustion even when absorption took the whole MW2 hit; `Steps::heard` (`crates/minecraft_survival/src/player.rs:101-107`, called at `crates/render_anim/src/minecraft_players.rs:334`) posts `entity_damage` from `hurts`. `record` (`:582-586`) counts only `dealt > 0`. The sleep wake (`sleep.rs:471`) must keep seeing every hurt.

#### Vanilla 26.3 (javap)
- `ContainerOpenersCounter.recheckOpeners` posts `level.gameEvent(null, CONTAINER_OPEN or CONTAINER_CLOSE, pos)`.
- `CauldronInteractions.fillBucket` and `emptyBucket` post `level.gameEvent(null, FLUID_PICKUP / FLUID_PLACE, pos)`.
- `Player.actuallyHurt`: after armour, magic and absorption, it returns when the amount left is 0, before `causeFoodExhaustion`, the combat tracker and `gameEvent(ENTITY_DAMAGE)`. The sleeper still wakes (`hurtServer` stops sleeping first).

#### Plan
1. **"Minecraft containers closed by the game and buckets used on cauldrons make vibrations without a source like vanilla"**: `Entities::game_event_without_source(event, pos)` beside `game_event`, sending a new `Command::GameEvent { event, pos }` beside `ChestOpeners` (`server.rs:4347`), dispatched as `level.game_event_from(event, pos, None, None)`. `lid_effects` uses it when there is no player; `actions.rs:1235` uses it for the cauldron events (a small `game_events` helper says which uses have no source).
2. **"Minecraft damage that absorption takes whole makes no hurt vibration and costs no hunger"**: `Survival` gains a `damaged` counter (and `damaged()`), counted in `observe` only when health was lost and in `record` with `dealt > 0`; the hurt exhaustion moves under the same check. `hurts` keeps counting every hurt for the sleep wake. `minecraft_players.rs:334` passes `damaged()`.

#### Tests
- server: `an_unsourced_container_close_reaches_a_sensor_but_not_a_shrieker`; menus: `recheck_lids_posts_container_close_without_a_player`; `game_events::tests::cauldron_fluid_events_have_no_source`.
- `survival::tests::fully_absorbed_damage_counts_a_hurt_but_no_damage_and_no_hunger`, `damage_past_absorption_counts_both`.

#### Owner's steps (survival, Normal, the sensor rig)
1. `/setblock ~-1 ~ ~4 minecraft:sculk_shrieker` and `/setblock ~1 ~ ~2 minecraft:cauldron`. Crouched at the start, empty the water bucket into the cauldron: the sensor lights 13 and **the shrieker stays quiet** (before: it shrieked for you). Wait 2 s and fill the bucket from it: 12, quiet. Wait 5 s and place a cobblestone 2 blocks from the sensor: 13 and the shrieker shrieks (a placed block has you as source). **e3s10_cauldron_quiet**
2. `/fill ~2 ~ ~2 ~2 ~4 ~2 minecraft:cobblestone`, `/tp @s ~2 ~5 ~2`, and jump down beside the pillar: the fall costs some health and the sensor lights 7 (if the fall does not hurt, add a block to the pillar). Go back up, eat a golden apple, wait 2 s and jump again: the yellow hearts take the fall, the sensor lights only 2 (the landing), and the hunger bar does not move from the hit. **e3s11_absorbed_fall**
3. The recheck close (a lid that lost its player without a close) has no reliable owner step; its test covers it.

```
IW4L_MINECRAFT_KIT=minecraft:water_bucket,minecraft:cauldron,minecraft:sculk_shrieker*2,minecraft:golden_apple*8,minecraft:cobblestone*64,minecraft:cooked_beef*32
```

---

### Group `sculk-projectiles`

**Gaps:** sculk-19, sculk-20, sculk-21, sculk-22, sculk-34, sculk-41 (`release-4-tests.md:788`, `:794`, `:798`).

#### Cause
- Arrows already post (`spawn_arrow` `third_party/minecraftoss/entities/src/world.rs:3339`; `arrow_landed` `world/game_events.rs:57`). Mob projectiles do not:
  - witches: `spawn_potion` after the throw (`world/witches.rs:380`), the break in `tick_potion` (`:411-427`, beside e2b's `projectile_block_hits` line `:424`);
  - fireballs and wither skulls: E3-7's `fireball-trails` (e3h) already posts their `projectile_shoot` on the first tick, with `HasBeenShot` saved, so they need no shoot line here (E3-0, "Projectile shoot events"). Their hit `fireball_hit` (`world/fireballs.rs:238`, e2b's line `:245`; `FireballHit { location, entity, block }` `fireball.rs:84`) posts no landing;
  - shulker bullets: `fireball-trails` posts their shoot too; `shulker_bullet_hit` (`:274`, e2b's line `:282`) posts no landing;
  - llama spit: `spawn_llama_spit` after the spit (`world/equines.rs:624`), the hit in `tick_llama_spit` (`SpitHit::Block(pos, at)` since e2b, `:704-715`).
  - The `spawn_*` functions also load saved projectiles (`crates/minecraft_terrain/src/server_mobs.rs:1167-1235`), so a shoot line there would post again on every load.
- An arrow a creative or unattackable player turns away (`accepted` false, `world.rs:6843-6873`) plays no hit sound, so `arrow_landed` (`:6879`) never runs.
- Dragon fireballs: `tick_dragon_fireballs` (`crates/minecraft_terrain/src/server.rs:3274-3310`) bursts with no game event; shot at `:3243`.
- Wind charges: `ServerSim::wind_burst` (`server.rs:2387-2388`) posts `explode` with no source; its caller (`:2780-2793`) has the burst's `owner` (a player, or `MOB_THROWER`) and `shooter` (a breeze).

#### Vanilla 26.3 (javap)
- `Projectile.tick`: while `!hasBeenShot`, `gameEvent(PROJECTILE_SHOOT, getOwner())` (at the projectile, from its owner), then `hasBeenShot = true` (saved `HasBeenShot`).
- `Projectile.onHit`: an entity hit calls `onHitEntity`, then `level.gameEvent(PROJECTILE_LAND, hit.getLocation(), Context.of(this, null))` (also when an arrow bounces off a failed hurt); a block hit posts it at the block with its state. Only a deflection in `hitTargetOrDeflectSelf` skips `onHit`. `AbstractThrownPotion`, `LargeFireball`, `SmallFireball`, `WitherSkull`, `DragonFireball`, `ShulkerBullet` and `LlamaSpit` all call `super.onHit` first.
- `AbstractWindCharge.explode` → `Level.explode(this, ...)`: `ServerExplosion.explode` posts `EXPLODE` from the charge; listeners read its owner as the projectile owner (`tryGetPlayer`, the warden).

#### Plan
1. **"Minecraft mob shots make landing vibrations, and witch potions and llama spit throw ones"**
   - A `projectile_landed(owner: Option<u64>, pos, affected)` beside `arrow_landed` in `world/game_events.rs` (block hits at the block centre with its state, entity hits at the location, `owner` as projectile owner, `mob: None`).
   - Shoot lines only for witch potions (`witches.rs:380`) and llama spit (`equines.rs:624`, one line in M3's file), at the shoot sites and never in `spawn_*`, with the shooter as source like `spawn_arrow`. Fireballs, wither skulls and shulker bullets get **no** shoot line here: `fireball-trails` (e3h) already posts theirs on the first tick, and a second line would post `projectile_shoot` twice per shot.
   - Land lines beside e2b's `projectile_block_hits` lines (in the `ProjectileBlockHit` shape target-blocks gave them) in `tick_potion`, `fireball_hit`, `shulker_bullet_hit` and `tick_llama_spit` (also for entity hits).
   - `world.rs:6873`: when `accepted` is false, `arrow_landed` at the hit location.
2. **"Minecraft dragon fireballs make vibrations and wind charge bursts are heard as their thrower's"**
   - `dragon_fireballs.shoot` (`server.rs:3243`) posts `projectile_shoot` and the burst loop (`:3300-3309`) `projectile_land` (block with state, or location), through `Level::projectile_game_event` with no owner (the dragon is no level or entity-world source).
   - `wind_burst` gains the burst's owner: `EventSource::Player(owner)` (not `MOB_THROWER`) or `EventSource::Mob(shooter)`, posted with `projectile_game_event(EXPLODE, ..)`. Its test callers pass `None`.
- The ominous trial spawner's dispensed arrows and fire charges (`server.rs:3805-3825`) are W's w3 (ominous item spawner): no line here.

#### Tests
- entities: `a_witch_potion_shoots_from_the_witch_and_lands_where_it_breaks`, `ghast_blaze_and_wither_shots_land_with_one_shoot_from_their_first_tick` (over a whole flight, exactly one `projectile_shoot`, the one `fireball-trails` posts, and one `projectile_land`), `a_shulker_bullet_lands_and_shoots_only_once`, `llama_spit_lands_on_a_block_with_its_state`, `an_arrow_a_creative_player_turns_away_still_lands`. `fireball-trails`' own test already covers a loaded fireball posting no second shoot.
- server: `a_dragon_fireball_shoots_and_lands`, `a_players_wind_charge_burst_makes_a_shrieker_shriek`, `a_breeze_charge_burst_has_the_breeze_as_owner`.

#### Owner's steps (survival, Normal, the sensor rig; iron armour)
1. `/summon minecraft:shulker ~-2 ~ ~6` (beside the dust, 2 blocks from the sensor). Walk crouched to 8 blocks behind the start (12 from the sensor, within the shulker's 16), stand up and stand still: each time it shoots, the sensor lights 3; a bullet that hits the floor near the sensor lights 2. **e3s12_shulker_shots** `/difficulty peaceful`, then `/difficulty normal`.
2. `/fill ~-2 ~ ~6 ~-2 ~3 ~6 minecraft:cobblestone` and `/summon minecraft:witch ~-2 ~4 ~6`. Stand at the start: the witch throws potions at you from the pillar, and the sensor lights 3 at each throw. `/difficulty peaceful`, `/difficulty normal`.
3. `/setblock ~-1 ~ ~4 minecraft:sculk_shrieker`. Walk crouched to 12 blocks from the sensor and throw a wind charge at the floor about 3 blocks before it: it bursts, the sensor lights 15 and **the shrieker shrieks** (before: quiet). **e3s13_wind_charge_shriek**

```
IW4L_MINECRAFT_KIT=minecraft:wind_charge*16,minecraft:sculk_shrieker*2,minecraft:iron_helmet,minecraft:iron_chestplate,minecraft:shield,minecraft:golden_apple*8,minecraft:milk_bucket,minecraft:milk_bucket,minecraft:cobblestone*64,minecraft:cooked_beef*32
```

---

### Group `sculk-mob-events`

**Gaps:** sculk-13 (bats only), sculk-14 (items; a mob's bounce is M2's m2e), sculk-15, sculk-18 (generic interact, wolf shake, ravager roar) (`release-4-tests.md:787`). **Waits for W's w3:** its leash events sit in the same `mob_actions::interact` path.

#### Cause
- Flaps: `Body::movement_emission` posts `flap` only with a `flap_speed` (`third_party/minecraftoss/entities/src/movement.rs:204`, `:211-214`), set only by chickens. `Bat::is_flapping` (`third_party/minecraftoss/entities/src/bat.rs:49`) is never given to the body; the bat tick is `world.rs:6160-6203`.
- Bounces: `Body::restitute` (`movement.rs:670-699`) belongs to M2 this round: m2e's `sulfur-cubes-blocks` rewrites it with `BodyPhysics` and posts `bounce` on a block bounce there (`track-m2.md:1770-1772`), so this group leaves the mob bounce to M2. The level entities' `restitute` (`third_party/minecraftoss/world/src/level/entity.rs:1132-1162`, which records `unsupported` at `:1157-1159`) post nothing.
- Hurt items: `burning.rs:169` `hurt_item` (callers `item_fire_tick` `:186`, `apply_block_actions` `:308`; reached from `entity.rs:802` and `item_burn_effects` `burning.rs:399-406`) and the blast (`third_party/minecraftoss/world/src/level/explosion.rs:336-341`) lower `health` with no event.
- Interactions: `ServerSim::mob_action` (`server.rs:565-598`) runs `mob_actions::interact` (`crates/minecraft_terrain/src/mob_actions.rs:156`; the leash branch returns at `:164-180`; `MobOutcome` `:28`) and posts nothing. The wolf's shake (`world/wolves.rs:217-219`) and the ravager's roar (`world/ravagers.rs:377-438`) post nothing.

#### Vanilla 26.3 (javap)
- `Entity.applyMovementEmissionAndPlaySound`: when no step is due or the block at `getOnPos` is air, and that block is air, `processFlappingMovement` posts `FLAP` when `isFlapping()`. `Bat.isFlapping` = `!isResting() && tickCount % 10 == 0`; `Bat.getMovementEmission` is `EVENTS`.
- `Entity.restituteMovementAfterCollisions`: a landing with bounciness above 0 (non-living × 0.8) and not `#suppresses_bounce` calls `Block.bounceOn`, then `gameEvent(BOUNCE)`, then `setDeltaMovement`.
- `ItemEntity.hurtServer`: after its checks, `markHurt`, `health -= amount`, `gameEvent(ENTITY_DAMAGE, source.getEntity())` at the item (fire and lava: no entity; a blast: its indirect source, e.g. the TNT's igniter), then discard at 0.
- `Mob.interact`: `ENTITY_INTERACT` from the player when `checkAndHandleImportantInteractions` or `mobInteract` consumed it; a leash (`Entity.interact`) returns before, with its own events. `Player.interactOn` posts it at the entity when `ItemStack.interactLivingEntity` consumed it. `Wolf.tick`: `ENTITY_ACTION` with `WOLF_SHAKE` when the shake starts. `Ravager.roar`: `ENTITY_ACTION` after the knockback loop.

#### Plan
1. **"Minecraft bats' wings, bouncing items and burning items make vibrations"**
   - `Body` gains `flapping: bool` beside `flap_speed`; the bat sets it each tick from `is_flapping` (with the tick count vanilla sees during the move: `ServerLevel.tickNonPassenger` counts it up first); `movement_emission` posts `flap` when it is set and the on-pos block is air.
   - The level `restitute` (`world/src/level/entity.rs`, which M2 does not touch) replaces its `unsupported` push with `game_event_from(bounce, e.pos, Entity(e.id))`. `Body::restitute` is not edited: M2's m2e posts the mob bounce there (E3-0, M2's hooks). If m2e has merged and its restitute posts no `bounce` for a plain mob on a slime block, log it for M2 instead of adding a second line.
   - Items: `base_tick` (`entity.rs:802`) and `item_burn_effects` post `entity_damage` (no source) when the item's health dropped; the blast posts it with `ex.owner` as player source right after `data.health` drops. A mob's blast reaches the level without its mob, so its item hurts have no source (log it).
2. **"Minecraft mobs a player uses, shaking wolves and roaring ravagers make vibrations"**
   - `MobOutcome` gains `leash` (set in the leash branch); `mob_action` posts `entity_interact` at the mob from `PLAYER_TARGET + player` when an interaction was handled and the leash did not take it. Family lines M1-M3 add (armadillo brush, sulfur cube) stay as well: vanilla posts both.
   - The wolf's shake start pushes `entity_action` into its body queue; `ravager_roars` posts it before its entity event 69.

#### Tests
- entities: `a_flying_bat_flaps_every_ten_ticks_and_a_resting_bat_never`, `a_wolf_starting_to_shake_posts_entity_action`, `a_ravager_roar_posts_entity_action`.
- world level: `an_item_bouncing_on_slime_posts_bounce`, `an_item_hurt_by_fire_lava_or_a_blast_posts_entity_damage` (the blast with its igniter).
- server: `feeding_a_cow_posts_entity_interact_from_the_player`, `tying_a_lead_posts_no_generic_interact`.

#### Owner's steps (survival, Normal, the sensor rig)
1. `/summon minecraft:bat ~ ~2 ~6`: while it flutters near the sensor, the lamp keeps relighting with 1 dust (its wing beats; before: dark). **e3s14_bat_flaps**
2. `/setblock ~-1 ~-1 ~2 minecraft:lava` (lava in the floor 2 blocks from the sensor). Crouched at the start, press Q toward it so a cobblestone falls in: it burns away with a hiss and the sensor lights 7. **e3s15_item_burns**
3. Hatch a cow 2 blocks beside the start and wait until it stands still. Feed it wheat: hearts, the sensor lights 6. Wait 2 s and milk it with a bucket: 6 again.
4. `/setblock ~1 ~-1 ~2 minecraft:water`. Hatch a wolf, tame it with bones, lead it through the water hole and let it stop beside you on dry stone: it shakes off the water with a shake sound and the sensor lights 4 (if its steps keep the lamp lit, repeat).
5. The bounce shares frequency 2 with the landing, so the rig cannot tell them apart; its tests cover it.

```
IW4L_MINECRAFT_KIT=minecraft:bat_spawn_egg*2,minecraft:cow_spawn_egg*2,minecraft:wheat*16,minecraft:bucket,minecraft:wolf_spawn_egg*2,minecraft:bone*16,minecraft:slime_block*4,minecraft:cobblestone*64,minecraft:cooked_beef*32
```

---

### Group `sculk-deaths`

**Gaps:** sculk-38, sculk-40, sculk-45, sculk-47 (`release-4-tests.md:796-797`, `:804`).

#### Cause
- `EntityWorld::take_game_events` (`world/game_events.rs:29-46`) reads `DamageState.damaged` only for mobs still in `mob_ids`; a killing blow's `entity_damage` is lost when the mob is gone. `take_deaths` posts only `entity_die` (`world.rs:4897-4899`). `MobDeath` (`:637`) is built only in `death_of` (`:663-667`), which holds the `DamageState`.
- `prune_riding` (`world/riding.rs:657-677`) drops the riders of a vehicle that left the stores (player riders only go to `player_dismounts`) and posts nothing; the vehicle's position is already gone. Callers: despawn `world.rs:6044`, chunk removal `:6089`, `:8317`, and a filtered copy `:5878`.
- The dragon: `DragonEntity::tick_death` sets `removed` (`crates/minecraft_terrain/src/ender_dragon.rs:1250-1281`); the server records `dragons_killed` and drops it (`server.rs:3255-3256`) and awards its last experience (`:3258`). No `entity_die`, no catalyst (`catalyst_takes` `:1941`).
- `LevelTrials::spawn_mob` posts `entity_place` with no source before the mob has an id (`crates/minecraft_terrain/src/trial_spawner.rs:1370-1373`); the mob joins in `take_spawned` (`server.rs:1353-1357`); `tick_spawners` returns a `TrialOutcome` (`trial_spawner.rs:1078`, call `server.rs:3820`).

#### Vanilla 26.3 (javap)
- `LivingEntity.actuallyHurt` posts `ENTITY_DAMAGE`, then `LivingEntity.die` posts `ENTITY_DIE`, in the same tick and place: ordinary sensors keep 15, a calibrated sensor set to 7 keeps the 7.
- `Entity.removeVehicle` posts `level.gameEvent(rider, ENTITY_DISMOUNT, vehicle.position)` when the rider's removal reason is null or destroys it; `Entity.setRemoved` makes the passengers stop riding.
- `EnderDragon.tickDeath` at 200: `ExperienceOrb.award` (last share), `EnderDragonFight.setDragonKilled`, `remove(KILLED)`, `gameEvent(ENTITY_DIE)` (also `EnderDragon.kill`). `SculkCatalystBlockEntity$CatalystListener` blooms for any living death not yet consumed; the dragon's reward is 0, so no cursors, but it blooms.
- `TrialSpawner.spawnMob` posts `level.gameEvent(entity, ENTITY_PLACE, pos)` with the new mob as source.

#### Plan
1. **"Minecraft killing blows and the ender dragon's death make their vibrations, and the dragon blooms catalysts"**: `MobDeath` gains `damaged`, taken from the `DamageState` in `death_of`; `take_deaths` posts `entity_damage` before `entity_die` when it is set (whichever of the two paths runs first takes the flag, so it posts once). The server collects the removed dragons' positions and, after their experience, posts `entity_die` (no source) and calls `catalyst_takes(position, 0)`.
2. **"Minecraft riders thrown off a dying mount and trial spawner mobs are heard as themselves"**: `prune_riding` keeps the position of each vehicle with riders from its last call and posts `entity_dismount` from each surviving rider at the lost vehicle's last position; the filtered copy (`:5878`) gets no such map and posts nothing. `TrialOutcome` gains `placed: Vec<(u128, BlockPos)>` instead of the sourceless post; after `take_spawned` the server posts `entity_place` with `EventSource::Mob(mob_with_uuid(uuid))` (this replaces the line W's merge added).

#### Tests
- entities: `a_killing_blow_posts_entity_damage_before_entity_die`, `a_rider_of_a_mount_that_died_posts_entity_dismount_where_the_mount_was`, `a_filtered_copy_posts_no_dismounts`.
- server: `the_dragons_death_posts_entity_die_and_blooms_a_catalyst_with_no_cursors`, `a_trial_spawner_mob_is_the_source_of_its_entity_place`.

#### Owner's steps (survival, Normal, the calibrated rig with 9 dust: it hears only 7)
1. Hatch a cow 2 blocks beside the calibrated sensor and punch it once: the lamp lights (a hurt). Wait 2 s. Hatch a second cow and kill it with one hit of the Sharpness V sword: **the lamp lights** for the killing blow (before: dark; the death's 15 is filtered out). **e3s16_killing_blow**
2. Break the lever, `/fill ~ ~ ~14 ~ ~ ~15 minecraft:redstone_wire` and place the lever after the new dust (11 dust: it hears only 5). `/summon minecraft:ravager ~2 ~ ~4 {NoAI:1b,Passengers:[{id:"minecraft:pillager",NoAI:1b}]}`. Kill the ravager with the sword (about 10 hits): when it dies the pillager drops to the floor and **the lamp lights** (its dismount; before: dark). Kill the pillager. **e3s17_rider_dropped**
3. The dragon dies about 20 blocks above where its death starts, and a trial spawner's mob as source matters only to a warden: tests only.

```
IW4L_MINECRAFT_KIT=minecraft:calibrated_sculk_sensor,minecraft:redstone*16,minecraft:lever*2,minecraft:redstone_lamp,minecraft:cow_spawn_egg*4,minecraft:diamond_sword+sharpness=5,minecraft:golden_apple*4,minecraft:cobblestone*64,minecraft:cooked_beef*16
```

---

### Group `sculk-saves`

**Gaps:** sculk-30, sculk-31, sculk-32, sculk-33 (`release-4-tests.md:793`).

#### Cause
- The listener tag is written only when a vibration is picked or arrives: `Vibrations::tick` (`crates/minecraft_terrain/src/vibrations.rs:463-506`) calls `user.changed` only then, so a save during the flight keeps the full `event_delay` (`Listener::to_tag` `:245-253`). `Hearing::changed` (`server.rs:4251`) writes the block entity or the warden's mob tag. `Command::Save` (`server.rs:5112-5115`) only saves entities; the client's save already sends `Save` and waits for the block-entity output (`crates/minecraft_survival/src/entities.rs:1339-1350`).
- `vibrations.rs:488-496` sends the reloaded ripple once and clears `reload_particle` whether or not a player can see it; `User::particle` (`:279`) returns nothing; `Hearing::particle` (`server.rs:4247`) only pushes a burst.
- `Info::from_tag` (`vibrations.rs:104-117`) restores only `source_uuid` and `owner_uuid`. The shrieker branch of `Hearing::receive` (`server.rs:4232-4237`) uses `player_behind(info.source, info.owner)`, which ignores the UUIDs; the sensor branch (`:4244`) passes `info.source.or(info.owner).map(level_source)`, `None` after a load, so its `tendrils_clicking` has no source and a shrieker beside it stays quiet. `Hearing::living` (`:4171-4184`) already resolves a UUID to a player or a mob.

#### Vanilla 26.3 (javap)
- `SculkSensorBlockEntity.saveAdditional`, `SculkShriekerBlockEntity.saveAdditional` and `Warden.addAdditionalSaveData` encode `VibrationSystem.Data` on every save, with `event_delay` = the current `travelTimeInTicks`.
- `VibrationSystem$Ticker.tryReloadVibrationParticle` clears `shouldReloadVibrationParticle` only when `ServerLevel.sendParticles` reached a player (count above 0), so it retries every tick.
- `VibrationInfo.getEntity` / `getProjectileOwner` resolve the saved UUIDs with `ServerLevel.getEntity`; `VibrationSystem$Ticker.receiveVibration` passes them to the user: `SculkShriekerBlockEntity$VibrationUser.onReceiveVibration` → `tryGetPlayer`, `SculkSensorBlockEntity$VibrationUser.onReceiveVibration` → `SculkSensorBlock.activate(entity, ..)` → `gameEvent(entity, SCULK_SENSOR_TENDRILS_CLICKING, pos)`.

#### Plan
1. **"Minecraft vibrations in flight are saved with the ticks they have left"**: `Vibrations::save_in_flight(&mut impl User)` calls `changed` for every listener (blocks and wardens) with a current vibration; `Command::Save` runs it through a `Hearing` (as `server.rs:1053-1054` builds one) before `save_all_entities`. A chunk unloaded mid-flight still keeps the pick's delay (log it).
2. **"Minecraft vibrations loaded from a save still reach shriekers through the player who made them"**: in `Hearing::receive`, a missing `source`/`owner` is resolved from `source_uuid`/`owner_uuid` (players through `uuid_of(PLAYER_TARGET + id)`, mobs through `mob_with_uuid`, as `living` does) before the shrieker and sensor branches.
3. **"A reloaded Minecraft vibration ripple waits until a player is near to be drawn"**: `User::particle` returns whether a player is within 32 blocks of the ripple's start; the reload clears its flag only then; the pick ignores the result.

#### Tests
- server: `a_vibration_saved_in_flight_keeps_the_ticks_it_has_left` (sensor block entity and warden tag), `a_loaded_player_vibration_makes_a_shrieker_shriek`, `a_loaded_vibration_clicks_a_sensor_for_its_player_and_the_shrieker_beside_it_shrieks`.
- `vibrations::tests::a_reloaded_ripple_waits_for_a_viewer`.

#### Owner's steps (survival, the sensor rig)
1. `/setblock ~-1 ~ ~4 minecraft:sculk_shrieker`. Walk crouched to 3 blocks behind the start (7 from the sensor), stand up, take one step and **at once** press Esc and Save and Quit to Title. Load: if the save caught the vibration in flight, a ripple finishes its way right after the world appears, the sensor clicks and **the shrieker shrieks for you** (before: the click came up to 0.8 s late and the shrieker stayed quiet). If nothing happens, the save came too late: try again; the tests check the exact timing. **e3s18_saved_vibration**

```
IW4L_MINECRAFT_KIT=minecraft:sculk_shrieker*2,minecraft:cobblestone*64,minecraft:cooked_beef*16
```

---

### Group `sculk-blocks`

**Gaps:** sculk-6, sculk-7, sculk-5 (`release-4-tests.md:786`). Cakes and player composting are features the fork lacks; the game event comes with each.

#### Cause
- **Cake:** only the comparator reads `bites` (`third_party/minecraftoss/world/src/level/redstone.rs:506`); `handles_use` and `use_block_facing` (`redstone.rs:997-1005`, `:1013-1071`, falling back to `harvest` `:1069`) have no cake or candle cake; the survival use path (`crates/minecraft_survival/src/actions.rs:827-862`: `bell`, then `used` filtered at `:851-861` through `entities.use_block` → `ServerHandle::use_block` `server.rs:4675` with the state table `:4618-4621`) gives no food; `ItemOnBlock` (`server.rs:31`, `item_on_block_by` `:3080`) has no candle on cake.
- **Candles:** no empty-hand use; only wind (`wind_trigger`) and splashes put them out; the sound constant is `crates/minecraft_terrain/src/splash.rs:9` `CANDLE_SOUND`; candles and candle cakes are recognised in `flint.rs:36`.
- **Composter:** farmers compost (`third_party/minecraftoss/entities/src/villager_brain/behaviors.rs:2245-2262` `insert_item`, `:2265-2282` `extract_produce`) but post nothing (their body is `ctx.me.body`, whose `game_events` queue drains with the villager as source). Players cannot use a composter at all; no fill level event (`crates/minecraft_terrain/src/level_events.rs`, `LAVA_FIZZ` 1501 at `:14`); the 7-to-8 tick is `third_party/minecraftoss/world/src/level/mod.rs:880-885`.

#### Vanilla 26.3 (javap)
- `CakeBlock.useItemOn`: a `#candles` item on a cake with 0 bites → `consume(1)`, `cake.add_candle` sound, `CandleCakeBlock.byCandle`, `gameEvent(player, BLOCK_CHANGE, pos)`; else `TRY_WITH_EMPTY_HAND`. `CakeBlock.eat`: `PASS` unless `Player.canEat(false)`; `eat_cake_slice` stat, `FoodData.eat(2, 0.1)`, `gameEvent(player, EAT, pos)`, bites + 1, or at 6 `removeBlock` and `gameEvent(player, BLOCK_DESTROY, pos)`. No sound.
- `CandleCakeBlock.useItemOn`: flint and steel or a fire charge pass; an empty hand on the candle (`candleHit`: hit y above 0.5) while lit → `AbstractCandleBlock.extinguish`. `useWithoutItem`: `CakeBlock.eat(Blocks.CAKE.defaultBlockState())`, and when eaten `dropResources` (the candle drops).
- `CandleBlock.useItemOn`: empty hand, `mayBuild`, lit → `extinguish`. `AbstractCandleBlock.extinguish(player, ..)`: `setLit(false)`, `SMOKE` at each `getParticleOffsets` point (a server `addParticle`, so only the acting client draws it), `block.candle.extinguish` (1, 1), `gameEvent(player, BLOCK_CHANGE, pos)`.
- `ComposterBlock.useItemOn`: level below 8 with a `DataComponents.COMPOSTABLE` item: below 7, `insertItem` → `addLayer` (the item's `Compostable.layers` provider; on a layer: `setBlock`, `gameEvent(BLOCK_CHANGE, pos, Context.of(entity, newState))`, a tick in 20 at 7), level event 1500 (data 1 if it rose), consume 1; at 7 it only succeeds. `useWithoutItem` at 8: `extractProduce` (bone meal popped, `empty` → `gameEvent(BLOCK_CHANGE)` and `block.composter.empty`). `ComposterBlock.handleFill` (1500): `composter.fill_success` or `composter.fill` and `COMPOSTER` particles. `Items.<clinit>` gives 122 items `Item$Properties.compostable(ContextIntProviders.COMPOSTABLE_LOW / LOW_MEDIUM / MEDIUM / MEDIUM_HIGH / ALWAYS_ADD_ONE)`; the providers are data: `data/minecraft/context_int_provider/compostable/*.json` (a `number_dispatcher` that gives 1 on an empty composter, else a `weighted_list` of 1 and 0).

#### Plan
1. **"Minecraft cakes are eaten a slice at a time and take candles"**: survival side (`actions.rs`, beside `bell`): a candle on an uneaten cake sends a new `ItemOnBlock` arm beside `Ignite`; an empty-hand or non-candle use on a cake or candle cake (lower half) uses it only when the player can eat (food below 20, or invulnerable), adds 2 food and 0.4 saturation (`FoodData.eat(2, 0.1f)`: 0.1 is the saturation modifier, and `FoodConstants.saturationByModifier` gives 2 × 0.1 × 2; the fork's `FoodData::eat(2, 0.4)` in `player/src/survival.rs:124` takes the saturation itself, not the modifier) and sends `use_block`; a full player's click falls through to placing. Level (a new vendored `level/cake.rs`): bite or remove with `eat` / `block_destroy` from the acting player; a candle cake becomes a cake with one bite and drops its candle; `handles_use` true for both. The stat if the fork counts it.
2. **"Minecraft candles are put out by hand"**: an empty-hand use (not in adventure) on a lit candle, or on a lit candle cake's upper half: `extinguish` in `cake.rs` (lit false, `CANDLE_SOUND`, `block_change` from the acting player); smoke at the wick offsets in the acting player's window only (vanilla's server `addParticle` draws nothing); if a friend's own window can't get it, log it.
3. **"Minecraft composters take compost by hand and farmers' composting is heard"**: a new `crates/minecraft_terrain/src/compost.rs` with the 122 item → provider names (javap `Items.<clinit>`), the providers read at runtime from the 26.3 data pack. An `ItemOnBlock` arm (beside `BoneMeal`) adds a layer by the provider with the level random, posts `block_change` from the player only when a layer was added, schedules the tick at 7, sends level event 1500; the survival side consumes one item below level 7. At 8 an empty-hand use pops bone meal, empties it with its sound and `block_change`. `level_events.rs` gains `COMPOSTER_FILL` 1500 beside `LAVA_FIZZ`. Farmers: one `ctx.me.body.game_events` line after each block push in `insert_item` and `extract_produce`.

#### Tests
- world level: `a_cake_loses_a_slice_per_bite_and_its_last_bite_removes_it`, `a_candle_on_an_uneaten_cake_makes_a_candle_cake`, `eating_a_candle_cake_drops_its_candle`, `a_lit_candle_goes_out_by_hand`.
- survival: `a_full_player_cannot_eat_cake`, `a_slice_gives_two_food_and_four_tenths_saturation`.
- `compost::tests::the_first_layer_is_certain_and_later_ones_follow_the_provider`, `a_ready_composter_gives_bone_meal_and_empties`; villager: `a_farmer_composting_posts_block_change_from_itself`; `level_events::tests::composter_fill_plays_success_or_plain_fill`.

#### Owner's steps (survival, Normal, the sensor rig; `/effect give @s minecraft:hunger 5 99` makes you hungry, as in `release-4-tests.md:113`)
1. Place a cake 2 blocks beside the start. Get hungry and use it with an empty hand: a slice is gone, one drumstick fills, the sensor lights 8. With a full bar the use does nothing. Eat it down: the last slice takes the cake away and the sensor lights 12. **e3s19_cake**
2. Place a cake and use a candle on it: the candle stands on it (a soft sound), 11. Light it with flint and steel, then use the flame with an empty hand: it goes out with a hiss and a puff of smoke, 11. Get hungry and use the cake part: a slice is eaten and the candle drops, 8.
3. Put 4 candles into one block, light them and use them with an empty hand: all go out, smoke at each wick, 11. In `/gamemode adventure` the same use does nothing; `/gamemode survival`.
4. Place a composter 2 blocks from the sensor. Use wheat seeds on it: the first always raises it (a fill sound, green specks, 11); later seeds rise only now and then (a plainer sound, no dust when not). Pumpkin pies always raise it. At level 7 it turns to bone meal a second later; use it with an empty hand: bone meal pops out, 11. **e3s20_composter**

```
IW4L_MINECRAFT_KIT=minecraft:cake,minecraft:cake,minecraft:cake,minecraft:candle*16,minecraft:white_candle*4,minecraft:flint_and_steel,minecraft:composter*2,minecraft:wheat_seeds*64,minecraft:pumpkin_pie*16,minecraft:oak_leaves*64,minecraft:cobblestone*64,minecraft:cooked_beef*16
```

---

### Group `sculk-pots-shelves`

**Gaps:** sculk-8, sculk-9 (`release-4-tests.md:786`). Both are features first; the game event comes with each.

#### Cause
- A decorated pot is only placed and drawn (`crates/minecraft_terrain/src/special_models.rs:1082` `push_decorated_pot`) and has a block entity type (`third_party/minecraftoss/core/src/block_entity.rs:229-230`); no insert, no wobble. Animated block entities follow the bell swing (`crates/minecraft_terrain/src/lids.rs:137` `BellSwing`, commit 8f39a9ae).
- A chiseled bookshelf is only a block entity type (`block_entity.rs:178`); nothing puts in or takes out books or sets `slot_N_occupied`. The comparator reads states only (`redstone.rs:502-530` `state_analog_output`).

#### Vanilla 26.3 (javap)
- `DecoratedPotBlock.useItemOn`: an empty pot, or the same item and components below a full stack: `wobble(POSITIVE)`, `ITEM_USED`, one item moves in, `decorated_pot.insert` at pitch `0.7 + 0.5 × count / max`, 7 `DUST_PLUME` at (x + 0.5, y + 1.2, z + 0.5), `gameEvent(player, BLOCK_CHANGE, pos)`. `useWithoutItem`: `wobble(NEGATIVE)`, `decorated_pot.insert_fail` (1, 1), `gameEvent(player, BLOCK_CHANGE, pos)`. Its comparator reads the contents.
- `ChiseledBookShelfBlock.useItemOn` (a `#bookshelf_books` item on an empty slot) → `addBook`; `useWithoutItem` on a filled slot → `removeBook`. The slot comes from `SelectableSlotContainer.getHitSlot` (2 rows, 3 columns, on the front face only). Sounds `chiseled_bookshelf.insert` / `pickup` (`_enchanted` for enchanted books). `ChiseledBookShelfBlockEntity.updateState` sets the slot states, `last_interacted_slot` and posts `gameEvent(BLOCK_CHANGE, pos, Context.of(state))`; `removeBook` also posts it from the player. Comparator: last slot + 1.

#### Plan
1. **"Minecraft decorated pots take items and wobble"**: a new vendored `level/pots.rs`: insert and fail with sounds, dust plume, `block_change` from the acting player, the block entity `item` saved, the comparator, and the contents dropped when it breaks. The survival side sends a new command carrying the stack (beside `ItemOnBlock` users) and consumes one item. A `LidKind::Pot` wobble beside the bell's in `lids.rs`, driven by the block event.
2. **"Minecraft chiseled bookshelves hold books"**: a new vendored `level/bookshelves.rs`: the hit slot from the face hit point, add and remove with sounds, slot states, `last_interacted_slot`, `block_change` as vanilla, the comparator, and the books dropped when it breaks.

#### Tests
- `a_pot_takes_the_same_item_up_to_a_stack_and_its_pitch_rises`, `an_empty_hand_wobbles_the_pot_the_other_way`, `a_broken_pot_drops_its_item`, `the_pot_comparator_reads_its_fill`.
- `the_hit_point_picks_the_bookshelf_slot`, `books_go_in_and_out_and_set_the_slot_states`, `the_comparator_reads_the_last_slot`, `a_broken_shelf_drops_its_books`.

#### Owner's steps (survival, Normal, the sensor rig)
1. Place a decorated pot 2 blocks from the sensor. Use diamonds on it one at a time: each goes in, the pot wobbles, a clink rising in pitch and a puff of dust; the sensor lights 11. An empty-hand use wobbles it the other way with a dull knock: 11. Break it: the diamonds drop with it. **e3s21_pot**
2. `/setblock ~1 ~ ~2 minecraft:chiseled_bookshelf[facing=south]` and face it. Aim at each of the six slots and use a book: each goes into the slot you aim at, 11. An empty-hand use on a filled slot takes that book back. Put a comparator behind it: it reads the last slot used (1 to 6). Break it: the books drop. **e3s22_bookshelf**

```
IW4L_MINECRAFT_KIT=minecraft:decorated_pot,minecraft:decorated_pot,minecraft:chiseled_bookshelf*4,minecraft:book*16,minecraft:diamond*64,minecraft:comparator*2,minecraft:redstone*16,minecraft:iron_pickaxe,minecraft:cobblestone*64
```

---

### Left for later
- **sculk-42** (a bullet hitting a player makes no landing): see "left".
- **sculk-57** (the warden's glowing layers do not flash red): see "left".
