# Track B log: striders, hoglins, piglins and sleeping

Branch `track-b`, worktree `C:\Dev\mc-wt\track-b`, target dir `D:\iw4l-wt\track-b`. Briefs, in the order they run: `b3-striders.md`, `b2-hoglins.md`, `b1-piglins.md`, `b4-sleeping.md`.

A commit can't name its own hash, so each entry names the commit before it. Its own hash is in `git log` under its subject.

Hit keys for MW2 bullets: track B uses 21 to 25 (20 is the dragon's `DRAGON_KEY`), track C uses 26 to 28.

## Group b3-striders-1

### 1. Minecraft players use items on mobs

After 0bcbfb17.

- **Changed:**
  - `render_anim/src/minecraft_world.rs`: the use chain tries the mob under the crosshair first, for each hand in `use_hands` order. A mob counts when it is within the entity reach (3 in survival, 5 in creative), nearer than the targeted block, and no fireball, crystal or dragon part is in front of it. A consumed click swings the hand.
  - `minecraft_survival/src/entities.rs`: `Entities::use_on_mob` picks the mob (`mob_to_use`), asks `would_interact`, and only then sends `ServerHandle::mob_action(hit, None, …)`. The answer's changed slots (`MobResult.slots`) are written back into the inventory when it arrives (`apply_mob_result`), and broken shears go to `broken_items` (the break sound and burst).
  - `minecraft_terrain/src/mob_actions.rs`: new `would_interact(world, hit, inventory, slot, infinite, player)`. `Actor` carries the acting player's id, which `villager_interact` and `wolf_interact` now get instead of a literal 0. `MobOutcome.broken` names a tool the action broke.
  - `minecraft_terrain/src/server.rs`: `Command::MobAction`, `ServerHandle::mob_action` and `ServerSim::mob_action` take the player id; `merchant_view` reads it. `MobResult.broken`.
  - This makes every existing mob interaction live in game: feeding and breeding (cows, sheep, pigs, chickens, wolves, mooshrooms), saddling pigs, milking cows, mushroom stew from mooshrooms, shearing sheep and mooshrooms, taming wolves with bones, wolf collars and sitting orders, and repairing iron golems with iron ingots.
- **Tests:** `mob_actions`: `would_interact_agrees_with_interact` (a table of 25 cases over cows, calves, pigs, saddled pigs, piglets, sheep, chickens, mooshrooms, wolves, golems and villagers, in survival and creative), `a_saddle_on_a_pig_is_used_up_with_its_sound`. `entities`: `a_mob_behind_a_block_is_not_used`, `creative_reaches_mobs_at_five_blocks_and_survival_at_three`, `a_saddle_used_on_a_pig_leaves_the_inventory_and_saddles_it` (through a real server thread; skips without the game data).
- **Decisions:**
  - `would_interact` runs `interact` on a clone of the host's copy of the mob world and of the inventory, instead of mirroring each arm by hand. It can't drift from `interact`, and the arms the later briefs add (striders, hoglins, piglins) are covered without touching it. The one exception is shears on a sheep: the host has no shearing loot table, so it answers true for any live sheep (vanilla's `Sheep.mobInteract` consumes the click whether or not the sheep is ready).
  - Villagers stay out: `would_interact` is false for them, so a right-click on a villager falls through to the item and opens nothing (no trading screen exists yet, and an opened server menu would hold the villager).
  - A click that the mob passes (`would_interact` false) goes on down the old chain, blocks behind the mob included. Vanilla would only try the item, not the block behind the mob; the old chain already did this, so nothing new breaks.
  - The `merchant` operations (`server.rs`, `MerchantOp`) still use player 0: nothing on the host opens a trading screen, so they are left for track E.
- **Test steps (survival):**
  1. Use a pig spawn egg, right-click the pig with a saddle: the saddle appears on the pig with its sound, and leaves your hand (**1**). Right-click it again with the second saddle: nothing happens, the saddle stays in your hand.
  2. Right-click the pig with carrots: hearts, one carrot used. Use the second pig egg, feed that pig too: a piglet appears (**2**).
  3. Use both cow eggs, feed them wheat: hearts, a calf (**3**). Right-click an adult cow with the empty bucket: you get a milk bucket and hear the milking sound. Right-click the calf with a bucket: nothing.
  4. Use a sheep egg and right-click the sheep with shears: wool drops, the shears lose durability (**4**).
  5. Use a wolf egg and right-click the wolf with bones until it tames (hearts, a red collar, it sits). Right-click it with an empty hand: it stands up; again: it sits (**5**).
  6. Stand a sheep behind a glass pane and right-click the pane with wheat: the sheep is not fed.
  7. `/gamemode creative`: feed a cow from 4.5 blocks away; nothing is used up. In survival the same distance does nothing.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:pig_spawn_egg*2,minecraft:saddle,minecraft:saddle,minecraft:carrot*16,minecraft:cow_spawn_egg*2,minecraft:wheat*16,minecraft:bucket,minecraft:sheep_spawn_egg,minecraft:shears,minecraft:wolf_spawn_egg,minecraft:bone*16,minecraft:glass_pane*4
```
