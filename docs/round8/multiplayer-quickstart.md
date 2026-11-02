# Multiplayer quickstart: two windows on one PC, by hand

This starts the relay, a host window and a friend window yourself, one PowerShell window each, from the game folder `C:\Games\Mashup\2010-Rust-Rewrite-Mashup`. `scripts\mp-local.ps1` does the same in one go (see `docs/round8/track-e-tests.md`); this page is the manual way.

How it works:
- **The relay** (`iw4l-master.exe`) runs on your PC at `127.0.0.1:4433`. Both games talk to it over an encrypted connection, so it needs a certificate. The games trust that certificate through `iw4l-ca.pem`.
- **The player name** comes only from `player_name=` in a window's `settings.cfg`. There is no command-line switch for it. Each window therefore gets its own copy of your settings, chosen with `IW4L_SETTINGS_PATH`. The copies also make both windows small and windowed instead of fullscreen. Names must differ, because the host refuses a second player with the same name (capitals don't count). Use plain letters, digits and `_`, at most 15 characters.
- **Host or friend** is decided by the environment variables of each PowerShell window. They win over the `.env`, and they last only as long as that PowerShell window stays open.
  - `IW4L_MASTER_HOST_NAME` makes a window the host.
  - `IW4L_MASTER_JOIN` with the room's join code makes it a friend.
  - A window with both refuses to use the relay.
- **The host's world** is `IW4L_MINECRAFT_WORLD=mp-test`, so your own world `r7final` from the `.env` is never opened. It is saved in `iw4l-artifacts\saves\mp-test`.
- **The kit** comes from the `.env` as usual. Both players get it on their first join to a fresh world.

## Once: certificates and the two settings files

In any PowerShell window:

```powershell
& C:\Dev\mashup-crafting\scripts\mp-local.ps1 -CertsOnly
$game = "C:\Games\Mashup\2010-Rust-Rewrite-Mashup"
$dir = "$game\iw4l-artifacts\mp-manual"
New-Item -ItemType Directory -Force $dir | Out-Null
foreach ($role in @(@("host", "Sos"), @("friend", "Friend"))) {
    $lines = (Get-Content "$game\iw4l-artifacts\settings.cfg") -replace '^player_name=.*', "player_name=$($role[1])" -replace '^fullscreen=.*', 'fullscreen=false' -replace '^resolution=.*', 'resolution=960x540'
    [System.IO.File]::WriteAllLines("$dir\$($role[0]).cfg", [string[]]$lines)
}
Select-String -Path "$dir\*.cfg" -Pattern '^(player_name|fullscreen|resolution)='
```

- The first line makes `iw4l-artifacts\mp-local\certs\`: the relay's `server-cert.pem` and `server-key.pem`, and `iw4l-ca.pem` for the games. It keeps certificates that already exist.
- The loop copies your `settings.cfg` twice, keeping your keys:
  - `mp-manual\host.cfg` with `player_name=Sos`
  - `mp-manual\friend.cfg` with `player_name=Friend`

  Both are windowed at 960x540. Change the names in the `foreach` line if you like.
- The last line shows the three changed lines of each file.

Run it again only if you want fresh copies of your settings, for example after changing your keys.

## Window 1: the relay

```powershell
$game = "C:\Games\Mashup\2010-Rust-Rewrite-Mashup"
$certs = "$game\iw4l-artifacts\mp-local\certs"
& "$game\iw4l-master.exe" serve --bind 127.0.0.1:4433 --cert "$certs\server-cert.pem" --key "$certs\server-key.pem"
```

- It prints `iw4l-master listening on 127.0.0.1:4433` and keeps running. Leave this window open.
- Ctrl+C stops it, once both games are closed.
- `127.0.0.1` means only games on this PC can reach it. For a friend over the internet, see `friend-install.md`.

## Window 2: the host

```powershell
$game = "C:\Games\Mashup\2010-Rust-Rewrite-Mashup"
Remove-Item Env:IW4L_MASTER_JOIN -ErrorAction SilentlyContinue
$env:IW4L_MASTER_ADDR = "127.0.0.1:4433"
$env:IW4L_MASTER_SERVER_NAME = "iw4l-prod"
$env:IW4L_MASTER_CA_CERT = "$game\iw4l-artifacts\mp-local\certs\iw4l-ca.pem"
$env:IW4L_MASTER_HOST_NAME = "Local world"
$env:IW4L_MASTER_MAX_PLAYERS = "5"
$env:IW4L_MASTER_STATUS_FILE = "$game\iw4l-artifacts\mp-manual\host.status"
$env:IW4L_SETTINGS_PATH = "$game\iw4l-artifacts\mp-manual\host.cfg"
$env:IW4L_MINECRAFT_WORLD = "mp-test"
& "$game\iw4l-survival.exe" map minecraft:overworld
```

- The game opens in a window, loads the world `mp-test` and plays as `Sos`.
- Once the world shows, the chat says `Friends join with the code [<32 letters and digits>]`. That is the join code.
- The same code is also in the `room=` line of `iw4l-artifacts\mp-manual\host.status`.
- For a fresh test world, delete `iw4l-artifacts\saves\mp-test` before starting the host.

## Window 3: the friend

Start it after the host's chat shows the join code.

```powershell
$game = "C:\Games\Mashup\2010-Rust-Rewrite-Mashup"
Remove-Item Env:IW4L_MASTER_HOST_NAME -ErrorAction SilentlyContinue
$env:IW4L_MASTER_ADDR = "127.0.0.1:4433"
$env:IW4L_MASTER_SERVER_NAME = "iw4l-prod"
$env:IW4L_MASTER_CA_CERT = "$game\iw4l-artifacts\mp-local\certs\iw4l-ca.pem"
$env:IW4L_MASTER_STATUS_FILE = "$game\iw4l-artifacts\mp-manual\friend.status"
$env:IW4L_SETTINGS_PATH = "$game\iw4l-artifacts\mp-manual\friend.cfg"
$env:IW4L_MASTER_JOIN = Read-Host "Join code from the host's chat"
& "$game\iw4l-survival.exe" map minecraft:overworld
```

- Type the code from the host's chat at the prompt.
  - Spaces, dashes and the square brackets are fine, because the game strips them, and capitals are fine too.
  - Not in the mood to type it? Paste this line instead of the `Read-Host` line:

    ```powershell
    $env:IW4L_MASTER_JOIN = ((Get-Content "$game\iw4l-artifacts\mp-manual\host.status") -match '^room=') -replace '^room=', ''
    ```

- The friend window joins the host's room and loads into the host's world as `Friend`. The host's chat says `Friend joined the game`.
- **If the code is wrong**, the friend window opens a world of its own instead of the host's. Check the code and start window 3 again.
- **If the name is taken**, the friend window says `A player with that name is already in the world`. Give `friend.cfg` another `player_name`.

## Playing and stopping

- Only the window in front gets your keys and mouse. Alt-Tab between them.
- Quit each game through its menu. Quit the friend first, then the host, because the host saves the world. Then press Ctrl+C in the relay window.
- **Logs:** each start writes its own log in `iw4l-artifacts\logs\` (named by its start time). `latest.log` points at the window started last, which is the friend.
- **Settings:** each window keeps its own changes, such as keys or volume, in its own `.cfg`. Your normal `settings.cfg` stays as it was.
