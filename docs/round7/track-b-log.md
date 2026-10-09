# Track B log: throwables, lids, held items and armour

Branch `track-b`, worktree `C:\Dev\mc-wt\track-b`. Briefs: `b1-throwables.md`, `b2-shulker-lid.md`, `b3-held-armour.md`.

A commit can't name its own hash, so each entry is written with its commit and its hash is filled in by the next commit; the last entry's hash is in `git log` under its subject.

This is the early part of the track: the crate split is moving `render_anim/src/minecraft_*.rs`, so every feature is built and tested in `minecraft_terrain` (and the vendored MinecraftOSS crates) behind a small API, and the glue that still has to go into the game files is listed under **After the split** at the end.

## Group: throwables

### 1. The Minecraft kit takes potion contents

Commit: 427e772

- **Changed:** `crafting.rs`: a kit entry can name a potion after `@`, like `splash_potion@poison` or `lingering_potion@minecraft:strong_healing`, and gets `minecraft:potion_contents` `{"potion": ...}`. It combines with enchantments (`tipped_arrow@slowness+unbreaking`) and counts (`*2`). New `kit_potion` (splits the entry) and `kit_potion_stack` (`kit_stack` plus the contents); `kit_stack` is unchanged for `/give`.
- **Tests:** `kit_gives_potion_contents`.
- **Decision:** `@` and not `#`, because `#` starts a comment in `.env`. `/give` (`minecraft_commands/src/args.rs`) belongs to track C, so `/give @s splash_potion[potion_contents={potion:"minecraft:poison"}]` is not part of this commit (see After the split).
- **Test steps:**
  1. Start a new world with the kit below and open the inventory: the first three backpack slots hold a splash potion, a lingering potion and 8 tipped arrows. Screenshot. (Tooltips don't name potions yet, so what is inside is checked by the splash steps of the later entries.)
- **Kit:**

```
IW4L_MINECRAFT_KIT=splash_potion@poison,lingering_potion@strong_healing,tipped_arrow@slowness*8
```

### 2. Minecraft thrown items remember their thrower and their vanilla angle, power and gravity

Commit: (next entry)

- **Changed:** `thrown.rs`: `Throwable` gets `pitch_offset`, `power`, `inaccuracy` and `gravity` (the values of each item's `use`: `shootFromRotation(player, xRot, yRot, offset, power, 1)`), and the flight uses them; `shot_direction_offset` adds the offset to the rise only, as `Projectile.shootFromRotation` does. `Thrown` has an `owner` (the thrower's player ID, the same ID the entity world keys players by, `PLAYER_TARGET + id`; its UUID comes from `EntityWorld::set_player_uuid`) and the stack's `components`. New `Throw` (owner, item, components, eye, look, motion) and `Thrown::launch`; the old `Thrown::throw` stays and throws as player 0. New `PlayerBody` (from a `PlayerCandidate`), `player_targets`, `owner_box` and `struck_player`.
- `server.rs`: `tick_thrown` takes every player's body (from `TickInput.mob_players`) instead of the first player's feet. Every living, non-spectating player can be struck, the thrower only after the throw has left it (`Projectile.checkLeftOwner`), so a throw straight up comes back down on you. A pearl vanishes when its own thrower is dead, and its landing carries the thrower: new `Output.pearl_teleports: Vec<(player, position)>` (`ServerSim::take_pearl_teleports`). `Output.pearl_landings` still carries every landing so the game works unchanged until the glue reads the keyed list. Mobs struck by a throw blame its thrower.
- **Tests:** thrown: `snowballs_eggs_and_pearls_leave_level_at_one_and_a_half_under_light_gravity`, `a_pitch_offset_lifts_only_the_rise_of_the_throw`, `a_throw_remembers_its_thrower`, `players_are_struck_by_their_id_and_the_thrower_is_marked`, `a_throw_straight_up_spares_its_thrower_until_it_falls_back_on_it`. server: `each_pearl_teleports_its_own_thrower_and_dies_with_it`; the two pearl tests and the egg test take player bodies.
- **Decision:** a snowball, egg or pearl that strikes a player does nothing to that player, as in vanilla Java (`Player.hurtServer` turns away 0 damage, so there is no knockback either).
- **Test steps:**
  1. Throw snowballs, eggs and an ender pearl at the ground and at a cow: they fly and land as before (the cow is knocked back by the snowball).
  2. Stand still on flat ground, look straight up and throw a snowball: it comes back down and pops on you (it vanishes about 1.5 blocks above the ground, not at your feet).
  3. Look straight up and throw an ender pearl: it lands on you and you take the 5 pearl damage without moving.
- **Kit:**

```
IW4L_MINECRAFT_KIT=snowball*16,egg*16,ender_pearl*16,cow_spawn_egg*2
```

## After the split

The glue still needed in the game files (today `render_anim/src/minecraft_*.rs`, after the split their new crate). Each item names the function, its inputs and the call site.

1. **Throws carry the thrower and the stack.** In `use_hands` (`minecraft_world.rs`, the throwables branch, now at about line 1181) and `Entities::throw` (`minecraft_entities.rs:269`): build a `minecraft_terrain::thrown::Throw { owner: PLAYER (the local player's ID), item: &stack.id, components: stack.components.as_ref(), eye, yaw, pitch, motion, on_ground }` and call `Thrown::launch(&throw, &mut self.random)` instead of `Thrown::throw`. With several players the server makes the throw from the player's own state.
2. **Pearl teleports by player.** In `Entities::poll` (`minecraft_entities.rs:672`): read `output.pearl_teleports` and keep only the entries whose player is the local player (instead of every `output.pearl_landings` entry). `pearl_landings` can then be dropped from `Output`.
3. **`/give` with potion contents** (track C's crate, `minecraft_commands/src/args.rs:248`, and the call in `minecraft_chat.rs:209`): parse `potion_contents={potion:"minecraft:x"}` in the item argument and build the stack with `minecraft_terrain::crafting::kit_potion_stack(recipes, id, 1, &enchantments, potion)` instead of `kit_stack`.
