# Track D0: the shared mob registry

## The owner's request

> D. Finish track D: the rest of d5 (trial chambers and trial spawners, lodestones), d3 (deep dark, warden, sculk), and d4 (the remaining mobs), with d4 split into two or three tracks by mob group. **Land the shared changes (mob list, spawn tables, render registry) first so the later merges stay small.**

Round 10 splits that request into track D (trial chambers, vaults, breezes, lodestones: `track-d.md`), track W (deep dark and the warden: `track-w.md`), tracks M1-M3 (the remaining mobs by group) and this track. **D0 is the "shared changes first" step.** It builds one registry that every new mob family plugs into, proves it with one small family (silverfish and endermites), builds the flying navigation that M1 and M3 share, and writes the round 10 mob list.

Why it is needed: in round 9 one family touched about 55 match or chain sites in about 15 files (`docs/round9/mobs.md`, "The mob pattern"). Nearly every site was "add a line after the newest family", so two tracks adding families at the same time conflict at every site. **After D0, a new family touches only its own new files plus one registration line per registry, and those lines sit in files that belong to one track.** Five mob tracks then merge without touching each other.

**Base and order.** Branch `track-d0`, worktree `C:\Dev\Soscraft\mc-wt\track-d0`, target dir `D:\iw4l-wt\track-d0` (or `C:\iw4l-wt\track-d0`, see `decisions.md`), base `minecraft-survival` (`aa1f1413`; the code is `6e0e0573`). Paths and line numbers below are as of `6e0e0573` (checked out in `C:\Dev\Soscraft\mc-wt\track-d`). Tracks E1, E2, F, D (trial spawners) and W (segment w1) run at the same time, so re-check every line before editing. **D0 merges into `minecraft-survival` before** D's breeze groups, W's segment w2, M1, M2, M3 and F's boss (if F uses the registry). Those branches merge `minecraft-survival` in before their mob groups start. Track log: `docs/round10/track-d0-log.md` (decisions go there and are copied into `docs/round10/decisions.md` at the merge).

Vanilla facts below were checked with javap on the 26.3 `client.jar` and the 26.3 data pack this round, unless marked "javap during the work". No Mojang code or data is copied: numbers and behaviour only.

---

## D0-0. Rules for this track

- One commit per item, plain-English subjects, no code comments of any kind, minimal diffs to existing files (never reformat a line D0 does not need to change), new files pass `rustfmt --check` for their crate's edition on the staged content, no Mojang files, cargo only through `scripts\cargo-slot.ps1`.
- **Existing families keep their code.** D0 never converts zombies, withers, cats or any other existing store to the registry. It only adds one generic arm at each shared site. The only behaviour changes to existing families are the two listed fixes (D0-5.4, the sleep check; D0-7.1, endermen hunting endermites) and the optional wither flight (D0-6.2).
- **Vanilla 26.3 formats.** Registered mobs are saved in the vanilla entity format through the existing save path (`mob_tags_where` merges into the original tag), keyed by UUID like every other mob.
- **Host owns everything.** Friends run no AI: they mirror a registered mob from its look tag and its per-tick `ServerMob` with `MobShow` (D0-5.2). Nothing keys on player 0.
- **Agreement with track E** (`track-e.md`, "Files shared with the other round 10 tracks"):
  - D0's generic arm goes **at the end of each list or chain** (after the last family, before any catch-all `_ =>` or final `else`). E puts its new arms next to a related one, never at the end, so the two never touch adjacent lines.
  - D0 never edits `thrown.rs`, `splash.rs` or `wind.rs`. Registered mobs reach thrown things through E2's `hit_boxes` and `player_projectile_hurt` once E2's group `ti1` lands. D0 adds its arm in `arrow_targets` (or in E2's `hit_boxes` if E2 merged first), `hurt_by_player` (or E2's `hurt_for_player`), `apply_arrow_hit`, the four `witches.rs` potion sites and `body_of`.
  - D0's generic arm in `mob_actions::interact` is where E1's commit `particles-spit-love` 4 hooks registered families.
  - In `MonsterGoalContext`, D0 appends its field after `pub shulker: ShulkerState` (`monster_ai.rs:571`) and its default after `shulker: ShulkerState::default()` (`:1000`). E's door flag goes next to `can_break_doors`.
- **Protocol.** `minecraft_net::PROTOCOL` is 6 and shipped in round 10 release 1. **D0 does not bump it** (`docs/round10/decisions.md`, "PROTOCOL is bumped only at release time", which overrides every brief that says "bump to 7"). D0's `MobShow` commit (D0-5, commit 2) changes a serialized struct, so the track log gets a "## Protocol" section naming the commit and the struct (`MobShow.family`). Before the next release build, one commit bumps `PROTOCOL` by one and updates the four asserts in `crates/minecraft_net/src/codec.rs` (`:882`, `:912`, `:930`, `:1004`); that is the release's job, not D0's.
- **E2's renamed hurt.** If E2's `ti1` merged first, `hurt_by_player` is `hurt_for_player(id, amount, player, kind, source)`: D0's arm there passes that `kind` as the `Hurt.damage_type` instead of the fixed `"minecraft:player_attack"`. `player_projectile_hurt` wraps `hurt_for_player`, so it needs no arm of its own.
- **Vendored files.** Every new or patched file under `third_party/minecraftoss` goes on the track log's vendored list (patched and added counts), as round 9 did.

---

## D0-1. The shared sites (condensed inventory)

Line numbers are the function start as of `6e0e0573`. "One line" means the list is one long source line, so any two additions collide.

**Vendored `entities/src/world.rs`** (33 sites, all "after the newest family"): `mod`/`pub use` (21-83); `enum EntityKey` (2057) and `EntityKey::id` (2096); `pub enum MobHit` (2137) and `MobHit::id` (2171, one line); `struct EntityWorld` (2487) and `Default` (2634); `blast_target` (2893); `hurt_by_blast` (2992); `blasted_by_player` (3052); `arrow_targets` (3272); `apply_arrow_hit` (3486); `mob_candidates` (4080, what the goal AI and iron golems target); `mob_hits_mob` (4179); `take_deaths` (4432); `entity_type` (4807); `bodies_mut` (4917); `body_mut` (4949); `damage_state` (5012); `damage_state_mut` (5044); `take_sounds` (5115, `resolve_voices`); `positions` (5518); `burning` (5559); `clone_where` (5606, filter and struct literal); `census` (5770); `check_despawn` (5830); `remove_where` (5923); `tick_selected` (5976: the dispatch, the death retain at 8055, the alive-ids chain at 8098 and the `order.retain` match at 8137); `mob_on_ray_except` (8202).

**Vendored `entities/src/world/*.rs`:** `combat.rs` `mob_body` (144), `knock_back` (233), `hurt_by_player` (300, its chain ends in a final `else` for blazes); `witches.rs` `mob_effects_mut` (478), `mob_effects` (562), `heal_mob` (627), `hurt_mob_by_magic` (693); `pushing.rs` `push_body` (156, catch-all one line), `exposed_mut` (193), `pushes_on_local_player` (329) (a kind missing from `push_body` is never filed in sections, `file_in_section` 226); `hazards.rs` one `exposed!` per family (208-249); `fireballs.rs` `fireball_damages_mob` (392: fire immunity by kind through `hazards::fire_immune`, `:401`); `riding.rs` `remove_mobs` (756) and, for mounts, `carries_riders` (224, one line), `passenger_point` (227), `controlling_mob` (259), `controlling_player` (287), `mob_vehicle_attachment` (322, the rider's offset), `vehicle_body` (362), `start_riding` (409), `stop_riding` (436), `order_vehicle` (576, a mob rider steering its mount), `jumpable_by` (663) and `tick_ride_jumps` (668, the jump bar); `villagers.rs` `seen_all` (253); `leashes.rs` `body_of` (111), `leash_yaw` (248), `leash_no_ai` (286), `leash_panicking` (300), `mob_baby` (308), `can_be_leashed` (336), `leash_follow_speed` (842) and `leash_walk` (850) (without the last five a leashed registered mob is never pulled). `fireball_targets` (`fireballs.rs:96`) and `bullet_targets` (`shulker_bullets.rs:174`) walk `order` with `mob_body`, so the generic `mob_body` arm covers them.

**Vendored `entities/src/monster_ai.rs`:** `enum MonsterKind` (36), `goal_names` (70), `target_goal_names` (99), `is_enemy` (364), `enum Prey` (415, closed list of prey kinds), `MonsterGoalContext` fields (one per kind, 477-571) and init (936-1000), the raider check (659, one line), the fall-sacrifice match in `profile` (781, one line), `walk.can_float` (847, one line), follow range (848), goals tuple (909). `lib.rs` `pub mod` lines.

**Vendored `world/src/natural_spawner.rs` and `natural_spawner/spawning.rs`:** `type_info` (271: the size table, `_ => return None` at 324, the one-line `monster` list at 327, the `rules` match at 328), `enum Rules` (216) and `creature()` (254), `finalize_spawn` (896), `enum Group` (1108); `MobCategory::of_type` (97, multi-line category lists), `follow_range_base` (397), `natural_spawn_rules` (730), `summon` (886), `finalize_monster` (1152, catch-all at 1336 calls `note_unsupported` for any kind without an arm). The rule helpers rows need (`dark_enough` 774, `mob_spawn_rules` 820, `difficulty_at` 936, `populate_armor` 946, `roll_enchantments` 974) are private methods of `impl CreatureSpawns` in `spawning.rs`, so a sibling module (`natural_spawner/families`) cannot call them (D0-4). `finalize_spawn` (`natural_spawner.rs:896`, the creature path, also called by chunk-generation spawns at `:628`) has no `SpawnContext` and no `DifficultyInstance`. Many remaining kinds already have rows (rabbit, fox, goat, frog, parrot, polar bear, turtle, camel, armadillo, axolotl, panda, glow squid, zombie horse, camel husk); squid, the fish, dolphin, phantom and silverfish are in `of_type` only; tadpole, guardians, nautiluses, skeleton horse, endermite, bee, allay, sniffer, copper golem, creaking, sulfur cube, happy ghast, warden and breeze appear nowhere.

**`crates/minecraft_terrain`:** `lib.rs` `pub mod` lines; `server_mobs.rs` `SIMULATED_KINDS` (433, used by `/summon` suggestions, `carries` and a server test), `VEHICLES` (489, one line) and `carries` (491), `spawn_saved` (580, `_ => return None` at 1321), `mob_tags_where` (1497), one `<mob>_save.rs` each with a copied `Saved` struct; `mob_tracking.rs` `VOLATILE: [&str; 37]` (15), `MobShow` (65, one field per drawn state, serde wire format), `tracking_range` (285, one-line groups), `look` (314), `shows` (330), `show_on` (592); `client_mobs.rs` `server_mobs` pose push (1196), the swings chain (1230), the head-turn tweaks (1316), `shadow_casters` (1343), keyframe start ticks on `ClientMob`/`MobPose` (366-810); `mesh.rs` `build_internal` texture list (478); `spawner_display.rs` `DRAWN_KINDS` (19), `draw` (196), cage test (919); `server.rs` test `HATCHING_EGGS: [&str; 50]` (7717, names without `minecraft:`) and `assert_eq!(hatched, HATCHING_EGGS)` (7752); `server.rs` test `live_shulkers_stay_far_from_players_while_dormant_monsters_go` (8169, uses the silverfish as a dormant monster); `mob_actions.rs` `interact` (149, `_ => return outcome` at 376; the lead attach and snip at 156 run before the per-kind match and only ask `can_be_leashed`); `mount_inventory.rs` `MountInventory::of` (31) and `still_valid` (198), cow store only; `thrown.rs` (E's, see D0-0).

**`crates/minecraft_survival`:** `entities.rs` `encode` (1987), `decode` (2022), the key constants (2058-2072, `WITHER_KEY = 35`, next free 36, 24 and 25 reserved), `resting_monsters` (1788, missing illagers, vexes, ravagers and withers), `mob_hit` (1882, exhaustive over `PlayerHitKind`); `entity_view.rs` `boxes` (630), `meshes` (829, which also fills `MobMeshes.culled` for back-face-culled models such as the bat's, and the per-kind glowing outline loops at 1108-1121 that draw a mob with the Glowing effect into `out.glow`/`out.glow_cover`), the held-items chain (1350), the per-family particle passes (`ravager_effects` 257, `wither_effects` 278, called from `advance` 150); `death.rs` `MobAttack` (67) and its one-line lists; `commands.rs:147` (`/summon` entity list).

**Hand lists already behind:** `thrown::targets`/`hurt_by` (E2 fixes them) and `resting_monsters` (D0-5, commit 4).

**Kind-keyed lists that already name the new mobs** (no arm needed): `wither::is_undead` (the 26.3 `#undead` tag, phantom and zombie nautilus included), `hazards::fire_immune` (warden included), `sleep::MONSTERS`, `generator` `NOT_IN_PEACEFUL`, `Prey::WolfPrey` (rabbit, fox) through `mob_candidates`.

---

## D0-2. The design

### D0-2.1 Groups, files and hit keys

A **family** is one store of mobs sharing a vanilla class (one or more kinds). A **group** is a registration bucket owned by one round 10 track. D0 creates every group's registration files up front, so a later track only edits its own group's files.

| Group | Track | Hit-key bytes | Families planned this round |
|---|---|---|---|
| `pests` | D0 | 36-39 | `silverfish` 36, `endermites` 37 (39 is the test probe) |
| `trial` | D | 40-43 | `breezes` 40 |
| `deep_dark` | W | 44-47 | `wardens` 44 |
| `land` | M1 | 48-63 | see D0-8 |
| `water` | M2 | 64-79 | see D0-8 |
| `night` | M3 | 80-95 | see D0-8 |
| `bosses` | F | 96-99 | F's boss, if it uses the registry |

Bytes 0-35 stay as they are (24 and 25 reserved by round 8 and unused); 100-255 are free for later rounds. **A family's hit-key byte is also its registry key** (`Family::KEY`), so one number names it in `EntityKey`, `MobHit` and the MW2 hit key.

Per group, three registration files (D0 creates all 21; empty lists except `pests`):

- entities: `third_party/minecraftoss/entities/src/world/families/<group>.rs`, family files in `world/families/<group>/`
- world: `third_party/minecraftoss/world/src/natural_spawner/families/<group>.rs`
- terrain: `crates/minecraft_terrain/src/families/<group>.rs`, family files in `families/<group>/`

Root files, written by D0 and never edited by a mob track: `entities/src/world/families.rs`, `world/src/natural_spawner/families.rs`, `crates/minecraft_terrain/src/families.rs`. D0 adds one module line to each parent: `pub mod families;` in `world.rs` (terrain names the family types as `minecraftoss_entities::world::families::<group>::<family>::<Type>`, so the group and family modules are `pub` too), `mod families;` in `natural_spawner.rs`, `pub mod families;` in terrain `lib.rs`. Family files live under `world/` so they can use `EntityWorld`'s private fields and helpers (`monster_ai_step`, `apply_explosion`, `death_of`, `resolve_voices`, `push_entities`, `file_in_section`, `mob_hits_mob`), exactly as `world/vexes.rs` does today; no family touches `lib.rs`.

### D0-2.2 The entity registry (vendored entities crate)

New `entities/src/world/families.rs`. Names are binding; field lists may grow.

```rust
pub struct MobCore {
    pub id: u64,
    pub kind: &'static str,
    pub body: Body,
    pub eye_height: f32,
    pub health: f32,
    pub max_health: f32,
    pub damage: DamageState,
    pub effects: MobEffects,
    pub yaw: f32,
    pub look: LookControl,
    pub baby: bool,
    pub persistent: bool,
    pub no_ai: bool,
    pub tick_count: i32,
    pub ambient_sound_time: i32,
    pub no_action_time: i32,
    pub hurt_by: Option<(Target, i32)>,
    pub swings: u32,
    pub voices: Vec<(Voice, DVec3)>,
    pub random: LegacyRandom,
    pub previous_position: DVec3,
    pub removed: bool,
}

pub struct FamilyMob<S> {
    pub core: MobCore,
    pub state: S,
}

pub struct FamilyStore<S> {
    pub mobs: Vec<FamilyMob<S>>,
}

pub struct Hurt {
    pub amount: f32,
    pub damage_type: &'static str,
    pub source: DamageSourceKind,
    pub attacker: Option<Target>,
    pub attacker_kind: Option<&'static str>,
    pub bypasses_armor: bool,
}

pub struct Despawn {
    pub distance: f64,
    pub removable: bool,
    pub peaceful_ok: bool,
    pub kept: bool,
}

pub trait Family: Clone + Send + Sized + 'static {
    const KEY: u8;
    const KINDS: &'static [&'static str];
    fn spawn(kind: &'static str, position: DVec3) -> Option<FamilyMob<Self>>;
    fn tick<W: World>(world: &mut EntityWorld, id: u64, level: &mut W, players: &[PlayerCandidate], ticks: &dyn Fn(DVec3) -> bool);
    fn category(kind: &str) -> &'static str;
    fn armor(mob: &FamilyMob<Self>) -> f32;
    fn hurt(mob: &mut FamilyMob<Self>, hurt: &Hurt) -> DamageResult;
    fn experience(mob: &mut FamilyMob<Self>) -> i32;
    fn death_drops(mob: &mut FamilyMob<Self>) -> Vec<ItemStack>;
    fn despawn(mob: &FamilyMob<Self>) -> Despawn;
    fn undead(mob: &FamilyMob<Self>) -> bool;
    fn immune_to_poison(mob: &FamilyMob<Self>) -> bool;
    fn fire_immune(mob: &FamilyMob<Self>) -> bool;
    fn breathes_water(mob: &FamilyMob<Self>) -> bool;
    fn enemy(kind: &str) -> bool;
    fn leashable(mob: &FamilyMob<Self>) -> bool;
    fn pushable(mob: &FamilyMob<Self>) -> bool;
    fn knockback_resistance(mob: &FamilyMob<Self>) -> f64;
    fn voice(mob: &FamilyMob<Self>) -> (&'static str, f32, &'static str);
    fn seats(mob: &FamilyMob<Self>) -> usize;
    fn passenger_point(mob: &FamilyMob<Self>, seat: usize) -> DVec3;
    fn dismount(mob: &FamilyMob<Self>, level: &dyn World, width: f32, heights: &[f32]) -> DVec3;
    fn monster(kind: &str) -> Option<&'static MonsterSpec>;
    const MOUNT: bool;
    fn step_sound(kind: &str) -> Option<&'static str>;
    fn loot(mob: &FamilyMob<Self>, context: &mut EntityLootContext);
    fn panicking(mob: &FamilyMob<Self>) -> bool;
    fn leash_walk<W: World>(mob: &mut FamilyMob<Self>, level: &W, target: DVec3, speed: f64);
    fn controller(mob: &FamilyMob<Self>, passengers: &[Rider]) -> Option<Rider>;
    fn rider_jumps(mob: &FamilyMob<Self>) -> bool;
    fn rider_jump(mob: &mut FamilyMob<Self>, power: i32);
    fn rider_order(mob: &mut FamilyMob<Self>, target: DVec3, speed: f64);
    fn vehicle_attachment(mob: &FamilyMob<Self>) -> f64;
}

pub trait Registered: Family {
    fn store(world: &EntityWorld) -> &FamilyStore<Self>;
    fn store_mut(world: &mut EntityWorld) -> &mut FamilyStore<Self>;
}
```

Every hook after `tick` has a default in the trait, so a family writes only what differs from vanilla `Monster`/`Mob`:

- `category`: `"monster"`. It is vanilla's `MobCategory` name and drives the census, `seen_all`, the default despawn rule and the default sound category.
- `hurt`: `MobCore::hurt_plain(amount)` mirroring `VexEntity::hurt` (`world/vexes.rs:34`: idle clock reset, `hurt_generic`, `place_death`, the hurt or death voice on a full hit), with armour from an optional `armor()` hook through `damage_after_armor` unless `bypasses_armor`. `Families::hurt` sets `core.hurt_by` and resets `no_action_time` on an applied hit with an attacker, so families never repeat the blame.
- `experience`: 5 for `monster`; 1 to 3 from the mob's random for an adult `creature`; else 0. `take_deaths` applies the existing `earns` rule (`mob_drops`, killed by a player, monsters at any age).
- `despawn`: by category, the `check_despawn` tuples (`world.rs:5832-5836`). Water ambient uses 64 blocks; verify each category with javap (`MobCategory`, `removeWhenFarAway`).
- `armor`: 0. `undead`: `crate::wither::is_undead(kind)` (the 26.3 `#undead` list). `immune_to_poison` is the third flag of `mob_effects_mut` (today "spider"). `fire_immune`: `hazards::fire_immune(kind)`. `breathes_water`: false (`living::breathe` drowns it after its air runs out).
- `enemy`: false. A family whose vanilla class is an `Enemy` (every `Monster`) answers true, so iron golems hunt it and `is_enemy` answers true.
- `leashable`: false. `pushable`: true. `knockback_resistance`: 0.
- `voice`: the kind without `minecraft:` as the sound family, volume 1, `hostile_volume` for monsters, otherwise `neutral_volume`. Check vanilla `getSoundSource` per family.
- `seats`: 0, so the mob is no vehicle (D0-2.5); `passenger_point` the vehicle's height; `dismount` on top of the vehicle. `monster`: `None` (D0-2.4).
- `MOUNT`: false. A family whose kinds can carry riders sets it; it answers the static `families::is_vehicle(kind)` that saved jockeys need before any mob exists (D0-2.5). `controller`: `None` (no rider steers it); `rider_jumps`: false (no jump bar, vanilla `PlayerRideableJumping`); `rider_jump`, `rider_order`: nothing; `vehicle_attachment`: 0 (the rider-side `getVehicleAttachmentPoint` offset, as `mob_vehicle_attachment` gives the zombie 0.7).
- `step_sound`: `None` (the block's step). `family_base_tick` builds the family's `MovementSounds` from it: `MovementSounds::monster(step)` for category `monster`, else `MovementSounds::creature(step)`. `MovementSounds` itself is `pub(crate)`, so it never appears in the public trait.
- `loot`: nothing. `take_deaths` already sets `baby: core.baby` and the killer fields; the hook adds the family's fields (`cube_size`, a variant) to the `EntityLootContext`.
- `panicking`: false (`leash_panicking`: a panicking mob is not walked by its lead). `leash_walk`: nothing, so a family that is `leashable` must override it (vanilla `PathfinderMob` walks to the holder through its navigation).
- `removed` (a `MobCore` field, not a hook): vanilla `discard()`. The family sets it for a removal without death (the endermite's 2400-tick end, a silverfish merging into stone, a vanilla `discard` on conversion). The death retain drops it, `take_deaths` never reports it (no loot, no experience), and `mob_tags_where` never saves it.

`family_group!` (defined in `families.rs`, invoked once in each group file) takes the group's name, its byte range and its entries, and generates the group's stores, the dispatch and the `Registered` impls:

```rust
family_group!(land 48..=63 {
    rabbits: rabbits::Rabbit,
    foxes: foxes::Fox,
});
```

It must accept zero entries and entries with attributes (the `#[cfg(test)]` probe), and compile without warnings in both cases. It generates, per group:

- `pub struct Group { pub <field>: FamilyStore<T>, .. }` with `Clone` and `Default`;
- `impl Registered for T`, reaching `world.families.<group>.<field>`;
- `fn tick<W: World>(world, key, id, level, players, ticks) -> bool`, dispatching on `key == T::KEY`;
- lookups: `core(id) -> Option<(u8, &MobCore)>`, `core_mut`, `core_in(key, id)`, `for_each(&mut dyn FnMut(u8, &MobCore))`, `for_each_mut`;
- per-mob work: `hurt(key, id, &Hurt)`, `exposed_mut(key, id) -> Option<&mut dyn Exposed>`, `effects_mut(id) -> Option<(&mut MobEffects, bool, bool)>`, `traits(id) -> Option<MobTraits>` (all the per-mob hook answers in one struct: undead, poison and fire immunity, enemy, leashable, pushable, panicking, knockback resistance, category, voice, seats, rider jumps, vehicle attachment), `passenger_point(id, seat)`, `controller(id, passengers)`, `rider_jump(id, power)`, `rider_order(id, target, speed)`, `leash_walk<W>(id, level, target, speed)`;
- whole-store work: `retain_mut(&mut dyn FnMut(u8, &mut MobCore, &Despawn) -> bool)`, `clone_where(&dyn Fn(DVec3) -> bool) -> Self`, `take_deaths(&mut Vec<(u64, MobDeath)>, mob_drops)`;
- static queries: `kinds()`, `has_key(u8)`, `enemy(kind)`, `is_vehicle(kind)` (from `MOUNT`), `monster_spec(kind)`, `keys_in_range()` (for the uniqueness test).

The root `pub struct Families { pub pests: pests::Group, pub trial: .., pub deep_dark: .., pub land: .., pub water: .., pub night: .., pub bosses: .. }` delegates each method to the seven groups (a small root macro or written out by hand; written once). `EntityWorld` gains one field, `families: families::Families`, and one `Default` line.

`EntityWorld` API added in `families.rs` (public unless noted):

- `spawn_family<S: Registered>(&mut self, mob: FamilyMob<S>) -> u64`: `next_id += 1`, the id into `core.id`, push, `order.push(EntityKey::Family(S::KEY, id))`, `file_in_section`. Same id rule as every `spawn_*`, so `set_next_entity_id` and `spawn_tracked` keep working.
- `family<S: Registered>(&self) -> &[FamilyMob<S>]`, `family_mob_mut<S: Registered>(&mut self, id) -> Option<&mut FamilyMob<S>>`.
- `family_core(id) -> Option<(u8, &MobCore)>`, `family_core_mut(id)`, `family_cores() -> Vec<(u8, &MobCore)>` (for terrain and survival).
- `hurt_family(id, &Hurt) -> Option<DamageResult>`.
- `pub(crate)` helpers for family ticks:
  - `family_base_tick::<S, W>(id, level, players) -> Option<BaseTick>`: `previous_position`, `tick_count`, `base_tick_fluid` (with the `MovementSounds` built from `S::step_sound` and the category), `hazards::burn`, `hazards::suffocate`, `living::breathe` (air and drowning; `S` says whether it breathes water), the death clock, the 100-tick `hurt_by` expiry, `effects.tick` with heal and magic hurt, `float_from`, the ambient voice roll, and `living::dying_travel` for a dead mob with AI. It is the shared part of the spider arm (`world.rs:6522-6570`) and `tick_vex`.
  - `family_goal_step::<S, W>(id, ai: &mut MonsterAi, level, step)`, for goal families: `MobCore` is the one source of the body, health, `hurt_by`, look and yaw. Goals read `ai.state.hurt_by` (`HurtByTargetGoal`) and `ai.state.look_control`, while `Families::hurt` writes `core.hurt_by`. The helper copies `core.hurt_by`, the body and the health into `ai.state` before `monster_ai_step` (`world.rs:1022`), and copies the body, `ai.yaw`, `ai.state.look_control` and `ai.state.hurt_by` back into the core after it. A goal family never reads or writes those copies itself; skipping the sync leaves a hurt silverfish that never turns on its attacker and a client that never sees its head turn.
  - `family_after_move::<S>(id, level, game_time, ticks)`: `hazards::blocks_act`, `push_entities`, `file_in_section`.
  - `family_strike(id, target, damage, kind)`: a player hit or `mob_hits_mob`, and `core.swings += 1`.
  - `impl<S: Family> Exposed for FamilyMob<S>` (one generic impl instead of an `exposed!` line), honouring fire resistance like `hazards::resists`. Its `hurt_hazard` goes through `S::hurt` with the hazard's damage type and `bypasses_armor`, so a family's own hurt rules (armour, the silverfish's `#always_triggers_silverfish`) see lava, fire, drowning and suffocation too.

`EntityKey::Family(u8, u64)` and `MobHit::Family(u8, u64)` are the only new variants. `mob_ids` needs no change: it only excludes projectile keys.

### D0-2.3 One generic arm per shared site

Each arm goes at the end of its list (D0-0). "core" means the `MobCore` the arm finds through `families.core_in(key, id)` (when it has the key) or `families.core(id)`.

| Site | The one generic arm |
|---|---|
| `EntityKey`, `EntityKey::id`, `MobHit`, `MobHit::id` | the `Family(u8, u64)` variant and `\| Self::Family(_, id)` |
| `EntityWorld`, `Default` | `families`, `families: Default::default()` |
| `blast_target` | `(core.body.clone(), core.eye_height)` |
| `hurt_by_blast` | `families.hurt(key, id, Hurt { source: Explosion, damage_type: blaster.damage_type(), attacker, attacker_kind (undead blasters) })`, then `&mut core.body` for the push |
| `blasted_by_player` | `core.hurt_by = Some((Player, core.tick_count))` |
| `arrow_targets` (or E2's `hit_boxes`) | every living core, its body box |
| `apply_arrow_hit` | `hurt` with `Projectile` and the owner as attacker; `projectile_knockback` scaled by `1 - knockback_resistance` |
| `mob_candidates` | `add(core.id, core.kind, &core.body, core.eye_height, alive, core.baby)` |
| `mob_hits_mob` | `hurt` with `MobAttack`, the attacker blamed |
| `take_deaths` | `families.take_deaths`: `death_of(core.id, &mut core.damage, kind, table, ctx)` with `ctx = EntityLootContext { baby: core.baby, .. }` plus the `loot` hook, and `table = Some(kind)` for category `monster` at any age, otherwise only for adults (the existing `adult` rule, `world.rs:4438`); then `experience`, `death_drops`. A `removed` mob is skipped |
| `entity_type` | `core.kind` |
| `bodies_mut`, `body_mut`, `positions`, `burning`, `census` | from every core |
| `damage_state`, `damage_state_mut` | `&core.damage` |
| `take_sounds` | `resolve_voices(&mut out, &mut core.voices, family, volume, category)` from `voice` |
| `clone_where` | `families: self.families.clone_where(&keep)` |
| `check_despawn` | `retain_mut` with the family's `Despawn` through the existing `check` closure |
| `remove_where`, `riding::remove_mobs` | `retain_mut` by position or id |
| `tick_selected` dispatch | `EntityKey::Family(key, id) => { Families::tick(self, key, id, world, players, ticks); continue; }` |
| `tick_selected` death retain, alive ids, `order.retain` | `death_ticks < 20 && !removed`; the ids; `\| EntityKey::Family(_, id)` |
| `mob_on_ray_except` | every living core, `MobHit::Family(key, core.id)` |
| `combat::mob_body` | `(core.body.clone(), core.health)` |
| `combat::knock_back` | `knockback(&mut core.body, &mut core.random, power * (1 - kr), ..)` |
| `combat::hurt_by_player` (or `hurt_for_player`) | `hurt` with `PlayerAttack` and `"minecraft:player_attack"` (or `hurt_for_player`'s `kind`, D0-0), blamed on the player; placed before the final `else` (the blazes) |
| `fireballs::fireball_damages_mob` | `fire_immune(kind) \|\| self.families.traits(victim).is_some_and(\|t\| t.fire_immune)` (the per-mob hook, which a kind list cannot answer) |
| `witches::mob_effects_mut`, `mob_effects` | `effects_mut`: `(&mut core.effects, undead, immune_to_poison)` |
| `witches::heal_mob` | `(&mut core.health, core.max_health)` |
| `witches::hurt_mob_by_magic` | `hurt` with `Magic` and the owner |
| `pushing::push_body`, `exposed_mut`, `pushes_on_local_player` | the core's body when `pushable`, else none (never filed, like vexes); `exposed_mut` the generic `Exposed` impl |
| `hazards` `exposed!` lines | none: the generic `Exposed` impl |
| `villagers::seen_all` | `seen_of(core.kind, category, ..)` |
| `leashes::body_of`, `can_be_leashed` | `&core.body`; `leashable` |
| `leashes::leash_yaw`, `leash_no_ai`, `mob_baby`, `leash_panicking` | `core.yaw`, `core.no_ai`, `core.baby`, `panicking` |
| `leashes::leash_walk`, `leash_follow_speed` | the `leash_walk` hook (after the existing `leash_no_ai` check); speed 1 |
| `riding` mount functions | D0-2.5 |
| `monster_ai::is_enemy` | `matches!(..) \|\| crate::world::families::enemy(kind)` (the closing line) |

The guard test in D0-9.1 spawns the probe family and asserts that every row of this table sees it.

### D0-2.4 Goals for registered monsters

Families on the goal framework reuse `MonsterAi` and all its goals (`FloatGoal`, `MeleeAttackGoal`, `StrollGoal`, `LookAtPlayerGoal`, `HurtByTargetGoal`, `NearestTargetGoal`, ..) without a new `MonsterKind` variant per family:

- `MonsterKind::Registered(&'static str)` (the kind). `goal_names`, `target_goal_names` and `of_kind` look up `crate::world::families::monster_spec(kind)`:

  ```rust
  pub struct MonsterSpec {
      pub goal_names: &'static [&'static str],
      pub target_names: &'static [&'static str],
      pub goals: fn() -> GoalSelector<MonsterGoalContext>,
      pub targets: fn() -> GoalSelector<MonsterGoalContext>,
      pub eye_height: f32,
      pub follow_range: f64,
      pub walk: fn(&mut WalkProfile),
  }
  ```

  `of_kind` applies the monster defaults (fire maluses cleared, `WalkTarget::Monster`), then `spec.walk` (which may set `can_float`, maluses, step height or the animal walk values). `MonsterKind::Registered(_)` joins the fall-sacrifice arm of `profile` (`:781`). `of_kind` takes the spec by reference; there is no per-tick lookup.
- `MonsterGoalContext.extra: GoalExtra` (appended after `shulker`, D0-0): typed per-family goal state (`Option<Box<dyn ExtraState>>` with `get::<T>()`, `get_mut::<T>()`, `set(T)`, `Clone` through `clone_box`, blanket impl for `T: Any + Clone + Send`). Goals keep their own state as today. `extra` carries what the family tick and its goals exchange, such as the silverfish's "look for friends" clock and the blocks its goal wants changed (goals only see `&dyn World`).
- `Prey::Kinds(&'static [&'static str])`: one new arm in `NearestTargetGoal::can_start` (`:2957` area) picking `ctx.mobs` of those kinds. Families and later tracks target registered kinds without adding `Prey` variants. D0 uses it for endermen hunting endermites (`enderman_targets`, `:2644`, `Prey::Absent` becomes `Prey::Kinds(&["minecraft:endermite"])`).
- Brain families (the warden, breeze, allay, armadillo, axolotl, camel, copper golem, creaking, frog, tadpole, goat, happy ghast, nautiluses, sniffer) keep their brain inside their state type and need none of this.

### D0-2.5 Riding (mount hooks)

Nautiluses (M2) and happy ghasts (M3) are registered mounts; zombie nautiluses carry drowned. D0 gives `riding.rs` one generic arm per mount function, so neither track edits it:

- `carries_riders`: `|| self.families.traits(vehicle).is_some_and(|t| t.seats > 0)` (the one-line site at 224, extended once).
- `passenger_point(vehicle)`: the rider's seat is its index in `passengers_of(vehicle)`; `Family::passenger_point(mob, seat)`.
- `vehicle_body`: `&core.body`.
- `start_riding`: `passengers_of(vehicle).len() < seats && force_riding(..)`.
- `stop_riding`: the dismount spot from a `Family::dismount` hook (default: on top of the vehicle, as the ravager does, `riding.rs:457`).
- `controlling_player` and `controlling_mob` (`:287`, `:259`): `Family::controller(mob, passengers_of(vehicle))` (vanilla `getControllingPassenger`: the saddled nautilus and the harnessed happy ghast answer their first player passenger, the zombie nautilus its drowned). `controlling_player` is public and read outside the crate (`minecraft_survival/src/entities.rs:1151`), so a ridden registered mount must answer it. Without this arm no player can steer a nautilus or a happy ghast.
- `order_vehicle` (`:576`, from `end_ride_tick`): a mob rider that controls its vehicle passes its navigation target down; the arm calls `rider_order` (the drowned steering its zombie nautilus).
- `jumpable_by` (`:663`, today `vehicle_horse(vehicle)?` then `controlling_player`): a registered vehicle whose `rider_jumps` holds also answers its controlling player; `tick_ride_jumps` (`:668`, today it looks the vehicle up in `cows` only) calls `rider_jump(vehicle, power)` on release. javap this round: `AbstractNautilus` implements `PlayerRideableJumping` and `HasCustomInventoryScreen`, so the nautilus dash uses the jump bar (`ride_jump_scale`, shown by `render_anim/src/minecraft_players.rs:869`).
- `mob_vehicle_attachment` (`:322`): its `_ => 0.0` arm becomes `_ => self.families.traits(id).map_or(0.0, |t| t.vehicle_attachment)`, the offset of a registered rider.
- Terrain: `VEHICLES`/`carries` (`server_mobs.rs:489`) gain `|| families::is_vehicle(kind)` (the static `MOUNT` const, since `carries` reads a tag before any mob exists), so registered jockeys load from their `Passengers`.
- The mount inventory screen (`crates/minecraft_terrain/src/mount_inventory.rs`, `MountInventory::of` `:31` and `still_valid` `:198`) reads the cow store only. The nautilus's saddle and armour slots are M2's: one arm next to the horse lookup in each of the two functions, the logic in M2's own file (the "shared file" rule of D0-3), and `MobOutcome.mount_inventory` set from the family's `interact`.

The horse family is not registered: camels, camel husks, zombie horses and skeleton horses extend `HorseKind` in the cow store (byte 11), which already has taming, saddles, riding and the inventory. They all go to one track (M3, D0-8) so `horse.rs`, `equines.rs` and the horse arms in `riding.rs` see one editor.

### D0-2.6 Typed hits on players

New player attacks (the warden's sonic boom, a bee's sting with poison, a pufferfish's poisonous touch) would each add a `PlayerHitKind` variant and arms in about ten exhaustive matches (`survival/entities.rs` `mob_hit`, `death.rs`, `bin/check_zombie_player_hits.rs`). Two tracks doing that collide. D0 adds one generic variant instead:

- `PlayerHitKind::Typed { damage_type: &'static str, message: &'static str, from: DVec3, push: DVec3, bypasses_armor: bool, bypasses_shield: bool, disable_shield_seconds: f32, effect: Option<(EffectKind, u32, u8)> }`.
- `death.rs`: `MobAttack::Typed { damage_type: &'static str, message: &'static str, bypasses_armor: bool }`; its `damage_type` and `message_id` are those strings, and `bypasses_armor` reads the flag.
- `survival/entities.rs` `mob_hit` (1882): one arm per match for `Typed` (from, toward, attack, cause, effect, push); the shield check honours `bypasses_shield`; difficulty scaling as for any mob source.
- `bin/check_zombie_player_hits.rs`: the arm.

The checklist (D0-3) tells families to use `Typed` instead of new variants. W planned `PlayerHitKind::SonicBoom`; with `Typed` it needs no new variant (`damage_type: "minecraft:sonic_boom"`, `message: "sonic_boom"`, both bypasses, the push).

### D0-2.7 Performance

- **Static dispatch.** Stores are typed (`FamilyStore<S>` per family); the tick dispatch is an `if key == T::KEY` chain over at most 30 families (no boxing per mob, no trait object per store, no `dyn World` in the movement code), and `Family::tick` is monomorphised for the server's `World` like every family tick today.
- Lookups by id scan the stores as the existing `.iter().find(|e| e.id == id)` chains do; arms that carry an `EntityKey::Family(key, id)` go straight to the store (`core_in`).
- Walks use `for_each` with `&mut dyn FnMut` (no allocation). `bodies_mut` and `positions` may return a boxed iterator or a `Vec`; they run a few times per tick.
- `EntityWorld` stays `Clone` (`would_interact` and the host's view clone it every tick): `FamilyStore<S>` derives `Clone`, `GoalExtra` clones through `clone_box`.

---

## D0-3. The plug-in checklist (what a mob track does)

The example is family `foxes` in group `land` (track M1). Replace the names.

**Before the first commit**
1. javap the entity class, its `EntityTypes` entry (size, eye height, tracking range, category, `notInPeaceful`, `fireImmune`), `createAttributes`, the goals or brain, the model, the renderer and any keyframe class; record the numbers in the track log.
2. Take the family's byte from the group's range in `docs/round10/mobs.md`; write it in the family's rows.

**Files the family creates**
1. `third_party/minecraftoss/entities/src/world/families/land/foxes.rs` (more files under `land/foxes/` if it grows, such as `ai.rs`): the state type (`pub struct Fox`), `impl Family for Fox` (`KEY`, `KINDS`, `spawn`, `tick` and the hooks that differ), the AI (`MonsterSpec` and goals in `MonsterGoalContext` terms, or a brain in the state), and `#[cfg(test)] mod tests`.
2. `crates/minecraft_terrain/src/families/land/foxes.rs`: `impl TerrainFamily for Fox` (D0-5) and the model port (`foxes_render.rs` beside it, or `foxes/render.rs`).
3. Its rows in `third_party/minecraftoss/world/src/natural_spawner/families/land.rs` (D0-4): one row per kind, with the `EntityTypes` size, even for kinds that never spawn naturally (eggs and `/summon` read the size).

**The registration lines (only in the group's own files)**
- `entities/src/world/families/land.rs`: `pub mod foxes;` and `foxes: foxes::Fox,` inside `family_group!(land 48..=63 { .. })`.
- `crates/minecraft_terrain/src/families/land.rs`: `mod foxes;` and the type inside `terrain_group!(land { .. })`.
- `world/src/natural_spawner/families/land.rs`: the rows in `ROWS`.

**Never edit, for a registered family:** the per-family lists of vendored `world.rs`, `EntityKey`, `MobHit`, `world/{combat,witches,pushing,hazards,riding,villagers,leashes}.rs`, `MonsterKind`, `MonsterGoalContext`, `Prey`, `lib.rs`; `server_mobs.rs` (`SIMULATED_KINDS`, `spawn_saved`, `mob_tags_where`, `VEHICLES`), `mob_tracking.rs` (`MobShow`, `VOLATILE`, `tracking_range`, `shows`, `show_on`), `client_mobs.rs`, the `mesh.rs` texture list, `spawner_display.rs`, terrain `lib.rs`; survival `entity_view.rs`, `entities.rs` (keys, `encode`, `decode`, `resting_monsters`, `mob_hit`), `death.rs`; `HATCHING_EGGS`; `PROTOCOL` (bumped once per release, `decisions.md`; a track that changes a serialized struct notes it under "## Protocol" in its log). If one of these seems to need an edit, the registry is missing a hook: add the hook to the root file in a separate commit named for it, and record it in the track log.

**The one exception: M3's horse kinds.** Camels, camel husks, zombie horses and skeleton horses are not registered (D0-2.5); they extend the cow store (byte 11). Track M3, and only M3, edits the cow-store lines for them: `SIMULATED_KINDS`, `VEHICLES`, `HATCHING_EGGS`, `tracking_range`, `spawner_display.rs` (`DRAWN_KINDS` and the cow arm), `horse_render.rs`, the horse arms of `server_mobs.rs`, `mob_actions.rs`, `riding.rs`, `equines.rs` and `mount_inventory.rs`. No other track touches those lines this round, so this does not collide.

**Behaviour that still needs a shared file** (blocks, items, player state, effects): keep the logic in a new file of the family's and call it from one line placed next to a related arm, never at the end of a list. Known cases:
- block ticks and block entities (`world/src/level/mod.rs`, `level/container.rs`): turtle eggs, sniffer eggs and beehives (M1), frogspawn (M2), creaking hearts, dried ghasts and copper chests (M3);
- player state (`player_nbt.rs`): parrots on shoulders (M1);
- effects (`player/src/survival.rs`): dolphin's grace (M2);
- the mount inventory screen (`mount_inventory.rs`): the nautilus's saddle and armour (M2, D0-2.5);
- item use: buckets (M2), brushes (M1);
- structure mobs (`generator/src/feature/entities.rs` `finalize_spawn`);
- existing families' goals: zombies hunting baby turtles and cats hunting rabbits (M1), cats scaring phantoms (M3); use `Prey::Kinds`;
- E1's `mob_particles::puff_for` rows for entity-event particles.

**Multiplayer**
- Every drawn state goes through `TerrainFamily::show`/`show_on` (`FamilyShow`: a `u32` of flags and eight `i32` values).
- Send animation clocks as ticks since the animation started (or countdowns, as the ravager does). The renderer adds the partial tick. Host and friend then compute the same pose from their own `ClientMob` age.
- Every saved field that changes while the mob lives goes in `TerrainFamily::VOLATILE`, or the friend respawns it every 10 ticks (`LOOK_TICKS`).
- The friend loads the family from its look tag through the generic `spawn_saved`; never key anything on player 0.

**Attacks on players:** `PlayerHitKind::Melee` for plain melee, `Typed` (D0-2.6) for anything with its own damage type, effect or push.

**Keyframes:** read the family's `*Animation` class through `minecraft_terrain::keyframes::shared()` (D0-6.1); never create another `AnimationLibrary`.

**Tests** (the family's own files):
- entities: `a_<mob>_is_<size>_with_<health>_and_its_kind` (size, eye height, health, `entity_type`, census, `mob_candidates`, loot table and experience from `take_deaths`), one test per vanilla behaviour;
- terrain: `a_<mob>_saves_and_loads_with_<fields>`, `a_friend_sees_a_<mob>_from_its_look_tag` (`spawn_tracked`, then `show_on` with its `FamilyShow`), render tests (part count, textures, baby scale);
- world: a spawn-rule test when the family has natural spawns.
- Nothing per family is needed for the eggs survey, the hit keys or the spawner cage: D0's generic tests cover every registered kind.

**Docs:** the family's rows in `docs/round10/mobs.md` (only the track's own section) in the same commit as the code; the track log.

---

## D0-4. Spawn tables (vendored world crate)

New `world/src/natural_spawner/families.rs` (root) and the seven group files `natural_spawner/families/<group>.rs`, each `pub(super) const ROWS: &[FamilySpawn] = &[..];` plus the rule and finalize functions it names.

```rust
pub(super) struct FamilySpawn {
    pub kind: &'static str,
    pub info: TypeInfo,
    pub category: MobCategory,
    pub follow_range: f64,
    pub baby_chance: Option<f32>,
    pub rule: Option<FamilyRule>,
    pub finalize: Option<FamilyFinalize>,
}

type FamilyRule = fn(&CreatureSpawns, &mut dyn SpawnLevel, &SpawnContext, &TypeInfo, BlockPos, bool) -> bool;
type FamilyFinalize = fn(&CreatureSpawns, &mut dyn SpawnLevel, Option<(&SpawnContext, DifficultyInstance)>, &mut Tag, BlockPos, &mut Option<i32>);
```

- `info` is the existing `TypeInfo` (width, height, `Placement`, heightmap, fire immunity, rules, walk, `remove_far`, cluster size). The group files are descendants of `natural_spawner`, so they can build `TypeInfo` and `Rules` and call the private helpers defined in `natural_spawner.rs` itself (`check_spawn_rules` `:761`, the `registries` field).
- **Privacy.** `dark_enough`, `mob_spawn_rules`, `difficulty_at`, `populate_armor` and `roll_enchantments` are private methods in `natural_spawner/spawning.rs`, and `natural_spawner/families` is a sibling of `spawning`, not a descendant, so it cannot call them. D0 changes those five `fn` to `pub(super) fn` (one word on each line; `pub(super)` from `spawning` reaches all of `natural_spawner`). A later track that needs another `spawning.rs` helper widens it the same way, in its own commit.
- **The finalize context.** `finalize_monster` has a `SpawnContext` and a `DifficultyInstance` and passes `Some((context, difficulty))`. `finalize_spawn` (the creature path, `natural_spawner.rs:896`, also used by chunk-generation spawns at `:628`) has neither and passes `None`; a creature row's finalize must work without them, as vanilla's animal `finalizeSpawn` overrides only use the random and the biome.
- `baby_chance`: `Some(chance)` for an `AgeableMob` (the creature path calls `ageable` with it), `None` for a mob that is not an `AgeableMob` (for example the `AbstractFish` kinds; javap each family's superclass): no `ageable` call and so no baby draw from the level random, which a fixed 0.0 chance would still make.
- New `Rules::Family { creature: bool }`: `creature()` returns the flag (append `| Self::Family { creature: true }` to its one-line `matches!`). `natural_spawn_rules` (`spawning.rs:730`) gets one arm calling the row's `rule` with `spawner = false`; `spawner_may_spawn` (`:700`) gets one arm before its `_ =>` fallthrough calling it with `spawner = true` (several 26.3 rules return true at once for `EntitySpawnReason.isSpawner`). A row without `rule` passes, as vanilla's `SpawnPlacements.checkSpawnRules` does for a type with no placement.
- **Rows are checked first**, so a row also overrides an existing kind. A track never edits the big table, only adds a row in its own group file (goats, for example, have size and category but `Rules::Other` today and never spawn):
  - `type_info`: first line `if let Some(row) = families::row(kind) { return Some(row.info); }`;
  - `MobCategory::of_type`: first line from `row.category`;
  - `follow_range_base`: first line from `row.follow_range`;
  - `summon` (`spawning.rs:886`) and the natural spawn (`:605-608`) need no change: `type_info` returns the row, and their existing code routes by `info.rules.creature()`;
  - `finalize_monster`: right after the follow-range attribute line (`:1167`), **any row** runs its `finalize` if it has one, then `finalize_mob`, and returns. A row without `finalize` (the silverfish and endermite rows) must return here too, or it falls through to the catch-all at `:1336` and logs `note_unsupported("finalizeSpawn of minecraft:silverfish")` on every spawn;
  - `finalize_spawn` (`natural_spawner.rs:896`): at the top, any row runs its `finalize` if any, then `ageable` when `baby_chance` is `Some`, then `finalize_mob`, and returns (its own catch-all also calls `note_unsupported`).
- Group data: vanilla's `SpawnGroupData` (one variant per group of spawns) is the `&mut Option<i32>` the first member sets and later members read. D0 adds `Group::Family { data: i32, size: i32 }` and `MonsterGroup::Family { data: i32, size: i32 }`, and an arm in `ageable`.
- Kinds that already have built-in rows (rabbit, fox, frog, parrot, polar bear, turtle, camel, armadillo, axolotl, panda, glow squid, zombie horse, camel husk) need no row unless the track changes them. Their finalize arms in `finalize_spawn` (fox, rabbit, armadillo) keep working.
- D0's own rows (group `pests`): `minecraft:silverfish` and `minecraft:endermite`, 0.4 x 0.3, `OnGround`, `MotionBlockingNoLeaves`, not fire immune, `Rules::Family { creature: false }`, monster walk, removed far away, cluster 4, `MobCategory::Monster`, follow range 16, `baby_chance: None`, no `finalize`. The rule (javap `checkSilverfishSpawnRules` and `checkEndermiteSpawnRules`, the same for both) is `checkAnyLightMonsterSpawnRules` (not peaceful, a valid block below), then true for a spawner, else no player within 5 blocks of the block centre.

Structure mobs keep their finalize in `generator/src/feature/entities.rs` (the generator cannot see the world crate). That function is "adjacent only", so add arms next to a related kind.

---

## D0-5. The client registry, friends and render (terrain and survival)

New `crates/minecraft_terrain/src/families.rs` (root), the seven group files and `terrain_group!`. One trait, implemented in terrain for each entity-side family type (a local trait on a foreign type, which the orphan rule allows):

```rust
pub trait TerrainFamily: Registered {
    const TRACKING_RANGE: i32;
    const VOLATILE: &'static [&'static str];
    const TEXTURES: &'static [&'static str];
    const SHADOW_RADIUS: f32;
    const MAX_HEAD_Y_ROT: f32;
    const CAGE_MARGIN: f32;
    fn read(mob: &mut FamilyMob<Self>, tag: &Tag);
    fn fields(mob: &FamilyMob<Self>) -> Vec<(&'static str, Tag)>;
    fn show(mob: &FamilyMob<Self>) -> FamilyShow;
    fn show_on(mob: &mut FamilyMob<Self>, show: &FamilyShow);
    fn draw(out: &mut FamilyDraw<'_>, mobs: &[FamilyMob<Self>], poses: &ClientMobs, atlas: &Atlas, light: &SkyLight, partial: f32);
    fn interact(world: &mut EntityWorld, id: u64, actor: &mut Actor, outcome: &mut MobOutcome) -> bool;
    fn client_effects(mobs: &[FamilyMob<Self>], poses: &ClientMobs, fx: &mut ClientFx<'_>);
}

pub struct FamilyDraw<'a> {
    pub models: &'a mut ChunkMesh,
    pub culled: &'a mut ChunkMesh,
    pub translucent: &'a mut ChunkMesh,
    pub held: &'a mut Vec<HeldItem>,
}

pub struct ClientFx<'a> {
    pub ambient: &'a mut AmbientParticles,
    pub spell: &'a mut SpellParticles,
    pub portal: &'a mut PortalParticles,
    pub poof: &'a mut PoofParticles,
    pub sounds: &'a mut Vec<(String, DVec3, f32, f32)>,
    pub random: &'a mut LegacyRandom,
    pub seen: &'a mut HashMap<u64, i32>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FamilyShow {
    pub flags: u32,
    pub values: [i32; 8],
}
```

Defaults: `VOLATILE` empty, `MAX_HEAD_Y_ROT` 75, `CAGE_MARGIN` 0.05, `read`/`fields` nothing, `show` default, `show_on` nothing, `interact` false (pass), `client_effects` nothing. `TRACKING_RANGE`, `TEXTURES`, `SHADOW_RADIUS` and `draw` have no default. `terrain_group!(land { Rabbit, Fox })` generates the group's walks; the root delegates to the seven groups.

`draw` keeps the shape of every `append_*` today (`vex_render.rs:137`): for each mob, `poses.pose(mob.core.id, partial)` gives the `MobPose` (feet, body and head rotation, walk, age, swing, overlay), the family's own pose function turns it and the mob's state into part rotations, and `posed_cube::{Cube, Skin, append, part_pose}` appends the model to `out.models` (a model whose vanilla render type culls back faces, `entityCutout` like the bat's, to `out.culled`; translucent layers to `out.translucent`; held items to `out.held`). The same `draw` serves the host, every friend (their mirror world) and spawner cages (`spawner_display.rs:196` already has `models`, `culled` and `translucent`).

### D0-5.1 Saving, loading and hatching (commit 1)

- `server_mobs::spawn_saved`: before the `match`, the common values are parsed once into `families::Common { pos, yaw, health, persistent, no_ai, random, fire, age }` (they already are, `:582-591`). The `_ => return None` arm (`:1321`) becomes `_ => crate::families::spawn_saved(world, kind, tag, &common)?`. It finds the family whose `KINDS` holds the tag's `id` and calls `S::spawn` with that `&'static str` entry (the tag's own `&str` is not `'static`), applies the common values, calls `TerrainFamily::read`, then `world.spawn_family`. The existing tail (UUID, leash, effects) then runs for registered mobs too.
- `mob_tags_where` (`:1497`): one call, `crate::families::save(world, &mut add)`. For each core it calls `add(core.id, core.kind, &core.body, core.yaw, core.health, core.persistent, S::fields(mob))`.
- `pub fn is_simulated(kind) -> bool` and `pub fn simulated_kinds() -> Vec<&'static str>` in `server_mobs.rs`: `SIMULATED_KINDS` plus `families::kinds()`. `carries` (`:496`), `minecraft_survival/src/commands.rs:147` (`/summon` suggestions) and the server test at `server.rs:8150` use them. Track D's trial spawners ask `is_simulated` (`track-d.md`).
- `VEHICLES`/`carries`: `|| families::is_vehicle(kind)` (D0-2.5).
- Eggs survey (`server.rs:7717`, test only): `assert_eq!(hatched, HATCHING_EGGS)` becomes a comparison with `HATCHING_EGGS` plus the registered kinds with `minecraft:` stripped (both lists hold short names such as `"bat"`), sorted. `HATCHING_EGGS` itself is never edited again, and every family that lands is checked automatically.

### D0-5.2 Friends (commit 2)

- `mob_tracking::tracking_range` (`:285`): first line `if let Some(range) = crate::families::tracking_range(kind) { return range; }`.
- `look` (`:314`): also removes the `VOLATILE` keys of the family whose `KINDS` holds the tag's `id`, and only those. Removing every family's keys from every tag would hide a field an old family shows through its look tag (a slime's `Size`, for example) the day a new family lists the same name.
- `MobShow` (`:65`) gains `pub family: Option<FamilyShow>` (Copy, Default, serde; postcard writes 1 byte for `None`, so other mobs pay one byte per tick). No `PROTOCOL` bump: note the change under "## Protocol" in the log (D0-0).
- `shows` (`:330`): one call, `crate::families::shows(world, &mut put)` (sets `family: Some(S::show(mob))`).
- `show_on` (`:592`): one call, `crate::families::show_on(world, mob)`. It places the core's body with the existing `place` and calls `S::show_on` when `mob.show.family` is `Some`.
- `spawn_tracked` needs nothing: it calls `spawn_saved`.

### D0-5.3 Draw, hit, use (commit 3)

- `client_mobs::server_mobs` (`:1196`): one loop pushing every core (`core.yaw`, `&core.look`, `&core.body`, `core.baby`, `core.eye_height`). The swings chain gains `families::swings(world)` (`core.swings`), and the head-turn tweaks gain `families::max_head_y_rot(world, id)`.
- `shadow_casters` (`:1343`): `SHADOW_RADIUS`, halved for babies.
- `mesh.rs` `build_internal` (`:478`): `for texture in crate::families::textures() { .. }`.
- `spawner_display.rs`: `DRAWN_KINDS.contains(kind) || families::knows(kind)`, one `draw` arm `_ if crate::families::knows(kind) => crate::families::draw(..)`, and the cage test iterates the registered kinds with their `CAGE_MARGIN`.
- `mob_actions::interact`: a `MobHit::Family(key, id)` arm before `_ => return outcome` (`:376`) calls `families::interact`. A family that handles the click sets `outcome` and returns true; `would_interact` follows through its clone, as today.
- `minecraft_survival/src/entities.rs`: `encode` maps `MobHit::Family(key, _)` to `key`. `decode` maps any byte for which `minecraftoss_entities::world::families::has_key(byte)` holds to `MobHit::Family(byte, id)`. `resting_monsters` (`:1788`) gains one generic loop; the sleep check's `MONSTERS` list already names every registered monster of this round.
- `minecraft_survival/src/entity_view.rs`: `boxes` (`:630`) one generic loop (`MobHit::Family(key, core.id)`, living cores). `meshes` (`:829`) one call `minecraft_terrain::families::draw(&mut FamilyDraw { models, culled, translucent, held: &mut family_items }, w, poses, atlas, light, partial)`, with `family_items` chained into `held` (`:1350`), and one generic glow loop after the per-kind ones (`:1108-1121`): each registered mob with `w.glowing(id)` is drawn alone into a fresh `ChunkMesh` (`draw` with `std::slice::from_ref(mob)`) and handed to `glow_render::append_glowing(&mut out.glow, &mut out.glow_cover, &mob)`, so a spectral arrow, a raid bell or `/effect give @e minecraft:glowing` outlines registered mobs too. `advance` (`:150`) one call `families::client_effects(..)` with a `ClientFx` built from the view's particle systems and a new `family_seen: HashMap<u64, i32>` field.
- The keyframe holder (D0-6.1).

### D0-5.4 The sleep fix (commit 4)

`resting_monsters` lists no family added since round 8, so a player can sleep beside a pillager. D0 adds the illager, vex, ravager and wither stores (next to the piglins, which is the end of the list), filtered as today by `sleep::MONSTERS`. Commit: "Minecraft players cannot sleep beside pillagers, vexes, ravagers or withers". Track E does not list it.

---

## D0-6. Shared prerequisites

| | Prerequisite | Where | Used by |
|---|---|---|---|
| P1 | Keyframe holder: one `AnimationLibrary` for the whole client | **D0** (D0-6.1) | W, D, M1, M2, M3 |
| P2 | Water: `WaterBoundPathNavigation`/`SwimNodeEvaluator`, `SmoothSwimmingMoveControl`/`LookControl`, `RandomSwimmingGoal`, `TryFindWaterGoal`, `WaterAnimal` air, fish flopping, `Bucketable` and the `*_bucket` items with `bucket_entity_data` | **M2**: only M2's mobs swim or fit in buckets (the sulfur cube is `Bucketable`, so it moves to M2) | M2 |
| P3 | Flying: `FlyingPathNavigation`/`FlyNodeEvaluator`, a general `FlyingMoveControl` | **D0** (D0-6.2): M1 (parrots, bees) and M3 (allays, happy ghasts) both need it | M1, M3 (and the wither, optionally) |
| P4 | Riding for registered families | **D0** (D0-2.5) | M2 (nautiluses), M3 (happy ghasts) |
| P7 | Game events and vibrations | **W** | W; M3's allays listen to note blocks through it, so that part waits for W's merge or goes to "left for later" |
| P8 | Natural lightning (`ServerLevel.tickThunder`, `findLightningTargetAround`, the bolt's fire and damage, `thunderHit` conversions, the skeleton trap, the bolt for host and friend) | **M3**: only the skeleton horse trap needs it, and it touches pigs, villagers, creepers and mooshrooms, which no other track edits | M3 |

Other shared pieces already exist: P5 effects with icons (dolphin's grace is M2's new `EffectKind`), P6 custom spawners (phantoms, M3), P10 raids.

### D0-6.1 The keyframe holder

`crates/minecraft_terrain/src/keyframes.rs` gains `static SHARED: OnceLock<AnimationLibrary>`, `pub fn install(jar: Option<PathBuf>)` (the first call wins) and `pub fn shared() -> &'static AnimationLibrary`. `shared()` returns an empty library when nothing was installed, so it reads nothing and says why, as `AnimationLibrary::new(None)` does. `AnimationLibrary` already locks its cache (`keyframes.rs:761`). `render_anim` calls `minecraft_terrain::keyframes::install(minecraft_world::data_jar())` once where the Minecraft client starts, for host and friend alike. W's warden, D's breeze and every keyframed M family read through `shared()`. Moving the bat to `BatAnimation` stays optional (round 9 leftover); if done, it is its own commit, and the gated `the_bat_tables_match_the_jar` keeps proving the same pose.

### D0-6.2 P3: flying navigation (group `flying-navigation`)

javap first: `FlyingPathNavigation` (`createPathFinder`, `canMoveDirectly`, `canUpdatePath`, `getTempMobPos`, `createPath(Entity, int)`, `tick`, `isStableDestination`, `canNavigateGround`), `FlyNodeEvaluator` (`prepare`, `getStart`, `canStartAt`, `getTarget`, `getNeighbors`, `hasMalus`, `isOpen`, `findAcceptedNode`, `getCachedPathType`, `getPathType`, the start-node candidates and `SMALL_MOB_SIZE`) and `FlyingMoveControl(mob, maxTurn, hoversInPlace)`.

- New vendored `entities/src/flying.rs` (one `pub mod flying;` line in `lib.rs`):
  - `FlyingMoveControl { max_turn, hovers_in_place }`, generalising the wither's `fly` (`wither.rs:204`): the speed is the modifier times flying speed in the air and movement speed on the ground, the yaw turns at most 90 degrees, the pitch at most `max_turn`, `yya` is plus or minus the speed, and gravity comes back on `WAIT` unless the mob hovers in place.
  - `FlyingPathNavigation`, whose paths come from `FlyNodeEvaluator`'s neighbours on the existing A* in `path_search.rs` (a new `PathTerrain` or a search mode; the implementer picks after reading `path_search.rs:17-420`), with vanilla's node reach, `isClearForMovementBetween`, the stable destination (`entityCanStandOn`, the wither's `solid_render` decision) and the stuck check (`wither_ai::FlightPath` has the same numbers).
- Users: parrots `FlyingMoveControl(this, 10, false)`, bees and allays `(this, 20, true)` (javap per family); the happy ghast has its own controls and `HappyGhast$BabyFlyingPathNavigation`.
- **Optional second commit** "Minecraft withers fly around walls": the wither's `FlightPath` replaced by the new navigation. This is the round 9 wither leftover. Skip it if the group runs long, and keep it on the "left for later" list.

---

## D0-7. The proof: silverfish and endermites

Two small families in group `pests`, each live, saved, loaded, spawned, drawn, hittable and seen by friends on the registry alone. They are removed from M3's list. Two families (not one family with two kinds) prove that the dispatch picks the right store by key.

**Vanilla 26.3** (javap this round):
- `EntityTypes`: both `MONSTER`, `sized(0.4, 0.3)`, `eyeHeight(0.13)`, `passengerAttachments(0.2375)`, `clientTrackingRange(8)`, `notInPeaceful`.
- `Silverfish`: health 8, speed 0.25, attack 1, `xpReward` 5 (`Monster`'s). Goals: 1 `FloatGoal`, 1 `ClimbOnTopOfPowderSnowGoal`, 3 `SilverfishWakeUpFriendsGoal`, 4 `MeleeAttackGoal(1.0, false)`, 5 `SilverfishMergeWithStoneGoal` (a `RandomStrollGoal(1.0, 10)` with only the `MOVE` flag). Targets: 1 `HurtByTargetGoal().setAlertOthers()`, 2 `NearestAttackableTargetGoal(Player, true)`.
  - `hurtServer`: an attacker entity or `#always_triggers_silverfish` calls `notifyHurt` (look for friends `adjustedTickDelay(20)`).
  - `tick` sets `yBodyRot = yRot`. Step sound at 0.15. Movement emission `EVENTS`.
  - `getWalkTargetValue`: 10 above an infested host block, else `Monster`'s.
- `Endermite`: health 8, speed 0.25, attack 2, `xpReward` 3. Goals: 1 `FloatGoal`, 1 `ClimbOnTopOfPowderSnowGoal`, 2 `MeleeAttackGoal(1.0, false)`, 3 `WaterAvoidingRandomStrollGoal(1.0)`, 7 `LookAtPlayerGoal(Player, 8)`, 8 `RandomLookAroundGoal`. Targets: 1 `HurtByTargetGoal().setAlertOthers()`, 2 `NearestAttackableTargetGoal(Player, true)`.
  - `Lifetime` is saved; `aiStep` counts it up unless persistence is required and discards the mite at 2400.
  - On the client: two `PORTAL` particles a tick at a random point within half its width, moving `(r - 0.5) * 2, -r, (r - 0.5) * 2`.
  - Step sound at 0.15.
- Spawn rules: D0-4.
- `ThrownEnderpearl.onHit`: for a player owner whose connection accepts messages, `random.nextFloat() < 0.05` with `isSpawningMonsters` and not peaceful spawns a `TRIGGERED` endermite at the owner's position and rotation before the teleport.
- Loot tables `entities/silverfish` and `entities/endermite` are empty. `#arthropod` is bee, endermite, silverfish, spider, cave spider.
- Textures `minecraft:entity/silverfish/silverfish` and `minecraft:entity/endermite/endermite`.
- javap during the work: the two goals' exact loops (`SilverfishWakeUpFriendsGoal.tick` searches y 0, -1, +1 .. up to 5 and x, z up to 10 in vanilla's order, breaking with `mob_griefing` or restoring the host block, and stops on `nextBoolean`); `SilverfishMergeWithStoneGoal` (`Direction.getRandom`, `canUse` with no target, navigation done, `mob_griefing`, `nextInt(reducedTickDelay(10)) == 0`, a host block that direction; `start` turns it infested, `spawnAnim`, `discard`); `InfestedBlock` (`spawnAfterBreak` with the `block_drops` rule and no `#prevents_infested_spawns` enchantment, `wasExploded`, the host map); the models, renderers (shadow radius, flip degrees) and `Silverfish`/`Endermite` `setupAnim`; `InfestedMobEffect.onMobHurt` (optional).

**D0-7.1 Live (group `silverfish-live`).** `world/families/pests/silverfish.rs` (key 36) and `world/families/pests/endermites.rs` (key 37) on `MonsterKind::Registered` with their `MonsterSpec`s, using the base-tick helpers.
- The silverfish's wake-friends clock lives in `GoalExtra`; `Silverfish::hurt` sets it.
- Melee strikes go through `family_strike`. The alert goes to same-kind mobs within the follow range, as `vex_alerts` does.
- The endermite's `life` counts up (saved as `Lifetime`); at 2400 it sets `core.removed` (vanilla `discard`: no death, no loot, no experience).
- Both step at 0.15 through `step_sound` (`entity.silverfish.step`, `entity.endermite.step`).
- Endermen hunt endermites (`Prey::Kinds`, D0-2.4).
- Both answer `enemy` (vanilla `Monster` is an `Enemy`), so iron golems hunt them. Neither refuses poison. (Bane of Arthropods has no damage bonus anywhere in the fork yet; the 26.3 `#arthropod` tag is bee, endermite, silverfish, spider and cave spider when someone adds it.)
- Status in `mobs.md` stays `left` until group `registry-client`, which makes them `live`.

**D0-7.2 Drawn (group `silverfish-drawn`).** `crates/minecraft_terrain/src/families/pests/` gets the two `TerrainFamily` impls and the models, ported from `SilverfishModel` and `EndermiteModel` (`createBodyLayer`, `setupAnim`) on `posed_cube`.
- The renderers' shadow and death flip come from javap.
- The endermite's portal particles go through `client_effects`, for the host and every friend.
- Spawner cages work through the generic arm.
- Rows become `drawn`.

**D0-7.3 Behaviour (group `silverfish-behaviour`).**
1. "Minecraft silverfish hide in stone and wake their friends":
   - a player breaking an infested block without Silk Touch (and with `block_drops`) releases a silverfish with its spawn poof, and so does a blast. The terrain hook is one call in the block-break path, placed after the drops call; the logic goes in a new `crates/minecraft_terrain/src/infested.rs`, and W adds game events in the same function, so keep to one line;
   - the wake-friends goal breaks nearby infested blocks (or restores them without `mob_griefing`); the family tick applies the blocks its goal asked for;
   - the merge goal turns a host block infested and removes the silverfish (`core.removed`, no death);
   - optional: the Infested effect's silverfish on hurt.
   - The poof of a merge shows through E1's entity-event particles once E1 lands; until then it is on the "left for later" list.
2. "Endermites come from ender pearls": the 5% roll in the pearl landing (`server.rs:2156`, `Throwable::EnderPearl`), at the thrower before the teleport, as a `TRIGGERED` spawn (no finalize).
   - Rows become `done`. The owner's steps and kit (D0-10) go into the track log.

---

## D0-8. The round 10 mob list and the split

### The split

- **Horse family in one track.** Camels, camel husks, zombie horses and skeleton horses all extend `HorseKind` in the cow store. In round 9's batches they were split between A (camels) and N (undead horses), which would put two tracks in `horse.rs`, `equines.rs`, `horse_render.rs`, the horse arms of `server_mobs.rs` and `riding.rs`. All four go to M3 with P8, which the skeleton trap needs.
- **Bees and sniffers** (batch G) are land animals with block homes. They go to M1, which keeps M3 balanced.
- **The sulfur cube** is `Bucketable` and `Shearable` (javap). It goes to M2 with the buckets (P2).
- **Phantoms** stay in M3 (decisions.md).
- **Silverfish and endermites** are D0's proof.

| Track | Group (registry) | Bytes | Mobs | Suggested group keys |
|---|---|---|---|---|
| M1 land animals | `land` | 48-63 | rabbit, fox, goat, parrot, panda, turtle, armadillo, polar bear, sniffer, bee | `rabbits`, `foxes`, `goats`, `parrots`, `pandas`, `turtles`, `armadillos-polar-bears`, `sniffers`, `bees` |
| M2 water and buckets | `water` | 64-79 | squid, glow squid, cod, salmon, tropical fish, pufferfish, dolphin, axolotl, frog, tadpole, guardian, elder guardian, nautilus, zombie nautilus, sulfur cube | `water-movement` (P2), `squids`, `fish`, `dolphins`, `axolotls`, `frogs-tadpoles`, `guardians`, `nautiluses`, `sulfur-cubes` |
| M3 night, built and horses | `night` (+ cow store, byte 11) | 80-95 | phantom, zombie horse, skeleton horse, camel, camel husk, allay, copper golem, creaking, happy ghast | `phantoms`, `lightning` (P8), `undead-horses`, `camels`, `allays`, `copper-golems`, `creakings`, `happy-ghasts` |

### The file `docs/round10/mobs.md`

D0's first commit creates it with the content below (from `# Round 10 mob list` to the end of this section) and nothing else. One section per track keeps every track's edits away from the others' lines. Rows keep round 9's numbers. The family column names the registered store, or the old store the kind extends.

```markdown
# Round 10 mob list: the 38 mobs left after round 9

Rows 1-13 and 51 of `docs/round9/mobs.md` are done. This file holds the rest, one section per round 10 track; each track edits only its own section, in the same commit as the code. Status: `left`, `live` (in the entity world, saved, loaded, hatched and seen by friends), `drawn`, `done` (its vanilla behaviours, spawning and the owner's steps written). How a family plugs in: `docs/round10/track-d0.md`, D0-3 (the plug-in checklist). "Exists" lists only what goes beyond the default: a texture, a loot table, sounds, an unported model class and a `/summon` tag exist for every one.

## Hit keys

A MW2 bullet finds a mob through one number: the kind in the top byte, the entity id below (`crates/minecraft_survival/src/entities.rs` `encode`, `decode`). A registered family's byte is its `Family::KEY`.

| Bytes | Owner |
|---|---|
| 0-35 | round 9 and earlier (24 and 25 reserved, unused) |
| 36-39 | D0, group `pests`: 36 silverfish, 37 endermites, 39 the test probe |
| 40-43 | D, group `trial`: 40 breezes |
| 44-47 | W, group `deep_dark`: 44 wardens |
| 48-63 | M1, group `land` |
| 64-79 | M2, group `water` |
| 80-95 | M3, group `night` (camels, camel husks and the undead horses use the cow store's byte 11) |
| 96-99 | F, group `bosses` |
| 100-255 | free |

## D0: the registry's proof (group `pests`)

| # | Mob | Where the player meets it | Exists beyond the default | Left | Size | Family | Hit key | Status | Commits |
|---|---|---|---|---|---|---|---|---|---|
| 42 | Silverfish | strongholds, infested blocks | stronghold spawner shows one; strongholds place infested stone bricks; sleep list; the `infested` potion is listed | infested blocks, waking friends, merging into stone, `SilverfishModel` | M | `silverfish` | 36 | left | |
| 43 | Endermite | ender pearls | sleep list; the enderman's endermite target slot rolls with no prey | 5% pearl roll, endermen hunt it, 2400-tick life, `EndermiteModel`, portal particles | S | `endermites` | 37 | left | |

## D: trial chambers (group `trial`)

| # | Mob | Where the player meets it | Exists beyond the default | Left | Size | Family | Hit key | Status | Commits |
|---|---|---|---|---|---|---|---|---|---|
| 52 | Breeze | trial chambers | wind charge burst (`wind.rs`), wind charges trigger blocks (f142f9c1), `BreezeAnimation` readable | brain, wind charges, `BreezeModel`, keyframes | L | `breezes` | 40 | left | |

## W: the deep dark (group `deep_dark`)

| # | Mob | Where the player meets it | Exists beyond the default | Left | Size | Family | Hit key | Status | Commits |
|---|---|---|---|---|---|---|---|---|---|
| 14 | Warden | deep dark, shriekers | dormant, sleep and fire lists; darkness effect; `WardenAnimation` readable; ancient cities generate with shriekers and loot | everything | XL | `wardens` | 44 | left | |

## M1: land animals (group `land`, bytes 48-63)

| # | Mob | Where the player meets it | Exists beyond the default | Left | Size | Family | Hit key | Status | Commits |
|---|---|---|---|---|---|---|---|---|---|
| 15 | Rabbit | taiga, snowy, desert, meadow, cherry (dormant today) | spawn row, rules and group variants | jump control, carrot raids, biome variants, killer bunny, keyframes | M | `rabbits` | 48 | left | |
| 16 | Fox | taiga, snowy taiga, grove (dormant today) | spawn row and finalize; wolves hunt foxes | sleeping by day, pounce, trust, mouth item, berries, keyframes | L | `foxes` | 49 | left | |
| 17 | Goat | peaks, snowy slopes (dormant today) | size row (rules missing, never spawns) | brain, ram, long jump, screaming, horns, milking | L | `goats` | 50 | left | |
| 18 | Parrot | jungles (dormant today) | size row | flying (P3 from D0), imitation, dancing, cookies, shoulder | L | `parrots` | 51 | left | |
| 19 | Panda | jungles (dormant today) | spawn row with the animal rules | genes and personalities, bamboo, sneeze, roll | L | `pandas` | 52 | left | |
| 20 | Turtle | beaches (dormant today) | size row; zombies' baby-turtle target slot | home beach, eggs hatching, scute | L | `turtles` | 53 | left | |
| 21 | Armadillo | savanna, badlands (dormant today) | spawn row and finalize | brain, roll up, scute and brushing, keyframes | M | `armadillos` | 54 | left | |
| 22 | Polar bear | frozen biomes (dormant today) | size row | defends cubs, standing attack | M | `polar_bears` | 55 | left | |
| 46 | Sniffer | sniffer eggs | | egg hatching, digging seeds, brain, keyframes | M | `sniffers` | 56 | left | |
| 44 | Bee | flower forests, plains, nests | generator places nests with bees | flying (P3 from D0), nests and hives, pollination, honey, anger | L | `bees` | 57 | left | |

## M2: water and buckets (group `water`, bytes 64-79)

| # | Mob | Where the player meets it | Exists beyond the default | Left | Size | Family | Hit key | Status | Commits |
|---|---|---|---|---|---|---|---|---|---|
| 25 | Squid | oceans, rivers | category only | swimming (P2), swim impulse, ink | M | `squids` | 64 | left | |
| 26 | Glow squid | underground water (dormant today) | spawn row and rules | glow ink, glow | S | `squids` | 64 | left | |
| 27 | Cod | oceans | category only | swimming (P2), schools, flopping, bucket | L | `fish` | 65 | left | |
| 28 | Salmon | oceans, rivers | category only | sizes, bucket | L | `fish` | 65 | left | |
| 29 | Tropical fish | warm oceans, lush caves | category only | patterns, bucket | L | `fish` | 65 | left | |
| 30 | Pufferfish | warm oceans | category only | puffing, poison, bucket | L | `fish` | 65 | left | |
| 31 | Dolphin | oceans | category only | breathing, treasure leading, dolphin's grace | L | `dolphins` | 66 | left | |
| 32 | Axolotl | lush caves (dormant today) | size row | brain, playing dead, variants, bucket | L | `axolotls` | 67 | left | |
| 33 | Frog | swamps (dormant today) | size row | brain, variants, tongue, froglights, keyframes | L | `frogs` | 68 | left | |
| 34 | Tadpole | frogspawn | | hatching, bucket, growth | M | `tadpoles` | 69 | left | |
| 35 | Guardian | ocean monuments | mining fatigue effect | laser, spikes | L | `guardians` | 70 | left | |
| 36 | Elder guardian | ocean monuments | mining fatigue effect | elder curse, shares `guardians` | L | `guardians` | 70 | left | |
| 37 | Nautilus | oceans (26.3) | | brain, riding (D0 mount hooks), dash, keyframes | L | `nautiluses` | 71 | left | |
| 38 | Zombie nautilus | oceans (26.3) | | drowned rider, shares `nautiluses` | L | `nautiluses` | 71 | left | |
| 49 | Sulfur cube | sulfur caves (26.3) | `AbstractCubeMob` like the slime; `Bucketable`, `Shearable` | bucket, shearing, archetypes, fuse | M-L | `sulfur_cubes` | 72 | left | |

## M3: night, built mobs and the horse family (group `night`, bytes 80-95)

| # | Mob | Where the player meets it | Exists beyond the default | Left | Size | Family | Hit key | Status | Commits |
|---|---|---|---|---|---|---|---|---|---|
| 39 | Phantom | after 3 sleepless nights | `Player.time_since_rest`; custom spawners (P6); cats exist | `PhantomSpawner`, swoop, daylight burning, cats scare them; the terrain test `live_shulkers_stay_far_from_players_while_dormant_monsters_go` uses the phantom as its dormant monster and must switch kind or drop that half | M | `phantoms` | 80 | left | |
| 40 | Zombie horse | plains, savanna, snowy plains | spawn row, rules and the jockey finalize (dormant) | `HorseKind::ZombieHorse`, rider, rideable when tamed | M | cow store (`HorseKind`) | 11 | left | |
| 41 | Skeleton horse | lightning trap | | trap with natural lightning (P8, M3), swimming | M | cow store (`HorseKind`) | 11 | left | |
| 23 | Camel | deserts, desert villages (dormant today) | size row | `HorseKind::Camel`, two riders, sit, dash, keyframes | L | cow store (`HorseKind`) | 11 | left | |
| 24 | Camel husk | desert husk jockey (dormant today) | size row; the husk's finalize builds the jockey | monster camel with a husk rider | S | cow store (`HorseKind`) | 11 | left | |
| 45 | Allay | outposts, mansions | | brain, flying (P3 from D0), item collecting, note blocks (needs W's game events), duplication | L | `allays` | 81 | left | |
| 47 | Copper golem | built (26.3) | iron and snow golem builds (`golem_build.rs`) | build, sorting copper chests, oxidation, statue | L | `copper_golems` | 82 | left | |
| 48 | Creaking | pale garden | `player_views` from endermen | heart link, frozen while looked at, keyframes | L | `creakings` | 83 | left | |
| 50 | Happy ghast | dried ghast (26.3) | ghast model | ghastling, harness, flying, four riders (D0 mount hooks) | L | `happy_ghasts` | 84 | left | |
```

---

## D0-9. Tests

All tests sit in the files they test; new files pass rustfmt.

**D0-9.1 Vendored entities (groups `registry-core` and `registry-hooks`)**, with a `#[cfg(test)]` probe family `minecraft:registry_probe` (key 39, group `pests`) whose hooks are switchable (undead, fire immune, enemy, leashable with a `leash_walk`, pushable, `MOUNT` with seats 1, a controller, rider jumps, a `MonsterSpec` with melee):
- `families::tests::every_family_key_is_unique_and_inside_its_group_range` (also never one of 0-35).
- `a_registered_mob_is_seen_by_every_world_walk`: `mob_ids`, `entity_type`, `census`, `positions`, `mob_candidates`, `body_mut`, `body_of`, `damage_state`, `mob_effects`, `seen_all`, `arrow_targets`, `can_be_leashed`, `mob_on_ray` as `MobHit::Family(39, id)`, `MobHit::id`.
- `a_registered_mob_takes_melee_arrows_blasts_magic_and_fire`: a player attack (hurt, knockback, blame, credit), an arrow hit (knockback scaled by the resistance), a creeper blast (hurt and push), harming and healing splashes (undead flips them), poison refused when immune, burning every 20 ticks unless fire immune, drowning, suffocation.
- `a_registered_mob_dies_with_its_table_experience_and_voices` (`take_deaths` order, `Some(kind)`, experience by the hook, `entity.<kind>.hurt` and `.death` from `take_sounds`).
- `a_registered_mob_despawns_by_its_rule_leaves_with_its_chunk_and_is_copied_where_kept` (`check_despawn` far and on peaceful, `remove_where`, `clone_where`, the 20-tick death retain).
- `a_removed_registered_mob_leaves_without_a_death` (`core.removed`: gone from `order` after the tick, no `take_deaths` entry, no experience).
- `a_lead_pulls_a_leashable_registered_mob` (`can_be_leashed`, `leash_yaw`, `leash_walk` called, not while `panicking` or without AI; a baby refuses the lead through `mob_baby`).
- `a_fireball_spares_a_fire_immune_registered_mob` (the `fireball_damages_mob` arm).
- `a_hurt_registered_monster_turns_on_its_attacker` (`Families::hurt` sets `core.hurt_by`, `family_goal_step` hands it to `HurtByTargetGoal`, and the head turn comes back into `core.look`).
- `two_registered_mobs_push_apart_and_cram` (sections filed; cramming through `exposed_mut`).
- `an_iron_golem_hunts_an_enemy_registered_mob` (`is_enemy`, `Prey::Enemy` over `mob_candidates`).
- `a_registered_monster_runs_its_goals_and_strikes_a_player` (`MonsterKind::Registered`, `running_goals` names, `retain_goals`, a `PlayerHitKind::Melee`, `GoalExtra` read and written by a goal).
- `prey_kinds_targets_the_named_kinds_only`.
- `a_registered_mount_seats_a_rider_and_lets_it_off` (`carries_riders`, seat point, `start_riding` limit, `stop_riding` spot).
- `a_player_steers_and_jumps_a_registered_mount` (`controlling_player` from `controller`, `ride_jump_scale` through `jumpable_by`, `rider_jump` on release; a mob controller passes its target through `order_vehicle`).
- `a_typed_hit_reaches_the_player` (fields carried) and, in survival, `entities::tests::a_typed_hit_skips_armour_and_shields_when_told_and_gives_its_effect` and `death::tests::a_typed_attack_reads_its_own_death_line` (the sonic boom line from the language file).
- Existing tests stay green unchanged (they pin every old family), except the one terrain test that uses the silverfish as a dormant monster (D0-9.4).

**D0-9.2 Vendored world (group `registry-spawning`):**
- `natural_spawner::families::tests::a_family_row_comes_before_the_built_in_tables` (silverfish: size, category, follow range, `type_width`).
- `a_row_overrides_a_built_in_kind` (a `#[cfg(test)]` row for `minecraft:goat` is chained into `rows()` with `#[cfg(test)]` on the statement).
- `silverfish_spawn_by_any_light_from_spawners_and_never_near_a_player` (natural refused within 5 blocks of a player, spawner true, peaceful refused).
- `a_family_finalize_runs_for_summons_and_natural_spawns_and_shares_group_data` (the monster path with `Some((context, difficulty))`, the creature path with `None`).
- `a_row_without_finalize_or_babies_is_supported_and_draws_nothing` (a silverfish summon logs no `note_unsupported`; a `baby_chance: None` row makes no baby draw from the level random).

**D0-9.3 Silverfish and endermites live (group `silverfish-live`):**
- `a_silverfish_is_a_small_eight_health_monster_that_bites_for_one`, `an_endermite_is_a_small_eight_health_monster_that_bites_for_two`.
- `a_hurt_silverfish_alerts_the_others`.
- `an_endermite_leaves_after_2400_ticks_unless_persistent`.
- `an_enderman_hunts_an_endermite`.
- `silverfish_and_endermites_give_five_and_three_experience`.

**D0-9.4 Client registry (groups `registry-client` and `registry-draw`):**

The `#[cfg(test)]` probe is compiled only in the entities crate's own tests, so the terrain and survival tests run on silverfish and endermites. Those use no `FamilyShow` values, no `interact`, no mount and no lead, so the terrain-side arms for them (`show_on` with values, `families::interact`, `is_vehicle` in `carries`, `mount_inventory`) get no real-family test in D0. The first track whose family uses one (M2's nautilus for mounts and the inventory, any family with a synced flag for `FamilyShow`, any right-click family for `interact`) adds the terrain test for that arm in its family files; D0 lists this under "left for later".
- `server_mobs::tests::a_silverfish_and_an_endermite_save_and_load_with_its_lifetime`.
- `every_registered_kind_is_simulated_and_summonable` (`is_simulated`, `simulated_kinds`, `/summon` suggestions).
- The eggs survey hatches silverfish and endermites (gated on `MINECRAFTOSS_ROOT`, as today).
- `mob_tracking::tests::a_friend_sees_a_silverfish_and_an_endermite_from_their_look_tags` (`Lifetime` volatile, `tracking_range` 8, `show_on` places them).
- `minecraft_survival` `remote` or `entities` test: a `ServerMob` with `family: Some(FamilyShow { .. })` round-trips through `minecraft_net` encode and decode.
- `entities::tests::every_registered_family_hit_key_encodes_and_decodes` (all `has_key` bytes).
- `entity_view::tests::a_registered_mob_is_shot_by_its_family_hit_box`.
- `entity_view::tests::a_glowing_registered_mob_is_outlined` (the generic glow loop).
- `mob_tracking::tests::a_registered_look_tag_drops_only_its_own_volatile_keys`.
- `spawner_display` cage test over every registered kind.
- `keyframes::tests::the_shared_library_reads_after_install_and_says_why_before` (gated part on `MINECRAFTOSS_ROOT`).
- `entities::tests::a_player_cannot_sleep_beside_a_pillager_vex_ravager_or_wither`.
- `minecraft_net` codec tests stay at protocol 6 (no bump in D0).
- `server.rs` `live_shulkers_stay_far_from_players_while_dormant_monsters_go` (`:8169`) summons `minecraft:silverfish` as its **dormant** monster and asserts it lands in `server.dormant`. Once silverfish are simulated (commit 1 of this group) that test fails. The same commit switches it to `minecraft:phantom`, a monster in `MobCategory::of_type` that stays dormant until M3. M3's phantom commit must switch it again (to any monster still dormant then) or drop that half of the test; the phantom row of D0-8 says so.

**D0-9.5 Drawn (group `silverfish-drawn`):** `silverfish_render::tests` (segments, layers, textures, the wiggle against hand-computed angles), `endermite_render::tests`, `an_endermite_gives_off_portal_particles_for_every_viewer`.

**D0-9.6 Behaviour (group `silverfish-behaviour`):**
- `breaking_infested_stone_without_silk_touch_releases_a_silverfish` (and not with Silk Touch, not with `block_drops` off).
- `a_blast_on_infested_stone_releases_a_silverfish`.
- `a_hurt_silverfish_wakes_infested_blocks_nearby` (with and without `mob_griefing`).
- `an_idle_silverfish_merges_into_stone`.
- `about_one_pearl_in_twenty_leaves_an_endermite_where_its_thrower_stood` (seeded; none on peaceful).

**D0-9.7 Flying (group `flying-navigation`):**
- `a_flying_path_goes_over_a_wall_and_through_a_gap_in_three_dimensions`.
- `a_flying_path_ends_on_a_stable_spot`.
- `flying_move_control_turns_and_climbs_like_vanilla` (`maxTurn` 10 and 20, `hoversInPlace`).
- `a_flying_path_gives_up_when_stuck`.
- The wither tests keep passing (or move with the optional commit).

**Runs:** `check -p` and `test -p` for `minecraftoss-entities`, `minecraftoss-world`, `minecraft_terrain`, `minecraft_survival`, `minecraft_net` and `render_anim` as each commit touches them; `check -p launcher` before each commit that touches game code; clippy on the touched crates once per track (`-A clippy::approx_constant`); `scripts\full-suite.ps1` after the merge.

---

## D0-10. The owner's in-game steps and kit

Setup: release build, survival, **Normal** difficulty, by day, flat open ground in the Overworld, `/gamerule mob_griefing true`. The kit below in `.env`; the class loadout's M4A1 is used. Steps R-14 to R-17 need two windows (`scripts\mp-local.ps1`). Name each screenshot by its label, `R1.png`; for both windows `R14-host.png` and `R14-friend.png`.

Silverfish and endermites:
- **R-1.** Use a silverfish egg three times on the ground 6 blocks away. Three small grey segmented silverfish wiggle towards you and bite (half a heart). **Screenshot R1.**
- **R-2.** Punch one once without killing it: it flashes red and is knocked back, and the other two also turn on you. Kill one with the sword: it leaves experience. Shoot one with the bow and one with the M4A1: both flash and die.
- **R-3.** Use an endermite egg: a small purple mite with purple sparkles around it comes at you (a heart a bite). Use an enderman egg 5 blocks away: the enderman goes for the endermite and kills it. **Screenshot R3.**
- **R-4.** Use an endermite egg in a cobblestone pen and wait two minutes without touching it: it vanishes.
- **R-5.** Stand still and throw ender pearls one by one at the ground 10 blocks away. On average one pearl in twenty leaves an endermite where you stood before the teleport; keep throwing until one appears. **Screenshot R5.** `/difficulty peaceful`: no endermite, and the silverfish and endermites vanish. `/difficulty normal`.
- **R-6.** Build a wall of infested stone bricks and infested stone 3 high and 5 wide. Hatch a silverfish beside it and hit it once without killing it: within a second some of the wall's blocks crack and more silverfish crawl out. **Screenshot R6.** Mine an infested stone block with the pickaxe: a silverfish pops out.
- **R-7.** Build a pen of stone bricks 4 x 4 x 2 with a silverfish inside, step back and `/gamemode creative`. Within about a minute the silverfish dives into a wall block and is gone, with a puff. `/gamemode survival`, mine the blocks of that wall: one of them releases it again.
- **R-8.** Pour lava beside two silverfish: they burn, flash and die. Light TNT among three silverfish: the blast throws and kills them. Dig a hole 1 wide and 2 deep, hatch a silverfish in it, pour water in and put a block on top: after about 15 seconds it flashes red every second and drowns.
- **R-9.** Throw a splash potion of harming on a silverfish: it dies. Throw the poison splash on another: green swirls, it drops to half a heart and keeps biting.
- **R-10.** Use an iron golem egg near three silverfish: the golem walks to them and kills them.
- **R-11.** Place a spawner and use a silverfish egg on it: a tiny silverfish turns in the cage, and silverfish spawn around it. **Screenshot R11.** Break the spawner.
- **R-12.** Leave two silverfish and one endermite alive, save and quit, load the world: they are still there and move. The endermite still vanishes two minutes after it hatched.
- **R-13 (optional).** `/locate structure minecraft:stronghold`, `/tp` there and dig down to the portal room: silverfish come out of its spawner. **Screenshot R13.**

Friend (two windows):
- **R-14.** The host hatches two silverfish and an endermite. The friend sees them in the same places with the same wiggle and the endermite's sparkles. The friend shoots one with the M4A1 and hits another with the sword: both windows show the red flash and the kills. **Screenshots R14-host and R14-friend.**
- **R-15.** The friend throws pearls until an endermite appears where the friend stood; the host sees it too. The friend mines infested stone: a silverfish comes out in both windows.

Existing mobs still work (regression, two windows):
- **R-16.** Hatch a zombie, a creeper, a cow and a villager. The zombie comes at you and burns in daylight, the creeper hisses and blows up, the cow wanders, and the villager walks about. Shoot the zombie and the cow with the M4A1 and the bow, and light TNT near the others: they all flash, are pushed and die like before. **Screenshot R16.**
- **R-17.** Save and quit with a zombie and a cow alive, load: both are there, and the friend sees them and can hit them.

Thrown things (snowballs, eggs, tridents) hit silverfish and endermites only once E2's group `ti1` is merged; the release that has both tests it then.

```
IW4L_MINECRAFT_KIT=minecraft:silverfish_spawn_egg*16,minecraft:endermite_spawn_egg*16,minecraft:infested_stone*32,minecraft:infested_stone_bricks*32,minecraft:infested_deepslate*16,minecraft:stone_bricks*64,minecraft:spawner*2,minecraft:ender_pearl*16,minecraft:enderman_spawn_egg*2,minecraft:iron_golem_spawn_egg*2,minecraft:villager_spawn_egg*2,minecraft:zombie_spawn_egg*4,minecraft:cow_spawn_egg*4,minecraft:creeper_spawn_egg*2,minecraft:iron_sword,minecraft:bow,minecraft:arrow*64,minecraft:diamond_pickaxe,minecraft:tnt*8,minecraft:flint_and_steel,minecraft:lava_bucket,minecraft:water_bucket,minecraft:splash_potion@harming,minecraft:splash_potion@poison,minecraft:shield,minecraft:cooked_beef*32,minecraft:torch*64
```

27 stacks; every id checked against the 26.3 item models (`assets/minecraft/items`). R-15's friend needs the same kit (it reads the `.env` on join).

---

## D0-11. Commit groups (track runner)

| # | Group key | Commits | Section |
|---|---|---|---|
| 1 | `registry-core` | 1. Round 10 mob list with tracks, families and hit keys. 2. Minecraft mob families plug into the entity world through one registry. | D0-2.1 to D0-2.3, D0-2.7, D0-8, D0-9.1 (walks, combat, deaths, despawn, pushing, leashes, fireballs, removal) |
| 2 | `registry-hooks` | 1. Registered Minecraft mobs use the shared monster goals and targets. 2. Registered Minecraft mobs can carry riders. 3. Minecraft mobs can hurt players with their own damage types. | D0-2.4 to D0-2.6, D0-9.1 (goals, prey, mounts, typed hits) |
| 3 | `registry-spawning` | Minecraft mob families bring their own spawn rules. | D0-4, D0-9.2 |
| 4 | `silverfish-live` | Minecraft silverfish and endermites live in the entity world. | D0-7.1, D0-9.3 |
| 5 | `registry-client` | 1. Registered Minecraft mobs are saved, loaded and hatched. 2. Friends see registered Minecraft mobs (no `PROTOCOL` bump; "## Protocol" in the log). | D0-5.1, D0-5.2, D0-9.4 |
| 6 | `registry-draw` | 1. Registered Minecraft mobs are drawn, shot and used through one registry (with the keyframe holder). 2. Minecraft players cannot sleep beside pillagers, vexes, ravagers or withers. | D0-5.3, D0-5.4, D0-6.1, D0-9.4 |
| 7 | `silverfish-drawn` | Minecraft silverfish and endermites are drawn like vanilla. | D0-7.2, D0-9.5 |
| 8 | `silverfish-behaviour` | 1. Minecraft silverfish hide in stone and wake their friends. 2. Endermites come from ender pearls. | D0-7.3, D0-9.6, D0-10 |
| 9 | `flying-navigation` | 1. Minecraft mobs can fly with vanilla flying navigation. 2. (optional) Minecraft withers fly around walls. | D0-6.2, D0-9.7 |

Groups 1 and 2 were one group, and so were 5 and 6; each half is a full agent's work (group 1 alone touches about 60 sites in 12 files plus the macros and the probe), so they run as separate groups.

Every group updates `docs/round10/track-d0-log.md` (javap numbers, decisions, vendored files, test runs) and its `mobs.md` rows in the same commit as the code. Group 6 writes the owner's steps and the kit (D0-10) into the log.

---

## D0-12. Not in D0, and notes for the other briefs

- **Boss bars stay wither-only.** F generalises `wither_bar.rs` and the four pieces after it (`docs/round9/track-d-log.md`, group wither-fight commit 2) if its boss needs a bar. F's boss can use the registry (group `bosses`, bytes 96-99) once D0 is merged.
- **No block-behaviour registry.** Block hooks (eggs, hives, frogspawn, creaking hearts, dried ghasts, copper chests, sculk) stay in the level code; the checklist's "next to a related arm, logic in a new file" rule keeps those merges small.
- **Entity-event particles** (taming hearts, the silverfish poof) are E1's `mob_particles::puff_for`; one row per mob event, next to a related row.
- **The old families stay as they are.** Thrown things reach them through E2's `hit_boxes`; their own lists are not converted.
- If a mob track finds a site the registry misses, it adds the hook to the root file in its own small commit (named for the hook) and notes it in its log; the next merge carries it to the other tracks.
