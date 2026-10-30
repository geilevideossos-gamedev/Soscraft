# Soscraft

Soscraft is Minecraft without its player model. You play a Minecraft 26.3 world as a Modern Warfare 2 soldier, with MW2's guns, player models and movement, together with friends in one persistent world.

It is built on [chasmlol's 2010 Rust Rewrite Mashup](https://github.com/chasmlol/2010-rust-rewrite-mashup), which runs on [IW4L](https://github.com/vladtrc/iw4L), a Rust rewrite of MW2.

The plan:

- **Minecraft survival** with MW2 guns, player models and movement.
- **Elden Ring bosses** to fight in the Minecraft world.
- **ARK: The Island creatures** and taming.
- **Skate 3 skateboarding** anywhere in the world.
- **One Minecraft-style settings menu** for all of it.
- **Multiplayer for 4 to 5 players** on Windows.

## What works today

- **Minecraft survival** in a real Minecraft 26.3 world: vanilla world generation, blocks, crafting, block screens, mobs, villagers, health and hunger, the Nether and the End with the dragon fight. Everything is saved in the vanilla 26.3 world format. Details and known limits: [docs/MINECRAFT-SURVIVAL.md](docs/MINECRAFT-SURVIVAL.md).
- **MW2** guns, movement and the operator's body in Minecraft worlds. Bullets break blocks and hurt mobs, and Minecraft mobs fight back.
- **Multiplayer:** the host runs the world, and friends join through the relay with a join code. A room holds up to 4 players today. Friends cannot skate, trade with villagers or hit other players with Minecraft weapons yet. Details: [docs/MULTIPLAYER.md](docs/MULTIPLAYER.md).
- **Skate 3 skating** for the host, with full block collision. Press **J**. See [docs/SKATE.md](docs/SKATE.md).
- **Elden Ring:** the first boss, Margit, is being built and is not playable in a release yet. The plan is [docs/ELDEN-RING-BOSSES.md](docs/ELDEN-RING-BOSSES.md).
- **ARK** creatures and the shared settings menu are not started.

## What you need

Soscraft runs on Windows.

- **Every player:** the Soscraft exe, Minecraft Java Edition and the original Call of Duty: Modern Warfare 2 (2009, PC, with multiplayer; the Steam version works).
- **Only the host:** Elden Ring, ARK (for The Island) and Skate 3 (the Xbox 360 version), for the bosses, creatures and skating.

No game files are in this repository. The game reads MW2 from your own install, and on its first start it downloads Minecraft 26.3's files (about 125 MB) from Mojang's own servers into `iw4l-artifacts\minecraft-26.3`.

## Build

You need [Rust](https://rustup.rs/) (stable) and the Visual Studio C++ build tools.

```
cargo build --release --locked -p launcher -p iw4l-master
```

This builds the game, `target\release\iw4l.exe`, and the relay for multiplayer, `target\release\iw4l-master.exe`. Copy both into a game folder you can write to (not Program Files).

## Run

1. Start `iw4l.exe`. The first start asks for your MW2 folder (the one with `iw4mp.exe` and `zone`) and whether you have Skate 3.
2. Go to **Create Game**, pick the **Minecraft** tab and the map **overworld**. Or start `iw4l.exe map minecraft:overworld`.

The world is saved in `iw4l-artifacts\saves`, named by `IW4L_MINECRAFT_WORLD` in the `.env` next to the exe (default `world`).

## Play with friends

1. **The host, once:** make the relay's certificates, set a room password, open UDP port 4433, and make the start files and the friend's zip with `scripts\mp-local.ps1`.
2. **Each friend, once:** unpack the host's zip, start the exe once so it finds MW2 and downloads the Minecraft files, and set a player name.
3. **To play:** the host starts `Host Minecraft World.bat` and sends the join code from the chat. Each friend starts `Join Minecraft World.bat` and pastes the code.

Every step, and what to do when something fails: [docs/round8/friend-install.md](docs/round8/friend-install.md). Host and friends must run the same build.

## More

- [docs/MINECRAFT-SURVIVAL.md](docs/MINECRAFT-SURVIVAL.md): what is built, its limits and how it is tested.
- [docs/MULTIPLAYER.md](docs/MULTIPLAYER.md): how multiplayer works and what is left.
- [docs/PROGRESS.md](docs/PROGRESS.md): the current round's state.
- [docs/INDEX.md](docs/INDEX.md): upstream's docs on the MW2 engine.

## Licence

Soscraft is licensed under the GNU General Public License v3, in [LICENSE](LICENSE).

The parts that come from upstream (IW4L and the 2010 Rust Rewrite Mashup) are licensed under the Apache License 2.0, in [LICENSE-APACHE](LICENSE-APACHE), with their notices in [NOTICE](NOTICE). Code under `third_party/` keeps the terms it came with.

## Credits

- [2010 Rust Rewrite Mashup](https://github.com/chasmlol/2010-rust-rewrite-mashup) by chasmlol: the fork Soscraft starts from, with Skate 3 skating and the Minecraft world.
- [IW4L](https://github.com/vladtrc/iw4L) by vladtrc and contributors: the MW2 rewrite everything runs on.
- [MinecraftOSS](third_party/minecraftoss/README.md): the Rust Minecraft engine behind the world, mobs and items.

Minecraft is a trademark of Mojang Studios. Call of Duty: Modern Warfare 2, Elden Ring, ARK and Skate 3 belong to their owners. You need your own copies. Soscraft is not affiliated with any of them.
