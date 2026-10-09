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

Commit: 5efe4bf

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

### 3. Minecraft bottles o' enchanting are thrown and burst into experience like vanilla

Commit: 1c21ff7

- **Changed:** `thrown.rs`: `Throwable::ExperienceBottle` (`minecraft:experience_bottle`: thrown 20 degrees up at 0.7, gravity 0.07, sound `entity.experience_bottle.throw`), so right-click (main hand or offhand, both go through `Throwable::of`) throws it with no glue. `Landed` now carries the struck `block` and `face` (`struck_face`: the face of the collision box the hit point lies on, or the opposite of the motion when it started inside, as `VoxelShape.clip`). New `bottle_experience` (`3 + nextInt(5) + nextInt(5)` from the bottle's own random), `break_pitch`, `sound_cell`, `face_normal`, `BOTTLE_COLOR` (0x385DC6) and `BREAK_SOUND`.
- `server.rs`: a landed bottle queues a potion break (`PotionBreak`, the bottle's colour, not instant), plays `entity.splash_potion.break` at its block's centre (volume 1, pitch 0.9 to 1, through the level sounds the client already plays) and awards the experience with `ExperienceOrb.awardWithDirection`: the struck face's normal on a block, the reversed motion on a mob or player. New `ServerSim::take_potion_breaks` returns the witches' and the thrown breaks together (`Output.potion_breaks`).
- Vendored: `minecraftoss-world` `Level::award_experience_toward(pos, direction, amount)` (`award_experience` is now that with no direction): an orb whose throw points against the direction is flipped, and every new orb starts a quarter block out along it; merging still looks at the hit point. `minecraftoss-entities` `PotionHit.block`: the block a potion struck.
- **Tests:** thrown: `each_throwable_item_names_its_kind` (was `only_snowballs_eggs_and_pearls_are_thrown`), `a_bottle_o_enchanting_leaves_20_degrees_up_at_0_7_and_falls_faster`, `bottle_experience_is_3_to_11`, `a_throw_lands_on_the_face_it_met`. server: `a_bottle_on_stone_bursts_into_3_to_11_experience_above_it` (eight bottles: one break each, its sound at the block centre, orbs a quarter block above the top face moving up), `directed_orbs_never_fly_into_the_face`.
- **Decisions:** the break sound is played by the server through the level sounds (the same path as the burning sounds), so it works before the glue. The blue swirl and glass crumbs need the client particle glue (entry 4).
- **Test steps:**
  1. Note the XP bar (screenshot). Select the bottles, look at the ground about 4 blocks ahead and right-click: the bottle arcs lower and slower than a snowball, breaks with the glass sound and green orbs pop up. Walk over them: the XP bar grows by 3 to 11 points per bottle. Screenshot after.
  2. Put the bottles in the offhand (F), select an empty hand and right-click: same.
  3. Stand 2 blocks from a wall and throw a bottle straight at it: the orbs come out of the wall toward you; none end up behind it.
  4. Throw one straight up while standing still: it breaks on you and the orbs drop around you.
  5. In creative (`/gamemode creative`) the stack doesn't go down.
- **Kit:**

```
IW4L_MINECRAFT_KIT=experience_bottle*64
```

### 4. Minecraft potions break with the vanilla glass sound and swirl

Commit: 6355430

- **Changed:** `server.rs`: witch potions now break with `entity.splash_potion.break` too (`AbstractThrownPotion.onHit`'s event 1053/1054: volume 1, pitch 0.9 to 1 at the block centre), played through the level sounds the client already plays; `ServerSim` keeps its own `sound_random` for those pitches so the mobs' randoms are untouched. Every break, witch or thrown, reaches `Output.potion_breaks`.
- New `spell_particles.rs` (client side, `LevelEventHandler.potionSplashParticles` with `SpellParticle`): `SpellParticles::burst(position, colour, instant)` swirls 100 `effect` (or `instant_effect` for instant potions) particles from the bottom centre of the break's block, each shaded 0.75 to 1 of the colour, with the vanilla speed (`setPower`), upward drift (gravity -0.1, friction 0.96), 8 to 40 tick life and the 8 shrinking sprites; `tick` and `append_mesh` (camera-facing quads, lit by the block light at the particle). `mesh.rs` preloads the 16 `particle/effect_*` and `particle/spell_*` sprites. `BlockParticles::spawn_potion_crumbs(centre, texture)`: the 8 item crumbs (gaussian 0.15 sideways, up to 0.2 up), drawn with the item's particle texture `item/potion_overlay`.
- **Tests:** spell_particles: `a_break_swirls_a_hundred_spells_of_its_colour_from_the_block_bottom`, `spells_drift_up_and_are_gone_within_forty_ticks`, `the_sprite_shrinks_from_the_seventh_to_the_first_frame`. block_particles: `a_potion_breaks_into_eight_untinted_crumbs_flying_out_and_up`. server: `a_witch_potion_breaks_with_the_glass_sound`.
- **Decision:** the swirl and crumbs are client particles, so they need the glue (After the split, item 4); the sound does not.
- **Test steps:**
  1. Find or summon a witch (`/summon minecraft:witch`) and stand 6 blocks from it in survival: each potion it throws breaks with the glass sound where it lands. (The swirl comes with the glue.)
  2. Throw a bottle o' enchanting: same glass sound as entry 3.
- **Kit:**

```
IW4L_MINECRAFT_KIT=experience_bottle*16,witch_spawn_egg*2
```

### 5. Minecraft splash potions are thrown and splash their effects like vanilla

Commit: 90002b2

- **Changed:** new `potions.rs`: the 26.3 `Potions` table (46 potions, read with javap) and the effects' colours; `PotionContents` reads `minecraft:potion_contents` (a bare potion ID, or `potion`, `custom_color` and `custom_effects`) with `all_effects`, `scaled_effects` (`withScaledDuration`), `has_effects`, `instant_potion` (`Potion.hasInstantEffects`) and `color` (the level-weighted mix, `0x385DC6` when empty); `duration_scale` reads `minecraft:potion_duration_scale` (lingering 0.25, tipped arrow 0.125 by default). `PotionEffect` crosses to a mob effect or a player `EffectKind` where those exist. `PlayerPotion` (an effect, a heal, a harm naming the thrower, putting out fire) and `PlayerPotionHit` (player ID, what, where); `apply_to_player(status, potion)` applies it to a `SurvivalStatus` and returns the damage a harm still has to deal; `drink_hits` is what drinking a potion does, from the same table.
- New `splash.rs`: `ThrownSplashPotion.onHitAsPotion`: every living mob and player whose box (grown by the potion's margin) is within 4 blocks of the potion's box gets each effect at `1 - distance/4`: instant health and damage on mobs through the vendored `EntityWorld::splash_instant` (undead the other way round, endermen dodge), lasting effects scaled `(int)(scale * duration * durationScale + 0.5)` and dropped when they would end within 20 ticks. A mob hurt by a player's potion turns on that player (`blame_player`). `of_witch` turns a witch's `PlayerSplash` into the same `PlayerPotionHit`.
- `thrown.rs`: `Throwable::SplashPotion` (thrown 20 degrees up at 0.5, gravity 0.05, sound `entity.splash_potion.throw`). `ThrownView` (item, thrower, positions, age, yaw, pitch, tint) and `append_views`, which draws potions as their bottle over the overlay tinted with the contents' colour (`ThrownItemRenderer`, ground display). A potion or bottle that strikes a mob no longer knocks it back; only snowballs, eggs and pearls do (`onHitEntity`).
- `server.rs`: a splash potion's landing queues its break in its own colour (instant or not), plays the glass sound and splashes. New `Output.player_potions` holds what potions did to players, witch splashes included (`Output.player_splashes` stays for now). New `EntitySnapshot.thrown_views`. **Bridge:** a throw that arrives without its stack's components takes them from the thrower's main hand or offhand as the last tick's inventory showed them (`ServerSim::note_held`, keyed by player ID), so the potion thrown by today's client keeps its contents before the glue sends them.
- Vendored `minecraftoss-entities`: `EntityWorld::splash_instant` is public and returns the hit; a player owner (`PLAYER_TARGET + id`) is credited as `minecraft:player` and a wolf hurt by it turns on the player.
- **Tests:** potions: `strong_poison_is_432_ticks_level_2`, `contents_read_the_potion_its_colour_and_custom_effects`, `colours_mix_by_level`, `durations_scale_down_but_not_to_nothing`, `effects_cross_to_mobs_and_players_where_they_exist`, `instant_potions_heal_four_and_harm_six_a_level_scaled`, `a_drunk_potion_gives_its_effects_whole`, `a_player_takes_what_it_can_hold`. splash: `a_lasting_splash_is_scaled_rounded_and_dropped_under_a_second`, `a_potion_two_blocks_off_gives_the_player_half_the_poison`, `harming_on_a_player_names_the_thrower`, `witch_splashes_reach_the_player_as_the_same_hits`. thrown: `a_splash_potion_leaves_20_degrees_up_at_half_speed_and_shows_its_colour`. server: `a_splash_of_harming_hurts_a_near_cow_more_than_a_far_one`, `a_splash_of_poison_reaches_every_near_player_and_its_thrower`, `a_throw_without_its_stack_takes_it_from_the_throwers_hand`.
- **Decisions:** effects mobs and players don't have yet (speed on the player, strength, night vision...) do nothing, as before; the table is complete so they work once the effect exists. The swirl, the tinted bottle in flight, and the effects on the player need the glue (After the split, items 4 to 6).
- **Test steps:**
  1. Spawn two cows in a line, one 1 block and one 3 blocks from where you aim. Throw a splash potion of Harming at the ground just in front of the near one: both flash red and the near one is pushed harder; a second potion kills the near cow first.
  2. Spawn a zombie and throw Harming at it: it doesn't flash or lose health (harming heals the undead). Throw Healing II at it: it takes damage.
  3. Throw Poison at a pig: it flashes red about every 1.25 s for a while. Spawn a spider, hit it once so it chases you, back off and throw Slowness at it: it slows down clearly.
  4. Throw a potion straight up: it breaks on you with the glass sound.
  5. After the glue (items 5 and 6): the bottle in flight shows the potion's colour, Poison at your feet poisons you (less when it lands 3 blocks away), Healing II heals you, and witches' potions reach you.
- **Kit:**

```
IW4L_MINECRAFT_KIT=splash_potion@harming*3,splash_potion@strong_healing,splash_potion@poison*2,splash_potion@slowness*2,cow_spawn_egg*4,zombie_spawn_egg*2,pig_spawn_egg*2,spider_spawn_egg*2
```

### 6. Minecraft splash water bottles put out fire, campfires, candles and burning mobs and players like vanilla

Commit: a293969

- **Changed:** `thrown.rs`: `douse_cells(block, face)`: the cell in front of the struck face, the struck block and the front cell's four horizontal neighbours (`AbstractThrownPotion.onHitBlock`, north, east, south, west). `server.rs` `douse_fire`: `#fire` is removed without drops (`destroyBlock(pos, false)`), a lit `#candles`/`#candle_cakes` block goes out with `block.candle.extinguish` (volume 1, pitch 1), a lit `#campfires` block goes out with event 1009's `block.fire.extinguish` (volume 0.5, pitch 2.6 plus or minus 0.8); the tags are read from the runtime data. Only a water potion that struck a block douses (`#douses_fire` is water).
- `splash.rs` `wet` (`affectEntitiesAround` for water: `#extinguishes_entities` and `#hurts_water_sensitive_entities`): every mob in the potion's box grown 4, 2, 4 whose feet are within 4 blocks stops burning; an enderman (the only water-sensitive mob we have) takes 1 damage, turns on the thrower and teleports away. Players in reach get `PlayerPotion::Extinguish`.
- **Tests:** thrown: `water_douses_the_face_cell_the_struck_block_and_the_four_sides`. server: `splash_water_puts_out_fire_candles_and_campfires_around_where_it_lands` (a fire two blocks off keeps burning; the candle keeps its count), `splash_water_puts_out_burning_mobs_and_players_and_stings_endermen`.
- **Decision:** players get the extinguish whether they burn or not; the client ignores it when they don't (the server doesn't know the player's fire).
- **Test steps:**
  1. Place netherrack in a plus shape, light fire on each arm and on a block 2 blocks away. Light a campfire (it lights itself when placed) and a candle (flint and steel) right next to where you will aim.
  2. Throw the water bottle at the middle: the fires within one block, the campfire and the candle go out with the hiss; the fire 2 blocks away keeps burning.
  3. Use flint and steel on the block a cow stands on so it catches fire, then throw water next to it: it stops burning.
  4. Spawn an enderman and throw water at it: it teleports away and takes a hit.
  5. After the glue (item 6): stand in fire until you burn, throw water at your feet: your flames go out.
- **Kit:**

```
IW4L_MINECRAFT_KIT=splash_potion@water*6,netherrack*16,campfire*2,candle*4,flint_and_steel,cow_spawn_egg*2,enderman_spawn_egg
```

### 7. Minecraft lingering potions leave a shrinking effect cloud like vanilla

Commit: (next entry)

- **Changed:** `thrown.rs`: `Throwable::LingeringPotion` (as the splash potion, sound `entity.lingering_potion.throw`, tinted in flight). On landing it does the water part of a splash (douse, wet), breaks with its colour and sound, and when it has effects leaves an effect cloud at the mob or player it struck, or where it broke (`ThrownLingeringPotion.onHitAsPotion`).
- New `effect_cloud.rs`: `EffectCloud::lingering`: radius 3, -0.5 per use, -3/600 per tick, 600 ticks after a 10-tick wait, the potion's effects at the lingering item's `potion_duration_scale` (0.25). `tick(visitors)` is `AreaEffectCloud.serverTick`: gone after `duration + wait` ticks or below radius 0.5; every 5 ticks it touches each visitor whose box meets its 0.5-high slab and whose feet are inside the radius, unless touched in the last 20 ticks or unable to take any of its effects. `CloudView` (position, radius, colour, waiting) for the client.
- `server.rs`: `ServerSim::tick_clouds(players)` (run before the throws each tick, so a new cloud first ticks on the next one, as a fresh entity does): touched mobs get the effects (instant ones at half strength through `splash_instant`, the thrower blamed), touched players get `Output.player_potions` hits. `EntitySnapshot.clouds` carries the views. `spell_particles.rs` `SpellParticles::cloud(position, radius, colour, waiting)`: one client tick of the cloud's `entity_effect` particles (`AreaEffectCloud.clientTick`: `ceil(pi r^2)` motionless particles over the disc, or 2 white or coloured ones at 0.2 while waiting).
- **Tests:** effect_cloud: `a_lingering_cloud_carries_a_quarter_of_the_potion`, `the_cloud_is_gone_by_tick_610`, `a_victim_waits_20_ticks_and_each_use_shrinks_the_cloud`, `only_those_inside_the_radius_and_able_are_touched`, `a_cloud_without_effects_touches_nobody`. spell_particles: `a_cloud_fills_its_circle_each_tick`. thrown: `a_lingering_potion_flies_like_a_splash_potion`. server: `a_lingering_poison_cloud_poisons_whoever_stands_in_it_once_a_second` (5 touches 20 ticks apart, 225-tick poison each, the fifth use ends the cloud), `a_lingering_harming_cloud_hurts_a_cow_at_half_strength` (3 damage).
- **Decisions:** clouds are not saved yet (vanilla saves them; they live 30 s at most). Without the glue the cloud is invisible but works.
- **Test steps:**
  1. Spawn a cow, throw Lingering Harming at the ground next to it: about every second it flashes red and loses health while it stays inside; the effect stops after 4 or 5 hits as the cloud shrinks.
  2. Spawn a spider and throw Lingering Poison under it: it keeps flashing from poison while inside.
  3. After the glue (items 6 and 8): the cloud shows as a disc of drifting purple-green specks that shrinks over about 25 s, and standing in Lingering Poison poisons you again about once a second.
- **Kit:**

```
IW4L_MINECRAFT_KIT=lingering_potion@harming*2,lingering_potion@poison*2,lingering_potion@regeneration,cow_spawn_egg*4,spider_spawn_egg*2
```

## After the split

The glue still needed in the game files (today `render_anim/src/minecraft_*.rs`, after the split their new crate). Each item names the function, its inputs and the call site.

1. **Throws carry the thrower and the stack.** In `use_hands` (`minecraft_world.rs`, the throwables branch, now at about line 1181) and `Entities::throw` (`minecraft_entities.rs:269`): build a `minecraft_terrain::thrown::Throw { owner: PLAYER (the local player's ID), item: &stack.id, components: stack.components.as_ref(), eye, yaw, pitch, motion, on_ground }` and call `Thrown::launch(&throw, &mut self.random)` instead of `Thrown::throw`. With several players the server makes the throw from the player's own state.
2. **Pearl teleports by player.** In `Entities::poll` (`minecraft_entities.rs:672`): read `output.pearl_teleports` and keep only the entries whose player is the local player (instead of every `output.pearl_landings` entry). `pearl_landings` can then be dropped from `Output`.
3. **`/give` with potion contents** (track C's crate, `minecraft_commands/src/args.rs:248`, and the call in `minecraft_chat.rs:209`): parse `potion_contents={potion:"minecraft:x"}` in the item argument and build the stack with `minecraft_terrain::crafting::kit_potion_stack(recipes, id, 1, &enchantments, potion)` instead of `kit_stack`.
4. **Potion break swirl and crumbs.** `Entities` (`minecraft_entities.rs`) gets a `spell: minecraft_terrain::spell_particles::SpellParticles` (`SpellParticles::new(seed)`, cleared with the other particles on a world change). In `Entities::poll`, for each `output.potion_breaks` entry: `self.spell.burst(b.position, b.color, b.instant)` and `minecraft_mining`'s particles `spawn_potion_crumbs(spell_particles::break_center(b.position).as_vec3(), &ResourceId::parse("minecraft:item/potion_overlay"))` (a new `Mining::potion_burst` beside `eat_burst`, `minecraft_mining.rs:153`). Tick it with `self.portal.tick()` (`minecraft_entities.rs:618`) and draw it with `self.spell.append_mesh(&mut out.translucent, atlas, forward, partial, light)` beside `self.portal.append_mesh` (`minecraft_entities.rs:917`).
5. **Potions drawn in flight.** In `Entities::poll` (`minecraft_entities.rs:667`) keep `snapshot.thrown_views` (as `self.thrown` is kept today) and in the mesh build (`minecraft_entities.rs:905`) call `minecraft_terrain::thrown::append_views(&mut out.items, &self.thrown_views, partial, camera, atlas, forward, light)` instead of `append_sprites`. `EntitySnapshot.thrown` can then go.
6. **Potions on the player.** In `Entities::poll` collect `output.player_potions` whose `player` is the local player, and where the pearl landings are applied (`minecraft_world.rs:1610`), for each: `if let Some(damage) = minecraft_terrain::potions::apply_to_player(&mut survival.status, &hit.potion)` then hurt the player with `damage` past armour (`Armor::default()`), from `hit.at`, as `DamageCause::Magic` (indirect magic when `by` is set), skipped while invulnerable (creative). `PlayerPotion::Extinguish` also stops the burning overlay.
7. **Drinking uses the table.** `MinecraftSurvival::drink_potion` (`minecraft_survival.rs:573`) takes the stack's components and does `for hit in potions::drink_hits(&PotionContents::from_components(components), potions::duration_scale(id, components))` with the same `apply_to_player` and harm handling, instead of its own list (it then covers turtle master, custom effects and every long and strong form).
8. **Lingering clouds drawn.** In the client tick beside `self.portal.tick()` (`minecraft_entities.rs:618`): for each `snapshot.clouds` view, `self.spell.cloud(view.position, view.radius, view.color, view.waiting)` (keep the last snapshot's `clouds` like `self.orbs`), then `self.spell.tick()`; the particles draw with item 4's `append_mesh`.
