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

Commit: 8695be2

- **Changed:** `mesh.rs`: `ItemVisuals::append_open_lids(mesh, lids, level, packs, atlas, light)` draws each `PosedLid` (from `Lids::posed`) at its real cell through `append_block`, the same function the section build uses, with `lid_model(block, progress)`: same light (an entity sheet is lit at its own cell), same entity shading, same atlas regions. The block is drawn dry (no `waterlogged`), because the section keeps a waterlogged chest's water (glue item 15 draws only the fluid for an apart cell).
- **Tests:** `a_shut_lid_drawn_apart_is_its_section_block_and_leaves_the_water_to_the_section`: for a waterlogged east chest and a north red shulker box, the frame pass at progress 0 gives exactly the section's model vertices and indices, `append_baked` with no quads (what the section keeps of an apart cell) gives exactly the rest (the water, none for the box), and at progress 1 the lid has moved.
- **Decision:** the frame pass goes into the entities' `items` mesh, like falling blocks and primed TNT. The crack overlay and the selection outline stay the closed block's while a lid is open, as before.
- **Test steps:** nothing changes in game yet (After the split, items 15 and 16).
- **Kit:** none.

### 5. Minecraft containers count their openers per player like vanilla

Commit: d919dff

- **Changed:** new `openers.rs` (server side, one per level): `Openers` keeps, per container cell, the set of player UUIDs that have it open (`ContainerOpenersCounter` and `ShulkerBoxBlockEntity.openCount`, 26.3, javap). `open(pos, uuid, spectator)` (spectators are not counted, as `startOpen` skips them; a player counts once) and `close(pos, uuid)` give an `OpenerEvent { pos, count, change }`: its `block_event()` is `Level.blockEvent(pos, block, 1, count)` for every client's `Lids::block_event`, and its `change` is `Opened` on the first opener and `Closed` when the last one leaves (`onOpen`/`onClose`: the sound, the barrel's `open`). `recheck(has_open)` is `recheckOpeners` (every `RECHECK_TICKS` = 5), `leave(uuid)` closes what a leaving player had open, `removed(pos)` forgets a broken container without a close. `Openers::counted(block)`: chests, trapped and copper chests, ender chests, shulker boxes and barrels.
- `sound(block, pos, change)`: the vanilla event and where it plays, at volume 0.5 (`pitch(random)` is 0.9 to 1): shulker boxes and ender chests at the block centre; a single chest at its centre; a double chest once, from its right half moved half a block towards the left one (`ChestBlockEntity.playSound` skips the left half and moves the right one's along `getConnectedDirection`); copper chests with their weathering sounds; a barrel from its opening face (`BarrelBlockEntity.playSound`: centre plus half its facing). `opened_state(block, change)`: the barrel's `open` property (`BarrelBlockEntity.updateBlockState`).
- **Tests:** `the_first_opener_opens_it_and_the_last_one_shuts_it`, `spectators_look_inside_without_opening_it`, `a_player_who_leaves_shuts_what_only_it_had_open`, `a_recheck_forgets_players_who_no_longer_have_it_open`, `a_broken_container_forgets_its_openers_without_a_sound`, `opening_sounds_come_from_where_vanilla_plays_them`, `a_barrel_shows_open_while_someone_has_it_open`.
- **Decisions:** a set of UUIDs instead of vanilla's bare counter, so the count can't drift when a player's close goes missing (vanilla needs its 5-tick recheck for that); the events and sounds are the same. This also covers brief commit 6 (the barrel sounds from its front face) once glue item 17 uses it.
- **Test steps:** nothing changes in game yet (After the split, item 17).
- **Kit:** none.

### Group summary: lids

- Built and tested in `minecraft_terrain`: the shulker box and chest models posed by a progress (entries 1 and 2), the client lids ticking like vanilla and handing a cell between its section and the frame pass (entry 3), the frame pass itself (entry 4) and the server's per-player openers with the vanilla block events and sounds (entry 5).
- Nothing shows in game until After the split items 15 (the section skip, in `terrain.rs`), 16 (the lids in `Entities`) and 17 (opening and closing feed the openers) are in.
- Not done (brief commit 7, deferred there too): an opening shulker box does not yet grow its collision or push players and mobs (`Shulker.getProgressAabb`, `moveCollidedEntities`).
- **Test steps** (after glue items 15 to 17):
  1. Place a red shulker box on the floor. Open it: the lid rises half a block and turns three quarters round in half a second. Close it: it comes back down in half a second. Screenshot 1, mid-animation.
  2. Do the same with a box on a wall and one on a ceiling: the lid slides out of the wall, and down out of the ceiling. Screenshots 2 and 3.
  3. Open a box, press Escape and open it again at once: the lid turns back from where it was, with no jump. Screenshot 4.
  4. A bottom slab on top of a box keeps it shut (nothing opens); a top slab lets it open. Screenshot 5.
  5. Open a box, close it and break it at once: no lid is left floating, and there is no second close sound. Screenshot 6.
  6. Watch from 10 blocks away while a box opens and shuts: no flicker, and never a second, closed lid inside the open one. Screenshot 7.
  7. Open and close a barrel: the sound comes from its front face. Then close the game window while a barrel is open, start again and say whether it still shows open. Screenshot 8.
  8. A single chest, a trapped chest, a copper chest and an ender chest swing their lids up about the back edge, slowing at the top; a double chest swings both halves together and plays one sound from its middle. Screenshot 9.
  9. Pour water on a chest so it is waterlogged and open it: the water stays while the lid moves. Screenshot 10.
- **Kit:**

```
IW4L_MINECRAFT_KIT=shulker_box,red_shulker_box*4,chest*2,trapped_chest,copper_chest,ender_chest,barrel*2,water_bucket,cobblestone*64,oak_slab*16,stone*32
```

## Group: body items and armour

Brief: `b3-held-armour.md`. The brief puts `body_gear` in `minecraft_survival.rs` and the resources in `frame`; both are off limits in this part of the track, so the pure parts live in `minecraft_terrain` (`body_gear.rs`, `body_render.rs`, `armor_render.rs`) and the MW2 side in `render_anim/src/anim/minecraft_gear.rs`, with the few lines in the upstream body files (`anim/remote_body.rs`, `anim/dobj_pose.rs`, `occupancy/remote_body.rs`). The glue is After the split items 18 onwards.

### 1. Minecraft players' worn and held items are read per player for the body

Commit: c72043b

- **Changed:** new `body_gear.rs`: `BodyGear { main, off, armor: [head, chest, legs, feet], hides_gun }` of `GearItem { id, glint, dye }`, from `body_gear(slots, selected, is_gun)`: the selected hotbar slot, the offhand (40) and the armour slots 39, 38, 37, 36 (`ARMOR_SLOTS`). An MW2 gun (`is_gun`, the glue passes `weapon_of(stack).is_some()`) is never drawn as a Minecraft item; `hides_gun` is true whenever the selection is not a gun, an empty hand included (vanilla draws an empty hand empty). `glint` is `glint::has_glint` (vanilla `hasFoil`), `dye` is `dyed_color(stack)`: the `minecraft:dyed_color` component (a bare RGB int as 26.3 writes it, or the older `{rgb}` form), `None` when undyed.
- **Tests:** `armour_is_read_head_to_feet_and_the_hands_from_the_selection_and_offhand`, `a_selected_gun_is_left_to_the_mw2_body`, `an_empty_hand_puts_the_gun_away_too`, `glint_and_dye_come_from_the_stack`.
- **Decision:** the gear is read from the server-owned inventory of each player (keyed by the player's UUID on the server, like the inventory itself); what the renderer keeps is a copy per MW2 client number (entry 4), filled from that.
- **Test steps:** nothing changes in game yet (After the split, item 18).
- **Kit:** none.

### 2. The Minecraft kit dyes leather armour

Commit: 2583171

- **Changed:** `crafting.rs` `kit_potion_stack`: an `@` entry on an item in the runtime `#minecraft:dyeable` tag (leather armour, leather horse armour, wolf armour) that names one of the 16 dyes gets `minecraft:dyed_color` with that dye's `textureDiffuseColor`, which is what one dye on undyed leather gives in vanilla (`DyedItemColor.applyDyes` with a single colour keeps it); every other `@` entry is still a potion. `special_models::dye_rgb` (the existing dye table, as an RGB int).
- **Tests:** `kit_dyes_dyeable_items_with_a_dye_name` (`leather_chestplate@red` is 0xB02E26, `leather_boots@minecraft:blue+unbreaking` is 0x3C44AA with its enchantment, `potion@red` stays a potion, plain leather stays undyed).
- **Decision:** the crafting grid has no armour dye recipe yet (`crafting_special_armordye`), so this is the only way to get dyed leather for the body test.
- **Test steps:**
  1. Start with the kit below and open the inventory: the five leather pieces are in the first five slots. Their icons all stay brown, because item icons still draw the undyed colour; the dye shows on the body once the gear is drawn (entry 6 and its glue).
- **Kit:**

```
IW4L_MINECRAFT_KIT=leather_helmet@red,leather_chestplate@blue,leather_leggings@green,leather_boots@black,leather_chestplate
```

### 3. MW2 bodies publish their limb and hand bones for Minecraft gear

Commit: f8573f5

- **Changed:** new `render_anim/src/anim/minecraft_gear.rs`: `GEAR_BONES` (19 names: `j_head`, `j_neck`, `j_spine4`, `j_spineupper`, `j_spinelower`, shoulder, elbow, wrist, hip, knee and ankle on each side, `tag_weapon_right`, `tag_weapon_left`; indices `HEAD`, `SHOULDERS[side]` and so on, right side first). `GearBones::resolve(dobj)` finds each by name and keeps its bind pose in body model space (`dobj.compose(&dobj.bind_locals(), IDENTITY)`, so a bone of the attached head model is in body space too); `missing()` names the ones the kit lacks. `GearBones::posed(world, placement)` gives each found joint's `PosedJoint { posed: placement * world[i] (map space), bind }`, and `PosedJoint::skin()` is `posed * bind^-1`, the same skinning delta the MW2 mesh uses (`DObj::skin_matrices`). `PosedGearFrame` (a resource, one `PosedGear { client, joints }` per posed body, keyed by MW2 client number) and `publish_posed_gear`.
- `anim/remote_body.rs`: `PreparedRemoteKit.gear_bones`, resolved in `PreparedRemoteKits::prepare` beside `head_bone`. `anim/dobj_pose.rs`: `begin_dobj_pose_frame` clears the gear frame with the others. `occupancy/remote_body.rs`: the resource is registered, `pose_one_remote` publishes the gear bones of every living body right after `publish_posed_player_head` (so live bodies, the F5 body at the predicted origin, skating bodies and the inventory puppet, but never corpses), and `prepare_remote_kits` logs `remote kits: axis=.. Minecraft gear bones missing: [..]` for both kits.
- **Tests:** `gear_bones_are_found_by_name_with_their_bind_pose`, `posed_joints_are_placed_and_skin_from_the_bind_pose` (a bent elbow carries a point below it round the elbow, the shoulder's point stays), `a_body_publishes_once_per_client_and_without_a_client_not_at_all`.
- **Decision:** the armour is placed from the bind pose and carried by each bone's skinning delta, not from assumed MW2 bone axes: whatever the joints' own axes are, a piece rides its bone exactly as the MW2 mesh around it does. Only the two hand tags are used with their own axes (IW weapon tags: X along the barrel, Y left, Z up, which the world gun attaches with).
- **Test steps:**
  1. Start the game and open `iw4l-artifacts/logs/latest.log`: find `Minecraft gear bones missing` and send the two lines (an empty list is the hoped-for answer).
- **Kit:** none.

### 4. The MW2 body puts its gun away while a Minecraft item is held

Commit: ba18264

- **Changed:** `minecraft_gear.rs`: `McBodyGear` (a resource: one `BodyGear` per MW2 client number, `set`, `remove`, `clear`, `get`) and `shown_weapon(client, corpse, weapon)`: 0 (no gun) for a living body whose gear `hides_gun`, the MW2 weapon otherwise. `occupancy/remote_body.rs`: the resource is registered, and both places that pick the body's kit by weapon use it, `occupy_remote_scene_ents` (the scene's model list) and `pose_one_remote` (the posed and skinned models), so the two always agree. A corpse keeps its gun as before.
- **Tests:** `a_minecraft_item_in_hand_puts_the_mw2_gun_away_but_not_on_a_corpse`.
- **Decision:** only the gun model goes; the MW2 arm animation still holds the pose of the last gun, which is the grip the items sit in (entry 5). A vanilla-like arm swing and use pose on the MW2 rig is later work (the brief defers it too).
- **Test steps** (after glue item 18):
  1. Select the cobblestone and press F5: the MW2 body holds no gun. Select a gun slot: the gun is back. Screenshot of each.
- **Kit:**

```
IW4L_MINECRAFT_KIT=cobblestone*64
```

### 5. Minecraft blocks and items sit in the MW2 body's hands in third person like vanilla

Commit: 9b69a6f

- **Changed:** `model.rs`: `item_third_person_transform(pack, item, left_hand)`: the `thirdperson_righthand` or `thirdperson_lefthand` display (the left one falls back to the right one and is mirrored as `ItemTransform.apply(leftHand)` does: rotation y and z and translation x negated). `mesh.rs`: `HeldDisplay::LeftHand` and `ItemVisual.left_hand`. New `body_render.rs`: `BodyRig { hands: [right, left], light }` (each hand a block-space frame, x right, y forward, z up, in blocks, as vanilla's after `ItemInHandLayer.submitArmWithItem`'s turns and its step out to the hand), `held_items(rig, gear)` (the main hand in the right hand with `THIRD_PERSON_RIGHT_HAND`, the offhand in the left with `THIRD_PERSON_LEFT_HAND`, as for a right-handed vanilla player) and `append_body_gear(mesh, items, rig, gear, packs, atlas, light)`, which draws them through `ItemVisuals::append_held_items` (the mobs' path: item definition tints, item shading, lit at the body) and marks an enchanted one's vertices in `mesh.glint`.
- `minecraft_gear.rs`: `GEAR_FIT` (the one tuning table: `item_scale` 0.9375, vanilla's player scale; `hand_offset` (0, 1, -1) px right, forward, up), `map_to_block(origin)` (map units to blocks, as `sim::voxel::to_block`), `hand_frame(joints, side, fit)`: the hand tag (`tag_weapon_right`/`_left`) turned by Rz(-90 degrees), which takes MW2's tag axes (X along the barrel, Y left, Z up) to vanilla's hand frame (x right, y forward, z up), scaled from blocks to map units, then the offset (mirrored in x for the left hand); without a tag, the wrist's bind position turned the same way and carried by the wrist's skinning delta. `body_rig(joints, origin, fit)` (lit at `j_spine4`), `gear_rigs` (only bodies whose `McBodyGear` is not bare; the inventory puppet's apart) and `gear_meshes(posed, gear, origin, puppet, items, packs, atlas, light) -> GearMeshes { world, puppet }`.
- **Tests:** model: `third_person_hands_take_their_display_and_the_left_mirrors_the_right`. body_render: `the_main_hand_holds_its_item_in_the_right_hand_and_the_offhand_in_the_left`, `held_items_take_their_hand_display_and_only_enchanted_ones_glint` (a generated item from a test pack: the two hands' copies sit half a block apart from the mirrored display, only the enchanted one glints). minecraft_gear: `a_hand_tag_turns_into_the_vanilla_hand_frame_in_blocks` (an identity tag at map (36, 0, 72) lands at origin + (1, 2, 0); forward is block +x, up +y, right +z), `the_hand_offset_is_in_pixels_and_mirrored_for_the_left_hand`, `without_a_hand_tag_the_hand_rides_the_wrist_upright_from_the_bind_pose`, `only_bodies_with_gear_get_a_rig_and_the_puppet_is_kept_apart`.
- **Decisions:** the item sits in the MW2 hand as a vanilla player holds it with the arm down (forward along the gun's barrel, up along the gun's top), not turned with the forearm: MW2's arms stay in the gun grip, and turning the vanilla frame with a forearm that points forward would stand a sword straight up. Not done here: a held potion's contents tint (`first_tint`), the trident's 3D in-hand model (the item definition's `display_context` select; `trident::MODEL` is ready), the spyglass at the eye and the vanilla arm swing on the MW2 rig.
- **Test steps** (after glue items 18 and 19):
  1. Select the cobblestone and press F5: the block is in the right hand, small and turned like a vanilla player's, and there is no gun. Press F5 again for the front view. Screenshot 1 (back) and 2 (front).
  2. Put the torch in the offhand (F): it is in the left hand. Select the sword: it is in the right hand and shimmers. Screenshot 3.
  3. Select the lever: a flat item in the hand. Select a gun slot: the gun is back, the right hand is empty and the torch stays in the left hand. Screenshot 4.
  4. Walk, crouch and go prone with the cobblestone: it stays in the hand. If an item floats off the hand or is turned wrongly, the screenshot says by how much, and `GEAR_FIT` is tuned from it.
- **Kit:**

```
IW4L_MINECRAFT_KIT=cobblestone*64,torch*64,diamond_sword+sharpness=5,lever*8
```

### 6. Minecraft armour is worn on the MW2 body like vanilla

Commit: 10c3a35

- **Changed:** `armor_render.rs`: `ArmorRig { head, body, arms: [right, left], legs: [right, left] }`, each a block-space frame of the vanilla part (pixels from the part's pivot, x left, y down, z back as `HumanoidModel`), the torso and limbs as a `SplitPart { upper, lower, split }`: the part's cube above `split` (in the part's pixels) rides `upper`, below it `lower`. `cube_posed_slice(mesh, part, panel, from, size, grow, uv, (y0, y1), mirror)`: a `ModelPart.Cube` (the same box UV, polygon order and mirror as `cow_render::cube_scaled`, inflated by `grow` over the uninflated UV) cut to a y range, its four sides' V cropped to the cut, the top only on the slice that holds it and the bottom likewise, lit with `entity_shade` of its posed normal, and given the entity sheet alpha so the section shader samples it unmipped like the mob textures. `append_armor_on_rig(mesh, atlas, rig, armor, sky, block)`: `HumanoidArmorLayer` in its order (chest, legs, feet, head), each piece only in its own slot, with the existing slot rules (helmet and hat layer on the head; chestplate on the body and both arms, the left one mirrored; leggings from `humanoid_leggings` on the body and legs inflated 0.5; boots on the legs; legs 0.1 thinner) and `armor_layers`: the equipment textures of `EquipmentLayerRenderer` (leather's dyeable layer tinted with the item's `dyed_color`, else `color_when_undyed`, then its untinted overlay). `body_render.rs`: `BodyRig.armor`, drawn by `append_body_gear` lit at the body.
- `minecraft_gear.rs`: `part_frame(origin, down, scale, anchor)`: a vanilla part frame in bind space (vanilla y along `down`, vanilla z backwards against the model's +X, x completing it, each axis scaled in map units per pixel, `anchor` the vanilla point placed at `origin`). `bound_armor(joints, fit)` builds each part in the bind pose: the helmet a `head`-sized cube centred `head_offset` from `j_head`; the torso from the shoulders' midpoint (raised `torso_top`) to the hips' midpoint, `torso_width` by `torso_depth`, its upper half riding `j_spine4` and its lower half `j_spinelower` (`j_spineupper` when one is missing); each arm a chain shoulder to elbow to wrist and each leg hip to knee to the sole (`sole` below the ankle along the shin), the vanilla limb's 10 or 12 pixels spread over the chain so the split falls exactly on the elbow or knee, the upper slice riding the shoulder or hip, the lower the elbow or knee, `arm_girth` or `leg_girth` across. `BoundArmor::posed(to_block)` carries every frame by its rider's skinning delta into blocks, so the pieces bend at the elbows and knees and follow the spine. All sizes are in `GEAR_FIT` (head 9, offset (0.5, 0, 3.5), torso 13 by 8, top 3.5, arms 4, legs 5.5, sole 4, map units).
- **Tests:** armor_render: `a_cube_cut_in_two_keeps_its_sides_and_its_texture_panels` (the whole cube is 6 faces, two halves 5 each, their V meet at the cut), `a_posed_slice_follows_its_frame_and_faces_outward` (plain and mirrored), `each_piece_is_drawn_on_its_parts_in_the_vanilla_order` (a helmet is 12 faces on the head only, a helmet in the chest slot draws nothing, iron in the order chest, legs (leggings texture), feet, head), `leather_is_dyed_under_its_undyed_overlay` (dyed and undyed). body_render: `worn_armour_is_drawn_with_the_body_and_lit_where_it_is`. minecraft_gear: `a_part_frame_turns_vanilla_left_down_back_into_the_model_axes`, `limbs_run_from_joint_to_joint_and_split_at_the_elbow_and_knee`, `a_bent_elbow_turns_only_the_lower_slice`, `missing_joints_leave_their_part_out`.
- **Decisions:** Minecraft's box proportions are kept but sized to the MW2 joints, not drawn at Minecraft's own size: a vanilla-sized helmet on an MW2 head would be twice too big. The leggings' belt rides the lower torso like the chestplate's (vanilla draws both on the body part). Trims, carved pumpkins and heads (`CustomHeadLayer`), the elytra and armour on corpses (only with `keep_inventory`) are later work, as in the brief.
- **Test steps** (after glue items 18 and 19):
  1. Wear the diamond helmet, iron chestplate, red leather leggings and iron boots (shift-click them in the inventory, or right-click with one in hand). Press F5 for the back view: boxy Minecraft armour sits on the head, torso and both arms, hips and legs, and the boots on the feet. Press F5 again for the front view. Screenshots 1 and 2.
  2. Walk, crouch, go prone and look up and down: the sleeves bend at the elbows and the legs at the knees; the chest follows the spine. Screenshot 3 while crouched.
  3. Swap in the golden, leather (red), chainmail and turtle helmets, the netherite, copper and blue leather chestplates, the chainmail leggings and the diamond boots, and check each texture. Screenshot 4.
  4. If a piece floats off the body, sinks into it or is too big or too small, say which and by how much: the sizes are tuned in `GEAR_FIT`.
- **Kit:**

```
IW4L_MINECRAFT_KIT=diamond_helmet,iron_chestplate,leather_leggings@red,iron_boots,golden_helmet,leather_helmet@red,chainmail_helmet,turtle_helmet,netherite_chestplate,copper_chestplate,leather_chestplate@blue,chainmail_leggings,diamond_boots,leather_boots
```

### 7. Enchanted Minecraft armour shimmers on the MW2 body like vanilla

Commit: a1b3259

- **Changed:** `mesh.rs`: `ChunkMesh.armor_glint` (vertex ranges) and `ARMOR_GLINT_VERTEX` (128); `section_vertices` writes it into the same flag byte as `GLINT_VERTEX` (255). `armor_render.rs`: an enchanted piece (`GearItem.glint`, vanilla `hasFoil`) marks only its first layer, as `EquipmentLayerRenderer.renderLayers` gives `armorCutoutNoCullGlint` to the first layer only (a leather piece's dyed layer, not its overlay). `glint.rs`: `ARMOR_GLINT_TEXTURE` and `armor_glint_texture(packs)` (`misc/enchanted_glint_armor`, 128 by 128, a different image from the item glint), sharing the loader with `glint_texture`.
- `render_gpu` `minecraft_world.rs`: `MinecraftWorldFrame.armor_glint_texture`, uploaded like the item glint and bound at binding 7 (the item glint stands in until a frame brings the armour one). `glint_light` reads the flag: at 1 the item glint as before (`GLINT_TEXTURING`, 8 per 512 atlas pixels), below 0.75 the armour glint with `ARMOR_ENTITY_GLINT_TEXTURING`'s scale (0.16 of a 64-wide armour texture, 0.16/64 per atlas pixel; the 64 by 32 armour textures sit in a 64 by 64 atlas cell, which makes their u and v steps equal), the same scroll, turn and strength (`TextureTransform.setupGlintTexturing`, 26.3 javap).
- **Tests:** mesh: `section_vertices_mark_armour_glint_apart_from_item_glint`. armor_render: `an_enchanted_piece_shimmers_on_its_first_layer_only`. body_render: the worn helmet's glint. glint: `the_armour_glint_comes_from_its_own_texture` (and the local pinned-pack test checks the vanilla one is 128 square and not the item glint). render_gpu: `the_glint_flags_the_meshes_write_pick_the_item_and_armour_glints` (binding 7 exists; 255 picks the item glint, 128 the armour one); `terrain_shader_validates` covers the new WGSL.
- **Decision:** the armour glint goes through the section vertex flag byte the item glint already uses, so the gear needs no new vertex format or pipeline.
- **Test steps** (after glue items 18 to 20):
  1. Wear the enchanted diamond helmet and iron chestplate and the plain leggings and boots. F5: the helmet and chestplate shimmer with broad purple streaks; the leggings and boots do not. Screenshot.
  2. Wear the enchanted leather chestplate: the brown leather shimmers, its overlay (the darker trim) does not. Screenshot.
- **Kit:**

```
IW4L_MINECRAFT_KIT=diamond_helmet+protection=4,iron_chestplate+protection=4,iron_leggings,iron_boots,leather_chestplate+unbreaking=3
```

### 8. The inventory's character wears its Minecraft armour and holds its items like vanilla

Commit: 4eeadc6

- **Changed:** `render_gpu` `minecraft_world.rs`: `MinecraftWorldFrame.puppet_gear` (section vertex bytes and indices in blocks, like the particles), uploaded each frame and drawn with the opaque section pipeline in the view model's depth band right after the inventory's black card, so the character box's MW2 body (drawn in that band, `DEPTH_HACK_SCENE_ENTNUM`) and its gear hide each other correctly and no wall comes between. The character's gear is already split off by `gear_meshes` (entry 5: the body whose client is the active `InventoryPuppet`'s goes to `GearMeshes.puppet`), and the puppet's bones carry its root transform, so the armour and items take its place, turn and scale.
- **Tests:** none new here: the routing is `only_bodies_with_gear_get_a_rig_and_the_puppet_is_kept_apart` (entry 5) and the frames follow the posed bones whatever their placement (entries 3, 5 and 6); the draw itself needs a GPU.
- **Decision:** a separate buffer in the band, as the brief planned, rather than drawing the character's gear with the world: in the world's depth range the card (in the band) would cover it.
- **Test steps** (after glue items 18, 19 and 21):
  1. Wear the armour from entry 6, hold the cobblestone and put the torch in the offhand. Open the inventory (E): the character in the box wears the same armour and holds the cobblestone and the torch. Move the mouse over the box: the gear turns with the character. Screenshot.
  2. Close the inventory and press F5: the gear is back on the body in the world, not left floating in front of the camera. Screenshot.
- **Kit:**

```
IW4L_MINECRAFT_KIT=diamond_helmet+protection=4,iron_chestplate,leather_leggings@red,iron_boots,cobblestone*64,torch*64
```

### 9. Minecraft body gear code passes clippy

Commit: 2bc09cf

- **Changed:** `anim/remote_body.rs` takes `GearBones::NONE` (new, the gear bones of a kit without a DObj; `GearBones::default()` is it) instead of `unwrap_or_default()`, which the workspace's `clippy.toml` disallows. Five test asserts on a single vertex range no longer build a one-element array of ranges (`single_range_in_vec_init`).
- **Tests:** unchanged; `clippy -p minecraft_terrain`, `-p render_anim` and `-p render_gpu` (`--all-targets`, with `-A clippy::approx_constant` for the 16 deny-level `approx_constant` errors `minecraft_terrain` already had) report nothing in this group's code; the warnings left in the files it touched (`occupancy/remote_body.rs`'s argument counts and complex types, `mesh.rs` and `model.rs`) were there before.
- **Test steps:** none.
- **Kit:** none.

### Group summary: body items and armour

- Built and tested: the per-player gear read from the inventory (entry 1), dyed leather in the kit (entry 2), the MW2 body's gear bones published every frame with their bind pose (entry 3), the gun put away while a Minecraft item is held (entry 4), held items and blocks in both hands (entry 5), the armour pieces bending with the MW2 joints (entry 6), the armour glint (entry 7) and the inventory character's gear (entry 8). Everything lives in `minecraft_terrain` (`body_gear.rs`, `body_render.rs`, `armor_render.rs`, `glint.rs`, `mesh.rs`, `model.rs`), `render_anim/src/anim/minecraft_gear.rs` (bones, `McBodyGear`, `GEAR_FIT`, rigs, `gear_meshes`), a few lines in the upstream body files and `render_gpu`'s Minecraft world pass.
- Nothing changes in game until After the split items 18 (each player's gear into `McBodyGear`), 19 (`draw_body_gear` appends the world gear to `view.particles`), 20 (the armour glint texture through the extract) and 21 (the character box gear through the extract) are in. Item 20 is optional for a first look: without it the armour shimmers with the item glint's image.
- **Screenshot 2 ("items at the head"):** confirmed from the code. Nothing draws a held Minecraft item on the body before this group (the only held-item mesh is the first-person one, built in view space and skipped in F5, in the inventory and while dead), so the items at head height are the death drops: `death_drops` spawns every stack with `WorldItems::drop_on_death(stack, death.spot.eye)` (`minecraft_world.rs:1817-1818`), which is vanilla's `Player.drop(stack, true, false)`: the item appears 0.3 below the eyes and flies up 0.2 and up to 0.5 sideways (`third_party/minecraftoss/player/src/items.rs:254-269`). The client draws its copy frozen where it spawned until the server has taken it over (`Entities::server_items_tick`, `minecraft_entities.rs:356-375`), so a slow server tick right after a death keeps them hanging at the head a little longer; that delay belongs to the death-at-height work, not to this track. Death-cam step 5 below checks it.
- **Not done** (and why): the trident's 3D in-hand model and the spyglass's in-hand model (both need the item definition's `display_context` select per context, and `ItemVisuals` caches one model per item; `trident::MODEL` is ready), a held potion's contents colour (the gear carries no tint yet), the vanilla arm swing and use poses on the MW2 rig, armour trims, carved pumpkins and mob heads on the head (`CustomHeadLayer`), the elytra, gear on corpses (only with `keep_inventory`), dyed colours on item icons and the crafting grid's armour dye recipe.
- **Test steps** (after glue items 18 to 21; step 5 works today):
  1. Wear the enchanted diamond helmet, enchanted iron chestplate, red leather leggings and iron boots (shift-click them in the inventory). Press F5 for the back view: Minecraft armour on the head, torso and arms, hips and legs, and boots on the feet; the helmet and chestplate shimmer purple, the leggings are red with the darker trim. Press F5 again for the front view. Walk, crouch, go prone and look up and down: the pieces bend at the elbows and knees. Screenshots 1 and 2.
  2. Select the cobblestone and put the torch in the offhand (F). In F5 the cobblestone is in the right hand, the torch in the left, and there is no MW2 gun. Select the sword: it shimmers. Screenshot 3.
  3. Select an MW2 gun slot: the gun is back, the right hand is empty, the torch stays in the left hand. Screenshot 4.
  4. Open the inventory (E): the character in the box wears the same armour and holds the items. Screenshot 5.
  5. Death cam: press Tab and note whether a bot is in the match. Hold the cobblestone and type `/kill`. The drops fly out of your eye height and land within about a second; no standing copy of your soldier appears. If a standing soldier appears and Tab shows no other player, say so. Screenshot 6.
  6. Swap in the golden, red leather, chainmail and turtle helmets, the netherite, copper and blue leather chestplates, the chainmail leggings and the diamond boots, and check each texture. Screenshot 7.
  7. Open `iw4l-artifacts/logs/latest.log` and send the two `Minecraft gear bones missing` lines.
  8. For anything floating off the body, sunk into it, too big, too small or turned wrongly, say which piece or item and by about how much: every size and offset is in `GEAR_FIT` (`render_anim/src/anim/minecraft_gear.rs`).
- **Kit:**

```
IW4L_MINECRAFT_KIT=diamond_helmet+protection=4,iron_chestplate+protection=4,leather_leggings@red,iron_boots,golden_helmet,leather_helmet@red,chainmail_helmet,turtle_helmet,netherite_chestplate,copper_chestplate,leather_chestplate@blue+unbreaking=3,chainmail_leggings,diamond_boots+feather_falling=4,leather_boots,cobblestone*64,torch*64,diamond_sword+sharpness=5,lever*8,shield
```

## Review fixes

Defects found by the adversarial review of the whole track (base `071dada`), each fixed in its own commit.

### 1. A creative Minecraft trident that comes back with Loyalty adds no second trident

Commit: f0d63f0

- **Defect:** `ThrownTrident::taken_by` gave the stack back for a `CreativeOnly` trident on its way back (`no_physics`), so a Loyalty trident thrown in creative (which leaves the original in the hand) put a second trident in the inventory when it returned. Vanilla (`AbstractArrow.tryPickup`, 26.3 javap): `CREATIVE_ONLY` returns `player.hasInfiniteMaterials()` first, so the creative thrower takes it with nothing added; only the `ThrownTrident.tryPickup` fallback (`isNoPhysics && ownedBy && inventory.add`) could add it, and it is never reached by a creative player.
- **Changed:** `trident.rs` `taken_by`: a `CreativeOnly` trident is always removed with nothing given.
- **Tests:** trident: `a_creative_loyal_trident_coming_back_is_taken_without_a_copy`.
- **Decision:** the server does not know the touching player's game mode, so a `CreativeOnly` trident touched by its owner after switching to survival is also removed with nothing given (vanilla would give it back while it returns, and leave it while it is stuck); a creative throw is the case that matters.
- **Test steps** (after glue items 9 and 10):
  1. `/gamemode creative`, hold the Loyalty trident, charge and throw it at the ground: the trident stays in your hand, the thrown copy comes back and vanishes on you. Open the inventory: still exactly one Loyalty trident. Screenshot.
- **Kit:**

```
IW4L_MINECRAFT_KIT=trident+loyalty=3
```

### 2. Anyone picks up a Minecraft trident whose thrower is not in the world, like vanilla

Commit: 08a22bf

- **Defect:** a trident whose thrower was not in the level (a loaded trident before its thrower joins, or one whose thrower left) could be picked up by nobody, and saving a trident with no known thrower wrote a made-up `Owner` UUID (the fallback UUID of an unknown entity ID, different every session). Vanilla (`ThrownTrident.playerTouch`, 26.3 javap): `ownedBy(player) || getOwner() == null`, so while the owner entity is not in the level anyone may take it; an entity with no owner saves no `Owner`.
- **Changed:** `trident.rs`: `taken_by(player, owner_here)` refuses others only while the thrower is in the level; `to_tag(owner: Option<u128>)` writes `Owner` only when known. `server.rs`: `pickup_tridents(player, players, take)` asks whether the thrower is among the level's players; a trident saves with its own `owner_uuid`.
- **Tests:** trident: `a_settled_trident_is_its_throwers_while_the_thrower_is_here` (was `only_its_thrower_picks_it_up_once_it_has_settled`; anyone takes it while its thrower is away), `a_stuck_trident_saves_as_the_vanilla_entity_and_loads_back` (an ownerless trident saves no `Owner`). server: `a_saved_trident_comes_back_and_still_knows_its_thrower` (a stranger is offered it while the thrower is away, not once the thrower is back), `a_trident_hits_a_cow_for_8_sticks_and_only_its_thrower_takes_it_back` (with the thrower present).
- **Decision:** a Loyalty trident still waits for its thrower and does not come back while the thrower is away (vanilla: `getOwner()` is null, so no return).
- **Found, not fixed here:** nothing calls `EntityWorld::set_player_uuid`, so a thrown trident's saved `Owner` is the fallback UUID of player 0, not the player's UUID from `players/data/<uuid>.dat`. It is stable between sessions, so single player works; the glue (After the split, item 22) has to name the real UUID.
- **Test steps:** single player cannot show the difference (the thrower is always in the world). With two players later: player A throws a plain trident into the ground and leaves; player B walks over it and takes it.
- **Kit:**

```
IW4L_MINECRAFT_KIT=trident
```

### 3. Mobs killed by a player's Minecraft trident or thrown item count as the player's kill

Commit: 1720ab8

- **Defect:** `thrown::hurt_by` (tridents for 8, wind charges for 1, snowballs, eggs and pearls for 0) hurt the mob and turned it on the thrower but never named the player as the damage's source, so a cow or zombie killed by a thrown trident died as if nobody killed it: no experience orbs and no player-only loot. Vanilla: the damage source of a thrown trident or item names its owner (`damageSources().trident(this, owner)`, `thrown(this, owner)`), and `LivingEntity.hurtServer` remembers the player (`resolvePlayerResponsibleForDamage`), even for a 0-damage snowball.
- **Changed:** `thrown.rs` `hurt_by` credits every hit to `minecraft:player` (`EntityWorld::credit`, the same call the player's melee, blasts and splash potions use). Vendored `minecraftoss-entities`: `EntityWorld::credit` is public.
- **Tests:** thrown: `a_mob_killed_by_a_thrown_trident_is_the_throwers_kill` (killed by the player, attacker `minecraft:player`, experience dropped).
- **Test steps** (after glue items 9 and 10):
  1. Spawn a cow and throw the trident at it twice (8 each): when it dies, green experience orbs come out with the beef and leather. Screenshot.
- **Kit:**

```
IW4L_MINECRAFT_KIT=trident,cow_spawn_egg*2
```

### 4. A Minecraft double chest half still drawn by its section stays shut

Commit: 00072ad

- **Defect:** `Lids::posed` gave each half of a double chest the wider-open half's progress even when that half had not left its section yet (its cell waiting for the rebuild, `Sending(true)`/`Rebuilding(true)`, or already handed back). Its section still drew the closed lid there, so for a frame or more the frame pass drew a moving lid over a closed one: the two-lid look the lids group set out to avoid (lids test step 6). It happens when the two halves' cells sit in different sections (a chest across a section border), or when one half shuts a tick before the other.
- **Changed:** `lids.rs` `posed`: only a half drawn apart takes the other half's progress; a half its section still draws keeps its own (0).
- **Tests:** lids: `both_halves_of_a_double_chest_swing_as_the_wider_open_one` now opens the second half before its section is rebuilt and checks it stays shut until it is apart, then swings with the first.
- **Test steps** (after glue items 15 to 17):
  1. Place a double chest across a chunk border (stand on the border shown by F3+G, one half on each side). Open and close it several times from 3 blocks away: both halves swing together and no closed lid shows inside an open one. Screenshot mid-swing.
- **Kit:**

```
IW4L_MINECRAFT_KIT=chest*2
```

### 5. Review notes: found and left open

Commit: af7a188

- **A trident waiting in a chunk that was never loaded is lost when the world closes.** Entry 11 keeps a trident that flew or came back into an unloaded chunk in memory, but `ServerSim::save_all_entities` (autosave and shutdown) only writes the loaded chunks' entities, so quitting while it waits there drops it and the item in it. Vanilla saves it: the entity section of the unloaded chunk is queued for loading (`PersistentEntitySectionManager.ensureChunkQueuedForLoad`) and written with that chunk's other entities. It is not fixed here because writing the trident alone into that chunk's entity file would replace the entities already saved there, or the generation mobs of a chunk never saved (`load_chunk` only uses `chunk.generation.entities` when no entity file exists); the fix needs the entity storage to merge into a chunk it has not loaded, which belongs with the per-level storage of the crate split. Until then it only happens to a trident thrown beyond the loaded area.
- **The server never learns the player's real UUID** (review fix 2, After the split item 22).
- **Checked and found right** (26.3 javap where it matters): projectile entity hits stop at the clip point (`EntityHitResult(entity, location)`), so a wind charge bursts where it struck a mob while a lingering cloud forms at the struck mob's position (`ThrownLingeringPotion.onHitAsPotion`); splash potions have no direct-hit bonus (`ThrownSplashPotion.onHitAsPotion`); shulker and chest lid timings, opener sounds and positions, potion table, colours and duration scaling, the trident's flight, deflection, Loyalty and despawn, the armour draw order, slot parts, inflation and first-layer glint.
- **Test steps:** none.
- **Kit:** none.

## Final check

### Track B log gets its final checks and one kit for the whole track

Commit: (next entry)

- **Changed:** this log only: review note 5's hash, and the kit below.
- **Clippy:** `clippy -p minecraft_terrain -p render_anim -p render_gpu -p minecraftoss-entities -p minecraftoss-world --locked --all-targets -- -A clippy::approx_constant` reports no warning on any line the track added or changed (`git diff -U0 071dada..HEAD`). The warnings left in the files it touched were there before (`occupancy/remote_body.rs`'s argument counts and complex types, `anim/remote_body.rs`, `mesh.rs`, `model.rs`, `server.rs`, `server_mobs.rs`, `block_particles.rs`), and the two vendored crates report none.
- **Tests:** `minecraft_terrain` 572 passed (3 ignored), `render_anim` 165, `render_gpu` 27, `minecraftoss-entities` 64, `minecraftoss-world` 55; none failed.
- **Test steps:** none new. Each entry above has its own steps, and the kit below covers most of them in one world. One shulker box does for the floor, wall and ceiling steps (break it and place it again), and cobblestone can be dropped where wind charges step 4 says dirt. Not covered, use those entries' own kits:
  - Throwables: the tipped arrows (entry 1), eggs (entry 2), the witch and the enderman (entries 4 and 6, or `/summon minecraft:witch` and `/summon minecraft:enderman`), the second and third Harming, Healing II, Slowness and the spider (entry 5, steps 1 to 3), Lingering Poison (entry 7, step 2).
  - Lids: the trapped, copper and ender chests and the waterlogged chest (group steps 8 and 9).
  - Body: the other leather pieces (entry 2), the lever (entry 5), the other armour textures (entry 6, step 3, and group step 6), the enchanted leather chestplate (entry 7, step 2).
- **Kit:** one kit for the whole track, 27 stacks (potions, tridents and the shulker box take one slot each):

```
IW4L_MINECRAFT_KIT=minecraft:experience_bottle*64,minecraft:snowball*16,minecraft:ender_pearl*16,minecraft:wind_charge*64,minecraft:splash_potion@harming,minecraft:splash_potion@poison,minecraft:splash_potion@water,minecraft:lingering_potion@harming,minecraft:trident,minecraft:trident+loyalty=3,minecraft:cow_spawn_egg*16,minecraft:zombie_spawn_egg*4,minecraft:flint_and_steel,minecraft:netherrack*16,minecraft:campfire,minecraft:candle*4,minecraft:red_shulker_box,minecraft:chest*4,minecraft:barrel,minecraft:oak_slab*16,minecraft:diamond_helmet+protection=4,minecraft:iron_chestplate+protection=4,minecraft:leather_leggings@red,minecraft:iron_boots,minecraft:cobblestone*64,minecraft:torch*64,minecraft:diamond_sword+sharpness=5
```

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
17. **Opening and closing drive the openers, the lids and the sounds** (`minecraft_world.rs`; with several players, the server):
    - The world (per level, beside `containers`) gets `openers: minecraft_terrain::openers::Openers`; the local player's UUID is `minecraftoss_entities::gossip::parse_uuid(host)` of the save's host UUID (`minecraft_save.rs:93`).
    - The loop at :1769-1795 (`for (opened, open) in [...]`): for each `pos` of `opened.cells` whose block (`Scene::block(&world.scene, pos)`) is `Openers::counted`, `let event = if open { world.openers.open(pos, uuid, spectator) } else { world.openers.close(pos, uuid) }` (`spectator`: `player_mode.mode == GameMode::Spectator`). For each event: `let (at, id, count) = event.block_event(); entities.lids.block_event(&world.scene, at, id, count);` and, when `event.change` is `Some(change)`, play `openers::sound(&block, pos, change)` at its `position` and `volume` with pitch `openers::pitch(entities.random_float())`, and `write_blocks` the `openers::opened_state(&block, change)` (the barrel's `open`) as the loop does today. This replaces today's sound and barrel code; ovens and brewing stands keep no openers and no sound, as now.
    - Where a counted block is broken or replaced (the paths that `spill` or `take_shulker`, :1534 and :1563, and the server's block changes), call `world.openers.removed(pos)` before the inventory screen closes, so breaking an open box plays no close sound.
    - Every `openers::RECHECK_TICKS` server ticks: `world.openers.recheck(|player, pos| player == uuid && inventory_ui.block().is_some_and(|o| o.contains(pos)))`; on death, a world close or a dimension change: `world.openers.leave(uuid)`; feed their events as above.
    - :1079-1082, the shulker box's check: `entities.lids.lid_closed(opened.cells[0]) && lid_obstructed(..)` (`ShulkerBoxBlock.canOpen` checks the space only while the lid is `CLOSED`).
    - Several players (roadmap 1.4): `Openers` stays per level on the server; each `OpenerEvent::block_event()` goes to every client near `pos` as a `minecraft_net` `BlockEvent { pos, id: 1, param: count }`, which feeds that client's `Lids::block_event`; the server plays the sound for everyone near.
    - Once items 15 to 17 are in, update `docs/MINECRAFT-SURVIVAL.md:187` (it says no block entity animates).
18. **Each player's body gear, every frame** (`minecraft_world.rs` `update`, after `inventory_ui.publish` at about :1827; with several players, the server for each player and then the equipment messages): add `mut body_gear: ResMut<crate::anim::minecraft_gear::McBodyGear>` to `update` (or to one of its parameter tuples), and each frame `body_gear.set(local.0.0, minecraft_terrain::body_gear::body_gear(&entities.inventory.slots, entities.selected, |stack| crate::minecraft_inventory::weapon_of(stack).is_some()))` while `alive && player_mode.interacts()`, else `body_gear.remove(local.0.0)`; `body_gear.clear()` where the world closes (`close_world`) and while no Minecraft world is loaded. Several players: the host keeps each player's `BodyGear` with its inventory (by UUID), sends it on change as a `minecraft_net` equipment message (as vanilla's `ClientboundSetEquipmentPacket`, slot and stack), and each client `set`s it under that player's MW2 client number. The gun comes back one frame late at most, because `update` (`ClientSet::Present`) is not ordered before `occupy_remote_scene_ents` (`render_scene::GfxSceneAdd`); ordering `update.before(render_scene::GfxSceneAdd)` would remove that frame if it makes no cycle.
19. **The body gear drawn every frame** (`minecraft_world.rs`): `Runtime` gets `gear_items: minecraft_terrain::mesh::ItemVisuals` (cleared with the world in `close_world`). A new system `draw_body_gear(mut view: ResMut<MinecraftWorldView>, mut runtime: NonSendMut<Runtime>, posed: Res<crate::anim::minecraft_gear::PosedGearFrame>, gear: Res<crate::anim::minecraft_gear::McBodyGear>, puppet: Res<frame::InventoryPuppet>)`, added in `register` to `Update` with `.after(update).after(frame::WorkerCmdSet::SkinModel)` (the bodies are posed in `SkinModel`, and `update` rebuilds `view.particles`). It takes `let Runtime { world: Some(world), light: Some(light), gear_items, .. } = &mut *runtime else { return };`, calls `crate::anim::minecraft_gear::gear_meshes(&posed, &gear, view.origin, puppet.active.then_some(puppet.client), gear_items, &world.packs, &world.atlas, light)` and appends `minecraft_terrain::mesh::section_vertices(&meshes.world)` and its indices to `view.particles` exactly as :1929-1933 append the items mesh (indices offset by the vertices already there). `meshes.puppet` goes to item 21. Once items 18 and 19 are in, update `docs/MINECRAFT-SURVIVAL.md:194` (the body no longer holds the last gun while a Minecraft item is selected).
20. **The armour glint texture** (`minecraft_world.rs` and `crates/render/src/extract.rs`): `Loaded` gets `armor_glint_texture: Option<Arc<image::RgbaImage>>`, loaded beside the item glint (:304) with `minecraft_terrain::glint::armor_glint_texture(&packs).ok().flatten().map(Arc::new)`; `MinecraftWorldView` gets `pub armor_glint_texture: Option<Arc<image::RgbaImage>>`, set beside `view.glint_texture` (:465) and cleared beside it (:2513). In `extract_minecraft_world` (`render/src/extract.rs:937`) add `mut armor_glint_of: Local<Option<(usize, std::sync::Arc<render_gpu::MinecraftAtlasImage>)>>` (the system then has 8 parameters) and set `frame.armor_glint_texture` from `view.armor_glint_texture` exactly as `frame.glint_texture` is set from `view.glint_texture` (:1012-1025). Until then the armour shimmers with the item glint's image at the armour's scale.
21. **The inventory character's gear** (`minecraft_world.rs` and `crates/render/src/extract.rs`): `MinecraftWorldView` gets `pub puppet_gear: (Vec<u8>, Vec<u32>)`; item 19's `draw_body_gear` sets it to `(bytemuck::cast_slice(&section_vertices(&meshes.puppet)).to_vec(), meshes.puppet.indices)` every frame (empty when the inventory is shut, and cleared with the other meshes at :2514). `extract_minecraft_world` adds `frame.puppet_gear = std::mem::take(&mut view.puppet_gear);` beside `frame.backdrop = view.backdrop;` (:994).
22. **The server knows each player's real UUID** (`minecraft_terrain/src/server.rs` and `minecraft_world.rs`; review fix 2): nothing calls `EntityWorld::set_player_uuid` today, so `uuid_of(PLAYER_TARGET + 0)` is a made-up fallback UUID and a thrown trident saves that as its `Owner`. Add `Command::PlayerUuid { player: u64, uuid: u128 }` with `ServerHandle::set_player_uuid(player, uuid)`, handled in `server_loop` as `sim.mobs.set_player_uuid(player, uuid)`, and send it when the world opens with `PLAYER` and `minecraftoss_entities::gossip::parse_uuid` of the save's host UUID (`minecraft_save.rs:93`, the same UUID item 17 gives the openers). Several players: the host sends it for each player as it joins. Tridents saved before this keep the old fallback `Owner`; with the vanilla rule of review fix 2 anyone can still pick those up.

## Group: throw glue

The second half of the track wires the throwables into the game (After the split items 1 to 14, without item 3, which is track C's). The game state moved with the crate split: `Entities` is `minecraft_survival/src/entities.rs`, the player's hand timers are `minecraft_survival/src/player.rs` `Hand`, and the adapter is `render_anim/src/minecraft_world.rs`.

### 1. Minecraft throws carry their thrower and the stack they came from

Commit: 4749db4

- **Changed:** `entities.rs` `Entities::throw` takes the thrown stack and launches it with `Thrown::launch` (`Throw { owner: PLAYER, item, components, .. }`), so a potion carries its own contents and every throw names its thrower. `minecraft_world.rs` passes the stack. `server.rs`: the hand bridge (`ServerSim::note_held`, its `held` map and the `TickInput.pickup` call to it) is gone, since every throw now brings its components (After the split item 1).
- **Tests:** server: `a_throw_without_its_stack_takes_it_from_the_throwers_hand` removed with the bridge it tested; the throws in the other tests already carry their stacks.
- **Test steps:**
  1. Spawn a pig and throw the splash Poison at it: it flashes red from poison. Put the second Poison in the offhand (F), select the stick and throw again: same.
  2. Throw the splash Harming at a cow: it takes the hit.
- **Kit:**

```
IW4L_MINECRAFT_KIT=splash_potion@poison*2,splash_potion@harming,stick,pig_spawn_egg*2,cow_spawn_egg*2
```

### 2. A Minecraft ender pearl moves only the player who threw it

Commit: 243bc13

- **Changed:** `entities.rs` reads `Output.pearl_teleports` and keeps only the local player's (`PLAYER`) landings; `server.rs` drops `Output.pearl_landings`, which carried every pearl's landing without its thrower (After the split item 2).
- **Tests:** none new; `each_pearl_teleports_its_own_thrower_and_dies_with_it` covers the keyed list.
- **Test steps:**
  1. Throw an ender pearl at the ground 10 blocks away: you land there with the teleport sound and take the 5 pearl damage, as before.
- **Kit:**

```
IW4L_MINECRAFT_KIT=ender_pearl*16
```

### 3. Minecraft snowballs and eggs break into crumbs and pearls into portal specks where they hit

Commit: cc5a400

- **Changed:** `thrown.rs`: `ThrownHit { item, kind, at }`; `hit_crumbs` is the client's entity event 3 (26.3 javap): a snowball breaks into 8 still `item/snowball` crumbs (`Snowball.handleEntityEvent`), an egg into 8 crumbs of its own item each pushed `(nextFloat - 0.5) * 0.08` on every axis (`ThrownEgg.handleEntityEvent`); `pearl_portal` is `ThrownEnderpearl.onHit`'s 32 `PORTAL` specks at the hit point, `nextDouble() * 2` up, moving `(gaussian, 0, gaussian)`. `server.rs`: every snowball, egg and pearl hit reaches `Output.thrown_hits` (`ServerSim::take_thrown_hits`), whoever threw it.
- `block_particles.rs` `spawn_item_crumbs(at, velocities, texture)` (the `BreakingItemParticle` start the potion crumbs use). `entities.rs` turns each hit into portal specks or `item_crumbs`, which the adapter spawns with the item's particle texture through the new `Mining::item_crumbs`. The pearl's old 16 enderman-style specks at the landing are gone.
- **Tests:** thrown: `snowballs_and_eggs_break_into_eight_crumbs_and_pearls_into_portal_specks`. block_particles: `item_crumbs_start_where_the_item_broke_with_their_own_push`. server: `snowballs_eggs_and_pearls_report_where_they_hit`.
- **Test steps:**
  1. Throw a snowball at a wall 4 blocks away: a little puff of white snowball crumbs drops where it hit. Screenshot.
  2. Throw an egg (and a brown and a blue one) at the ground: egg crumbs scatter a little. Screenshot.
  3. Throw an ender pearl: purple portal specks rise around the spot where it lands, and you are moved there. Screenshot.
- **Kit:**

```
IW4L_MINECRAFT_KIT=snowball*16,egg*16,brown_egg*16,blue_egg*16,ender_pearl*16
```

### 4. Minecraft potions and bottles o' enchanting break with the vanilla swirl and glass crumbs

Commit: 643656a

- **Changed:** `entities.rs`: `Entities.spell` (`SpellParticles`, seeded from the world seed) swirls every `Output.potion_breaks` entry (`burst`: 100 `effect` or `instant_effect` specks in the potion's colour; thrown bottles o' enchanting and potions and witches' potions alike), ticks with the portal specks and draws into the translucent entity mesh. Each break also queues its block's bottom centre in `Entities.potion_crumbs`, which the adapter spawns as `LevelEventHandler`'s 8 `splash_potion` item crumbs with the item's particle texture (`Mining::potion_crumbs` over `BlockParticles::spawn_potion_crumbs`). `thrown.rs` `SPLASH_POTION` (After the split item 4).
- **Tests:** none new; the swirl and the crumbs are entry 4 of the throwables group (`a_break_swirls_a_hundred_spells_of_its_colour_from_the_block_bottom`, `a_potion_breaks_into_eight_untinted_crumbs_flying_out_and_up`).
- **Test steps:**
  1. Throw a bottle o' enchanting at the ground: it breaks with the glass sound, a cloud of blue specks swirls up and out from the spot and a few glass crumbs jump out. Screenshot while the specks rise. (This is screenshot 5's complaint.)
  2. Throw the splash Poison: the swirl is green. Throw Harming: the specks are the dark red instant kind (smaller star-like sprites). Screenshot each.
  3. Stand 6 blocks from a witch in survival: her potions break with the same swirl in their colour.
- **Kit:**

```
IW4L_MINECRAFT_KIT=experience_bottle*64,splash_potion@poison,splash_potion@harming,witch_spawn_egg*2
```

### 5. Minecraft splash and lingering potions show their colour in flight

Commit: e1228a3

- **Changed:** `entities.rs` keeps the snapshot's `thrown_views` and draws them with `thrown::append_views`: a splash or lingering potion is its bottle over `item/potion_overlay` tinted with the contents' colour (`ThrownItemRenderer`), every other throw its item sprite as before. `EntitySnapshot.thrown`, `ServerSim::thrown_view` and `thrown::append_sprites` are gone (After the split item 5).
- **Tests:** thrown: `a_potion_in_flight_is_its_tinted_overlay_under_its_bottle`. The server tests that looked at `thrown_view` read `thrown_views`.
- **Test steps:**
  1. Throw the splash Poison, the splash Harming and the lingering Regeneration high into the air and watch them fly: each bottle shows its potion's colour (green, dark red, pink) on the way, not a grey bottle. Screenshot in flight.
  2. Throw a snowball and a bottle o' enchanting: they look as before.
- **Kit:**

```
IW4L_MINECRAFT_KIT=splash_potion@poison,splash_potion@harming,lingering_potion@regeneration,snowball*16,experience_bottle*16
```

### 6. Minecraft lingering potion clouds are drawn with their drifting specks

Commit: c1c9c4b

- **Changed:** `entities.rs` keeps the snapshot's `clouds` and, every client tick before the specks move, gives each one `AreaEffectCloud.clientTick`'s particles through `SpellParticles::cloud` (a disc of `ceil(pi r^2)` still specks in the cloud's colour, or a few white and coloured ones while it waits its first 10 ticks); they draw with the potion swirl of entry 4 (After the split item 8).
- **Tests:** none new; the particles are entry 7 of the throwables group (`a_cloud_fills_its_circle_each_tick`).
- **Test steps:**
  1. Throw the lingering Poison at the ground 4 blocks away: a flat disc of green specks covers about 3 blocks around the spot and shrinks away over about 25 seconds. Screenshot at the start and after 15 seconds.
  2. Throw the lingering Regeneration: the disc is pink.
- **Kit:**

```
IW4L_MINECRAFT_KIT=lingering_potion@poison,lingering_potion@regeneration
```

### 7. Minecraft splash potions, lingering clouds and witches' potions reach the player

Commit: c2c2290

- **Changed:** `survival.rs` `Survival::take_potion(potion, from, random)`: a lasting effect or a heal goes through `potions::apply_to_player`; a harm is magic damage past armour with no exhaustion (the `magic` and `indirect_magic` damage types have exhaustion 0), from where the potion broke, and does nothing while invulnerable (creative); `Extinguish` is `Entity.extinguishFire`: a burning player stops burning (`clearFire`, fire ticks to at most 0) and hears `entity.generic.extinguish_fire` at volume 0.7, pitch `1.6 + (a - b) * 0.4`. `entities.rs` keeps the local player's (`PLAYER`) `Output.player_potions` (player splashes, witch splashes and lingering cloud touches) in `Entities.player_potions`, and the adapter feeds them to that player's `Survival` after the pearl landings, so a totem still saves a player a harming splash would kill (After the split item 6).
- **Tests:** survival: `a_splash_gives_its_effect_heals_and_harms_past_armour`, `a_creative_player_takes_splash_effects_but_no_harm`, `splash_water_puts_out_a_burning_player_with_the_hiss`.
- **Decisions:** a creative player still takes the lasting effects (vanilla's `LivingEntity.isAffectedByPotions` is only `!isDeadOrDying`; `Player` doesn't override it). A harming death reads "was killed by magic" for every thrower: vanilla's `indirectMagic` names the thrower ("was killed by Witch using magic"), but the witch's splash carries no witch ID yet, and with one player in the world the only player thrower is the victim. Effects the player doesn't have yet (speed, strength, night vision...) still do nothing.
- **Test steps** (survival mode):
  1. Throw the splash Poison straight up so it lands on you: you are poisoned (green hearts) and lose health down to half a heart. Screenshot of the hearts.
  2. Throw the splash Harming 3 blocks in front of your feet: you lose about 2 hearts (less than the 3 a direct hit gives). Throw splash Healing II at your feet: you get 4 hearts back.
  3. Throw the lingering Poison at your feet and stand in the cloud: you are poisoned again about once a second; step out and it stops being renewed.
  4. Light netherrack with flint and steel and stand in the fire until you burn, step out and throw the splash Water at your feet: the flames go out with the hiss.
  5. Spawn a witch, stand 6 blocks from it: its potions poison you (green hearts) and hurt you.
  6. Switch to creative (`/gamemode creative`) and throw Harming at your feet: no damage.
- **Kit:**

```
IW4L_MINECRAFT_KIT=splash_potion@poison*2,splash_potion@harming*2,splash_potion@strong_healing*2,lingering_potion@poison,splash_potion@water*2,netherrack*4,flint_and_steel,witch_spawn_egg*2
```

### 8. Drinking a Minecraft potion gives every vanilla potion's effects

Commit: 2857372

- **Changed:** `survival.rs` `Survival::drink_potion(item, components)` is `PotionContents.onConsume`: the drunk stack's `minecraft:potion_contents` (its potion, custom effects and all) through `potions::drink_hits` at the item's `potion_duration_scale`, each applied with entry 7's `take_potion`, instead of its own list of eight potions. It runs for every finished use; a stack without potion contents does nothing. Harming drunk is magic damage without exhaustion, as before (After the split item 7).
- **Tests:** survival: `a_drunk_potion_gives_its_custom_effects_and_every_strong_and_long_form`; `potions_apply_their_vanilla_effects`, `a_potion_of_harming_kills_by_magic_and_a_respawn_forgets_it` and `an_invulnerable_player_takes_no_hits_falls_lava_fire_poison_or_harming` drink through the components now.
- **Decision:** potions the player has no effect for (swiftness, leaping, night vision...) still do nothing, now from the table, so they work once the effect exists.
- **Test steps** (survival mode):
  1. Take some damage, drink the Healing II potion: 4 hearts come back.
  2. Drink the long Weakness potion: punching a cow takes more hits than before. Drink the strong Turtle Master: hits on you hurt much less for 20 s.
  3. Drink the long Poison potion: your hearts turn green and you lose health down to half a heart, for 1:30.
- **Kit:**

```
IW4L_MINECRAFT_KIT=potion@strong_healing,potion@long_weakness,potion@strong_turtle_master,potion@long_poison,cow_spawn_egg*2,zombie_spawn_egg
```

### 9. A Minecraft trident or wind charge that strikes the player hurts them

Commit: 9ac2c01

- **Changed:** `entities.rs`: the local player's (`PLAYER`) `Output.players_struck` join the mob hits `Entities::tick` returns, so the adapter hurts the player with them past armour, a raised shield blocks them, and a totem saves the player as for any hit: a trident for 8, a wind charge's direct hit for 1, from just behind the player along the flight (`struck_hit`). They are not scaled by difficulty: only hits whose cause is a living non-player are (`DamageScaling.WHEN_CAUSED_BY_LIVING_NON_PLAYER`), and only players throw tridents and wind charges here (After the split item 12).
- **Tests:** entities: `a_thrown_trident_hits_the_player_for_8_from_where_it_flew_in`.
- **Decisions:** the death message is the generic "died": vanilla's "was impaled by" and "was blown up by" name the thrower, and with one player the only thrower is the victim; it gets a name with multiplayer. The hit's knockback is left out, as for every other hit on the MW2 body today (the hits path has no knockback).
- **Test steps** (once the trident charge entry below makes the trident throwable; survival mode):
  1. Take the armour off, look straight up, hold right-click with the plain trident for a second and release: it flies up, falls back on you and takes 4 hearts. Screenshot of the hearts.
  2. Put the diamond chestplate on and do it again: it takes less.
- **Kit:**

```
IW4L_MINECRAFT_KIT=trident,diamond_chestplate
```

### 10. Thrown Minecraft tridents are drawn as the 3D trident

Commit: ae11220

- **Changed:** `entities.rs` keeps the snapshot's `tridents` and draws them with `trident::append_tridents` beside the thrown items: the `TridentModel` (pole, base and three spikes) turned along its flight as `ThrownTridentRenderer` does, stuck at its angle in the ground, shimmering when enchanted (After the split item 10).
- **Tests:** none new; the model and its turn are the throwables group's entry 8 (`the_spikes_lead_along_the_heading`, `the_model_is_a_pole_a_base_and_three_spikes_on_a_32_sheet`).
- **Test steps** (once the trident charge entry below makes the trident throwable):
  1. Throw the plain trident at the ground 10 blocks away: a 3D trident flies spikes first and sticks in the ground at an angle. Screenshot of it stuck.
  2. Throw the Loyalty trident: it shimmers purple in flight and on its way back.
- **Kit:**

```
IW4L_MINECRAFT_KIT=trident,trident+loyalty=3
```

### 11. A held Minecraft trident is the 3D trident

Commit: 7db3399

- **Changed:** `special_models.rs`: item definitions are read for a display context (`item_leaves_in`, `uses_item_geometry_in`): a `minecraft:display_context` select takes the case whose `when` names the context, then its fallback, then the other cases; without a context it is read as `gui`. The `minecraft:trident` special model (`TridentSpecialRenderer`) is the `TridentModel`'s pole, base and spikes on `entity/trident/trident`, under the definition's `scale(1, -1, -1)`. `model.rs`: `resolve_item_model_in(pack, item, context)`, and every display transform reads the model of its own context, so a trident's first and third person transforms come from `item/trident_in_hand` (or `item/trident_throwing` while charging) and its gui, ground and frame ones from the flat `item/trident`. `mesh.rs`: `ItemVisual.hand` is the hand context's geometry when it differs from the flat one; items held by bodies and mobs (`append_held_items` in a hand) and the first person hand (`append_posed_items`) draw it. The same goes for spears and the spyglass: in the hand their `_in_hand` models, in the gui and on the ground their flat ones.
- **Tests:** special_models: `a_trident_is_flat_in_the_gui_and_on_the_ground_and_3d_in_the_hands`, `the_held_trident_is_the_pole_base_and_spikes_of_the_thrown_one`. body_render: `a_held_trident_is_the_3d_model_and_on_the_ground_it_stays_flat`.
- **Test steps:**
  1. Select the trident: in first person you hold the 3D trident (a long grey-green pole with three spikes), not the flat icon. Screenshot.
  2. Press F5 for third person: the MW2 body holds the same 3D trident in its right hand; put it in the offhand (F): it is in the left hand. Screenshot each.
  3. Open the inventory: the hotbar and the character box show the flat trident icon in the slots and the 3D trident in the character's hand. Drop it (Q): it lies on the ground as the flat item.
  4. The enchanted trident shimmers in the hand.
- **Kit:**

```
IW4L_MINECRAFT_KIT=trident,trident+loyalty=3
```

### 12. Minecraft tridents charge while right-click is held and are thrown on release like vanilla

Commit: (next entry)

- **Changed:** new `minecraft_survival/src/trident.rs`: `Charging { slot, ticks }` (`TridentItem.use`: it starts only with `trident::can_start`, so not when the next use would break it, and a Riptide one only in water or rain) and `released(charging, use_held, alive, inventory, selected)`: letting go of the use key releases (`releaseUsing`), while switching to another slot, losing the trident or dying only stops (`stopUsingItem`). `player.rs` `Hand.charging` (one field). `entities.rs` `Entities::release_trident`: `trident::release` (10 ticks of charge, one durability, Unbreaking, creative keeps it) and, for a throw, `ThrownTrident::launch` from the eyes with the player's motion, sent to the server under the player's ID. The adapter starts the charge from either hand (right-click, or the offhand key when the main hand holds a gun), counts game ticks while it is held, blocks attacks and other uses meanwhile, plays `item.trident.throw` (volume 1, pitch 1) on a throw, and draws the first-person charge pose (`ItemUseAnimation.TRIDENT` in `FirstPersonHandsAndItemsRenderer`: raised 55 degrees, pulled back and stretched over 10 ticks with the small shake) with the `item/trident_throwing` display. `offhand::has_use` knows the trident (After the split item 9).
- **Tests:** trident: `only_a_trident_starts_charging_and_a_riptide_one_only_when_wet`, `letting_go_releases_but_switching_away_or_dying_only_stops`. minecraft_hand: `a_charging_trident_is_raised_and_pulled_back_over_half_a_second`. offhand: `has_use` covers the trident.
- **Decisions:** the MW2 body keeps its normal hold in third person while charging (vanilla raises the arm over the head, `ArmPose.THROW_TRIDENT`; the MW2 body has no such pose), and the player is not slowed while charging (vanilla walks at a fifth of the speed while using an item; the MW2 movement has no hook for it yet). A Riptide trident released in water wears one durability but doesn't launch yet: that is the next entry.
- **Test steps** (survival mode):
  1. Select the plain trident and hold right-click: the 3D trident rises over your shoulder and pulls back with a small shake. Screenshot while holding.
  2. Release after a second: it flies far, spikes first, with the throw sound and sticks in the ground at an angle with the thud. Release after a quick click: nothing is thrown.
  3. Throw it at a zombie: the zombie takes a big hit (about 8) and the trident drops near it. Walk over it: it comes back into your inventory one durability lower (the bar under the icon).
  4. Throw the Loyalty trident at the ground: it flies back to you with the return sound and lands in your inventory.
  5. Hold right-click and switch hotbar slot before releasing: nothing is thrown.
  6. Put a trident in the offhand (F) and select a gun: press the offhand use key, hold it and let go: the trident is thrown from the left hand.
  7. Creative mode: the trident stays in your hand while a copy flies; it can't be picked up.
- **Kit:**

```
IW4L_MINECRAFT_KIT=trident,trident+loyalty=3,trident,zombie_spawn_egg*4
```
