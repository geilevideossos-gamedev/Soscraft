# Crate split: `minecraft_survival` and `minecraft_net`

**Done in `5a7a4e4`..`636e433`** (A1 to C5 on `crate-split`, based on `70a01b2`), followed by the docs commit D1. No step was skipped: the optional B7 and C5 stayed clean pure moves, so both were done. D1 updated only [MINECRAFT-SURVIVAL.md](MINECRAFT-SURVIVAL.md), because PROGRESS.md belongs to the round's orchestrator. The commit-by-commit record is in [round7/track-split-log.md](round7/track-split-log.md).

This is roadmap item 1.2 in [../CLAUDE.md](../CLAUDE.md) and Phase 1 item 4 in [PROGRESS.md](PROGRESS.md). It is the plan that agents follow commit by commit. Every line number refers to `d426239` on `minecraft-survival`.

**Goal.** The Minecraft game state leaves `crates/render_anim` for a new crate without bevy, `crates/minecraft_survival`. It is split into three kinds of state:

- one `Server` for the whole world;
- one `Level` per dimension;
- one `Player` per UUID.

`render_anim` keeps a thin Bevy adapter. A second new crate, `crates/minecraft_net`, holds the serde ids now and the network messages later. No commit changes behaviour.

**Why this comes before the dimension switch** (from CLAUDE.md):

- Travel has to drop and load a `Level` while the `Player` survives.
- Multiplayer needs several levels loaded at once.
- Both rewrite `load()`, `stop()`, `Entities` and `Runtime`.

**Done means:**

- Every Minecraft game-state type lives in `minecraft_survival` or `minecraft_terrain`.
- `render_anim` keeps only these:
  - presentation;
  - the MW2 bridges (`sim::voxel`, `AuthorityWorld`);
  - two documented exceptions: the menu half of `InventoryUi` and `Mining.progress` (section 6).
- The per-tick player-action code in `update()` still runs in `render_anim`, but on the new types. Moving it needs input messages, which come with multiplayer (section 6).
- All 149 `render_anim` tests still pass, split between the two crates (table in section 3).

## 0. Rules for every commit

- **Branch and commits.**
  - Work on `minecraft-survival` and make one commit per step below.
  - Use the subjects given here (plain English, no prefix) and end each message with the `Co-Authored-By` line.
  - Push only to `origin`.
- **Cargo.** Run every command from PowerShell through `scripts\cargo-slot.ps1`. It adds `-j 8`, uses `D:\iw4l-target` and holds a machine-wide slot. Run one command at a time.
  - `CHECK` is `& scripts\cargo-slot.ps1 check -p launcher --locked`. Every commit must pass it.
  - `T(x)` is `& scripts\cargo-slot.ps1 test -p x --locked`.
  - A commit that changes a `Cargo.toml` dependency list (A1 and B1) also changes `Cargo.lock`. Run `& scripts\cargo-slot.ps1 check -p launcher --offline` once without `--locked`, check that `git diff Cargo.lock` only adds the new path packages and their dependency lines, and commit the lock file.
- **Pure move.**
  - Use `git mv`, then change only these lines:
    - `use` and path lines (`crate::minecraft_x::` becomes `minecraft_survival::x::` in `render_anim`, and `crate::x::` inside the new crate);
    - `pub(crate)` becomes `pub`.
  - Do path edits with exact-token `sed` in Git Bash, for example `sed -i 's/crate::minecraft_save::/minecraft_survival::save::/g' <files>`.
  - Never reflow a line.
- **Formatting and comments (owner decision 1).**
  - Proposed default: a `git mv` rename that git reports as `R` counts as an existing file. It is not reformatted and its `//!`/`///` comments stay, as for any edit to an existing file.
  - Code written into a new path is a new file: a split-off half, or a function moved out of `minecraft_world.rs`. It has no comments of any kind and must pass `rustfmt --check`, and running `rustfmt --edition 2024 <file>` on it is allowed.
  - If the owner rules that renames are new files, add one "Format ..." commit right after each move.
  - Today `minecraft_death.rs` and `minecraft_offhand.rs` already pass `rustfmt --check`. `minecraft_save.rs` has 10 diff hunks, `minecraft_survival.rs` 21 and `minecraft_entities.rs` 69 (measured with `rustfmt --check --edition 2024`).
- **How to run the rustfmt check.**
  - Use Git Bash; rustfmt is not cargo.
  - For a new leaf module: `rustfmt --check --edition 2024 crates/minecraft_survival/src/<file>.rs`. Exit 1 means a diff.
  - For a new `lib.rs`, file mode follows `pub mod x;` into the moved children, so check the staged content: `git show :crates/minecraft_survival/src/lib.rs | rustfmt --check --edition 2024`.
  - In stdin mode rustfmt 1.10.0-stable prints the diff but **exits 0**, so the output must be empty (checked on `minecraft_save.rs`).
- **Line endings:** LF (`rustfmt.toml` sets `newline_style = "Unix"`).
- **Tests are the owner's.** Never delete or weaken one. A test may move to another crate or have its setup adapted to a new signature, and the totals in section 3 must hold.
- **No other edits** to the moved files while the split is in flight. The round-7 tracks branch off after it (PROGRESS.md "Phase 2 (after the crate split)").

## 1. Target layout

### 1.1 Crates

| Crate | Edition | Lints | Contents |
|---|---|---|---|
| `crates/minecraft_net` (new) | workspace (2024) | workspace | Serde ids now: `Dimension` and `PlayerUuid`. Network messages come later. No transport and no game logic. |
| `crates/minecraft_survival` (new) | workspace (2024) | workspace | Headless game state and rules without bevy, moved from `render_anim` (section 1.3). |
| `crates/minecraft_terrain` | **2021** (`minecraft_terrain/Cargo.toml:4`), no `[lints]` | none | Unchanged apart from re-exporting `Dimension` from `minecraft_net` (B1). It keeps the server thread, containers, crafting, stations, game modes, item-use rules and all meshing. |
| `crates/minecraft_commands` | workspace | workspace | Unchanged: the command tree, `CommandServer`, the `CommandWorld` trait (`minecraft_commands/src/server.rs:60-92`) and the serde chat protocol (`protocol.rs`). |
| `crates/render_anim` | workspace | workspace | The Bevy adapter (section 5), presentation and the MW2 bridges. |

`minecraft_survival` uses edition 2024 because everything moved into it comes from `render_anim`, which is already 2024 and uses let-chains (`minecraft_world.rs:284-285` and throughout). Nothing from the 2021 `minecraft_terrain` moves.

### 1.2 Dependency graph (no cycles)

```
minecraft_net       -> serde
minecraft_terrain   -> minecraft_net (new in B1), minecraftoss-core/world/generator/player/entities, glam 0.29, image, zip, ...
minecraft_commands  -> md5, minecraftoss-player, minecraftoss-world, serde, serde_json          (unchanged)
minecraft_survival  -> minecraft_net, minecraft_terrain, minecraft_commands,
                       minecraftoss-core, minecraftoss-entities, minecraftoss-player,
                       playerstate_iw4, diag, glam "0.29", serde_json
render_anim         -> minecraft_survival + its current dependencies (render_anim/Cargo.toml:9-51)
hud, render, sim, net, frame                                                                   (unchanged)
```

- **`minecraft_survival` must not depend on:**
  - `bevy`;
  - `frame`, which pulls in bevy (`frame/Cargo.toml:9`);
  - `sim`, which has `bevy_ecs` (`sim/Cargo.toml:9`) and the process-wide voxel statics (`sim/src/voxel.rs:41`);
  - `net`, `audio` or any `render_*` crate;
  - `assets`, which pulls in `render_material`, `xmodel_runtime` and the fastfile crates (`assets/Cargo.toml`).
- **`playerstate_iw4` is allowed.** It is plain data plus `math_iw4` and `libm` (`playerstate_iw4/Cargo.toml`), and `death.rs` needs its `PM_TYPE_*` constants (`minecraft_death.rs:386-396` and the tests at 889-913).
- **glam must be `glam = "0.29"`, never `glam.workspace = true`.** The workspace pins 0.32.1 (`Cargo.toml:173`), while minecraftoss-player and minecraftoss-entities pin `=0.29.3`. Workspace glam breaks every `DVec3` that crosses into them.
- **`minecraft_net` stays serde-only.** That way `net` (MW2) or the relay can depend on it later without a cycle. It takes a `minecraft_commands` dependency only when chat payloads become messages.

### 1.3 `minecraft_survival` modules (end state)

| Module | Comes from | Holds | Tests |
|---|---|---|---|
| `save.rs` | `git mv render_anim/src/minecraft_save.rs` (A2) | `WorldSave`, `open_world`, `AUTOSAVE_TICKS` | 12 |
| `death.rs` | `git mv minecraft_death.rs` (A3) | `DamageCause`, `CombatTracker`, `DeathWatch`, `death_drops`, `life_state` | 24 |
| `guns.rs` | new (A4), from `minecraft_inventory.rs:23,28-36` | `WEAPON_PREFIX`, `HOTBAR`, `weapon_of`, `weapon_item` | 0 |
| `offhand.rs` | `git mv minecraft_offhand.rs` (A4) | shield, totem and hand-use rules | 10 |
| `survival.rs` | `git mv minecraft_survival.rs` (A6) | `Survival`, `SurvivalRules`, `Motion`, `Vitals` (new in A5) | 61 |
| `entities.rs` | `git mv minecraft_entities.rs` (A8) | `Entities`: the level's integrated server handle and its mirrors | 6 |
| `commands.rs` | new (A9), from `minecraft_chat.rs:12-317` | `HOST`, `Host` (`CommandWorld`), `pump`, `summon_answers`, `registries`, `give_stacks`, `death_message` | 2 |
| `level.rs` | new (B2) | `Level`, `Terrain` | - |
| `server.rs` | new (B2-B3, C1) | `Server`, `Globals`, saving and closing | - |
| `data.rs` | new (B4) | `GameData`: recipes, loot, stations and enchantments from the jar | - |
| `player.rs` | new (B5-B7) | `Player` | - |

`lib.rs` holds only `pub mod` lines. The module names drop the `minecraft_` prefix, so the moved `minecraft_survival.rs` does not clash with the crate name.

### 1.4 `minecraft_net`

- **B1 adds:**
  - `Dimension`, moved from `minecraft_terrain/src/terrain.rs:975-999`. It keeps the variant names `Overworld`, `Nether` and `End`, `parse` and `dimension_type`, and adds the derives `Hash`, `PartialOrd`, `Ord`, `Serialize` and `Deserialize`.
  - `PlayerUuid(pub String)`, a hyphenated offline UUID as `minecraft_commands::offline_uuid` returns it (`players.rs:18`).
  - `terrain.rs` re-exports it with `pub use minecraft_net::Dimension;`, so `minecraft_terrain::terrain::Dimension` (`minecraft_world.rs:18`, `examples/smoke.rs:23`) still resolves.
- **Not in the split:**
  - the server snapshot types (`ServerFallingBlock`, `ServerTnt` and `ServerOrb` at `server.rs:126-177`, imported by `falling_render.rs:3` and `orb_render.rs:6`);
  - the click, action and chunk messages.
  
  They come with multiplayer (section 6).

### 1.5 What stays where

- **`render_anim`:**
  - `minecraft_world.rs`: the adapter, with `register`, `Runtime`, `load()`, `update()`, `stop()` and `data_jar()`;
  - `minecraft_chat.rs`: `show` and the 5 tests that use it;
  - `minecraft_inventory.rs`: `InventoryUi`, which uses bevy `Image`/`Assets` (`minecraft_inventory.rs:8-12`);
  - `minecraft_mining.rs`, which uses `sim::voxel::VoxelEvent` and particles (`minecraft_mining.rs:21,59,233`);
  - `minecraft_sounds.rs`, which uses `audio::McSoundRequest` (:28);
  - `minecraft_spawn_gate.rs`, `minecraft_hand.rs`, `minecraft_minimap.rs`, `minecraft_gui.rs` and `minecraft_camera.rs`.
- **`minecraft_terrain`:** everything, including:
  - `server.rs`, `containers.rs`, `crafting.rs`, `stations.rs` and `game_mode.rs`;
  - the item-use modules;
  - `TerrainStream`.
  
  Moving them would mix editions and break two seams. `scene.rs:3` imports `terrain::BlockStates` while `terrain.rs:20` imports `scene`. `falling_render.rs:3` and `orb_render.rs:6` import `server` types.
- **`minecraft_commands`:** everything. `hud` keeps using it for chat input (`hud/src/minecraft_chat.rs:6`).
- **`sim`:** all voxel statics and queues (`sim/src/voxel.rs:41`, 126-378).

### 1.6 Old path to new path (for the round-7 briefs in `docs/round7/`)

| Before | After |
|---|---|
| `render_anim/src/minecraft_save.rs` | `minecraft_survival/src/save.rs` |
| `render_anim/src/minecraft_death.rs` | `minecraft_survival/src/death.rs` |
| `render_anim/src/minecraft_offhand.rs` | `minecraft_survival/src/offhand.rs` |
| `minecraft_inventory.rs` `weapon_of`, `weapon_item` and `WEAPON_PREFIX` | `minecraft_survival/src/guns.rs` |
| `render_anim/src/minecraft_survival.rs` | `minecraft_survival/src/survival.rs` (`vitals()` now returns `Vitals`) |
| `render_anim/src/minecraft_entities.rs` | `minecraft_survival/src/entities.rs` (`data_jar` stays in `minecraft_world.rs`) |
| `minecraft_chat.rs` `Host`, `HOST`, `pump` and the other command code | `minecraft_survival/src/commands.rs` (`show` stays) |
| `minecraft_terrain/src/terrain.rs` `Dimension` | `minecraft_net/src/lib.rs`, re-exported at the old path |
| `Loaded.stream`, `.scene` and `.environment` | `Level.terrain.*` |
| `Runtime.entities` and `.containers` | `Level.entities` and `Level.containers` |
| `Loaded.seed`, `.registries`, `.save`, `.game_rules`, `.difficulty`, `.chat` and `.level_writer`, plus `Runtime.day` and `.autosave_ticks` | `Server.globals.*` |
| `Runtime.survival`, `.player_mode` and `.death_watch` | `Player.survival`, `Player.mode` and `Player.death_watch` |
| `Entities.inventory` and `.selected` | `Player.inventory` and `Player.selected` |
| the kit (`minecraft_entities.rs:213-216`) | player creation (B6) |
| `spill`, `write_blocks`, `finish_item_use`, `hold_off_death`, `autosave`, `save_and_close`, `save_player_mode` and `starting_game_mode` | `level.rs` and `server.rs` (C1-C2) |

## 2. State model

Vanilla 26.3 has this shape, checked with javap (JDK 21 at `C:\Program Files\Eclipse Adoptium\jdk-21.0.8.9-hotspot`; the JDK 8 `javap` on PATH can't read 26.3 classes):

- `net.minecraft.server.MinecraftServer` owns `levels: Map<ResourceKey<Level>, ServerLevel>`, one `weatherData` and one `clockManager`.
- `ServerLevel.getWeatherData()` returns the server's copy.
- `WeatherData` is the saved-data id `weather`.
- In the jar's `data/minecraft/dimension_type/*.json`, the overworld's `default_clock` is `minecraft:overworld` and the End's is `minecraft:the_end`. The Nether has none, and its `coordinate_scale` is 8.0.

So weather and clocks are global, not per level. Our model follows that shape.

### 2.1 `Server` (one per open world, `minecraft_survival::server`)

```rust
pub struct Server {
    pub globals: Globals,
    pub levels: BTreeMap<Dimension, Level>,
    pub players: BTreeMap<PlayerUuid, Player>,
}
pub struct Globals {
    pub seed: i64,
    pub registries: Arc<Registries>,
    pub save: Option<WorldSave>,
    pub game_rules: GameRules,
    pub difficulty: Difficulty,
    pub day: DayCycle,
    pub chat: CommandServer,
    pub data: GameData,
    pub level_writer: Option<JoinHandle<()>>,
    pub autosave_ticks: u32,
}
```

- **The three fields are separate on purpose.**
  - `update()` destructures `Server { globals, levels, players }`, so a `&mut Level` and a `&mut Player` can be held while `globals.*` is used.
  - Helpers take the part they need (`&mut Globals`, `&mut Level`, `&mut Player`), never `&mut Server`.
- **`day`** is the `minecraft:overworld` clock. `world_clocks` already stores clocks by id (`minecraft_save.rs:164-169`). A map of clock ids, with `minecraft:the_end` added, comes with the End.
- **Weather** stays where it runs today, in the overworld level's `ServerSim` (`Entities.weather`, `minecraft_entities.rs:96`). Moving it up to `Globals`, as vanilla's `MinecraftServer.weatherData`, is part of the dimension switch.
- **`chat: CommandServer`** already maps `PlayerId(u32)` to `Profile { id, name, uuid }` (`minecraft_commands/src/server.rs:95-110`). It is the server's player list for names and ops.
- **The world spawn** (`TerrainStream.world_spawn`, `terrain.rs:946`) is global in vanilla (`level.dat` `spawn` with a dimension). It is still read from the overworld level's stream.

### 2.2 `Level` (one per dimension, `minecraft_survival::level`)

```rust
pub struct Level {
    pub dimension: Dimension,
    pub entities: Option<Entities>,
    pub containers: Containers,
    pub terrain: Terrain,
}
pub struct Terrain {
    pub stream: TerrainStream,
    pub scene: HandcraftedScene,
    pub environment: DimensionEnvironment,
}
```

- **Field order is drop order.**
  - `entities` must drop before `terrain`. Dropping `Entities` stops the level's "Server thread" (`server.rs:1222-1235`), which saves entities into the `ChunkStorage` that the chunk map shares (`server.rs:49-51`, `terrain.rs:949-951`).
  - Dropping `TerrainStream` saves chunks.
  - Today `save_and_close` drops `entities` first (`minecraft_world.rs:2467`) and the stream later (:2482).
- **`entities` stays an `Option`** so that the ~30 `entities.as_mut()` sites in `update()` stay as they are. It is `Some` from load completion on.
- **`Terrain` is a struct of its own** so that `world.scene` and `world.stream` turn into `terrain.scene` and `terrain.stream` by token replacement, with the same place types, while `entities` and `containers` are borrowed from the same `Level`.
- **`TerrainStream` stays one object.** It mixes the server `ChunkMap` (`terrain.rs:949`) with the client section workers (:952-967), and the host needs both. Splitting it belongs to multiplayer.

### 2.3 `Player` (one per UUID, `minecraft_survival::player`)

```rust
pub struct Player {
    pub uuid: PlayerUuid,
    pub id: PlayerId,
    pub name: String,
    pub dimension: Dimension,
    pub survival: Survival,
    pub mode: PlayerMode,
    pub death_watch: DeathWatch,
    pub inventory: Inventory,
    pub selected: usize,
    pub hand: Hand,
    pub steps: Steps,
}
```

| Vanilla player data | Home after the split | Notes |
|---|---|---|
| Inventory, armour and offhand | `Player.inventory` (B6) | `Inventory.slots` holds 0-35 storage, 36-39 armour, 40 offhand, 41 body and 42 saddle (`third_party/minecraftoss/player/src/inventory.rs:76-80`) |
| Selected slot | `Player.selected` (B6) | |
| Ender chest | `Containers.ender` for now (`containers.rs:431`) | Track D commit 8 ("Each Minecraft player has their own ender chest") moves it to the player, so the split leaves it alone |
| Health, absorption, hunger, saturation, XP, effects, air and fire | `Player.survival` (B5) | `Survival.status: SurvivalStatus` (`minecraft_survival.rs:92`) plus private timers |
| Game mode and abilities | `Player.mode` (B5) | `PlayerMode` (`game_mode.rs:90`) |
| Position and rotation | the MW2 `PlayerState` in `AuthorityWorld` | The adapter reads it every frame (`minecraft_world.rs:566-571`, 657, 667). Saving it is track D commit 7 |
| Dimension | `Player.dimension` (B5) | Always `Overworld` until the dimension switch |
| Spawn point | `TerrainStream.player_spawn` (`terrain.rs:948`), which is also the MW2 map origin (`minecraft_world.rs:457-458`) | A per-player respawn is track D commit 9. The origin stays in the adapter |
| Per-player outputs of the level server: picked stacks, pearl landings, XP found, item-use waits, eggs, summon answers | stay in `Entities` (`minecraft_entities.rs:79,93,95,99-102`) | `Output` has no player key yet (`server.rs:1167-1205`). This is multiplayer work |

- **`Hand` and `Steps`** are today's `HandState` and `StepState` (`minecraft_world.rs:93-124`), moved whole in B7. They hold game parts (place delay, pearl cooldown, attack ticks, miss time, blocking, creative delay, recent placements, air peak) and client parts (swing, the frame-to-tick `clock`, key edges, step distance). The client parts go back to the adapter when a fixed 20 Hz game tick exists.
- **Owner decision 2: survival carries over between matches.**
  - Today `Runtime.survival` is never reset. Neither the load completion (`minecraft_world.rs:448-528`) nor `stop()` (2500-2522) touches it, so health, hunger and XP carry from one Minecraft match to the next in the same process.
  - The split keeps that: `close_world` moves the local player's `Survival` into `Runtime.carried_survival`, and the next load moves it into the new `Player`.
  - Track D's player file replaces this with the saved stats.
  - Hand and step timers do reset on load after B7. That can't be seen, because the load takes longer than the 20-tick pearl cooldown.

### 2.4 The adapter (`render_anim`), per viewer

- **`Runtime` (NonSend) keeps:**
  - the lifecycle: `loading`, `loader` and `closing`;
  - `world: Option<Loaded>`, reduced to client assets: `packs`, `atlas`, `celestial`, `cloud_mask`, `crack_texture` and `glint_texture`;
  - `server: Option<Server>` (new);
  - `local: Option<PlayerUuid>` (new) and `carried_survival` (new);
  - presentation: `environment_*`, `light`, `light_volume_*`, `cloud_center`, `minimap`, `sounds`, `fire_frames`, `atlas_animation`, `animation_clock` and `spawn_gate`;
  - `inventory_ui` and `mining`;
  - the MW2 collision shape table `shapes`/`shape_ids` (`minecraft_world.rs:147-150`), which belongs to the process-wide `sim::voxel` world and is reset with it.
- **`light`** is client lighting, but placement's survive check reads it (`minecraft_world.rs:1297`), so the adapter passes it in.
- **The environment** is shared:
  - The game reads `water_evaporates` (1151) and `sky_light_level` (1504).
  - The sky reads the rest (1941-2090).
  - It lives in `Level.terrain`, and the adapter borrows it.

### 2.5 Field map

`Loaded` (`minecraft_world.rs:73-91`):

| Field | New home | Commit |
|---|---|---|
| `stream`, `scene`, `environment` | `Level.terrain` | B2 |
| `packs`, `atlas`, `celestial`, `cloud_mask`, `crack_texture`, `glint_texture` | stay in `Loaded` (client assets) | - |
| `registries`, `seed`, `save`, `game_rules`, `difficulty`, `level_writer`, `chat` | `Globals` | B3 |
| `saved_day` | read into `Globals.day` by the loader | B3 |

`Runtime` (`minecraft_world.rs:126-159`):

| Field | New home | Commit |
|---|---|---|
| `entities`, `containers` | `Level` | B2 |
| `day`, `autosave_ticks` | `Globals` | B3 |
| `survival`, `player_mode`, `death_watch` | `Player` | B5 |
| `hand`, `steps` | `Player.hand`, `Player.steps` | B7 (optional) |
| everything else | stays (section 2.4) | - |

`Entities` (`minecraft_entities.rs:57-106`) moves whole in A8 and lives in `Level.entities` from B2 on.

| Field | New home | Commit |
|---|---|---|
| `inventory`, `selected` | `Player` | B6 |
| `stations`, `enchantments`, the block `loot` book | `Globals.data` (as `Arc`s; `Entities` keeps clones) | B4 |
| `recipes` (new field, from `GameData`) | `Entities` | B4 |
| `server`, `world`, `world_items`, `server_handed`, `server_snapshot`, `server_handled`, `loot_sequences`, `seed`, `random`, `destroyed`, `weather`, `difficulty`, `spawn_mobs`, `block_drops`, `clock`, `ticks`, `sounds` | stay (per level) | - |
| `server_picked`, `pearl_landings`, `experience_found`, `item_use_waiting`, `item_uses_done`, `eggs_waiting`, `command_summons` | stay (per player; needs a player key in `Output`) | after the split |
| `client`, `poof`, `portal`, `items`, `burning_items`, `thrown`, `orbs`, `falling`, `orb_pickups` | stay (client mirrors; moved out when the host and its clients split) | after the split |

`InventoryUi` (`minecraft_inventory.rs:38-67`) stays whole in `render_anim`.

- Its menu fields are per-player game state: `table`, `block`, `bench`, `experience`, `spent`, `level`, `shelves`, `seed`, `used`, `anvil_used`, `book` and `enchantments`.
- The rest is presentation or MW2 glue.
- `apply_input` (383-538) consumes `frame::McClick` from `MinecraftUi`, so the menus can only move once a click message exists in `minecraft_net`.

`minecraft_terrain` state, where it ends up:

- `Containers` is per level, apart from `ender` (`containers.rs:431`), which belongs to track D.
- In `ServerSim`, `take_xp_delay` (`server.rs:62`) and `merchant_closing` (:64) are per player. They are multiplayer work, together with the hard-coded player `0` at `server.rs:299`, 357, 364, 734 and 1508-1516, and `players.first()` at 1519 and 1531.

## 3. Commits

Expected test totals: `T(...)` must report these numbers passed, plus any new tests. `minecraft_terrain` stays at 466 (3 ignored) and `minecraft_commands` at 53.

| After | `minecraft_survival` | `render_anim` |
|---|---|---|
| base `d426239` | - | 149 |
| A2 | 12 | 137 |
| A3 | 36 | 113 |
| A4 | 46 | 103 |
| A6 | 107 | 42 |
| A8 | 113 | 36 |
| A9 | 115 | 34 |

### Phase A: pure moves

**A1. "Empty minecraft_survival and minecraft_net crates"**

- **New files:**
  - `crates/minecraft_net/Cargo.toml`: `edition.workspace`, `license.workspace`, `serde = { workspace = true }`, `[lints] workspace = true`.
  - `crates/minecraft_survival/Cargo.toml` with the dependencies of section 1.2. `minecraft_terrain` and the three `minecraftoss-*` crates are path dependencies, as in `render_anim/Cargo.toml:43-46`.
  - An empty `src/lib.rs` in each.
- **Root `Cargo.toml`:**
  - Add the two members after `"crates/minecraft_commands"` (:46).
  - Add `minecraft_net = { path = "crates/minecraft_net" }` and `minecraft_survival = { path = "crates/minecraft_survival" }` after `minecraft_commands` (:151).
- **`render_anim/Cargo.toml`:** add `minecraft_survival.workspace = true` after :29.
- **Run:** `check --offline` (lock update), then `CHECK`.

**A2. "Minecraft world saving code moves to minecraft_survival"**

- `git mv crates/render_anim/src/minecraft_save.rs crates/minecraft_survival/src/save.rs`, then `pub(crate)` becomes `pub`, and add `pub mod save;`.
- `render_anim/src/lib.rs`: remove `mod minecraft_save;` (:51).
- `crate::minecraft_save::` becomes `minecraft_survival::save::` in `minecraft_world.rs` (7 sites) and `minecraft_chat.rs` (1).
- **Run:** `CHECK`, `T(minecraft_survival)`, `T(render_anim)`.

**A3. "Minecraft death rules move to minecraft_survival"**

- `git mv minecraft_death.rs` to `death.rs`, `pub(crate)` becomes `pub`, and add `pub mod death;`.
- `lib.rs`: remove `mod minecraft_death;` (:43).
- `crate::minecraft_death::` becomes `minecraft_survival::death::` in:
  - `minecraft_world.rs` (8);
  - `minecraft_entities.rs` (5);
  - `minecraft_survival.rs` (3, including its `use` at line 1);
  - `minecraft_chat.rs` (2).
- **Run:** `CHECK`, `T(minecraft_survival)`, `T(render_anim)`.

**A4. "Minecraft gun items and offhand rules move to minecraft_survival"**

- **New `guns.rs`** (rustfmt-clean, no comments):
  - `pub const WEAPON_PREFIX`, `pub fn weapon_of` and `pub fn weapon_item`, moved from `minecraft_inventory.rs:23,28-36`;
  - `pub const HOTBAR: usize = 9`, vanilla's selection size and the same value as `frame::minecraft_ui::MC_HOTBAR` (`frame/src/minecraft_ui.rs:11`), which `hud` keeps.
- **`minecraft_inventory.rs`:** delete those lines and add `use minecraft_survival::guns::{WEAPON_PREFIX, weapon_item, weapon_of};`. It uses them at 165, 181, 191, 529, 548, 561, 611, 742 and 852.
- **Move `minecraft_offhand.rs`** with `git mv` to `offhand.rs`.
  - In it, `crate::minecraft_inventory::weapon_of` (:106) becomes `crate::guns::weapon_of`, and `frame::minecraft_ui::MC_HOTBAR` (:108) becomes `crate::guns::HOTBAR`.
- **`crate::minecraft_offhand::` becomes `minecraft_survival::offhand::`** in `minecraft_world.rs` (23) and `minecraft_survival.rs` (2).
- **`crate::minecraft_inventory::weapon_of`** in `minecraft_world.rs:763` and :1817 becomes `minecraft_survival::guns::weapon_of`.
- **Run:** `CHECK`, `T(minecraft_survival)`, `T(render_anim)`.

**A5. "Minecraft vitals are plain data"** (seam, inside `render_anim`)

- In `minecraft_survival.rs`, add `pub(crate) struct Vitals` with exactly the fields and derives of `frame::McVitals` (`frame/src/minecraft_ui.rs:212-232`), and make `vitals()` (:762-789) build it.
- In `minecraft_world.rs`, add `fn mc_vitals(vitals: Vitals) -> frame::McVitals` and call it at :1650.
- It has to be a function: `impl From` would break the orphan rule once both types live in other crates.
- The 15 test uses of `.vitals().on_fire` and similar compile unchanged.
- **Run:** `CHECK`, `T(render_anim)`.

**A6. "Minecraft health, hunger and hazards move to minecraft_survival"**

- `git mv minecraft_survival.rs` to `survival.rs`, `pub(crate)` becomes `pub`, and add `pub mod survival;`.
- Its `minecraft_survival::death::` and `minecraft_survival::offhand::` paths become `crate::death::` and `crate::offhand::`.
- `lib.rs`: remove :54.
- `crate::minecraft_survival::` becomes `minecraft_survival::survival::` in `minecraft_world.rs` (15) and `minecraft_chat.rs` (3).
- **Run:** `CHECK`, `T(minecraft_survival)`, `T(render_anim)`.

**A7. "Minecraft entities take the asset root from the caller"** (seam)

- `Entities::new(stream, seed)` (`minecraft_entities.rs:162`) becomes `Entities::new(stream, seed, root: Option<&Path>, jar: Option<PathBuf>)`.
  - :175 uses `jar`.
  - :182 uses `root`.
- Move `data_jar()` (:976-996) into `minecraft_world.rs` as `pub(crate) fn data_jar()`, without its two comments.
- Callers:
  - `minecraft_world.rs:470` passes `assets::minecraft_map::root().as_deref()` and `data_jar()`;
  - `minecraft_inventory.rs:587` calls `crate::minecraft_world::data_jar()`.
- After this, `minecraft_entities.rs` has no `assets` dependency, which was its last tie to `render_anim`.
- **Run:** `CHECK`, `T(render_anim)`.

**A8. "Minecraft mobs and the level server handle move to minecraft_survival"**

- `git mv minecraft_entities.rs` to `entities.rs`, `pub(crate)` becomes `pub`, and `minecraft_survival::death::` becomes `crate::death::`.
- `lib.rs`: remove :44.
- `crate::minecraft_entities::` becomes `minecraft_survival::entities::` in `minecraft_world.rs` (14) and `minecraft_chat.rs` (1).
- This keeps the client mirrors and mesh code in `Entities`. They don't use bevy (they come from `minecraft_terrain`), and splitting them off belongs to multiplayer (owner decision 3, proposed default).
- **Run:** `CHECK`, `T(minecraft_survival)`, `T(render_anim)`.

**A9. "Minecraft chat commands run from minecraft_survival"**

- **Seam:** `pump(chat: &mut McChat, ...)` (`minecraft_chat.rs:301`) becomes `pump(sent: &mut Vec<String>, ...)`.
  - `McChat.sent` is a `Vec<String>` (`frame/src/minecraft_chat.rs:27`).
  - The caller (`minecraft_world.rs:587`) passes `&mut ui.chat.sent`.
- **New `commands.rs`**, rustfmt-clean: `HOST`, `Host` and its `CommandWorld` impl, `registries`, `effect_kind`, `give_stacks`, `nbt_error`, `summon_answers`, `pump` and `death_message` (`minecraft_chat.rs:12-324`), with `pub` visibility.
- **Two tests move with it:**
  - `leftover_given_items_drop_at_the_feet`;
  - `host_effects_reach_survival_and_teleports_wait_for_the_world`, using a plain `Vec<String>` instead of `McChat::default()`.
- **`render_anim/src/minecraft_chat.rs` keeps** `show` (:326-345) and the 5 tests that call it, now importing from `minecraft_survival::commands`.
- `crate::minecraft_chat::{HOST, Host, pump, ...}` in `minecraft_world.rs` (9 of its 12 sites; the 3 `show` calls stay) becomes `minecraft_survival::commands::`.
- **Run:** `CHECK`, `T(minecraft_survival)`, `T(render_anim)`.
- **Checkpoint:** release build and the owner test (section 7).

### Phase B: the state model (`Server`, `Level`, `Player`), no behaviour change

**B1. "Minecraft dimension and player ids live in minecraft_net"**

- Move `Dimension` (`terrain.rs:974-999`, without its doc line) into `minecraft_net/src/lib.rs` with the extra derives. Add `PlayerUuid`.
- In `terrain.rs`, add `pub use minecraft_net::Dimension;` and give `minecraft_terrain/Cargo.toml` a `minecraft_net` path dependency.
- That edit is in an edition-2021 file: no let-chains.
- New tests in `minecraft_net`:
  - `parse` and `dimension_type` round-trip for all three dimensions;
  - serde round-trip, with `serde_json` as a dev-dependency.
- **Run:** `check --offline` (lock update), `CHECK`, `T(minecraft_net)`, `T(minecraft_terrain)`.

**B2. "Minecraft terrain, mobs and containers belong to a per-dimension Level"**

- **New `level.rs` and `server.rs`:**
  - `Level` and `Terrain` as in section 2.2;
  - `Server { levels }`, whose `globals` and `players` come in B3 and B5.
- **Loading:**
  - `load()` builds `Terrain` (today :275-300) and returns it next to a `Loaded` without `stream`, `scene` and `environment`.
  - Completion (:454-521) builds `Level { dimension: Dimension::Overworld, entities: Some(Entities::new(&terrain.stream, ...)), containers, terrain }`.
  - `containers` is the default value, which replaces :468, with `set_book` as today (:472).
  - It stores `runtime.server = Some(Server { levels })`.
- **`Runtime`** loses `entities` and `containers`.
- **`update()`:**
  - Bind the overworld level once, after the `world` check (:560), and destructure `let Level { entities, containers, terrain, .. } = level;`.
  - `world.scene` (81 uses), `world.stream` (30) and `world.environment` (5) become `terrain.scene`, `terrain.stream` and `terrain.environment`.
  - The `entities` and `containers` bindings keep today's types, so their ~200 uses (153 and 47) don't change.
- **Helpers that took `&mut Loaded` for terrain** take `&mut Terrain` (plus `&Registries` or `&Loaded` where they also read packs):
  - `stream_around_spawn` (2142);
  - `observe_spawn` (2177);
  - `write_blocks` (2237);
  - `autosave` (2393).
- **`close_world` and `save_and_close` (2431-2498)** take the `Server`. They keep today's order:
  1. take every level's `entities` and drop them;
  2. join the loader and drop `loading`;
  3. join the writer;
  4. read the overworld's `world_spawn`;
  5. drop the levels, which saves the chunks;
  6. write `level.dat`, then the player file.
- **Run:** `CHECK`, `T(minecraft_survival)`, `T(render_anim)`.
- **Checkpoint:** release build, focused on save and reload (section 7, steps 9-10).

**B3. "Minecraft world-wide state is kept in Server globals"**

- `Globals` as in section 2.1, without `data`, which comes in B4.
- The loader builds `Server { globals, levels }`, with the level's `entities: None`. It sets `globals.day` from the saved clock, which is today's :476-479.
- Completion keeps, in today's order:
  - `IW4L_MINECRAFT_TIME` (:482-487);
  - `save.started` and `set_host` (:488-493);
  - the starting game mode (:495-499);
  - `Entities::new`.
- `world.game_rules`, `.save`, `.chat`, `.seed`, `.difficulty`, `.registries` and `.level_writer` become `globals.*`.
- The `day` binding (8 uses in `update`) and `autosave_ticks` become `globals.day` and `globals.autosave_ticks`.
- Don't bind `let day = &mut globals.day`: it would hold a borrow across the `autosave(globals, ...)` call at :1517.
- `Host` (`minecraft_world.rs:572-586`) borrows the disjoint `globals` fields.
- `autosave` and `save_player_mode` (2358) take `&mut Globals`.
- **Run:** `CHECK`, `T(minecraft_survival)`, `T(render_anim)`.

**B4. "Minecraft recipes, loot, stations and enchantments load once per world"**

- **New `data.rs`:**
  - `GameData { jar, recipes: Arc<RecipeBook>, loot: Option<Arc<LootBook>>, stations: Arc<StationBook>, enchantments: Option<Arc<Enchantments>> }`;
  - `GameData::load(root, jar)`, holding the jar reading and its log lines from `entities.rs` (today :175-212, without the kit).
  - `LootBook::roll_drops_named` takes `&self` (`third_party/minecraftoss/player/src/loot.rs:112-118`), so an `Arc` works.
- **`Entities::new(stream, seed, &GameData)`:**
  - still sends `load_loot` and `set_recipe_book` to its own server thread (:181, :191), because those are per level;
  - stores `recipes` and Arc clones of the books.
- Completion loads `GameData` at the same point as today (on the main thread) into `globals.data`.
- :472-474 read the books from `globals.data`.
- **Run:** `CHECK`, `T(minecraft_survival)`, `T(render_anim)`.

**B5. "Minecraft players are kept by UUID"**

- **New `player.rs`:** `Player { uuid, id, name, dimension, survival, mode, death_watch }`. Add `players` to `Server`.
- **`Runtime`:**
  - loses `survival`, `player_mode` and `death_watch`;
  - gains `local: Option<PlayerUuid>` and `carried_survival: Option<Survival>` (owner decision 2).
- **Completion:**
  - After `chat.join(HOST, name)` (:456), take the uuid from `chat.profile(HOST)`.
  - Insert `Player { id: HOST, dimension: Overworld, survival: carried.take().unwrap_or_default(), mode: starting_game_mode(...), .. }`.
  - Set `runtime.local`.
- **`update()`:**
  - Looks up the local player, then the level by `player.dimension`.
  - `let Player { survival, mode: player_mode, death_watch, .. } = player;` keeps every use in the body as it is.
- **`close_world`** takes `Survival` out of the local player before the server goes to the save thread. `save_and_close` writes the host file from the player's mode.
- **`stop()`:** the `death_watch` reset (:2504) goes, because the `Player` is dropped with the `Server`, which has the same effect.
- **Run:** `CHECK`, `T(minecraft_survival)`, `T(render_anim)` (chat tests).

**B6. "Minecraft inventories belong to the player"**

- **`Player`** gains `inventory` and `selected`, created at completion with `globals.data.recipes`. The kit (`entities.rs:213-216`) is given there, with the same log line.
- **`Entities` loses `inventory` and `selected`.** These methods take `(&mut Inventory, usize)`, or the held stack, instead:
  - `punch` (:467-483);
  - `hatch_egg` (:511-516);
  - the hand-enchantment helpers (:552-562) and their callers `award_block_experience` (:293), `drop_blocks` (:312) and `mined` (:546);
  - `server_items_tick` (:356);
  - `tick` (:597).
- **`drop_blocks`** reads `self.recipes` (:334, :345).
- **`update()`** binds `inventory` and `selected` from the player.
  - About 60 lines change: `entities.inventory` becomes `*inventory`, and `entities.selected` and `e.selected` become `*selected`.
  - Watch the closures at :762-767 and :776, which today go through `entities.as_ref()`.
- **`Host`** gains `inventory: &'a mut Inventory` and `selected: usize`. The test helper `Parts::host` (`minecraft_chat.rs:391-407`, then in the file that keeps those tests) gets an `Inventory::default()`.
- **Run:** `CHECK`, `T(minecraft_survival)`, `T(render_anim)`.
- **Checkpoint:** release build and the full owner test.

**B7 (optional). "Minecraft hand and fall timers belong to the player"**

- Move `HandState` and `StepState` (`minecraft_world.rs:93-124`) whole into `player.rs` as `Hand` and `Steps`, with `pub` fields and without comments.
- `Player.hand` and `Player.steps` are destructured into the same `hand` and `steps` names, so there is no body churn.
- **Run:** `CHECK`, `T(render_anim)`.

### Phase C: game procedures leave `minecraft_world.rs`

Each step is independent, and all of them are new code in `minecraft_survival` (rustfmt-clean, no comments). Call sites keep their argument order.

**C1. "Minecraft saving and closing run from minecraft_survival"**

- `starting_game_mode` (2345-2356), `save_player_mode` (2358-2374) and `autosave` (2393-2423) move to `server.rs`.
- So does the file-writing tail of `save_and_close` (2475-2497), with the drop order of B2.
- The adapter keeps the loader and loading joins.
- **Run:** `CHECK`, `T(minecraft_survival)`, `T(render_anim)`.

**C2. "Minecraft block edits run from the level"**

- **`Level::write_blocks(writes)`:** `scene.set`, `record_edits`, `mark_edited`, and `entities.placed` or `broke` (today 2245-2259, without the `sim` line).
  - The adapter's `write_blocks` first sets `sim::voxel::set_block_shape(shape_id(...))` for every write, then calls the level.
  - Nothing reads collision between the two loops. `sim` reads it in FixedUpdate.
- **`spill`** (2229-2235) becomes `Level::spill`.
- **`finish_item_use`** (2262-2297) returns the sounds it heard instead of playing them.
- **`hold_off_death`** (2299-2312) returns whether a totem popped, and the adapter plays `TOTEM_SOUND`.
- **Run:** `CHECK`, `T(minecraft_survival)`, `T(render_anim)`.

**C3. "Minecraft containers tick in their level"**

- `Level::tick_containers(ticks) -> (writes, brewed)` takes the code from :825-857.
- The adapter plays the brew sound and applies the writes through its `write_blocks`.
- **Run:** as in C2.

**C4. "Minecraft chunk streaming runs from the level"**

- `Level::stream_tick(block)` runs `stream.server_tick` (:663, :2155) and `entities.load_chunk` and `unload_chunk`, and returns the loaded and forgotten chunks.
- The adapter keeps the shape-id pass and `sim::voxel::set_chunk` and `remove_chunk` (2100-2138). Entity commands and voxel updates are independent, so their order between chunks can change.
- **Run:** as in C2.

**C5 (optional). "Minecraft chat command handling runs from minecraft_survival"**

- `commands::run(globals, level, player, sent, pose, alive)` takes :566-603: building `Host`, `pump`, the rules and mode follow-up and `save_player_mode`.
- It returns `{ delivered, teleports }`.
- **Run:** as in C2.

**D1. "Docs: Minecraft code map after the crate split"**

- Update `docs/MINECRAFT-SURVIVAL.md:122-127`. These lines are now wrong about where the survival state lives and about `PLAYER = 0` in `minecraft_entities.rs`.
- Update the PROGRESS.md row and log.
- The roadmap line in CLAUDE.md changes only if the owner wants it.

## 4. Risks

1. **Edition.**
   - `minecraft_terrain`, minecraftoss-player and minecraftoss-entities are 2021 (`third_party/minecraftoss/{player,entities}/Cargo.toml:4`).
   - Moving their code into a 2024 crate changes `if let` and tail-expression drop order and `impl Trait` capture. The plan moves none of it.
   - The only 2021 edit is B1's `pub use` in `terrain.rs`. Any later edit there must avoid let-chains.
   - Code moved from `render_anim` needs 2024: it uses let-chains.
2. **glam 0.29 against 0.32** (section 1.2). Using the workspace glam gives type errors on `DVec3` at every minecraftoss boundary.
3. **Visibility.**
   - `pub(crate)` becomes `pub` in each moved file. Counts in `render_anim`: survival 75, entities 52, death 30, save 28, chat 22, offhand 22.
   - Struct literals built in `render_anim` need every field `pub`: `Motion` (:782), `Blow` (:910), `PlayerView` (:1494), `SentItemUse` (:1163), `Host` (:572) and `DeathSpot` (:1433). They all are once the blanket replace is done.
   - Nothing in `render_anim` adds an inherent `impl` to a moved type (checked with grep), which would fail across crates.
4. **Orphan rule.** `Vitals` to `McVitals` must be a function (A5).
5. **Bevy Resource against plain struct.**
   - `Server`, `Level` and `Player` stay plain structs inside the NonSend `Runtime` (`minecraft_world.rs:241`).
   - They are not `Sync`: `PackStack` has a `RefCell` (`pack.rs:400`) and `TerrainStream` has an `mpsc::Receiver` (`terrain.rs:954`).
   - They must stay `Send`, because the loader thread sends them through `mpsc` (:429-439).
   - Don't make them Resources, and don't add systems for them: `update`'s place in the schedule is not pinned against the HUD's Minecraft systems (`hud/src/plugin.rs:123-125`), which also write `MinecraftUi`. `register` (229-254) stays exactly as it is.
6. **Save order.** `Level` field order and `save_and_close` keep entities, then chunks, then `level.dat`, then the player file (section 2.2). A wrong order loses entities or writes `level.dat` before the chunks.
7. **The RNG draw order.**
   - `Entities.random` (`minecraft_entities.rs:88`) feeds both game rolls (loot, egg yaw, throws, XP) and sound pitches (:706, :742), and `update` draws from it through `random_float` dozens of times.
   - Splitting it into a game RNG and a client RNG would change every roll. Keep one.
8. **Early returns gate the game.**
   - `update` returns before any game tick when there is no `AuthorityWorld` (444-446) or no presented local player (615-620).
   - It also returns before mobs, autosave and the day advance when the client light is missing (1464-1466).
   - Keep these returns exactly. A headless tick that doesn't depend on presentation is multiplayer work.
9. **Four tick accumulators:** `Entities.clock` (604-608), `HandState.clock` (768-770), `Mining.tick_clock` (`minecraft_mining.rs:163-171`) and `animation_clock` (604-610). Don't merge them in the split.
10. **One voxel world per process** (`sim/src/voxel.rs:41`), with queues keyed by MW2 client id (126-378).
    - The adapter mirrors only the local player's level.
    - Players in different dimensions inside one MW2 authority need a voxel world per dimension. That blocks multiplayer across dimensions, not this split.
11. **Merges.** Files split in two (chat in A9; later `Entities` and `InventoryUi`) can't be merged against old branches. The round-7 track worktrees must branch after the split.
12. **`Cargo.lock` and `--locked`:** see section 0.
13. **Hard to move; they stay put for now.**
    - `minecraft_terrain`'s server, containers and item rules: edition, the `scene`/`terrain` cycle, and render modules that import server types.
    - `TerrainStream`, which mixes server and client.
    - `InventoryUi`: bevy `Image`, plus the `McClick` input.
    - `Mining`: `VoxelEvent`, particles and `sim::voxel::set_block_shape` at `minecraft_mining.rs:233`.
    - `Sounds`, `SpawnGate`, `minimap`, `camera` and `gui`.
    - The player-action block of `update` (862-1340).

## 5. The adapter left in `render_anim`

- **`register`** (`minecraft_world.rs:229-254`), unchanged:
  - `update` in Update, `ClientSet::Present`, after `PresentedPublished`;
  - `save_before_exit` in Last, after `ExitSystems`, before `exit_process` (`console/src/plugin/mod.rs:209`);
  - `place_inventory_puppet`.
- **`Runtime`**, as in section 2.4.
- **`load()`** runs on the `minecraft-world-load` thread:
  - `minecraft_survival::save::open_world`, registries and packs;
  - `TerrainStream::for_dimension_at(.., Dimension::Overworld, ..)`;
  - `level.dat`, `CommandServer` and ops;
  - it returns `(Loaded, Server)`.
- **`update()`, per frame:**
  1. The lifecycle (418-446).
  2. Load completion: `GameData`, `Entities::new`, the local `Player`, `sim::voxel::activate`.
  3. Look up the local player and their level.
  4. Read the MW2 `PlayerState` and `AuthorityWorld` into `Motion`, `PlayerView` and the pose.
  5. Call the game types and procedures.
  6. Write back into MW2: abilities (634), health (1626-1628), damage and death (1644), teleports (1606-1614), the sprint block (1646-1648) and totems (1854).
  7. Present: `MinecraftWorldView`, `MinecraftUi`, sounds, meshes, the sky and the light volume.
- **MW2 glue it keeps:**
  - `minecraft_yaw` (372), `ability_bits` (2376), `ui_game_mode` (2384), `obstacles` (2321) and `mc_vitals`;
  - `shape_id` and the voxel half of `apply_streamed` and `write_blocks`;
  - `data_jar` and the asset root;
  - `show_chat` and `minecraft_chat::show`.

Sketch of the bindings at the top of `update` after B6:

```rust
let (Some(world), Some(server), Some(local)) = (world.as_mut(), server.as_mut(), local.clone()) else { ... };
let Server { globals, levels, players } = server;
let Some(player) = players.get_mut(&local) else { ... };
let Some(level) = levels.get_mut(&player.dimension) else { ... };
let Player { survival, mode: player_mode, death_watch, inventory, selected, .. } = player;
let Level { entities, containers, terrain, .. } = level;
```

## 6. After the split (not in this plan)

These come with the dimension switch (roadmap 1.3) or multiplayer (1.4):

- **Dimension switch:**
  - `Server::load_level(Dimension)` and `drop_level`;
  - `sim::voxel` re-activation and a new `view.origin` per level;
  - weather and clocks moved up into `Globals`;
  - the `OVERWORLD` constant in `save.rs` (:16, used at 150, 168, 259-262 and 276) becomes per-level.
- **Server thread, per player:**
  - `TickInput` with several players (`server.rs:1127-1152`);
  - `Output` keyed by player (1167-1205), so `server_picked`, `pearl_landings`, `experience_found`, item uses, eggs and summon answers can move to `Player`;
  - `take_xp_delay` and `merchant_closing` per player;
  - player `0` removed at the `server.rs` sites in section 2.5.
- **Client and host:**
  - `Entities` splits into the level part and the client mirrors (`ClientMobs`, particles, `ItemVisuals`, `FallingBlocks`, orb pickups);
  - `TerrainStream` splits into the `ChunkMap` and client sections;
  - `minecraft_net` gains the snapshot types (`server.rs:126-177`) and `ToServer`/`ToClient` enums.
- **Input messages:**
  - the player-action block (`minecraft_world.rs:862-1340`) becomes `minecraft_survival::actions`, driven by a `minecraft_net` action message;
  - the menus of `InventoryUi` move to `Player`, driven by a click message;
  - `Mining.progress` moves to `Level` once `VoxelEvent` has a mirror type without `sim`.
- **A fixed 20 Hz game tick** replaces the frame-time accumulators.

## 7. Owner test (release build after A9, B2, B6 and the end)

Build with `& scripts\cargo-slot.ps1 build --release --locked -p launcher`. The wrapper takes both slots for `--release`. Copy `D:\iw4l-target\release\iw4l.exe` to `C:\Games\Mashup\2010-Rust-Rewrite-Mashup\iw4l-survival.exe`. Put this kit line in the `.env` next to the exe (27 stacks):

```
IW4L_MINECRAFT_KIT=diamond_pickaxe+silk_touch=1,diamond_sword,shield,cooked_beef*32,cobblestone*64,oak_planks*64,crafting_table,furnace,chest*2,ender_chest,coal*16,raw_iron*16,water_bucket,lava_bucket,flint_and_steel,ender_pearl*16,egg*16,zombie_spawn_egg*2,cow_spawn_egg*2,totem_of_undying,shulker_box,anvil,enchanting_table,bookshelf*15,experience_bottle*64,torch*64,glass_bottle*4
```

Steps (send numbered screenshots):

1. Load a Minecraft match in a world folder (`IW4L_MINECRAFT_WORLD=split`). The kit is in the backpack, and hearts, hunger and XP show.
2. Mine stone with the pickaxe and shoot a block with a gun. Both drop items, and a silk-touch ore drops itself.
3. Place a crafting table, a furnace and a chest. Craft something, smelt raw iron with coal, and put items in the chest.
4. Place the ender chest and put an item in it. Break it: the item doesn't drop. Place it again, and the item is still there.
5. Eat beef. Hold the shield up against a zombie from the spawn egg. Throw a pearl (teleport and fall damage) and an egg.
6. Fall from 6 blocks or more (fall damage and sound), then swim in water and stand in lava briefly.
7. Die with the totem in the offhand (it pops). Die again without it: items and XP drop, and guns stay.
8. Type `/gamemode creative`, `/give @s minecraft:diamond 5`, `/tp ~ ~10 ~`, `/time set 13000`, `/weather rain` and `/summon minecraft:cow`. Then `/gamemode survival`.
9. Leave the match, and check the log for "Minecraft world saved". Start the match again: the same seed and terrain, your placed blocks, and your game mode are back.
10. Wait 5 minutes (autosave, "Minecraft autosave: N chunks queued"), then quit the game from the menu. On the next start, the world loads with your edits.
