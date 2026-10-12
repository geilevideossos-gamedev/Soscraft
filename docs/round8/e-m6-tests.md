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

## Friends freeze in powder snow like the host

A friend in powder snow now freezes on their own screen as the host does: the white frost outline grows over about 7 seconds, the hearts turn to the frozen sprites when fully frozen, and walking slows by up to half while frozen with ground under the feet. With leather boots a friend walks on top of powder snow without sinking, and the host sees them do it.

1. Start both windows: `& scripts\mp-local.ps1 -World mp-m6 -Fresh`. Spawn in both. Host window: `/time set day`.
2. **Sinking and freezing.** Friend window: dig a hole one block wide and one deep and pour the powder snow bucket into it (look at the hole and right click). Step in and stay there: the friend sinks to the bottom of the snow. Over about 7 seconds the white frost outline covers the screen; then the hearts turn light blue (frozen), and one heart is lost every 2 seconds with the hurt sound. Screenshot the friend window with frozen hearts. **S8**
3. **Slowed while frozen.** Friend window: jump out and walk away on flat ground at once. For a few seconds the friend walks clearly slower, and the outline fades; the walk speed comes back as it fades. The friend's view does not jump or stutter while slowed.
4. **The host sees it.** Host window: do steps 2 and 3 yourself in the same hole: same outline, hearts and slowdown as before this change.
5. **Leather boots.** Friend window: put on the leather boots (E, drag them to the feet slot) and walk onto the powder snow: the friend stands and walks on top of it without sinking. Hold Shift: the friend sinks in. Host window: watch the friend's soldier walk on the snow and sink on Shift. Screenshot the friend standing on the snow in the host window. **S9**

```
IW4L_MINECRAFT_KIT=minecraft:powder_snow_bucket,minecraft:leather_boots,minecraft:cobblestone*64
```

## Friends see snowflakes around every player in powder snow

White snowflakes rise from powder snow around anyone who moves inside it, in both windows: around the friend's own body and around the other player.

1. Use the powder snow from the frost steps above (or fill a fresh hole two blocks wide and one deep with the bucket). Host window: stand inside the powder snow and walk back and forth with A and D.
2. Friend window: watch the host's soldier. Small white snowflakes rise out of the snow around the host's legs while the host moves, and stop when the host stands still. Screenshot the friend window. **S10**
3. Friend window: step into the snow and walk back and forth yourself (F5 for third person helps). Snowflakes rise around your own body in the friend window too. Host window: the host sees them around the friend as well. **S11**

```
IW4L_MINECRAFT_KIT=minecraft:powder_snow_bucket,minecraft:cobblestone*64
```

## A friend sees their own totem rise in first person

When a totem of undying saves the friend, the friend's own screen now shows the vanilla animation: the totem flies up in front of the view, spins and drops away over 2 seconds. The other window still shows the totem's green and yellow particles around the saved player.

1. Both windows hold the totem: Friend window, select the totem in the hotbar (or put it in the offhand with F).
2. Friend window: `/tp @s ~ ~40 ~` and fall (survival mode, no water below). The fall would kill; the totem pops instead: the friend's screen shows the totem rising, spinning and falling back, with the totem sound, and the friend keeps a little health. Screenshot the friend window during the spin. **S12**
3. Host window, at the same time: the friend's soldier is wrapped in green and yellow particles; no totem shows on the host's screen.
4. Host window: do the same fall with the host's totem: the host sees its own animation as before, and the friend window shows only the particles around the host. The friend does not see the host's totem on their own screen. **S13**

```
IW4L_MINECRAFT_KIT=minecraft:totem_of_undying
```

## Friends see the mob spinning in spawner cages

The friend's game now draws the small mob turning inside a spawner, with the flames and smoke around the cage while a player is near, as the host's game does.

1. Host window: `/op Friend`, `/time set night` (zombies stay out of the sun). Place the spawner on open ground, then `/gamemode creative`, right click the spawner with the zombie spawn egg and `/gamemode survival`. A small zombie turns inside the cage in the host window.
2. Friend window: walk to within a few blocks of the spawner. The same small zombie turns in the cage, with flames and smoke rising from it. Screenshot. **S14**
3. Friend window: walk more than 16 blocks away and look back: the zombie stops turning and the flames stop. Walk back: it turns again.
4. Wait near it until it spawns zombies (up to 40 seconds): each time it spawns, the zombie in the cage spins fast for a moment in both windows.
5. Friend window: `/gamemode creative`, break the spawner: the cage and its zombie are gone in both windows. Then `/gamemode survival`.
6. A spawner placed by the friend: Friend window: place the spawner from the kit a few blocks from the first one, `/gamemode creative`, right click it with the zombie spawn egg from the kit, `/gamemode survival`. The zombie shows in both windows.

```
IW4L_MINECRAFT_KIT=minecraft:spawner,minecraft:zombie_spawn_egg
```

## Friends read books on lecterns

A friend can now open the book on a lectern, turn its pages and take it, like the host. Both readers see the same page: a page turned in one window turns in the other.

1. Friend window: place the lectern and right click it with the written book: the book lies on the lectern in both windows.
2. Friend window: right click the lectern: the book screen opens with "Page one text." and the page count. Click the arrow to the right: "Page two text.", with the page turn sound. Screenshot. **S15**
3. Host window: right click the same lectern: the host's screen opens on page 2 as well. Host: turn to page 3. Friend window: the friend's screen shows page 3 too.
4. Friend window: press Escape: the screen closes. Host window: close yours too.
5. **Out of reach.** Friend window: open the book again, then have the host push you away: Host window: `/tp Friend ~10 ~ ~`. The friend's screen closes by itself.
6. **Adventure mode.** Host window: `/op Friend`, then Friend window: `/gamemode adventure`, open the book: there is no Take Book button. `/gamemode survival` and open it again: the button is there.
7. **Take Book.** Friend window: click Take Book: the screen closes, the book is in the friend's hotbar, and the lectern is empty in both windows. If the host had the book open at that moment, the host's screen closes too.

```
IW4L_MINECRAFT_KIT=minecraft:lectern,minecraft:written_book[written_book_content={title:"Test",author:"Owner",pages:["Page one text.","Page two text.","Page three text."]}]
```

## Friends lie down in beds and count for the night

A friend can now sleep: their view goes to the pillow, the screen darkens, the chat bar shows the Leave Bed button, and everyone sees them lying on the bed. They count for the night like the host: with both asleep the night passes.

1. Host window: `/op Friend`, `/time set 13000`. Both place their bed on flat ground, side by side.
2. **Lying down.** Friend window: right click the bed. The friend's view moves to the pillow looking down the bed, the screen slowly darkens, and the chat bar opens with a "Leave Bed" button. Both windows show "1/2 players sleeping" above the hotbar. Host window: the friend's soldier lies on its back on the bed, head on the pillow. Screenshot both windows. **S16**
3. **Leave Bed.** Friend window: click Leave Bed. The friend stands up beside the bed, the dark fades out, and the soldier stands in the host window too.
4. **The night passes.** Friend window: lie down again. Host window: lie down too. After about 5 seconds of both in bed the night skips to morning: both wake up beside their beds and the sky is bright in both windows. Screenshot the friend window just after. **S17**
5. **Only one asleep.** Host window: `/time set 13000`. Friend window: lie down and stay; the host stays up. The night does not pass (the default `players_sleeping_percentage` is 100). Friend window: press Escape: the friend gets up as with the button.
6. **Hit in bed.** Friend window: lie down. Host window: shoot the friend once (pvp is on): the friend wakes and stands up at once.

```
IW4L_MINECRAFT_KIT=minecraft:white_bed
```

## With pvp off, a player's harming potion hurts no player

With the `pvp` game rule off, a splash potion of harming thrown by a player no longer hurts the other player (nor the thrower). Other potion effects still land, as in vanilla.

1. Host window: `/gamerule pvp false`. Stand next to each other.
2. Friend window: throw the splash potion of harming at the host's feet (select it, right click at the ground). Both windows show the purple burst. Neither player loses health. Screenshot the host window with full hearts. **S18**
3. Friend window: throw the splash potion of poison the same way: both players get poisoned (green hearts) and lose health down to half a heart at most, as in vanilla.
4. Host window: `/gamerule pvp true`. Repeat step 2 with the host's own harming potion thrown at the friend: now the friend loses 3 hearts (and the host too if it stood in the splash).

```
IW4L_MINECRAFT_KIT=minecraft:splash_potion@harming,minecraft:splash_potion@poison
```
