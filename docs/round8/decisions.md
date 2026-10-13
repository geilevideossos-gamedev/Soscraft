# Round 8: decisions the owner should know about

One line each, most important first in each group. "**Not vanilla**" marks behaviour that is still different from Minecraft 26.3 and open; "Deferred" marks work moved to a later round. Sources: `docs/PROGRESS.md`, `docs/round8/track-*-log.md`, `docs/MULTIPLAYER.md` sections 1, 20 and 23.

## Phase 1: release build time
- Release builds now use ThinLTO with 16 codegen units: about 21 min clean and 13-14 min after an edit to a low crate, instead of 50 with fat LTO. The exe grows from 122 to 150 MB, and the frame time should sit between fat LTO and no cross-crate LTO (upstream measured fat LTO 5.7 % faster at p50). `--profile fat` keeps the old build for an A/B if you see lower frame rates.
- `cargo-slot.ps1` keeps cargo, rustc and the linker off Windows power throttling, which moved them to the E-cores while a game window was in front (a big part of the 49 min build).
- Gentle builds while you play: `%LOCALAPPDATA%\iw4l-cargo-slot.txt` (`slots=1`, `jobs=4`, `priority=low`) runs one cargo at a time with 4 jobs below normal priority. Delete it or set `slots=2`, `jobs=8` for full speed.
- Release 2 also builds the relay (`-p iw4l-master`), which goes next to the exe for multiplayer.

## A. Fixes from your test of e231362 (gateways, eyes of ender, mining, spawners, overload, /tick)
- Mining now follows vanilla: a block mined with the wrong tool or a bare hand drops nothing (stone by hand, iron ore with a wooden pickaxe), and mining wears the tool. MW2 bullets keep their own rules (drops as an enchantless diamond pickaxe).
- The server tick runs on vanilla's schedule: when it falls more than 2 s behind it logs `Can't keep up!` and skips ticks, like vanilla. The tick state (`/tick rate`, `freeze`) is not saved, as in vanilla.
- **Not vanilla:** under `/tick rate` or `/tick freeze` your own timers (hunger, air, effects, fire, item cooldowns, attack strength, the portal timer, mining) stay at 20 a second, because they run with the MW2 player; the sky, fog and MW2 itself never freeze.
- **Not vanilla:** only players and ender pearls go through end gateways (vanilla also takes items, mobs and other projectiles), and a pearl's landing is worked out at once below the exit instead of the pearl falling over several ticks.
- The first trip through a ring gateway generates the chunks on its search line on the main thread, without a loading screen (about 0.5 s in a debug build, less in release).
- Eyes of ender: the stronghold rings are worked out once per world on a background thread; one click throws one eye however long it is held, as in vanilla.
- Deferred: trial spawners (no mob in the cage, no spawning, eggs refused, which is **not vanilla**). They need the trial spawner simulation and go to "Finish survival".
- **Not vanilla:** an egg refused by a spawner because `spawner_blocks_work` is off is only logged, with no chat message.

## B. Nether and overworld leftovers (piglins, hoglins, striders, sleeping)
- Right-clicking a villager does nothing on purpose: there is no trading screen yet, and an open trade menu would hold the villager.
- **Not vanilla (open, in PROGRESS.md):** under `/tick freeze` a strider carrying a player stands still; vanilla keeps an entity with a player on it ticking.
- **Not vanilla:** a player saved asleep stands up beside the bed when they join (vanilla puts them back in bed). The MW2 body lies in its standing idle pose (no sleeping animation).
- **Not vanilla:** only baby striders and zombie kinds steer a strider they ride; a skeleton or ghast rider rides along while the strider walks on its own.
- **Not vanilla:** piglins don't ride hoglins or each other (the RIDE activity was left out), and striders placed by chunk generation come without riders.
- **Not vanilla:** a piglin's crossbow loses no durability, mob weapons carry no enchantment damage, and golden spears fight like swords (spears have no use yet).
- **Not vanilla:** a piglin shuts a door behind it even when another mob is about to walk through.
- **Not vanilla:** a ghast's or blaze's fireball doesn't make a hoglin retaliate; Looting and Fire Aspect don't affect their drops.
- A piglin that turns into a zombified piglin keeps only its main-hand item and armour as live gear; the rest stays in its saved tag.
- Piglin brutes generated in bastions before round 8 keep what they were saved with: no golden axe and no home. Unvisited bastions get the new ones.

## C. End and visual leftovers (shulkers, End cities, item frames, End sky, dragon, elytra body)
- Music: only the End's three tracks (22 MB) are fetched from Mojang at start, so the overworld and the Nether stay silent. There is no music slider; music follows the master volume (**not vanilla**).
- Deferred: the dragon's dissolve while it dies (needs shader and atlas changes in shared files).
- **Not vanilla:** shulkers are not solid to the MW2 player, and an opening lid pushes mobs but not players.
- **Not vanilla:** Looting is not supported anywhere (the weapon doesn't reach the loot rolls), so shulker shells always drop at 50 %.
- **Not vanilla:** skeleton arrows and explosions don't pop shulker bullets; MW2 guns, MW2 blasts and punches do.
- **Not vanilla:** you can't jump while levitating on the ground, and slow falling on a ladder keeps the ladder's own slide speed.
- **Not vanilla:** powder snow catches a fall by its speed (faster than a 2.5-block drop), not by the fall distance; mobs don't freeze in it and a burning mob doesn't melt it.
- **Not vanilla:** a gliding MW2 body is drawn lying along its flight, but MW2 bullets still hit the upright pose, so in PvP shots at a glider can miss. There is no MW2 glide animation, only a pose over the in-air one.
- **Not vanilla:** item frames: a hit costs the usual attack exhaustion, a creative player's snowball still knocks the item out, TNT exploding in water still breaks frames, and there are no maps to frame.
- MW2 bullets knock the flying dragon back like a melee hit from the shooter's position; grenades don't (explosions are exempt, as in vanilla).
- The totem's rise is drawn in the hand pass, so an MW2 gun swinging close can cover part of it.
- Structure chests whose loot uses a function not ported yet (desert pyramids, shipwrecks, ocean ruins and others) still roll nothing; End city and dungeon chests roll.

## D. Skating in Minecraft worlds
- Guns and Minecraft items are put away on the board: no firing, aiming, mining, placing, eating or blocking. Pick the board up (one key) to fight.
- Skating is off until turned on in Options > Controls > Minecraft, saved per install; the key is J (rebindable) and works on MW2 maps too.
- The keyboard skate controls are fixed this round and use physical key positions (on your German keyboard "Z" is the key labelled Y); Tab steps off the board.
- The board is picked up by teleports, deep water, flying, gliding, riding, sleeping and powder snow; levitation and slow falling leave it alone, as for a vanilla rider.
- **Not vanilla / known limit:** every block has the same friction, so ice and soul sand feel like stone.
- The board is not an item and is not saved; a player saved while skating stands where the skater was.
- A bail in the air takes the fall damage of the whole drop; a bail on the ground that then rolls over an edge takes none.
- The ramp and rail fixes also apply on MW2 maps (a ramp foot no longer counts as a lip, no rails across the middle of steep sloped faces).
- Deferred: skating for friends and networking the skate pose and board (a friend presses J and nothing happens).

## E. Multiplayer, first playable
- The host owns the world: every level, player and player file runs and is saved on the host's PC; a friend's game holds no world and saves nothing. No host migration, no reconnect, no world without the host.
- Friends join through our MW2 relay (`iw4l-master`, UDP 4433) with the join code only: the server browser refuses a running world, and a still-wrong code starts a world of the friend's own.
- Host and friends must run the same exe (protocol 3 and grown player state); a different build is refused.
- A Minecraft world always runs MW2 free-for-all, and the `pvp` game rule alone decides whether players hurt each other.
- Minecraft deaths stay in the Minecraft chat and out of the MW2 kill feed, as you asked in round 7; gun kills show in both, as "slain by <player> using [<gun>]". You can overrule this.
- MW2 movement stays authoritative on the host; block changes are not predicted, so a friend's blocks appear after one round trip.
- A new friend joins in the world's default game mode with the host's kit. The host is always the owner (level 4); friends need `/op` for commands.
- Names are compared ignoring capitals, and a second player with a taken name is refused; the default MW2 name is `Player`, so two friends who keep it clash.
- The join code shows once in the host's chat, as a 32-character hex string that can't be clicked to copy.
- Each dimension has its own height band in the one MW2 collision world, so players can be in different dimensions at once.
- Trust is friends only: the host's checks stop mistakes, not cheaters. Set `IW4L_MASTER_PASSWORD`.
- **Not vanilla:** the dragon's death sound reaches only the players in the End (vanilla sends it to every dimension).
- **Not vanilla:** TNT lit by a player hurts players even with `pvp` off (the primed TNT keeps no igniter).
- **Not vanilla:** a gun hit doesn't wake a sleeping player (only Minecraft damage does), the host included.
- **Not vanilla:** a gun hit followed by a fall or lava is told as a plain fall or lava death, not "doomed to fall by <player>".
- **Not vanilla:** the enchanting seed is not saved, so a table offers new enchantments after a restart; every player gets the server's view distance of 8 chunks.
- **Not vanilla:** under `/tick freeze` or `/tick rate` a friend's clocks run at 20 a second, corrected by the host once a second.
- Deferred: Minecraft melee between players (swords and axes reach only mobs), MW2 damage through Minecraft armour, and trading screens for anyone.
- Deferred: the friend's first-person hand (held item, swing, eating, blocking, trident charge), the friend's offhand key, music on the friend's game, the creative inventory for friends, and several friend-side particles, the portal overlay, lightning and campfire smoke.
- Deferred: the "host closed the world" message (the friend's game just goes back to the menu).
- Known: another `iw4l-master.exe` already running (for example `mp-local.ps1`'s) keeps `Host Minecraft World.bat` from starting one friends can reach; close it first.
- Every tracked mob's move is sent every tick (no vanilla per-kind intervals), and more than 2 players is untested.

## F. Fixes from your interim test of release 1
- The black MW2 body and guns were an engine bug older than round 8 (a partly filled texture array read as zeros on DX12). The fix also changes MW2 maps: models get their light grid and reflections back and the MW2 sky box shows instead of black.
- G throws the lethal grenade and Shift+G the tactical; B is freed and F only swaps hands. Shift is the only chord modifier; both grenade keys can be rebound.
- Your old `settings.cfg` is moved to the new layout once (header v6); a file saved later by upstream's `iw4l.exe` would drop the `SHIFT+G` line.
- Spawn eggs: 36 of the 88 eggs hatch their mob; the other 52 are mobs the game doesn't have and stay unused in the hand. None is faked with another mob's model.
- Cave spiders are real cave spiders (small, 12 health, poison for 7 s on normal and 15 on hard), and mineshaft spawners spawn them.
- **Not vanilla:** a jockey rolled by an egg (spider with skeleton, baby zombie with chicken) hatches both mobs side by side, not mounted, since only striders carry riders.
- The ender dragon egg makes the same dragon as `/summon minecraft:ender_dragon`.
- The spyglass works from either hand for you, but only from the main hand for a friend (no offhand key yet).
- **Not vanilla:** in third person the MW2 body doesn't raise the spyglass to its eye; there are no spyglass statistics or advancements.
