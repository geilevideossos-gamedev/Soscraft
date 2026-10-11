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

Commit: see `git log` ("A friend's game shows the host's world")

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
