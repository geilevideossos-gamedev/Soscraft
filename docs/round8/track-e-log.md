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

Commit: 3a98934f

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

### E6. The host streams its world to each friend

Commit: 40b5a4fa

- **Changed:**
  - `crates/minecraft_survival/src/session.rs` (new):
    - `ChunkSender`, a simple form of vanilla's `PlayerChunkSender`: the friend's level, their `TrackingView` (vanilla's `ChunkTrackingView`, now re-exported as `minecraft_terrain::view`), `sent` and `pending`. A chunk entering the view becomes pending; one leaving it is dropped from `pending`, or gets a `ChunkForget` if it was sent. Batches hold up to 9 loaded chunks, nearest first. One batch is in flight until two have been acknowledged, then two. A new level starts the stream again.
    - The `ChunkBatch { seq }` marker goes into the friend's reliable stream when the chunks are taken from the chunk map, and the chunks count as sent from then on.
    - `cut_batch` / `encode_batch` build the `"MC"` blob on the `minecraft-chunk-send` worker. A batch is cut at 128 KiB (at least one chunk stays). Chunks that did not fit go back to pending. A single chunk over the 256 KiB bulk limit is skipped with a warning. Every batch gets a blob, even an empty one, so the friend never waits on a marker forever.
    - `Updates` / `block_updates`: each tick's changed positions in a level with friends in it become one `BlockUpdates` (state ids), plus a `BlockEntity` where the chunk map holds a block entity at a changed position. Each friend gets only the changes in chunks it was sent, and only for its own level.
    - `Sessions`:
      - `hello`: the protocol and registry check, then "A player with that name is already in the world". Then `Server::join_friend`, `Login` first, the full `PlayerList` to the newcomer, the newcomer to everyone else, and "multiplayer.player.joined" in yellow to everyone including the host.
      - `leave`: `Server::leave`, `PlayerList.removed` and "multiplayer.player.left".
      - `take_retiring`: a refused friend's MW2 client is retired 1 s after its `Rejected`.
      - `lock_travel`, `fan_out`, `select_batches` and `take_outgoing`.
    - `registry_fingerprint`: FNV-1a 64 over the block state count and every state's name with its properties. `rules_of` fills `Rules` from the game rules.
  - `crates/minecraft_survival/src/server.rs`:
    - **One writer thread with a queue** (`SaveWriter`, thread `minecraft-world-save`): `save_player`, `autosave` and the leave save are sends on a channel, written in order. The main thread never joins a writer any more. `save_and_close` waits for it (`finish`).
    - **`recently_left`** lives in the writer: a leaving player's `PlayerData` stays there until the writer reports their file written.
    - `join` is split into `join` (reads the file) and `join_from` (builds the player from `PlayerData`).
    - `Server::join_friend(id, name, kit, host_dimension)`:
      - It takes the profile from the MW2 name (`chat.join`).
      - It reads `recently_left` first, else the player file, else it makes a new player with the kit.
      - A friend saved in another dimension than the host's is placed at the spawn of the host's level (section 5.3).
      - The first placement is always set as an arrival, so the host teleports the friend's body there.
    - `Server::leave(uuid)`, in this order:
      - it settles the level thread for that player;
      - it folds the crafting grid, workbench and carried stack back into the inventory and drops what does not fit at their feet;
      - it removes them as an opener and calls `chat.leave`;
      - it saves them through the queue and removes them.

      It returns `Left { player, dropped }`.
  - `player.rs`: `Player::close_menu` (returns the overflow instead of losing it), `travel_locked`, `travel_refused` and `refuse_travel`.
  - `entities.rs`: `settle_player(id, ..)` waits for the level thread, takes the picks of outputs not yet read (`take_late`), drops that player's pearl landings, pushes and potions, then runs `leave`. The level thread serves only player 0 until M2, so it does nothing for a friend yet.
  - `level.rs`: `Terrain.friend_changes`. While a friend is in the level, `stream_tick` copies the drained change log into it for the fan-out.
  - `commands.rs`: `/execute in <other dimension> run tp` is refused while travel is locked.
  - `crates/render_anim/src/minecraft_host.rs` (new): the host system, run after `minecraft_world::update`. Each frame it:
    - reads `MinecraftWire`: `Hello` and `ChunkBatchReceived`, and counts broken messages;
    - holds a `Hello` until the world is ready and the MW2 name is known, then runs the sessions;
    - drops refused friends through the wire;
    - places friends' bodies;
    - runs the encode worker and hands the blobs to `send_batch`;
    - fans out block updates;
    - sends every queued message, and shows host chat lines in the host's own chat.
  - `minecraft_world.rs`:
    - `place_friend` puts a friend's body at their arrival once the host's collision has the chunks around it (the per-player form of the spawn gate). After that it follows their MW2 state into `player.place` with `place_player`, as for the host.
    - After an MW2 death the friend comes back at the level's spawn.
    - `Runtime` is `pub(crate)`, with `host_view` and `show_lines` for the host system.
  - `minecraft_portal.rs`: a due portal (Nether portal, End portal, exit portal) is refused while travel is locked.
  - `crates/minecraft_terrain/src/lib.rs`: re-exports `minecraftoss_world::view`.
- **Tests:**
  - `minecraft_survival`, `session`:
    - `one_batch_flies_until_two_are_acknowledged_then_two`
    - `an_unacknowledged_batch_holds_the_stream_and_stray_acks_do_not_open_it`
    - `only_loaded_chunks_are_sent_and_a_cut_chunk_is_sent_again`
    - `a_forget_after_a_load_stays_behind_the_marker`
    - `another_level_starts_the_chunk_stream_again`
    - `block_updates_reach_only_friends_in_that_level_with_that_chunk`
    - `a_batch_is_cut_at_its_size_limit_and_keeps_at_least_one_chunk`
    - `a_friend_respawns_only_after_a_life_has_been_seen`
    - `a_hello_joins_the_friend_with_login_first`
    - `a_second_hello_with_a_name_in_the_world_is_refused_and_retired` (also the protocol and registry refusals)
    - `a_leaving_friend_is_listed_out_and_their_session_ends`
    - `travel_is_refused_while_a_friend_is_online`
    - `the_registry_fingerprint_is_stable_and_sees_every_state`
  - `minecraft_survival`, `server`:
    - `a_leaving_friend_folds_their_menu_back_drops_the_rest_and_saves_off_the_main_thread`: a job waiting on a gate holds the writer. `leave` still returns at once and the file is not there yet; it is there after the gate opens.
    - `a_quick_rejoin_reads_the_player_the_writer_has_not_written_yet`
    - `a_friend_saved_in_another_dimension_joins_the_hosts`
  - `minecraft_survival`, `entities`: `a_leaving_player_gets_the_late_picks_and_loses_their_pearls_and_pushes`.
  - `minecraft_survival`, `commands`: `execute_in_is_refused_while_friends_are_online`.
  - Whole lib suites: `minecraft_survival` 355 passed, `render_anim` 73, `minecraft_terrain` 849 (4 ignored as before). `check -p launcher` is green.
- **Decisions:**
  - **Pacing is the session's, the resend is the wire's.** The session keeps a batch in flight until `ChunkBatchReceived`. The wire (E3) keeps the blob and sends it again with the same `seq`. A stray or repeated acknowledgement does not open the window.
  - **`recently_left` is a field of the writer** (`Globals.writer`), not its own `Globals` field. An entry goes away exactly when the writer reports the file written, and each `Globals` constructor changes by one line.
  - **A temporary world** (`IW4L_MINECRAFT_WORLD=temp`) has no writer, so a leaving friend's state stays in `recently_left` until the host closes the world. A friend who rejoins gets it back, and it goes away with the world.
  - **A refused friend gets `Rejected`, and 1 s later its MW2 client is retired** through the wire's `drop_peer` (`retire_client_with_reason`, with the refusal in English). The friend then sees the reason on the MW2 disconnect line even if `Rejected` has not arrived.
  - **Names are compared ignoring case**, like vanilla's `PlayerList.getPlayerByName`. Offline UUIDs differ by case, so "steve" and "Steve" would otherwise be two players with two files.
  - **A new friend joins in the world's default game mode**, not in `IW4L_MINECRAFT_GAMEMODE`, which is the host's own choice. A friend with no player file gets the host's `IW4L_MINECRAFT_KIT`.
  - **Travel lock:**
    - `Player.travel_locked` is set for every player each frame while any session is open.
    - A portal is refused when it becomes due. That gives one chat line per entry, because the portal cooldown keeps the player from re-entering at once.
    - Respawning into another dimension is not locked. The design does not list it, and while travel is locked everyone is in one dimension, so only a death with a respawn point in another dimension can split the players.
    - End gateways are not built yet (X6).
  - **The first teleport of a friend waits until the host's collision has the chunks around the target** (3 chunks). Until E9 loads chunks around every player, a friend whose saved spot is outside the host's loaded area waits at the MW2 spawn. The M1 owner test keeps both players together.
  - **After an MW2 death, a friend in M1 comes back at the level's spawn.** Beds and anchors for friends come with the per-player step in M2.
  - **Block entities follow block updates** only where the block changed. Chest contents that change without a block change reach a friend only in later chunk sends (menus come in M2).
  - Friends have no MW2 body gear or credits hold yet, so the leave flow has nothing of those to clear.
- **Test steps:** a friend's game starts sending `Hello` only with E7c, so nothing new shows in a two-window test yet. The saving changes run in every single-player game, so check them with the M1 build:
  1. Start the Minecraft world as usual. Put the kit's chest down and put some items in it. Break and place a few blocks.
  2. Play past an autosave (5 minutes; the log says `Minecraft autosave: N chunks queued`). The game must not hitch at the autosave.
  3. Run `/gamemode creative`, then `/gamemode survival` (each saves the player file through the new queue). Then quit through the menu.
  4. Start again. The chest, its items, the blocks, your inventory and your position are as you left them.
  5. Build a Nether portal with the obsidian and light it. Walk in: you still travel to the Nether when you play alone.
- **Kit:** `minecraft:obsidian` x10, `minecraft:flint_and_steel` x1, `minecraft:chest` x1, `minecraft:cobblestone` x32.

```
IW4L_MINECRAFT_KIT=obsidian*10,flint_and_steel,chest,cobblestone*32
```

### E7a. The world's presentation is built by functions both games can call (pure move)

Commit: 0eb26b35

- **Changed:** `crates/render_anim/src/minecraft_world.rs` only. No behaviour changes; the host calls the same code through these functions:
  - `load_look(packs)`: the atlas, sun and moons, clouds, crack strip, glints and End look, taken out of `load()`. `Loaded` and its `packs` and `atlas` fields are `pub(crate)`.
  - `show_look(view, world)`: hands that look to `MinecraftWorldView` when a world is ready.
  - `animate_atlas(view, animation, clock, dt, atlas)`: the animated textures' tick.
  - `view_forward(viewangles)`, `cull_camera(drawn_from, origin, eye, forward, aspect)` and `terrain_frame(terrain, world, view, camera)`: the terrain frame (camera, section uploads, visible list), returning the light columns for the caller's `SkyLight`.
  - `SkyInputs` and `present_sky(view, terrain, registries, light, (accumulator, primed), inputs)`: eye light, rain fog, the environment tick and the sky uniform. The host fills `SkyInputs` from its globals, weather, lava check and boss bars.
  - `present_clouds(view, world, center, eye, game_time)` and `present_light_volume(view, light, block, at, age)`.
  - `clear_view(view)`: the view reset of `stop`.
  - `observe_spawn` and `shape_id` are `pub(crate)` for the friend's game.
- **Tests:** none new (pure move). `test -p render_anim --lib` and `check -p launcher` are green.
- **Decisions:**
  - **Only what the friend draws in M1 moved.** The design lists entity meshes, the hand and held item, and the HUD too. In M1 the friend has no mobs, no `PlayerView` and no menus, so those stay inline until the groups that first draw them on the friend (M2), and no function exists with a single caller.
  - The existing comments inside the moved code moved with it unchanged; no comments were added.
- **Test steps:** the single-player world must look and behave exactly as before. With the M1 build:
  1. Start the Minecraft world. Look around: terrain, sky, sun, clouds and fog as before; walk into a dark cave with a torch: the MW2 body and gun are lit by the torch.
  2. Wait for rain or run `/weather rain`: rain fog and darker sky as before. Run `/time set night`: stars and moon.
  3. Stand in lava in creative (`/gamemode creative`, then a lava bucket): the orange lava fog.
  4. Build a Nether portal and go through: the loading screen, then the Nether's red fog and no sky.
- **Kit:** `minecraft:obsidian` x10, `minecraft:flint_and_steel` x1, `minecraft:lava_bucket` x1, `minecraft:torch` x16.

```
IW4L_MINECRAFT_KIT=obsidian*10,flint_and_steel,lava_bucket,torch*16
```

### E7b. RemoteWorld applies what the host sends

Commit: bd3fd000

- **Changed:**
  - `crates/minecraft_survival/src/remote.rs` (new), with no Bevy:
    - `RemoteLevel { dimension, epoch, origin, feet, terrain, block_entities, mobs }`. `terrain` is a `level::Terrain` with a remote `TerrainStream` (E5), its scene, the dimension's environment and rules, so the friend's game presents it with the host's own functions (E7a).
    - `RemoteWorld` holds the login, the level, the player list, chat, the player view, rules, the clocks (`Clocks`: Overworld and End, advanced locally between `Time` messages), weather, the last `StateFrame`, a `rejected` reason and a `failed` reason. `apply(ToClient)` and `apply_state(StateFrame)` update it.
    - **Ordering (design 11.2):** reliable messages apply strictly in arrival order. At a `ChunkBatch { seq }` marker whose batch is not decoded yet, the stream stops and what follows is held, so block updates, forgets, mob adds and a `Dimension` never pass a missing batch. Nothing but `Login` (or `Rejected`) applies before `Login`.
    - **Batches:** `receive_batch(blob)` keeps the raw blob by its `seq` (`ChunkBatchBlob::peek_seq`). `take_decode_jobs()` hands out the blobs to read once a level exists, tagged with the level's epoch and its `BlockStates`. `decoded(seq, epoch, chunks)` stores the result, and a batch read for another epoch is read again for the current level. `decode_now()` reads inline (tests, and the fallback when no worker runs). A batch is applied to the stream (`TerrainStream::apply` with `ChunkEvent::Load`) and acknowledged (`take_acks`) when its marker is reached. A blob whose `seq` was already applied (the host's resend) is not read again, only acknowledged again.
    - Block updates set the scene only in loaded chunks, check the state id against the registry, and remesh through `mark_edited`. `ChunkForget` removes the chunk and its block entities.
    - `take_shown()` reports what the game must apply to the MW2 collision: a new level, chunks loaded and forgotten, and changed blocks with their state.
    - Messages the friend does not use yet (sounds, level events, menus, boss bars, credits and the rest) go to `effects` for the presenter.
    - The held blobs and messages are counted; past 64 MiB the stream fails with "Lost the world stream".
  - `crates/minecraft_survival/src/level.rs`: `Terrain::remote(registries, seed, view_distance, dimension)`.
  - `crates/minecraft_terrain/src/lib.rs`: re-exports `ChunkEvent`.
- **Tests (`minecraft_survival`, `remote`):**
  - `a_block_update_behind_a_missing_batch_waits_and_applies_after_it` (a chat line behind it waits too; the batch is acknowledged when applied)
  - `a_dimension_behind_a_missing_batch_keeps_later_mob_adds_from_the_old_level`
  - `a_batch_decoded_for_the_old_level_is_read_again_for_the_new_one`
  - `a_batch_before_login_waits`
  - `a_resent_batch_is_ignored`
  - `a_state_frame_older_than_the_last_or_of_another_epoch_is_ignored`
  - `time_weather_players_and_rejections_reach_the_friend`
  - `a_stream_held_past_its_limit_is_lost`
  - **Loopback:** `a_friend_sees_the_hosts_blocks_in_order_and_their_leave_is_saved`. A real `Server` on a temp world with an End level, a `Sessions` with a friend joined by `Hello`, the host's batches encoded with `encode_batch`, every message encoded and decoded with `minecraft_net`, feeding a `RemoteWorld`, and acknowledgements fed back. Every block of the 3x3 chunks around the platform equals the host's; a placed and then broken block, and a second broken block, reach the friend in the host's order; leaving saves the player file.
- **Decisions:**
  - **A batch is acknowledged when it is applied**, as vanilla's client sends `ServerboundChunkBatchReceivedPacket` after the batch is handled. The host's pacing then follows what the friend has really shown.
  - **A blob is decoded against the level it was read for.** Decoding ahead keeps the stream moving, and the epoch check makes a batch that crossed a dimension change be read again with the new dimension's heights, instead of going into the wrong level.
  - **A batch that cannot be read is applied as empty with a warning**, so the stream does not stop for good; the friend sees a hole until the chunk is sent again.
  - **Mobs are kept as their `MobAdd` tag only** until the mob mirror lands (E16, M2); that is enough to keep the order rule tested.
  - `ToServerFrame` / `ToClientFrame` stay unused (E3's decision); the wire frames the messages.
- **Test steps:** none in game yet; E7c uses `RemoteWorld`.
- **Kit:** none.

### E7c. A friend's game shows the host's world

Commit: a2a714cb

- **Changed:**
  - `crates/render_anim/src/minecraft_remote.rs` (new), the `friend` system, in the same set as `minecraft_world::update` and only in `RuntimeRole::Client`:
    - **Loads** the registries, the resource pack, the look (E7a's `load_look`), the language and the registry fingerprint on the `minecraft-world-load` thread when the Minecraft map is installed. No world folder is opened.
    - **Hello:** once `MinecraftWire` is up as a friend, it sends `Hello` with protocol, view distance 8 and the fingerprint, again every 2 s until `Login`, `Rejected` or a failure.
    - **Receives:** reliable messages into `RemoteWorld::apply`, state frames into `apply_state`, `"MC"` blobs into `receive_batch`; blobs are read on a `minecraft-chunk-read` worker (inline if it could not start). Each applied batch is acknowledged with `ChunkBatchReceived`. A message that does not decode is dropped and counted (`bad_message`).
    - **Collision:** when a level arrives, `sim::voxel::activate` with the MW2 prediction world's brushes and the level's origin from `Login.origins` (the host's origin for that dimension, so both games put a block at the same map point). Loaded and forgotten chunks go through `apply_streamed`, changed blocks through `shape_id` and `set_block_shape`.
    - **Loading screen:** `ui.loading_world` holds MW2 admission while the look loads, until `Login`, and then until the spawn gate sees collision around `Login.feet` and a visible section there (the host's own gate, with its 30 s limit).
    - **Presents** with E7a's functions: the atlas animation, the terrain frame from the player's eye (or the spawn, while the gate holds), the light columns, the sky and environment from the host's clocks and weather, clouds and the light volume.
    - Chat lines from the host, a refusal, a failed stream and "The host's Minecraft world does not answer: the host may run a different build" (no `Login` 30 s after the look loaded, E3's finding) go into the Minecraft chat.
    - A torn-down match clears the view, the collision and the friend's state.
  - `minecraft_world.rs`: `update` returns at once in the Client role, so the friend's game never calls `load()` or opens a world folder; the `friend` system is registered.
  - **Host:** `session::time_of(globals)` and `Sessions::send_time`: every friend gets `Time` (game time, the Overworld and End clocks with their pause, from the `advance_time` rule and `/time`) and the weather of their level once a second (`minecraft_host.rs`).
- **Tests:**
  - `minecraft_survival`, `session`: `every_friend_gets_the_time_of_both_clocks_with_their_pause`.
  - No other new tests (the friend system is Bevy glue over E7b's tested `RemoteWorld`). `test -p minecraft_survival --lib`, `test -p render_anim --lib` and `check -p launcher` are green.
- **Decisions:**
  - **Time and weather go once a second**, as vanilla's `ServerLevel` sends `ClientboundSetTimePacket` every 20 ticks. Between them the friend advances its clocks itself, like a vanilla client; a `/time set` shows within a second. The friend eases rain and thunder toward the host's values at vanilla's 0.01 per tick, so the fog does not jump once a second.
  - **After 30 s without `Login` the loading screen lets the friend in**, with the "does not answer" line, instead of holding them forever; the MW2 match itself still works.
  - **The friend's Minecraft HUD stays off in M1** (`ui.active` false): its hearts, hotbar and hand come from `PlayerView` in M2. The MW2 HUD shows meanwhile. The friend's chat lines are kept; the chat is drawn and typed into from E8.
  - **Sounds, level events and the other effects** are taken from `RemoteWorld` and dropped until the groups that play them (M2, M3).
- **Test steps** (with the M1 play build; `scripts\mp-local.ps1` picks the newest `iw4l-mp-*.exe`):
  1. Build once: `& scripts\cargo-slot.ps1 build --profile play --locked -p launcher`, then copy `D:\iw4l-target\play\iw4l.exe` to `C:\Games\Mashup\2010-Rust-Rewrite-Mashup\iw4l-mp-m1.exe`. The relay from E3 must be the patched one.
  2. `& scripts\mp-local.ps1 -World mp-m1 -Fresh`. The host window loads the world as usual. The friend window stays on the loading screen until the host's chunks have arrived (a few seconds), then shows class select. Screenshot the friend's loading screen and then class select.
  3. Friend window: spawn. The friend stands next to the host's spawn on the same terrain the host sees (same trees, same blocks), with the same sky and sun position. Screenshot both windows from about the same spot.
  4. Friend window: walk, jump, fall off a block. The friend never falls through the ground or walks through blocks.
  5. Host window: shoot a block until it breaks. Friend window: it is gone too. Friend window: shoot a block until it breaks; both windows show it gone (the host breaks it for the friend's bullets).
  6. Host window: `/time set night`. Within a second the friend's sky is night too. `/weather rain`: the friend's sky darkens and fogs over within a few seconds.
  7. Quit the friend window. Host chat: "Friend left the game". Start it again with `scripts\mp-local.ps1 -FriendOnly`: it loads into the same world again.
  8. Stay within about 8 chunks of the host (chunks only load around the host until M2).
- **Kit:** none needed.

### E8. Chat and commands work for every player

Commit: ceaf6959

- **Changed:**
  - `crates/minecraft_commands`:
    - New `/msg <targets> <message>`, with `/tell` and `/w` redirecting to it. It works like vanilla's `MsgCommand`: level 0, and for each target the sender gets "You whisper to %s: %s" and the target "%s whispers to you: %s", both gray and italic (`commands.message.display.outgoing` / `.incoming`).
    - The help and suggestion tests list the three new commands.
  - `crates/minecraft_survival/src/commands.rs`:
    - `run` takes the lines as `Serverbound` and returns every `Outgoing`, not only what reaches the sender. The same goes for the answers to finished jobs (`job_answers`, used for `/fill`, `/setblock` and `/locate`) and for `summon_answers`, which now carries the source of each summon.
    - New `take_lines` (the host's typed lines), `from_friend` (a friend's line, normalized and cut at 256 characters like the host's own), `reaching` (the lines one player sees) and `handle_all`.
    - `pump` and `answer_jobs` keep their sender-only form for the existing tests.
  - `entities.rs`: `Entities::summon` takes the source and keeps a queue of them, so each `/summon` answer from the level thread goes back to the player who ran it.
  - `session.rs`:
    - `Sessions::deliver(chat, out, to_host)` puts each line into the outbox of every session it reaches, and into `host_lines` when the host should see it.
    - `Sessions::send_levels` sends each friend their command level (`Clientbound::Commands`) on joining and whenever it changes, as vanilla sends the permission level.
  - `remote.rs`: `RemoteWorld.command_level` keeps that level, and it never shows as a chat line.
  - `render_anim/src/minecraft_world.rs`:
    - The host's command run lists every online player's position and life (`friends_presence`, `presence_of`), so `/tp Friend`, `/give Friend` and `@a` see the friend where their soldier stands.
    - The host shows only the lines that reach it. Everything its commands and summons produced, and its teleports of other players, go into `Runtime.relayed` for the host system.
  - `render_anim/src/minecraft_host.rs`:
    - `ToServer::Chat` from a friend with a session is queued.
    - Each frame, after the bodies are placed, every friend's lines run through `commands::run` as that player. Their output is routed to the host and to every friend it reaches. The host's own output goes to the friends.
    - Teleports from any command move that player's MW2 body. Command levels are sent.
  - `render_anim/src/minecraft_remote.rs`:
    - The friend's game loads the Minecraft GUI atlas, which holds the chat font.
    - It sets `MinecraftUi.chat_only` once logged in, sends each typed line to the host as `ToServer::Chat` and keeps the command level for its chat's suggestions.
  - `frame`: `MinecraftUi.chat_only`, `chat_usable()` and `typing()`.
  - `hud`: the chat opens and types while `chat_usable()`. The menus' Escape gate uses `typing()`.
  - `console`: with `chat_only`, only the chat and command keys (T and /) are taken from MW2, and MW2 input stops while the chat is open. `skate.rs` uses `typing()` too.
  - `docs/MULTIPLAYER.md` section 13.1 says what was built.
- **Tests:**
  - `minecraft_commands`:
    - `tell_whispers_to_the_named_player_only` covers `/msg`, `/tell` and `/w`, an unknown name and `@a`.
    - `a_level_zero_source_sees_help_and_seed_only`, `help_lists_smart_usages_of_visible_commands`, `root_literals_are_filtered_by_level` and `redirects_suggest_and_colour_the_target_command` now list the new commands.
  - `minecraft_survival`, `session`. Each runs on a real `Server` with the game data and real `Hello` joins:
    - `chat_reaches_everyone_and_tell_reaches_one`: `Recipient::All` from a friend and from the host reaches the host and both friends. `/tell` from the host reaches only that friend, and a friend's `/msg` to the host reaches only the host.
    - `an_op_only_command_is_refused_for_a_new_friend_and_allowed_for_the_host`:
      - a level-0 friend's `/time set` is refused and changes nothing;
      - the host runs it without `ops.json`;
      - after `/op alex` the friend runs it, and the host sees the `[Alex: ...]` admin line;
      - the command level is sent once, then again after the change.
    - `a_friend_is_named_in_chat_while_in_the_world_and_never_after`: the join and leave lines reach the host and the friends. A left player's name no longer resolves ("No player was found").
  - `minecraft_survival`, `remote`: `chat_lines_show_and_the_command_level_is_kept`.
  - `console`, `binds`: `only_the_chat_keys_work_while_only_the_chat_does`.
  - `render_anim`, `minecraft_chat`: `summon_answers_follow_the_feedback_rule` gives a friend's summon answer to that friend.
  - Results:
    - `test -p minecraft_commands`: 117 passed.
    - `test -p console`: 36 passed.
    - `test -p frame`: 23 passed.
    - `test -p hud --lib`: 138 passed.
    - `test -p minecraft_survival --lib`: 369 passed.
    - `test -p render_anim --lib`: 73 passed.
    - `check -p launcher`: green.
    - The session tests were checked to run on the real data: one was made to fail without it, it passed, then the skip was put back.
- **Decisions:**
  - **Friends' commands run in the host system, not in `minecraft_world::update`.** Their lines arrive there, and track A is reshaping the update loop. The host's own commands stay where they were. What they send to others is handed over in `Runtime.relayed` and routed in the same frame.
  - **`/msg`, `/tell` and `/w` were added.** The plan's test needs `/tell`, and the command tree had no private message. `/me`, `/say` and `/teammsg` are not built.
  - **The command level is sent like vanilla's permission-level entity event.** It goes on join and on every change, so a friend's chat suggests the commands they may run. Names, positions and other hints are still not sent (design 13.1).
  - **A friend's lines are cut at 256 characters on the host,** as the host's own are, whatever the friend's game sent.
  - **The friend's chat works while the rest of its Minecraft HUD is off.** It is drawn and typed into once `Login` has arrived. On the friend, T and / open it and are taken from MW2. The other Minecraft keys stay with MW2 until the friend has a hotbar (M2).
  - **Summon answers find their source through a queue in `Entities`,** in the order the level thread answers. Nothing in the level thread changed.
  - **Not covered yet:**
    - `/kill`, `/gamemode`, `/effect` and `/give` on a friend change the friend's server-side player, but nothing shows on the friend until the per-player step and `PlayerView` (M2).
    - Death lines are still host-only (13.2).
- **Test steps:** the owner's M1 test is `docs/round8/e-m1-tests.md`.
- **Kit:** none needed.

### M1 owner test

Commit: see `git log` ("The owner's two-window test for multiplayer M1")

- **Changed:** new `docs/round8/e-m1-tests.md`. It holds:
  - the owner's numbered M1 test with two windows on one PC: the builds, the `mp-local.ps1` call, which window does what, and the screenshots to send;
  - the steps for a friend on their own PC (section 18 of the design): what the host sets up, what is in the friend's zip, the friend's first start, and the ways to join.
- **Decisions:**
  - The test follows the plan's M1 steps (design section 19, E8) and adds checks for each E8 behaviour: permissions before and after `/op`, whispers both ways, `/tp` of the friend by either player, a friend's `/summon` answer, and the op being kept after a rejoin.
  - The commands whose effect on the friend only shows from M2 (`/give`, `/gamemode`, `/kill`, `/effect`) are listed as not testable yet.
- **Tests:** none (a document).
- **Test steps:** `docs/round8/e-m1-tests.md`.
- **Kit:** none needed.

### Review fix: a host that is still loading its world is not taken for a different build

Commit: see `git log` ("A friend waits for a host that is still loading its world"). The previous commit is `ab7db8ea`.

- **Defect (found in the adversarial review of M1):** the friend's game let itself into the MW2 match with the line "The host's Minecraft world does not answer: the host may run a different build" whenever no `Login` came 30 s after its look loaded. The host holds a `Hello` until its own world has loaded, so a host still generating a fresh world (`mp-local.ps1 -Fresh`, two windows sharing the CPU) was taken for an incompatible build. The friend then entered the match with no Minecraft collision and fell through the world, and when `Login` came later its spawn gate held again in the middle of the match.
- **Changed:**
  - `crates/net/src/minecraft_wire.rs`: `MinecraftWire::heard_from(peer)`, true once that peer has acknowledged one of our frames or sent one. A host on this build acknowledges each resent `Hello` while it holds it; a host on an older build drops the unknown packets and never does.
  - `crates/render_anim/src/minecraft_remote.rs`: `host_silent(ready_at, now, heard)`. The 30 s fallback and its chat line apply only while the host has never been heard. A host that answers on the wire keeps the loading screen up until its `Login` and chunks arrive, like a vanilla client on "Loading terrain" while the connection is alive.
- **Tests:** `net`: `minecraft_wire::a_host_that_holds_the_hello_is_still_heard` (a friend's `Hello` reaches the host, the host answers with an acknowledgement only, and the friend counts it as heard). `render_anim`: `minecraft_remote::only_a_host_that_never_answers_lets_the_friend_in_without_a_world`. `test -p net --lib minecraft_wire` 14 passed, `test -p render_anim --lib` 74 passed, `check -p launcher` green.
- **Decision:** no upper limit while the host answers. The MW2 connection's own timeouts end a dead host; a live host whose world failed to load ends the match on its side.
- **Test steps** (with the M1 play build, before the owner test of `docs/round8/e-m1-tests.md`):
  1. `& scripts\mp-local.ps1 -World mp-slow -Fresh`.
  2. Friend window: it stays on the loading screen while the host window still shows its world loading, however long that takes, and never shows "The host's Minecraft world does not answer".
  3. Friend window: once the host's world is in, class select shows and the friend spawns on the host's terrain without falling.
- **Kit:** none needed.

### Review fix: the M1 owner test closes both scripts' windows

Commit: see `git log` ("The M1 owner test closes the rejoined friend window from its own script"). The previous commit is `1ef41e28`.

- **Defect:** step 13 of `docs/round8/e-m1-tests.md` said that `quit` in the first PowerShell window closes both windows. After step 12 the friend window was started by the second (`-FriendOnly`) script, and `quit` only reaches the windows a script started itself, so the friend window stayed open and its script kept running.
- **Changed:** `docs/round8/e-m1-tests.md` step 13 types `quit` in the second window first, then in the first, and asks for both peak-memory lines. Step 4.3 adds that chat lines fade after 10 seconds (vanilla's 200 ticks), so the join line may need T to be seen after a long load.
- **Tests:** none (a document).
- **Test steps:** `docs/round8/e-m1-tests.md`.
- **Kit:** none needed.

### Final verification: clippy on the track's own lines

Commit: see `git log` ("Multiplayer M1 code passes clippy"). The previous commit is `ccece5de`.

- **Not track work:** `7f64c38d` is the same `cargo-slot.ps1` change as `39407502` on `minecraft-survival`, taken early so the track could build gently. It has no game code and no in-game test.
- **Clippy** on the crates the track touched (`console`, `frame`, `hud`, `iw4l-master`, `minecraft_commands`, `minecraft_net`, `minecraft_survival`, `minecraft_terrain`, `net`, `render_anim`, `sim`, `minecraftoss-core`) with `--all-targets -A clippy::approx_constant`. Every warning on a line the track added is fixed; older warnings stay.
- **Changed (no behaviour change):**
  - `unwrap_or_default()` (refused by the workspace `clippy.toml`) became an explicit fallback: `map_or_else(Vec::new, ...)` in `minecraft_survival::remote` and `render_anim::minecraft_host`, `unwrap_or_else(|_| ...)` in `minecraft_survival::session` and `render_anim::minecraft_remote`, `map_or_else(ChunkMapStats::default, ChunkMap::stats)` in `minecraft_terrain::terrain`. The friend-change drain in `session` takes the list with `let ... else`, which skips a level with no list exactly as the empty list did.
  - `net::minecraft_wire::MinecraftWire::drop_peer`: the friend's `if` became a match guard.
  - `render_anim`: `type ColumnLights` for `terrain_frame`'s result, and `clippy::type_complexity` allowed on the `friend` system next to its `too_many_arguments` (Bevy system parameters).
  - `sim::damage::height_bands`: the `drop(frame)` calls on a type with no `Drop` are gone; the borrow ends at its last use.
- **Tests:** none new. `test -p` of each touched crate is green: `minecraft_survival` 369 + 1, `minecraft_terrain` 849 (4 ignored), `net` 14, `render_anim` 74, `sim` 78, `minecraft_net` 15, `minecraft_commands` 117, `hud` 138, `console` 36, `frame` 23, `minecraftoss-core` 23, `iw4l-master` 1. `check -p launcher` is green.
- **Test steps:** none new; the M1 owner test in `docs/round8/e-m1-tests.md` covers these lines.
- **Kit:** none needed.

## Milestone M2: the friend plays survival

This run builds E9, E10a and E10b. E11 to E15 follow after track A (which reshapes the level-thread tick loop) has merged into `minecraft-survival` and into this branch, so this run leaves `server_loop` and its tick scheduling in `crates/minecraft_terrain/src/server.rs` alone. The owner's two-window steps for M2 are collected in `docs/round8/e-m2-tests.md`.

### E9. The chunk map loads chunks around every player

Commit: see `git log` ("The chunk map loads chunks around every player"). The previous commit is `79ba1549`.

- **Changed:**
  - `third_party/minecraftoss/world/src/chunk_map.rs` (vendored, patched): `ChunkMap::tick(&[ChunkPos])` keeps one `TrackingView` per player in `views` (and `queued_for` is the list the queue was built for). `tracks`, `is_idle` and the tracking diff use the union of every view and the tickets: `tracking_changes` collects the changed positions of each old and new view pair and of the ticket squares, then keeps those whose membership in the union changed, so a chunk another player still sees is never forgotten. `ChunkEvent::Center` follows the first player (the host). The generation queue (`wanted_for`) dilates each player's square and each ticket square on its own small grid (`wanted_around`, the old separable max filter) and merges them, keeping FEATURES over TERRAIN, so two players far apart never build one grid spanning both. The queue is sorted by the nearest player's chessboard then squared distance (ties by position), chunks and biome cells are kept within `view distance + 5` of any player, and pending chunks are sent nearest to any player first. Distances are computed in `i64`, so players far apart cannot overflow.
  - `crates/minecraft_terrain/src/terrain.rs`: `TerrainStream::server_tick(players, local, scene)` passes every player's chunk to the chunk map and every chunk it loads to the scene. Only chunks inside the host's own `TrackingView` (`shown`, the server view distance around `local`) are registered with `Sections` and lit; when that view moves, `show` registers the scene chunks that entered it and drops the sections, light and queued work of the ones that left. Forgetting, `mark_edited`'s emptiness updates, `reshow` and `set_view_distance` follow the same filter, so a section's emptiness is never recorded for a chunk the host does not draw. A remote (friend) stream shows every chunk, as before. `Sections::has_chunk` is new (for the test).
  - `crates/minecraft_survival/src/level.rs`: `stream_tick(terrain, entities, containers, players, local)`. New `player_blocks(players)`, each player's block with their dimension (the arrival if one is pending, else their place, only when it is in the player's current dimension, exactly what the chunk sender follows), and `blocks_in(blocks, dimension)`.
  - `crates/render_anim/src/minecraft_world.rs`: the host's frame collects the other players' blocks once, before it borrows the host's player, and both `stream_tick` calls (the normal one and `stream_around_spawn` at the spawn gate) pass the host first, then every other player in that level, with the host as `local`.
  - Callers in tests and `examples/smoke.rs` pass one player.
  - `docs/MULTIPLAYER.md` 7.1: the "until this lands" note now says what E9 built.
- **Tests:**
  - `minecraftoss-world` (no game data needed): `chunk_map::two_players_a_hundred_chunks_apart_track_and_queue_both_squares` (the union of both squares enters; both are queued with FEATURES around each view and TERRAIN one ring further, 2 x 169 positions), `a_player_leaving_forgets_only_their_own_square` (either player leaving, an overlapping player whose shared chunks stay, a ticket that keeps its square), `chunks_go_nearest_to_any_player_first` (send order by the nearest player, generation order non-decreasing from both centres, no overflow 1,000,000 chunks apart).
  - `minecraft_terrain` (real 26.3 data, checked to run on it): `terrain::chunks_load_around_every_player_and_only_the_hosts_are_drawn` (End, view distance 2, host at chunk 6,0 and friend at chunk 106,0: both squares reach the scene, only the host's are in `Sections` or lit; when the host moves to the friend, exactly the host's old square is forgotten and the friend's chunks are registered and lit).
  - `minecraft_survival`: `level::the_chunk_map_follows_every_player_standing_in_the_level` (a pending arrival wins over the old place, a player whose place is still in the old dimension is skipped, players are grouped by dimension).
  - `test -p minecraftoss-world --lib` 66, `test -p minecraft_terrain` 850 (4 ignored), `test -p minecraft_survival` 370 + 1, `test -p render_anim --lib` 74, `check -p launcher` green.
- **Decisions:**
  - **Every player's view uses the server view distance.** Vanilla clamps each player's view to the server's; friends do not send their own yet.
  - **Ticketed chunks outside the host's view are loaded but not drawn on the host.** Before E9 the host drew every chunk the chunk map loaded, tickets included. A vanilla client is only sent the chunks in its own view, so this is the vanilla behaviour, and it keeps the host from meshing and lighting the area around a far friend.
  - **A friend is followed at their pending arrival, else their place,** the same position the chunk sender already follows (`Sessions::select_batches`), so the chunks loaded for a friend are the ones sent to them.
  - **A level with no player in it schedules nothing,** as before with no view; closing an empty level is unchanged.
  - **The host's own early returns are untouched** (switching level, credits): while the host is mid-switch no `stream_tick` runs, as before. E10a turns them into "skip this player's step". Travel is still locked while a friend is online.
- **Test steps:** `docs/round8/e-m2-tests.md`, section E9.
- **Kit:** none needed.

### E10a. The per-player step runs for the host by client number (pure move)

Commit: see `git log` ("The per-player step runs for the host by client number"). The previous commit is `da5b8685`.

- **Changed:**
  - `crates/render_anim/src/minecraft_players.rs` (new): the host's per-player rules, moved out of `minecraft_world::update` and keyed by an MW2 `ClientId` and the level's origin instead of `local`:
    - `begin`: the remote-killstreak flag, leaving a level, the arrival at a new level (respawn block, arrival place, portal travel sound, placement, abilities, the End platform), the MW2 life watch with the respawn, placement, the End step and the elytra. It returns `Begun::{Live, Left, Arriving, Hidden}`; the host does its own presentation for each (the switch screen, `leave_level` with its open screen, the voxel switch, the spawn-gate stream, the clocks), exactly as before.
    - `life`: alive and dead, team switches, keep-inventory, rules and game mode, abilities, creative flight, the death watch, the survival respawn or the MW2 health observation, the totem pop and the deaths that name a killer.
    - `movement`: footsteps, falls, the hand clock and cooldowns, breathing, hazards, Frost Walker and the totem check. It returns what the HUD needs (`holding_item`, `empty_hand`, cooldowns) instead of writing it.
    - `act`: the whole hand action block (punch, eat, mine by hand, creative breaking, stations, doors, buckets, throws, placement and the rest), with the host's Bevy input and screens passed in as `HostHand`. It returns the stations to open and the mined positions.
    - `struck` (mob hits with the shield, finished item uses, command teleports, pearl landings, pushes, potions) and `settle` (the totem hold-off and the MW2 health settle, which returns the death message instead of showing it).
    - `VoxelQueues`: the process-global queues `take_minecraft_deaths`, `take_totem_pops` and `take_team_switches` are drained once per frame into maps by client. Each player's step takes only its own entries, and an entry waits until that player's step takes it. Entries of a client with no Minecraft player are dropped.
    - The helpers only the moved code used (`finish_item_use`, `hold_off_death`, `scene_block`, `obstacles`, `pad_trigger`) moved with it. The moved code's comments were dropped (fork rule: no comments in new files). The file passes `rustfmt --check`.
  - `crates/render_anim/src/minecraft_world.rs`: `update` calls those functions in the same order as before, between its own presentation steps; the constants and helpers both files use are `pub(crate)`. The mining block, the container tick, the entity tick, the inventory UI and the presentation stay in `update`.
  - `crates/sim/src/voxel.rs`: `player_damage_pending(client)` is per client, so one player's queued damage no longer holds back another player's observe and settle (section 6.1 step 3).
- **Tests:**
  - `render_anim`: `minecraft_players::tests::a_death_queued_for_one_client_waits_for_that_clients_own_step` (client 0 steps first and takes only its own death; client 1's death, totem pop and team switch survive another drain and are taken once), `queued_entries_of_a_client_without_a_minecraft_player_are_dropped`.
  - `sim`: `voxel::tests::damage_pending_for_one_player_leaves_the_others_free_to_settle`.
  - `test -p render_anim --lib` 76, `test -p sim` 79, `check -p launcher` green.
- **Decisions:**
  - **The host's step still reads its own MW2 state from the presented snapshot** (`PresentedSnapshot::player(local)`), as before; only the code moved. The step takes the `PlayerState` as an argument, so a friend's step (E10b) passes the one from `AuthorityWorld`. Switching the host to the authority state would aim placement one tick behind the crosshair the host sees, which is a behaviour change, not a move.
  - **The early returns now return a `Begun`** and the host's frame still stops on them, as before. In E10b a friend's step that is skipped never stops the host's frame or another player's step.
  - **The commands, the dragon fight tick and the portal step stay in `update`** between `begin` and `life`: the commands are the host's chat lines, the dragon fight ticks the level once per frame, and the portal step writes the host's overlay. Keeping them where they were keeps the order unchanged.
- **Test steps:** `docs/round8/e-m2-tests.md`, section E10a (single-player regression, one window).
- **Kit:** `minecraft:cooked_beef` x8, `minecraft:cobblestone` x32, `minecraft:crafting_table` x1, `minecraft:chest` x1, `minecraft:shield` x1, `minecraft:skeleton_spawn_egg` x1, `minecraft:ender_pearl` x4, `minecraft:totem_of_undying` x1, `minecraft:white_bed` x1, `minecraft:obsidian` x10, `minecraft:flint_and_steel` x1, `minecraft:elytra` x1, `minecraft:firework_rocket` x8.

### E10b. Every player has their own hearts, hunger, XP and respawn

Commit: see `git log` ("Every player has their own hearts, hunger, XP and respawn"). The previous commit is `cb4251e9`.

- **Changed:**
  - `crates/render_anim/src/minecraft_players.rs`: `step_friend` runs E10a's step for every friend with a live MW2 client in the host's shown level, reading their `PlayerState` from `AuthorityWorld`: `begin` (remote-killstreak flag, the MW2 life watch with the respawn, placement through `place_friend`, which waits for the chunks around an arrival, the End platform and the elytra), `life`, `movement` (footsteps, falls, breathing, hazards, Frost Walker), `settle` (totem hold-off, the MW2 health settle, the death message), the death drops, the guns on the hotbar (`sync_weapons`) and the follow-MW2 rule, and the totem flag. A friend's step plays no sounds (`sounds` is `None`); its chat lines (a missing respawn point) go to that friend. `begin` takes the spawn gate as an `Option`: the host passes its own, a friend's step passes none. `drop_on_death` and `owned_weapons` are shared by the host and the friends.
  - `crates/render_anim/src/minecraft_host.rs`: the host system calls `step_friend` for each session (replacing M1's `observe_life` and `place_friend` loop), reads `ToServer::SelectSlot` (newest wins, applied at the friend's next step), sends a friend's death message to everyone (vanilla's `show_death_messages`), clears a leaving friend's totem flag and sends each friend's `PlayerView` on change.
  - `crates/render_anim/src/minecraft_world.rs`: the voxel queues keep the entries of every player's client, not only the host's; the host's own death message also goes to the friends (`relayed`); `HostView` carries the queues, the shape tables, the packs and the language for the friends' steps.
  - `crates/minecraft_survival/src/views.rs` (new): `player_view(player)` builds the `PlayerView` (health, absorption, food, saturation, air, fire, armour points, XP, game mode and abilities, effects, the 41 slots with the MW2 gun items, the selected slot, cooldowns and the item in use); `sent_key` is the view without the countdowns (effect durations, cooldowns left, use ticks), so a running effect is not sent every tick; `view_stack` turns a slot back into a stack; `HudClock` rebuilds the HUD's vitals on the friend's game (the damage blink, the settling shown health and the regeneration wobble, with the host's constants); `Selection` predicts the friend's hotbar slot until the host echoes it.
  - `crates/minecraft_survival/src/session.rs`: `Session::send_view` and `Sessions::send_views`; M1's `Session::observe_life` is gone (the step uses `Player::observe_life`, which has the same "the first spawn is the join" rule; its test now checks `Player::observe_life`).
  - `crates/minecraft_survival/src/guns.rs`: `sync_weapons` and the follow-MW2 rule (`Follow`) moved here from `InventoryUi`, so they run per player; `InventoryUi` keeps the host's `Follow`, and `Player.follow` holds each friend's.
  - `crates/minecraft_survival/src/respawn.rs`: while travel is locked (a friend online), a respawn whose point is in another dimension stands at this level's spawn and tells the player travel is off (`refuse_travel`), instead of switching level.
  - `crates/minecraft_survival/src/cooldowns.rs` (`entries`), `survival.rs` (the blink constants are `pub(crate)`), `player.rs` (`follow`).
  - `crates/render_anim/src/minecraft_remote.rs`: the friend's game loads the recipe book (for stack sizes and durability bars), decodes its `PlayerView` slots into an inventory and publishes it through its own `InventoryUi` (hotbar, icons, armour), sets the hearts, hunger, air, XP and fire overlay from the view with `HudClock`, and makes the HUD active while the friend is alive. The hotbar selection is predicted with `Selection` and sent as `SelectSlot`; the selected item drives `holding_item`, the bare hands and the `weapon_request` of the friend's own usercmd, with the same follow rule. The inventory screen stays closed and drops, hand swaps and clicks are dropped until E13 and E14.
- **Tests:**
  - `minecraft_survival`: `views::tests::two_players_lose_hunger_and_health_on_their_own`, `the_view_carries_items_and_guns_and_they_come_back`, `only_a_change_that_shows_makes_a_new_key`, `the_hud_blinks_on_damage_and_settles_the_shown_health`, `a_picked_slot_waits_for_the_host_and_an_mw2_switch_is_followed`; `session::tests::a_player_view_is_sent_once_for_each_change`, `a_friend_respawns_only_after_a_life_has_been_seen` (now on `Player`); `respawn::tests::each_player_respawns_at_their_own_point`, `a_respawn_in_another_dimension_stays_in_this_level_while_travel_is_off` (both on real 26.3 data, checked to load); `guns::tests::guns_follow_the_mw2_weapons_and_a_lost_gun_leaves`, `a_picked_gun_is_asked_for_and_an_mw2_switch_moves_the_selection`.
  - E10a's tests cover "a death queued for client 1 is not lost when client 0 steps first" and "one player's pending damage does not skip the other's settle".
  - `test -p minecraft_survival` 380 + 1, `test -p render_anim --lib` 76, `check -p launcher` green. `the_end::dragon_fight::tests::the_first_visit_scans_once_builds_the_podium_and_saves_the_fight` failed once in the full run while other builds loaded the PC (the dragon had already flown below y 100 when the test looked) and passed alone and in the next full run; it does not touch this change.
- **Decisions:**
  - **The host's own HUD keeps its direct path** (`survival.vitals()` and `InventoryUi`) instead of being rebuilt from its `PlayerView`, so the single-player HUD stays exactly as it was; the friend's HUD is built from the view with the same blink and settle constants.
  - **A friend's step runs in the host system, after the host's own frame,** because the sessions live there. It runs even when the host's own step is skipped (switching, waiting at the spawn gate); the level tick itself still runs only in the host's frame until E11.
  - **A friend's mob hits, item uses, pearls, pushes, potions, picked items and XP orbs** still come only from the host's entity tick, which knows one player (E11 and E12). A friend's XP changes with `/xp`, deaths and the level-up rules; picking up orbs comes with E11.
  - **A friend's kill card (`KilledBy`) and a friend's own sounds** are not sent yet; the queued deaths and totem pops of a friend are still taken by their own step, so they never pile up.
  - **Respawning into another dimension while travel is off** stands the player at this level's spawn and keeps their respawn point, the most vanilla-like thing short of travelling; the travel-off line tells them why.
  - **A friend's hotbar slot is predicted** (vanilla sends `ServerboundSetCarriedItemPacket` and keeps the slot locally); a slot the host never echoes is dropped after one second, and an MW2-initiated switch on the host is followed from the next view.
  - **The friend's own view of the inventory screen stays closed** until menus (E14), so no click is ever lost on a screen the host does not run.
- **Test steps:** `docs/round8/e-m2-tests.md`, section E10b (two windows).
- **Kit:** `minecraft:cooked_beef` x8, `minecraft:cobblestone` x32, `minecraft:white_bed` x1, `minecraft:torch` x16.

### Final verification of E9, E10a and E10b: clippy on the track's own lines

Commit: see `git log` ("Multiplayer M2 per-player step passes clippy"). The previous commit is `4a6c0dc7`.

- **Clippy** on the crates touched since `79ba1549` (`minecraft_survival`, `minecraft_terrain`, `render_anim`, `sim`, `minecraftoss-world`) with `--all-targets -A clippy::approx_constant`. Two warnings were on lines the track added, both in `render_anim::minecraft_players`; older warnings stay.
- **Changed (no behaviour change):**
  - `VoxelQueues::take_deaths`: `unwrap_or_default()` (refused by the workspace `clippy.toml`) became `into_iter().flatten().collect()`, which returns the same empty list for a client with no queued death; `unwrap_or(Vec::new())` and `unwrap_or_else(Vec::new)` are refused by `clippy::unwrap_or_default`.
  - `begin`: the last use of the spawn gate passes `gate` instead of `gate.as_deref_mut()` (`needless_option_as_deref`), and the call fits on one line as rustfmt wants for the new file.
- **Tests:** none new. `test -p` of each touched crate is green: `minecraft_survival` 380 + 1, `minecraft_terrain` 850 (4 ignored), `render_anim` 76, `sim` 79, `minecraftoss-world` 66. `check -p launcher` is green.
- **Test steps:** none new; sections E10a and E10b of `docs/round8/e-m2-tests.md` cover these lines (E10a step 8 and E10b step 9: a death message in both windows and each player's respawn at their own point).
- **Kit:** none needed.

### Merge of minecraft-survival (track A) into track E

Commit: the merge commit "Merge minecraft-survival (track A) into track E". Its first parent is `b264b59d`, its second `080b5c1e` (track A merged into `minecraft-survival`).

- **Conflicts and how they were resolved** (both sides' behaviour kept):
  - `crates/minecraft_commands/src/commands.rs`, `suggest.rs`: the command lists hold both E's `/tell` alias and A's `/tick`; the `/t` suggestion test expects `teleport`, `tell`, `tick`, `time`, `tp`.
  - `crates/minecraft_survival/src/server.rs`: `Globals` keeps E's `writer: SaveWriter` (the save thread with the quick-rejoin copy) and A's `autosave_due` (the autosave countdown lives in A's `TickRate`). A's `level_writer` is gone with E's writer; A's `autosave` and `plan_ticks` queue on the writer. E's test servers (`friend_server`, `remote.rs`, `session.rs`) build `pending_batch: None`.
  - `crates/minecraft_survival/src/commands.rs`: `run` keeps E's recipient-tagged `Outgoing` lines (so every player gets their own) and adds A's `/tick sprint` reports as `server_feedback` lines, which E's routing hands to every operator, like vanilla's `sendSuccess(..., true)` from the server source. A's `report_sprints` stays for its own test.
  - `crates/minecraft_survival/src/level.rs`: both new tests are kept (E's `a_chunk_sent_after_edits_from_every_source_matches_the_scene`, A's `tick_until` and `an_eye_thrown_from_the_hand_leaves_the_stack_and_flies`).
  - `crates/render_anim/src/minecraft_world.rs`: the frame keeps E's calls into `minecraft_players` (`begin`, `life`, `movement`, `act`) and A's tick schedule around them: `plan_ticks` before the autosave, `autosave_due`, the dragon fight by the planned ticks, `pearls_through`, the containers by `Level.container_ticks`, and `entities.tick` with the pending batch and `client_timing`. E's arrival branch lost its `advance_clocks(dt)` (the clocks now move in `plan_ticks`, as A made them), and so did the end of the frame. The cracks come from `hand.dig.crack()`. A's `player_motion` and `arrival_velocity` (the arrival keeps the departure's speed) sit beside `place_player`, which `place_friend` calls too, so a friend's arrival also gets them.
  - **What the merge broke without a conflict:** E made `TerrainStream.server` an `Option<ChunkMap>` (a friend's stream has none) and gave `server_tick` every player's block. A's new `TerrainStream::write_states` (the gateway's blocks) now loads through `load_chunk_now` and sets the blocks through the host's chunk map (`expect(HOST_ONLY)`, as the other host-only writes), and A's tests in `terrain.rs` and `the_end/gateway.rs` use the optional map and pass one player to `server_tick`. The frame's unused `hand_ticks` binding is gone (the containers count `container_ticks` now).
  - **A's hand changes carried into `minecraft_players::act`** (E10a moved the hand block there): vanilla digging (`hand.dig` with `DigInput`, the mining speed of `mining_speed::destroy_progress`, hit sounds and particles, `hand.dig.press()` on a click at a block, the `!digging` stop at the end), the attack speed multiplier, `block_press` for creative breaking, the eye of ender thrown from the hand (`throw_use`, `hand.using_eye`, `offhand::eye_held`) and `frame_use` for frames, and spawn eggs on spawners (`EggUse::Spawner`, `egg_on_spawner`). `Acted` returns `hand_broken` for the frame's `break_now`, the wrong-tool drops and the tool wear, which stayed in `minecraft_world` and merged cleanly. `Moved` carries `eye_in_water` for the digger. `act` no longer takes the voxel events, because the hand no longer mines by `Shot` events.
- **Tests:** none new (both sides' tests are kept). `test -p minecraft_commands` 130, `minecraft_net` 15, `minecraft_survival` 444 + 1, `minecraft_terrain` 922 (3 ignored), `render_anim` 78, all green on the real 26.3 data; `check -p launcher` green. `minecraft_players.rs` and `the_end/gateway.rs` still pass `rustfmt --check`.
- **Decisions:**
  - **Only the host's level is in `ticking`,** as A wrote it: the level tick still runs in the host's frame until E11, which builds the server tick for every occupied level on A's `plan_ticks` and `take_batch`.
  - **A friend's own clock prediction still runs at 20 ticks a second** between the host's clock updates; under `/tick freeze` or `/tick rate` the friend's sky is corrected by the next update. Sending the tick rate and the frozen state (vanilla's `ClientboundTickingStatePacket`) belongs with E11's server tick.
  - **A friend does not dig yet:** `act` runs for the host only, as before the merge; a friend's hand (with A's digging) comes with E13.
- **Test steps:** `docs/round8/e-m2-tests.md`, section "Merge of track A".
- **Kit:** `minecraft:stone_pickaxe` x1, `minecraft:ender_eye` x4, `minecraft:cobblestone` x32.

### E11. The level thread serves every player, not player 0

Commit: see `git log` ("The level thread serves every player, not player 0"). The previous commit is `8230328a` (the merge of track A).

- **Changed:**
  - `crates/minecraft_terrain/src/server.rs`, built on track A's batches (`run_tick` per tick, `finish_batch` once per batch):
    - `TickInput.players` is a list of `TickPlayer { id, feet, alive, main_hand, pickup }`, which replaces the old feet list, the single `pickup` and the held item read from it. `simulation_centres` replaces `simulation_center`, and `player_hurts` carries the hurt player: `(player, source, damage type)`.
    - `run_tick` runs `tick_player`, `player_hurt` and `set_player_main_hand` for each player. Each player with a `pickup` takes the items, then the tridents (with their own body, not the first one), then one orb, in list order (vanilla's `Player.aiStep` touch, player after player).
    - `take_xp_delay` is a map by player, and so are the trading menu's `merchant_closing` and `check_merchant` (every player with an open trading screen is checked). `mob_action` and `merchant` take the player, and `merchant_view(0)` is gone. `mob_actions::Actor` carries the player, so feeding, taming and trading act for that player (`villager_interact`, `wolf_interact`).
    - `set_simulation_areas(centres, distance)`: random ticks and the ticking chunks are the union of every player's square, each chunk once, in X then Z order. With one player this is the same list as before. `set_simulation_area` calls it with one centre, so the existing callers are unchanged.
    - **Commands carry the player:** `ItemOnBlock`, `Summon`, `HatchEgg`, `SpawnerEgg`, `MobAction`, `Merchant` and `BottleBreath` gain `player`, and `ThrowEye` already had its `owner`.
    - **`Output` carries the player:** `picked`, `orbs_taken`, `item_uses`, `summoned`, `command_summons`, `merchant` and `mob_results`.
    - `Command::PlayerLeft(id)` and `ServerHandle::player_left`. The player is taken out of a batch that is still running (`TickInput::remove_player`: players, mob candidates, views, vitals and hurts), so the ticks left in it pick nothing more for them. What they picked before is already in the output being built, and goes out with it. The player's XP delay, trading state, last feet, fights and held item (`EntityWorld::forget_player`, new in the vendored `minecraftoss/entities/src/world.rs`) are dropped. An open trading screen is closed, and its payments drop at their last feet.
    - `weather` follows the first player (the host) while alive, as before.
  - `crates/minecraft_survival/src/entities.rs`:
    - The host's tick hands the level thread one `TickPlayer` (id `PLAYER`, the held item only while it picks up, as before).
    - The host takes only its own picks, orbs, item uses and egg answers.
    - `/summon` passes its source as the player id. The `summoners` queue is gone, because the answer names the source.
    - `settle_player` sends `PlayerLeft` for every leaving player.
    - `take_late` takes only the leaving player's late picks.
- **Tests:**
  - `minecraft_terrain` `server::tests`, all on real 26.3 data:
    - `two_players_pick_up_their_own_items_and_orbs`: each gets their own stack and orb, in their own inventory copy.
    - `each_player_waits_out_their_own_xp_delay`: both take an orb on ticks 0 and 2. With one shared delay, the second player would wait.
    - `a_zombie_targets_the_nearer_player`: 3 blocks against 7, with the farther player listed first.
    - `a_hurt_on_player_one_sends_only_player_ones_wolf_after_the_mob`: two owners and two tame wolves. Only player 1's wolf targets the zombie that hurt player 1.
    - `random_ticks_run_around_every_player`: two squares 10 chunks apart, and overlapping squares tick each chunk once.
    - `a_player_who_leaves_keeps_the_picks_made_and_picks_nothing_more`: the picks and the orb before the leave are kept, the running batch drops the player, and their state is cleared. It also runs a batch, `player_left` and a further tick through the thread.
  - Existing tests follow the new shapes. `command_summons_do_not_reach_the_egg_results` and `the_server_answers_each_item_use_so_one_source_fills_one_bucket` now also check that each answer names its player, and the eye of ender answers name their owner.
  - `minecraft_survival`: `a_leaving_player_gets_the_late_picks_and_loses_their_pearls_and_pushes` also checks that another player's late pick stays.
  - Results: `test -p minecraft_terrain` 928 passed (3 ignored), `test -p minecraft_survival` 444 + 1, `test -p minecraftoss-entities --lib` 83, `check -p launcher` green.
- **Decisions:**
  - **`UseBlock` carries no player.** The level's block use reads no player, and its only output (the dragon egg's teleport) is level-wide, so a player field would never be read. `BoneMeal` is unchanged too: friends' item uses come with E13c.
  - **The tracker inputs stay as they are:** `TickInput.tracking` and `tracked_mobs` are still the host's mob mirror. Tracking for each friend is section 9.2 (M3).
  - **`mob_players`, `mob_views` and `mob_vitals` stay separate lists.** They were already keyed by player and the entity world already reads them for every player, so `TickPlayer` holds only what was player 0's alone.
  - **A disconnected player's trading payments drop at their feet.** This is vanilla's `AbstractContainerMenu.clearContainer` for a player who `hasDisconnected()`. The level thread has no inventory of theirs to return them to.
  - **The host still hands only itself to the level thread.** Friends become `TickPlayer`s, with their own picks, orbs, mob hits and kill credit, in E12 (`Entities::tick` with every player in the level, `PlayerOutputs` per player). Until then, outputs for other ids are left alone, and none arrive.
- **Test steps:** `docs/round8/e-m2-tests.md`, section E11 (the host's pickups, orbs, tame wolf and crops as before, and a friend leaving).
- **Kit:** `minecraft:stone_pickaxe` x1, `minecraft:cobblestone` x32, `minecraft:experience_bottle` x8, `minecraft:wolf_spawn_egg` x1, `minecraft:bone` x8, `minecraft:dirt` x4, `minecraft:wooden_hoe` x1, `minecraft:wheat_seeds` x4.

### E12. Mobs, items and XP treat every player alike

Commit: see `git log` ("Mobs, items and XP treat every player alike"). The previous commit is `3caf3ef0` (E11).

- **Changed:**
  - `crates/minecraft_survival/src/entities.rs`:
    - `const PLAYER` is gone. Throws, tridents, punches, end hits, fireball deflects, bullets (`shoot`), blasts, item uses on blocks, eyes of ender, dragon's breath bottles and spawn eggs all take the acting player's id. `blast` takes the credited player as an `Option<u64>`.
    - `set_player_uuid` became `set_players(&[(u64, PlayerUuid)])`. It sends a player's UUID to the level thread only when it is new or changed, so the host frame can call it every frame.
    - `Entities::tick` takes `&mut [TickActor { id, view, inventory, selected }]`, one per player in the level. Every actor becomes a `TickPlayer`, a mob candidate, a mob view, a mob vitals entry and a simulation centre. Each actor's picked stacks go into that actor's own inventory, and egg answers spend that actor's egg. The first actor is the one this game shows: its pickups animate, the pop and the mob sounds are heard from its feet, and the mob mirror is tracked around it. `tick` returns only the block changes now.
    - The per-player queues live in `PlayerQueues`, a map of `PlayerQueue` by id: the picks still to go into the inventory, the end gateway pearls, the orb XP, the item use waiting for its answer, the eggs waiting and their answers, the feet the last tick saw, and `PlayerOutputs` (hits, pearl landings, potions, pushes and finished item uses). `PlayerQueues::route` sorts each level output by player: picks, orbs, pearls, gateways, potions, pushes, item use answers, egg answers, trident strikes and mob hits (with the direction worked out from that player's feet). Outputs for an id with no queue, such as a player who has left, are handled safely: picked stacks are spawned again where they were picked up, taken orbs are given back to the world as orbs at the same place, and everything else is dropped.
    - New accessors take the place of the old public fields: `take_player_outputs(id)`, `player_outputs(id)`, `take_experience(id)`, `take_pearl_gateways(id)`, `land_pearl(id, landing)` and `awaits_item_use(id)`.
    - `leave(id, inventory, selected, feet)` replaces both `leave` and `settle_player`, for any player. It sends `PlayerLeft`, waits for the level to go idle, takes that player's late picks and orbs from the outputs not yet read, applies their egg answers, puts the picks into the inventory (what does not fit drops at their feet), removes their queue and returns the XP they still had coming. Other players' queues are untouched.
  - `crates/minecraft_survival/src/level.rs`: `spill` gives a broken furnace's stored XP back as orbs at the block's centre. Before, it went straight to the host's XP bar.
  - `crates/minecraft_survival/src/server.rs`: a leaving friend gets the XP that `leave` returns before their file is saved.
  - `crates/minecraft_survival/src/the_end/gateway.rs`: `pearls_through(level, player)` works on one player's pearls.
  - `crates/render_anim/src/minecraft_world.rs`:
    - The host frame splits the players into the host and the friends, so the host's step and the friends' inventories can both be borrowed.
    - The entity tick gets the host and every friend in the level (`tick_actor`: friends who are not travelling or arriving and have an MW2 body), and `set_players` with all of them.
    - A `MobBlast` credits the player behind the attacker's client: the host, or a friend in this level, and nobody otherwise. A `MobShot` still credits the host until E13a gives it an attacker.
    - The host's XP comes from `take_experience` plus the furnace output. Leaving a level for travel uses `leave` and keeps the XP returned.
  - `crates/render_anim/src/minecraft_players.rs`:
    - `act` passes the player's id to every entity call.
    - `struck` takes that player's `PlayerOutputs` itself. The hits are no longer a parameter.
    - `tick_actor` builds a friend's actor from their MW2 state: feet, look, alive, game mode and health.
    - `step_friend` now runs the friend's gateway pearls, `struck` (mob hits with shield and armour, pearls, pushes, potions and item uses) and the friend's orb XP.
  - `crates/render_anim/src/minecraft_end.rs`: `pearls_through` passes the player's id.
- **Tests:**
  - `minecraft_survival` `entities::tests`:
    - `a_skeleton_arrow_hurts_the_friend_it_hit_and_not_the_host`: the arrow lands in the friend's hits with its direction and the skeleton as the cause. The host's hits stay empty.
    - `a_friends_picks_orbs_pearls_pushes_and_item_uses_reach_only_the_friend`: every kind of output reaches only the friend, the trident's direction comes from the friend's feet, and the host's own item use keeps waiting.
    - `outputs_for_a_player_no_longer_in_the_level_go_back_to_the_world`: a late pick and a late orb of a player who left come back as unclaimed at the place they were taken, and the rest is dropped.
    - `a_friends_kill_gives_the_friend_the_experience` (real 26.3 data): a cow is summoned at the friend's feet on a stone floor, the friend shoots it, and the friend takes 1 to 3 points while the host, 35 blocks away, takes none. It also checks that `set_players` sends each UUID once.
    - `a_host_drop_reaches_the_level_and_the_friend_picks_it_up` (real 26.3 data): a drop the host's game spawned goes to the level thread and lands in the friend's inventory, not the host's. Then `leave(FRIEND)` removes only the friend's queues.
  - Existing tests follow the new shapes. `a_leaving_player_gets_the_late_picks_and_loses_their_pearls_and_pushes` also checks the late orbs. `mobs_leave_creative_and_spectator_players_alone` checks the candidate ids. The eye of ender, end gateway pearl and dragon fight tests use the per-player accessors and actors.
  - Results: `test -p minecraft_survival` 449 + 1, `test -p render_anim --lib` 78, `check -p launcher` green.
- **Decisions:**
  - **A friend's picks go straight into the friend's inventory during the host's entity tick,** like the host's. The host frame borrows the friends' players next to the host's for that. Hits, pearls, pushes, potions, item uses and XP wait in the friend's `PlayerOutputs` until the friend's own step, which applies them with the friend's shield, armour and MW2 body.
  - **Only the host's own pickups show the pickup animation and the pop on the host's screen.** A friend's pickup makes the item vanish there. The flying item and the sound for other players come with the item tracking and sounds of M3 (E16, E17).
  - **Taken orbs of a player who has left are dropped as orbs again** where they were taken, as their picked stacks are. This keeps the XP in the world rather than losing it.
  - **A broken furnace's XP is dropped as orbs at the block's centre.** This is vanilla's `AbstractFurnaceBlockEntity.preRemoveSideEffects` (`getRecipesToAwardAndPopExperience` at `Vec3.atCenterOf`, checked with javap). Before, the level handed it to the host, which would be wrong once anyone can break the furnace.
  - **Leaving a level for travel now waits for the level to go idle,** as a friend's leave does. Late picks and orbs that were still in flight are no longer lost on a trip.
  - **Bullets still credit the host** until E13a adds the attacker to `MobShot`. Kill XP drops for any player's kill, so it already goes to whoever picks it up.
- **Test steps:** `docs/round8/e-m2-tests.md`, section E12 (two windows: the friend picks up a block the host broke, takes the orbs of thrown bottles, gets a grenade kill's loot and XP, and is shot by a skeleton while the host stays unhurt).
- **Kit:** `minecraft:stone_pickaxe` x1, `minecraft:cobblestone` x32, `minecraft:experience_bottle` x8.

### E13a. Voxel events say which level and which player they belong to

Commit: see `git log` ("Voxel events say which level and which player they belong to"). The previous commit is `0582018d` (E12).

- **Changed:**
  - `crates/sim/src/voxel.rs`: every `VoxelEvent` carries `level: u8` (`ONLY_LEVEL`, 0, until the bands of E19). `Shot`, `MobShot` and `Ray` carry the `attacker` client. The events are built by `VoxelWorld::shot`, `explosion`, `ray`, `mob_shot` and `mob_blast`, which the `push_*` functions call with the global world, so a test can build them from its own world.
  - `crates/sim/src/combat.rs`: bullets pass their shooter to `push_mob_shot`, `push_ray` and `push_shot`, and the melee hit passes its attacker to `push_shot`.
  - `crates/render_anim/src/minecraft_world.rs`:
    - A `MobShot` credits its attacker, the host or a friend in this level, with that player's own yaw for the knock-back, through the same `credit` as `MobBlast`. A shot by nobody in this level (a bot) hurts no mob.
    - A `Shot` or `Ray` mines only when its attacker is a player in this level who may build. Before, the host's game mode decided for every bullet, so a host in adventure mode also stopped a friend from mining.
  - `crates/render_anim/src/minecraft_mining.rs`: the readers take the new fields with `..`. Mining progress, drops and block XP still go through the host; the attacker decides them in E13c.
- **Tests:** `sim` `voxel::tests::a_voxel_shot_carries_its_attacker_and_every_event_its_level` (a shot, a melee hit, a mob shot and a bullet's ray carry their attacker; all five kinds carry level 0; the shot's block is the one behind the struck face). `test -p sim` 80, `test -p render_anim --lib` 78, `check -p launcher` green.
- **Decisions:**
  - **`Ray` carries its attacker too,** not only `Shot` and `MobShot` as the design lists: a bullet's ray breaks grass and flowers, so it needs the same game-mode check and, in E13c, the same drops by the shooter.
  - **`Explosion` keeps no attacker.** MW2 blasts on blocks are level-wide (vanilla's explosion loot has no tool); the blast on a mob already credits its thrower through `MobBlast`.
  - **A bullet from a client with no Minecraft player in this level does nothing to mobs or blocks.** Bots are not a goal (section 8.4), and a friend in another level cannot shoot into this one once bands land.
- **Test steps:** `docs/round8/e-m2-tests.md`, section E13a.
- **Kit:** none needed.

### E13b. The action rules live in minecraft_survival (pure move)

Commit: see `git log` ("The action rules live in minecraft_survival"). The previous commit is `e84116e7` (E13a).

- **Changed (no behaviour change for the host):**
  - `crates/minecraft_survival/src/actions.rs` (new): the hand's rules, moved from `render_anim::minecraft_players::act` (which E10a had moved out of `minecraft_world`). `act(player, level, globals, body, actor, hands, moved, light, now)` punches, eats, digs (A's digging), breaks in creative, opens tables, stations and benches, uses blocks and doors, campfire food, beds and respawn anchors, bottles, buckets and the other item uses, eyes of ender, spawn eggs, tridents, throws, planting and placement (doors, scaffolding, shulker boxes), exactly as before. What it read from Bevy, MW2 or the host's screens now comes in as plain values, and what it played comes back:
    - `Hands`: the held buttons, their presses this frame, the offhand key and whether a screen has the input (`captured`).
    - `Actor`: the MW2 state as Minecraft values (feet, MW2 eye, yaw, pitch, velocity per tick, gravity, crouching, swimming, sprinting, gliding, MW2 offhand busy).
    - `Moved` (moved here from `render_anim`): what the step's movement found.
    - `Body`: the MW2 side, which `render_anim` implements (`MwBody`): block writes with their collision shapes, the bodies that block a placement, the Riptide push and the rocket launch.
    - `Acted.effects`: `Effect::Sound` (block space, with the door's pitch spread), `Crumbs`, `HitParticle`, `Chat` and `Overlay`, in the order they happened.
    - The respawn block use (`use_block`, `use_anchor` and the game half of `finish`) moved here from `render_anim::minecraft_respawn`, which keeps the overlay.
    - `SWING_TICKS` and `RECENT_PLACE_SECONDS` live here; `render_anim` uses them.
    - New for E13c: `reaches_block`, `reaches_box`, `hit_on_block` and `allows(target, eye, mode, mob_box)`, vanilla's checks on a client's target: a block within the interaction range + 1 of the eye (`Player.isWithinBlockInteractionRange`, the distance to the block's box), a hit point within 1.0000001 of the block's centre on every axis (`ServerGamePacketListenerImpl.handleUseItemOn`, both checked with javap), a mob within the entity range + 3. A player target is refused (Minecraft melee on players is deferred).
  - `crates/render_anim/src/minecraft_input.rs` (new): `host_hands` samples the mouse and the gamepad triggers into `Hands` (the `pad_trigger` helper moved here), and `play` presents `Effect`s: sounds through `Sounds`, crumbs and hit particles through `Mining`, chat lines and the overlay through the host's chat with its language.
  - `crates/render_anim/src/minecraft_players.rs`: `act`, `HostHand`, `Acted` and `Moved` are gone; `actor(ps, origin, alive)` and `MwBody` are new.
  - `crates/render_anim/src/minecraft_world.rs`: the frame calls `host_hands`, `actions::act` and `play` where it called `act`.
  - `crates/render_anim/src/minecraft_elytra.rs`: `launch_rocket` is gone (`Body::launch_rocket` does it).
- **Tests** (`minecraft_survival` `actions::tests`):
  - `reach_is_refused_beyond_five_and_a_half_blocks`: survival reaches a block 4.5 away and not 5.5; creative reaches 5.5 and not 6.5; a mob 5.2 away is reached and one 9.2 away is not; an unknown mob and a player are refused.
  - `a_hit_point_off_the_face_is_refused`: hit points within 1 of the centre pass, 1.1 and 1.6 away fail.
  - On real 26.3 data (checked to run on it): `a_held_block_is_placed_on_the_face_the_player_looks_at` (the cobblestone lands, the stack drops to 31, the swing starts, the place sound is at the block; a second frame without a press places nothing), `a_door_opens_when_used` (the stack is untouched, the wooden door's open sound has its spread, and the level thread opens the door), `held_food_is_eaten_while_use_is_held` (chewing is heard, one beef is eaten and the food bar rises, holding on starts the next one; letting go stops eating).
  - `test -p minecraft_survival` 454 + 1, `test -p render_anim --lib` 78, `check -p launcher` green.
- **Decisions:**
  - **The effects are played right after `act` returns,** before the frame's bullet mining, so the hit particles still read the block they hit and the sounds keep their order.
  - **The debug line for chest placement** (`MINECRAFTOSS_DEBUG_SERVER`) prints the actor's crouching and swimming instead of the raw MW2 flags, which the rules no longer see.
- **Test steps:** `docs/round8/e-m2-tests.md`, section E13b (one window, the host's own hand as before).
- **Kit:** `minecraft:cobblestone` x32, `minecraft:oak_door` x2, `minecraft:cooked_beef` x8, `minecraft:water_bucket` x1, `minecraft:white_bed` x1, `minecraft:snowball` x16, `minecraft:stone_pickaxe` x1.

### E13c. Friends break, place and use blocks and items

Commit: see `git log` ("Friends break, place and use blocks and items"). The previous commit is `5febc5ed` (E13b).

- **Changed:**
  - `crates/minecraft_survival/src/session.rs`:
    - `HandQueue` on every `Session`: the friend's `HandEvent`s wait in order (a repeated or older `seq` is ignored, at most 256 wait) and `take` uses each one once, on the host frame that runs the friend's step. A press counts only when its target passes `actions::allows`, and holds its button from then on. A release lets go. `HandState` (newest `tick` wins) keeps a held button alive, and a button with no fresh press or state for 500 ms (`HELD_FOR`) counts as let go. A state never holds a button that a release let go. `SwapHands` and `Drop` come out of `take` as `HandTake.swap` and `drops`.
    - Cracks: `level_cracks` collects the level's bullet progress (breaker `BULLETS`) and every player's digging crack in that level. `Sessions::send_cracks` sends each friend the cracks within 32 blocks of them in chunks they have, as `ToClient::Crack`, only when one is new or changes stage, and `-1` when one goes.
    - Break events: `fan_out` sends each friend in the level `ToClient::LevelEvent { event: 2001 }` (vanilla's block break event) with the broken block's state for chunks they have. They are collected in `Terrain.friend_breaks`, only while a friend watches that level, as `friend_changes` is.
  - `crates/minecraft_survival/src/level.rs`: `Level.shots` holds the bullet mining progress of the level, keyed by block (it was the host's `Mining.progress`), and `shot_cracks` turns it into crack stages. `Terrain.friend_breaks` is new.
  - `crates/minecraft_survival/src/actions.rs`:
    - `harvest(player, level, globals, broken, dug, hand_mined)` is what the host frame did after a break, for any player: the exhaustion, the drops and block XP with that player's held item (silk touch, fortune, the wrong tool), shulker contents, the spill of containers, the openers and the tool wear. It returns the worn-out items and their break sounds, and records the break events for friends.
    - `Effect::Dig` replaces `HitParticle` and carries the dig's hit sound, which vanilla plays only for the player digging. `face_index` gives vanilla's direction number of a face.
  - `crates/render_anim/src/minecraft_mining.rs`: `apply` and `break_now` work on a level's `shots` and `apply` returns the attacker of each block a bullet broke. `crack_mesh` draws a list of cracks.
  - `crates/render_anim/src/minecraft_world.rs`: blocks broken by a friend's bullets are harvested for that friend (held item, XP, wear), the rest for the host as before. The host sees the cracks of the bullets, its own digging and every friend's digging in the level.
  - `crates/render_anim/src/minecraft_players.rs`: `step_friend` takes the friend's `HandQueue`. It takes the hand once per step (with the reach and hit checks from the friend's authoritative MW2 eye), swaps hands, drops the selected stack (guns stay), runs `actions::act` for the friend with their MW2 state (`MwBody`), breaks what they dug or broke in creative (with the break particles on the host) and harvests it for them. The host hears the friend's sounds where they happen, sounds at the listener at the friend's eye, but not the friend's dig hits. Chat lines and the overlay (beds, anchors) go to the friend.
  - `crates/render_anim/src/minecraft_host.rs`: `ToServer::Hand` goes into the friend's queue and `HandState` from the state lane too. A friend whose step does not run (no MW2 body, another level) has their presses dropped and releases kept. Cracks are sent after the block updates.
  - `crates/render_anim/src/minecraft_input.rs`: `HandSampler` turns the friend's mouse and gamepad into `HandEvent`s (a press and a release in one frame give both) and a `HandState` at most every 50 ms while a button is held. An open chat or console, or a dead friend, lets the buttons go.
  - `crates/render_anim/src/minecraft_remote.rs`: the friend's game samples its hand with the target from its own scene (block, face, hit point; mobs are not mirrored yet, so a mob is a miss) and sends both. It draws the cracks the host sends and the break particles of `LevelEvent` 2001, steps its own particles, and shows overlay lines (a bed refusing to sleep) above the hotbar.
- **Tests:**
  - `minecraft_survival` `session::tests`: `a_press_and_a_release_in_one_host_frame_both_act` (both edges, the swap and the drop come out; on real 26.3 data a click through the queue places one block over three frames), `a_press_is_used_once_however_many_host_frames_pass` (a repeated `seq` is ignored, nine more frames see no press and keep the button held, the release lets go), `a_held_use_without_a_fresh_hand_state_for_half_a_second_stops` (a repeated tick does not refresh, 500 ms without a state let go, a state after the release does not hold it again), `a_press_at_a_target_out_of_reach_is_refused`, `cracks_and_breaks_reach_a_friend_once_for_chunks_they_have`.
  - `render_anim` `minecraft_input::tests::a_click_sends_a_press_and_a_release_and_a_hold_keeps_its_state_coming`.
  - `minecraft_mining::tests::real_blocks_crack_on_their_own_model_shapes` draws from `shot_cracks`.
  - `test -p minecraft_survival` 459 + 1, `test -p render_anim --lib` 79, `check -p launcher` green.
- **Decisions:**
  - **The host aims a friend's hand from the friend's authoritative MW2 eye and view angles,** exactly as it aims its own, so every rule runs with one code path and nothing is trusted from the friend's game. The `Target` in a press is the vanilla check only: a block within the interaction range + 1 and a hit point within 1.0000001 of the block's centre (`allows`). A press that fails it is dropped. A mob target always fails until mobs are mirrored on the friend's game (M3), and a friend's game sends a miss for them.
  - **The host feeds its own hand as `Hands` straight from Bevy,** not through a `HandQueue`; the queue only carries what crosses the wire. Both run `actions::act`.
  - **A friend's bullet breaks count for the friend** (held item, XP, wear, exhaustion), the last shot that broke the block decides. Blocks broken by nobody in the level stay the host's.
  - **What the host plays for a friend's action:** positional sounds (placing, breaking, doors, buckets) where they happen, listener sounds (eating, throwing) at the friend's eye, no dig-hit sounds and no hit or crumb particles (vanilla shows those to the digging or eating player only). The friend's own sounds come with E17 (sounds to friends, M3).
  - **A friend's own block changes are not predicted:** a placed or broken block appears after one round trip, as the design says (section 11.4). Cracks and break particles come from the host.
  - **The light for a friend's placement checks is the host's light volume.** Friends stand in the host's level until per-player levels; a block whose survival needs light (mushrooms) far from the host reads the light the host has there.
  - **Not yet for friends:** opening tables, chests and stations (E14 menus; the click is used up and nothing opens), the offhand key, the first-person swing and held item on the friend's game, and the item break particles when a friend's tool or shield breaks.
- **Known limitation found:** a friend's shield that breaks in `struck` (E12) adds its break particles to the host's own list, so the host sees them at its own eye. The friend's tool wear in `harvest` does not; the shield path is left for E15, which draws other players' items.
- **Test steps:** `docs/round8/e-m2-tests.md`, section E13c (two windows). Section E13b now starts the script with `-HostSound`, since its steps listen for sounds in the host window.
- **Kit:** `minecraft:cobblestone` x32, `minecraft:stone_pickaxe` x1, `minecraft:oak_door` x2, `minecraft:cooked_beef` x8, `minecraft:water_bucket` x1, `minecraft:snowball` x16, `minecraft:coal_ore` x4.

### E14. Each player has their own inventory screens, and chests are shared

Commit: see `git log` ("Each player has their own inventory screens, and chests are shared"). The previous commit is `4d2055d9` (E13c).

- **Changed:**
  - `crates/minecraft_survival/src/menus.rs` (new): the menu rules that lived in `render_anim::InventoryUi`, for any player.
    - `Menus` on `Player` (`player.menus`): `open: Option<Menu>` (the station) and `inventory: Menu` (vanilla's always-open inventory menu, id 0), the container id counter (1..=100, vanilla's `nextContainerCounter`) and the id the friend was last told is open.
    - `Menu`: `id`, `state` (0..0x7fff), the `dimension` it was opened in, `Open::{Inventory, Table, Block(Opened), Bench(pos, Bench)}` with the station inputs, the title as `Text`, the bookshelves, and what was last sent.
    - `open_station` opens a table, a container (with structure loot on first open and the custom name as title) or a bench (with the bookshelves for enchanting); `still_valid` is the host's old reach and block check.
    - `apply(player, containers, books, MenuAction, roll)` is the old `apply_input` and `bench_click`, click for click: the gun rule, bench clicks with the levels spent at once, the enchanting seed, the anvil wear and the station sounds, container clicks with oven XP, and the 2x2 and table grids. `click` is the checked path for friends: a click on another menu id, from a dead or spectator player, from another dimension, or with a slot or hotbar index out of range is ignored and the menu is sent again in full; a stale `state` is applied and the menu is sent again in full (vanilla's `handleContainerClick`). `rename` applies `RenameItem` to an open anvil (the item's own name clears it). `close` returns the carried stack first, then the grids and the station inputs, and returns the overflow (vanilla's `AbstractContainerMenu.removed`, then `clearContainer`).
    - `changes(player, containers, books)` is vanilla's `broadcastChanges` for the inventory menu and the open station: `MenuOpen` the first time, after a stale click and when the anvil name, the cost or the enchanting seed changes; `MenuSlots` with only the changed slots, carried stack and data otherwise; `MenuClose` when the host closed the station.
    - `screen` (host) and `mirror` (friend) build the same `Screen` for drawing, `finish` plays the station sounds and wears the anvil through `Body`, `lids` and `recheck_lids` are the host's old chest lid code for any player.
  - `crates/minecraft_survival/src/player.rs`: `menus` and `enchantment_seed`. `close_menu` is `menus::close`, and `data()` folds the station inputs back into the saved inventory.
  - `crates/minecraft_survival/src/session.rs`: `Session.menu_inputs` (at most 256 per frame) and `Sessions::send_menus`.
  - `crates/minecraft_survival/src/remote.rs`: `RemoteWorld.menu` and `inventory_menu`, kept up to date from `MenuOpen`, `MenuSlots` and `MenuClose`.
  - `crates/minecraft_net/src/messages.rs`: `Click { menu, state, action: MenuAction }` with `MenuSlot` (the HUD's own slots and actions) replaces vanilla's slot number, button and mode; `MenuExtras::Enchanting { seed, shelves }`; `MenuExtras::Anvil { cost, name: Option<String> }`. `PROTOCOL` is 2.
  - `crates/render_anim/src/minecraft_inventory.rs`: `InventoryUi` keeps only the screen: `apply_input` turns `McClick`s into `MenuAction`s for `menus::apply` (and the host's hotbar selection and drop), `show_menu` opens the screen for a `Menu`, `publish` draws from a `Screen`.
  - `crates/render_anim/src/minecraft_world.rs`: the host frame runs the host's screens through `menus` and plays `finish`, `lids` and `recheck_lids` (every player in the level, not only the host); leaving a level closes through the same path.
  - `crates/render_anim/src/minecraft_players.rs`: a friend's step opens the station their hand used, closes their menus before the death drops and on travel; `friend_menus` applies their clicks, renames and closes, closes a station out of reach, throws what falls out from their eye, gives the furnace XP and plays the station and lid sounds where they happen.
  - `crates/render_anim/src/minecraft_host.rs`: `Click`, `RenameItem` and `CloseMenu` go into the friend's session; the menus are sent after the player views.
  - `crates/render_anim/src/minecraft_remote.rs`: the friend's game loads the station data (`GameData`), opens the screen for `MenuOpen`, closes it on `MenuClose`, draws the 2x2 grid, the carried stack and the station from the host's menus, and sends `Click`, `RenameItem` and `CloseMenu`.
- **Tests:**
  - `minecraft_survival` `menus::tests`: `two_players_in_one_chest_see_each_others_clicks` (the host's shift-click reaches the friend as one `MenuSlots`, the friend takes the stack with the state it saw, and the friend's copy built from `MenuOpen` and `MenuSlots` equals a fresh view), `a_stale_state_resends_everything`, `a_click_on_a_closed_menu_or_from_another_dimension_is_ignored` (also a slot past the chest, a dead player, and `MenuClose` only for a close the host made), `closing_with_a_full_inventory_drops_the_overflow` (grid, carried stack and anvil inputs), `an_anvil_rename_applies` (the rename is sent, costs one level and names the sword; the item's own name clears it), `dying_with_items_in_the_grid_drops_them` (the grid, the carried stack and a grindstone's input), `the_friend_mirrors_a_chest_a_table_and_a_stonecutter`.
  - `minecraft_survival` `remote::tests::menus_open_change_and_close_on_the_friend` (real 26.3 data).
  - `render_anim` `minecraft_inventory::tests::screen_clicks_become_menu_actions_and_the_rest_stay_on_the_screen`.
  - `minecraft_net` codec round trips with the new `Click` and extras.
  - `server::tests::a_leaving_friend_folds_their_menu_back_drops_the_rest_and_saves_off_the_main_thread` now expects vanilla's order: the carried stack goes back first, the grid's stack drops.
  - `test -p minecraft_survival` 467 + 1, `test -p minecraft_net` 15, `test -p render_anim --lib` 80, `check -p launcher` green.
- **Decisions:**
  - **`Click` carries the HUD's own actions, not vanilla's slot number, button and mode.** Both games are ours, so the friend sends exactly what the host's own screen does, and the host runs one code path for both. A drag is one `Spread` with its slots instead of vanilla's three-part quick craft. The `state` check is vanilla's.
  - **The friend computes the station outputs itself** (crafting results, the anvil and smithing result, the enchanting offers, the stonecutter list) from the cells the host sends, with the same 26.3 data the registry check already guarantees; the host sends the enchanting seed and bookshelves, the anvil name and cost and the stonecutter choice. Taking an output always runs on the host.
  - **The inventory menu (id 0) is always open on the host**, as vanilla's: the friend's 2x2 grid and carried stack are synced through it when no station is open, and closing the friend's inventory screen sends `CloseMenu { menu: 0 }`, which returns the grid.
  - **Closing returns the carried stack first, then the grid** (javap: `InventoryMenu.removed` calls `AbstractCraftingMenu.removed`, which places the carried stack back, before `clearContainer(craftSlots)`). The host's own screens already did this; a leaving friend now does too.
  - **The enchanting seed is per player** (vanilla's `Player.enchantmentSeed`), started from the player's UUID. Before, the host used the world seed; the host's first enchanting offers in a world therefore differ from before. It is not saved yet (vanilla saves `XpSeed`).
  - **A friend in creative mode gets the survival inventory screen** (the creative tab for friends is deferred, section 10.2).
  - **The station sounds, anvil wear and lid sounds of a friend's menu play on the host where they happen**; the friend hears them with E17.
- **Test steps:** `docs/round8/e-m2-tests.md`, section E14 (two windows, with a single-player check of the host's screens first).
- **Kit:** `minecraft:oak_log` x4, `minecraft:chest` x1, `minecraft:furnace` x1, `minecraft:raw_iron` x3, `minecraft:coal` x6, `minecraft:cobblestone` x32, `minecraft:anvil` x1, `minecraft:iron_sword` x1.

### E15. Players see each other's armour, held items and elytra

Commit: see `git log` ("Players see each other's armour, held items and elytra"). The previous commit is `192bd9ab` (E14).

- **Changed:**
  - `crates/minecraft_survival/src/gear.rs`: `gear_view` and `gear_of` turn a `BodyGear` into the wire's `GearView` and back; `player_gear(player)` is the view of a player's selected item, offhand and armour (an empty view for a spectator, as the host draws none).
  - `crates/minecraft_survival/src/session.rs`: `Session::send_gear` and `Sessions::send_gear` send `Equipment { client, gear }` for each player only when that player's gear changed since the last one this friend got, including the friend's own entry; a player who left is forgotten, so their gear is sent again when they come back.
  - `crates/minecraft_survival/src/remote.rs`: `RemoteWorld.gear`, the last `Equipment` for each client.
  - `crates/render_anim/src/anim/minecraft_gear.rs`: `McBodyGear::follow(client, pm_type, gear)` is the host's rule for its own body, for any client: alive shows the gear, dead keeps the corpse rule (the gun stays away on the corpse of a player who died holding a Minecraft item), otherwise nothing.
  - `crates/render_anim/src/minecraft_host.rs`: every friend's gear is followed from their inventory and their MW2 state each frame (only in the level the host shows), a leaving friend's gear is removed, and every player's gear is sent after the player views. The host's own MW2 client number is used for the host's entry.
  - `crates/render_anim/src/minecraft_remote.rs`: the friend's game follows every client's gear from `RemoteWorld.gear` and that client's MW2 state in its own snapshot, and keeps the gear items' meshes (`gear_look`).
  - `crates/render_anim/src/minecraft_world.rs`: `draw_body_gear` also draws on the friend's game, with the friend's look and light, after the friend's frame.
  - `crates/render_anim/src/minecraft_players.rs`: a friend's broken shield (in `struck`) and worn-out tools (in `harvest`) burst at the friend's eye on the host, instead of at the host's own eye on the next frame (the known limitation from E13c).
- **Tests:**
  - `minecraft_survival`: `session::tests::gear_is_sent_once_for_each_change` (two friends, each player's gear once, nothing again unchanged, one message for a change, a player who left and came back is sent again), `gear::tests::the_gear_sent_to_friends_comes_back_the_same_and_carries_the_elytra`, and `remote::tests::menus_and_gear_reach_the_friend` (renamed from E14's test, now also with `Equipment`).
  - `render_anim`: `anim::minecraft_gear::tests::the_corpse_rule_follows_each_client_on_its_own` (one client dies holding an item and its corpse hides the gun while the other stays alive; the gear at the death decides; another client's corpse keeps its gun; no body, no gear).
  - `test -p minecraft_survival` 469 + 1, `test -p render_anim --lib` 81, `check -p launcher` green.
- **Decisions:**
  - **`Equipment` carries the whole gear of a player** (vanilla's `ClientboundSetEquipmentPacket` lists the changed slots); a change in any slot sends the player's gear again. It is small, and a friend never has to merge slots.
  - **The corpse rule runs on each game from the MW2 state of each client.** Every client's `PlayerState` (with `pm_type`) already reaches every game, so no death message is needed: each game keeps the gear it last saw while that player was alive and turns it into the corpse rule when the player dies.
  - **The friend's game now draws gear** (it drew none before): the same `gear_meshes` as the host, with the friend's own look and light.
  - **The elytra is sent as the chest item and drawn nowhere yet.** Track C's c4 is not merged into `minecraft-survival`; its wings and glide pose for every body will draw it from the same `McBodyGear` entries. The owner step for the wings waits for c4.
  - **The host's own entry keeps its own code path** in the host frame (the same rule as `follow`), so the host's single-player drawing is unchanged.
- **Test steps:** `docs/round8/e-m2-tests.md`, section E15 (two windows; step 6 shows the wings only once c4 is merged).
- **Kit:** `minecraft:iron_helmet` x1, `minecraft:iron_chestplate` x1, `minecraft:iron_leggings` x1, `minecraft:iron_boots` x1, `minecraft:iron_sword` x1, `minecraft:elytra` x1, `minecraft:firework_rocket` x8.

## Adversarial review of M2

The review covers every M2 commit from `79ba1549` (E9 to E15 and the merge of track A). Each fix below is its own commit.

### Review fix: a burning furnace no longer makes a friend's clicks stale

Commit: see `git log` ("A burning furnace no longer makes a friend's clicks stale"). The previous commit is `6457abca` (E15).

- **Defect:** `menus::changes` counted up a menu's `state` for every change it sent, including a change of data alone (a furnace's flame and arrow, a brewing stand's progress) or of the carried stack. A furnace or brewing stand that works changes its data every tick, so a friend's click in it almost always carried a `state` one behind the host's and was taken for stale: it still acted, but the whole menu was sent again after nearly every click.
- **Vanilla (javap, `ServerPlayer$1`):** `sendInitialData` and `sendSlotChange` call `incrementStateId`; `sendDataChange` (`ClientboundContainerSetDataPacket`) and `sendCarriedChange` (`ClientboundSetCursorItemPacket`) do not.
- **Fix:** `menus::sync` counts the state up only for a full `MenuOpen` and for a `MenuSlots` that changes a slot. Data and the carried stack alone keep the state.
- **Test:** `menus::tests::a_burning_furnace_does_not_make_a_click_stale` (a lit furnace burns one tick: the `MenuSlots` carries only its new data and the old state, and a click with that state is not stale).
- **Test steps (two windows, `docs/round8/e-m2-tests.md`, section E14, step 5):** while the furnace smelts, shift-click the coal out of the furnace and back in a few times: each click moves the stack at once, as before.
- **Kit:** the E14 kit.

### Review fix: a chest lid closes when a friend leaves with it open

Commit: see `git log` ("A chest lid closes when a friend leaves with it open"). The previous commit is `f1f4dbd0` (the furnace fix).

- **Defect:** `level::leave`, which runs when a friend leaves the world (`Server::leave`), took the friend out of the level's chest openers and wrote a barrel's `open` state back, but sent no lid event. Since E14 a friend can open chests, so a friend who quit with a chest open left its lid drawn open on the host for good, with no close sound. The host's own leave (`leave_level`) already sent the lid events.
- **Vanilla:** a player who disconnects is removed from `ContainerOpenersCounter`; the last one out plays the close sound and sends the block event that shuts the lid.
- **Fix:** `level::leave` turns each opener event into the lid's block event and the close sound (in `entities.sounds`, which the host frame plays), besides the barrel state.
- **Test:** `level::tests::a_chest_lid_closes_when_its_last_player_leaves_the_world` (real 26.3 data: the host and a friend open a chest; the friend leaves and the lid stays open; the host leaves and it shuts with `block.chest.close`).
- **Test steps:** `docs/round8/e-m2-tests.md`, section "Review fixes" (this commit also adds the step for the furnace fix there).
- **Kit:** the E14 kit.

### Final verification of M2: clippy on the track's own lines

Commit: see `git log` ("Multiplayer M2 passes clippy"). The previous commit is `374561a7` (the chest lid fix).

- **Clippy** on the crates touched since `79ba1549` (`minecraft_net`, `minecraft_survival`, `minecraft_terrain`, `render_anim`, `sim`, `minecraftoss-entities`, `minecraftoss-world`) with `--all-targets -A clippy::approx_constant`. Eight warnings were on lines the track added (E11 to E14); older warnings stay.
- **Changed (no behaviour change):**
  - `minecraft_survival::entities`: `mob_hit` returns `Option<MobHitTaken>`, a type alias for its tuple (`type_complexity`), and `Entities::throw` allows `too_many_arguments`, as the other entity calls do.
  - `minecraft_survival::menus::recheck_lids` uses `is_multiple_of` (`manual_is_multiple_of`).
  - `minecraft_survival::session`: the break event's data is `unwrap_or(0)`, the friend breaks `unwrap_or_else(Vec::new)` and the level cracks `map_or_else(BTreeMap::new, ...)`, in place of `unwrap_or_default()`, which the workspace `clippy.toml` refuses.
  - `minecraft_terrain::server` tests: `held` uses `map_or_else(Vec::new, ...)` for the same reason.
  - `render_anim::minecraft_world`: the open cells use a `type OpenCells` alias (`type_complexity`).
- **Tests:** none new. `test -p` of each touched crate is green on the real 26.3 data: `minecraft_net` 15, `sim` 80, `minecraftoss-entities` 83, `minecraftoss-world` 66, `minecraft_survival` 471 + 1, `minecraft_terrain` 928 (3 ignored), `render_anim` 81. `check -p launcher` is green.
- **Combined kit:** the line below now names the items with their `minecraft:` ids, as the section kits do.
- **Test steps:** none new; the sections of `docs/round8/e-m2-tests.md` cover these lines (E12 for the mob hits and throws, E13c for the cracks and break events, E14 for the chest lids).
- **Kit:** none needed.

## Milestone M3: friends see and hear the world

This run builds E16 and E17 (M3) and E18 (M4) on top of tracks B, C and D, merged first. The owner's two-window steps for M3 are collected in `docs/round8/e-m3-tests.md`, those for M4 in `docs/round8/e-m4-tests.md`.

### Merge of minecraft-survival (tracks B, C and D) into track E

Commit: the merge commit "Merge minecraft-survival (tracks B, C and D) into track E". Its first parent is `03f1bc8d` (M2 passes clippy), its second `4d45e4fe` (tracks B, C and D merged into `minecraft-survival`).

- **Conflicts and how they were resolved** (72 hunks in 11 files; every side's behaviour kept):
  - `minecraft_survival/src/{lib,player,level,commands}.rs`: both sides' modules and fields (E's `session`, `menus`, `shots`; B's `sleep`, `riding`, `saved_vehicle`, `reattach`, the sleep timer and `sleep_status`). The tests that built B/C's `PlayerView` use E's `TickActor` list.
  - `minecraft_terrain/src/server.rs`: E's per-player `TickInput.players` and outputs, with B/C's commands added beside them (`Dismount`, `WakeVillagerIn`, `AngerPiglins`, `Boost`, `RootVehicle`, `Reattach`, `HurtShulkerBullet`) and B/C's outputs (`dismounts`, `boosts`, `mounts`, `root_vehicles`). `mob_action` keeps E's argument order. The tick sets every player's main hand (E) and B's player controls and seat checks. Both sides' tests are kept.
  - `minecraft_survival/src/entities.rs`: E's `PlayerQueues` with B/C's new per-player state moved into each player's queue (see below); C's item frames, shulker bullets, End hit positions and particles, B's piglin anger, strider riding, mob uses and the bed's monster check are kept for every player.
  - `render_anim/src/minecraft_world.rs`: the host frame keeps E's calls (`begin`, `life`, `movement`, `actions::act`, `harvest`, `struck`, `settle`, `menus`) and takes B/C/D's new frame code: sleeping and the bed screen, music and the End flash sound, the rain and snow mesh, the powder snow fog, the totem's first-person animation, skating (board gear, `board_allowed`, the Skate camera), lectern screens and the frost move scale. `load_look` loads the weather texture, and `present_sky` takes powder snow and the credits and returns whether the environment ticked and an End flash, so the host plays the flash sound and steps the music; the friend's game passes the same inputs.
  - `render_anim/src/minecraft_inventory.rs`: E's `menus` clicks with C's lectern screen in front (its buttons and close go to the lectern, other clicks are ignored while it is open). The lectern test uses E's `apply_input`.
  - `render_anim/src/minecraft_respawn.rs`: E moved the bed and anchor rules to `actions.rs`; B's `show_messages` stays here, B's bed changes went into `actions.rs`.
  - `render_anim/src/skate.rs`: D's keyboard skating with E's `typing()` gate.
  - `the_end/dragon_fight.rs`: E's tick call.
- **B/C/D changes carried into the code E moved:**
  - `minecraft_players::movement` (every player): D's skate ground and step rules, B's riding (no steps while riding), C's powder snow landing sound, levitation and slow falling resetting the fall peak, freeze immunity from leather, snowflakes, melting powder snow when burning, and the totem pop's particles on the player's body (`Moved.totem_popped`; the host also plays the first-person totem).
  - `minecraft_players::struck` (every player): B/C's hit tuple (melee throw, shield disable with its cooldown and break sound, the arrow and shulker bullet effects via `survival.struck`, the landing push), strider boosts, and the riding step with dismounts on teleports and pearls.
  - `minecraft_players::begin`: leaving a level dismounts the player (host and friends). `settle` returns `Settled { told, popped }` and shows the totem pop on the body.
  - `actions::act` (every player): D's skating stops the hand (`Actor.skating`, the host's only), C's item frames (place, use, punch), B's mob uses (saddles, feeding, mounting, steering boost), C's lecterns (place a book, open), B's bed rules (night by sky light, monsters nearby, villagers asleep in it, creative, returning the bed to sleep in as `Acted.bed`), C's shield cooldown, C's frame placement, and the reach of the punch past blocks. `harvest` angers piglins guarding the mined block for the player who mined it. `menus::lids` angers piglins when a player opens a guarded container and sends trapped chest viewer counts; `level::leave` and the host's `leave_level` do too.
  - `minecraft_riding`, `Entities` and the vendored `ServerSim` take the player for every riding call (`take_mounts`, `take_dismounts`, `ride_seat`, `dismount`, `reattach`, `root_vehicle`, `leave_seat`, `can_boost`, `boost`, `take_boosts`, `take_lecterns`, `use_frame`, `use_on_mob`, `frame_on_ray`, `mob_distance`, `lectern`, `anger_piglins`, `guard_mined`); the host's client mirror only follows its own seat.
- **What the merge broke without a conflict, fixed here:** B and E both added `player` to `mob_actions::Actor` (one kept); C's tests used `stream.server.load_now` (now `load_chunk_now`); C's frame and lectern item-use answers carry their player (`Command::FrameAction` gains `player`, answers are `(player, bool)`); `refresh_vehicles`, the dismount in `sleep::start` and C's structure loot use the new signatures; the friend's `Vitals` and the test `Motion`s gain C's fields; the test `Server`s gain B's `published`.
- **Per player now (server-owned, keyed by player id and UUID):** riding (mounts, dismounts, seat, boosts, reattach, the saved vehicle), mob use results (the used item comes out of that player's inventory), lecterns (viewers and views), item frame answers, piglin anger, player controls (look, held items, armour, sneaking) for every player in the level, levitation and slow falling (C's `set_float`, already per client), freezing and frost damage, totem pops, trapped chest viewers.
- **Also changed:**
  - `Server::leave` saves a leaving friend's vehicle with them (`root_vehicle(player, true)`), as B's log asked.
  - `ServerSim::player_left` drops the player's lectern view and seat; `TickInput::remove_player` drops their controls.
  - The host's game sets `Server.published` while it hosts, so B's "players sleeping" lines go to everyone in the level, as on a vanilla LAN world.
  - Sleep and wake messages for other players (`sleep::serve`, `wake_teleported`) go to them through the sessions; a teleport from a friend's command wakes a sleeping player and dismounts a rider.
  - E15's elytra for friends: `draw_body_gear` passes C's glide state on the host and on the friend's game, so each game draws every player's wings and glide pose from the gear E15 sends.
- **Tests added:**
  - `minecraft_survival` `entities::tests::rides_boosts_lecterns_and_mob_uses_reach_only_their_player` (mob use slots, broken shears, mounts, dismounts, boosts and lectern views reach only their player; a refused boost is let go).
  - `minecraft_terrain` `server::tests::a_player_who_leaves_stops_reading_and_turns_no_more_pages` (a friend who left turns no page and the host still reads; a leaving rider's controls are dropped).
  - Existing tests follow the per-player shapes: the item frame and lectern answers carry their player, the saddle, strider and piglin tests pass the player.
  - Results on the real 26.3 data: `test -p minecraft_net` 15, `minecraft_commands` 134, `minecraft_survival` 543 + 1, `minecraft_terrain` 1103 (3 ignored), `render_anim` 133, `sim` 109, `movement_iw4` 61, `hud` 148; the vendored `minecraftoss-core`, `-world`, `-generator`, `-player` and `-entities` the way `scripts\full-suite.ps1` runs them (with the stand-in files, removed afterwards), all green. `check -p launcher` is green.
- **Decisions:**
  - **Friends do not sleep yet.** A friend's bed sets their respawn point and the night rules apply, but they do not lie down (`Acted.bed` is used for the host only): lying down needs the sleep screen, the Leave Bed button and the lying body on the friend's game, which need new messages. With a friend online the night passes only by the sleep percentage, as in vanilla when someone stays up.
  - **The lectern screen opens for the host only.** A friend can place a book; a friend's Open is answered and closed again at once on the host, so no viewer is left behind.
  - **Frost slowdown and the powder snow walker are set for the host only.** They live in the host's `sim` per client and are not replicated, so setting them for a friend would make the friend's prediction and the host disagree. A friend freezes and takes frost damage like the host.
  - **Skating stays per viewer** (track D): only the host's own board stops its hand.
  - **A friend's broken shears from a mob use** are not shown as a burst on the host (they are for the host's own).
- **Left for later (from the B/C/D logs and this merge):**
  - E16/E17 (this milestone): other players' totem pops, snowflakes, rain streaks, item frames, shulker bullets, striders and piglins drawn on the friend's game; the friend's own sounds of riding, frames, lecterns, freezing and totems.
  - M5/M6: friends sleeping (lying body for every player in `McSleepers.bodies`, sleep screen, Leave Bed message), the lectern book screen for friends (and the Take Book `mayBuild` check from the player's mode on the host), the frost move scale and the powder snow walker replicated to the friend, the friend's frozen hearts in `PlayerView`, skating for friends, the creative-mode check for a frame struck by a snowball (C's log), the merchant screen for friends.
- **Test steps:** `docs/round8/e-m3-tests.md`, section "Merge of tracks B, C and D" (two windows).
- **Kit:** `minecraft:elytra` x1, `minecraft:firework_rocket` x8, `minecraft:saddle` x1, `minecraft:warped_fungus_on_a_stick` x1, `minecraft:strider_spawn_egg` x1, `minecraft:shulker_spawn_egg` x1, `minecraft:powder_snow_bucket` x1, `minecraft:item_frame` x2, `minecraft:iron_sword` x1, `minecraft:cobblestone` x32, `minecraft:lectern` x1, `minecraft:writable_book` x1, `minecraft:trapped_chest` x1, `minecraft:redstone_lamp` x1, `minecraft:gold_block` x4, `minecraft:stone_pickaxe` x1, `minecraft:golden_helmet` x1, `minecraft:totem_of_undying` x1, `minecraft:white_bed` x1.

```
IW4L_MINECRAFT_KIT=minecraft:elytra,minecraft:firework_rocket*8,minecraft:saddle,minecraft:warped_fungus_on_a_stick,minecraft:strider_spawn_egg,minecraft:shulker_spawn_egg,minecraft:powder_snow_bucket,minecraft:item_frame*2,minecraft:iron_sword,minecraft:cobblestone*32,minecraft:lectern,minecraft:writable_book,minecraft:trapped_chest,minecraft:redstone_lamp,minecraft:gold_block*4,minecraft:stone_pickaxe,minecraft:golden_helmet,minecraft:totem_of_undying,minecraft:white_bed
```

### E16. Friends see the mobs, items and projectiles the host sees

Commit: see `git log` ("Friends see the mobs, items and projectiles the host sees"). The previous commit is `8c7b95b5` (the merge of tracks B, C and D).

An earlier session was interrupted in the middle of this group; its uncommitted work (never built) was taken from the stash "E16 partial (interrupted session, 2026-10-12)", checked against the design, fixed where it was wrong or incomplete, built, tested and committed here.

- **Changed:**
  - `crates/minecraft_terrain/src/mob_tracking.rs` (new), vanilla's `ChunkMap.TrackedEntity` for every friend, run on the level thread:
    - `Watcher { id, feet, view_distance }`, one per friend in the level, in `TickInput.watchers`.
    - `Trackers`: a tracked set per player. Once per batch (`finish_batch`) a mob is in a player's range when its horizontal distance is within `min(tracking_range(kind), view_distance) × 16` (`tracking_range`: vanilla's `clientTrackingRange` per entity type, 5 for anything not listed). A mob entering a player's range gives `MobEvent::Add(id, tag)` with its full saved tag (`mob_tags_where`, which keeps the equipment, attributes, UUID and villager data of the originals); a mob leaving range or removed gives `Remove`. Every 10 ticks each tracked mob's `look` (its tag without the values that change all the time: position, motion, rotation, health, fire, air, hurt and death times, anger end, age, love, gossips, food, effects, brain and a few more, plus `Baby`) is compared with the last one sent, and a change gives `Look(id, tag)`. `moves` holds, per player, the `ServerMob` of every mob that player tracks. A dead mob already tracked stays tracked until it is gone, so its death is shown.
    - A player who stops being a watcher (left the level, no place) gets a `Remove` of everything they tracked, so nothing stays behind on their game.
    - **Projectiles:** ghast and blaze fireballs, shulker bullets and witch potions live in the mob world, so they are tracked the same way (`projectiles`: a `ServerMob` with their position and the bullet's turn), never resent as looks, and `is_projectile` tells them apart on the friend.
    - `MobShow` on `ServerMob` carries the render state that neither the tag nor the moves had (see the decisions). `shows` fills it on the host; `show_on` writes the moves and the show state into the friend's mob world (bodies, previous positions, fire, the bullets' turn and age).
    - `spawn_tracked` spawns a tag under the host's id, replacing a mob already there.
  - `crates/minecraft_terrain/src/server.rs`: `ServerSim.trackers`; `finish_batch` mirrors the mobs around the host **and** every watcher (`tracked_mobs_around`, 160 blocks each), so MW2 bullets and hands find the mobs next to a friend, and outputs `Output.tracked`; `player_left` forgets the player's tracker. `EntitySnapshot` and its view types (`ServerItem`, `ServerTnt`, `ServerFallingBlock`, `ServerOrb`, thrown, cloud, trident, eye, crystal, dragon, breath, fireball, gateway and frame views, `Block`, `ResourceId`, `Direction`) derive serde, with glam's `serde` feature.
  - `crates/minecraft_terrain/src/server_mobs.rs`: `mob_tags_where(world, keep(id, pos), originals, nest)`; `mob_tags` calls it as before. Tracked tags are not nested: a rider is its own mob, and the moves carry its seat.
  - `crates/minecraft_terrain/src/client_mobs.rs`: `ServerMob` derives serde and has `show`.
  - `third_party/minecraftoss/entities`: `remove_mobs` also removes shulkers, fireballs, shulker bullets and potions; `fireball_mut` and `potion_mut`.
  - `crates/minecraft_survival/src/entity_view.rs` (new), section 9.1's split of `Entities`: `EntityView` holds the client mirrors that were fields of `Entities` (the mob world, `ClientMobs`, particles, the drawn server items, thrown items, clouds, tridents, crystals, gateways, dragons and their fireballs, frames, orbs, falling blocks, lids, spawner displays, orb pickups, crumbs, sounds), with `advance` (the client tick), `receive` (an `EntitySnapshot`), `show_items`, `show_mobs`, `boxes`, `shot_boxes`, `tick_scene` and `meshes`, moved unchanged. `Entities.view` is one; the host feeds it as before. `show_mobs` remembers which mobs are shown, and `meshes` spawns client mobs only for those.
  - `crates/minecraft_survival/src/entities.rs`: the mirror fields and their code moved to `EntityView`. `set_watchers`, `take_tracked`, `take_friend_snapshot` (the level's snapshot, kept only while a friend watches) and `mob_box(id)`. The host's mirror shows only the mobs in **its own** range (`shown_mobs`: the per-kind range capped by the view distance of 8 chunks), not those around a far friend.
  - `crates/minecraft_survival/src/mob_sender.rs` (new): `MobSender` per session. It turns the friend's events into `MobAdd` (the tag as NBT), `MobLook` (the new tag as NBT) and `MobRemove`, holding an add until the friend has the mob's chunk (a look for a held add replaces the held tag, a remove cancels it). `frame` builds the friend's `StateFrame`: the moves of every mob sent to them and the level's entities in their range and chunks (`near`: items and orbs 6 chunks, TNT and falling blocks 10, thrown items, tridents and eyes 4, frames and clouds 10, crystals 16, the dragon, its fireballs, breath and the gateways always). A new level forgets what was sent.
  - `crates/minecraft_survival/src/session.rs`: `Session.mobs` and `Sessions::send_mobs`, run each host frame after the menus; `minecraft_host.rs` sends the state frames on the state lane.
  - `crates/minecraft_survival/src/remote.rs`: `RemoteLevel.view` is an `EntityView`. `MobAdd` spawns the tag with the host's id, `MobLook` spawns it again in place, `MobRemove` removes it; a `StateFrame` moves only mobs that had an add (moves for any other id are dropped before `ClientMobs` sees them), shows the items and the level's entities. `RemoteWorld.recipes` sizes the items' stacks.
  - `crates/render_anim/src/minecraft_world.rs`: the host frame sets every friend in the level as a watcher (at their arrival, else their place, like the chunk sender).
  - `crates/render_anim/src/minecraft_remote.rs`: the friend's game ticks its `EntityView` and draws its meshes, with the particles of the mirror, into the friend's view. The friend's hand now aims at mobs (`Target::Mob`) with the vanilla entity reach, and the host checks it against the mob's box (`actions::allows` with `Entities::mob_box`), so a friend can saddle, feed, shear and ride mobs from their own window.
  - The other files only follow `entities.view.*`.
- **Tests:**
  - `minecraft_terrain` `mob_tracking::tests`: `the_tracking_range_is_vanillas_per_kind_and_the_view_distance_caps_it`, `a_mob_is_added_moved_and_removed_as_a_watcher_comes_and_goes`, `a_watcher_who_stops_watching_is_told_to_remove_what_they_had`, `projectiles_are_tracked_moved_and_never_resent_as_looks`, `a_look_change_is_sent_once_and_a_health_change_is_not`, `an_added_tag_keeps_equipment_profession_and_the_hosts_id`, `the_mirror_moves_and_shows_what_the_host_sends`.
  - `minecraft_survival` `mob_sender::tests`: `an_add_waits_for_its_chunk_and_a_remove_cancels_it`, `a_look_follows_its_add_and_a_held_add_takes_the_newer_look`, `a_mob_removed_and_added_again_in_one_frame_is_removed_first`, `a_new_level_forgets_what_was_sent`, `the_state_frame_holds_only_sent_mobs_and_entities_in_range`.
  - `minecraft_survival` `entities::tests`: `the_hosts_mirror_skips_mobs_beyond_its_own_range`, `a_friend_tracks_the_mob_next_to_them` (real 26.3 data: a cow summoned at the friend's feet is added to the friend only, then moved, and the snapshot goes to the friend).
  - `minecraft_survival` `remote::tests`: `a_mob_is_added_moved_and_removed_and_moves_without_an_add_are_dropped`; the loopback `a_friend_sees_the_hosts_blocks_in_order_and_their_leave_is_saved` now also summons a cow next to the friend on the host and checks that, through `send_mobs`, the wire codec and `apply_state`, the friend's game shows it under the host's id where the host has it.
  - Results on the real 26.3 data: `test -p minecraft_terrain` 1110 (3 ignored), `minecraft_survival` 551 + 1, `render_anim --lib` 133, `minecraftoss-entities --lib` 229; `check -p launcher` green.
- **Decisions:**
  - **`MobLook` carries the mob's whole tag, not a separate look struct.** The tag already holds every look value (equipment, saddle, villager data, colours, names, baby, variants), and the friend spawns it again in place, so one loader (`spawn_saved`) builds the friend's mob both times and no field can be forgotten. The comparison that decides whether to send is on the tag without its volatile values, every 10 ticks, so health, fire, age and the like never cause a resend.
  - **Render flags added to `ServerMob` (`MobShow`):** fire (burning mobs), a zombie's and skeleton's raised arms (`aggressive`), a creeper's swell (old and new), an enderman's anger (creepy), a wolf's anger, a ghast's charging mouth, a shulker's peek (old and new), a hoglin's and piglin's conversion shake, a piglin's arm pose, aggression and the item it uses. Resting bats and deaths were already in `ServerMob`.
  - **Moves go every tick** (each host batch, about 20 a second) in the friend's state frame, as the design says, without vanilla's per-kind intervals; `ClientMobs::receive` replays vanilla's tracker encoding, so the friend's poses match the host's.
  - **The host's own mirror now shows mobs by vanilla's per-kind range capped at 8 chunks** (cows 128 blocks, zombies 128, bats 80) instead of every mob within 160 blocks. The mob world it keeps for bullets still covers 160 blocks around the host and every friend; flames, potions, fireballs and shulker bullets are drawn only within 160 blocks of the camera, so those near a far friend do not float in the host's fog.
  - **`StateFrame.epoch` is 0:** the level's epoch only changes with `Dimension`, which nothing sends while travel is locked; M5 sets it.
  - **The friend's mirror sounds and crumbs are dropped** (blaze burning, the dragon's wings and growls, thrown items' crumbs): E17 decides what the friend plays itself and what the host sends.
- **Left for later:**
  - E17: the friend's sounds and particles (mob sounds from the mirror, explosions, level events, pickups' flying item and pop, totem pops, snowflakes, rain streaks, lid block events and the chest lids), and the friend's own sounds of riding, frames, lecterns, freezing and totems.
  - Spawner cages on the friend's game: the remote stream keeps no chunk objects to read the spawners from.
  - The dragon's one-shot effects (`breath_bursts`, `dragon_breath`, `dragon_explosions`, `eyes.shattered`) ride in the state frame, which is newest-wins and can be dropped; they belong with the dragon fight for friends (M6).
  - M5: the epoch in `StateFrame`, watchers in levels the host is not in (today only the host's level ticks with watchers), and clearing the friend's tracked set on travel.
- **Test steps:** `docs/round8/e-m3-tests.md`, section E16 (two windows).
- **Kit:** `minecraft:cow_spawn_egg` x5, `minecraft:pig_spawn_egg` x1, `minecraft:saddle` x1, `minecraft:snowball` x16, `minecraft:ender_pearl` x4, `minecraft:shulker_spawn_egg` x1, `minecraft:item_frame` x1, `minecraft:iron_sword` x1, `minecraft:cobblestone` x32.

```
IW4L_MINECRAFT_KIT=minecraft:cow_spawn_egg*5,minecraft:pig_spawn_egg,minecraft:saddle,minecraft:snowball*16,minecraft:ender_pearl*4,minecraft:shulker_spawn_egg,minecraft:item_frame,minecraft:iron_sword,minecraft:cobblestone*32
```

### E17. Friends hear and see what happens around them

Commit: see `git log` ("Friends hear and see what happens around them"). The previous commit is `396071df` (E16).

- **Changed:**
  - `crates/minecraft_survival/src/heard.rs` (new): what a level says to the players in it. `Heard` is a sound (event, place, volume, pitch), a level event, an explosion, a block event (chest and shulker lids) or a player event (the totem, vanilla's entity event 35); `Said` adds `only` for what one player alone hears. `reaches` is vanilla's broadcast radius: a sound reaches `16 × max(volume, 1)` blocks (`SoundEvent.getRange`), level events, block events and explosions 64 (`ServerLevel.levelEvent`, `runBlockEvents`, `explode`), a player event the players tracking that player (`min(32, view distance)` chunks), and the dragon's death sound the whole level. `HeardLog` keeps lines only while a friend is in the level. `of_effects` turns an action's effects into lines (a positionless sound comes from the player's eye; a dig-hit sound goes to the digger only), `played` does the same for a sound the presenter played, and the bubble pop and the portal noises stay with the player who made them.
  - `crates/minecraft_survival/src/entities.rs`: `Entities.heard` (one log per level). The level thread's outputs go in it: mob sounds (with their real place, not the host's `heard_at`), mob-use sounds, explosion sounds and particles, level events, splash and lingering potion breaks (vanilla's level events 2002 and 2007) and every item pickup (vanilla's pickup pop at the item; the host now also hears a friend's pickup). `Entities::sound` (the host hears it and it is logged) replaces the direct pushes in `commands.rs` (give and level up), `level.rs` (chest sounds when a player leaves; `lid_event` also logs the lid's block event, as `menus.rs` does), `respawn_block.rs`, `sleep.rs` and the fireball deflection.
  - `crates/minecraft_survival/src/entity_view.rs`: pure moves from `Entities` so the friend's game can run them: `animate_tick`, `present` and `level_event` (now also drawing a potion break from its level event), `rain_tick` and `weather_mesh` (taking the rain level and the partial tick), `track_body`, `totem_pop` and `explosion`. `Entities` keeps the same methods, now calling these.
  - `crates/minecraft_survival/src/session.rs`: `Sessions::send_heard` takes each level's log every host frame, and `tell` sends each friend in that level the lines that reach them from their place (`arrival`, else `place`), with their own view distance.
  - `crates/minecraft_net`: `ToClient::Explosion` carries `radius`, `blocks` (a count) and `interacts`, as the 26.x explode packet does, instead of block positions the host never had; new `ToClient::PlayerEvent { client, event }`. `docs/MULTIPLAYER.md` section 4 follows.
  - `crates/render_anim/src/minecraft_sounds.rs`: `Sounds` can record what it plays (`record`, `take_recorded`), `play_local` plays without recording, and `Sounds::recorder()` records and plays nothing.
  - `crates/render_anim/src/minecraft_world.rs`: the host frame records its world sounds from the start of the player step to the sound drain (footsteps, falls, hurts, deaths, shields, eating, placing and breaking, doors, buckets, bullets into blocks, MW2 blasts, XP and level-up, menus and lids) and logs them, positionless ones from the host's eye; the ambient display ticks, rain, UI clicks and dig hits use `play_local`. Blocks the level destroys are logged as level event 2001, so friends see their burst. The crumbs presenter is a shared function (`present_crumbs`, pure move).
  - `crates/render_anim/src/minecraft_players.rs`: a friend's step plays into a recorder; what it recorded is logged, and the host hears it where it happens. A friend's XP pickup and level-up chime are played (they were silent). The effects of a friend's actions and menus are logged. `pop_totem` logs the player event.
  - `crates/render_anim/src/minecraft_remote.rs`: the friend's game has its own `Sounds` and fills the sound queue. It plays the host's sounds; presents level events (lava fizz, page turns, bone meal, potion breaks, block bursts), explosions, lid block events (chest and shulker lids open and close, with `draw_apart` on its stream) and totem pops on the popping player's MW2 body; runs its own display ticks (ambient block sounds and particles, rain sounds and splashes); draws the rain and snow; and plays its mirror's sounds (blaze burning, the dragon's wings and growls, the dragon's death from where vanilla puts it), the crumbs of potions and eyes, its UI clicks and the End flash sound from its own sky.
- **Tests:**
  - `minecraft_survival` `heard::tests`: `a_sound_reaches_sixteen_blocks_times_its_volume_and_never_less_than_sixteen`, `level_events_block_events_and_explosions_reach_sixty_four_blocks`, `the_dragon_dies_for_everyone_and_a_totem_pops_for_those_tracking_the_player`, `a_said_line_for_one_player_reaches_only_them`, `the_log_keeps_lines_only_while_someone_listens`, `listener_sounds_come_from_the_eye_and_dig_sounds_go_to_the_digger_only`, `bubbles_and_portal_noises_stay_with_the_player_who_made_them`, `each_line_becomes_its_vanilla_message`.
  - `minecraft_survival` `session::tests::what_is_heard_reaches_friends_in_range_in_that_level_and_never_across_levels`.
  - `minecraft_survival` `entities::tests::a_friends_pickup_is_heard_by_all_and_the_mirrors_own_sounds_are_not_sent` (real 26.3 data: the friend's pickup is logged once for everyone from the item, the host hears it, a mirror sound is not logged, and nothing is kept while no friend listens).
  - `render_anim` `minecraft_sounds::tests::a_recorder_keeps_what_was_played_and_a_local_sound_is_not_kept`.
  - `minecraft_net` `codec::tests` cover the new `Explosion` and `PlayerEvent`.
  - Results on the real 26.3 data: `test -p minecraft_net` 15, `minecraft_survival` 561 + 1, `render_anim --lib` 134; `check -p launcher` green.
- **Decisions:**
  - **What is sent and what each game makes itself** (the design's rule): sounds, level events, explosions and block events that come from the level or from a player's action are sent; the mirror's own sounds (blaze burning, the dragon's wings and growls), the ambient display ticks, rain sounds, rain and snow, the End flash and UI clicks are made by each game itself, as vanilla's client makes them, so nothing plays twice.
  - **Sounds are taken where the host plays them** (the presenter records them) instead of rewriting each rule to return its sounds. Positionless sounds (vanilla plays them at the player for everyone else) are sent from the player's eye.
  - **A friend hears their own actions after one round trip.** Vanilla's client predicts its own place, break and use sounds; the friend's game does not predict blocks (section 11.4), so the host sends them to the actor too. Dig-hit sounds go to the digger only, as vanilla plays them on the digging client only.
  - **The host hears its friends** (footsteps, falls, hurts, eating, pickups, totems), as on a vanilla LAN world. Before this only a friend's block actions were heard.
  - **Nothing crosses levels.** Vanilla sends its global level events (the dragon's death, the wither's spawn, the End portal opening) to every player in every dimension; here the dragon's death sound reaches every friend in the End only. Friends cannot be in another level before M5, which decides this.
  - **Potion breaks** go as vanilla's level events 2002 and 2007 at the block, and the friend's burst is drawn at that block's bottom centre, as vanilla does; the host still draws its own at the exact place.
  - **E15's wings on friends needed no further commit.** The merge of track C already passes C's glide state to `draw_body_gear` on the host and on the friend's game (see the merge entry), so each game draws every player's elytra and glide pose from the gear E15 sends.
- **Left for later (M5/M6 or later rounds):**
  - The item pickup animation (the item flying into the picker) on the friend's game; only its sound is sent (vanilla's `ClientboundTakeItemEntityPacket`).
  - Particles that are neither level events nor explosions: thrown snowball and egg crumbs, the ender pearl's portal particles, the dragon egg's teleport trail, MW2 bullet puffs on mobs, and a friend's own dig-hit, eating and broken-tool particles.
  - Snowflakes around other players in powder snow on the friend's game, and the friend's own first-person totem animation.
  - Campfire smoke and spawner cages on the friend's game (its stream keeps no containers or chunk objects).
  - Lightning and thunder for friends.
  - From the merge: friends sleeping, the lectern book screen for friends, the frost slowdown and powder snow walker replicated to friends, frozen hearts in `PlayerView`, skating for friends, the merchant screen (M5/M6).
- **Test steps:** `docs/round8/e-m3-tests.md`, section E17 (two windows; it is also the M3 owner test).
- **Kit:** `minecraft:tnt` x4, `minecraft:flint_and_steel` x1, `minecraft:cow_spawn_egg` x3, `minecraft:cooked_beef` x8, `minecraft:chest` x1, `minecraft:cobblestone` x32, `minecraft:water_bucket` x1, `minecraft:lava_bucket` x1, `minecraft:bone_meal` x8, `minecraft:totem_of_undying` x1, `minecraft:snowball` x16, `minecraft:ender_pearl` x4, `minecraft:iron_sword` x1.

```
IW4L_MINECRAFT_KIT=minecraft:tnt*4,minecraft:flint_and_steel,minecraft:cow_spawn_egg*3,minecraft:cooked_beef*8,minecraft:chest,minecraft:cobblestone*32,minecraft:water_bucket,minecraft:lava_bucket,minecraft:bone_meal*8,minecraft:totem_of_undying,minecraft:snowball*16,minecraft:ender_pearl*4,minecraft:iron_sword
```

## Milestone M4: fighting

### E18. Guns hurt other players unless pvp is off, and deaths are told to everyone

Commit: see `git log` ("MW2 guns hurt other players unless pvp is off, and every death is told to everyone"). The previous commit is `c1e5be0b` (E17).

- **Changed:**
  - `crates/session/src/match_apply.rs`: `match_kind` takes the zone and returns free-for-all for a Minecraft load, whatever the menu, the master offer or `IW4L_GAMETYPE` picked (section 13.4). Both games run the same install, so the friend loads `dm` too. The time and score limit zeroing moved unchanged into `zero_minecraft_limits` so a test can pin it.
  - `crates/sim/src/voxel.rs`: `set_pvp` / `pvp` (an atomic that defaults to on and is reset by `deactivate`), and a `PvpDeath { victim, attacker, weapon }` queue (`take_pvp_deaths`).
  - `crates/sim/src/damage.rs`: `DamageRefusal::PvpDisabled`. `apply_damage_attempt` refuses a hit when the voxel world is active, pvp is off, the attacker is a player client (`client_meta` knows them) and the attacker is not the target (`pvp_refuses`). Bullets, knives, MW2 grenades and killstreaks all go through it; self-damage and the Minecraft world's damage (`apply_block_world_damage`, which never calls it) still hurt.
  - `crates/sim/src/script_player.rs`: `kill` queues a `PvpDeath` when the authoritative sim kills a player in the voxel world and the attacker is another player client, with the weapon of the death commit.
  - `crates/minecraft_survival/src/death.rs`: `CombatTracker::death_text` builds the death line as a translatable `Text` (the key, the victim's name, the attacker as an entity name or the bad respawn point's link), and `player_kill_text` builds `death.attack.player` ("%1$s was slain by %2$s") or `death.attack.player.item` ("... using %3$s") with the MW2 gun's name in brackets, as vanilla's `ItemStack.getDisplayName` shows a weapon.
  - `crates/minecraft_survival/src/survival.rs`: `tell_death` marks the current life's death as told (cleared by respawn); `death_text`.
  - `crates/minecraft_survival/src/commands.rs`: `death_message` takes a `Text`; `tell_death(rules, survival, text)` returns the line once per death and only with `show_death_messages` on.
  - `crates/minecraft_survival/src/entities.rs`: `Entities.pvp` (from `set_game_rules`). With pvp off, a trident or wind charge thrown by a player in the level does not strike any player (`unharmed_without_pvp`), as vanilla's `ServerPlayer.canHarmPlayer`.
  - `crates/render_anim/src/minecraft_players.rs`: `settle` builds the translatable line through `tell_death` (each game resolves it in its own language). `tell_pvp_deaths` turns the queued PvP deaths into lines: the victim and the attacker by their MW2 client (the host's by the host's client number, friends by their own), their names from the command server, the gun's MW2 display name. `step_friend` returns the friend's Minecraft killers.
  - `crates/render_anim/src/minecraft_world.rs`: the host frame writes the `pvp` rule into `sim::voxel::set_pvp` every frame, tells the PvP deaths right after draining the voxel queues (before any player's life step, so a death already told by `settle` is not told again), shows them and relays them to every friend. The weapon names come from `PreparedWeapons` and `PreparedLocalizedStrings`, as the HUD's weapon name. The host's own Minecraft death line is now resolved with its language.
  - `crates/render_anim/src/minecraft_host.rs` / `minecraft_remote.rs`: a friend's Minecraft killer goes to them as `ToClient::KilledBy`, and the friend's game puts it on its killed-by card (`ui.killed_by`), as the host does for itself.
- **Already per player, checked:** death drops (each player's own `death_watch` at their own death spot, `keep_inventory` for everyone) and the friend's broadcast death line (E10b/E11).
- **Tests:**
  - `session` `match_apply::tests::a_minecraft_world_runs_free_for_all_with_no_time_or_score_limit` (every menu choice gives `dm` on a Minecraft load and keeps itself elsewhere; `scr_dm_timelimit` and `scr_dm_scorelimit` are zeroed, another mode's limit is left alone).
  - `sim` `damage::tests::with_pvp_off_players_cannot_hurt_each_other_but_still_hurt_themselves_and_the_world_hurts_them`, `damage::tests::with_pvp_on_one_player_knifes_another` (with both players known clients, nothing else refuses the hit).
  - `minecraft_survival` `commands::tests::a_pvp_death_is_told_once_with_the_killers_gun_and_only_with_death_messages_on`, `death::tests::the_translatable_line_reads_as_the_resolved_one`, `entities::tests::with_pvp_off_a_players_trident_or_wind_charge_spares_every_player`.
  - `render_anim` `minecraft_players::tests::a_gun_kill_between_players_is_told_once_with_both_names_and_the_gun` (host and friend by client number, a repeated death and an unknown victim give no line, no line with death messages off); `minecraft_chat::tests::death_messages_are_white_system_lines_each_game_reads_in_its_language` replaces the plain-line test.
  - Results on the real 26.3 data: `test -p sim` 111, `session` 1, `minecraft_survival` 564 + 1, `render_anim --lib` 135; `check -p launcher` green.
- **Decisions:**
  - **No `DamageCause::Player`.** MW2 damage never enters the Minecraft combat tracker (it only lowers the MW2 health the survival state follows), so a cause in the tracker would never be read; the line is built straight from the sim's death. A gun hit followed by a fall or lava therefore says "fell from a high place" / "tried to swim in lava", not vanilla's "doomed to fall by <player>". Recorded for later.
  - **The gun's name is in brackets** ("using [M4A1]"), as vanilla shows the weapon's display name. Vanilla only names the weapon when it has a custom name; here every gun kill names the MW2 gun, as the design asks.
  - **One line per death:** `Survival::tell_death` is set by whichever path tells the death first (the Minecraft settle or the MW2 kill) and cleared on respawn.
  - **Minecraft hits with pvp off follow vanilla:** `canHarmPlayer` has no exception for the thrower, so with pvp off a player's own trident does not hurt them either. The MW2 rule spares only other players, as the design says (a player's own grenade still hurts).
  - **The friend's sim is not told the pvp rule:** damage is decided only on the host, whose snapshot corrects the friend.
  - **`KilledBy.line` is empty:** the killer arrives a frame after the death line, and the friend reads the line from the chat; the card uses `killer` only.
- **Left for later:**
  - Minecraft melee against another player (a sword) and TNT lit by another player still ignore the pvp rule, as section 13.4 says for melee; a harming splash potion thrown by another player too.
  - From the B/C/D logs and the merge, for M5/M6: friends sleeping, the lectern book screen for friends, the frost slowdown and powder snow walker replicated to friends, frozen hearts in `PlayerView`, skating for friends, the merchant screen, other players' snowflakes and the friend's own totem animation (see E17's list).
- **Test steps:** `docs/round8/e-m4-tests.md`, section E18 (two windows; it is also the M4 owner test).
- **Kit:** `minecraft:creeper_spawn_egg` x2, `minecraft:trident` x1, `minecraft:wind_charge` x8, `minecraft:cooked_beef` x8, `minecraft:cobblestone` x32.

```
IW4L_MINECRAFT_KIT=minecraft:creeper_spawn_egg*2,minecraft:trident,minecraft:wind_charge*8,minecraft:cooked_beef*8,minecraft:cobblestone*32
```

## Adversarial review of M3 and M4

The review covers every commit from `03f1bc8d` (the merge of tracks B, C and D, E16, E17 and E18). Each fix below is its own commit.

### Review fix: friends see striders and other mobs that were loaded with riders

Commit: see `git log` ("Friends see striders and other mobs that were loaded with riders"). The previous commit is `04f9321b` (E18).

- **Defect:** a mob's tracked tag starts from the tag it was loaded with (`mob_tags_where` copies `originals`). A vehicle loaded with riders (`spawn_saved_all`: a strider jockey from a chunk, `/summon` with `Passengers`, a reattached root vehicle) keeps its whole loaded tag there, `Passengers` included. With `nest` off nothing took that list out, so the friend's `MobAdd` carried it, and `spawn_saved` refuses a strider, piglin, zombie, skeleton or spider whose tag has `Passengers` (those go through `spawn_saved_all`). Every ridden strider in the Nether (and every spider jockey) stayed invisible on the friend's game, even after its rider got off, with "Minecraft mob N from the host not shown" in the friend's log.
- **Fix:** `mob_tags_where` with `nest` off drops `Passengers` from every tag, as the E16 decision says (a rider is its own tracked mob, and the moves carry its seat). Saving (`nest` on) is unchanged.
- **Test:** `mob_tracking::tests::a_loaded_strider_and_its_rider_are_added_as_two_mobs_the_friend_can_spawn` (a strider loaded with a zombified piglin on it is added as two mobs, neither tag has `Passengers`, and the friend's world spawns both).
- **Test steps:** `docs/round8/e-m3-tests.md`, section "Review fixes", step 1.
- **Kit:** none.

### Review fix: the totem steps use a fall, not `/kill`

Commit: see `git log` ("The owner's totem steps use a fall, since /kill goes past a totem"). The previous commit is `599f2024` (the riders fix).

- **Defect:** the owner's totem steps (`e-m3-tests.md` merge step 10 and E17 step 8, `e-m2-tests.md` E10a step 7) said to pop the totem with `/kill`. Vanilla's `/kill` deals `generic_kill`, which is in `bypasses_invulnerability`, and `LivingEntity.checkTotemDeathProtection` gives up for that tag; the game does the same (`survival::tests::generic_kill_has_the_vanilla_message_and_bypasses_the_totem`). So the player would just die and the steps would read as a failure.
- **Fix (docs only):** the steps drop the player 40 blocks with `/tp` instead (37 fall damage), which the totem stops.
- **Also corrected:** the riders fix's step R1 no longer summons a spider jockey. Only a strider carries riders through `spawn_saved_all` (`carries`), so a summoned spider with `Passengers` never spawns on the host either; the defect text above should say "a strider" where it says "every spider jockey".
- **Test:** none (docs).
- **Kit:** the totem is in the E17 kit and the merge kit.

### Final verification of M3 and M4: clippy on the track's own lines

Commit: see `git log` ("Multiplayer M3 and M4 pass clippy"). The previous commit is `bf0a3a47` (the totem steps fix).

- **Clippy** on every crate track E changes over `minecraft-survival` (`console`, `frame`, `hud`, `iw4l-master`, `minecraft_commands`, `minecraft_net`, `minecraft_survival`, `minecraft_terrain`, `net`, `render_anim`, `session`, `sim`, `minecraftoss-core`, `minecraftoss-entities`, `minecraftoss-world`) with `--all-targets -A clippy::approx_constant`. Six warnings were on lines this run added (E16 and E17); none on the merge, E18 or the review fixes. Older warnings stay.
- **Changed (no behaviour change):**
  - `minecraft_survival::entity_view`: the mob count log uses `is_multiple_of` (`manual_is_multiple_of`), and `receive` takes `&[(BlockPos, Option<Block>)]`, as `falling_render` does (`type_complexity`).
  - `minecraft_survival::heard::HeardLog::listen` uses `unwrap_or_else(Vec::new)`, `minecraft_survival::mob_sender::MobSender::frame` fills `entities` through an `if let`, `minecraft_terrain::mob_tracking::show_on` falls back to `ArmPose::Default` by name, and `render_anim::minecraft_sounds::Sounds::take_recorded` uses a `let else`, in place of `unwrap_or_default()`, which the workspace `clippy.toml` refuses (the two sites where clippy's own `unwrap_or_default` lint refuses `unwrap_or_else(Vec::new)` too take the `if let` and `let else` forms).
- **Tests:** none new. `test -p` of each touched crate (`--lib --bins --tests`) is green on the real 26.3 data: `console` 45, `frame` 24, `hud` 148, `iw4l-master` 1, `minecraft_commands` 134, `minecraft_net` 15, `minecraft_survival` 564 + 1, `minecraft_terrain` 1111 (3 ignored), `minecraftoss-core` 27, `minecraftoss-entities` 229, `minecraftoss-world` 74, `net` 15, `render_anim` 135, `session` 1, `sim` 111. `check -p launcher` is green. The new files still pass `rustfmt --check` on their staged content.
- **Combined kit:** the section below now covers the M3 and M4 owner tests.
- **Test steps:** none new; `docs/round8/e-m3-tests.md` (E16 for the mob view and the mob sender, E17 for the heard log and the recorded sounds) covers these lines.
- **Kit:** none needed.

## Milestone M5: different dimensions

This run builds E19, E20 and E21 (M5). The owner's two-window steps are collected in `docs/round8/e-m5-tests.md`.

### E19. Each dimension has its own collision, so players can be in different dimensions

Commit: see `git log` ("Each Minecraft dimension has its own collision band"). The previous commit is `38c7339b` (M3 and M4 pass clippy).

- **Changed:**
  - `crates/sim/src/voxel.rs`, section 8.3 of the design:
    - `VoxelWorld { brushes, levels: [Option<VoxelLevel>; 3], spawn, shapes, fluids }` with `VoxelLevel { origin, chunks, min_y, height }`. The shape table stays one table for every level, so `runtime.shapes` and `shape_ids` are no longer cleared on a switch.
    - `BANDS` (overworld `[-20,000, 20,000)`, Nether `[-60,000, -20,000)`, End `[20,000, 60,000)` in map z) and `band_of(z)`. Every query picks its level from the band of its point: traces and `trace_as` from the start, `ground_within`, `water`, `lava`, `climb`, `eye_in_lava`, `stuck`, `touches_wall`, `push_out`, `body_box`, `camera_reach`, `awaiting_ground` (a point in a band with no level waits for ground), the skate meshes (`collision_triangles`, `skate_ramps`, `track_rails`, `missing_chunk_walls`) and `changed_near` (the change log now records the level of each change).
    - The collision methods moved from `VoxelWorld` to `Level<'_>`, a view of one level with the shared shape table and its wall range; their bodies are unchanged.
    - **Walls:** `shape_at` answers a full block below `min_y - 128` and from `max_y + 256` up (`WALL_BELOW`, `WALL_ABOVE`), whether or not a chunk is loaded there, so nothing crosses into the next band.
    - `activate(brushes, shapes)`, `set_level(level, origin, min_y, height)` (a new, empty level), `drop_level(level)` and `has_level(level)` replace `switch_level`. `set_chunk`, `remove_chunk`, `set_block_shape` and `chunks_ready_around` take the level.
    - Events: `Shot`, `Explosion`, `Ray`, `MobShot` and `MobBlast` carry the level of the band they happened in (`ONLY_LEVEL` is gone).
    - `MOB_BOXES` is one list per level: `set_mob_boxes(level, boxes)`; `mob_on_segment` uses the level of the segment's start, `mob_targets` returns every level's boxes in map space.
    - `set_spawn` / `spawn_point`: the world spawn in map space. `spawn_origin`, `spawn_clear_of` (and its free spot search) and `spawn::ground_spawn` use it instead of map origin, because the overworld origin's y is now clamped.
    - `kept_in_walls`: a player without collision (`MC_NOCLIP`, a spectator) is held between its band's walls after each move (`step.rs`).
    - `mark_level_switch` keeps `LEVEL_SWITCH` for the friend's prediction world (E20 stamps it when the friend's own dimension changes); the host never marks it now, since old ordnance stays in its own band.
  - `crates/minecraft_survival/src/bands.rs` (new): `level(dimension)` (0, 1, 2), `band`, `heights` (vanilla 26.3: the overworld from -64 with 384 blocks, the Nether and the End from 0 with 256), `wall_range`, `origin(dimension, world_spawn)` (the overworld at the spawn with its y clamped to 32..=320, the Nether at spawn x / 8, 1314, spawn z / 8, the End at 0, -1140, 0), `origins`, `check(world_spawn)` (every wall range inside its band, no bands overlapping), `at_bottom_wall` and `kept_in_walls` (a teleport's feet clamped into the wall range).
  - `crates/minecraft_survival/src/survival.rs`: `hazards` kills a player whose feet reach the bottom wall with `DamageCause::OutOfWorld` ("fell out of the world"), in any game mode, as the void would have done a little lower.
  - `crates/render_anim/src/minecraft_world.rs`:
    - At load the three origins are computed once from the world spawn (`bands::origins`) and kept in `runtime.origins` for the whole session, so `Login.origins` now carries all three. `bands::check` runs and warns if it fails. The overworld level is set right after `activate`, with the spawn point.
    - `follow_levels` sets a level's collision when the level opens (`finish_opening`) and drops it when the level closes (`close_empty_levels`). Showing a level no longer wipes the collision or the shape table; it only applies the stream's chunks to that level again.
    - Leaving a level (`Begun::Left`) clears that level's mob boxes instead of switching the whole block world.
    - The host frame takes only the voxel events of its own level (`event_level`). `apply_streamed`, `write_blocks`, `refresh_shapes`, `observe_spawn`, `stream_around_spawn`, `place_player`, `place_friend`, `MwBody` and the mining `WorldRefs` carry the level index.
    - `place_player` keeps the arrival's feet inside the walls.
  - `crates/render_anim/src/minecraft_players.rs`: a landing on the bottom wall takes no fall damage; command teleports are kept inside the walls (`minecraft_host.rs` too, for friends).
  - `crates/render_anim/src/minecraft_remote.rs`: the friend's game holds only its own level: `activate`, then `set_level` with the friend's dimension and its origin from the host.
  - `crates/sim/src/damage/height_bands.rs` (E4's spike): sets the level of the band it tests.
- **Tests:**
  - `sim` `voxel::tests`: `the_three_bands_cover_their_own_heights_and_nothing_past_them` (`band_of` for the three bands and their edges), `a_trace_in_one_band_never_hits_another_bands_blocks`, `the_wall_stops_a_trace_past_its_levels_range` (a trace down in the Nether stops at y -128 and one up at y 512, both inside the band), `a_flier_without_collision_stays_between_the_walls`, `every_dimension_keeps_its_own_level_and_dropping_one_leaves_the_others`; `a_voxel_shot_carries_its_attacker_and_every_event_its_level` now also checks that every event in the End's band carries level 2. The test level of the other voxel tests has the overworld's heights. `a_level_switch_drops_the_old_level_and_waits_for_ground_at_the_new_origin` is replaced by the level test above.
  - `minecraft_survival` `bands::tests`: `the_bands_hold_for_a_superflat_a_normal_and_the_highest_spawn` (`check` for a spawn at y -60, 64 and 320), `the_origins_are_fixed_by_the_world_spawn`, `the_design_numbers_for_every_wall`, `a_teleport_past_a_wall_stops_at_it`, `only_feet_on_the_bottom_wall_count_as_on_it`; `survival::tests::a_player_who_reaches_the_bottom_wall_falls_out_of_the_world_at_once` (survival and creative, through resistance, with the vanilla line).
  - `render_anim` `minecraft_world::band_tests`: `the_collision_bands_are_the_ones_each_dimension_is_given` (sim's `BANDS`, walls and block size match `bands`, and a point at each origin lies in its own band), `the_host_reads_each_events_level`.
  - Results on the real 26.3 data: `test -p sim` 115, `minecraft_survival` 570 + 1, `render_anim --lib` 137; `check -p launcher` green.
- **Decisions:**
  - **The band table is written twice**, in `sim::voxel::BANDS` and `minecraft_survival::bands::band`, because `minecraft_survival` does not depend on `sim`. A `render_anim` test pins them together.
  - **The walls use vanilla 26.3's dimension heights** (`bands::heights`), not the level's registries, so the origins, the walls and `check` are fixed per world and the same on the host and the friend.
  - **A new player and a respawn at the world spawn land on the spawn, not at map origin.** With the overworld origin's y clamped, a superflat spawn at y -60 is 92 blocks below map origin; the sim's spawn point (`set_spawn`) is the real spawn, so nobody falls from y 32.
  - **The bottom wall kills in every game mode.** The game already lets the void kill creative and spectator players (`creative_and_spectator_players_fall_out_of_the_world_past_a_totem`); that test now falls at y -191, above the overworld's wall at -192, since no player can be below it. A spectator is held between the walls by the step, so the top wall stops them too.
  - **Teleports are kept inside the walls** (vanilla allows any y): `/tp @s ~ 700 ~` in the overworld lands just under the top wall instead of in the End's band.
  - **The host frame drops the voxel events of other levels.** Only the host's level ticks until E20, which runs every occupied level and takes each level's events.
  - **The friend still re-activates its block world for every level it is shown**, as before; E20 makes the friend's dimension change drop the old level, set the new one and stamp the level switch.
- **Nothing from the B/C/D lists belongs to E19.** Friends sleeping goes with E20 (every occupied level ticking, the vanilla percentage rule per level); other players' snowflakes and the friend's own totem animation go with E21 or M6; the frost slowdown and powder snow walker for friends, frozen hearts in `PlayerView`, skating for friends, the lectern book screen and the merchant screen stay for M6 or later.
- **Test steps:** `docs/round8/e-m5-tests.md`, section E19.
- **Kit:** `minecraft:obsidian` x10, `minecraft:flint_and_steel` x1, `minecraft:iron_pickaxe` x1, `minecraft:cobblestone` x32, `minecraft:cow_spawn_egg` x2, `minecraft:cooked_beef` x8.

```
IW4L_MINECRAFT_KIT=minecraft:obsidian*10,minecraft:flint_and_steel,minecraft:iron_pickaxe,minecraft:cobblestone*32,minecraft:cow_spawn_egg*2,minecraft:cooked_beef*8
```

### E20. Every dimension with a player in it keeps running

Commit: see `git log` ("Every Minecraft dimension with a player in it keeps running"). The previous commit is `a4058b05` (E19).

- **Changed:**
  - `crates/minecraft_survival/src/server.rs`, the one server clock:
    - `Server.pending_batch` is gone; each `Level` has its own `pending_batch`. `plan_ticks(now, ticking)` plans one batch for every occupied level, advances the clocks and the weather once, counts the level ticks and the autosave once, and gives every ticking level the same batch and its container ticks. A level that still holds an unconsumed batch keeps new ticks from being planned (as before for the one level); a level that leaves the ticking set lets its batch go. `take_batch(dimension)`.
    - `join_friend` lost the host's dimension: a friend joins in the dimension and place their file (or `recently_left`) has, and their level opens for them through `open_levels`.
  - `crates/minecraft_survival/src/level.rs`: `Level.pending_batch`.
  - **The interim rules are gone** (section 5.3): `Player.travel_locked`, `travel_refused` and `refuse_travel`, the refusals in `respawn::respawn` and `/execute in` (`commands.rs`), `Sessions::lock_travel`, `TRAVEL_OFF`, `travel_off_line` and `JoinContext.dimension`.
  - `crates/minecraft_survival/src/session.rs`: `Session.dimension` (the level the friend's game shows, from `Login`) and `Session.epoch`. `Sessions::follow_levels(players, origins)`: when a friend's player is in another dimension and their place there is known, it sends `ToClient::Dimension { epoch: next, dimension, origin, feet }`, restarts the chunk sender in the new level (no forgets: the friend drops the old level whole), forgets what the mob sender sent and the cracks. `send_mobs` stamps each `StateFrame` with the session's epoch.
  - `crates/render_anim/src/minecraft_levels.rs` (new): the server side of a level frame for every occupied level the host's own frame did not tick this frame (`unticked`): the chunk stream around every player in it (`stream_tick`, with the host's view only for the level the host shows) into its collision band, container ticks, the voxel events of its band (friends' bullets on blocks and mobs, MW2 blasts, the dragon egg), the friends' harvest, the sleepers on every server tick (unless the host's own frame served them), the entity tick with every friend in it as an actor and a watcher, the level thread's block changes into the scene, the collision and the chunk map, the destroyed blocks, the mob boxes of its band, and the sounds of all that into the level's heard log. Nothing is drawn or played on the host.
  - `crates/render_anim/src/minecraft_world.rs`: the host frame plans ticks for every dimension with a player in it, takes its own level's batch from that level, marks the level it ticked (`Runtime.ticked`) and keeps the voxel events of other bands for the host system (`Runtime.events`) instead of dropping them. The End's fight ticks before the host's own step, so a fight with only a friend in the End goes on while the host travels. `Runtime.minings` holds a `Mining` per dimension the host does not show. `HostView` carries them.
  - `crates/render_anim/src/minecraft_host.rs`: friends are stepped in whatever dimension they are in (the "only in the host's shown level" skip is gone); after the friends' steps and commands, `tick_levels` runs every other occupied level and its lines go out through the sessions; `follow_levels` runs before the views, menus, mobs and chunks are sent. A host recording left running by an early return of the host frame is dropped before the friends' sounds play on the host.
  - `crates/render_anim/src/minecraft_players.rs`, portal and End travel per player:
    - `step_friend` runs `minecraft_portal::step` with the friend's feet (no UI: `step` takes `Option<&mut MinecraftUi>`), so friends travel through nether portals, End portals, exit portals and gateways; a friend's step stops on the frame their switch starts.
    - Leaving a level (`Begun::Left`) settles the friend with that level like the host's own trip: `Entities::leave` (picks and orbs in flight, `PlayerLeft` on the level thread, which forgets their tracked mobs), the menu closed with the overflow dropped at their feet, the lids they held closed (`level::leave`), and their hand state reset.
    - Arriving: once the friend stands in the new level and their arrival is placed, their switch ends.
    - Sounds recorded while a friend leaves or arrives (the portal travel sound) go to their new level's heard log instead of being lost.
    - `FriendWorld.shown` and `minings`: a friend in a level the host does not show is not heard on the host, and their break particles go to that dimension's `Mining`, never drawn; the host's sky light is used for placement only in its own level.
  - `crates/render_anim/src/minecraft_portal.rs`: `step` takes the UI as an option and no longer refuses travel.
  - `crates/render_anim/src/minecraft_remote.rs`, the friend's level switch: on a `Dimension` after the first level, the friend's block world drops the old level's band, sets the new one and stamps the level switch (`mark_level_switch`), keeping its shape table; the first level still activates the world. The vanilla loading screen (`LoadingBackgrounds`, the reason from `loading_reason`: the Nether swirl, the End screen, the plain one after a death) shows until the friend's spawn gate opens.
- **Tests:**
  - `minecraft_survival` `server::tests::two_levels_each_tick_once_per_server_tick_and_the_clocks_advance_once` (both levels hold the same batch and the same container ticks, the clocks and level ticks advance once, nothing new is planned while either level still holds its batch); `a_batch_left_by_an_early_return_is_sent_on_the_next_frame` now also checks that a level nobody ticks any more lets its batch go; `a_friend_saved_in_another_dimension_joins_there` replaces `..._joins_the_hosts` (the friend joins in the Nether at their saved place, their level is opened for them, and a rejoin comes back there).
  - `minecraft_survival` `session::tests::a_friends_portal_arrival_sends_dimension_with_the_next_epoch_and_starts_the_chunks_again` replaces `travel_is_refused_while_a_friend_is_online` (the portal sets off with a friend online; nothing is sent until the arrival is known; then `Dimension` with epoch 1, the Nether's origin and the arrival's feet, no forgets, the sent set and the batches in flight cleared; told once; a trip back sends epoch 2).
  - `minecraft_survival` `respawn::tests::a_friends_respawn_in_another_dimension_sets_off_their_own_switch` replaces `..._stays_in_this_level_while_travel_is_off`. `commands::tests::execute_in_is_refused_while_friends_are_online` is removed with the rule; `execute_in_teleports_the_player_into_another_dimension_through_the_switch` covers the trip.
  - The `remote` loopback test joins its friend in the End through a saved player file, since the host no longer places a joining friend in its own dimension.
  - `render_anim` `minecraft_levels::tests::every_open_level_with_a_player_in_it_ticks_once_and_the_hosts_own_level_is_left_alone` (one tick for a level with two friends, none for a level that is not open, the host's level only when its own frame did not tick it, nobody on the way counts), `minecraft_remote::tests::a_dimension_change_drops_only_the_old_level_behind_its_loading_screen`.
  - Results on the real 26.3 data: `test -p minecraft_survival` 570 + 1, `render_anim --lib` 139; `check -p launcher` green. Clippy on `render_anim` and `minecraft_survival` shows nothing on the lines this commit added.
- **Decisions:**
  - **The host's own level keeps its frame code**; every other occupied level runs the server-only `tick_levels`, after the friends' steps, as the host's level runs after the host's step. The host's own level falls back to it on a frame where the host's frame returned early (switching, waiting for its MW2 state), so friends in it keep playing (section 6.1: an early return skips only that player's step).
  - **`Dimension` is sent once the friend's place in the new level is known**, as vanilla sends the respawn packet when the server moves the player, which is after the portal search. Until then the friend's game keeps showing the old level, invulnerable and without input.
  - **A friend's trip settles them with the old level's thread** (`Entities::leave`), so the tracker forgets them there and they get every mob again when they come back; without it a level that stopped ticking would have kept their tracked set.
  - **Sleeping:** `SleepStatus` already counts every non-spectator player in the level (vanilla's `SleepStatus.update`); every occupied level now ticks its sleepers on the server clock, so a friend in the Nether no longer keeps the host's night from passing, and a friend in the host's level does (with `players_sleeping_percentage` 100). Friends still cannot lie down (M6).
  - **Friends skip the credits until E22:** a friend leaving through the End's exit portal goes straight to their respawn point, and their `seenCredits` stays false so they see the credits on a later trip once E22 shows them.
  - **A level the host does not show** decides whether monsters burn and spawn by its own sky light from the clock and the weather (`level_sky_light`), since its environment is not stepped by a presenter.
- **Left for later:**
  - E21: other players in another dimension are still drawn (far away) and still count as MW2 obstacles; the host's body gear already hides a friend in another dimension.
  - The purple portal overlay on a friend's screen while they stand in a portal (vanilla computes it on the client); the friend hears the trigger sound.
  - Friends lying down in beds (sleep screen, Leave Bed, the lying body for every player), the frost slowdown and powder snow walker replicated to friends, frozen hearts in `PlayerView`, skating for friends, the lectern and merchant screens: M6 or later. Other players' snowflakes and the friend's own first-person totem animation: E21 or M6.
  - The boss bar and credits per player: E22.
- **Test steps:** `docs/round8/e-m5-tests.md`, section E20.
- **Kit:** `minecraft:obsidian` x10, `minecraft:flint_and_steel` x1, `minecraft:iron_pickaxe` x1, `minecraft:iron_sword` x1, `minecraft:cobblestone` x32, `minecraft:cow_spawn_egg` x2, `minecraft:white_bed` x1, `minecraft:cooked_beef` x8.

```
IW4L_MINECRAFT_KIT=minecraft:obsidian*10,minecraft:flint_and_steel,minecraft:iron_pickaxe,minecraft:iron_sword,minecraft:cobblestone*32,minecraft:cow_spawn_egg*2,minecraft:white_bed,minecraft:cooked_beef*8
```

### E21. Players in another dimension are not drawn

Commit: see `git log` ("Players in another Minecraft dimension are not drawn"). The previous commit is `87eecc0e` (E20).

- **Changed:**
  - `crates/sim/src/voxel.rs`: the band gate, pure: `shown_to(viewer_z, z)` is true when there is no viewer or both map heights fall in the same band (`band_of`); `viewer_z(local_z)` gives the viewer's height only while a block world stands in for the map, so MW2 matches without Minecraft are never gated.
  - `crates/render_anim/src/occupancy/remote_body.rs`: `occupy_remote_scene_ents` and `sync_remote_bodies` skip a body (and a corpse) whose presented origin is in another band than the local player's. `sync_remote_bodies` removes its `RemotePlayer` like any other body that is not submitted, so it is not posed either and publishes no head for the names and no posed gear.
  - `crates/render_anim/src/anim/minecraft_gear.rs`: `gear_rigs` and `gear_meshes` take the viewer's height and leave out a posed body whose joints are in another band. The posed gear frame keeps a client's last pose until it is posed again, so without this a friend's armour and held item would have stayed at the last place they were posed in the old dimension.
  - `crates/render_anim/src/minecraft_world.rs`: `draw_body_gear` passes the local player's height.
  - `crates/hud/src/overhead_names.rs`: a player in another band is not a name candidate (checked before the posed head, so no head gap is raised for them).
  - `crates/render_anim/src/minecraft_players.rs`: `obstacles` keeps only other MW2 players in the band of the player whose obstacles they are. Before, a friend in the Nether or the End counted as a body in the host's level at the same block coordinates of their own level, and could refuse a block placed there.
- **Tests:**
  - `sim` `voxel::tests::only_bodies_in_the_viewers_band_are_shown_to_them` (the pure gate for the three bands, past every band, and without a viewer).
  - `render_anim` `minecraft_gear::tests::gear_is_drawn_only_on_bodies_in_the_viewers_band` (three posed bodies, one per band: each viewer gets only its own band's rig, and no viewer gets all three); the two older `gear_rigs` tests pass no viewer.
  - Results on the real 26.3 data: `test -p sim` 116, `render_anim --lib` 140, `hud --lib` 148; `check -p launcher` green.
- **Decisions:**
  - **The viewer is the local player's presented origin**, not the camera: every gate in a frame reads the same height, so the body, its gear and its name come and go together. A frame with no local player state gates nothing.
  - **The gate only runs while a block world is active** (`viewer_z`), so MW2 maps keep drawing every player as before.
  - **Gear is gated by its own posed joints** rather than by the client, so it is hidden exactly where the body that carried it is drawn, including a pose left over from the old dimension.
  - **The thermal-sight markers are not gated**: they are MW2 only and need the thermal scope, which a Minecraft world does not give.
  - **Other players' snowflakes and the friend's own first-person totem animation are not part of E21**: they are about sending per-player effects, not about dimensions. They go to M6 or later with the frost slowdown and powder snow walker for friends, frozen hearts in `PlayerView`, skating for friends, friends lying down in beds, and the lectern and merchant screens.
- **Test steps:** `docs/round8/e-m5-tests.md`, section E21, which ends with the design's Owner test M5.
- **Kit:** `minecraft:obsidian` x10, `minecraft:flint_and_steel` x1, `minecraft:iron_pickaxe` x1, `minecraft:cobblestone` x32, `minecraft:iron_helmet` x1, `minecraft:iron_chestplate` x1, `minecraft:torch` x16, `minecraft:cooked_beef` x8.

```
IW4L_MINECRAFT_KIT=minecraft:obsidian*10,minecraft:flint_and_steel,minecraft:iron_pickaxe,minecraft:cobblestone*32,minecraft:iron_helmet,minecraft:iron_chestplate,minecraft:torch*16,minecraft:cooked_beef*8
```

## Adversarial review of M5

The review covers every commit from `38c7339b` (E19, E20 and E21). Each fix below is its own commit. The owner's steps are in `docs/round8/e-m5-tests.md`, section "Review fixes".

### Review fix: a friend arriving alone in a dimension gets its chunks

Commit: see `git log` ("A friend arriving alone in a Minecraft dimension gets its chunks"). The previous commit is `d2bc0e76` (E21).

- **Defect:** `minecraft_levels::unticked` left out every player with a switch, so a level whose only player was a friend on the way in (through a portal, `/execute in` or a respawn in another dimension) was never run. Nothing streamed around their arrival: no chunk came into the level's collision band, so `place_friend` waited for `chunks_ready_around` for ever, and no chunk was generated for the friend's chunk sender either. The friend stayed on the loading screen, invulnerable and frozen, until another player came into that dimension. This is E20's own step 2 (a friend going into the Nether alone) and step 10 (`/execute in` while the host stays in the overworld); the host's own trips were not hit, since the host frame streams its arrival itself (`stream_around_spawn`).
- **Fix:** a player whose switch has left the old level (`Switch.left`) counts for the level they are arriving in, so that level streams around their arrival, applies the chunks to its band and ticks (with no batch of its own, as before, since the server clock still plans only for players who are not switching). A player who set off this frame and has not left yet still counts for no level.
- **Test:** `minecraft_levels::tests::a_friend_arriving_alone_keeps_their_new_level_streaming` (a friend set off for the Nether counts nowhere until they have left, then the Nether runs for them while the host's overworld is left to the host frame).
- **Results:** `test -p render_anim --lib` green on the real 26.3 data; `check -p launcher` green.
- **Test steps:** `docs/round8/e-m5-tests.md`, section "Review fixes", step R1.
- **Kit:** the E20 kit.

### Review fix: a friend's menu is not served in the level they are travelling to

Commit: see `git log` ("A friend's Minecraft menu is not served while they travel"). The previous commit is `e11696bc` (the arrival streaming fix).

- **Defect:** E20 lets a friend set off through a portal inside `step_friend`, which changes `player.dimension` at once. The host loop then ran `friend_menus` for that friend in the same frame, against the level they are going to: their queued menu clicks were applied to that level's containers, the open menu was checked against that level's blocks (so it closed there), its overflow was tossed into that level at a point worked out with the old level's origin, and the lid of the chest they had open was closed in the wrong level. The same held on every frame of the trip and after a respawn in another dimension. Vanilla serves a player's menu only in the level the player is in, and `leave_level` already closes the menu and drops its overflow at the friend's feet in the level they left.
- **Fix:** `friend_menus` does nothing while the friend has a switch; their menu is settled with the old level by `leave_level` on the next frame, and served again once they stand in the new level.
- **Test:** none new: `friend_menus` needs the MW2 authority and a live level, which the unit tests do not build. R2 checks it in game.
- **Results:** `test -p render_anim --lib` green on the real 26.3 data; `check -p launcher` green.
- **Test steps:** `docs/round8/e-m5-tests.md`, section "Review fixes", step R2.
- **Kit:** the E20 kit and `minecraft:chest` x1; the line in the "Review fixes" section of `docs/round8/e-m5-tests.md` holds both.

### Final verification of M5: clippy on the track's own lines

Commit: see `git log` ("Multiplayer M5 passes clippy"). The previous commit is `6571e5db` (the friend's menu fix).

- **Clippy** on every crate M5 changes (`hud`, `minecraft_survival`, `render_anim`, `sim`) with `--all-targets -A clippy::approx_constant`. Four warnings were on lines this run added (E19), all `type_complexity`; none on E20, E21 or the review fixes. Older warnings stay.
- **Changed (no behaviour change):**
  - `render_anim::minecraft_world`: `apply_streamed`, `write_blocks` and `refresh_shapes` take their shape tables and level as `ShapeTables<'_>`, a type alias for the same tuple.
  - `sim::voxel`: `MOB_BOXES` holds `[MobBoxes; LEVELS]`, a type alias for one level's list.
- **Tests:** none new. `test -p` of each touched crate is green on the real 26.3 data: `hud` 148, `minecraft_survival` 570 + 1, `render_anim` 141, `sim` 116. `check -p launcher` is green. The new files (`bands.rs`, `minecraft_levels.rs`) still pass `rustfmt --check` on their staged content.
- **Combined kit:** the section below now covers the M5 owner tests.
- **Test steps:** none new; `docs/round8/e-m5-tests.md` (E19 for the mob boxes, E19 and E20 for the streamed chunks and block writes) covers these lines.
- **Kit:** none needed.

## Milestone M6: the End and finishing

This run builds E22, E23 and E24 and the per-player leftovers, the last run of the track before its merge and release build 2. The owner's two-window steps are collected in `docs/round8/e-m6-tests.md`.

### E22. The dragon fight and the credits work for every player

Commit: see `git log` ("The Minecraft dragon fight and the credits work for every player"). The previous commit is `beb54197` (M5 passes clippy).

- **Changed:**
  - `crates/minecraft_net`: `ToClient::Bursts { entities: Blob }` (the one-shot effects of a level snapshot, as an `EntitySnapshot` holding only them) and `PROTOCOL` 2 to 3, so a friend on an older build is refused at `Hello`. Design sections 4 and 9.4 follow.
  - `minecraft_survival/src/the_end/dragon_fight.rs`: `shown_bar(fight, player)`, the one rule for whose screen shows the dragon's bar: a player in the End, not switching, whom the fight's scan counted (vanilla's `EndDragonFight.updatePlayers`: alive and within 192 blocks of 0, 128, 0). The host's own bar (`minecraft_end::tick_fight`) uses it too.
  - `minecraft_survival/src/session.rs`:
    - `Session.send_boss_bar` sends vanilla's boss event operations for the dragon's bar (`DRAGON_BAR`, id 1): `Add` (name `entity.minecraft.ender_dragon`, pink, progress overlay, boss music and world fog, as vanilla's `ServerBossEvent` for the dragon) when the bar appears for that player, `Progress` when the health changes, `Remove` when the player leaves the range or the End, or starts a trip. `Sessions::send_boss_bars(players, fight)`.
    - `Session.send_credits` sends `Credits` once when that player's credits hold starts (`the_end::holding_for_credits`); `Sessions::send_credits(players)`.
    - `Sessions::credits_done(players, client)`: a friend's `CreditsDone` (vanilla's `PERFORM_RESPAWN` after the win screen) ends that friend's hold only, and only while they are held.
    - `send_mobs` takes the one-shot effects out of the level's friend snapshot before the state frames are built, and sends each friend in that level the ones in their range as `Bursts` on the reliable stream (the dragon's always, a shattered eye within the thrown range and in a chunk they have).
  - `minecraft_survival/src/mob_sender.rs`: `take_bursts`, `keep_bursts`, `has_bursts`, `bursts_message`.
  - `minecraft_survival/src/entities.rs`: when the level thread hands out several outputs in one host frame, the friend snapshot keeps the one-shot effects of the earlier ones (`keep_bursts`) instead of losing them to the newest.
  - `minecraft_survival/src/entity_view.rs`: `EntityView::bursts` presents the one-shot effects (breath bursts, mouth breath, dragon explosions, shattered eyes); `receive` calls it, so the host draws them exactly as before. `minecraft_terrain/src/eye_of_ender.rs`: `EyeTrails::shatter`, taken out of `receive`.
  - `minecraft_survival/src/remote.rs`: `RemoteWorld.boss_bars` (`BossBar`), kept by the boss event operations (`Add`, `Remove`, `Progress`, `Name`, `Style`, `Flags`; an operation for an unknown bar is ignored, as vanilla's `BossHealthOverlay` does); `Bursts` is presented in the friend's level in stream order.
  - `minecraft_survival/src/server.rs`: `Server::leave` ends a credits hold (design 5.2 step 7).
  - `render_anim/src/minecraft_players.rs`: a friend's exit portal trip is no longer cut short. Their credits hold keeps them in their arrival (invulnerable, no input) like the host's, until `CreditsDone`; the E20 stopgap that skipped the credits and reset `seenCredits` is gone.
  - `render_anim/src/minecraft_host.rs`: reads `CreditsDone` and applies it after the leaves; sends the credits and the boss bars after `follow_levels`.
  - `render_anim/src/minecraft_end.rs`: `roll(name, ui, packs, images)`, the credits screen shared by the host and the friend.
  - `render_anim/src/minecraft_remote.rs`: the friend's game draws the host's boss bars (`follow_boss_bars`: the name in the friend's language, sliding to each new value), uses their world fog and the open credits for its sky (the End flash sound waits while the credits roll, as on the host), rolls the credits on `Credits` with the friend's own name, takes their input away while they roll, sends `CreditsDone` when they end or are skipped (at once if the GUI font is missing, like the host's skip), and clears the credits and bars when the match ends.
- **Tests:**
  - `minecraft_survival` `session::tests`: `the_dragon_bar_reaches_only_the_players_near_the_fight_and_follows_its_health` (a player near the fight gets `Add` once, `Progress` once per change, `Remove` when their trip starts and nothing after; a player in the End out of range and a player counted by the fight but in the overworld get nothing; the bar's name and flags), `the_credits_roll_once_per_player_and_credits_done_releases_only_that_player`, `a_friend_leaving_during_the_credits_is_released_and_saved_as_having_seen_them` (real 26.3 data: the leaving friend's hold ends, their saved data keeps `seenCredits`, the other friend still watches).
  - `minecraft_survival` `mob_sender::tests::the_dragons_one_shot_effects_leave_the_state_frame_and_are_kept_until_sent`; `remote::tests::the_boss_bar_follows_the_host_and_the_dragons_bursts_show_in_the_level`.
  - `render_anim` `minecraft_remote::tests::the_friends_boss_bars_follow_the_hosts_and_slide_to_a_new_health`.
  - `minecraft_net` `codec::tests` cover `Bursts`.
  - Results on the real 26.3 data: `test -p minecraft_net` 15, `minecraft_survival` 575 + 1, `minecraft_terrain --lib` 1111 (3 ignored), `render_anim --lib` 142; `check -p launcher` green.
- **Decisions:**
  - **The one-shot effects travel as a new `Bursts` message** rather than as vanilla's separate particle and level-event packets: they are already shaped as part of `EntitySnapshot`, the friend presents them with the host's own code, and one message keeps them in the reliable order with the `Dimension` they belong to. The protocol number goes to 3 for it.
  - **A friend's credits hold behaves like the host's:** they stay in their trip at the respawn point, invulnerable and without input, until their credits end. Vanilla removes the player from every level during the win screen; here the friend's body waits at the arrival, hidden by the trip.
  - **Leaving during the credits** ends the hold in `Server::leave`; `seenCredits` was already set when the credits started, as vanilla sets it when it sends the win screen. A friend who left before their arrival was placed comes back at the exit portal in the End (vanilla saves the player there too) and goes home without the credits; one who left after it comes back at their spawn.
  - **The bar's id is a constant (1):** there is one dragon fight per world. Vanilla gives the dragon's `ServerBossEvent` a random UUID; nothing on the friend's side reads it.
  - **The friend's game has no music**, so it plays neither the boss music nor the credits music. That is older than E22 (the friend's game has no music system at all) and goes to the limitations.
- **Left for later:** music on the friend's game (any music).
- **Test steps:** `docs/round8/e-m6-tests.md`, section E22.
- **Kit:** `minecraft:bow` x1, `minecraft:arrow` x64, `minecraft:diamond_sword` x1, `minecraft:diamond_helmet` x1, `minecraft:diamond_chestplate` x1, `minecraft:diamond_leggings` x1, `minecraft:diamond_boots` x1, `minecraft:golden_apple` x8, `minecraft:cooked_beef` x32, `minecraft:cobblestone` x64, `minecraft:snowball` x16, `minecraft:ender_pearl` x16, `minecraft:eye_of_ender` x12, `minecraft:water_bucket` x1.

```
IW4L_MINECRAFT_KIT=minecraft:bow,minecraft:arrow*64,minecraft:diamond_sword,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*8,minecraft:cooked_beef*32,minecraft:cobblestone*64,minecraft:snowball*16,minecraft:ender_pearl*16,minecraft:eye_of_ender*12,minecraft:water_bucket
```

### Leftover: friends freeze in powder snow like the host

Commit: see `git log` ("Friends freeze in powder snow like the host"). The previous commit is `4221598e` (E22).

- **Changed:**
  - `crates/minecraft_net`: `PlayerView.frozen`, the player's ticks frozen (vanilla syncs `TICKS_FROZEN` as entity data to every client). `docs/MULTIPLAYER.md` section 4 follows. The protocol stays 3: E22 raised it in this same run, and no build with protocol 3 has been released yet.
  - `minecraft_survival/src/views.rs`: `player_view` fills it; `HudClock::vitals` turns it into the frost outline's strength and the fully frozen hearts (140 ticks), as the host's `Survival::vitals` does.
  - `minecraft_survival/src/survival.rs`: `percent_frozen(ticks)` and `frost_scale(ticks, on_block)`, vanilla's `LivingEntity.tryAddFrost` slowdown (up to half the walk speed while frozen with a block under the feet), readable from the ticks alone, so the friend's game can compute it.
  - `render_anim/src/minecraft_players.rs`: `frost_scale(scene, feet, ticks, alive)` (the block 0.2 under the feet, as before) for every game. `step_friend` now sets a friend's powder snow walker (leather boots) and frost slowdown in the host's MW2 world for that friend's client, as the host frame does for the host.
  - `render_anim/src/minecraft_world.rs`: the host's own slowdown uses the same `frost_scale`.
  - `crates/net`: `ClientPrediction::set_minecraft_move_scale`, so a friend's game slows its own predicted walk.
  - `render_anim/src/minecraft_remote.rs`: the friend's game sets that scale each frame from its `PlayerView.frozen` and the block under its predicted feet. The powder snow walker needs nothing on the friend's side: it is a `pm_flags` bit of the friend's own `PlayerState`, which the host's snapshot carries.
- **Tests:**
  - `minecraft_survival` `survival::tests::the_frost_slows_only_a_player_standing_on_a_block_and_any_game_reads_it_from_the_ticks`, `views::tests::a_friends_frost_reaches_their_hearts_and_outline`.
  - `sim` `frame::tests::a_predicting_game_keeps_its_own_frost_through_the_hosts_snapshots` (the slowdown is client meta that no snapshot carries, so the friend's own value survives each adoption).
  - `minecraft_net` `codec::tests` carry a frozen view.
- **Decisions:**
  - **The friend computes its own slowdown** from the synced ticks and its predicted feet, as vanilla's client applies the frost modifier itself from `TICKS_FROZEN`; the host computes the same from its authoritative feet. A difference of one round trip at the edge of the snow is corrected by the next snapshot like any other prediction error.
  - **The ticks ride in `PlayerView`**, which is sent again whenever it changes: while a player freezes or thaws that is once a tick (as vanilla's entity data), and nothing while it stays the same.
- **Test steps:** `docs/round8/e-m6-tests.md`, section "Friends freeze in powder snow like the host".
- **Kit:** `minecraft:powder_snow_bucket` x1, `minecraft:leather_boots` x1, `minecraft:cobblestone` x64.

```
IW4L_MINECRAFT_KIT=minecraft:powder_snow_bucket,minecraft:leather_boots,minecraft:cobblestone*64
```

### Leftover: friends see snowflakes around every player in powder snow

Commit: see `git log` ("Friends see snowflakes around every player in powder snow"). The previous commit is `e130f5a5` (friends freeze in powder snow).

- **Changed:**
  - `minecraft_survival/src/entity_view.rs`: `powder_snow_flake` moved here from `Entities` unchanged, and `snowflakes(world, feet, height)`: vanilla's `PowderSnowBlock.entityInside` on the client, one `snowflake` with a chance of one half for each powder snow block the body is in while its feet are, rising from the top of that block.
  - `render_anim/src/minecraft_players.rs`: the host's player step calls `snowflakes` (the same rolls as before, from the same random).
  - `render_anim/src/minecraft_remote.rs`: each client tick, the friend's game makes the snowflakes of every living player whose feet moved sideways since the last tick, its own body included, from the bodies in its MW2 snapshot (`Flakes`: the last feet per client and the last tick seen; a new level forgets them).
- **Tests:** `minecraft_survival` `remote::tests::a_body_moving_in_powder_snow_makes_snowflakes_on_any_game`.
- **Decisions:**
  - **Each game makes the snowflakes itself**, as vanilla's client does for every entity it sees, so nothing is sent. The host already made them for every player in its level (each player's step runs on the host).
  - **A frame with several client ticks rolls once a tick, at most four,** with the body where the frame shows it; the friend has no per-tick positions of other players.
- **Test steps:** `docs/round8/e-m6-tests.md`, section "Friends see snowflakes around every player in powder snow".
- **Kit:** `minecraft:powder_snow_bucket` x1, `minecraft:cobblestone` x64.

```
IW4L_MINECRAFT_KIT=minecraft:powder_snow_bucket,minecraft:cobblestone*64
```

### Leftover: a friend sees their own totem rise in first person

Commit: see `git log` ("A friend sees their own totem rise in first person"). The previous commit is `a1a6dc3f` (snowflakes).

- **Changed:**
  - `minecraft_survival/src/entity_view.rs`: `held_item_mesh` moved here from `Entities` unchanged (`Entities` keeps a call to it), and `random_float`.
  - `render_anim/src/minecraft_hand.rs`: `append_totem` and `hand_clip`, the host frame's totem mesh (the item's `fixed` display under the activation pose, full bright) and first-person projection, moved here unchanged so both games use them; `TOTEM` and `FULL_BRIGHT` moved with them.
  - `render_anim/src/minecraft_remote.rs`: the friend's game starts its own `ItemActivation` when the host's totem player event (vanilla's entity event 35) names the friend's own client, ticks it on its client ticks, and draws it as the first-person hand mesh, hidden with the HUD as on the host.
- **Tests:** `render_anim` `minecraft_remote::tests::only_the_friends_own_totem_rises_in_their_view`.
- **Decisions:**
  - **Vanilla's client plays the activation on entity event 35 for its own player** (`ClientPacketListener.handleEntityEvent` calls `GameRenderer.displayItemActivation` for the local player only), so the friend's game reacts to the event the host already sends; nothing new goes over the wire. Other players' totems keep showing as particles on their body.
  - **The friend's first-person hand shows only the totem.** The friend's game draws no other held item in first person yet (E12's list); that is older than this commit and stays in the limitations.
- **Test steps:** `docs/round8/e-m6-tests.md`, section "A friend sees their own totem rise in first person".
- **Kit:** `minecraft:totem_of_undying` x1.

```
IW4L_MINECRAFT_KIT=minecraft:totem_of_undying
```

### Leftover: friends see the mob spinning in spawner cages

Commit: see `git log` ("Friends see the mob spinning in spawner cages"). The previous commit is `a4ebf119` (the friend's totem).

- **Changed:**
  - `minecraft_survival/src/remote.rs`: the friend's level feeds its `SpawnerDisplays` (round 8 track A's spawner render) from the stream: the spawner block entities of every chunk a batch loads (the chunk blob already carries them, as vanilla's chunk packet carries their update tags), a `BlockEntity` from the host, a block update that turns the block into something other than a spawner, and a forgotten chunk.
  - `minecraft_survival/src/entities.rs`: `Entities.spawner_tags`, the spawner tags the level thread hands out (a spawn egg changing the mob, a spawn resetting the delay), kept only while a friend listens in that level.
  - `minecraft_survival/src/session.rs`: `send_heard` sends them with `Sessions::send_spawners` as `BlockEntity` to each friend in that level who has the chunk (vanilla's `ClientboundBlockEntityDataPacket` from `sendBlockUpdated`).
  - `minecraft_terrain/src/spawner_display.rs`: `is_spawner` is public.
- **Tests:** `minecraft_survival` `remote::tests::a_spawner_streamed_to_the_friend_shows_its_cage_until_the_block_or_chunk_goes`, `session::tests::a_spawners_new_data_reaches_friends_in_that_level_with_that_chunk`.
- **Decisions:**
  - **The friend's game spins and smokes the cage itself**, near its own player, from the spawner's data, as vanilla's client does (`BaseSpawner.clientTick`). The host sends the data again when the level thread changes it; a spawn then resets the friend's spin delay as on the host (vanilla sends block event 1 for that; here the changed tag carries the same reset).
- **Test steps:** `docs/round8/e-m6-tests.md`, section "Friends see the mob spinning in spawner cages".
- **Kit:** `minecraft:spawner` x1, `minecraft:zombie_spawn_egg` x1.

```
IW4L_MINECRAFT_KIT=minecraft:spawner,minecraft:zombie_spawn_egg
```

### Leftover: friends read books on lecterns

Commit: see `git log` ("Friends read books on lecterns"). The previous commit is `b082f71f` (spawner cages).

- **Changed:**
  - `crates/minecraft_net`: `ToClient::Lectern(Option<BookView>)` and `ToServer::Lectern { pos, button }`; `BookView { pos, pages, page, take }`. Design section 4 follows. The protocol stays 3 (see the frost entry).
  - `minecraft_survival/src/session.rs`: `Session.lectern` (the lectern a friend has open) and `friend_lectern(open, clicks, views, eye, may_build)`, the host's side of a friend's book screen: a click counts only for the open lectern; a button goes to the level thread as `LecternAction::Button`, Take Book only for a player who may build (vanilla `LecternMenu.clickMenuButton`); no button closes; a friend out of reach of their lectern is closed (vanilla `stillValid`, the host's own `within_table_reach`); each view from the level thread opens or updates the friend's screen with the pages as text, or closes it, and a taken book is handed back.
  - `minecraft_survival/src/remote.rs`: `RemoteWorld.lecterns` keeps the screens to show in order.
  - `render_anim/src/minecraft_players.rs`: `step_friend` takes the friend's open lectern and their lectern clicks and runs `friend_lectern` instead of closing every view at once; the taken book goes into the friend's inventory as before. A friend who leaves the level closes their lectern.
  - `render_anim/src/minecraft_host.rs`: reads `ToServer::Lectern` and sends the friend's screens.
  - `render_anim/src/minecraft_inventory.rs`: `InventoryUi::lectern_clicks`, the friend's book screen input (its buttons and closing; other clicks are ignored while it is open, as on the host).
  - `render_anim/src/minecraft_remote.rs`: the friend's game opens, updates and closes the book screen from the host (`show_book`, the host's own `show_lectern`), sends its buttons and its close, and sends a close back when another screen is already open.
- **Tests:**
  - `minecraft_survival` `session::tests::a_friends_lectern_turns_pages_closes_and_gives_the_book_only_to_a_builder`.
  - `render_anim` `minecraft_inventory::tests::a_friends_lectern_screen_sends_its_buttons_and_closes_once`.
  - `minecraft_net` `codec::tests` cover both messages.
- **Decisions:**
  - **The pages travel as plain text** (`pages_text`, what the host's screen shows), not as the book item: the friend's screen needs nothing else, and the host already resolves them the same way for itself.
  - **The Take Book check happens on the host** from the friend's mode, as vanilla's server checks `mayBuild`; the button is also hidden on the friend's screen when they may not build.
  - **A view that was already on its way when the friend closed can open the screen again for a moment** (a page turned by another reader in the same tick). The friend closes it again with Escape; vanilla has the same window with its container ids.
- **Test steps:** `docs/round8/e-m6-tests.md`, section "Friends read books on lecterns".
- **Kit:** `minecraft:lectern` x1, `minecraft:written_book` x1 (three pages).

```
IW4L_MINECRAFT_KIT=minecraft:lectern,minecraft:written_book[written_book_content={title:"Test",author:"Owner",pages:["Page one text.","Page two text.","Page three text."]}]
```

### Leftover: friends lie down in beds and count for the night

Commit: see `git log` ("Friends lie down in beds and count for the night"). The previous commit is `79ca1533` (lecterns).

- **Changed:**
  - `crates/minecraft_net`: `ToClient::Sleeping { client, bed: Option<Lying>, timer }`, `Lying { feet, facing }` and `ToServer::LeaveBed`. Design section 4 follows. The protocol stays 3.
  - `minecraft_survival/src/sleep.rs`: `next_sleep_timer`, one client tick of vanilla's `sleepCounter` (up to 100 while asleep, then on to 110 and back to 0 after getting up), as `tick_sleepers` counts it on the host.
  - `minecraft_survival/src/session.rs`: `Sessions::send_sleepers(players, host)` and `Session::send_sleepers`: each friend learns who lies in a bed in the level their game shows (the host by its MW2 client), when it changes, with their own timer on their own entries; a new level forgets what was sent.
  - `minecraft_survival/src/remote.rs`: `RemoteWorld.sleepers`, `sleep_timer` (counted on with the client clock) and `lying()`; a new level clears the sleepers.
  - `render_anim/src/minecraft_players.rs`: a friend's bed use (`Acted.bed`, already ruled by night, monsters and the rest) comes back from `step_friend`. `friend_beds` lies them down (after a dismount) or gets them up with `sleep::start` / `stop` in their own level and refreshes the bed's collision; `lay_friends` pins every sleeping friend's MW2 player on the pillow with `ride`, lets it go once when they get up, and adds the lying body of each friend in the host's level to `McSleepers.bodies`.
  - `render_anim/src/minecraft_host.rs`: reads `LeaveBed`, hands the bed actions to the host frame (`Runtime.friend_beds`) and sends the sleepers each frame.
  - `render_anim/src/minecraft_world.rs`: the host frame runs the friends' bed actions before its own sleep step, so their overlay lines ("1/2 players sleeping") go to everyone through the same relay, and lays the friends after the host.
  - `render_anim/src/minecraft_remote.rs`: the friend's game draws every lying player (`McSleepers.bodies`), puts its own camera on the pillow, darkens with its own timer (`ui.sleep`), opens the chat with the Leave Bed button while asleep, sends `LeaveBed` from it, and closes it when it wakes.
- **Tests:**
  - `minecraft_survival` `sleep::tests::the_sleep_timer_deepens_to_a_hundred_and_the_wake_up_fades_out_by_a_hundred_and_ten`, `session::tests::friends_learn_who_lies_in_a_bed_and_their_own_wake_up_timer`, `remote::tests::the_friend_sees_who_lies_in_a_bed_and_wakes_with_the_hosts_fade`.
  - `render_anim` `minecraft_players::tests::friends_in_bed_lie_in_the_hosts_level_and_are_let_go_when_they_wake`.
  - `minecraft_net` `codec::tests` cover the new messages.
- **Decisions:**
  - **Lying is sent as the feet and the bed's facing,** what vanilla syncs as the sleeping position; each game builds the lying body and the pillow camera from them with the host's own functions (`minecraft_sleep`).
  - **The timer runs on each game,** as vanilla's client counts its own `sleepCounter`; the host sends it only when lying down and getting up (0 or 100), so a morning wake-up fades and a Leave Bed does too, as on the host.
  - **A friend's bed actions run in the host frame,** next to the host's own, one frame after they arrive, so the overlay lines and the sleep status go through the one path that already handles them.
- **Test steps:** `docs/round8/e-m6-tests.md`, section "Friends lie down in beds and count for the night".
- **Kit:** `minecraft:white_bed` x1.

```
IW4L_MINECRAFT_KIT=minecraft:white_bed
```

### Leftover: with pvp off, a player's harming potion hurts no player

Commit: see `git log` ("With pvp off, a player's harming potion hurts no player"). The previous commit is `7b90520f` (sleeping).

- **Changed:** `minecraft_survival/src/entities.rs`: `unharmed_by_potions_without_pvp`, next to E18's `unharmed_without_pvp`: with the `pvp` rule off, the instant harm of a splash or lingering potion whose thrower is a player in the level reaches no player (the thrower included, as vanilla's `canHarmPlayer` has no exception for oneself). Its other effects (poison, slowness, weakness) still land, and a witch's potion still harms. Design section 13.4 follows.
- **Tests:** `minecraft_survival` `entities::tests::with_pvp_off_a_players_harming_potion_spares_every_player_but_other_effects_land`.
- **Decisions:**
  - **Only the harm is refused,** as vanilla refuses the hurt (`ServerPlayer.hurtServer` with an `indirectMagic` source whose entity is the thrower) but adds the other effects with no pvp check.
  - **TNT lit by a player is not covered:** the vendored level's primed TNT keeps no igniter (vanilla's `PrimedTnt.owner`), so nothing names the player behind its blast; it stays in the design's "Not covered" and in section 20. Minecraft melee between players stays deferred too (it does not exist yet, pvp or not).
- **Test steps:** `docs/round8/e-m6-tests.md`, section "With pvp off, a player's harming potion hurts no player".
- **Kit:** `minecraft:splash_potion@harming` x1, `minecraft:splash_potion@poison` x1.

```
IW4L_MINECRAFT_KIT=minecraft:splash_potion@harming,minecraft:splash_potion@poison
```

### Leftovers that stay for later, written down

Commit: see `git log` ("The multiplayer leftovers that stay for later are written down"). The previous commit is `30564a2c` (harming potions with pvp off).

- **Changed:**
  - `docs/MULTIPLAYER.md`: section 19 lists the per-player leftovers of M2 to M5 done in M6; section 20 adds skating for friends, the friend's first-person hand, music on the friend's game, TNT lit by a player with pvp off and a gun hit waking a sleeper, each with its reason.
  - `docs/MINECRAFT-SURVIVAL.md`: the multiplayer limitation that listed the per-player state still to send now lists only what friends still lack. The rest of that section ("Multiplayer today") is E24's.
  - `docs/round8/e-m6-tests.md`: the totem steps start with `/op Friend` (the friend's `/tp` needs it); the sleeping section's step "Hit in bed" is gone, because a gun hit does not wake a sleeper on the host either (below); the intro points to the combined kit.
- **Not done in M6, and why:**
  - **Skating for friends.** A friend's game has no `AuthorityWorld`, so its skate worker cannot move the friend's MW2 player; it needs a client-reported origin checked by the host (an external-motion command through the MW2 netcode), the skating bits per player in the Minecraft rules, and the skater's pose and board sent to the other games every frame. Track D's brief (section 6) put the pose out of round 8. This is several commits of MW2 netcode, so it goes to a later round.
  - **The merchant screen.** Not wired in single player either; it stays in section 20 ("Trading screens").
  - **Minecraft melee between players.** There is no Minecraft hit on a player yet (pvp or not): `entities.punch` reaches mobs only, and a player target needs a hurt path with armour and shields. Section 20.
  - **TNT lit by a player with pvp off.** The vendored level's primed TNT keeps no igniter, so nothing names the player behind its blast. Section 20.
  - **The friend's first-person hand** (the held item, its swing, eating, blocking, the trident's charge). E12 left it; only the totem's activation shows now. Section 20.
  - **Music on the friend's game** (E22). Section 20.
- **Found while testing the sleeping change:** a gun hit does not wake a sleeping player, the host included: MW2 damage only lowers the health the survival state follows (`Survival::observe`), and the sleeper wakes on the hurt counter, which counts Minecraft hurts. Vanilla wakes on any damage. Recorded in section 20 for later.
- **Tests:** none (documents only).
- **Test steps:** the fixes above in `docs/round8/e-m6-tests.md`.
- **Kit:** none new.

### The M6 leftovers pass clippy

Commit: see `git log` ("The Minecraft multiplayer leftovers pass clippy"). The previous commit is `69343e48` (the leftovers that stay for later).

- **Changed:** clippy (`clippy -p minecraft_net -p minecraft_survival -p render_anim -p net -p sim -p minecraft_terrain --all-targets`, `approx_constant` allowed as for every track) on the lines of this group's commits: `minecraft_host.rs` names the friends' lectern clicks (`LecternClicks`, a complex type) and builds them without `unwrap_or_default` (disallowed in `clippy.toml`); a test in `views.rs` copies `SurvivalStatus` instead of cloning it. No other finding on the group's lines.
- **Tests:** the touched crates on the real 26.3 data: `minecraft_net` 15, `minecraft_survival` 585 + 1, `render_anim --lib` 145, `sim --lib` 117, `net --lib` 15, `minecraft_terrain --lib` 1111 (3 ignored); `check -p launcher` green. No new `.rs` file in this group.
- **Test steps:** none new.
- **Kit:** none.

## Combined kit for every test in this track

This line covers the M6 owner tests in `docs/round8/e-m6-tests.md`. The M1 to M5 tests keep their own kit lines in `docs/round8/e-m1-tests.md` to `docs/round8/e-m5-tests.md`; the M5 combined line is in the git history of this file at `beb54197`, the M3 and M4 line at `6571e5db`, and the M2 line (E11 to E15) at `bf0a3a47`.

`minecraft:bow` x1, `minecraft:arrow` x64, `minecraft:diamond_sword` x1, `minecraft:diamond_helmet` x1, `minecraft:diamond_chestplate` x1, `minecraft:diamond_leggings` x1, `minecraft:diamond_boots` x1, `minecraft:golden_apple` x8, `minecraft:cooked_beef` x32, `minecraft:cobblestone` x64, `minecraft:snowball` x16, `minecraft:ender_pearl` x16, `minecraft:eye_of_ender` x12, `minecraft:water_bucket` x1, `minecraft:powder_snow_bucket` x1, `minecraft:leather_boots` x1, `minecraft:totem_of_undying` x1, `minecraft:spawner` x1, `minecraft:zombie_spawn_egg` x1, `minecraft:lectern` x1, `minecraft:written_book` x1 with three pages, `minecraft:white_bed` x1, `minecraft:splash_potion@harming` x1, `minecraft:splash_potion@poison` x1 (24 stacks).

```
IW4L_MINECRAFT_KIT=minecraft:bow,minecraft:arrow*64,minecraft:diamond_sword,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*8,minecraft:cooked_beef*32,minecraft:cobblestone*64,minecraft:snowball*16,minecraft:ender_pearl*16,minecraft:eye_of_ender*12,minecraft:water_bucket,minecraft:powder_snow_bucket,minecraft:leather_boots,minecraft:totem_of_undying,minecraft:spawner,minecraft:zombie_spawn_egg,minecraft:lectern,minecraft:written_book[written_book_content={title:"Test",author:"Owner",pages:["Page one text.","Page two text.","Page three text."]}],minecraft:white_bed,minecraft:splash_potion@harming,minecraft:splash_potion@poison
```
