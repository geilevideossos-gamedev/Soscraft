# Multiplayer: the first playable version

Round 8, track E. This is the design and the build plan. Nothing in it is built yet. Every file and line reference was read at `e2313625`. Line numbers move as other tracks land, so check each one before relying on it.

This version was revised after three reviews. Every point the reviews raised was checked against the code; section 22 lists what changed.

## 0. Goal and what "done" means

The owner's request is to play in one world with a friend. The friend joins the owner's world through the MW2 relay, and then:

- Both players see each other's MW2 model with Minecraft armour, the held item and the elytra.
- Blocks, containers, mobs, dropped items and the dragon fight stay in sync.
- Each player has their own inventory, XP, hunger and spawn point, and the two can be in different dimensions.
- Chat, death messages and the kill feed work.
- MW2 guns hurt other players unless the `pvp` game rule is off.
- It can be tested on one PC with two game windows: a host and a client over localhost.
- A friend can install the game and join by following written steps.

This is roadmap item 1.4 in `CLAUDE.md`. The list there says what is still to build:

- a server tick with several players;
- every occupied level ticking;
- chunk streaming per player;
- an MW2 collision world per dimension;
- the network messages in `minecraft_net`;
- the player actions and menus moved out of `render_anim`.

The plan targets 2 players and should work for up to 4.

## 1. The decisions on one page

| Question | Decision |
|---|---|
| Who owns the world | **The host (option A).** The host's process runs the whole `Server`: every open `Level`, every `Player` and every player file. A friend's game holds no world and writes no save. |
| Transport | **The existing MW2 relay** (`iw4l-master`, QUIC). The relay's wire does not change. Minecraft bytes travel as opaque payloads on four lanes: the reliable control stream, the datagram lane in each direction and uni streams (section 3). The master gets one behaviour fix: a full queue closes the friend's connection, never the host's (section 3.2). |
| Flow control | **A credit window per friend.** The host never has more than a few unacknowledged Minecraft frames or chunk batches out to one friend, and it budgets by bytes per second. The friend acknowledges both. Nothing relies on a queue the host cannot see. |
| Movement | **MW2 stays authoritative.** The host already steps every client's pmove from its usercmds. The friend's game predicts its own movement against a local copy of the collision built from chunks the host sends. No vanilla move checks are needed. |
| Blocks | The host decides every change. The friend sends what it wants to do (hand events with a target) and the host checks reach. Block changes are not predicted on the client in this round. |
| Identity | The MW2 player name of the friend's **MW2 connection** gives the vanilla offline UUID (`OfflinePlayer:<name>`), as today. The friend's game cannot claim another name. The in-game player number is the MW2 `ClientId` (`u32`, never reused in a session): the host is 0, friends are 1 and up. The level thread's `u64` player id and the command `PlayerId(u32)` use that same number. |
| Collision per dimension | **One `sim::voxel` world with a separate height band per dimension** (section 8). Each dimension's blocks sit in their own band of MW2 map space, far apart. Traces pick the band from the point's height, so no trace call needs a dimension argument. MW2 players, bullets and blasts in different dimensions are then tens of thousands of units apart. Until bands land (M5), **dimension travel is refused while a friend is online** (section 5.3). |
| Codec | `minecraft_net` messages are serde types encoded with `postcard`. Vanilla-shaped payloads stay vanilla: chunks as region NBT (`chunk_nbt::write_chunk`) compressed like region files, and new mobs as their saved NBT built **on the level thread**, where the original tags are. `net` compresses control and state packets with zstd, as it already does for snapshots. |
| Ordering | **One total order on the reliable lane.** A chunk batch on a uni stream has a marker in the reliable stream, and the friend's game pauses the reliable stream at a marker until that batch has arrived (section 11.2). |
| Other players' bodies | The MW2 body path stays as it is. A friend is an MW2 client, so their soldier, animations, gun, hit boxes, name tag and kill feed already work. Every client's full `PlayerState`, including `pm_flags` and the glide bit, already reaches every other client. The Minecraft side adds the gear per client and hides bodies that are in another dimension. |
| MW2 game type | **Free-for-all.** A Minecraft world always runs `dm`, whatever game type was picked, so friendly fire and team balancing never get in the way of the `pvp` rule (section 13.4). |
| Host's own view | It stays integrated, with the scene shared between the server and the host's renderer, as today. Friends get a thin client. The HUD for both is built from the same view types (`PlayerView`, `MenuView`), so the screens have one code path. |
| What leaves `render_anim` | The decision code for actions and menus moves to `minecraft_survival::actions` and `minecraft_survival::menus`, driven by hand events and `Click`. The per-player rules (portal, End, elytra, respawn, mining, chat) return per-player effects instead of writing the host's UI and sounds. `render_anim` keeps input sampling, raycast presentation, particles, sounds and the screens. |

**Open question for the owner:** none blocks the plan. One choice is recorded so the owner can overrule it: Minecraft deaths (mobs, falls, lava) stay out of the MW2 kill feed and show in the Minecraft chat, as the owner asked in round 7 (`docs/round7/a2-chat.md`: "Death messages go to the Minecraft chat like vanilla, not to the MW2 kill feed on the left"). Gun kills between players appear in both.

## 2. Architecture: who runs what

```
 Host process (RuntimeRole::Listen, MW2 client 0)                   Friend process (RuntimeRole::Client, MW2 client 1..)
 ------------------------------------------------                   ---------------------------------------------------
 MW2 AuthorityWorld (all players' pmove, guns, GSC)                 MW2 prediction world (own pmove), proxies for others
 sim::voxel: every open level, one band each       <- chunks ----  sim::voxel: own dimension's band, filled from chunks
 minecraft_survival::Server                                         minecraft_survival::remote::RemoteWorld
   levels: Level per open dimension                                   one RemoteLevel (TerrainStream in remote mode,
     TerrainStream (ChunkMap + host sections)                           scene, entity mirror, level entities)
     Entities (level thread + host mirrors)                          own PlayerView, MenuView, chat, boss bar, time
     Containers                                                     render_anim::minecraft_remote (present, HUD, input)
   players: Player per UUID (host and friends)
   sessions: Session per friend (chunk sender, window,       <--- ToServer (Hello, HandEvents, Click, Chat, Ack, ...)
     tracked mobs, last sent views, menus' state ids)        <--- HandState datagram (held buttons, target)
                                                             ----> ToClient (Login, chunk markers, blocks, mobs, views, chat, ...)
 render_anim::minecraft_world (host adapter, host view)
 render_anim::minecraft_host (sessions <-> net::MinecraftWire)
```

**The host runs:**
- everything it runs today;
- a per-player step for every player with a live MW2 client: survival, hazards, actions, MW2 write-back;
- chunk loading around every player;
- a tick for every occupied level;
- one `Session` per friend that turns server state into `ToClient` messages and applies `ToServer` messages.

**The friend runs:**
- the MW2 client with prediction against its own voxel copy;
- a `RemoteWorld` that applies `ToClient` messages: chunks into a remote `TerrainStream`, block updates, the entity mirror, its own player and menu views;
- the renderer and HUD, fed from that state;
- input sampling: a raycast against its own scene and entity mirror, turned into hand events, `HandState`, `Click` and chat messages.

It runs no level thread, no `ChunkMap`, no generator and no save.

**Why not run the friend's own levels and sync them?** Option A matches vanilla's integrated server and the MW2 host-authority model. It also keeps one truth for blocks, containers and mobs.

## 3. Transport

### 3.1 What exists

Every game byte goes through the `iw4l-master` QUIC relay as a star. The host is at the centre, and a client can only reach the host (`crates/iw4l-master/src/main.rs:304-385`). `RelayMailbox` (`crates/net/src/transport/udp_session.rs:32-131`) has data and control queues in each direction, each capped at 64 entries on the host (`HOST_DATA_CAP`, `master.rs:84`). Uni streams carry bootstrap blobs (`master.rs:2944-3103`, magic `"BS"` at `bootstrap.rs:46`).

**What fails today when a queue fills** (all of it fatal, which is why section 3.2 adds flow control):

- **Master, control queue.** A relay frame whose target's queue is full (`CONTROL_CAP` 32, `main.rs:30`) closes the target **and** breaks the sender's own loop (`main.rs:694-697`). `handle_connection` then closes the sender's connection too (`main.rs:606-627`). If either side is the host, the room closes.
- **Host, control mailbox.** `control_outbound` holds at most 64 entries, shared by every friend and by MW2's own reliable control packets. MW2's own send fails with `?` when it is full (`udp_session.rs:246`). The bridge moves the mailbox into the 32-frame `HOST_CONTROL_CAP` channel (`master.rs:85`) with `try_send`, and a full channel ends the host's whole bridge (`master.rs:1772-1790`, `enqueue_control` at `:1987-2001`).
- **Host, control inbox.** The control reader stops reading at 64 queued inbound entries (`udp_session.rs:66-82`). QUIC backpressure then fills the master's 32-frame queue to the host, which closes the room (first item). A main-thread stall of about 1.5 s with 3 friends sending at 20 Hz would do it. Stalls exist today: `save_player` and `autosave` join the previous writer thread on the main thread (`minecraft_survival/src/server.rs:717-719`, `:744-746`).
- **Bulk, host side.** `bootstrap_egress` runs `send_bootstrap_stream(...).await?` (`master.rs:2852`), so a failed or timed-out write ends the egress. When the master has no slot, it answers with `recv.stop` (`main.rs:705-711`). The host may see that as a write error (fatal) or not at all, because `send_bootstrap_stream` returns after `finish()` without waiting for `stopped()` (`master.rs:3065-3103`). The master's own forward only logs a failure or its 8 s deadline (`bootstrap_forward`, `main.rs:1295-1322`).
- **Bulk, friend side.** `uni_ingress` reads streams one after another inside its accept loop. A read past `IO_DEADLINE` (8 s) and a full `push_from_worker` queue are both a `TransportFault` (`master.rs:2944-3003`).
- **Decoding.** The MW2 client tears its link down when a *control* packet fails to decode (`udp_session.rs:1043`).

### 3.2 Four Minecraft lanes

| Lane | Carried as | Holds | Limits that matter |
|---|---|---|---|
| **Reliable** (ordered) | New `ServerPacket::Minecraft` (tag 15) and `ClientPacket::Minecraft` (tag 4) on the control queues (`push_control_outbound` / `take_control_inbound`), relayed as `ControlFrame::Relay` | Every `ToServer` message except `HandState`, and every `ToClient` message except chunk payloads and state frames | 16 KiB per frame (`MAX_CONTROL_BYTES`, `master_protocol/src/lib.rs:7`), and the queue caps in section 3.1. |
| **State, host to friend** (unreliable, newest wins) | New `ServerPacket::MinecraftState` (tag 16) on the data queues (`push_outbound`), fragmented into ≤1100 B datagrams | One `StateFrame` per tick per friend: mob moves and the level's entity snapshot near them | The whole frame is lost if one fragment is lost (`fragment.rs:8-14`). That is fine, because the next tick replaces it. |
| **State, friend to host** (unreliable, newest wins) | New `ClientPacket::MinecraftState` (client tag 5) on the data queues | One `HandState` per tick while anything is held: the held buttons and the current target | One datagram. A lost one is replaced next tick. |
| **Bulk** (uni streams) | New magic `"MC"` beside `"BS"`, sent by its **own** host egress task | Chunk batches | 256 KiB per stream (`MAX_BOOTSTRAP_PAYLOAD`). At most 4 concurrent forwards per connection on the master (`MAX_CONCURRENT_BOOTSTRAP`, `main.rs:704-712`), counted on the host's connection, so shared by every friend and by MW2's bootstrap. |

**Rules the host enforces in `net::minecraft_wire`:**

1. **Reliable lane, host to friend: a credit window.**
   - Each tick, all of a friend's reliable messages are batched and split into frames of at most 15 KiB after compression. Each frame carries a per-friend frame number.
   - The friend acknowledges the highest frame it has received, cumulatively, in its own next frame (an `Ack` is sent on its own if nothing else is going).
   - The host keeps at most **`window` unacknowledged frames per friend**, where `window = min(8, 24 / friends)`, so all friends together never have more than 24 Minecraft frames in the master's queues. That leaves at least 8 of each 32-frame master queue for MW2's own control packets.
   - On top of the window, a **byte budget**: each friend gets at most 64 KiB/s of reliable bytes by default (`IW4L_MC_RELIABLE_KIB`), counted per second, not per tick.
   - **The shared mailbox.** Minecraft frames are pushed into `control_outbound` only while it holds fewer than 48 entries (a new `control_outbound_len()`), so 16 entries always stay free for MW2. A frame that does not fit waits for the next tick in order. `net` never `try_send`s a Minecraft frame past what is left.
   - If a friend's unsent backlog passes 4 MiB, or no acknowledgement has arrived for 20 s, the host drops that friend with "connection too slow".
2. **Reliable lane, friend to host.**
   - The friend sends **at most one coalesced frame per tick**, and keeps at most 4 of them unacknowledged by the host (the host acknowledges in its own frames). Held buttons never go on this lane; they go in `HandState`.
   - Nothing on the host may stall the control reader for long: the leave save and autosave never join a writer thread on the main thread (section 14).
3. **The master is patched** (`iw4l-master/src/main.rs:690-699`): when a relay frame finds its target's queue full, the master closes **the connection that is not the room's host** (the friend, whichever direction the frame went) and keeps the host's loop running. This is a safety net only. The windows above should keep it from firing. A master on a VPS needs the patched build.
4. **Bulk lane.**
   - `"MC"` streams have their own egress task. A failed write is reported back to the session as "batch `seq` not sent", never raised with `?`, so it cannot end the host's link.
   - At most **2** `"MC"` streams are in flight across all friends. The other 2 master slots are left for MW2 bootstrap offers, which the host sends one at a time but which the master may still be forwarding after the host's write has finished. Two joins at once therefore still fit.
   - A stream counts as in flight until the friend's `ChunkBatchReceived` arrives on the reliable lane.
   - A friend may have at most 1 batch in flight, and 2 once 2 have been acknowledged.
   - **Resend.** If no `ChunkBatchReceived` arrives within 3 s, the host sends the same batch again with the same `seq`. The friend ignores a `seq` it already has. If a batch is still unacknowledged after 30 s, the host drops that friend with "Lost the world stream".
   - **Size.** A batch is cut at 128 KiB of encoded data, so one stream fits in the master's 8 s forward deadline (`BOOTSTRAP_FORWARD`, `main.rs:35`) and the friend's 8 s read deadline down to about 20 KiB/s.
   - **Friend side.** `uni_ingress` hands an `"MC"` stream to its own reading task with its own bounded queue to the game. A slow or broken `"MC"` stream is dropped with a warning, never a `TransportFault`, and never delays a `"BS"` stream. The host's resend covers the loss.
5. **Ordering across lanes.** Uni streams are not ordered with the control stream. The host puts a `ChunkBatch { seq }` marker in the reliable stream at the point where the batch belongs, and the friend applies the reliable stream strictly in order, pausing at a marker until that batch is decoded (section 11.2).
6. **Admission.** `UdpAuthorityHub::ingress` drops a relay peer's packets until it is `PeerAdmission::Committed` (`udp_session.rs:472-479`, `admits_gameplay` at `:257-262`). `Minecraft` and `MinecraftState` packets are instead accepted from any enrolled peer once the `peers[conn]==from` check passes, so `Hello` and `Login` flow while the friend is still loading. The friend also resends `Hello` every 2 s until `Login` or `Rejected` arrives (section 5.1).
7. **Protocol versions.** Adding the tags bumps `net::PROTOCOL_VERSION` from 94 to 95 (`crates/net/src/lib.rs:172`). A build without them is then refused at the MW2 handshake of a direct match. A relay match compares only the map, weapon and class digests, so there the check that counts is `minecraft_net::PROTOCOL` (new, starting at 1) in `Hello` (track log, E3).

### 3.3 Bandwidth budget

These figures are estimates to check in M1. The agent logs the measured numbers in the track log.

| Traffic | Size | Rate per friend |
|---|---|---|
| First chunks at view distance 8 (`VIEW_DISTANCE`, `minecraft_world.rs:22`): about 290-360 chunks of compressed region NBT, about 6-12 KiB each | about 3-4 MiB once | Paced by acks. Instant on localhost, about 3-5 s at 1 MiB/s. |
| Chunks while walking: one row of 17 chunks per chunk crossed | about 150 KiB | About 30 KiB/s when sprinting |
| `StateFrame`: about 60 mobs at about 70 B, plus about 20 items, orbs and projectiles, zstd | 2-4 KiB | 40-80 KiB/s at 20 Hz |
| Reliable: block updates, views on change, mob adds and looks, chat, sounds | Small | About 5-20 KiB/s, capped at 64 KiB/s |
| Friend to host: `HandState` and hand events | About 50 B | About 1 KiB/s while a button is held |
| MW2 snapshots (already there) | - | Unchanged |

With 3 friends the host uploads about 250-400 KiB/s steadily, plus join bursts. The relay carries everything twice: host to master to friend. A master on the host's own PC costs the host's upload once per friend.

## 4. The messages (`crates/minecraft_net`)

`minecraft_net` gains:
- `postcard` (serde, `use-std`);
- `minecraft_commands`, for the chat types `Serverbound`, `Clientbound`, `Text` and `PlayerId`. `CRATE-SPLIT.md` section 1.2 allows this dependency "when chat payloads become messages".

It still has no transport and no game logic.

Some types are owned by `minecraft_terrain`: `ServerMob`, `EntitySnapshot`, `MobLook` and the view structs. Those travel as `Blob(Vec<u8>)` fields, encoded by `minecraft_survival::wire`, so `minecraft_net` does not depend on `minecraft_terrain`. The same goes for vanilla NBT: `Nbt(Vec<u8>)` holds uncompressed binary NBT, and `ChunkBlob` holds a region-format compressed chunk.

```rust
pub const PROTOCOL: u16 = 1;

pub struct ToServerFrame { pub frame: u32, pub ack: u32, pub messages: Vec<ToServer> }
pub struct ToClientFrame { pub frame: u32, pub ack: u32, pub messages: Vec<ToClient> }

pub enum ToServer {
    Hello { protocol: u16, view_distance: u8, registry: u64 },
    Ack,
    ChunkBatchReceived { seq: u32 },
    Hand(Vec<HandEvent>),
    SelectSlot(u8),
    Click(Click),
    RenameItem(String),
    CloseMenu { menu: u8 },
    Chat(minecraft_commands::protocol::Serverbound),
    CreditsDone,
}

pub struct HandState { pub tick: u32, pub attack: bool, pub use_item: bool, pub target: Target }
pub struct HandEvent { pub seq: u32, pub kind: HandEventKind, pub target: Target }
pub enum HandEventKind { AttackPressed, AttackReleased, UsePressed, UseReleased, SwapHands, Drop { whole_stack: bool } }
pub enum Target { Miss, Block { pos: [i32; 3], face: u8, hit: [f32; 3] }, Mob { id: u64 }, Player { client: u32 } }
pub struct Click { pub menu: u8, pub state: u16, pub slot: i16, pub button: u8, pub mode: ClickMode }

pub enum ToClient {
    Login(Login),
    Rejected { reason: Text },
    ChunkBatch { seq: u32 },
    ChunkForget { pos: [i32; 2] },
    BlockUpdates { changes: Vec<([i32; 3], u32)> },
    BlockEntity { pos: [i32; 3], tag: Nbt },
    BlockEvent { pos: [i32; 3], kind: u8, param: i32 },
    Crack { breaker: u32, pos: [i32; 3], stage: i8 },
    LevelEvent { event: i32, pos: [i32; 3], data: i32 },
    Sound { event: String, pos: [f64; 3], volume: f32, pitch: f32, seed: i64 },
    Explosion { center: [f64; 3], power: f32, blocks: Vec<[i32; 3]> },
    MobAdd { id: u64, tag: Nbt },
    MobLook { id: u64, look: Blob },
    MobRemove { ids: Vec<u64> },
    Player(Box<PlayerView>),
    MenuOpen(MenuView),
    MenuSlots { menu: u8, state: u16, slots: Vec<(i16, Option<Nbt>)>, carried: Option<Option<Nbt>>, data: Vec<(u8, i32)> },
    MenuClose { menu: u8 },
    Equipment { client: u32, gear: GearView },
    Chat(minecraft_commands::protocol::Clientbound),
    PlayerList { added: Vec<(u32, String, PlayerUuid)>, removed: Vec<u32> },
    Time { game_time: i64, clocks: Vec<(String, f64, bool)> },
    Weather { rain: f32, thunder: f32 },
    Dimension { epoch: u32, dimension: Dimension, origin: [f64; 3], feet: [f64; 3] },
    BossBar(BossBarUpdate),
    KilledBy { killer: Option<String>, line: Text },
    Credits,
}

pub struct Login {
    pub client: u32, pub uuid: PlayerUuid, pub seed: i64, pub dimension: Dimension,
    pub origins: Vec<(Dimension, [f64; 3])>, pub feet: [f64; 3], pub view_distance: u8,
    pub rules: Rules, pub registry: u64,
}

pub struct StateFrame { pub tick: u32, pub epoch: u32, pub mobs: Blob, pub entities: Blob }
```

**Hand input** is split by how it is used (vanilla sends `ServerboundPlayerActionPacket` and `ServerboundUseItemPacket` as events, and the held state is implied):
- **Edges are events.** Each `HandEvent` has a sequence number and its own target. The host queues them and uses each one **exactly once**, in order, on the server tick that processes it. A press and a release in one host frame are both seen, and a press is never repeated across host frames.
- **The held state is `HandState`,** sent every tick on the friend-to-host datagram lane while anything is held, newest wins. The host reads it for mining progress and for holding use (eating, drawing a bow, blocking with a shield). A `HandState` older than the last one is ignored. If none has arrived for 500 ms, the host treats the buttons as released.

**`PlayerView`** holds what the HUD and hand need:
- health, max health, absorption, food, saturation, air, fire ticks, armour points;
- XP level, progress and total;
- game mode and abilities;
- active effects (kind, amplifier, duration, ambient);
- the 41 slots (36 storage, 4 armour, offhand) as `Option<Nbt>`, the selected slot, the item cooldowns, and the item in use with its ticks.

**`MenuView`** holds:
- `menu` (the container id, 1..=100, vanilla's `containerCounter % 100 + 1`);
- `kind` (`InventoryUi`'s screens: inventory, crafting table, chest of N rows, furnace, smoker, blast furnace, brewing stand, enchanting, anvil, grindstone, smithing, stonecutter, loom, cartography, ender chest, shulker box, hopper, dispenser, dropper, beacon later);
- `title: Text`, `state: u16`, `slots`, `carried`, `data`, and kind-specific extras (enchant seeds and costs, anvil cost and name).

**`RenameItem`** is vanilla's `ServerboundRenameItemPacket`: the anvil's name field (`Bench`'s name, `minecraft_terrain/src/stations.rs:472`), at most 50 characters, accepted only while an anvil menu is open.

**`GearView`** mirrors `minecraft_terrain::body_gear::BodyGear`: `main`, `off`, `armor[4]` and `hides_gun`, with each `GearItem { id, glint, dye }`.

**`MobLook`** is the render state of a mob that is not in `ServerMob` and changes rarely: equipment item ids, villager type, profession and level, colours, saddle, custom name and baby. Section 9.2 says how it is sent.

**`Rules`** carries the game rules the client needs to know: `pvp`, `show_death_messages`, `keep_inventory`, `natural_health_regeneration`, `reduced_debug_info` and `do_immediate_respawn`.

**Codec.**
- `minecraft_net::encode<T>` / `decode<T>` wrap postcard. They refuse input larger than the lane allows: 4 MiB for one reliable message after batching is undone, 256 KiB for a bulk batch and 256 KiB for a state frame. They also refuse trailing bytes.
- A message that fails to decode is dropped with a `diag::warn!`. On the host, 20 bad messages from one friend in a minute drop that friend.
- The MW2 client tears its link down when a *control* packet fails to decode (`udp_session.rs:1043`). So `net` hands `Minecraft` packets on as bytes and never decodes their contents.

**The chunk batch blob** (`"MC"` stream): `magic "MC", version u8, seq u32, count u16`, then for each chunk `x i32, z i32, len u32, bytes`. The bytes are a `StoredChunk::encode` of `chunk_nbt::write_chunk(chunk, registries, 0)` (`third_party/minecraftoss/core/src/chunk_nbt.rs:181`, `world/src/storage.rs:84-90`). The friend reads it back with `read_chunk` (`chunk_nbt.rs:296`).

The registry fingerprint (`registry`) hashes the block state count and every block state name with its properties. A mismatch means the two games resolved different 26.3 data, so the friend is refused with a clear message.

## 5. Joining and leaving

### 5.1 Join flow

1. **The host opens the world to friends.** This is the existing MW2 hosting path. Either the env path, `IW4L_MASTER_HOST_NAME=<room name>` with `map minecraft:overworld`, or the menu (Host, then the Minecraft map).
   - The Minecraft map always runs the `dm` game type (section 13.4).
   - When the room is created, the host's Minecraft chat shows the join code, the room's `AdvertId` in hex. `IW4L_MASTER_STATUS_FILE` also writes it as `room=`.
2. **The friend joins.** Either `IW4L_MASTER_JOIN=<code>` with `map minecraft:overworld`, the console command `ui_join_lobby_id <code>`, or the server browser.
   - MW2 runs its handshake, bootstrap and `EnterMatch` as for any match.
3. **The friend's game sees the Minecraft map installed** (`assets::minecraft_map::is_minecraft`).
   - In the `Client` role it **does not** start `load()`. Today it does (`minecraft_world.rs:432-450`), and that opens a world folder.
   - Instead it starts loading the registries, packs and atlas on the `minecraft-world-load` thread without a world.
   - It sets `ui.loading_world`, which keeps `session::admission` waiting (`crates/session/src/admission.rs:33-38`).
4. **The friend sends `Hello`** once `net::MinecraftWire` reports that the link is up, and **again every 2 s** until `Login` or `Rejected` arrives. `Hello` passes the relay admission gate (section 3.2 rule 6).
5. **The host handles `Hello`.** A repeated `Hello` from a connection that already has a session, or is already being answered, is ignored.
   - The name is the **MW2 connection's** player name, read from the host's `AuthorityWorld` for that client. `Hello` carries no name.
   - It refuses with `Rejected`:
     - on a protocol or registry mismatch;
     - when the name is already online ("A player with that name is already in the world").

     After sending `Rejected`, the host retires that MW2 client (`retire_client`, `net/src/authority/inbox.rs:461`), so no soldier stays in the world without a Minecraft player.
   - When the world is still loading, `Hello` is held and answered once the server is ready, not refused.
   - It derives the `Profile` from the name and runs `server::join` (`server.rs:572-634`). That reads the player from `recently_left` if it is there (section 14), otherwise loads `players/data/<uuid>.dat`, or makes a new player with the kit.
   - **Until M5,** if the player's saved dimension differs from the host's, they are placed at the host's level's spawn instead (section 5.3).
   - It inserts the `Player`, creates a `Session { client, uuid, name, view_distance, .. }`, opens the player's level if needed (`open_levels`), and adds their position to that level's chunk tracking.
   - It sends `Login` first in the friend's reliable stream, then `PlayerList` to everyone, and "multiplayer.player.joined" (yellow) to everyone's chat. It calls `globals.chat.join(id, name)`.
6. **Chunks stream to the friend** (section 7).
   - The friend's `RemoteWorld` builds its level and feeds `sim::voxel`.
   - When the chunks around `Login.feet` have collision, which is the same test as the host's spawn gate, `ui.loading_world` turns false. MW2 admission then lets the friend pick a class and spawn.
7. **The friend's MW2 body spawns.**
   - The host's per-player step sees a new life and puts the body at `player.place.feet`, facing the saved rotation.
   - It does this only once the host's own voxel has that chunk. It is the per-player form of today's `spawn_gate` and `awaiting_level` (`minecraft_world.rs:2767`).
   - On every later death, the per-player respawn (`respawn.rs`, `respawn_block.rs`) runs exactly as it does for the host.

### 5.2 Leave flow

A friend leaves in one of three ways:
- they quit, and the MW2 client disconnects;
- the link times out after 8 s (`SESSION_IDLE`);
- the host drops them.

`net` reports the retired client in `MinecraftWire.left`. The host then:

1. **Settles the level thread.** It sends `Command::PlayerLeft(id)` and waits for the level to go idle (`wait_idle`, used at `entities.rs:631`). Outputs for that player that were still in flight are applied: picked stacks and taken orbs go into their inventory and XP; anything that does not fit is dropped at their feet. Pearl landings, pushes and potions for them are discarded. Today `Entities::leave` only drains the picks that have already arrived (`entities.rs:604-615`), which would delete items the level thread has already taken out of the world.
2. **Closes their menu.** The carried stack, the crafting grid and any station inputs (anvil, grindstone and the other bench inputs) go back to the inventory, and **what does not fit is dropped at their feet**, as vanilla's `AbstractContainerMenu.removed` does. Today `Player::data` folds them back with `let _ = add_item` (`player.rs:209-211`), which loses the overflow.
3. Removes them as an opener (`level::leave`, `level.rs:299`).
4. Removes their chunk view and entity tracking.
5. Saves their player file through the writer queue (section 14), without blocking the main thread, and keeps their `PlayerData` in `recently_left` until the write is done.
6. Removes them from `server.players`, calls `globals.chat.leave(id)` (`minecraft_commands/src/server.rs:331`) so their name stops resolving for `/tp`, `@a` and the rest, and clears the per-id state on the level thread (XP delay, merchant).
7. Clears their `McBodyGear` and releases a credits hold they had.
8. Sends `PlayerList.removed` and "multiplayer.player.left".

Levels nobody is in close after 15 s, as today (`close_empty_levels`).

**Rejoining** with the same name gives the same UUID and file, so the friend comes back where they left, even if they rejoin before the file is on disk.

### 5.3 Dimension travel before M5

Until bands (E19) and per-player levels (E20) land, the host's collision holds one level, and today's code wipes it on any trip (`sim::voxel::switch_level`, `minecraft_world.rs:634` and `follow_levels` at `:2803`). So, from E6 until E20:

- **While a friend is online, portal and End travel are refused for everyone,** with the chat line "Dimension travel is off while friends are online (not built yet)". This covers nether portals, the End portal, the exit portal, end gateways and `/execute in … run tp`.
- **A joining friend is placed in the host's dimension**, at that level's spawn if they were saved elsewhere.
- E20 removes both rules.

## 6. The host: one server tick for every player

### 6.1 The frame on the host after this track

1. Lifecycle and load, unchanged.
2. `sessions.receive`: apply every `ToServer` from `MinecraftWire.inbox` (hellos, hand events, clicks, chat, acks) and the newest `HandState` per friend, and handle `joined` and `left`.
3. **Drain the process-global voxel queues once** into per-client maps: `take_minecraft_deaths`, `take_totem_pops` and `take_team_switches` (`voxel.rs:341`, `:385`, `:353`), and the voxel events. Today each is drained inside the host's step and filtered to `local` (`minecraft_world.rs:768-795`), so in a per-player loop player 0 would throw away the friends' entries. `player_damage_pending()` (`voxel.rs:327`) becomes per client, so one player's pending damage no longer skips everyone's `observe` and `settle` (`minecraft_world.rs:784`, `:1858`).
4. **For every player with a live MW2 client**, in client order (host first), `players::step(actor, input)`:
   - read their MW2 `PlayerState` from `AuthorityWorld` into `Motion` and `PlayerView`;
   - run survival, hazards, falls and the death watch;
   - run actions from their hand input: the host's own from Bevy, a friend's from their queued `HandEvent`s and newest `HandState`;
   - run portals and End travel with their feet;
   - write back into MW2 for that client: abilities, health and damage, teleports, the sprint block, totems, `push_velocity`, and the weapon that follows their selected slot (section 10.3).

   This is today's code from `minecraft_world.rs:610-1527` and `1803-1896`, moved into `render_anim/src/minecraft_players.rs` and keyed by `ClientId` instead of `local`. The move is bigger than the action block:
   - **It reads MW2 state per client.** Today it reads `presented.player(local.0)` (`minecraft_world.rs:669`, `:688-711`). On the host every client's state comes from `AuthorityWorld`.
   - **The rules write the host's UI and sounds directly**, and must return per-player effects instead (sounds, chat lines, screens), which the host plays for itself and sessions send to friends:
     - `minecraft_end::credits(player, &mut ui, …)` (`:619`);
     - `minecraft_portal::arriving` and `minecraft_portal::step(…, sounds, packs, &mut ui)` (`:668`, `:723`);
     - `minecraft_end::tick_fight(…, local_uuid, &mut ui)` (`:716`);
     - `minecraft_elytra::step(…, sounds, packs)` (`:705`);
     - respawn and `respawn_block::return_to_block` messages (`:655`, `:690`);
     - mining progress (`minecraft_mining.rs`);
     - `show_chat(&mut ui.chat, …)` throughout.
   - **The early returns stop the whole frame.** While the host is switching level, waiting at the spawn gate or holding for credits, `update` returns early (`minecraft_world.rs:626-642`, `:649-687`). In the per-player step, those branches skip **only that player's** step and the host's presentation, never another player's tick or the level ticks.
5. **For every open level with a player in it**:
   - `level::stream_tick` with the block positions of all its players (section 7);
   - `entities.tick` with all its players (section 9);
   - container ticks on one server tick clock (below).
6. Feed `sim::voxel` from every level's loaded and forgotten chunks (`apply_streamed`, `minecraft_world.rs:2374-2413`), each into its dimension's band (section 8).
7. `sessions.send`: build each friend's `ToClient` messages and `StateFrame`, and push them into `MinecraftWire` within the window and budget.
8. Present the host's own view, as today.

### 6.2 One server clock

Today the world clocks, `level_ticks`, autosave and weather advance on the host's frame and the host's level (`minecraft_world.rs:678, 1716-1719, 2213`). Container ticks use the host's hand clock (`:984`).

This track adds `Globals.tick_clock`: one 20 Hz accumulator that yields `n` whole ticks per frame. For each tick:
- every occupied level's `Entities` gets one `Command::Tick`;
- containers tick once;
- the world clocks and weather advance once;
- autosave counts once;
- each player's queued hand events for that tick are used.

`Entities::tick` keeps its partial-tick value for interpolation but takes the tick count from the server clock, not its own `clock`. Per-player timers stay on each `Player`: hand, cooldowns and the eating clock (`player.rs:48-71`).

### 6.3 The level thread with several players (`crates/minecraft_terrain/src/server.rs`)

**`TickInput` (`:2314-2341`):**
- `players: Vec<TickPlayer { id, feet, eye, spectator, creative, main_hand, inventory_for_pickup, selected }>` replaces `players: Vec<[f64;3]>`, the single `pickup` (`:2322`) and `set_player_main_hand(0, ..)` (`:2782`);
- `simulation_centres: Vec<[f64;3]>` replaces `simulation_center` (random ticks run around each player, vanilla `ChunkMap` ticking chunks);
- `tracking: Vec<(u64, [f64;3], u8)>`: each player's position and view distance, for the entity tracker (section 9.2);
- `player_hurts: Vec<(u64, Option<u64>, &'static str)>`: the hurt player, the mob behind it and the damage type. Today the field is `Vec<(Option<u64>, &'static str)>` (`:2336`) and applies to player 0 (`:2774-2775`); tame wolves use the source to avenge their owner.

**`server_loop` (`:2673-2845`):**
- `tick_player(id)` and `player_hurt(id, ..)` run for each player;
- **pickup runs for every player.** Each `TickPlayer` with an inventory gets the items and tridents touching it (`:2800-2834`), and `take_experience` runs per player (`:2836`), so drops and orbs reach whoever stands on them;
- the weather follows each player for rain sounds. This is presentation, so it moves out: `sim.weather(..)` takes the host's level-wide state only;
- trident pickup tests every body.

**Commands carry the player.** `Command::UseBlock`, `ItemOnBlock`, `MobAction` and `Merchant` (`:2189-2230`, handled at `:2687-2733`) carry no player today, and their results come back in order. Each gains `player: u64`, and each result carries it back. `mob_action` and the merchant update stop calling `merchant_view(0)` (`:388`, `:446`) and use that player. `Throw` already carries its `owner` in `Thrown`. A new `Command::PlayerLeft(u64)` clears that player's state and flushes its pending outputs (section 5.2).

**Per player:**
- `take_xp_delay` becomes a map by player (`:64, :716-725`);
- the merchant state becomes a map by player, including `merchant_closing` (`:66, :388-461`). Trading is still not wired to a screen (out of scope), but this removes the player-0 sites;
- both maps lose a player's entry on `PlayerLeft`.

**`Output` (`:2356-2407`):** `picked`, `orbs_taken`, `item_uses`, `summoned`, `command_summons`, `merchant` and `mob_results` gain the player id. New fields for the entity tracker (section 9.2): `mob_adds: Vec<(u64 player, u64 id, Tag)>`, `mob_removes: Vec<(u64 player, Vec<u64>)>` and `mob_looks: Vec<(u64 id, MobLook)>`.

**`tracked_mobs(position, 160)` (`:1200`)** clones the mobs within range of the host's camera for the host's own mirror, as today. Friends' tracking is on the level thread (section 9.2).

### 6.4 `Entities` with several players (`crates/minecraft_survival/src/entities.rs`)

- Remove `const PLAYER: u64 = 0` (`:36`). Every site takes the acting player's id: throws, the eye, punches, end hits, deflects, `mob_blast`, `mob_views` and `mob_vitals`.
- `set_player_uuid` (`:293`) becomes `set_players(&[(u64, PlayerUuid)])`.
- `Entities::tick` takes `&mut [TickActor]`. Each one has the id, `PlayerView`, inventory and selected slot, so `server_items_tick` (`:401`) puts each picked stack into the right inventory.
- The per-player queues in the struct (`:60-131`) become `HashMap<u64, PlayerOutputs>`:
  - `server_picked`, `pearl_landings`, `player_potions`, `player_pushes`, `experience_found`, `item_use_waiting`, `item_uses_done`, `eggs_waiting`, `command_summons`.

  `leave(id)` settles and clears one player, not the level (section 5.2). Outputs that arrive later for an id that is no longer in the level are handled safely: picked stacks are spawned again where they were picked up, and teleports and pushes are discarded.
- `hits` (`:993-1010`) and `players_struck` (`:992`) return `(id, hit)`. The per-player step applies each hit to that player's survival, with shield and armour (`minecraft_world.rs:1803-1824`).
- The drop staging stays on the server side: `world_items` holds both the drawn mirror and the host's own drops (container drops and cooking output, `level.rs:216-223`, and block drops), which `server_items_tick` hands to the level thread through `to_hand` / `server_handed` (`entities.rs:401-420`). That hand-off stays in `Entities`. Only the mirror of server items (`SERVER_ITEM`) moves to `EntityView` (section 9.1).

### 6.5 What is still wrong with one global

- **`commands::HOST = PlayerId(0)`** (`commands.rs:27`) stops being used as the only recipient: `pump` routes by `Recipient` to every player (section 13). **`CommandServer::new(PlayerId(0))` stays**: the host's operator rights come from `CommandServer.host` (`minecraft_commands/src/server.rs:350-363`, set at `minecraft_world.rs:257`), not from `ops.json`.
- **`Runtime.local`** stays and means "the host's own player" for presentation only. Every game rule path takes an explicit player.
- **The nether entity-lighting toggle** (`cow_render.rs:332`) is a process-global presentation switch. On the host it follows the host's shown level, on a friend's game the friend's level. That is correct, because each process draws one level.

## 7. Chunk streaming per player

### 7.1 Loading: the union of every player's view

`ChunkMap` holds one `view: Option<TrackingView>` and one `pending_send` (`third_party/minecraftoss/world/src/chunk_map.rs:282-284`). Its `tick(player_chunk)` serves a single player (`:641-692`).

This track patches the vendored file:
- `tick(&[ChunkPos])` keeps `views: Vec<TrackingView>`;
- `tracks(pos)` is true when any view contains `pos` or a ticket covers it;
- `update_tracking` diffs the old union against the new one. The `was` and `is` closures already take that shape;
- `schedule` and `send` sort by the smallest squared distance to any centre.

The events stay `Center`, `Load` and `Forget`, but now for the **server's loaded set**: the authoritative `HandcraftedScene` and the level thread.

`TrackingView` (`world/src/view.rs:24-108`) and `PlayerChunkCounter` (`distance.rs:301-360`) are already per player and are reused unchanged.

**`TerrainStream::server_tick(players, local)`** (`terrain.rs:1241-1310`):
- passes the players' chunks to the chunk map;
- inserts or removes chunks in the scene as today;
- registers a chunk with the host's `Sections` and lighting **only if the host's own `TrackingView` contains it**;
- when the host's view moves, it registers and unregisters the scene chunks that enter and leave that view.

That way the host never meshes or draws the area around a far-away friend. `level::stream_tick` (`level.rs:185`) takes the players' block positions and returns `(loaded, forgotten)` for the level thread and containers, as now.

Until this lands (E9, early in M2), the chunk map loads only around the host, and a friend gets chunks only where the host's view has them. The M1 owner test keeps the friend near the host.

### 7.2 Sending: one sender per friend

`minecraft_survival::session::ChunkSender` is a simple form of vanilla's `PlayerChunkSender`:

- **`level`:** the dimension the friend is in. Everything below belongs to that level and is cleared when the friend changes dimension.
- **`view`:** the friend's `TrackingView` (their chunk, their view distance clamped to the server's).
- **`sent: HashSet<ChunkPos>` and `pending: HashSet<ChunkPos>`:**
  - a chunk entering the view goes to `pending` if the scene has it, or as soon as it loads;
  - a chunk leaving the view is dropped from `pending`, or if it was sent, a `ChunkForget` is queued.
- **Batches:** up to 9 chunks per batch, nearest first, cut at 128 KiB of encoded data, within the bulk window of section 3.2. Vanilla's adaptive `chunks_per_tick` quota is deferred; the ack window paces the stream.
- **Encoding is off the main thread.** A `minecraft-chunk-send` worker per host gets `(session, seq, Vec<Arc<Chunk>>)`:
  - `ChunkMap::chunk(pos)` first writes pending edits (`chunk_map.rs:618-622`), so the sent chunk is current;
  - the worker returns the batch blob, which goes to `MinecraftWire` as bulk, and the session queues the `ChunkBatch { seq }` marker in the reliable stream at the same point;
  - the chunk counts as sent when it is queued, so later block updates for it are sent too and land after it in the friend's order;
  - the session keeps the blob until `ChunkBatchReceived { seq }`, for the resend.
- **Block entities:** containers and server block entities are written into the chunk map every frame by `stream_tick`, so the sent NBT carries them. Chest contents are not secret between friends.

### 7.3 Block updates

`HandcraftedScene` gains a change log, vanilla's `ChunkHolder.blockChanged`:
- `set`, `clear` and the chunk edits push the position into `changed: Vec<BlockPos>`, deduplicated per tick;
- `take_changed()` drains it.

This is the one choke point for every source of change: player placement, mining, server-thread changes (`minecraft_world.rs:1734`), explosions, fluids and commands.

**The chunk map follows the same log.** Sent chunks come from `ChunkMap::chunk`, which only holds edits pushed through an explicit `record_edits` (`terrain.rs:1314-1324`, 9 call sites). A `scene.set` without a matching `record_edits` would give a friend who arrives later a stale chunk. So each tick, per level, the drained positions are also passed to `record_edits(scene, changed)`, which takes the block the scene shows and is safe to repeat. The scene and the chunk map then agree for every change, and the explicit calls become redundant. A test compares a sent chunk with the scene after edits from each source.

Each tick, per level, the session code:
1. drains the log once;
2. groups the changes by chunk;
3. sends `BlockUpdates { changes: (pos, state id) }` to every friend **whose current level is this one** and whose `sent` set has that chunk. A block entity whose data changed follows as `BlockEntity`, for example a placed sign or banner, or a changed spawner mob.

Chest lids and other `BlockEvent`s come from the host's openers (`level.rs:234-297`) and go to friends **in the same level** within 64 blocks. Every radius test in this design (block events, sounds, level events, explosions, cracks) requires the same level first.

### 7.4 The friend's side

`TerrainStream::remote(registries, seed, dimension)` is a stream with no `ChunkMap`. The `server` field becomes `Option<ChunkMap>`, or a `Source` enum. `apply(ChunkEvent)` runs the same `Load`/`Forget` body as `server_tick` (`terrain.rs:1260-1298`), so light, sections and meshing are reused unchanged. `BlockStates::new` needs the seed (`Login.seed`) and the dimension's height, which comes from the registries.

## 8. The MW2 collision world per dimension: height bands

### 8.1 The choice

`sim::voxel` keeps one `WORLD` with one `origin` (`crates/sim/src/voxel.rs:31-41`). Every query converts a map point with `to_block(origin, p)`. `switch_level` wipes the chunks (`:424`, `:742-747`). Traces carry no client or dimension (`world.rs:3938`, `step.rs:1844-1909`).

There are two ways to support several dimensions:
- **Keyed worlds:** give every trace a dimension argument. That means threading it through `clip_trace`, `ClipBackend`, bullets, blasts, spawns and GSC.
- **Height bands:** give every dimension its own height band of map space, at origins that never overlap.

**This design takes the bands.**
- Nothing outside `voxel.rs` changes its signature.
- MW2 players, bullets, blasts, killstreaks and player-to-player clipping in different dimensions are tens of thousands of units apart, so they cannot meet.
- A body's band also tells the renderer which dimension it is in, with no message.

Whether MW2 copes with players at map z ±45,000 is checked first, in E4, before anything depends on it (section 21, risk 1).

### 8.2 The numbers

MW2 is Z-up, and one block is 36 units (`voxel.rs:15`). Map z = (block y − origin y) × 36. Each level's **wall range** is `[min_y − 128, max_y + 256]` (below).

| Dimension | Origin (block space) | Blocks y | Wall range y | Map z of the wall range | Band (map z) |
|---|---|---|---|---|---|
| Overworld | (spawn x, **clamp(spawn y, 32, 320)**, spawn z) | -64..320 | -192..576 | at origin y 32: -8,064..+19,584; at origin y 320: -18,432..+9,216 | [-20,000, 20,000) |
| Nether | (spawn x / 8, **1314**, spawn z / 8) | 0..256 | -128..512 | -51,912..-28,872 | [-60,000, -20,000) |
| End | (0, **-1140**, 0) | 0..256 | -128..512 | +36,432..+59,472 | [20,000, 60,000) |

- **The overworld origin's y is clamped.** Today the origin is the world spawn (`minecraft_world.rs:2793-2796`). A spawn below y 21, such as a superflat spawn at y -60, would put the overworld's top wall at map z 22,896, inside the End band. Clamping the origin's y to 32..=320 keeps the whole wall range inside the overworld band for every spawn; the spawn itself may then sit a little off map z 0, which the spawn gate already handles.
- **The End origin moves to -1140.** At -1186 the End's top wall would be at map z (512 + 1186) × 36 = 61,128, past its band. A larger negative origin moves it further up, so the origin moves the other way.
- **`bands::check()`** asserts at startup, and in a unit test, that every level's wall range lies inside its band and that the bands do not overlap.
- **The origins are fixed per world**, computed from the world spawn by `minecraft_survival::bands::origin(dimension, world_spawn)`. They replace the lazy `runtime.origins` (`minecraft_world.rs:116, 2796-2800`) and go to friends in `Login.origins`.
- **Precision:** f32 at |z| ≤ 60,000 resolves about 0.004 units, a ten-thousandth of a block. MW2's `PlayerState.origin` is replicated as exact floats (`netfields.rs:246`). No bounds clamp was found in `sim` or `net`; E4 tests it.
- **A wall at each band's edge:** outside a level's wall range, a trace hits a solid wall. Nothing can cross into the next band.
- **Falling into the void.** Vanilla's void damage starts at `min_y − 64` and does 4 damage every 10 ticks (`survival.rs:603`), but a player falling off an End island reaches y −64 at about 74 blocks/s (MW2 gravity 800 u/s², about 22 blocks/s²) and would hit the wall at `min_y − 128` about 0.9 s later, after one or two hits. So:
  - **a player whose feet reach the bottom wall is killed with the out-of-world cause** (`death.attack.outOfWorld`, "fell out of the world"), as vanilla's void would have done a little lower;
  - landing on a band wall never applies fall damage;
  - a spectator stops at the wall.

### 8.3 Changes in `sim::voxel`

- `VoxelWorld { brushes, levels: [Option<VoxelLevel>; 3], shapes, fluids }`, where `VoxelLevel { origin, chunks, min_y, height }`.
  - `band_of(z) -> Option<usize>` picks the level.
  - `to_block` and `to_map` take the level.
  - `trace_in` uses the start point's band and treats the band wall as solid.
- `activate(brushes, shapes)`, `set_level(dimension_index, origin, min_y, height)` and `drop_level(index)` replace `switch_level`. `set_chunk(index, ..)`, `remove_chunk(index, ..)` and `set_block_shape(index, ..)` are keyed. Shape ids stay in one table, and `runtime.shapes` and `shape_ids` are no longer cleared on a switch (`minecraft_world.rs:2813-2814`).
- **Events gain `level: u8`:**
  - `Shot`, `Explosion`, `MobShot`, `MobBlast` and `Ray` (`voxel.rs:111-124`);
  - `Shot` and `MobShot` also gain `attacker: Option<u32>`, as `MobBlast` has.

  This reshape is done **once, in E13a**, before the action and mob code is rewritten against it. Until E19 every event has `level: 0`.

  `MOB_BOXES` becomes per level, and `mob_on_segment` uses the segment's band.
- **`LEVEL_SWITCH`** (`:471-493`) is no longer needed for the host, because old ordnance stays in its own band. It is kept for the friend's prediction world, where it is stamped when the friend's own dimension changes.
- **Queries on the host:** water, lava, climb, stuck, ground, `awaiting_ground` and spawn all work from the point's band.
- **On a friend's game**, voxel holds only the friend's own level. On a dimension change, the client drops the old level and sets the new one.

### 8.4 Everything else that is level-wide on the host

- `obstacles()` (`minecraft_world.rs:2597-2619`) keeps only MW2 players in the level's band.
- The MW2 health bridge, `push_player_damage` and the Minecraft death queue carry the client in each entry (`voxel.rs:299-383`), but they are drained as a whole. Section 6.1 step 3 drains them once per frame into per-client maps, and pending damage becomes per client.
- Bots keep following the MW2 map. A bot ends up in the band of wherever GSC spawns it, which is the overworld origin. Bots in Minecraft multiplayer are untested and not a goal.

## 9. Entities and the dragon

### 9.1 Splitting `Entities`

`Entities` holds two kinds of thing:
- the level's server handle and outputs, including the drop staging that hands the host's own drops to the level thread (section 6.4);
- the client mirrors: `client` (`ClientMobs`), `world` (the cloned `EntityWorld`), `poof`, `portal`, `eyes`, `spell`, `ambient`, the drawn server items, `falling`, `lids`, `orb_pickups`, `sounds` and the weather view. `CRATE-SPLIT.md` section 2.5 lists the fields.

This track moves the mirrors into **`EntityView`**, fed by one `EntityFrame { mobs: Vec<ServerMob>, snapshot: EntitySnapshot }` per tick:
- on the host, `Entities::tick` builds the frame from its own server output for the host's view, as now;
- on a friend's game, `RemoteLevel` builds it from `StateFrame` and the mob messages.

`EntityView` mirrors only server items (`SERVER_ITEM`). The staging of drops and `to_hand` / `server_handed` stay in `Entities` on the host, so the host keeps its drops and a friend's mirror never tries to hand items to a server it does not have.

`Entities::meshes` (`entities.rs:1179`) becomes `EntityView::meshes` and works unchanged on either side.

### 9.2 Tracking per friend

- **Mobs: the tracker runs on the level thread.** The tags a new mob needs are built from `mob_tags(world, keep, originals)` (`server_mobs.rs:1048`), which takes equipment, attributes, the UUID and villager data from `originals`, the `ServerSim::mob_tags` map (`server.rs:556`). Those originals exist only on the level thread; the main thread only gets the `tracked_mobs` clone (`server.rs:1200-1206`). So:
  - `TickInput.tracking` lists each player's position and view distance. The level thread keeps a tracked set per player.
  - A mob is in range when its horizontal distance is within `min(range(kind) × 16, view_distance × 16)`. This is vanilla's `TrackedEntity.updatePlayer`. The per-kind range is in chunks: player 32; zombie, skeleton, creeper, spider, enderman, witch and blaze 8; cow, pig, villager, ghast and ender dragon 10; end crystal 16; anything else 5.
  - A mob entering a player's range is output as `mob_adds (player, id, tag)` with its full saved tag, and sent as `MobAdd`. A mob leaving range, or removed, is output as `mob_removes` and sent as `MobRemove`.
  - The friend loads the tag into its `EntityWorld` with `spawn_saved` (`server_mobs.rs:424`), extended to keep the host's id.
  - The session only sends adds and removes for chunks in the friend's `sent` set; an add for a chunk not sent yet waits until the chunk is queued.
- **Mob looks.** The tag also holds values that change all the time (`fall_distance`, `Fire`, `Health`, `anger_end_time`, `server_mobs.rs:1065-1085`), so hashing the tag would resend most mobs every few ticks. Instead the level thread keeps a small `MobLook` per tracked mob (section 4) and, every 10 ticks, compares it with the last one sent. A change is output in `mob_looks` and sent as `MobLook` to every friend tracking that mob.
- **Mob moves.** The friend's `ServerMob` list goes in `StateFrame.mobs` every tick, without vanilla's per-kind intervals. **It must hold every mob that friend tracks**, because `ClientMobs::receive` (`minecraft_terrain/src/client_mobs.rs:896`) marks any mob missing from a batch as unseen and spawns any mob it has not seen from the `ServerMob` alone. The friend's `RemoteLevel` therefore drops ids that have had no `MobAdd` before it calls `receive`. `receive` replays vanilla's tracker encoding itself, so the friend's poses match the host's exactly.
  - Flags that renderers read but neither the NBT nor `MobLook` carry are added to `ServerMob`, and the agent records the list in the log. Likely ones are a creeper's swell, a zombie's raised arms and an enderman's anger.
- **Level entities.** `StateFrame.entities` is the level's `EntitySnapshot` filtered to the friend's range:
  - items and orbs within 96 blocks;
  - TNT and falling blocks within 160;
  - thrown items, tridents, eyes and fireballs within 64;
  - crystals within 256;
  - the dragon, its fireballs and its breath always in the End.

  `EntitySnapshot` and its view types derive serde (`minecraft_terrain/src/server.rs:170-226`; `serde` is already a dependency of `minecraft_terrain`). Item components already travel as JSON text (`ServerItem.components`).
- **Changing dimension** clears the friend's tracked sets on the old level's thread and its `sent` and `pending` chunk sets, and `StateFrame.epoch` changes with `Dimension.epoch`.

### 9.3 Who did it

- **`VoxelEvent::MobShot`** gains the attacker (section 8.3), and `entities.shoot` takes the attacker's yaw, not the host's (`minecraft_world.rs:1532-1564`).
- **`MobBlast`** credits `attacker` to that player, not only `local` (`:1553-1555`).
- **XP, loot and mob anger** go to the right player as a result.
- **Mining:** `Mining.progress` moves from `Runtime` (`minecraft_mining.rs:27`) to `Level`, keyed by block. The bullet's attacker decides whose held item counts for drops (silk touch, fortune) and who gets the block XP. Particles stay in the presenter.

### 9.4 The dragon fight

The fight already takes every player (`tick_level(players.values())`). The dragon, its crystals, fireballs and breath bursts are in `EntitySnapshot`.

Per player:
- a `BossBar` message for every player in the End within the fight's range. `minecraft_end::tick_fight` only shows it for `local` today (`minecraft_end.rs:120-124`);
- `Credits` when a player first enters the exit portal (`seenCredits` per player; `minecraft_world.rs:619` is local only today). The player is held in the travel switch while the credits roll (`holding_for_credits`, `minecraft_world.rs:679`). Today only the host's UI ends it (`credits_done`, `minecraft_end.rs:205-216`). A friend's game ends it with `ToServer::CreditsDone`, vanilla's `PERFORM_RESPAWN` after the win screen. A friend who leaves during the credits is released by the leave flow;
- the dragon's hits, breath and wing knock-back reach each player through the keyed `player_hits` and `players_pushed`.

## 10. Player actions and menus

### 10.1 Actions

The action block (`minecraft_world.rs:1031-1527`) reads `buttons.pressed(MouseButton::...)` and `pad_trigger` directly and acts for `local`. It acts on edges every frame (`just_pressed`, `:1058`, `:1159`) and on hand ticks (`:924-929`). It is split in two:

- **`render_anim/src/minecraft_input.rs`** (the presenter, on any game):
  - samples Bevy input into `HandEvent`s (press and release of attack and use, hand swap, drop) and a `HandState` (held buttons);
  - computes `Target` with the same raycast as today, against the local scene and the local entity mirror. On the host that is the shared scene; on a friend's game, the `RemoteLevel`;
  - plays the first-person swing at once.
- **`minecraft_survival::actions::act(actor, events, state, level, player, data) -> Vec<ActionEffect>`** (the rules, on the host only):
  - uses each queued event once, in order, and the held state for continuous use;
  - punch, open a station or block, use a block, campfire food, respawn blocks, buckets, item use, throws, placement, doors and scaffolding, and shulker placement;
  - it checks the target first:
    - a block within 4.5 + 1 blocks (creative 5 + 1) of the actor's **authoritative** eye;
    - a hit point within 1.0000001 of the block centre on every axis;
    - a mob within 3 + 3 blocks (`ServerGamePacketListenerImpl` constants);
  - it returns effects (sounds, particles, the hand animation of others) instead of playing them. The host plays effects in its own level and range, and sessions forward them to friends in the same level within vanilla's radius.

The host's own input goes through the same events and state, so the host and friends run one code path.

Some parts stay in `render_anim` behind small traits, because they read MW2 or Bevy state:
- the MW2 push for Riptide and the trident release (`:1011-1029`), which goes through the MW2 write-back of the per-player step;
- the creative "break now" list.

The camera, F5 and puppet code stays in `render_anim`.

### 10.2 Menus

`InventoryUi` (`minecraft_inventory.rs:28-58`) holds game state per player (`table`, `block: Option<Opened>`, `bench`, `experience`, `spent`, `level`, `shelves`, `seed`, `used`, `anvil_used`) mixed with UI state. It is split:

- **`minecraft_survival::menus::Menu`** lives on `Player` as `player.menu: Option<Menu>`. It holds:
  - the game state above, **including the station inputs in `bench`** (`minecraft_inventory.rs:41`), which today are in no snapshot. They are returned to the player when the menu closes and are included in the autosave snapshot of the player;
  - the **dimension** the menu was opened in;
  - the `container_id`, cycling 1..=100, and `state_id: u16`, which increments `& 0x7fff` on every change;
  - `remote: Vec<HashedSlot>` and `remote_carried`: vanilla's `remoteSlots`, the last state sent to that player.
- **`menus::click(player, level, Click)`:**
  - ignores a click whose `menu` is not the open one, that comes from a spectator or a dead player, or whose menu's dimension is not the player's current one;
  - checks reach (`stillValid`, the block within 8 blocks of the eye) and the slot index;
  - applies the click through the transactions in `minecraftoss-player` (`inventory.rs:433`, `chest.rs:30`, `furnace.rs:164`) and the stations;
  - if the `state` was stale, marks the menu for a full resend.
- **`menus::rename(player, name)`** applies `RenameItem` to an open anvil.
- **`menus::view(player, level) -> MenuView`** builds what the screen shows. **`menus::changes(player, level)`** builds what changed since `remote` (`MenuSlots`), vanilla's `broadcastChanges`, and runs every tick for every open menu.
  - Two players in one chest each have their own `Menu` bound to the same `Containers` entry (keyed by position, `level.rs:234-297`), so each sees the other's clicks within a tick.
- **Closing.** A menu closes with `CloseMenu`, or the host closes it on its own with `MenuClose`: reach lost, block broken, **dimension travel**, and **death, before the death drops**, so what was in the grid and the cursor drops with the rest. On every close, items go back to the inventory and the overflow drops at the player's feet.
- **The UI in `render_anim::minecraft_inventory`:**
  - it draws from a `MenuView`. On the host that view comes from `menus::view` for the host's player; on a friend's game, from the last `MenuOpen` plus `MenuSlots`;
  - `ui.clicks` (`frame::McClick`) becomes `Click`. On the host it goes straight to `menus::click`; on a friend's game it is sent;
  - the friend's screen does not predict. It shows the carried stack and slots as the host sends them, one round trip later. That is acceptable on localhost and with friends.
- **Creative inventory:** the host's creative screen keeps its direct path. A friend's creative screen (`ServerboundSetCreativeModeSlotPacket`) is deferred; a friend in creative mode has no creative tab in this round.
- **Lids** keep working from the openers per UUID.

### 10.3 The hotbar and the MW2 gun

MW2 guns are items in the Minecraft inventory (`sync_weapons`, `minecraft_inventory.rs:154`). The selected slot drives the MW2 weapon through a client-side `weapon_request` (`minecraft_inventory.rs:533-559`, used at `minecraft_world.rs:2076-2083`), and an MW2-side switch (out of ammo, a killstreak) moves the selection the other way. For friends:

- The host runs `sync_weapons` and the follow-MW2 rule for **every** player from `AuthorityWorld`, so gun items are in each player's inventory.
- The friend's game turns `PlayerView.selected` and the item there into the weapon in its own usercmd, with the same `weapon_request` rule (including the "needs sky" refusal).
- `SelectSlot` from the friend sets `selected`. The host changes `selected` itself only on an MW2-initiated switch, and sends it in `PlayerView`.

## 11. The friend's game: a thin client

### 11.1 `minecraft_survival::remote::RemoteWorld`

`RemoteWorld` has no Bevy and can be unit-tested. It holds:
- `RemoteLevel { dimension, epoch, origin, stream: TerrainStream (remote), scene, view: EntityView, mobs: EntityWorld, added: HashSet<u64> }`;
- `player: PlayerView`, `menu: Option<MenuView>`, `players` (the player list), `chat: Vec<Clientbound>`, `boss_bars`, `time`, `weather`, `rules`;
- `reliable: VecDeque<ToClient>` (messages waiting behind a marker), `batches: BTreeMap<u32, DecodedBatch>` (batches that arrived early) and `applied_seq`.

`apply(ToClient)` and `apply_state(StateFrame)` update it.

Chunk blobs are decoded on a `minecraft-chunk-read` worker with `read_chunk`, then applied in order on the main thread.

### 11.2 Ordering

The reliable lane is already ordered. Only chunk batches travel apart, on uni streams. So:

- The friend applies reliable messages **strictly in the order they arrive**. When it reaches a `ChunkBatch { seq }` marker, it applies that batch if it has it; if not, it **stops applying the reliable stream** and queues what follows until the batch has been decoded. Every message (chunk forgets, block updates, mob adds, menus, player views, `Dimension`) is therefore applied in the one order the host sent it, and nothing for a new level can reach the old one.
- **`Login` is the first message in the stream.** A batch that arrives before `Login` is kept in `batches`; nothing is applied before `Login`.
- A batch whose `seq` is already applied (the host's resend) is ignored. Both queues are bounded: past 64 MiB the friend leaves with "Lost the world stream".
- A block update applies only to a loaded chunk. The host sends updates only for chunks it already queued, and the order guarantees the chunk arrived first.
- A `StateFrame` is ignored if it is older than the last applied one, or if its `epoch` is not the current level's. Mob moves for an id with no `MobAdd` are dropped before `ClientMobs::receive` (section 9.2).

### 11.3 Presenting

**`render_anim/src/minecraft_remote.rs`** is a system in the same set as `minecraft_world::update`, active only in `RuntimeRole::Client`. It:
1. **Loads.** On install it loads the registries, packs and atlas with no world, then waits for `Login`, then builds the `RemoteLevel`.
2. **Feeds the collision.** It calls `sim::voxel::activate` with the **prediction world's** brushes (`net::ClientPredictionState`, `crates/net/src/client/runtime.rs:31`) and `set_level` with `Login.origins`. Each loaded chunk goes through the shape half of `apply_streamed`.
3. **Holds the loading screen** until the chunks around the feet have collision.
4. **Presents.** It fills `MinecraftWorldView` (terrain uploads, sky, light, fog, clouds, entity meshes, particles, cracks, gear) and `MinecraftUi` (hearts, food, XP, hotbar, effects, screens, chat, boss bar, credits, death card) from `RemoteWorld`.
   - The presentation half of `update` (`minecraft_world.rs:1655-1700, 2160-2260`) is extracted into functions that both adapters call: terrain frame, environment and sky, entity meshes, hand and held item, and HUD from `PlayerView` and `MenuView`.
5. **Plays** the `Sound`, `LevelEvent`, `Explosion` and `Crack` messages through the same particle and sound code the host uses.
6. **Sends:**
   - hand events as they happen, coalesced into at most one reliable frame per tick;
   - `HandState` every tick while a button is held, on the datagram lane;
   - `SelectSlot` on change, and `Click`, `RenameItem`, `CloseMenu`, `Chat` and `CreditsDone`;
   - `Ack` when nothing else carries the acknowledgement.

   Respawning is MW2's: a new MW2 life (`observe_life`, `minecraft_world.rs:689-690`) respawns the Minecraft player on the host. There is no separate respawn message.

### 11.4 Prediction

- **Movement:** MW2 prediction with reconciliation (`net/src/client/predict.rs`, `reconciliation.rs`) against the friend's own voxel copy.
  - Teleports, knock-back, Riptide and wind charges are applied by the host to the friend's MW2 state. The friend sees them after one round trip as a correction.
  - Abilities come through the friend's own replicated `pm_flags`, so prediction agrees with the host: creative flight, `MC_MAY_GLIDE`, the sprint block.
- **Blocks:** not predicted. A placed or broken block appears after one round trip. The first-person swing and the hand animation play at once. Vanilla's sequence-acked prediction is deferred.
- **Bullet impacts:** MW2's own client effects. Mining progress and cracks come from the host.

## 12. How other players look

- **The body:** the MW2 remote-body path (`crates/render_anim/src/occupancy/remote_body.rs`) draws each friend as their MW2 soldier with their gun and animations. Nothing changes there.
- **Gear:**
  - The host builds `body_gear(inventory, selected, mode)` (`minecraft_survival/src/gear.rs:7`) for **every** player each tick. It sets `McBodyGear` under each client number. Today only `local` is set (`minecraft_world.rs:2202-2211`).
  - On a change, it sends `Equipment { client, gear }` to every friend, vanilla's `ClientboundSetEquipmentPacket`.
  - The friend's game sets `McBodyGear` from those messages, including its own entry for F5. `shown_weapon` already hides the gun under a Minecraft item (`minecraft_gear.rs:230`).
  - The corpse rule is set per client when that client dies.
- **Elytra and gliding:**
  - The chest slot already carries `elytra` in `GearView`.
  - **No glide message is needed.** Every client's full `PlayerState`, with `pm_flags` and its `MC_GLIDING` bit, reaches every other client: snapshots carry `players: Vec<(ClientId, PlayerState)>`, `pm_flags` is a replicated field (`net/src/transport/netfields.rs:242`) with no per-viewer filtering, and `PresentedSnapshot::player(id)` returns it for any client (`net/src/client/presentation/presented.rs:257-267`). Track C's brief (`docs/round8/c4-elytra-body.md`, "Replicated state per body") builds the glide pose from exactly that.
  - **The wings are drawn nowhere today** (`armor_render.rs:22-47` knows only the four armour pieces). Track C (c4) builds the wing model and the glide pose for every body. E15 therefore **depends on c4 being merged**; if it is not, E15 sends the gear and the owner step for the elytra waits for c4.
- **Dimension gate:**
  - `sync_remote_bodies` / `occupy_remote_scene_ents` (`remote_body.rs:245-246, 367-376`), `gear_rigs` (`minecraft_gear.rs:483`), `draw_body_gear` and the overhead names (`crates/hud/src/overhead_names.rs:109`) skip a body whose `voxel::band_of(origin.z)` differs from the viewer's;
  - corpses follow the same rule.
- **Names:** overhead names are MW2's. The Tab list is MW2's scoreboard.

## 13. Chat, death messages, kill feed and PvP

### 13.1 Chat and commands

This comes early (E8), because every later owner test uses `/op`, `/give`, `/tp`, `/kill` and `/gamemode` on the friend.

- **Ids and inputs.** The `PlayerId` of each player is their MW2 client number. `commands::run(globals, levels, players, from, ..)` (`minecraft_world.rs:713`) runs for every player with lines waiting: the host from `ui.chat.sent`, a friend from `ToServer::Chat`.
- **Presence.** `globals.chat.join` runs for every player who joins (today only the host, `minecraft_world.rs:481`), and `presence` lists every online player (`minecraft_world.rs:707-712`), so `/tp Friend`, `/give Friend`, `/op Friend`, `@a` and `@p` see both. The leave flow calls `globals.chat.leave`.
- **Routing.** `pump` (`commands.rs:891-911`) stops keeping only what reaches the sender. It returns every `Outgoing`. The host shows those that reach player 0 and sends the rest to the session whose `PlayerId` they reach (`Recipient::reaches`, `protocol.rs:62-77`). `chat.type.text` is broadcast.
- **Permissions.** The host is the owner through `CommandServer.host` (section 6.5). Everyone else's level comes from `ops.json` by UUID, as today. A friend has level 0 until `/op <name>`.
- **Suggestions.** `CommandHints` are not sent to friends in this round. A friend's chat has no tab suggestions (deferred).

### 13.2 Death messages and the kill card

- **Translatable lines.** `DeathLine` keeps the translation key and its arguments (`survival.death_line`, `survival.rs:454`; `death.rs:372, 618`). `commands::death_message` (`commands.rs:1046`) returns a `Text::translate` instead of `Text::literal`, and each game resolves it when it shows it. Both games only ever load `lang/en_us` (`minecraft_terrain/src/item_icons.rs:74-76`, and `settings.cfg` has no language key), so in this round every line shows in English. A language setting is deferred.
- **Broadcast.** When `show_death_messages` is on, the line goes to everyone. Today it is built for `local` only (`minecraft_world.rs:1873-1892`).
- **PvP deaths are told from the MW2 event.** Today the death line is built only inside `if alive && !player_damage_pending()` → `settle.damage` (`minecraft_world.rs:1857-1892`). A lethal MW2 hit sets `pm_type` to dead in the same sim tick, so the step takes the `dead` branch (`:781-784`) and never settles. So:
  - `DamageCause::Player { attacker: String, weapon: Option<String> }` is new;
  - when the host's tick has `SimEvent::Died { victim, attacker: Some(a), weapon }` (`crates/sim/src/match_state/events.rs:141-151`) with `a != victim`, the host builds and broadcasts the line directly from that event: `death.attack.player` ("%1$s was slain by %2$s"), or `death.attack.player.item` with the MW2 weapon's display name as `%3$s`;
  - that death is marked as told on the victim's survival, so the settle path cannot add a second line for it.
- **Kill card.** The friend's card comes from `KilledBy`. Today the card is filtered to the host (`minecraft_world.rs:793-796`).
- **Death drops** come from each player's own `death_watch` at their own death spot. Today only `local` drops (`:2066-2073`). `keep_inventory` applies to everyone. An open menu is closed first (section 10.2).

### 13.3 Kill feed

- A gun kill between players already gives the MW2 obituary, killcam and card for everyone. A friend is an MW2 client.
- Minecraft deaths (mobs, falls, lava) stay out of the MW2 kill feed (`hides_minecraft_obituary`, `sim/src/script/host/players.rs:92-98`), as the owner asked in round 7 (section 1). They show in everyone's chat, as in vanilla.

### 13.4 The `pvp` game rule and the MW2 game type

- **The game type is free-for-all.** The Minecraft map runs whatever game type the host picked (`session/src/match_apply.rs:950-960`). In a team mode with MW2's friendly fire off, teammates could not hurt each other whatever `pvp` says, and team balancing kills players and triggers `death_watch.switch_teams`, which empties an inventory. So a Minecraft load always uses `dm`, and the `pvp` rule is the only gate between players. A test checks that.
- **Match end.** A Minecraft load already sets `scr_<gametype>_timelimit` and `scr_<gametype>_scorelimit` to 0 (`match_apply.rs:1049-1056`), and the sim's own `score_limit` only feeds the snapshot meta (`sim/src/world.rs:3102`). A test pins that a Minecraft load zeroes both for `dm`, so PvP kills never end the world.
- **Where the rule lives.** `pvp` is a vendored rule that defaults to on (`third_party/minecraftoss/world/src/game_rules.rs:150`), and today nothing reads it. The host writes it each tick into `sim::voxel::set_pvp(bool)`.
- **MW2 damage.** `apply_damage_attempt` (`crates/sim/src/damage.rs:881`) returns a new `DamageRefusal::PvpDisabled` when all of these hold:
  - voxel is active;
  - the attacker is a player client;
  - the attacker is not the target;
  - pvp is off.

  That covers bullets, MW2 grenades and killstreaks. Self-damage, mobs and the world still hurt.
- **Minecraft hits.** Thrown items and tridents hitting players (`players_struck`, now keyed) honour the same rule.
- **Not covered.** Minecraft melee against another player (a sword) is deferred. `entities.punch` reaches mobs only, and a player target needs a damage path through `survival.hurt` with armour.
- **Armour.** MW2 bullet damage ignores Minecraft armour, shields and Protection (`survival.rs:275-293`, the test at `:1182`). That stays. It is listed as a known difference.

## 14. Saving

- **The host owns every player file.** That includes friends: `players/data/<uuid>.dat` in the host's world. Files are written:
  - by autosave every 6000 ticks (`WorldFiles::of` already snapshots every player, `server.rs:671-686`);
  - when a friend leaves (section 5.2);
  - when the world closes (`Server::save_and_close`, `server.rs:523`).
- **One writer thread with a queue.** Today `save_player` and `autosave` join the previous writer on the main thread before starting the next (`server.rs:717-719`, `:744-746`), which can stall the main thread behind an autosave, and a stalled main thread stops the control reader (section 3.1). They become sends on a channel to one long-lived `minecraft-world-save` thread that writes in order. `save_and_close` still waits for it.
- **`recently_left`.** `join` reads player files synchronously (`server.rs:585`) without waiting for the writer. A friend who leaves and rejoins quickly, or retries the moment the old session is retired, would read the old file and get items back that they had already put in a chest. So the leave flow keeps the `PlayerData` in `Globals.recently_left` until the writer reports that file written, and `join` reads from there first.
- **`place_player`** (`minecraft_world.rs:2455-2488`) runs for every player from their MW2 state, so a friend's saved `Pos`, `Rotation` and `Dimension` are current.
- **The friend's game writes nothing.** It opens no world folder and takes no `session.lock`.
- **`level.dat`'s `singleplayer_uuid`** stays the host (`save.rs:109`). That is vanilla-correct for the owner of a world opened to LAN.
- **A `temp` world** (`IW4L_MINECRAFT_WORLD=temp`) throws away the friends' progress with the world.
- **Two copies of one name:** the second `Hello` is refused, so one file never has two writers.
- **A host crash after a friend left** leaves that friend's file newer than the chunks, which were saved at the last autosave. Items the friend moved from a chest into their inventory are then in both. Vanilla has the same window (it saves a player at logout and chunks at autosave), so this is recorded as vanilla-parity behaviour, not fixed.

## 15. Failure modes

| What happens | Behaviour |
|---|---|
| A friend quits or crashes | MW2 retires the client after its disconnect or the 8 s idle timeout. The leave flow settles and saves them. Their body and gear disappear. Everyone sees "<name> left the game". |
| The host quits or crashes | The MW2 match ends for everyone. The friend's game tears down, as for any ended MW2 match (`torn_down`, `minecraft_world.rs:426-430`), and goes back to the menu. Its chat shows "The host closed the world" when the link closed cleanly. A host that quits through the menu saves everything first (`save_before_exit`). After a host crash, the world and every player who was still online are as of the last autosave; a friend who left earlier keeps the file saved when they left (section 14). |
| A friend's connection is slow | The reliable lane and chunk batches both wait for the friend's acknowledgements, within a byte budget, so neither piles up in the master. A friend with no acknowledgement for 20 s, or an unsent backlog over 4 MiB, is dropped with "connection too slow". State frames that arrive late are ignored. If the master's queue fills anyway, the patched master closes the friend, never the host. |
| The host's main thread stalls | Friends send at most one reliable frame per tick within a window of 4, and held buttons go on the datagram lane, so a stall of several seconds does not fill the master's queue to the host. |
| A packet is lost | Reliable lane: QUIC resends it. State lanes: the next tick replaces it. Bulk lane: a batch with no `ChunkBatchReceived` after 3 s is sent again with the same `seq`, whether the master refused it, the forward failed or the friend dropped it; the friend ignores a `seq` it already has. After 30 s without it, the friend is dropped with "Lost the world stream". |
| Version mismatch | MW2 refuses mismatched builds at the handshake (protocol 95, content fingerprint). `Hello` checks `minecraft_net::PROTOCOL` and the registry fingerprint, and a refused friend's MW2 client is retired. |
| The host's world is still loading | `Hello` is held until the server is ready. The friend sees the loading screen and keeps resending `Hello`, which the host ignores until it answers. |
| The friend changes dimension | The host sends `Dimension { epoch }`. The friend drops its level, voxel band and mirrors, shows the dimension loading screen and builds the new level from the chunks that follow. The host clears the friend's chunk and entity tracking for the old level and closes their menu. |
| The master stops | Both games lose the relay. This is the same as the host quitting, but without a clean close. |

## 16. Trust and limits

The trust model is **friends only.** The room should have a password (`IW4L_MASTER_PASSWORD`, on the host and on every friend), and the master is the host's own or a VPS the owner runs. The limits stop mistakes and stuck clients, not determined cheaters:

- **Movement** is the host's MW2 simulation. A friend cannot teleport, fly without the ability, or walk through blocks the host has.
- **Identity** comes from the MW2 connection's name, not from anything the friend's Minecraft code sends.
- **Every target** is checked for reach from the authoritative eye. **Every click** is checked for menu id, dimension, slot range and reach. **Anvil names** are length-limited.
- **Strings:** chat up to `MAX_CHAT_LENGTH` (256, `minecraft_commands/src/protocol.rs`); names as MW2 allows.
- **Commands** use `ops.json` permission levels; the host is the owner.
- **Decoding** is size-limited per lane. Junk is dropped and counted, and repeated junk drops the friend.
- **Files:** a friend never names a file on the host. Their UUID comes from their name, which `save.rs` turns into a path, and the UUID format keeps the path safe.

## 17. Testing on one PC: two game windows

### 17.1 What the script does

`scripts/mp-local.ps1` is new in commit group E1. It follows `xtask duo`'s launch sequence (`xtask/src/duo.rs`: one working folder per role, console control through stdin, wait for `members>=2`), adapted to Windows and to the game folder `C:\Games\Mashup\2010-Rust-Rewrite-Mashup`.

**One working folder per role.** On Windows the launcher changes into the exe's own folder at start (`launcher/src/main.rs:34-49`), so a working folder is set by where the exe is, not by the process's current directory. The script therefore makes `iw4l-artifacts\mp-local\host\` and `...\friend\`, each with:
- a **hard link** to the game exe (`New-Item -ItemType HardLink`, same drive, no admin), so no copy of the exe is made;
- **directory junctions** (`mklink /J`, no admin) for `iw4l-artifacts\cache` and `iw4l-artifacts\minecraft-26.3` to the game folder's own;
- its own `settings.cfg`, logs, `latest.log`, `command_history.txt` (`hud/src/minecraft_chat.rs:29`), creative hotbars, `profile\` (copied once from the game folder's) and screenshots, which today two games in one folder would share;
- its own `.env`, rewritten on every run from the game folder's `.env` without `IW4L_MASTER_HOST_NAME`, `IW4L_MASTER_JOIN` and `IW4L_MINECRAFT_WORLD`, with relative paths made absolute and `IW4L_SKATE=off` added when skating was never set up. A role folder needs one: the launcher's first-run check reads only the `.env` beside the exe (`launcher/src/first_run.rs:17-29`), and without it would ask for the MW2 folder and offer the Skate 3 conversion in each role folder. `IW4L_GAMES`, the password and the kit therefore still apply. The script passes everything else per process.

The host's saves are in its role folder, so the owner's main world is never touched.

The steps:

1. **Memory.** The script takes both cargo slots (`Local\iw4l-cargo-slot-0` and `-1`, `scripts/cargo-slot.ps1:152-153`) while the windows run, so no build starts beside two games and a relay. It waits for them first and says so. E1 logs the peak working set of each window and the relay.
2. **Certificates**, made once in `iw4l-artifacts\mp-local\certs` with Git for Windows' OpenSSL (`C:\Program Files\Git\usr\bin\openssl.exe`). These are the same keys and names `cargo xtask certs` makes (`xtask/src/certs.rs`): an RSA 3072 CA `iw4l-ca.pem`, and `server-key.pem` / `server-cert.pem` with `subjectAltName=DNS:iw4l-prod,DNS:iw4l-dev`.
3. **The relay:** `iw4l-master.exe serve --bind 127.0.0.1:4433 --cert ...\server-cert.pem --key ...\server-key.pem`, logged to `mp-local\master.log`.
4. **The host window:** `mp-local\host\<exe> map <map>` with:
   ```
   IW4L_MASTER_ADDR=127.0.0.1:4433
   IW4L_MASTER_SERVER_NAME=iw4l-prod
   IW4L_MASTER_CA_CERT=<game folder>\iw4l-artifacts\mp-local\certs\iw4l-ca.pem
   IW4L_MASTER_HOST_NAME=Local world
   IW4L_MASTER_MAX_PLAYERS=4
   IW4L_MASTER_STATUS_FILE=<host folder>\master.status
   IW4L_SETTINGS_PATH=<host folder>\settings.cfg
   IW4L_MINECRAFT_WORLD=<world>
   IW4L_CONSOLE_STDIN=1
   ```
   `settings.cfg` is a copy of `iw4l-artifacts\settings.cfg`, made again on every run, with `fullscreen=false`, `resolution=1280x720` (960x540 when the main screen is narrower than 2560, or `-Resolution`) and, unless `-HostSound` is given, `master_volume=0` (`console/src/user_settings.rs:210`), so the owner hears only the friend window.
5. **Wait** for `state=hosting` and a `room=` line in the host's `master.status`. This takes up to 90 s.
6. **The friend window:** `mp-local\friend\<exe> map <map>` with the same first three variables and `IW4L_CONSOLE_STDIN=1`, plus:
   ```
   IW4L_MASTER_JOIN=<room from the host's master.status>
   IW4L_MASTER_STATUS_FILE=<friend folder>\master.status
   IW4L_SETTINGS_PATH=<friend folder>\settings.cfg
   ```
   Its `settings.cfg` is the same copy with `player_name=Friend` and full volume. A different name means a different UUID and player file.
7. **Wait** until the host's status shows `members>=2`.
8. **Console control.** The script keeps both games' stdin open (`IW4L_CONSOLE_STDIN`, as `xtask duo` does; each game runs under a hidden `cmd /c` that writes its stdout and stderr to `game.out.log` and `game.err.log` in the role folder) and reads lines from its own console: `h <command>` goes to the host's MW2 console, `f <command>` to the friend's, and `quit` sends `quit !` to both. That lets the owner run MW2 console commands in the window that does not have focus. Minecraft chat commands are typed in the window itself.
9. **Shutting down.** The script waits until both games have exited, then stops the relay and releases the cargo slots. It prints the peak working set of each window and the relay and appends it to `mp-local\memory.log`. A relay that was already running when the script started is used and left running.

**Parameters:**
- `-Exe` defaults to the newest `iw4l-mp-*.exe` in the game folder (section 19), never `iw4l-survival.exe` unless named;
- `-Map` defaults to `minecraft:overworld`. `iw4:mp_rust` tests the bare MW2 relay before any Minecraft code is in;
- `-Friend` defaults to `Friend`;
- `-World` sets the host's world name, default `mp-test`; **`-Fresh`** deletes that world from the host's role folder first, so both players get the kit again;
- `-HostSound` keeps the host's sound on;
- `-FriendOnly` starts only the friend window against the room in the host's `master.status`, for a rejoin while the host keeps running (it does not take the cargo slots, which the first run still holds);
- `-CertsOnly` makes the certificates and stops;
- `-FriendKit` builds the friend's zip (section 18.2) as `mp-local\friend-kit-<exe name>.zip` and stops; `-PublicAddress <ip>` fills the host's address into its `.env`;
- `-Bats` writes `Host Minecraft World.bat` and `Join Minecraft World.bat` into the game folder (section 18) and stops. These and the kit's join `.bat` start the map given with `-Map`, so `-Map iw4:mp_rust` makes them for the plain MW2 check;
- `-GameDir` (default `C:\Games\Mashup\2010-Rust-Rewrite-Mashup`), `-Relay` (default `iw4l-master.exe` in the game folder), `-Port` (default 4433) and `-Resolution` (for example `960x540`).

### 17.2 Rules for the two-window setup

- The script sets `IW4L_MASTER_HOST_NAME` and `IW4L_MASTER_JOIN` per process. **They must not be in `.env`.** `dotenvy::from_path` does not override variables the process already has, so a value in `.env` reaches both games, and `master.rs:368` refuses both being set.
- `IW4L_GAMES` and `IW4L_MINECRAFT_KIT` stay in `.env`. The kit is applied by the host, so the friend gets it too on their first join to a fresh world.
- The kit is given **once per player file** (`joining_inventory`, `server.rs:600`), so each milestone's test uses its own world (`-World mp-m1` … `mp-m6`) or `-Fresh`.
- The friend's game no longer opens the save folder, so there is no world-lock fallback.
- Two windows of 1280×720 fit side by side on a 2560-wide screen. On a 1920 screen, use `resolution=960x540`, as `docs/DUO.md` does.
- **Input only reaches the focused window** (`console/src/plugin/mod.rs:388` gates on `!devices.focused`). The owner tests alone, so every owner step is written as an alternating sequence: act in one window, Alt-Tab, check in the other. Commands for the window without focus go through the script's console (step 8).
- **Building the relay:**
  - `& scripts\cargo-slot.ps1 build --release --locked -p iw4l-master` gives `D:\iw4l-target\release\iw4l-master.exe`. Copy it next to the game exe.
  - The crate has no Unix-only code (`crates/iw4l-master/Cargo.toml`, `main.rs` reads only flags).
  - It is rebuilt once after E3's master patch (section 3.2 rule 3), and after any later change to `iw4l-master` or `master_protocol`.

### 17.3 Done by hand, without the script

1. Make the certificates with the five OpenSSL commands in the script.
2. Start `iw4l-master.exe serve ...`.
3. Make the two role folders with the exe hard link, the two junctions and a copy of the game folder's `.env` (section 17.1).
4. Start the host from its folder with the variables above in a PowerShell window (`$env:NAME = "value"`, then `& .\iw4l-mp-m1.exe map minecraft:overworld`).
5. Read the room from the host's `master.status`, or from the host's chat line.
6. Start the friend from its folder in a second PowerShell window with its variables.

## 18. What a friend installs and how they join

The internet path is checked in M1 with plain MW2 (`-Map iw4:mp_rust`), because it does not depend on any Minecraft code.

### 18.1 The host (the owner) once

1. **Build and copy the relay:** `iw4l-master.exe` (section 17.2).
2. **Make the certificates** with `scripts\mp-local.ps1 -CertsOnly`. Hand out `iw4l-ca.pem` only, never the keys.
3. **Run the relay for the internet:** `iw4l-master.exe serve --bind 0.0.0.0:4433 --cert ... --key ...`.
   - Allow it through the Windows firewall for UDP on private and public networks.
   - Forward **UDP 4433** on the router to the PC.
   - Alternatively run the master on a VPS (`docs/MASTER.md`), with the patched build.
4. **Host:** the `.env` gets `IW4L_MASTER_PASSWORD=<room password>`. Start the world from `Host Minecraft World.bat` (`-Bats`). It sets `IW4L_MASTER_HOST_NAME`, `IW4L_MASTER_MAX_PLAYERS=4`, `IW4L_MASTER_ADDR=127.0.0.1:4433`, `IW4L_MASTER_SERVER_NAME=iw4l-prod`, `IW4L_MASTER_CA_CERT` and `IW4L_MASTER_STATUS_FILE=iw4l-artifacts\master.status` (its `room=` line is the join code) itself, and starts `iw4l-master.exe serve --bind 0.0.0.0:4433` minimised when no relay is running, so step 3 is done by the `.bat`. To host from the menu instead, put the three `IW4L_MASTER_ADDR`, `_SERVER_NAME` and `_CA_CERT` lines in `.env`.
5. **Tell the friend** the public IP address, the room password and the join code from the chat.

### 18.2 The friend once

1. **Get the friend's zip,** made by `scripts\mp-local.ps1 -FriendKit`. It holds an explicit list of files and nothing else:
   - the same `iw4l-mp-<milestone>.exe` (or `iw4l-survival.exe`) build as the host's;
   - the licence files (LICENSE, NOTICE, the font licences) and README;
   - `iw4l-ca.pem`;
   - `Join Minecraft World.bat`;
   - a `.env` template with the lines below;
   - `How to join.txt` with these steps;
   - an empty `iw4l-artifacts\` folder.

   It never holds Mojang's files (`iw4l-artifacts\minecraft-26.3`), MW2 content (`iw4l-artifacts\cache`), the owner's saves, settings, logs, `profile\`, `command_history.txt`, `skate\`, `.env` or certificate keys. The friend's game makes its own copies from the friend's MW2 and from Mojang.
2. **Have their own copy of MW2.** `IW4L_GAMES` is empty in the template, so the first start finds or asks for the MW2 folder and writes it, as for any install of this game.
3. **Check `.env`.** The script fills in the address from `-PublicAddress` and the password from the host's `.env`; otherwise the friend fills them in:
   ```
   IW4L_GAMES=
   IW4L_SKATE=off
   IW4L_MASTER_ADDR=<host's public IP>:4433
   IW4L_MASTER_SERVER_NAME=iw4l-prod
   IW4L_MASTER_CA_CERT=iw4l-ca.pem
   IW4L_MASTER_PASSWORD=<room password>
   ```
   `IW4L_SKATE=off` is needed because the kit has no Skate 3 converter, and the first-run check fails without it when skating was never set up.
4. **Set their own player name** in the game's settings. It decides their UUID and player file in the host's world.
5. **Start the game once to the menu** and leave it there until `iw4l-artifacts\minecraft-26.3\.complete` exists (the log says "Minecraft files ready", `minecraft_setup.rs:98`). The first start converts their MW2 files and, from startup (`assets/src/plugin.rs:13`), downloads the Minecraft 26.3 files from Mojang (`crates/assets/src/minecraft_setup.rs`). It needs internet access and a few minutes. A first start straight into `minecraft:overworld` waits only 90 s for Mojang's files and then fails the load (`assets/src/minecraft_map.rs:96-103`).

### 18.3 Joining

- Run `Join Minecraft World.bat`. It checks for `iw4l-artifacts\minecraft-26.3\.complete` first (`minecraft_setup.rs:77`) and, if it is missing, says to start the game once to the menu. Then it asks for the code and starts the game with `IW4L_MASTER_JOIN=<code>` and `map minecraft:overworld`.
- Or start normally, open the server browser and pick the host's room.
- Or type `ui_join_lobby_id <code>` in the console.
- Joining in the middle of a match is allowed while the room is open.

## 19. Implementation plan

**How to run it:**
- Each milestone is one `track-runner` run on branch `track-e8`, with worktree `C:\Dev\mc-wt\track-e8` and target `D:\iw4l-wt\track-e8`. Each run continues from the previous one's commits.
- The owner tests after each milestone with a `--profile play` build, output `D:\iw4l-wt\track-e8\play\iw4l.exe`. **The last step of each milestone copies it to `C:\Games\Mashup\2010-Rust-Rewrite-Mashup\iw4l-mp-m<N>.exe`**, which the script uses by default. `iw4l-survival.exe` is never overwritten by this track. Steps and kit lines go to `docs/round8/track-e8-tests.md`.
- Each group is one commit with a plain-English subject. Where a group moves code, the move is its own group (marked "pure move"): host only, no behaviour change, with the single-player tests and `check -p launcher` as the gate. Friend support follows in the next group.
- Every group follows the fork rules: no comments, no rustfmt on existing files, new files `rustfmt --check` clean, and cargo only through `scripts\cargo-slot.ps1`. It runs `check -p` and `test -p` on the crates it changed, and `check -p launcher` before a commit that touches game code.

**Merge order with other tracks.** This track touches `minecraft_world.rs` heavily, so merge it **after** tracks A-D, rebasing per milestone.
- Track C's c4 (wings and glide pose) is needed for the elytra in E15.
- Track A's a2 fixes the eye of ender; M6 does not depend on it (travel is by command).
- Track B's sleeping should count every non-spectator player in the level (vanilla `SleepStatus`). If B merges first, E20 adds the percentage rule.

### M1: a friend joins, sees the world and chats

**E1. "Two game windows can play together through a local relay"**
- Files: `scripts/mp-local.ps1` (new: role folders, hard link, junctions, cargo-slot hold, muted host, console routing, `-Fresh`, `-CertsOnly`, `-FriendKit`, `-Bats` and the two `.bat` writers); this doc's sections 17 and 18 if anything changed.
- Tests: none in cargo. The agent runs `-CertsOnly` and checks the certificate with `openssl verify`, and runs `-FriendKit` and checks the zip's file list against section 18.2.
- Owner checks, possible before any game code:
  - `-Map iw4:mp_rust`: two MW2 soldiers see each other;
  - a real friend on their own PC joins `iw4:mp_rust` over the internet with section 18 (firewall, port forward, certificate names and password).

**E2. "minecraft_net carries the multiplayer messages"**
- Files: `crates/minecraft_net/Cargo.toml` (postcard, minecraft_commands), `src/lib.rs`, `src/messages.rs` and `src/codec.rs` (new), `Cargo.lock`.
- Adding a dependency needs one cargo run without `--locked`, to update `Cargo.lock`. If the registry cannot be reached, fall back to `serde_json` for the codec and note it in the log.
- Tests:
  - every message and both frame types round-trip;
  - oversize input and trailing bytes are refused;
  - `PROTOCOL` is checked;
  - the `"MC"` batch header round-trips.

**E3. "The MW2 relay carries Minecraft messages between the host and friends"**
- Files:
  - `crates/net/src/transport/protocol.rs` (server tags 15 and 16, client tags 4 and 5);
  - `transport/udp_session.rs` (demux in `UdpAuthorityHub::ingress` with the `peers[conn]==from` check and **no** `Committed` wait for Minecraft packets, and in `UdpClientLink::recv_ticks`; sending; `control_outbound_len()`);
  - `transport/master.rs` (`uni_ingress` hands `"MC"` streams to their own reader and queue; the host's `"MC"` egress task that reports failures instead of faulting; the bulk slot count);
  - `src/minecraft_wire.rs` (new: the `MinecraftWire` resource with inbox, outboxes, `connected`, `joined` and `left`, the frame numbers, acks and credit window, the byte budget, the mailbox reserve, and the batch resend timer);
  - `src/plugin.rs`;
  - `src/lib.rs` (`PROTOCOL_VERSION` 95);
  - `crates/iw4l-master/src/main.rs` (a full target queue closes the non-host side and keeps the host's loop).
- Tests:
  - the packet codec for the new tags;
  - a decode failure in a `Minecraft` packet does not end the link;
  - a `Minecraft` packet from an enrolled but uncommitted peer is delivered;
  - the window: a friend that never acknowledges gets at most `window` frames, and the mailbox never goes past 48 Minecraft entries;
  - the byte budget spreads a 1 MiB backlog over seconds;
  - an `"MC"` write that fails is reported, and the link stays up;
  - a batch with no acknowledgement is resent with the same `seq` after 3 s;
  - a broken `"MC"` stream on the friend is dropped without a fault, and a `"BS"` stream still reaches bootstrap;
  - master: a relay frame to a full queue closes the friend and keeps the host's connection.

**E4. "MW2 works at the heights of the other dimensions"** (a spike, before anything depends on bands)
- Files: `crates/sim` tests only.
- Tests (in a `sim` test with a voxel world placed at map z −45,000 and +45,000):
  - pmove walks, jumps and lands;
  - a bullet trace hits a block and a player;
  - a grenade blast hurts a player;
  - the death obituary and a respawn work;
  - a player spawned there stays there for 2 minutes of sim time (nothing kills, clamps or culls them).
- If anything fails, the agent stops the milestone and writes the finding in the track log; the bands in section 8.2 are then made smaller before E13a.

**E5. "A client terrain stream builds the world from chunks the host sends"**
- Files:
  - `terrain.rs` (`TerrainStream::remote`, `apply`);
  - `crates/minecraft_terrain/src/chunk_wire.rs` (new: encode and decode a batch, using `write_chunk`/`read_chunk` and `StoredChunk`);
  - `scene.rs` (change log and `take_changed`);
  - `minecraft_survival/src/level.rs` (each tick, the drained positions also go to `record_edits`).
- Tests:
  - an encoded and decoded chunk has the same blocks, biomes and block entities;
  - the change log reports every `set` once per tick;
  - a chunk sent after edits from each source (placement, server changes, explosion, fluids, command) matches the scene;
  - a remote stream meshes an applied chunk.

**E6. "The host streams its world to each friend"**
- Files:
  - `crates/minecraft_survival/src/session.rs` (new: `Session`, `ChunkSender` with the batch window, markers, resend and level, block-update fan-out by level, the `Hello` checks, `Login`, `Rejected` with the MW2 retire);
  - `server.rs` (`Server::leave` with the level-thread settle, menu close with overflow drops, `recently_left`, the writer queue; join for a remote profile from the MW2 name);
  - `render_anim/src/minecraft_host.rs` (new: the systems between `MinecraftWire` and the sessions, and the chunk-send worker);
  - `minecraft_world.rs` (`place_player` and the first teleport for friends from `AuthorityWorld`; the interim travel gate and the join placement of section 5.3).
- Tests (session):
  - pacing with 1 and then 2 batches in flight, and the resend;
  - a forget after a load keeps the order behind the marker;
  - a block update only for sent chunks in the friend's level;
  - leave applies late picks, drops overflow, saves and removes, and the main thread never joins the writer;
  - a quick rejoin reads `recently_left`;
  - a second `Hello` with the same name is refused and the MW2 client retired;
  - travel is refused while a friend is online.

**E7a. "The world's presentation is built by functions both games can call"** (pure move)
- Files: `minecraft_world.rs` (the presentation half of `update`, `:1655-1700, 2160-2260`, into functions: terrain frame, environment and sky, entity meshes, hand and held item, HUD).
- Gate: the single-player tests and `check -p launcher`.

**E7b. "RemoteWorld applies what the host sends"**
- Files: `crates/minecraft_survival/src/remote.rs` (new: `RemoteWorld`, `apply`, the marker pause, `epoch`).
- Tests (`RemoteWorld`):
  - a block update behind a missing batch waits, and applies after it;
  - a `Dimension` behind a missing batch does not let later mob adds reach the old level;
  - a batch before `Login` waits;
  - a resent batch is ignored;
  - a state frame older than the last, or of another epoch, is ignored.
- **Loopback test** in `minecraft_survival`: a real `Server` on a temp world (as in the `server.rs` tests) with a `Session`, frames encoded and decoded, feeding a `RemoteWorld`. It checks that the friend's blocks at spawn equal the host's, that a broken block reaches the friend in order, and that leaving saves the player file.

**E7c. "A friend's game shows the host's world"**
- Files: `render_anim/src/minecraft_remote.rs` (new: the client system, loading, voxel with the prediction brushes, presentation through E7a's functions); `minecraft_world.rs` (no `load()` in the Client role).
- Tests: none new beyond E7b; `check -p launcher`.

**E8. "Chat and commands work for every player"**
- Files: `minecraft_survival/src/commands.rs` (`pump` routing to every recipient), `minecraft_world.rs` (commands for every player with lines waiting, presence of all, `chat.join` and `chat.leave`), `session.rs`, `remote.rs`, `crates/hud/src/minecraft_chat.rs` (the friend's input goes to the wire).
- Tests:
  - `Recipient::All` reaches both;
  - `/tell` reaches one;
  - an op-only command is refused for a level-0 friend and allowed for the host without `ops.json`;
  - join and leave lines; a left player's name no longer resolves.
- **Owner test M1** (`-World mp-m1 -Fresh`):
  1. Run the script. In the friend window: the world loads at spawn, next to the host.
  2. Friend window: walk, jump and fall on Minecraft terrain. Alt-Tab to the host window: the friend's soldier is seen moving. Back in the friend window: the host's soldier is seen standing.
  3. Host window: break a block by shooting it. Friend window: it is gone. Friend window: shoot a block; host window: it is gone too.
  4. Friend window: type "hello" in chat. Host window: it shows. Host window: run `/op Friend`; friend window: run `/time set day`. Both see the day.
  5. Stay within about 8 chunks of the host in M1 (chunks only load around the host until M2).
  6. Quit the friend window, then start it again with `scripts\mp-local.ps1 -FriendOnly`: the friend is back in the same place.

### M2: the friend plays survival

**E9. "The chunk map loads chunks around every player"**
- Files:
  - `third_party/minecraftoss/world/src/chunk_map.rs` (`views`, the union, the distance order);
  - `crates/minecraft_terrain/src/terrain.rs` (`server_tick(players, local)` and the host-view filter for `Sections`);
  - `crates/minecraft_survival/src/level.rs` (`stream_tick` takes positions);
  - the call sites in `render_anim`, passing every player in the level.
- Tests (chunk map):
  - two viewers 100 chunks apart load both squares;
  - removing one forgets only its square;
  - the order is nearest first.
- Test (terrain): a chunk outside the host's view is in the scene but not in `Sections`.

**E10a. "The per-player step runs for the host by client number"** (pure move)
- Files: `render_anim/src/minecraft_players.rs` (new: the step from `minecraft_world.rs:610-1527` and `1803-1896`, keyed by `ClientId`, reading MW2 state from `AuthorityWorld`); `minecraft_world.rs`.
- Includes the voxel queue drain into per-client maps and per-client pending damage (section 6.1 step 3), and the early returns turned into "skip this player's step".
- Gate: the single-player tests, `check -p launcher`, and the owner's single-player steps from `docs/round7/release-2-tests.md` in M2's test.

**E10b. "Every player has their own hearts, hunger, XP and respawn"**
- Files:
  - `minecraft_players.rs` (the step for every player with a live MW2 client);
  - `minecraft_portal.rs`, `minecraft_end.rs`, `minecraft_elytra.rs`, `minecraft_mining.rs` and the respawn calls (per-player effects returned instead of writing the host's UI and sounds);
  - `minecraft_survival/src/views.rs` (new: `PlayerView`);
  - `minecraft_inventory.rs` (`sync_weapons` and the follow-MW2 rule per player; section 10.3);
  - `session.rs` (send on change);
  - `remote.rs` and `minecraft_remote.rs` (HUD from `PlayerView`, the weapon from the selected slot);
  - the host's HUD built from the same view.
- Tests:
  - `PlayerView` changes are sent once;
  - two players lose hunger and health independently;
  - each respawns at their own bed;
  - a death queued for client 1 is not lost when client 0 steps first;
  - one player's pending damage does not skip the other's settle.

**E11. "The level thread serves every player, not player 0"**
- Files: `crates/minecraft_terrain/src/server.rs` (`TickInput`, `Output`, `player_hurts` with the player, pickup and orbs per player, commands with a player, no `merchant_view(0)`, merchant and XP delay per player, `PlayerLeft`, simulation centres).
- Tests:
  - two players pick up their own items and orbs;
  - a zombie targets the nearer player;
  - a hurt on player 1 makes player 1's wolf avenge it;
  - XP delay per player;
  - random ticks run around both players;
  - `PlayerLeft` flushes that player's picks.

**E12. "Mobs, items and XP treat every player alike"**
- Files:
  - `crates/minecraft_survival/src/entities.rs` (no `PLAYER`, `PlayerOutputs` per player, `set_players`, hits keyed, `leave(id)` with the settle, outputs for unknown ids);
  - `minecraft_players.rs` (apply hits per player);
  - the `MobBlast` credit.
- Tests:
  - a friend's kill gives them the XP;
  - a skeleton's arrow hurts the friend, not the host;
  - leaving clears one player's queues, and a late pick is re-spawned in the world;
  - the host's own drops still reach the level thread.

**E13a. "Voxel events say which level and which player they belong to"**
- Files: `crates/sim/src/voxel.rs` and `combat.rs` (`level: u8` on `Shot`, `Explosion`, `MobShot`, `MobBlast` and `Ray`, `attacker` on `Shot` and `MobShot`); the readers in `render_anim`.
- Tests: a voxel shot carries its attacker; every event carries level 0.

**E13b. "The action rules live in minecraft_survival"** (pure move)
- Files: `minecraft_survival/src/actions.rs` (new: the rules and reach checks, from `minecraft_world.rs:1031-1527`), `render_anim/src/minecraft_input.rs` (new: Bevy input into `HandEvent` and `HandState`, the target raycast), `minecraft_players.rs` (the host's actions through them).
- Gate: the single-player tests and `check -p launcher`.
- Tests: reach is refused beyond 5.5 blocks; a hit point off the face is refused; placing, using a door and eating work on a test scene.

**E13c. "Friends break, place and use blocks and items"**
- Files: `session.rs` (hand events queued per player and consumed once per server tick; `HandState`), `minecraft_remote.rs` (sending), `render_anim/src/minecraft_mining.rs` (progress per level; cracks and break events to friends in the level).
- Tests:
  - a press and a release in one host frame both act;
  - a press is used once, however many host frames pass;
  - a held use without a fresh `HandState` for 500 ms stops.

**E14. "Each player has their own inventory screens, and chests are shared"**
- Files:
  - `minecraft_survival/src/menus.rs` (new: `Menu` with dimension and station inputs, `click`, `rename`, `view`, `changes`, close with overflow drops);
  - `player.rs` (`menu`, station inputs in the snapshot);
  - `render_anim/src/minecraft_inventory.rs` (draws from `MenuView`, sends `Click` and `RenameItem`);
  - the death path (close before the drops) and the travel path (close);
  - `session.rs` and `remote.rs`.
- Tests:
  - two players in one chest see each other's click;
  - a stale `state` resends everything;
  - a click on a closed menu, or from another dimension, is ignored;
  - closing with a full inventory drops the overflow;
  - an anvil rename applies;
  - dying with items in the grid drops them.

**E15. "Players see each other's armour, held items and elytra"**
- Depends on Track C's c4 for the wings (section 12).
- Files:
  - `minecraft_players.rs` (gear for every player);
  - `session.rs` (`Equipment` on change);
  - `minecraft_remote.rs`;
  - `crates/render_anim/src/anim/minecraft_gear.rs` (corpse rule per client; c4's wings for every client).
- Tests:
  - gear is sent on change only;
  - the corpse rule applies per client.
- **Owner test M2** (`-World mp-m2 -Fresh`):
  1. Single-player check first: the steps of `docs/round7/release-2-tests.md` that the owner picks, in `iw4l-mp-m2.exe` alone.
  2. Friend window: mine a block, pick up the drop, craft planks into a crafting table, smelt raw iron, eat beef. The friend gets the drops and XP, not the host.
  3. Friend window: open a chest and leave it open. Alt-Tab to the host: open the same chest and move a stack. Alt-Tab back: the friend's chest shows the move.
  4. Each window: the hotbar and XP bar are its own.
  5. Friend window: put on the iron armour and hold the sword. Host window: they show on the friend's soldier. Then the reverse.
  6. Friend window: wear the elytra. Host window: it shows; with c4 merged, the glide pose shows while the friend glides.
  7. Host window: `/op Friend`. Each player places their bed apart from the other's and sleeps in it once. Then each types `/kill` in their own window and respawns at their own bed.

### M3: one shared world

**E16. "Friends see the mobs, items and projectiles the host sees"**
- Files:
  - `entities.rs` (`EntityView` split, server items only);
  - `minecraft_terrain/src/server.rs` (serde on the snapshot types; the tracker per player with `mob_adds`, `mob_removes` and `mob_looks`);
  - `server_mobs.rs` (`spawn_saved` keeps the id; `MobLook`; extra render flags on `ServerMob`);
  - `session.rs` (`MobAdd`, `MobLook`, `MobRemove`, `StateFrame` with every tracked mob);
  - `remote.rs` and `minecraft_remote.rs`.
- Tests:
  - the per-kind range filter;
  - add, then moves, then remove;
  - an add tag keeps equipment and profession;
  - a look change is sent once, and a health change is not a look change;
  - moves for an id with no add are dropped;
  - the host's mirror skips mobs beyond its own range.
- The loopback test gains a mob add and its moves.

**E17. "Friends hear and see what happens around them"**
- Files: `session.rs` (sounds, level events, explosions, cracks, time and weather within vanilla radii, same level only), `minecraft_remote.rs`, `render_anim/src/minecraft_sounds.rs`.
- The rule: sounds and particles that come from server outputs or actions are sent. Those that a client's mirror makes itself (mob sounds from `ServerMob` counters, item pickups from the snapshot) are not, so nothing plays twice.
- Tests:
  - the radius filter, 16 × volume for sounds and 64 for level events, and never across levels;
  - no sound is sent for a mirror-made event.
- **Owner test M3** (`-World mp-m3 -Fresh`):
  1. Host window: `/time set night`. Friend window: spawn a zombie with the egg nearby and walk away; it chases the friend. Alt-Tab to the host: the zombie is seen chasing.
  2. Friend window: spawn cows with the egg and kill them. Alt-Tab to the host: the drops are on the ground. Friend window: pick them up; the friend gets the XP.
  3. Host window: place TNT and light it with flint and steel. Friend window: it blows up there too, and the friend hears it.
  4. Host window: `/weather rain`, then `/time set night`. Friend window: rain and night arrive.
  5. Friend window: shoot arrows, throw snowballs and a pearl. Host window: they fly.

### M4: fighting

**E18. "Guns hurt other players unless pvp is off, and deaths are told to everyone"**
- Files:
  - `crates/session/src/match_apply.rs` (a Minecraft load always runs `dm`);
  - `crates/sim/src/damage.rs` (`DamageRefusal::PvpDisabled`), `sim/src/voxel.rs` (`set_pvp`);
  - `minecraft_survival/src/death.rs` (`DamageCause::Player`), `survival.rs` (`death_line` keeps the key; a death marked as told), `commands.rs` (`death_message` translatable);
  - `minecraft_players.rs` (the PvP line from `SimEvent::Died`; death drops and kill card per player).
- Tests:
  - a Minecraft load picks `dm` whatever was chosen, and zeroes its time and score limits;
  - the sim refuses player damage with pvp off and allows it to self and from the world;
  - with pvp on, the `pvp` rule is the only gate between two players;
  - the death line keys `death.attack.player` and `.item`, and a PvP death gives exactly one line;
  - the line is broadcast only with `show_death_messages`.
- **Owner test M4** (`-World mp-m4 -Fresh`):
  1. Host window: shoot the friend until they die. Both windows: the MW2 kill feed shows it, and the chat shows "Friend was slain by <host> using <gun>" once.
  2. Host window: `/gamerule pvp false`, then shoot the friend: nothing happens.
  3. Host window: kill the friend 30 times with `pvp` on (or `/gamerule pvp true` and repeat); the world stays open (no intermission).
  4. Friend window: spawn a creeper next to the friend and let it explode. Both chats show the death line.

### M5: different dimensions

**E19. "Each dimension has its own collision, so players can be in different dimensions"**
- Files:
  - `crates/sim/src/voxel.rs` (levels, bands, walls, the void kill at the wall, keyed mob boxes);
  - `minecraft_survival/src/bands.rs` (new: fixed origins with the clamped overworld y and the End at −1140, `check()`);
  - `minecraft_world.rs` (feeds every open level, no wipe on switch);
  - `minecraft_remote.rs` (one band).
- Tests:
  - `bands::check()` passes for a spawn at y −60, 64 and 320;
  - a trace in one band never hits another band's blocks;
  - the wall stops a trace past `min_y − 128`;
  - a player who reaches the bottom wall dies out of the world, with no fall damage;
  - `band_of` covers the three bands;
  - events carry their level.

**E20. "Every dimension with a player in it keeps running"**
- Files:
  - `minecraft_world.rs` / `minecraft_players.rs` (`follow_levels` per player, every occupied level streamed and ticked; the interim travel gate and join placement removed);
  - `server.rs` (the one server clock);
  - portal and End travel per player;
  - `session.rs` (`Dimension` with the next `epoch`; tracking and menu cleared), `remote.rs` and `minecraft_remote.rs` (level switch with the loading screen);
  - sleeping for every player if Track B has merged.
- Tests:
  - two levels each tick once per server tick;
  - the clocks advance once;
  - a remote player's portal arrival sends `Dimension` with the next `epoch`, and clears their `sent` set;
  - travel by one player leaves the other's collision untouched.

**E21. "Players in another dimension are not drawn"**
- Files: `crates/render_anim/src/occupancy/remote_body.rs`, `anim/minecraft_gear.rs`, `crates/hud/src/overhead_names.rs`, `minecraft_world.rs` (`obstacles`).
- Tests: the band gate is pure.
- **Owner test M5** (`-World mp-m5 -Fresh`):
  1. Host window: `/op Friend`. Friend window: `/execute in minecraft:the_nether run tp @s 0 70 0`.
  2. Friend window: the Nether loads and the friend walks. Alt-Tab to the host: the host keeps walking in the overworld, and the friend is not seen anywhere.
  3. Friend window: mobs move in the Nether. Host window: mobs move in the overworld.
  4. Host window: fire at where the friend was; nothing reaches them.
  5. Friend window: build a nether portal with the obsidian and flint and steel, and walk through. Both windows: the friend is back and seen again.

### M6: the End and finishing

**E22. "The dragon fight and the credits work for every player"**
- Files: `render_anim/src/minecraft_end.rs` (boss bar per player, credits per player), `session.rs` (`BossBar`, `Credits`, `CreditsDone`), `remote.rs` and `minecraft_remote.rs`.
- Tests: the boss bar reaches players in range only; credits are shown once per player; `CreditsDone` releases that player only; leaving during the credits releases the hold.

**E23. "The host shows a join code"**
- Files: `minecraft_host.rs` (the chat line with the room code when hosting, read from the master bridge state).
- Tests: the code line format.

**E24. "The multiplayer state, limits and owner steps are written down"**
- Files:
  - `docs/MINECRAFT-SURVIVAL.md` ("Multiplayer today" and limitations);
  - `CLAUDE.md` (roadmap 1.4 status);
  - this doc (anything built differently);
  - `docs/round8/track-e8-tests.md` (all milestone steps and one kit line).
- **Owner test M6** (`-World mp-m6 -Fresh`):
  1. Host window: `/op Friend`, then `/execute in minecraft:the_end run tp @a 0 70 0`.
  2. Both fight the dragon, alternating windows. Both see the boss bar, crystals and breath.
  3. After the kill, each walks into the exit portal in turn and sees the credits; the other window keeps running meanwhile.
  4. A friend on another PC joins the Minecraft world over the internet with section 18.

### Kit for the multiplayer tests (27 stacks)

Both players get it on their first join to a fresh world:

```
IW4L_MINECRAFT_KIT=diamond_sword,diamond_pickaxe,shield,bow,arrow*64,cooked_beef*32,snowball*16,oak_planks*64,crafting_table,chest*2,furnace,coal*16,raw_iron*16,iron_helmet,iron_chestplate,iron_leggings,iron_boots,elytra,flint_and_steel,obsidian*14,ender_pearl*16,eye_of_ender*12,zombie_spawn_egg*2,cow_spawn_egg*2,water_bucket,tnt*4,white_bed
```

M4 also needs a creeper spawn egg; the friend gets it with `/give Friend creeper_spawn_egg` after `/op Friend`.

## 20. Deferred to later rounds

- **Block prediction:** vanilla's sequence-acked dig and place prediction (`ClientboundBlockChangedAckPacket`). Blocks appear after one round trip in this round.
- **Entity traffic:** vanilla's per-kind update intervals and delta-only entity traffic. This round sends every tracked mob's move every tick, and the level entities as a filtered full snapshot.
- **Chunk pacing:** vanilla's adaptive `chunks_per_tick` quota. The ack window paces the stream.
- **PvP without guns:**
  - Minecraft melee against players: swords and axes on another player, with armour and shields;
  - MW2 damage through Minecraft armour.
- **Chat:** command suggestions (`CommandHints`) for friends, signed chat, narration.
- **Languages:** a language setting; every game shows English this round.
- **Creative inventory for friends** (`ServerboundSetCreativeModeSlotPacket`).
- **Trading screens** for anyone (not wired in single player either).
- **Threads:** a shared thread budget for the chunk generators of several open levels. Each level keeps its own pool, which matters only when friends explore different dimensions at once.
- **Players:**
  - bots in a Minecraft multiplayer world;
  - a vanilla player mesh instead of the MW2 soldier (the `bot_model` retarget path, `remote_body.rs:1236`);
  - skins;
  - more than 4 players (nothing stops it, but it is untested).
- **Sessions:**
  - host migration;
  - the world surviving the host leaving;
  - a dedicated headless host;
  - automatic reconnect.
- **Joining:** Discord join (skycraft's precedent), NAT traversal without a forwarded port.
- **Missing visuals and sounds:**
  - mob flags that neither the NBT, `MobLook` nor the extra `ServerMob` fields carry (each one found goes into the limitations list);
  - sounds that only the host's local presentation makes and that E17 did not classify.

## 21. Risks and what to check first

1. **MW2 at large map z.** E4 checks it in `sim` before anything depends on it: pmove, traces, blasts, obituaries and spawns at z ±45,000. If something fails, shrink the bands before E13a. The void wall leaves room for bands about half as far apart.
2. **The relay's caps.** Section 3.1 lists every queue that is fatal when full. E3's window, budget, mailbox and bulk tests must pass before E6 sends real chunks, and the master patch is a safety net, not the plan.
3. **Admission order.** The friend's MW2 admission waits on `ui.loading_world`, and the host drops relay packets from uncommitted peers. E3 lets Minecraft packets through before `Committed`, and the friend resends `Hello`. E7c checks it end to end.
4. **Size of the `update()` refactor.** `minecraft_world.rs` is 2,936 lines, and the per-player code is about 1,000 of them, with host UI and sound calls inside the rules (section 6.1). The pure moves (E7a, E10a, E13b) come before the behaviour changes, each gated by the single-player tests. The owner's single-player steps run in M2's test.
5. **Merges with tracks A-D**, which edit the same adapter. Rebase per milestone. Never merge two tracks into `minecraft-survival` at once.
6. **Registry agreement.** Block state ids must match on both games. The fingerprint in `Hello` catches a mismatch. Both games read the same pinned 26.3 jar (`minecraft_setup.rs`, by SHA-1), so it should never differ.
7. **Match end.** E18 pins that a Minecraft world runs `dm` with no time or score limit, so PvP kills never end it; M4's owner test checks it.
8. **Memory.** Two games, a relay and a host generating chunks for two players use a lot of RAM on a 32 GB PC that often has about 10 GB free (`docs/PROGRESS.md`, Phase 1). The script holds both cargo slots while the windows run, and E1 records the peak.
9. **The elytra needs c4.** E15 cannot draw wings without Track C's c4; the owner step waits for it.

## 22. Changes after review

**Accepted and fixed:**
- Full queues: the master closes both connections when a target queue fills (`main.rs:694-697`), the host's shared mailbox and channel are fatal when full, and a stalled host stops reading. Replaced the per-tick frame cap with a per-friend credit window, a byte budget, a mailbox reserve, a friend send limit, `HandState` on the datagram lane, a writer queue that never blocks the main thread, and a master patch (sections 3.1, 3.2, 14).
- Bulk failures: a refused or failed `"MC"` stream ended the host's egress or went unnoticed, and a slow one faulted the friend. Added an own egress, an own friend-side reader, 2 bulk slots, ack-timeout resend with duplicate drop, a 30 s drop and 128 KiB batches (section 3.2).
- Admission: Minecraft packets pass before `Committed`, and `Hello` is resent (section 3.2, 5.1).
- Ordering: one total order via batch markers in the reliable stream; `Login` first; `epoch` on state frames; tracking cleared on `Dimension` (sections 7.2, 9.2, 11.2).
- Mob tags built on the level thread, and a small `MobLook` instead of hashing the tag (section 9.2).
- Travel refused while a friend is online until E20, a joining friend placed in the host's dimension, and early returns that skip only one player (sections 5.3, 6.1).
- The real size of the per-player move: MW2 state per client and the host UI and sound writes inside the rules (section 6.1).
- Pickup, XP, mob targeting and level-thread commands per player moved into M2 (E11, E12); `player_hurts` keeps its source and type; commands carry the player; `merchant_view(0)` removed (section 6.3).
- Band numbers: the End origin moves to −1140 (−1200 would make it worse), the overworld origin's y is clamped, `bands::check()`, and the void kill at the wall (section 8.2).
- The host keeps `CommandServer`'s owner rights; leave calls `chat.leave` (sections 5.2, 6.5).
- Friend install: an explicit file list without Mojang, MW2 or private files; the password; start once to the menu; the `.bat` checks `.complete` (section 18).
- References corrected: `ClientMobs::receive` at `client_mobs.rs:896` (and every tracked mob in each frame), `server_loop` at `:2673`, the control decode teardown at `udp_session.rs:1043`, the master queue-full branch at `main.rs:694-697`.
- Dimension-blind fan-out of block updates and radius messages (section 7.3).
- Hand input as sequence-numbered events used once, plus held state (sections 4, 10.1).
- Quick rejoin reads `recently_left` (section 14).
- Level-thread outputs in flight at leave are settled with `PlayerLeft` (section 5.2).
- Global voxel queues drained once into per-client maps; pending damage per client (section 6.1).
- The PvP death line built from `SimEvent::Died` (section 13.2).
- `dm` forced for Minecraft worlds (section 13.4).
- The hotbar and MW2 gun link per player (section 10.3).
- `CreditsDone` added; `Respawn` removed (sections 4, 9.4, 11.3).
- Menus tied to a dimension, closed on travel and death, overflow dropped, station inputs saved, anvil `RenameItem` (section 10.2).
- The drop staging stays on the server side (sections 6.4, 9.1).
- The chunk map follows the scene's change log (section 7.3).
- Client numbers are `u32`, and per-id maps are cleared at leave (sections 4, 6.3).
- A refused friend's MW2 client is retired; the name comes from the MW2 connection (sections 5.1, 16).
- The crash window after a friend leaves is recorded as vanilla parity (section 14).
- Plan order: chat and commands in M1 (E8); the chunk-map union at the start of M2; the band spike early (E4); the voxel event reshape once (E13a); pure moves split from behaviour (E7a-c, E10a-b, E13a-c); a loopback test (E7b, extended in E16).
- Owner steps written for one person with alternating windows, a muted host, console routing, and travel by command; per-milestone worlds and `-Fresh`; kit swaps for TNT and snowballs; the milestone exe copied to `iw4l-mp-m<N>.exe`; the internet check in M1; the cargo slots held while testing; per-role folders that work with the launcher's change into the exe's folder.
- No `Pose` message: `pm_flags` already reaches every client; E15 depends on c4 for the wings.
- Deferred: friends' creative inventory, the `chunks_per_tick` quota, `Hello.language` and per-language lines.

**Rejected:**
- "PvP kills can end the world through the score limit": a Minecraft load already sets `scr_<gametype>_scorelimit` to 0 (`match_apply.rs:1049-1056`), and the sim's `score_limit` only feeds the snapshot meta (`world.rs:3102`). E18 keeps a test that pins it, and M4 has a step for it.
- "Put Minecraft deaths in the MW2 kill feed": the owner asked in round 7 for the opposite (`docs/round7/a2-chat.md`). Recorded in section 1 for the owner to overrule.
- "Move the End origin to −1200 or lower": that pushes the top wall further past the band. Fixed with −1140 instead.
- "Send `MobData` on known changes instead of hashing": the cost came from hashing the whole tag. Comparing a small `MobLook` every 10 ticks is cheap and needs no hooks.
