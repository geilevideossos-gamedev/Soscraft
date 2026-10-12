# Multiplayer M6 tests: the End and finishing

This collects the owner's two-window steps for milestone M6 of track E (`docs/MULTIPLAYER.md` section 19, E22-E24), one section per change, as each lands. The windows, the builds and the script are the same as in `docs/round8/e-m1-tests.md` Part 1 (build the relay once, build the game with `--profile play`, copy it as `iw4l-mp-m6.exe`, then run `mp-local.ps1`).

The friend window is called **Friend**. The host window uses your own player name, written `<you>` below.

## E22. The dragon fight and the credits work for every player

Every player near the dragon now gets its boss bar, not only the host. The dragon's one-shot effects (the purple bursts of its fireballs, the breath from its mouth while it perches, the explosions when blocks break around it and when it dies, and a shattered eye of ender) now reach a friend on the reliable stream, so none of them is lost when a state frame is dropped. Each player who walks into the exit portal for the first time sees the credits on their own screen while the other window keeps playing, and the credits end for that player alone. A friend who quits during the credits is let go.

The friend's game plays no music yet (neither the boss music nor the credits music); the host's game does, as before.

Put the kit below in the `.env` next to the exe before the first start: both players get it on their first join.

1. Start both windows: `& scripts\mp-local.ps1 -World mp-m6 -Fresh`. Spawn in both. Host window: `/op Friend`, `/time set day`.
2. **An eye of ender breaks for both.** Stand next to each other. Friend window: throw eyes of ender one by one (right click) until one breaks in the air instead of dropping (about one in five does). Alt-Tab to the host window as it breaks: the host sees the eye's crumbs and the purple ring of particles where it broke. Then throw from the host window and watch the friend window: the friend sees the break too. Screenshot one break in the other window. **S1**
3. **Into the End.** Host window: `/execute in minecraft:the_end run tp @a 0 70 0`. Both land on the bedrock pillar in the middle of the island; jump down. Both windows show the pink "Ender Dragon" bar at the top of the screen within a second or two, and the End's fog thickens as in single player. Screenshot both windows. **S2**
4. **The bar only near the fight.** Friend window: `/gamemode spectator`, then `/tp @s 300 100 0`. Within a second the bar is gone in the friend window and stays in the host window. Friend window: `/tp @s 0 80 0`, then `/gamemode survival`: the bar comes back. Screenshot the friend window without the bar. **S3**
5. **Crystals and breath.** Both windows show the end crystals on top of the obsidian pillars, with their beams to the dragon when it flies close. Shoot a crystal (MW2 gun, arrow or snowball) from one window: both windows show the explosion. Wait near the portal in the middle until the dragon lands on it and breathes: the friend window shows the purple breath coming out of its mouth and the purple cloud on the ground, and when it spits a fireball at a player the purple burst where it lands. Screenshot the breath in the friend window. **S4**
6. **The bar follows the dragon's health.** Shoot the dragon from both windows, alternating. Both bars go down together; in the friend window the bar slides to each new value like in the host window.
7. **The kill.** Kill the dragon. Both windows show its death: the beams of light, the explosions around it while it rises, and the bar goes away in both. The exit portal opens in the middle. Screenshot the friend window during the death. **S5**
8. **The friend's credits.** Friend window: walk into the exit portal. The friend window shows the end poem with the name "Friend" in it and the credits. Alt-Tab to the host window: the host keeps playing, mobs and particles keep moving, and the friend's soldier is gone from the End. Screenshot both. **S6** Friend window: press Escape (or wait to the end). The credits close and the friend stands at their spawn in the overworld, can move, mine and take damage again.
9. **The host's credits.** Host window: walk into the exit portal: the credits roll on the host. Alt-Tab to the friend window: it keeps playing while the host watches (walk, break a block, it breaks). Host window: press Escape: the host stands at their spawn.
10. **Only once.** Host window: `/execute in minecraft:the_end run tp @a 0 70 0`. Each walks into the exit portal again: both go straight home, with no credits.
11. **Quitting during the credits.** Close the friend window (Alt+F4). Start a second friend: `& scripts\mp-local.ps1 -World mp-m6 -FriendOnly -Friend Bob`. Host window: `/op Bob`, then `/execute in minecraft:the_end run tp Bob 0 70 0`. Bob window: jump down and walk into the exit portal; the credits start. Close the Bob window (Alt+F4) during the credits. Host window: the chat says "Bob left the game" and the host keeps playing. Start Bob again with the same command: Bob comes back and can play at once (no endless loading screen, no credits again). Screenshot Bob's window after the rejoin. **S7**
12. Close the windows (type `quit` in the script's console).

```
IW4L_MINECRAFT_KIT=minecraft:bow,minecraft:arrow*64,minecraft:diamond_sword,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*8,minecraft:cooked_beef*32,minecraft:cobblestone*64,minecraft:snowball*16,minecraft:ender_pearl*16,minecraft:eye_of_ender*12,minecraft:water_bucket
```
