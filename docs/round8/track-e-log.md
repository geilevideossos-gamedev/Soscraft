# Track E log: multiplayer, first playable

Branch `track-e`, worktree `C:\Dev\mc-wt\track-e`, target `D:\iw4l-wt\track-e`. The design and plan are `docs/MULTIPLAYER.md` (section 19). The design names the branch `track-e8`; this run uses `track-e`.

A commit can't name its own hash, so each entry is written with its commit and its hash is filled in by the next commit; the last entry's hash is in `git log` under its subject.

## Milestone M1: a friend joins, sees the world and chats

### E1. Two game windows can play together through a local relay

Commit: 42262ec2

- **Changed:** new `scripts/mp-local.ps1` (section 17 of the design). It makes the relay's certificates once with Git for Windows' OpenSSL (same keys and names as `cargo xtask certs`), takes both cargo slots while the windows run, starts `iw4l-master.exe serve --bind 127.0.0.1:4433` (or uses one that is already running), prepares `iw4l-artifacts\mp-local\host` and `...\friend` (a hard link to the exe, junctions to `cache` and `minecraft-26.3`, a copy of `profile\`, their own `.env` and `settings.cfg`), starts the host with `IW4L_MASTER_HOST_NAME`, waits for its room in `master.status`, starts the friend with `IW4L_MASTER_JOIN`, waits for `members>=2`, then routes `h <command>` / `f <command>` / `quit` to the games' consoles until both have closed. It stops the relay it started, releases the slots and logs the peak working sets to `mp-local\memory.log`. Switches: `-Exe`, `-Map`, `-Friend`, `-World`, `-Fresh`, `-HostSound`, `-FriendOnly`, `-CertsOnly`, `-FriendKit` (`-PublicAddress`), `-Bats` (both use `-Map`), `-GameDir`, `-Relay`, `-Port`, `-Resolution`. The script never builds anything. Design sections 17.1, 17.3, 18.1 and 18.2 updated to match.
- **Tests (no cargo tests, as planned):** run against a scratch copy of the game folder (`-GameDir`), so the owner's folder was not touched:
  - `-CertsOnly` made `ca-key.pem`, `iw4l-ca.pem`, `server-key.pem`, `server-cert.pem`; `openssl verify -CAfile iw4l-ca.pem server-cert.pem` printed OK; the certificate's subject is `CN=iw4l-prod` with `subjectAltName=DNS:iw4l-prod, DNS:iw4l-dev`. A second run keeps them.
  - `-FriendKit -PublicAddress 203.0.113.5` wrote a zip holding exactly `iw4l-mp-m1.exe`, `LICENSE`, `NOTICE`, `COPYING-FreeFont.txt`, `OFL-Oxanium.txt`, `README.txt`, `iw4l-ca.pem`, `.env`, `Join Minecraft World.bat`, `How to join.txt` and an empty `iw4l-artifacts/`, all under one top folder: the list in section 18.2 and nothing else (no keys, no Mojang or MW2 files, no saves, settings or logs).
  - `-Bats` wrote both `.bat` files; `-FriendOnly` against a faked host status built the friend folder (hard link, two junctions, `.env` without the world and join lines plus `IW4L_SKATE=off`, `settings.cfg` with `player_name=Friend`, windowed, the chosen resolution).
  - The relay and two real windows were not started by the agent; that is the owner's step below.
- **Decisions:**
  - **Each role folder gets its own `.env`.** The design said none was needed, but the launcher's first-run check reads only the `.env` beside the exe (`launcher/src/first_run.rs:17-29`) and would ask for the MW2 folder and offer the Skate 3 conversion in each role folder, and `load_dotenv` stops at the exe's own `.env`. The copy drops `IW4L_MASTER_HOST_NAME`, `IW4L_MASTER_JOIN` and `IW4L_MINECRAFT_WORLD`, makes relative paths absolute and adds `IW4L_SKATE=off` if skating was never set up. It is rewritten on every run, as is `settings.cfg`, so changes to the main ones carry over.
  - **Games run under a hidden `cmd /c`** that sends their stdout and stderr to `game.out.log` / `game.err.log`, because PowerShell cannot redirect a child's stdout to a file while keeping its stdin as a pipe. The game inherits cmd's stdin pipe, which the console routing writes to.
  - **The host `.bat` starts the relay** (`serve --bind 0.0.0.0:4433`, minimised) when none is running, and sets the master address, server name and CA itself, so the host's `.env` only needs the password. To host from the menu the three lines still go in `.env`.
  - **The friend kit's `.env` has `IW4L_GAMES=` empty and `IW4L_SKATE=off`.** The first start then finds or asks for the friend's MW2 folder; without `IW4L_SKATE=off` the first-run check fails because the kit has no Skate 3 converter. The password is copied from the host's `.env` if set; it is the room password the owner hands out anyway. A `How to join.txt` with section 18's steps is added.
  - **Window size** is 1280x720 on a screen at least 2560 wide, else 960x540 (section 17.2), or `-Resolution`.
- **Test steps** (before any Minecraft code; plain MW2):
  1. Build the relay once: `& scripts\cargo-slot.ps1 build --release --locked -p iw4l-master`, then copy `D:\iw4l-target\release\iw4l-master.exe` to `C:\Games\Mashup\2010-Rust-Rewrite-Mashup\`.
  2. In PowerShell in the repo: `& scripts\mp-local.ps1 -Exe iw4l-survival.exe -Map iw4:mp_rust`. It says it waits for both cargo slots, makes the certificates, starts the relay, then the host window and, after the room line, the friend window.
  3. In the host window pick a class and spawn; Alt-Tab to the friend window, pick a class and spawn. Each sees the other's soldier move (alternate windows). Screenshot both.
  4. In the script's console type `h say hello` and `f say hi`: each line shows in its window's MW2 chat or console.
  5. Close both windows (or type `quit`). The script prints `peak working sets: ... host=... friend=... relay=...`; send that line.
  6. Internet: `& scripts\mp-local.ps1 -FriendKit -Exe iw4l-survival.exe -PublicAddress <your public IP>` and `& scripts\mp-local.ps1 -Bats -Exe iw4l-survival.exe -Map iw4:mp_rust` (the `.bat` files and the kit's join `.bat` start the map given with `-Map`; run `-Bats` and `-FriendKit` again without `-Map` for the Minecraft milestones). Allow `iw4l-master.exe` through the Windows firewall (UDP, private and public), forward UDP 4433 on the router, set `IW4L_MASTER_PASSWORD=<password>` in `.env`, then run `Host Minecraft World.bat`. The join code is the `room=` value in `iw4l-artifacts\master.status` (the host `.bat` writes it there), or the friend picks the room in the server browser. A friend unpacks the zip (made after the password was set, or they type it into its `.env`), follows `How to join.txt` and joins; both soldiers see each other. Screenshot both PCs.
- **Kit:** none (MW2 map only).

### E2. minecraft_net carries the multiplayer messages

Commit: 8fbff5af

- **Changed:** `crates/minecraft_net` gains `postcard` 1.1.3 (`use-std`, from crates.io, so the `serde_json` fallback was not needed) and `minecraft_commands` for the chat types. New `messages.rs`: `PROTOCOL = 1`, `ToServerFrame` / `ToClientFrame` (frame number and cumulative ack), `ToServer`, `ToClient`, `HandState`, `HandEvent`, `Target`, `Click` / `ClickMode`, `Login`, `Rules`, `StateFrame`, `PlayerView`, `MenuView` / `MenuKind` / `MenuExtras`, `GearView` / `GearItem`, `BossBarUpdate`, and the opaque `Nbt`, `Blob` and `ChunkBlob` byte holders, as in section 4 of the design. `check_hello(protocol, registry, host_registry)` and `HelloRefusal::reason()` give the `Rejected` text. New `codec.rs`: `encode(lane, &T)` / `decode(lane, bytes)` over postcard with the lane limits (reliable 4 MiB, bulk 256 KiB, state 256 KiB), refusing oversize input on both sides and trailing bytes; `ChunkBatchBlob` writes and reads the `"MC"` bulk blob (`"MC"`, version 1, seq, count, then x, z, length and bytes per chunk) and `peek_seq` reads a batch's seq without decoding it. Nothing uses them yet; E3 carries them.
- **Tests (`minecraft_net`):** `every_message_to_the_server_round_trips` and `every_message_to_the_client_round_trips` (each variant, checked by an exhaustive match so a new variant fails the test until it is added, then all of them in one frame), `state_lane_messages_round_trip` (`HandState`, `StateFrame`), `oversize_input_is_refused` (encode and decode, per lane), `trailing_and_truncated_bytes_are_refused`, `a_long_count_with_no_data_behind_it_is_refused`, `hello_carries_the_protocol`, `hello_is_checked_for_protocol_then_registry`, `refusals_explain_themselves`, `a_new_player_view_has_every_slot`, `the_chunk_batch_header_round_trips` (exact header bytes), `broken_chunk_batches_are_refused` (magic, version, truncation, trailing bytes, a count that lies), `a_chunk_batch_over_the_bulk_limit_is_refused`.
- **Decisions:**
  - `PlayerView`, `MenuView`, `GearView`, `Rules` and `BossBarUpdate` are plain `minecraft_net` types, not `Blob`s: both games build the HUD and screens from them (section 1, "Host's own view"), and they need nothing from `minecraft_terrain`. Only `ServerMob`, `EntitySnapshot` and `MobLook`, which `minecraft_terrain` owns, travel as `Blob`.
  - Fields the design left open follow vanilla: `ClickMode` is vanilla's `ClickType` (pickup, quick move, swap, clone, throw, quick craft, pickup all); the boss bar update is vanilla's `ClientboundBossEventPacket` operations (add, remove, progress, name, style, flags) with its colours and overlays; abilities are vanilla's `Abilities` (`fly_speed` 0.05, `walk_speed` 0.1); cooldowns carry their group with the remaining and total ticks so the hotbar can draw the overlay; the item in use carries its hand and ticks. `MenuExtras` holds the enchanting seed, costs, clues and levels and the anvil cost and name.
  - Refusal texts use vanilla's keys: `multiplayer.disconnect.outdated_client` when the friend's protocol is older, `outdated_server` when newer, with "the host's build (Minecraft multiplayer protocol 1)" as the argument. A registry mismatch is a literal line telling the friend to delete `iw4l-artifacts\minecraft-26.3` and start again.
  - The `"MC"` blob's integers are big-endian, like vanilla's own network and NBT data.
  - `encode` also refuses a message over its lane's limit, so the host can never send what the friend would refuse.
- **Test steps:** none in game; nothing uses the messages until E3 and E6.
- **Kit:** none.

### E3. The MW2 relay carries Minecraft messages between the host and friends

Commit: e06114b0

- **Changed:**
  - `net::PROTOCOL_VERSION` 94 to 95. New packets in `transport/protocol.rs`: `ClientPacket::Minecraft` (tag 4), `ClientPacket::MinecraftState` (5), `ServerPacket::Minecraft` (15) and `ServerPacket::MinecraftState` (16), each a length-prefixed opaque payload that `net` never decodes.
  - `transport/udp_session.rs`:
    - `UdpAuthorityHub::ingress` hands `Minecraft` packets from the control lane and `MinecraftState` packets from the datagram lane to `take_minecraft()` for any **enrolled** member (the sender's `MemberId` must map to a connection in `peers`), with no `Committed` wait. A reliable packet on the datagram lane or a state packet on the control lane is dropped.
    - `retire_connection` records the client for `take_retired()`. New `send_minecraft(client, lane, bytes)` and `member_of_client`.
    - `UdpClientLink::recv_ticks` hands both kinds to `take_minecraft()`. A `Minecraft` packet that fails to decode is dropped with a warning instead of returning the error, which used to lose the rest of that batch of control packets. New `send_minecraft(lane, bytes)`.
    - `RelayMailbox` gains `control_outbound_len()`, `take_control_outbound_up_to(n)` and the shared `MinecraftBulk` queues.
  - `transport/master.rs`:
    - The bridge moves control packets from the mailbox into its 32-frame channel only up to the channel's free capacity. It used to move all 64 at once and end the bridge when the channel was full. The rest now wait in the mailbox.
    - `uni_ingress` reads each stream's relay header and magic first. An `"MC"` stream goes to its own reader task with its own 8 s deadline and a bounded queue of 16 batches; a broken, slow or overflowing one is dropped with a warning, never a `TransportFault`. A `"BS"` stream is read and faulted exactly as before.
    - The host runs a new `minecraft_egress` task with one task per `"MC"` stream. A failed open, write or finish, a stop by the relay, or its 10 s deadline is reported back as `BulkReport { seq, Err }` and never ends the bridge. `"MC"` streams use stream priority -48, below bootstrap (-32).
  - `src/minecraft_wire.rs` (new), the `MinecraftWire` resource for both roles:
    - Fields and calls: `connected`, `joined`, `left`, `failed`, `bulk_failures`; `send`, `send_state`, `send_batch`, `batch_received`, `take_inbox`, `take_state`, `take_batches`, `bad_message`, `drop_peer`.
    - Reliable frames are `frame u32, ack u32`, then a body that is zstd-compressed when that helps. The body is a slice of a stream of length-prefixed messages, so a message of any size up to 4 MiB crosses in order, split over frames of at most 15 KiB.
    - Host: a window of `min(8, 24 / friends)` unacknowledged frames per friend; 64 KiB/s per friend (`IW4L_MC_RELIABLE_KIB`); Minecraft frames only while the control mailbox holds fewer than 48 entries; "connection too slow" after 20 s without an acknowledgement or over 4 MiB unsent; at most 2 `"MC"` streams in flight across all friends; the same batch sent again 3 s after the egress reported the last copy written or failed, and "Lost the world stream" 30 s after its first send; "Too many broken messages" at 20 broken frames in a minute.
    - Friend: at most one data frame per tick and 4 unacknowledged, and nothing at all is sent until the bridge is `Joined`.
    - Ack-only frames (frame 0) carry acknowledgements when no data goes, outside the window. State frames are newest-wins per tick.
    - Systems: receive in `PreUpdate`, send in `PostUpdate` (frames paced to 20 Hz, state and bulk every frame). A friend the wire drops is retired through `retire_client_with_reason`, so the friend sees the reason, and removed from the `AuthorityWorld`.
  - `crates/iw4l-master/src/main.rs`: when a relay frame finds its target's control queue full, the master closes the friend's connection (the target when the host sent it, the sender when a friend sent it) and keeps the host's loop running. A host frame for a member that is no longer in the room is dropped with a log line instead of closing the host, since a friend who just left could otherwise close the room.
- **Tests:**
  - `net` (`minecraft_wire.rs`):
    - `minecraft_packets_round_trip_with_their_own_tags`
    - `a_broken_minecraft_packet_does_not_end_the_link`
    - `minecraft_from_an_enrolled_but_uncommitted_peer_is_delivered`
    - `a_friend_that_never_acknowledges_gets_only_the_window`: 8 frames for one friend, 6 each for four.
    - `minecraft_frames_leave_room_in_the_mailbox_for_mw2`: with 30 MW2 entries queued and three friends, the mailbox stops at 48, then each friend stops at its window of 8.
    - `the_byte_budget_spreads_a_backlog_over_seconds`: 1 MiB of incompressible messages; at most 64 KiB in the first second; drained after 15-20 s, intact and in order.
    - `messages_of_any_size_cross_both_ways_in_order`: 100 KiB and 300 KiB messages, state frames, the friend's one frame per tick and window of 4.
    - `nothing_is_sent_before_the_link_is_up`
    - `an_unacknowledged_batch_is_sent_again_with_the_same_seq`: 2 streams in flight, no second copy before the egress reports, a resend 3 s after the report with the same seq and blob, a drop at 30 s.
    - `a_failed_minecraft_write_is_reported_and_the_link_stays_up`
    - `a_broken_minecraft_stream_is_dropped_and_bootstrap_still_arrives`
    - `a_silent_or_flooded_friend_is_dropped_as_too_slow`
    - `broken_frames_count_and_a_gap_loses_the_stream`
  - `iw4l-master`: `a_full_queue_closes_the_friend_and_keeps_the_host`.
- **Decisions:**
  - **Minecraft packets carry no `PacketHeader`.** A relay friend learns its `ConnectionId` only from the bootstrap offer, which comes after the map loads, long after `Hello` must go. The host identifies the sender by the relay's `MemberId`, which the master stamps and the friend cannot choose, and accepts it only if that member is enrolled in `peers`. That is the design's `peers[conn]==from` check, seen from the other side. There is no epoch either: a Minecraft session ends with the MW2 client, and `left` reports it.
  - **The frame number and acknowledgement live in the wire, not in `minecraft_net`.** `net` stays free of `minecraft_net`, which pulls in `minecraft_commands` and MinecraftOSS, and treats every message as opaque bytes. So `ToServerFrame`, `ToClientFrame` and `ToServer::Ack` from E2 are not used on the wire: the session hands each encoded `ToClient` or `ToServer` to `MinecraftWire::send`, and the wire frames, numbers and acknowledges it. They stay in `minecraft_net` for now; E6 or E7b may remove them.
  - **A batch is sent again only once the egress has reported its last copy, and 3 s after that report.** The design times the 3 s from the send. On a slow link a 128 KiB stream can still be in the host's write or the master's forward after 3 s, and a second copy would take another of the master's 4 stream slots that MW2 bootstrap offers need. Counting from the report (the master has all the data then) keeps one copy per batch in flight.
  - **Ack-only frames are outside the windows**, so a full window on one side can never stop the other side from acknowledging.
  - **The byte budget always lets the first frame of each second go.** Frames are at most 15 KiB and the budget is 64 KiB, so this only matters if `IW4L_MC_RELIABLE_KIB` is set very low, and the stream still moves then.
  - **A gap in frame numbers drops the friend with "Lost the world stream"** instead of skipping it. The control lane is ordered, so a gap means a message was cut in half.
  - **The friend counts broken frames but never drops the host for them.** Only a gap or an oversize message length sets `failed`.
  - **The master test is a unit test of the decision** (`full_queue_victim`). A real QUIC pair in a test needs a certificate generator, which the workspace does not have.
  - **Finding: in a relay match nothing compares `net::PROTOCOL_VERSION`.** The master only logs the `game_protocol` of its hello, and relay admission compares the map, weapon and class digests, so a protocol 94 build still joins a 95 host for MW2. That is harmless for Minecraft: the host only ever answers a friend that spoke first, and a 94 host ignores the new tags. A 95 friend on a 94 host would wait on the loading screen resending `Hello`, so E7c should show "The host runs a different build" after some seconds without `Login`. The master was not given a new refusal, because that needs a new `ServiceError` and a `master_protocol` change.
  - **The 64 MiB bound on the friend's queues** (section 11.2) belongs to `RemoteWorld` (E7b). The wire's own bound is the 16-batch `"MC"` queue.
- **Test steps:** nothing new shows in game until E6 sends chunks. With the M1 build:
  1. Rebuild the relay once, for the master patch: close any running relay, run `& scripts\cargo-slot.ps1 build --release --locked -p iw4l-master`, then copy `D:\iw4l-target\release\iw4l-master.exe` to `C:\Games\Mashup\2010-Rust-Rewrite-Mashup\`.
  2. Run `& scripts\mp-local.ps1 -Map iw4:mp_rust`. Both soldiers still see and shoot each other, as in E1, now on protocol 95.
- **Kit:** none.

### E4. MW2 works at the heights of the other dimensions

Commit: ec064dc1

- **Changed:** a test-only module, `crates/sim/src/damage/height_bands.rs` (declared in `damage.rs` under `#[cfg(test)]`, so it can reach the blast code's private `radius_player_attempts`). No game code changed. The result is written into `docs/MULTIPLAYER.md` sections 8.2 and 21.
- **How the spike works:** each test builds a real `SimWorld` with a stand-in map (one dummy brush, so the block world is active only for this content, and a world model of a normal MW2 map's size, -3,000..3,000 by -500..1,500), activates the block world with its floor at map z 0 (the control), +45,000 and -45,000, and drives it through `sim::step`, `bullet_trace`, the blast code and the spawn and death natives.
- **Result: nothing failed. The bands stay as designed.**
  - Walking into a wall for 3 s, jumping and landing gives the same path in all three bands to within 0.05 units: the walk stops at the wall's face, the jump peaks between 35 and 40 units, the landing is on the floor with the world as ground.
  - A bullet hits a player standing on the floor and a stone wall (the hit point is on the wall's face, normal -x), and a shot at the sky misses.
  - A frag between a player in the open and a player behind a 3-block wall hurts only the one in the open, by the same amount in every band.
  - A lethal hit kills, and the obituary event names the victim and the attacker.
  - A player standing still for 2 minutes of sim time (2,400 steps) is never moved, hurt or killed; the snapshot carries its exact origin, the area-entity world stays valid, and a blast query at its feet finds it.
- **Findings for later groups:**
  - **The sim spawns every player at map origin.** `voxel::spawn_origin` and `spawn_clear_of` ignore the band, so a respawn lands at map z 0 (the overworld band) or on a free block near the overworld origin. The host already teleports after a spawn; with bands (E13a, E19) the teleport must use the origin of the player's own level. The test pins the current behaviour.
  - **The stand-in map's bounds do not matter.** The area-entity world splits by x and y only, so players far above or below the map are linked and found as usual.
  - A landing after a jump leaves the player 0.125 units above the floor (the trace's pull-back from surfaces), the same in every band; walking keeps the feet on the floor exactly.
- **Tests (`sim`, `damage::height_bands`):** `mw2_walks_jumps_and_lands_at_the_height_of_every_band`, `a_bullet_hits_a_block_and_a_player_at_the_height_of_every_band`, `a_grenade_blast_hurts_a_player_and_a_wall_shields_one_at_the_height_of_every_band`, `a_killed_player_gets_an_obituary_and_respawns_into_its_band`, `a_player_stays_where_it_stands_for_two_minutes_at_the_height_of_every_band`. The whole `sim` suite passes (78 tests).
- **Decisions:**
  - **Steps run as `StepReason::PredictNew`.** An authority step refuses to run without a started GSC program, and the MW2 game scripts are not in the repo. Every system that moves players, traces, links areas and builds the snapshot runs the same for both reasons; only the script hooks differ. Deaths, obituaries and respawns go through the natives the scripts call (`finish_damage`, `kill`, `obituary`, `spawn`).
  - **Snapshot publishing is suppressed during the steps**, and the snapshot is built by hand with `SimWorld::snapshot`. A publishing step drains the process-wide Minecraft damage queue, which another `sim` test fills and reads, and could make that test fail at random.
  - **The tests share one lock and leave the block world active.** The block world is a process-wide static. Each test activates it with its own content's brushes, so `voxel::active_for` stays false for every other test's map; `voxel::deactivate` is not called because it empties queues other tests use.
  - The bands are tested at ±45,000 as the plan says. Every map z between 32,768 and 65,536 has the same f32 step (1/256 unit), so the edges of the bands at ±60,000 behave the same.
- **Test steps:** none in game; this group changes no game code.
- **Kit:** none.

### E5. A client terrain stream builds the world from chunks the host sends

Commit: see `git log` ("A client terrain stream builds the world from chunks the host sends")

- **Changed:**
  - `crates/minecraft_terrain/src/terrain.rs`:
    - `TerrainStream` holds `server: Option<ChunkMap>`. `TerrainStream::remote(registries, seed, dimension, view_distance)` builds a stream with no chunk map: no generator, no world folder, no spawn search. Its block states take the build height from the dimension type (`dimension_bounds`: overworld -64..320, Nether and End 0..256), and the section workers, lighting and meshing are the host's own.
    - `apply(events, scene)` runs the `Load`/`Forget` body that `server_tick` used to run inline; `server_tick` now ticks the chunk map and hands its events to `apply`. A friend's game calls `apply` with the chunks it decodes.
    - The constructors share `assemble`; `BlockStates::for_dimension` sets the sky light and Nether shading for both. `is_remote()` tells them apart.
    - On a remote stream, the host-side calls do nothing (`server_tick`, `record_edits`, `record_block_entities`, `set_tickets`), answer "none" (`loaded_chunk`, `storage`, `queue_save`, `is_settled`, the F3 and F9 lines), or panic with "only the host's terrain stream has a chunk map" (`world_gen`, `load_chunk_now`, `respawn_position`), which only the host's level code calls.
  - `crates/minecraft_terrain/src/chunk_wire.rs` (new): `encode_chunk`/`decode_chunk` write a chunk as vanilla's chunk NBT (`write_chunk`, so blocks, biomes, light, heightmaps and block entities) compressed as a region file stores it, and read it back with `read_chunk`; a chunk sent under another position is refused. `encode_batch`/`decode_batch` put chunks in the `"MC"` bulk blob of E2 (`ChunkBatchBlob`).
  - `third_party/minecraftoss/core/src/anvil.rs` (patched): `StoredChunk::from_zlib` and `zlib()`, so the compressed body crosses the wire as it is.
  - `crates/minecraft_terrain/src/scene.rs`: `HandcraftedScene` keeps a change log, vanilla's `ChunkHolder.blockChanged`: every `set` adds its position once (a set, so a position set twice in a tick is reported once), and `take_changed()` drains it. The authored scene of `HandcraftedScene::new` starts with an empty log; chunks being inserted or removed are not changes.
  - `crates/minecraft_survival/src/level.rs`: `stream_tick` drains the log of its level each tick and passes the positions to `record_edits`, so every `scene.set`, from whatever path, reaches the chunk map that chunks are sent from. The explicit `record_edits` calls stay; repeating one is harmless.
- **Tests:**
  - `minecraft_terrain`: `chunk_wire::a_chunk_crosses_the_wire_with_its_blocks_biomes_and_block_entities` (every block, every biome cell, a chest's block entity, a wrong position and a cut blob refused), `chunk_wire::a_batch_crosses_the_wire_in_order`, `scene::the_change_log_reports_every_set_once_until_it_is_taken`, `terrain::a_remote_stream_takes_its_bounds_from_the_dimension_and_has_no_chunk_map` (all three dimensions), `terrain::a_remote_stream_meshes_the_chunks_the_host_sends` (25 chunks through the wire, lit and built into sections, then one forgotten).
  - `minecraft_survival`: `level::a_chunk_sent_after_edits_from_every_source_matches_the_scene`. In a real End level it places and breaks through the player's world interface, writes a server change, blows a crater, lays water and flowing water and fills a box across four chunks, none of them calling `record_edits`. Before the tick the chunk map does not have the placement; after one `stream_tick`, every block of the four chunks, sent through the wire, equals the scene.
  - Whole lib suites: `minecraft_terrain` 849 passed (4 ignored as before), `minecraft_survival` 337, `minecraftoss-core` 23. `check -p launcher` is green.
- **Decisions:**
  - **The remote stream is `server: Option<ChunkMap>`, not a second type,** as the design allows, so lighting, sections and meshing stay one code path.
  - **The wire carries the region file's zlib body of vanilla's chunk NBT, light included.** Vanilla sends a chunk with its light (`ClientboundLevelChunkWithLightPacket`), and the zlib body keeps a chunk well under the 128 KiB batch cut (the test chunk is under 64 KiB). No new dependency was needed; the vendored `StoredChunk` gained two small accessors.
  - **The log is drained in `stream_tick`,** the one place each level's terrain ticks. E6 takes the same drained positions for the block-update fan-out (design 7.3); until then the positions only go to `record_edits`. Levels whose terrain does not tick keep their (deduplicated) log until they do, and `ChunkMap::set_blocks` ignores positions in chunks it no longer holds.
  - The remote stream's spawn fields start at 0, 0, 0, as a vanilla client's level does before the spawn packet; E7b sets them from `Login`.
- **Test steps:** none in game yet; the game still runs the host path only. `check -p launcher` is green. The single-player world must load, stream and save exactly as before; the owner's M1 test covers it once E7c shows chunks on a friend's game.
- **Kit:** none.
