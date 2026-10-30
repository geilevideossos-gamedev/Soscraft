# Playing in the owner's Minecraft world from another PC

This is what a friend installs and runs to join the owner's Minecraft world over the internet, and what the owner (the host) prepares first. Every step was checked against the code at `3cc89dfb` (track E, round 8): the launcher's first start (`crates/launcher/src/first_run.rs`), the `.env` loading (`crates/asset_transport/src/discover.rs`), the relay settings (`crates/net/src/transport/master.rs`), the Minecraft download (`crates/assets/src/minecraft_setup.rs`) and the files `scripts\mp-local.ps1` writes. None of it has been tried between two real PCs yet; the owner's internet test is part 8 of `docs/round8/track-e-tests.md`.

How it works in one paragraph: the host's game runs the whole world. The host's PC also runs the relay, `iw4l-master.exe`, which both games connect to over QUIC (UDP port 4433). The friend's game holds no world and saves nothing: it shows what the host sends, and the friend's inventory, XP and position are saved in the host's world as `players/data/<uuid>.dat`, named after the friend's player name.

## Part A: the host, once

Do this on the owner's PC, in PowerShell in the repo (`C:\Dev\Soscraft\mashup-crafting`). The game folder is `C:\Games\Mashup\2010-Rust-Rewrite-Mashup`.

1. **The game build.** The friend must run exactly the same exe as the host. With release 2 that is `iw4l-survival.exe`. Every command below passes it with `-Exe iw4l-survival.exe`; without `-Exe` the script takes the newest `iw4l-mp-*.exe` in the game folder and stops if there is none.
2. **The relay.** Build it once and copy it next to the game:

   ```
   & scripts\cargo-slot.ps1 build --release --locked -p iw4l-master
   ```

   Copy `D:\iw4l-target\release\iw4l-master.exe` to the game folder. It needs rebuilding only after a change to `crates/iw4l-master` or `crates/master_protocol`.
3. **The certificates.** The relay proves who it is with a certificate. Make them once (this needs Git for Windows, whose OpenSSL the script uses):

   ```
   & scripts\mp-local.ps1 -CertsOnly
   ```

   They go to `iw4l-artifacts\mp-local\certs` in the game folder. Only `iw4l-ca.pem` is ever handed out. Never hand out `ca-key.pem` or `server-key.pem`. If you make new certificates later, every friend needs the new `iw4l-ca.pem` (a new zip).
4. **The room password.** Add this line to the `.env` in the game folder:

   ```
   IW4L_MASTER_PASSWORD=<room password>
   ```

   The `.env` must not contain `IW4L_MASTER_HOST_NAME` or `IW4L_MASTER_JOIN`: the start files set them, and a value in `.env` would reach both kinds of start.
5. **Open the port.**
   1. Windows firewall: in an administrator PowerShell run

      ```
      New-NetFirewallRule -DisplayName "iw4l-master UDP 4433" -Direction Inbound -Protocol UDP -LocalPort 4433 -Action Allow -Profile Private,Public
      ```

      If Windows asks about `iw4l-master.exe` the first time it starts, allow it on private and public networks.
   2. Router: forward **UDP** port 4433 to this PC's IPv4 address (`ipconfig` shows it).
   3. Your public address: the router's status page, or any "what is my IP" site. If the router's own internet address differs from that public address, your provider uses carrier-grade NAT and a forwarded port cannot be reached. Then run the relay on a server instead (`docs/MASTER.md`, with this round's build of `iw4l-master`) and give friends that server's address.
6. **The start files and the friend's zip.**

   ```
   & scripts\mp-local.ps1 -Bats -Exe iw4l-survival.exe
   ```

   ```
   & scripts\mp-local.ps1 -FriendKit -Exe iw4l-survival.exe -PublicAddress <your public IP>
   ```

   - The first writes `Host Minecraft World.bat` and `Join Minecraft World.bat` into the game folder.
   - The second writes `iw4l-artifacts\mp-local\friend-kit-iw4l-survival.zip`. Make it after step 4, so the password is in it; otherwise the friend types it in.
   - Make a new zip, and send it again, after every new build: a friend on an older build is refused (see the troubleshooting table).

### Making the zip by hand

If the script cannot be used, put these files in one folder and zip it. They are all the fork's own or open-licence files:

| File | From |
|---|---|
| `iw4l-survival.exe` | the game folder (the same build the host runs) |
| `LICENSE`, `LICENSE-APACHE`, `NOTICE`, `COPYING-FreeFont.txt`, `OFL-Oxanium.txt`, `README.txt` | the game folder |
| `iw4l-ca.pem` | `iw4l-artifacts\mp-local\certs\iw4l-ca.pem` |
| `Join Minecraft World.bat` | the game folder, after step 6 |
| `.env` | written by hand, with the six lines of part B step 3 |
| `iw4l-artifacts\` | an empty folder |

**Never put these in it:** `iw4l-artifacts\minecraft-26.3` (Mojang's files: each player downloads their own from Mojang), `iw4l-artifacts\cache` (made from the player's own MW2), `skate-data` (made from Skate 3), `iw4l-artifacts\saves`, `settings.cfg`, `profile`, `logs`, `command_history.txt`, your own `.env` (it holds your MW2 path and kit) and any `*-key.pem`.

## Part B: the friend, once

### What you need

- A Windows PC that runs this game.
- Your own Call of Duty: Modern Warfare 2 (2009, PC, with multiplayer; the Steam version works).
- Internet access on the first start, for about 125 MB of Minecraft 26.3 files from Mojang.
- From the host: the zip, the room password (if it is not already in the zip's `.env`) and, when you join, the join code.

### Steps

1. **Unpack the zip** into a folder you can write to, not `Program Files`, for example `C:\Games\Mashup-friend`. It holds:
   - the game exe (`iw4l-survival.exe`, or `iw4l-mp-<name>.exe` for a test build);
   - `Join Minecraft World.bat`;
   - `.env` and `iw4l-ca.pem`;
   - `How to join.txt`, `README.txt` and the licence files;
   - an empty `iw4l-artifacts` folder.

   If Windows SmartScreen stops the exe on its first start ("Windows protected your PC"), choose **More info**, then **Run anyway**: the build is not signed.
2. **Leave the folder as it is.** The game makes everything else itself: `iw4l-artifacts\cache` from your MW2, `iw4l-artifacts\minecraft-26.3` from Mojang, `iw4l-artifacts\settings.cfg` and the logs.
3. **Check the `.env`.** Open it in Notepad. It should read:

   ```
   IW4L_GAMES=
   IW4L_SKATE=off
   IW4L_MASTER_ADDR=<host's public IP>:4433
   IW4L_MASTER_SERVER_NAME=iw4l-prod
   IW4L_MASTER_CA_CERT=iw4l-ca.pem
   IW4L_MASTER_PASSWORD=<room password>
   ```

   - `IW4L_GAMES=` stays empty: the first start fills in your MW2 folder.
   - `IW4L_SKATE=off`: the zip has no Skate 3 converter, and without this line the first start stops to ask for Skate 3. Skating does not work for friends anyway.
   - `IW4L_MASTER_ADDR` is the host's public IP with `:4433` (a host name such as a dynamic DNS name works too). If it says `HOST_IP:4433`, the host made the zip without `-PublicAddress`; type the address in.
   - `IW4L_MASTER_SERVER_NAME` must stay `iw4l-prod`: it is the name in the host's certificate.
   - `IW4L_MASTER_CA_CERT=iw4l-ca.pem` is read from the game's folder.
   - `IW4L_MASTER_PASSWORD` is the host's room password, exactly.
4. **Start the game once and wait in the menu.** Double-click the exe.
   1. It looks for MW2 in your Steam libraries and asks "Found Modern Warfare 2 here ... Use this copy?". Otherwise it asks you to select the MW2 folder: the one with `iw4mp.exe` and the `zone` folder. The folder is written into `.env` as `IW4L_GAMES`.
   2. The menu opens. In the background the game downloads Minecraft 26.3 from Mojang's own servers (`piston-meta.mojang.com` and `resources.download.minecraft.net`), checked against the pinned version. Leave the game in the menu for a few minutes.
   3. It is done when `iw4l-artifacts\minecraft-26.3\.complete` exists, and `iw4l-artifacts\logs\latest.log` has the line "Minecraft files ready". Then quit.
5. **Set your player name.** In the game's Options, on the page with **Player Name**, or with the game closed, in `iw4l-artifacts\settings.cfg` as `player_name=<name>`.
   - The name decides your player in the host's world: the same name always brings back the same inventory, XP and place. Keep it the same between visits.
   - It must differ from the host's and every other player's name, ignoring capitals ("Steve" and "steve" are the same player). The default name is `Player`, so change it.
   - Use up to 16 letters, digits or underscores, as Minecraft does.

## Part C: joining

1. **The host starts the world.** The host double-clicks `Host Minecraft World.bat` in the game folder.
   - It starts the relay minimised (`iw4l-master.exe serve --bind 0.0.0.0:4433`) when none is running, then the game hosting a room for up to 5 players.
   - It opens the world named by `IW4L_MINECRAFT_WORLD` in the host's `.env` (default `world`). To try things out in a separate world first, change that line.
   - As soon as the world shows, the host's Minecraft chat reads "Friends join with the code [<code>]", the 32-character code in green. Press T if the line has faded. The same code is the `room=` line of `iw4l-artifacts\master.status`.
   - The host sends the code to the friend. A new room (the host's game started again) has a new code.
2. **The friend joins.** Double-click `Join Minecraft World.bat`.
   1. It first checks that the Minecraft files are downloaded; if not, it says so and stops (do part B step 4).
   2. It asks "Join code from the host's chat:". Type or paste the code. Brackets, spaces, dashes and capitals are fine (`net::join_code` drops them).
   3. The game starts with that code, connects to the host's relay and joins the room.
3. **In the game.**
   1. The loading screen stays until the host's chunks around you have arrived (a few seconds on a good line).
   2. Class select shows: pick a class and spawn. You stand where you left last time, or, on your first visit, near the world spawn with the host's kit if the host has one in `.env`.
   3. Everyone's chat shows "<name> joined the game" in yellow.
4. **Leaving:** quit the game as usual. The host's chat shows "<name> left the game" and the host saves your player at once. Rejoin the same way, with the code of the host's current room.

**Other ways to join, and why the `.bat` is the one to use** (checked in `crates/console/src/frontend.rs`, not in a game):
- The **server browser** lists the room, but refuses to join a room that is already in a match ("Match already in progress"), and a hosted Minecraft world is always in its match.
- The console command `ui_join_lobby_id <code>` joins with the map picked in the Create Game menu and with no password, so it fails on a room with a password.

## Troubleshooting

The friend's log is `iw4l-artifacts\logs\latest.log` in the friend's folder; the host's is the same path in the game folder.

| What happens | Why | What to do |
|---|---|---|
| `Join Minecraft World.bat` says the Minecraft 26.3 files are not downloaded yet | The first start has not finished the download | Start the exe, wait in the menu until `iw4l-artifacts\minecraft-26.3\.complete` exists, quit, run the `.bat` again |
| The log says "Minecraft files could not be fetched" | No internet, or Mojang's servers were not reachable | Start the game again in the menu; it carries on. If it keeps failing, delete `iw4l-artifacts\minecraft-26.3` and start again |
| The first start keeps asking for the MW2 folder ("doesn't look like Modern Warfare 2 with its multiplayer maps") | The folder is not MW2 with its multiplayer files | Select the folder with `iw4mp.exe` and `zone` |
| The first start asks about Skate 3, or stops with "...iw4l-skate-convert.exe is missing" | `IW4L_SKATE=off` is missing from `.env`, or skating was switched on in the options | Put `IW4L_SKATE=off` back in `.env`; set `skating=false` in `iw4l-artifacts\settings.cfg` |
| The `.bat` opens a world of your own (no host, nobody joins) | The game could not use the relay settings: a code that is still wrong after the clean-up (not 32 characters of 0-9 and a-f), or `IW4L_MASTER_ADDR` / `IW4L_MASTER_SERVER_NAME` missing from `.env`. The log says `master launch disabled:` with the reason | Fix the `.env` or retype the code, then run the `.bat` again |
| Nothing connects; the log has `master relay failed` for another reason | The address or port is wrong, the host's relay is not running, the host's firewall rule or port forward is missing, or `iw4l-ca.pem` is not the host's current one | Check the address in `.env`; the host checks part A steps 3 and 5 and that `Host Minecraft World.bat` started the relay |
| The log's `master relay failed` line names `IncorrectPassword` | `IW4L_MASTER_PASSWORD` differs from the host's | Copy the password exactly; the host checks its own `.env` |
| On the host's PC, the friend can never connect although everything above is right | Another `iw4l-master.exe` was already running, for example one left by `scripts\mp-local.ps1`, which listens on `127.0.0.1` only; the host `.bat` then does not start its own | Close every `iw4l-master.exe` (Task Manager), then run `Host Minecraft World.bat` again |
| "A player with that name is already in the world" | Someone in the world has your name (capitals do not count) | Change your player name (part B step 5) |
| A refusal that asks for the host's build ("Minecraft multiplayer protocol 3") | Your exe and the host's are different builds | Get the host's current zip and replace the exe |
| "Your Minecraft 26.3 files differ from the host's..." | Your downloaded files do not match the host's | Delete `iw4l-artifacts\minecraft-26.3`, start the game to the menu again and wait for the download |
| The loading screen stays and the chat says "The host's Minecraft world does not answer: the host may run a different build" | No answer from the host's Minecraft side within 30 s, although the relay connected | The host runs a different build: get the host's current zip. A host that is still loading its world does answer, and then the loading screen simply waits |
| Dropped with "connection too slow" or "Lost the world stream" | The host's upload could not keep up, or a part of the world stream was lost for 30 s | Join again. The host can close other uploads; by the design's estimate every friend costs the host 50-100 KiB/s of upload while playing and 3-4 MiB when joining |
| The world closes and the game goes back to the menu | The host quit, or the host's game or relay stopped | Wait for the host and join again with the new code |

## What friends cannot do yet

Friends play survival like the host: blocks, chests and other screens, mobs, PvP, beds, all three dimensions and the dragon fight. Still missing for friends (details in `docs/MULTIPLAYER.md` section 20): skating, the held item and its swing in first person, music, trading screens, Minecraft melee against other players, the creative inventory, and the purple portal overlay while standing in a portal.
