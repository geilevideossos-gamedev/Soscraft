# Multiplayer M4 tests: fighting

This collects the owner's two-window steps for milestone M4 of track E (`docs/MULTIPLAYER.md` section 19, E18). The windows, the builds and the script are the same as in `docs/round8/e-m1-tests.md` Part 1 (build the relay once, build the game with `--profile play`, copy it as `iw4l-mp-m4.exe`, then run `mp-local.ps1`).

The friend window is called **Friend**. The host window uses your own player name, written `<you>` below. The MW2 guns are the ones of your class, as in every Minecraft world.

## E18. Guns hurt other players unless pvp is off, and deaths are told to everyone

A Minecraft world always runs free-for-all, so the `pvp` game rule is the only thing that decides whether players can hurt each other. Gun kills between players give one Minecraft death line in both chats ("Friend was slain by <you> using [<gun>]"), next to the MW2 kill feed. Minecraft deaths (mobs, falls, lava) show the Minecraft killed-by card in the window of the player who died, the friend's too.

Put the kit below in the `.env` next to the exe before the first start: both players get it on their first join.

1. Start both windows: `& scripts\mp-local.ps1 -World mp-m4 -Fresh`. Spawn in both. Host window: `/time set day`, `/op Friend`, `/difficulty normal`. Stand about 10 blocks apart on flat ground.
2. **A gun kill.** Host window: shoot the friend with your MW2 gun until they die.
   1. Both windows: the MW2 kill feed shows the kill, and the chat shows "Friend was slain by <you> using [<gun>]" exactly once, with the gun's MW2 name. Screenshot both chats. **S1**
   2. Host window: the friend's items lie where they died. Friend window: after respawning, walk back and pick them up.
3. **A kill the other way.** Friend window: shoot the host until the host dies. Both windows: "<you> was slain by Friend using [<gun>]" once. A knife kill (melee) gives the same line.
4. **pvp off.** Host window: `/gamerule pvp false`.
   1. Host window: shoot the friend with a full magazine and throw a grenade at their feet. The friend takes no damage: their hearts stay full and they do not die. Screenshot the friend's hearts in the friend window afterwards. **S2**
   2. Friend window: shoot the host. The host takes no damage either.
   3. Host window: throw a grenade at your own feet: it still hurts you (only other players are spared).
   4. Host window: throw the trident at the friend: it does not hurt them. Throw a wind charge straight at the friend: the hit does not hurt them (its gust still pushes them, as in vanilla).
   5. Host window: `/gamerule pvp true`, then shoot the friend once: they take damage again.
5. **The world stays open.** Host window: kill the friend 30 times in a row with `pvp` on (respawn the friend each time). The world never ends the round: no intermission, no scoreboard at the end, and both keep playing.
6. **A Minecraft death for the friend.** Friend window: use the creeper spawn egg on the ground right next to the friend and stand still until it explodes.
   1. Both windows: the chat shows "Friend was blown up by Creeper" once.
   2. Friend window: the death card names the Creeper (the Minecraft killed-by card, as the host gets for its own Minecraft deaths). Screenshot. **S3**
7. **Death messages off.** Host window: `/gamerule show_death_messages false`, then shoot the friend until they die. The MW2 kill feed still shows it, and neither chat has a death line. `/gamerule show_death_messages true` afterwards.
8. **Optional: a team mode is ignored.** Close both windows, add `IW4L_GAMETYPE=war` to the `.env`, and start again without `-Fresh`. The world loads as before, there are no teams (no team colours over names) and shooting the friend hurts them. Remove the line afterwards.
9. Close both windows (type `quit` in the script's console).

```
IW4L_MINECRAFT_KIT=minecraft:creeper_spawn_egg*2,minecraft:trident,minecraft:wind_charge*8,minecraft:cooked_beef*8,minecraft:cobblestone*32
```
