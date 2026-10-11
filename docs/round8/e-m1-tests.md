# Multiplayer M1 test: a friend joins, sees the world and chats

This tests milestone M1 of track E (`docs/MULTIPLAYER.md` section 19, E1-E8) with two game windows on one PC. Part 2 covers what a friend on their own PC installs.

The friend window is called **Friend**. The host window uses your own player name from `iw4l-artifacts\settings.cfg`, written `<you>` below.

## What M1 can and cannot do yet

- **The friend can:**
  - load the host's world and walk on it;
  - break blocks by shooting them, and see the host's blocks change;
  - see the host's time of day and weather;
  - chat and run commands;
  - leave and come back to the same place.
- **The friend cannot yet:**
  - see a Minecraft HUD (hearts, hotbar, hand) or open an inventory;
  - see mobs or dropped items.

  These come in M2.
- **Distance:** chunks load only around the host. Stay within about 8 chunks (128 blocks) of the host.
- **Commands on the friend:** `/give`, `/gamemode`, `/kill` and `/effect` change the friend's player on the host, but nothing shows in the friend window until M2. `/tp` and chat work now.

## Part 1: two windows on this PC

### Build once

Run these in PowerShell in `C:\Dev\mashup-crafting`, or in `C:\Dev\mc-wt\track-e` before track E is merged. Close any game first.

1. **The relay.** Skip this if you already rebuilt it after E3.
   1. Close any running `iw4l-master.exe`.
   2. Run:

      ```
      & scripts\cargo-slot.ps1 build --release --locked -p iw4l-master
      ```

   3. Copy `D:\iw4l-target\release\iw4l-master.exe` to `C:\Games\Mashup\2010-Rust-Rewrite-Mashup\`.
2. **The game.**
   - From the repo:

     ```
     & scripts\cargo-slot.ps1 build --profile play --locked -p launcher
     ```

     The game is then `D:\iw4l-target\play\iw4l.exe`.
   - From the track's worktree:

     ```
     & .\scripts\cargo-slot.ps1 +target=D:\iw4l-wt\track-e build --profile play --locked -p launcher
     ```

     The game is then `D:\iw4l-wt\track-e\play\iw4l.exe`.

   Copy it to `C:\Games\Mashup\2010-Rust-Rewrite-Mashup\iw4l-mp-m1.exe`. The script always takes the newest `iw4l-mp-*.exe`. It never uses or overwrites `iw4l-survival.exe`.

### Start both windows

3. Run this in PowerShell in the repo:

   ```
   & scripts\mp-local.ps1 -World mp-m1 -Fresh
   ```

   - `-Fresh` deletes the test world `mp-m1` first, so both players start new. It lives in `iw4l-artifacts\mp-local\host\`, and your main world is never touched.
   - The script says it waits for both cargo slots. It then:
     1. starts the relay;
     2. opens the **host window** (muted) and prints `room <code>`;
     3. opens the **friend window**;
     4. prints `both players are in the room`.
   - Keep this PowerShell window open: it is the script's console.

### What to check

Only the window in front gets your keys, so each step says which window to use. Alt-Tab between them.

4. **Joining.**
   1. Host window: the world loads as usual. Spawn.
   2. Friend window: the loading screen stays until the host's chunks have arrived (a few seconds), then class select shows.
   3. Friend window: the chat shows "Friend joined the game" in yellow. Chat lines fade after 10 seconds, so if the loading took longer, press **T** to see it, then Escape.
   4. Host window: its chat shows the same line.

   Screenshot both windows.
5. **The world.**
   1. Friend window: spawn. The friend stands near the host's spawn, on the same terrain the host sees (same trees, same blocks) and under the same sky.
   2. Friend window: walk, jump and drop off a block. The friend never falls through the ground or walks through blocks.
   3. Alt-Tab to the host window: the friend's soldier is seen moving.
   4. Back in the friend window: the host's soldier is seen standing.

   Screenshot both windows from about the same spot.
6. **Blocks.**
   1. Host window: shoot a block until it breaks.
   2. Friend window: it is gone.
   3. Friend window: shoot another block until it breaks.
   4. Host window: it is gone too.
7. **Chat both ways.**
   1. Friend window: press **T**, type `hello`, press Enter. The friend's chat shows `<Friend> hello`.
   2. Host window: the chat shows `<Friend> hello`.
   3. Host window: press T, type `hi back`, press Enter.
   4. Friend window: `<you> hi back` shows.

   Screenshot both chats.
8. **Permissions.**
   1. Friend window: press **/**, type `time set night`, press Enter. A red "Unknown or incomplete command. See below for error" shows, because a new friend is level 0. Nothing changes in either window.
   2. Host window: run `/op Friend`. The host sees "Made Friend a server operator".
   3. Friend window: the gray, italic line "[<you>: Made Friend a server operator]" shows.
   4. Friend window: run `/time set night`. The friend sees a white "Set ..." line.
   5. Host window: the gray, italic line "[Friend: Set ...]" shows. Within a second both skies are night.
   6. Friend window: run `/time set day`. Both skies are day again.

   Screenshot both chats.
9. **Whispers.**
   1. Host window: run `/tell Friend psst`. The host sees "You whisper to Friend: psst" in gray italics.
   2. Friend window: "<you> whispers to you: psst" shows.
   3. Friend window: run `/msg <you> got it`. The friend sees "You whisper to <you>: got it".
   4. Host window: "Friend whispers to you: got it" shows.
10. **Commands that move players.**
    1. Friend window: walk about 10 blocks away from the host. Run `/tp Friend <you>`. The friend's soldier jumps to the host.
    2. Host window: walk 10 blocks away, then run `/tp Friend ~ ~ ~5`. Alt-Tab to the friend window: the friend now stands 5 blocks from the host's new spot.
11. **Answers go back to whoever asked.**
    1. Friend window: run `/summon minecraft:cow ~2 ~ ~`. The friend sees "Summoned new Cow".
    2. Host window: the host sees the gray, italic line "[Friend: Summoned new Cow]" and the cow itself. The friend cannot see mobs until M2.
12. **Leaving and coming back.**
    1. Friend window: walk to a spot you will recognise, then close the window (Escape, then quit).
    2. Host window: "Friend left the game" shows in yellow.
    3. Open a **second** PowerShell window in the repo and run:

       ```
       & scripts\mp-local.ps1 -FriendOnly
       ```

    4. Friend window: it loads into the same world, at the spot where the friend left, and "Friend joined the game" shows.
    5. Friend window: run `/time set noon`. It still works, because the op is kept in the world's `ops.json`.

    Screenshot the friend window.
13. **Closing.** The friend window now belongs to the second PowerShell window from step 12, so each script closes its own window.
    1. In the second PowerShell window, type `quit`. The friend window closes and that script ends.
    2. In the first PowerShell window (the script's console), type `quit`. The host window closes and the relay stops.
    3. Each script prints a `peak working sets: ...` line: the first for the host, the first friend window and the relay, the second for the friend window of step 12. Send both lines.

**Kit:** none needed. Any `IW4L_MINECRAFT_KIT` in your `.env` is given to both players on their first join to `mp-m1`. The friend's items do not show until M2.

**Console help:** `h <command>` in the script's console runs an MW2 console command in the host window, and `f <command>` in the friend window, without switching windows. Minecraft chat and commands are typed in the window itself.

## Part 2: a friend on their own PC

M1 is first checked over the internet with plain MW2 (`-Map iw4:mp_rust`), because that does not depend on any Minecraft code. When it works, the same steps join the Minecraft world.

### You (the host), once

1. **Build the relay and copy it next to the game** (Part 1, step 1).
2. **Make the certificates:**

   ```
   & scripts\mp-local.ps1 -CertsOnly
   ```

   Hand out `iw4l-ca.pem` only. Never hand out the `*-key.pem` files.
3. **Open the port.**
   1. Allow `iw4l-master.exe` through the Windows firewall for UDP, on private and public networks.
   2. Forward **UDP 4433** on your router to this PC.
4. **Set the room password.** Add `IW4L_MASTER_PASSWORD=<room password>` to the game folder's `.env`.
5. **Write the start files and the friend's zip:**

   ```
   & scripts\mp-local.ps1 -Bats
   ```

   ```
   & scripts\mp-local.ps1 -FriendKit -PublicAddress <your public IP>
   ```

   - The first writes `Host Minecraft World.bat` and `Join Minecraft World.bat` into the game folder.
   - The second writes `iw4l-artifacts\mp-local\friend-kit-iw4l-mp-m1.zip`.
   - For the plain MW2 check, add `-Map iw4:mp_rust` to both commands. Run them again without it for the Minecraft world.
6. **Host.**
   1. Run `Host Minecraft World.bat`. It starts the relay if none is running.
   2. The join code is the `room=` value in `iw4l-artifacts\master.status`.
   3. Send your friend the zip, the room password and the join code.

### The friend, once

1. **Have their own copy of MW2.**
2. **Unpack the zip** into a folder of their own. It holds:
   - the same `iw4l-mp-m1.exe` you have;
   - the licence files and the README;
   - `iw4l-ca.pem`;
   - `Join Minecraft World.bat`;
   - a `.env`;
   - `How to join.txt`;
   - an empty `iw4l-artifacts\` folder.

   It holds none of Mojang's or MW2's files, none of your saves or settings, and no keys.
3. **Check the `.env`.** It should read:

   ```
   IW4L_GAMES=
   IW4L_SKATE=off
   IW4L_MASTER_ADDR=<your public IP>:4433
   IW4L_MASTER_SERVER_NAME=iw4l-prod
   IW4L_MASTER_CA_CERT=iw4l-ca.pem
   IW4L_MASTER_PASSWORD=<room password>
   ```

   The address and password are filled in when you made the zip with `-PublicAddress` and had the password in your `.env`. Otherwise the friend types them in.
4. **Start the game once to the menu** and wait.
   - The first start finds or asks for their MW2 folder and converts its files.
   - It also downloads the Minecraft 26.3 files from Mojang. That needs internet access and a few minutes.
   - It is done when `iw4l-artifacts\minecraft-26.3\.complete` exists, and the log says "Minecraft files ready".
5. **Set their own player name** in the game's settings. It decides their player file in your world, so it must differ from yours and stay the same between visits.

### Joining

The friend has three ways in:
- run `Join Minecraft World.bat` and type the join code (it first checks that the Minecraft files are there);
- open the server browser and pick your room;
- type `ui_join_lobby_id <code>` in the console.

Then steps 4-12 of Part 1 apply, with the friend's PC as the friend window. The friend rejoins the same way. Screenshot both PCs.
