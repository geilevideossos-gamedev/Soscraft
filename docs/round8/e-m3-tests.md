# Multiplayer M3 tests: friends see and hear the world

This collects the owner's two-window steps for milestone M3 of track E (`docs/MULTIPLAYER.md` section 19, E16-E17), one section per change, as each lands. The windows, the builds and the script are the same as in `docs/round8/e-m1-tests.md` Part 1 (build the relay once, build the game with `--profile play`, copy it as `iw4l-mp-m3.exe`, then run `mp-local.ps1`).

The friend window is called **Friend**. The host window uses your own player name, written `<you>` below.

## Merge of tracks B, C and D

Tracks B (piglins, hoglins, striders and riding, sleeping), C (shulkers and levitation, item frames, lecterns, trapped chests, powder snow, totems, the elytra on the MW2 body) and D (skating) are merged into track E. The host runs their rules for every player, so a friend can now ride a strider, wear and glide with an elytra, float on levitation, freeze in powder snow, use item frames and lecterns, open trapped chests and anger piglins.

Until E16 the friend's own window shows no mobs, item frames or dropped items, and until E17 it plays none of their sounds; check those from the host window. Friends cannot sleep yet, the lectern's book screen opens for the host only, and a friend's frost slowdown and frozen hearts are not shown on the friend's side yet (see the track log).

Put the kit below in the `.env` next to the exe before the first start: both players get it on their first join.

1. Start both windows: `& scripts\mp-local.ps1 -World mp-m3 -Fresh`. Spawn in both. Host window: `/time set day`, `/op Friend`, `/difficulty normal`. Stand next to each other on flat ground.
2. **Elytra.** Friend window: put the elytra on (open the inventory with E and shift-click it into the chest slot), then `/tp Friend ~ ~40 ~`, jump while falling to glide and use a rocket.
   1. Host window: the friend's MW2 body has the elytra on its back, spreads the wings and lies along its flight while gliding, and folds them on landing. Screenshot. **S1**
   2. Host window: put on your own elytra and glide the same way. Friend window: the host's body shows the wings and the glide pose. Screenshot. **S2**
3. **Striders.** Host window: use the strider spawn egg on the ground two blocks in front of the friend.
   1. Friend window: the strider is not drawn yet. Hold the saddle, aim at where the strider stands and right click: the saddle leaves the friend's hotbar.
   2. Friend window: right click the same spot with an empty hand. The friend's body is lifted into the seat.
   3. Host window: the friend sits on the saddled strider. Screenshot. **S3**
   4. Friend window: hold the warped fungus on a stick and look around: the strider walks the way the friend looks. Right click with it: the strider speeds up, and the stick loses durability.
   5. Friend window: crouch: the friend gets off next to the strider. Host window: the strider stands there without a rider.
4. **Levitation.** Host window: use the shulker spawn egg on the ground about 6 blocks from the friend, then step back out of its sight. Friend window: wait for a shulker bullet to hit the friend (the friend may need to walk around the spot). The friend floats upwards for about 10 seconds, then falls slowly back. Screenshot the friend in the air from the host window. **S4** Kill the shulker with a gun.
5. **Powder snow.** Friend window: empty the powder snow bucket on the ground and stand in the snow. The friend sinks into it. After about 7 seconds the friend takes a little damage every 2 seconds. Host window: snowflakes rise around the friend while they walk through it. Friend window: step out.
6. **Item frames.** Friend window: put up both item frames on a cobblestone wall (right click the wall with them). Hold the iron sword and right click the first frame: the sword leaves the hotbar. Right click again twice. Punch the frame: the sword pops out and the friend can pick it up.
   1. Host window: the frames hang on the wall; the sword turned with each right click and fell out when the friend punched it. Screenshot before the punch. **S5**
7. **Lectern.** Friend window: place the lectern and right click it with the book and quill. The book leaves the hotbar and the lectern now shows a book (the block changes in both windows). Friend window: right click it again: no book screen opens for the friend yet. Host window: right click it: the host's book screen opens and turns pages; press Take Book: the host gets the book.
8. **Trapped chest.** Friend window: place the trapped chest with the redstone lamp right next to it. Open the chest (right click): the lamp lights in both windows while the friend has it open and goes out when they close it. Host window: open it too while the friend has it open: the lamp stays lit until both have closed it.
9. **Piglins.** Host window: run `/summon minecraft:piglin ~4 ~ ~ {IsImmuneToZombification:1b}`.
   1. Friend window: put on the golden helmet. Host window: put on the golden helmet too. The piglin leaves both alone.
   2. Friend window: place the gold block and break it with the stone pickaxe next to the piglin. Host window: the piglin turns on the friend and attacks them, not the host. Screenshot. **S6** Kill it with a gun.
10. **Totem.** Friend window: hold the totem in the offhand (F swaps hands), then `/kill Friend` from the host window. Host window: the totem's particles burst around the friend's body, and the friend lives.
11. **Sleeping.** Host window: `/gamerule playersSleepingPercentage 50`, then `/time set night`, then place the bed.
    1. Friend window: right click the bed. The chat says "Respawn point set", and the friend does not lie down (friends cannot sleep yet).
    2. Host window: right click the bed. The host lies on the bed and sees "Sleeping through this night" above the hotbar; the night passes after a few seconds. Friend window: the sky turns to day. Screenshot both. **S7**
12. Close both windows (type `quit` in the script's console).

```
IW4L_MINECRAFT_KIT=minecraft:elytra,minecraft:firework_rocket*8,minecraft:saddle,minecraft:warped_fungus_on_a_stick,minecraft:strider_spawn_egg,minecraft:shulker_spawn_egg,minecraft:powder_snow_bucket,minecraft:item_frame*2,minecraft:iron_sword,minecraft:cobblestone*32,minecraft:lectern,minecraft:writable_book,minecraft:trapped_chest,minecraft:redstone_lamp,minecraft:gold_block*4,minecraft:stone_pickaxe,minecraft:golden_helmet,minecraft:totem_of_undying,minecraft:white_bed
```
