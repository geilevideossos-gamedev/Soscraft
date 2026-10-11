# Multiplayer M2 tests: the friend plays survival

This collects the owner's two-window steps for milestone M2 of track E (`docs/MULTIPLAYER.md` section 19, E9-E15), one section per change, as each lands. The windows, the builds and the script are the same as in `docs/round8/e-m1-tests.md` Part 1 (build the relay once, build the game with `--profile play`, copy it as `iw4l-mp-m2.exe`, then run `mp-local.ps1`).

The friend window is called **Friend**. The host window uses your own player name, written `<you>` below.

## E9. The chunk map loads chunks around every player

Before E9, chunks loaded only around the host, and a friend more than about 8 chunks away walked into missing terrain. Now the host's world loads and keeps chunks around every player, while the host still draws and lights only its own surroundings.

1. Start both windows: `& scripts\mp-local.ps1 -World mp-m2 -Fresh`. Spawn in both.
2. Host window: run `/op Friend`, then `/gamemode creative`.
3. Host window: run `/tp <you> ~600 ~ ~`. Fly down (or up out of the ground) until you stand on the surface. The terrain appears around you as usual.
4. Host window: run `/tp Friend <you>`. The friend now stands next to you, 600 blocks from spawn.
5. Host window: run `/tp <you> ~-600 ~ ~`, back towards spawn. Land on the surface again.
6. Friend window: the friend is now about 600 blocks from the host, far outside the host's view.
   1. The ground under the friend stays. The friend does not fall.
   2. Walk about 100 blocks in one direction, then back. New terrain keeps appearing ahead of the friend, and the friend never walks into a gap or falls through the ground.
   3. Shoot a block next to the friend until it breaks. Remember the spot.

   Screenshot the friend window.
7. Host window: the host's own view is unchanged. Its frame rate is about what it was with the friend standing next to it.
8. Host window: run `/tp <you> Friend`. The host lands next to the friend. The terrain there shows within a second or two, with no wait for it to generate, and the block the friend broke is gone.

   Screenshot the host window from about where the friend stands.
9. Friend window: walk back towards spawn with the host for about 50 blocks. Both keep seeing the same terrain.
10. Close both windows (type `quit` in the script's console).

**Kit:** none needed.
