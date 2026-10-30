---
name: release-build
description: Build a release of the game for the owner to test, then install it in the game folder: the release command and its timings, the play and fat profiles, backing up the old exe, copying the exe and the relay, and the protocol bump. Use when the owner asks for a release build.
---

# Release build

Release builds happen only when the owner asks for one.

- Command: `& scripts\cargo-slot.ps1 build --release --locked -p launcher` (ThinLTO, 16 codegen units: about 21 min clean, 13-14 min after an edit to a low crate such as `minecraft_net`).
- The old fat-LTO profile is `--profile fat` (output `D:\iw4l-target\fat\iw4l.exe`). Its link is one rustc thread at about 8 GB and took 26-50 min while the PC was in use.
- `cargo-slot.ps1` keeps cargo, rustc and the linker off Windows power throttling, which otherwise moves them to the E-cores while another window (a game) is in front.
- Output: `D:\iw4l-target\release\iw4l.exe`.
- Copy it to `C:\Games\Mashup\2010-Rust-Rewrite-Mashup\iw4l-survival.exe`, after keeping the old one as `iw4l-survival-<old commit>.exe`.
- When multiplayer is to be tested, add `-p iw4l-master` to the same build and copy `D:\iw4l-target\release\iw4l-master.exe` (the relay) next to the exe.
- A mid-round test build may use `--profile play` instead (the same code at opt-level 3 without fat LTO: about 10 min clean, seconds after an edit; output `D:\iw4l-target\play\iw4l.exe`). The last build of a round is always `--release`.
- Before the build, if any track merged since the last release noted a wire change under `## Protocol` in its log, bump `minecraft_net::PROTOCOL` by one in `crates/minecraft_net/src/messages.rs` and the four `assert_eq!(crate::PROTOCOL, ...)` lines in `codec.rs`, in its own commit ("Minecraft multiplayer moves to protocol N"), after `test -p minecraft_net`.
- After copying, check that the copied files' SHA-256 match the build output, then write the owner's test steps and one kit line per part (at most 27 stacks, each in its own code block) into the round's docs folder, and push.
