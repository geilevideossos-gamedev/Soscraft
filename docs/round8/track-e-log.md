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

## Combined kit for every test in this track

`minecraft:obsidian` x10, `minecraft:flint_and_steel` x1, `minecraft:chest` x1, `minecraft:cobblestone` x32, `minecraft:lava_bucket` x1, `minecraft:torch` x16, `minecraft:cooked_beef` x8, `minecraft:crafting_table` x1, `minecraft:shield` x1, `minecraft:skeleton_spawn_egg` x1, `minecraft:ender_pearl` x4, `minecraft:totem_of_undying` x1, `minecraft:white_bed` x1, `minecraft:elytra` x1, `minecraft:firework_rocket` x8, `minecraft:stone_pickaxe` x1, `minecraft:ender_eye` x4, `minecraft:experience_bottle` x8, `minecraft:wolf_spawn_egg` x1, `minecraft:bone` x8, `minecraft:dirt` x4, `minecraft:wooden_hoe` x1, `minecraft:wheat_seeds` x4 (23 stacks).

```
IW4L_MINECRAFT_KIT=obsidian*10,flint_and_steel,chest,cobblestone*32,lava_bucket,torch*16,cooked_beef*8,crafting_table,shield,skeleton_spawn_egg,ender_pearl*4,totem_of_undying,white_bed,elytra,firework_rocket*8,stone_pickaxe,ender_eye*4,experience_bottle*8,wolf_spawn_egg,bone*8,dirt*4,wooden_hoe,wheat_seeds*4
```
