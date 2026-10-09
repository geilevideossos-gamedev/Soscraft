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

Commit: 4f607a8

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

### 8. Minecraft tridents are thrown, stick, hit for 8 and come back with Loyalty like vanilla

Commit: eeffd7c

- **Changed:** new `trident.rs`. `release(inventory, slot, used_ticks, creative, wet, random)` is `TridentItem.releaseUsing`: nothing before 10 ticks of charge, nothing when the next use would break it or when it has Riptide outside water and rain; otherwise it costs one durability (Unbreaking spares it with chance L/(L+1), creative never wears it) and gives `Release::Throw(stack)` (taken from the hand, except in creative) or `Release::Riptide { strength, sound }`. `can_start` is `TridentItem.use`. `ThrownTrident::launch` (`spawnProjectileFromRotation(0, 2.5, 1)` from the eyes less 0.1, with the thrower's motion) and `tick` (`ThrownTrident.tick` with `AbstractArrow.tick`): flight under 0.05 gravity and 0.99 drag, the first mob or player in its path (not the thrower until it has left it) is struck once (`Struck`, then it bounces back by (-0.01, -0.1, -0.01) and turns 170-190 degrees), otherwise it sticks 0.05 back from the block it hit with `item.trident.hit_ground` and shakes for 7 ticks; it falls again when that block goes; a plain trident despawns after 1200 ticks in the ground, a Loyalty one never does. Loyalty L: after a hit or 4 ticks in the ground it flies back without collision (`v*0.95 + toward the eyes * 0.05 L`, rising 0.015 L of the height gap a tick) with one `item.trident.return`; if its thrower died it drops as an item. `touched_by`/`taken_by`: only the thrower picks it up, once it stopped (in the ground or returning) and stopped shaking; a creative throw is just removed. `TridentView` and `append_tridents` draw the `TridentModel` (pole, base, three spikes, `entity/trident/trident`, 32x32) turned as `ThrownTridentRenderer` (Y by yaw - 90, Z by pitch + 90) with the glint when enchanted.
- `server.rs`: `Command::ThrowTrident`/`ServerHandle::throw_trident`, `ServerSim::tick_tridents` (struck mobs take 8 `minecraft:trident` damage after armour, knocked back along the flight, and turn on the thrower; struck players go to the new `Output.players_struck`), `pickup_tridents` (run with the item pickup, so a trident the thrower walks over goes into the inventory through the existing `Output.picked`, no glue needed), `EntitySnapshot.tridents`. `thrown.rs`: `hurt_by` (any damage and damage type; `hurt` is its 0-damage case), `PlayerStruck`. `mesh.rs` adds the trident texture to the atlas.
- **Tests:** trident: `released_before_10_ticks_does_not_throw`, `a_trident_one_use_from_breaking_is_not_thrown`, `riptide_needs_water_or_rain_and_launches_instead_of_throwing`, `a_trident_flies_at_two_and_a_half_and_sticks_where_it_lands`, `a_trident_strikes_a_mob_once_and_drops_back`, `loyalty_pulls_it_back_to_its_thrower`, `a_loyal_trident_never_despawns_but_drops_when_its_thrower_dies`, `only_its_thrower_picks_it_up_once_it_has_settled`, `the_spikes_lead_along_the_heading`, `the_model_is_a_pole_a_base_and_three_spikes_on_a_32_sheet`. server: `a_trident_hits_a_cow_for_8_sticks_and_only_its_thrower_takes_it_back`, `a_trident_strikes_another_player_for_8`.
- **Decisions:** the Loyalty, Riptide and Unbreaking numbers are the enchantments' 26.3 effect formulas (`trident_return_acceleration` L, `trident_spin_attack_strength` 1.5 + 0.75 (L-1), `item_damage` remove_binomial L/(L+1)), written in code like the other enchantment effects here. Impaling (water mobs only) and Channeling (needs lightning) are not modelled. The trident in the hand stays the flat item: the 3D hand model needs the item model's display-context `select`, which belongs to the held-items group (b3); `trident::MODEL` is ready for it.
- **Test steps** (after glue items 9 and 10; until then right-click does nothing with a trident):
  1. Hold right-click with the plain trident for a second and release: it flies far and fast and sticks in the ground at an angle, with the thud. Releasing before half a second does nothing.
  2. Throw it at a zombie: the zombie takes a big hit (about 8) and is pushed, the trident drops near it. Walk over it: it goes back into your inventory one durability lower (F3+H tooltip or the durability bar).
  3. Throw the Loyalty trident at the ground: after a moment it flies back to you with the return sound and lands in your inventory.
  4. Throw the plain trident and let a friend (or nobody) walk over it: only you pick it up.
  5. Creative: the trident stays in your hand while a copy flies.
- **Kit:**

```
IW4L_MINECRAFT_KIT=trident,trident+loyalty=3,zombie_spawn_egg*4
```

### 9. Minecraft wind charges burst and knock back like vanilla

Commit: d37208e

- **Changed:** `thrown.rs`: `Throwable::WindCharge` (`WindChargeItem.use`: thrown level at 1.5 from the eyes themselves, sound `entity.wind_charge.throw`), flying straight with no gravity and no drag (`AbstractWindCharge`, inertia 1), bursting where it climbs 30 blocks above the world. `Throwable::cooldown` (pearl 20 ticks, wind charge 10) for the client's per-item cooldown. Right-click throws it with no glue.
- New `wind.rs`: `burst_point` (a quarter block off the face it hit, or where it struck a mob), `exposure` (`ServerExplosion.getSeenPercent`: the share of the box's sample grid with a clear line to the centre), `push` (`hurtEntities` with the wind charge's calculator: within twice the 1.2 radius, `(1 - d) * exposure * 1.22` along centre-to-eyes, no damage), `burst` (pushes every mob and returns each player's push). `server.rs`: a landed wind charge first hurts the mob or player it struck for 1 (`minecraft:wind_charge`), then bursts: mobs and players pushed, level items, orbs, TNT and falling blocks pushed too (vendored `Level::wind_burst`, push only), `entity.wind_charge.wind_burst` at volume 4, pitch about 0.7. New `Output.players_pushed: Vec<(player, push)>`.
- Vendored `minecraftoss-world`: `Level::wind_burst(center, radius, knockback)`.
- **Tests:** wind: `a_burst_pushes_away_and_weakens_with_distance`, `a_wall_shields_what_is_behind_it`, `it_bursts_a_quarter_block_off_the_face_it_hit`. thrown: `a_wind_charge_flies_straight_from_the_eye_without_slowing`, `a_wind_charge_bursts_where_it_climbs_out_of_the_world`. server: `a_wind_charge_bursts_on_the_ground_and_throws_cows_and_its_thrower_up`, `a_wind_charge_hits_a_mob_for_1_before_it_bursts`.
- **Decisions:** blocks the burst should trigger (doors, trapdoors, fence gates, buttons, levers, bells, lit candles) and the gust particles are left for later (the vendored explosion has no trigger-block behaviour yet); the charge is drawn as its item sprite, not `WindChargeRenderer`'s swirl. The player's own push and the fall-damage reset need the movement glue (item 14).
- **Test steps:**
  1. Right-click a wind charge at the ground 4 blocks away: it flies straight and fast and bursts with the whoosh. Right-clicking again within half a second still throws (the cooldown comes with glue item 13).
  2. Spawn three cows close together and throw a wind charge at the ground between them: they fly up and apart and take no damage.
  3. Throw one straight at a cow: it flashes red once (1 damage) and is flung.
  4. Drop some dirt items and burst a charge next to them: they jump away.
  5. After the glue (item 14): a charge at your feet launches you up and you take no fall damage from that jump.
- **Kit:**

```
IW4L_MINECRAFT_KIT=wind_charge*64,cow_spawn_egg*6,dirt*16
```

### 10. Minecraft tridents are saved with the world and still know their thrower

Commit: 2593421

- **Changed:** `trident.rs`: `ThrownTrident::to_tag(owner_uuid)` writes the vanilla `minecraft:trident` entity (`Pos`, `Motion`, `Rotation`, `Owner` as the thrower's UUID ints, `LeftOwner`, `HasBeenShot`, `life`, `shake`, `inGround`, `inBlockState`, `pickup`, `damage`, `crit`, `PierceLevel`, `SoundEvent`, `DealtDamage` and the `item` stack with its components); `from_tag` reads it back. `ThrownTrident.owner_uuid` (set from `EntityWorld::uuid_of(PLAYER_TARGET + owner)` when thrown). `server.rs`: tridents are saved with their chunk's entities (autosave, unload and shutdown), taken out when the chunk unloads, and a loaded one finds its thrower again by UUID on the first tick that player is in the world; until then nobody can pick it up and a Loyalty one waits. `server_mobs::json_tag` is crate-public for the item components.
- **Tests:** trident: `a_stuck_trident_saves_as_the_vanilla_entity_and_loads_back`. server: `a_saved_trident_comes_back_and_still_knows_its_thrower`.
- **Decision:** before this a trident stuck in the ground when the world closed was lost with the item in it; now it stays, as in vanilla. Thrown snowballs, potions and clouds are still not saved (they live a few seconds).
- **Test steps** (after glue items 9 and 10):
  1. Throw the plain trident into a hill, save and quit to the menu, open the world again: it is still stuck there; walk over it to take it back.
  2. Do the same with the Loyalty trident thrown at the ground just before quitting: after reopening it flies back to you.
- **Kit:**

```
IW4L_MINECRAFT_KIT=trident,trident+loyalty=3
```

### 11. Minecraft tridents wait where the world is not loaded

Commit: 657de8f

- **Changed:** `server.rs` `tick_tridents` skips a trident whose chunk is not loaded, as vanilla doesn't tick entities there. Before, a trident thrown or returning into unloaded ground found no blocks, fell out of the world and was lost.
- **Tests:** server: `a_trident_waits_where_the_world_is_not_loaded`.
- **Test steps** (after glue items 9 and 10):
  1. Stand at the edge of a cliff over the sea and throw the trident as far out as you can at a low render distance: it never disappears; fly or swim out and it lies on the sea floor or in the ground where it came down.
- **Kit:**

```
IW4L_MINECRAFT_KIT=trident
```

### Group summary: throwables

- Without glue, right-click already throws bottles o' enchanting, splash and lingering potions and wind charges (their contents come from the thrower's hand through `ServerSim::note_held`), with the vanilla flight, break sounds, experience, effects on mobs, water dousing, lingering clouds and wind pushes on mobs and items. Witch and player potions break with the glass sound.
- The glue (items 1 to 14 below) adds what the client has to do: the potion swirl and crumbs, tinted bottles and the trident model in flight, potion effects, trident hits and wind pushes on the player, the trident's charge and release, Riptide, per-item cooldowns and the keyed pearl teleports.

## Group: lids

Brief: `b2-shulker-lid.md`. `crates/minecraft_terrain/src/terrain.rs` is off limits in this part of the track (like the game files), so the section side of "draw an animating lid apart from its section" is glue too (After the split, item 15); everything it calls is built and tested here.

### 1. Minecraft shulker box model takes a lid opening progress

Commit: cf6f8a2

- **Changed:** `special_models.rs`: new `shulker_box_model(block, progress)` (the block model's shulker branch moved into it; `block_model` passes 0). `push_shulker` takes the progress and poses the lid as `ShulkerBoxRenderer$ShulkerBoxModel.setupAnim` (26.3, javap): `setPos(0, 24 - p * 0.5 * 16, 0)` and `yRot = 270 * p` degrees, under the box's facing pose; the base never moves. The item model passes 0.
- **Tests:** `a_shulker_lid_rises_half_a_block_and_turns_three_quarters` (progress 0 is today's model; at 1 an upward lid spans y 0.75 to 1.5, a downward one -0.5 to 0.25, a north one z -0.5 to 0.25, all times the 0.9995 scale; at 0.5 the lid is turned 135 degrees; the base is the same at 0, 0.5 and 1).
- **Test steps:** nothing changes in game yet (every box is still drawn closed):
  1. Place a shulker box on the floor, one on a wall and one on a ceiling, and hold one: they look exactly as before. Screenshot.
- **Kit:**

```
IW4L_MINECRAFT_KIT=shulker_box,red_shulker_box*4
```

### 2. Minecraft chest model takes a lid openness

Commit: b36fe90

- **Changed:** `model.rs`: new `opened_chest_model(block, openness)`: the closed chest model (single, left or right half, plain, trapped, ender or copper, any facing) with the lid and the lock turned about the hinge at (0, 9, 1) px by `-(1 - (1 - o)^3) * pi/2`, as `ChestRenderer.submit` eases the openness and `ChestModel.setupAnim` sets `lid.xRot = lock.xRot` (26.3, javap: lid box (1, 0, 0) 14x5x14 and lock (7, -2, 14) 2x4x1, both at offset (0, 9, 1)). At openness 0 it is the closed model, unrotated, so the section mesh and the frame pass draw the same vertices.
- **Tests:** `a_chest_lid_swings_up_about_its_back_hinge` (0 is the closed model; at 1 the body stays, the lid stands up behind the hinge, y 9 to 23 and z -4 to 1 px for a south chest, the lock on top, a north ender chest mirrored; at 0.5 the eased lid is at 78.75 degrees).
- **Test steps:** nothing changes in game yet.
- **Kit:** none.

### 3. Minecraft block entity lids tick open and shut like vanilla

Commit: 66ef4c3

- **Changed:** new `lids.rs` (client side, one per level, keyed by `BlockPos`):
  - `LidKind::of(block)`: shulker boxes, and chests, trapped chests, ender chests and copper chests (waxed too). `lid_model(block, progress)` gives entry 1's or entry 2's model.
  - `ShulkerLid` is `ShulkerBoxBlockEntity` (26.3, javap): `set_open_count` is `triggerEvent(1, count)` (0 closes, 1 opens, any other count changes nothing), `tick` is `updateAnimation` (0.1 a tick, so 10 ticks each way, f32 like vanilla), `progress(partial)` is `getProgress`.
  - `ChestLid` is `ChestLidController` (`shouldBeOpen(count > 0)`, `tickLid` 0.1 a tick clamped, `getOpenness`).
  - `Lids` drives them from block events, `block_event(scene, pos, SET_OPEN_COUNT, count)` (`Level.blockEvent(pos, block, 1, count)`), and tracks each lid's cell for the terrain: a lid that starts to open asks for its cell to be taken out of its section (`take_apart_changes` gives `(pos, true)`), and only moves once `settle(scene, cell_settled)` saw that section rebuilt, so the static lid never shows inside a rising one; a lid back at rest asks for its cell back (`(pos, false)`) and is drawn until the section has it again. A box broken or replaced while open gives its cell back and draws nothing. `posed(scene, partial)` lists every lid to draw this frame; the two halves of a double chest both swing as the wider open one (`ChestBlock.opennessCombiner`, the max). `lid_closed(pos)` is the shulker's `AnimationStatus.CLOSED` for `ShulkerBoxBlock.canOpen`.
  - `ApartCells` is the terrain's side: `set(pos, apart)` names the section to rebuild once per change and `in_section` lists a section's apart cells for its compile job.
- **Tests:** `a_shulker_lid_opens_in_ten_ticks_and_closes_in_ten`, `only_counts_of_zero_and_one_move_a_shulker_lid`, `a_lid_half_a_tick_on_is_halfway_between_its_ticks`, `a_chest_lid_clamps_and_turns_back_from_where_it_is`, `a_new_lid_waits_for_its_section_before_it_moves`, `a_closed_lid_goes_back_to_its_section_once`, `a_lid_reopened_while_closing_turns_back_without_a_jump`, `a_lid_shut_before_its_section_returned_it_is_taken_apart_again`, `a_broken_box_leaves_no_lid_and_gives_its_cell_back`, `a_count_that_opens_nothing_makes_no_lid`, `both_halves_of_a_double_chest_swing_as_the_wider_open_one`, `every_chest_and_shulker_box_has_a_lid`, `apart_cells_name_their_section_once_per_change`.
- **Decisions:** a lid waits for its section before it moves (a frame or two late) rather than showing two lids; a lid only goes back to its section one tick after it shut, when it is drawn shut at every partial tick. The lids are client state like vanilla's client block entities; the server keeps the openers (entry 5). Vanilla does not save lid progress, so neither does this.
- **Test steps:** nothing changes in game yet (After the split, items 15 to 17).
- **Kit:** none.

### 4. Minecraft open lids are drawn every frame apart from their section

Commit: (next entry)

- **Changed:** `mesh.rs`: `ItemVisuals::append_open_lids(mesh, lids, level, packs, atlas, light)` draws each `PosedLid` (from `Lids::posed`) at its real cell through `append_block`, the same function the section build uses, with `lid_model(block, progress)`: same light (an entity sheet is lit at its own cell), same entity shading, same atlas regions. The block is drawn dry (no `waterlogged`), because the section keeps a waterlogged chest's water (glue item 15 draws only the fluid for an apart cell).
- **Tests:** `a_shut_lid_drawn_apart_is_its_section_block_and_leaves_the_water_to_the_section`: for a waterlogged east chest and a north red shulker box, the frame pass at progress 0 gives exactly the section's model vertices and indices, `append_baked` with no quads (what the section keeps of an apart cell) gives exactly the rest (the water, none for the box), and at progress 1 the lid has moved.
- **Decision:** the frame pass goes into the entities' `items` mesh, like falling blocks and primed TNT. The crack overlay and the selection outline stay the closed block's while a lid is open, as before.
- **Test steps:** nothing changes in game yet (After the split, items 15 and 16).
- **Kit:** none.

## After the split

The glue still needed in the game files (today `render_anim/src/minecraft_*.rs`, after the split their new crate). Each item names the function, its inputs and the call site.

1. **Throws carry the thrower and the stack.** In `use_hands` (`minecraft_world.rs`, the throwables branch, now at about line 1181) and `Entities::throw` (`minecraft_entities.rs:269`): build a `minecraft_terrain::thrown::Throw { owner: PLAYER (the local player's ID), item: &stack.id, components: stack.components.as_ref(), eye, yaw, pitch, motion, on_ground }` and call `Thrown::launch(&throw, &mut self.random)` instead of `Thrown::throw`. With several players the server makes the throw from the player's own state. Once the components arrive with every throw, the hand bridge `ServerSim::note_held` (entry 5) and the `TickInput.pickup` call to it can go.
2. **Pearl teleports by player.** In `Entities::poll` (`minecraft_entities.rs:672`): read `output.pearl_teleports` and keep only the entries whose player is the local player (instead of every `output.pearl_landings` entry). `pearl_landings` can then be dropped from `Output`.
3. **`/give` with potion contents** (track C's crate, `minecraft_commands/src/args.rs:248`, and the call in `minecraft_chat.rs:209`): parse `potion_contents={potion:"minecraft:x"}` in the item argument and build the stack with `minecraft_terrain::crafting::kit_potion_stack(recipes, id, 1, &enchantments, potion)` instead of `kit_stack`.
4. **Potion break swirl and crumbs.** `Entities` (`minecraft_entities.rs`) gets a `spell: minecraft_terrain::spell_particles::SpellParticles` (`SpellParticles::new(seed)`, cleared with the other particles on a world change). In `Entities::poll`, for each `output.potion_breaks` entry: `self.spell.burst(b.position, b.color, b.instant)` and `minecraft_mining`'s particles `spawn_potion_crumbs(spell_particles::break_center(b.position).as_vec3(), &ResourceId::parse("minecraft:item/potion_overlay"))` (a new `Mining::potion_burst` beside `eat_burst`, `minecraft_mining.rs:153`). Tick it with `self.portal.tick()` (`minecraft_entities.rs:618`) and draw it with `self.spell.append_mesh(&mut out.translucent, atlas, forward, partial, light)` beside `self.portal.append_mesh` (`minecraft_entities.rs:917`).
5. **Potions drawn in flight.** In `Entities::poll` (`minecraft_entities.rs:667`) keep `snapshot.thrown_views` (as `self.thrown` is kept today) and in the mesh build (`minecraft_entities.rs:905`) call `minecraft_terrain::thrown::append_views(&mut out.items, &self.thrown_views, partial, camera, atlas, forward, light)` instead of `append_sprites`. `EntitySnapshot.thrown` can then go.
6. **Potions on the player.** In `Entities::poll` collect `output.player_potions` whose `player` is the local player, and where the pearl landings are applied (`minecraft_world.rs:1610`), for each: `if let Some(damage) = minecraft_terrain::potions::apply_to_player(&mut survival.status, &hit.potion)` then hurt the player with `damage` past armour (`Armor::default()`), from `hit.at`, as `DamageCause::Magic` (indirect magic when `by` is set), skipped while invulnerable (creative). `PlayerPotion::Extinguish` also stops the burning overlay.
7. **Drinking uses the table.** `MinecraftSurvival::drink_potion` (`minecraft_survival.rs:573`) takes the stack's components and does `for hit in potions::drink_hits(&PotionContents::from_components(components), potions::duration_scale(id, components))` with the same `apply_to_player` and harm handling, instead of its own list (it then covers turtle master, custom effects and every long and strong form).
8. **Lingering clouds drawn.** In the client tick beside `self.portal.tick()` (`minecraft_entities.rs:618`): for each `snapshot.clouds` view, `self.spell.cloud(view.position, view.radius, view.color, view.waiting)` (keep the last snapshot's `clouds` like `self.orbs`), then `self.spell.tick()`; the particles draw with item 4's `append_mesh`.
9. **Trident charge and release.** In `use_hands` (`minecraft_world.rs`, beside the shield's `Blocking`): right-click with a trident starts a `Charging { slot, ticks }` hand state when `trident::can_start(&inventory, stack, wet)` (`wet`: the player is in water or in rain); the held trident uses its raised (`using_item`) pose while charging; count game ticks while the key is held; on release call `trident::release(&mut inventory, slot, ticks, creative, wet, &mut random)`. On `Release::Throw(stack)`: `server.throw_trident(ThrownTrident::launch(&Throw { owner: PLAYER, item, components, eye, yaw, pitch, motion, on_ground }, stack, creative, &mut random))` and play `trident::throw_sound(&stack)` at volume 1, pitch 1. On `Release::Riptide`: item 11.
10. **Tridents drawn.** Keep `snapshot.tridents` in `Entities::poll` and call `minecraft_terrain::trident::append_tridents(&mut out.items, &self.tridents, partial, atlas, light)` beside the thrown sprites (`minecraft_entities.rs:905`).
11. **Riptide.** On `Release::Riptide { strength, sound }`: add `trident::riptide_push(yaw, pitch, strength)` to the player's velocity (in blocks a tick, MW2 units are `BLOCK * 20` per second), lift by 1.2 blocks if on the ground, start a 20-tick spin attack that hits mobs it touches for 8, and play `sound`.
12. **Players struck.** In `Entities::poll` read `output.players_struck` for the local player: hurt it by `damage` with armour, from the thrower, as a trident (`kind` `minecraft:trident`, 8) or wind charge (`minecraft:wind_charge`, 1) hit, knocked back along `velocity`.
13. **Per-item throw cooldowns.** `use_hands` (`minecraft_world.rs:1186`) and the cooldown overlay (`minecraft_world.rs:771`): replace the pearl-only `hand.pearl_cooldown` with a cooldown per item from `Throwable::cooldown()` (pearl 20 ticks, wind charge 10), shown on each item's slot.
14. **Wind pushes the player.** In `Entities::poll` read `output.players_pushed` for the local player and add the push (blocks a tick) to the MW2 velocity; as vanilla's `Player.currentImpulseImpactPos`, ignore the fall damage of the fall that follows a wind push until the player lands.
15. **Sections leave apart cells to the frame pass** (`crates/minecraft_terrain/src/terrain.rs`, off limits in this part of the track; the lids group's entries 3 and 4 build everything it calls):
    - `TerrainStream` gets `apart: crate::lids::ApartCells` (`Default::default()` in the constructor; not cleared on a view-distance change, so rebuilt sections still skip the cells).
    - New `pub fn draw_apart(&mut self, pos: BlockPos, apart: bool)`: `if let Some(section) = self.apart.set(pos, apart) { if self.sections.section(section).is_some() { self.sections.set_dirty(section, true); self.player_sections.insert(section); self.serials.remove(&section); } }`. Only the cell's own section rebuilds: no relight, no `record_edits`, no neighbours (shulker boxes and chests are `noOcclusion`, so no neighbour face depends on them). Removing the serial drops a compile already in flight from before the change, so `cell_settled` can't turn true on a mesh that still holds the cell.
    - `CompileJob` gets `apart: Vec<BlockPos>`, filled in `frame()` where the jobs are pushed (`apart: self.apart.in_section(pos)`), and `compile_section` passes `&job.apart` to both `build_section_blocks` calls.
    - `build_section_blocks(.., apart: &[BlockPos])`: right after `let index = usize::from(state.0);` add `if apart.contains(&pos) { mesh::append_baked(view, pos, &[], states.tint_kind[index], states.holds_fluid[index], |_, _| false, |_| false, atlas, tint, light, &mut mesh, &mut transparent)?; continue; }`. It is the same on both build paths, so `MINECRAFTOSS_MESH_CHECK` still holds, and a waterlogged chest keeps its water (entry 4's test proves the split adds up to the whole cell).
    - Test, modelled on the section tests that call `build_section_blocks` (`ModelCache::default()`, `Neighborhood::of`, `View::new`): a section with a shulker box and stone around it, built with `apart = [cell]`, has no vertex inside the box's cell and every other vertex unchanged, on the baked and the unbaked path.
16. **Lids in the client entities** (`minecraft_entities.rs`):
    - `Entities` gets `lids: minecraft_terrain::lids::Lids` (`Default`, cleared with the other client state on a world or dimension change; one per level).
    - `Entities::tick` (:610), inside `if ticked` beside `self.falling.tick_sent()`: `self.lids.tick();`.
    - `Entities::meshes` (:903), after `append_moving_blocks`: `self.lids.settle(scene, cell_settled); let lids = self.lids.posed(scene, partial); let _ = self.items.append_open_lids(&mut out.items, &lids, scene, packs, atlas, light);`.
    - `minecraft_world.rs`, right after the `entities.meshes(..)` call (:1908): `for (pos, apart) in entities.lids.take_apart_changes() { world.stream.draw_apart(pos, apart); }` (before the next frame's `world.stream.frame`, :1457).
