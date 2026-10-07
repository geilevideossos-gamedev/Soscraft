# CLAUDE.md

Fork of chasmlol/2010-rust-rewrite-mashup: a Rust/Bevy rewrite of MW2 (IW4) with a Minecraft 26.3 world. Detailed state, limitations and testing: [docs/MINECRAFT-SURVIVAL.md](docs/MINECRAFT-SURVIVAL.md).

## Goal

Play online with 2-4 friends in one persistent Minecraft world, with MW2 weapons, movement and character models. Later, fight Elden Ring bosses. Skate 3 movement comes after that.

## Roadmap

1. Multiplayer and saving
2. Finish survival
3. One Elden Ring boss as a proof of concept
4. Skate 3

## Rules

- **Branch and remotes**
  - Work on `minecraft-survival`.
  - Push only to `origin` (geilevideossos-gamedev/mashup-crafting). Never push to `upstream` (chasmlol).
  - One commit per fix or feature. Subjects are plain English with no prefix, e.g. "Minecraft campfires cook food like vanilla".
- **Code style**
  - No code comments of any kind: no `//`, `/* */`, `///` or `//!`.
  - Never run `rustfmt` or `cargo fmt` on existing files. Edits change only the lines they need, in the surrounding style.
  - New files must pass `rustfmt --check` on their staged content.
- **Mojang files:** never commit any. Vanilla assets and data are read at runtime from the downloaded 26.3 packs and jar.
- **Cargo**
  - Always pass `-j 8`, and run one cargo command at a time. More jobs or parallel builds have crashed the PC.
  - Set `CARGO_TARGET_DIR=D:\iw4l-target`.
  - Use PowerShell, not Git Bash, because Git Bash's `link.exe` shadows MSVC.
- **Release build:** `cargo build --release --locked -j 8 -p launcher`.
  - Output: `D:\iw4l-target\release\iw4l.exe`.
  - Copy it to `C:\Games\Mashup\2010-Rust-Rewrite-Mashup\iw4l-survival.exe`.
- **Test kit:** `IW4L_MINECRAFT_KIT=` in the `.env` next to the exe.
  - At most 27 stacks, because it only fills the 27 backpack slots.
  - Always give the kit line in its own code block so it isn't cut when copied.
- **Leave alone:** the untracked `blocks.txt` and `props.txt`, and the screenshot folders (`screenshots-*/`, `ac0c791/`).
- **Tests:** the fork's test policy is the owner's. Don't trim tests because of upstream's `approved_tests` rule.
- **How the owner tests:** they play the release build and send numbered screenshots. Every change therefore ends with exact in-game test steps and the kit line they need.

## Reference repos (read-only, not integrated)

Cloned into `C:\Dev\references\`, outside this repo:

- **[chasmlol/skycraft](https://github.com/chasmlol/skycraft):** Skyrim x Minecraft.
  - A Fabric mod and an SKSE plugin talk over shared memory.
  - Multiplayer comes from vanilla Open to LAN plus the e4mc tunnel, and saving from vanilla autosave.
  - Licence: MIT.
- **[Funny-Bones/ELDEN-RING-Combat-Rewrite](https://github.com/Funny-Bones/ELDEN-RING-Combat-Rewrite):** Elden Ring player combat in Rust/Bevy 0.19.
  - Loads animations, TAE timings and params from the user's own game install.
  - It has player combat and a sparring dummy, no boss AI.
  - **No licence file:** read it, don't copy code.

## Where upstream docs are wrong for this fork

AGENT.md, CONTEXT.md, README.md, CONTRIBUTING.md and docs/ come from upstream and stay unedited so upstream merges stay clean. For this fork:

- **Workflow**
  - There is no `make`, no `make mr`, no `context/` folder and no `externals/`. References live in `C:\Dev\references\`.
  - The branch is `minecraft-survival`, not `master`. Even upstream's default branch is `main`.
  - Ignore the "rustfmt the touched files", "squash onto master" and `make publish-check` landing and public-push recipes in CONTEXT.md and AGENT.md.
  - We keep small descriptive commits and push only to origin.
  - CONTRIBUTING.md's `cargo fmt --all` would reformat upstream files, so don't run it.
- **Tests and comments:** "Tests live only in `crates/approved_tests`" (CONTEXT.md) does not apply. The fork's comment rule is stricter than CONTEXT.md's: no comments at all.
- **README.md**
  - The README was already edited by the fork up to ac0c791. It has no feature notes from round 4 onward, and none of the fork goal.
  - Its "Minecraft World.bat" starts upstream's `iw4l.exe`, not `iw4l-survival.exe`.
- **Multiplayer is MW2 only:** the MW2 "multiplayer", `docs/DUO.md` and `docs/MASTER.md` cover MW2 matches only. The Minecraft world is neither shared nor saved yet (see the state doc).
- **Build docs:** `docs/BUILD.md` is Linux-first. This fork builds natively on Windows with MSVC.
- **Vendored code:** `third_party/minecraftoss` is MinecraftOSS `4013a68`, and the fork has patched about 24 of its files. If upstream ever re-vendors a newer MinecraftOSS, diff our patches first.
