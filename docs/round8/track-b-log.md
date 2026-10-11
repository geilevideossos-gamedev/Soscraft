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

### 1a. The mob use predicate test leaves villagers out of the comparison

After c8ccb784.

- **Changed:** `mob_actions::would_interact_agrees_with_interact` no longer compares `would_interact` with `interact` for villagers. `interact` handles a villager click (it would open trading), while `would_interact` turns it away on purpose (entry 1's decision), so only the expected answer (false) is checked for them.
- **Tests:** the same test, now green.
- **Test steps:** none (test only).

### 2. Minecraft striders stand on lava, warm up in it and shiver out of it

After 28e87e7c.

- **Changed:**
  - Vendored `entities`: new `strider.rs` (`Strider`: body 0.9 x 1.7, baby 0.45 x 0.85 with eye 0.4375, health 20, `movement_speed` 0.175 or 0.1155 while suffocating, `riding_attachment` 1.7 or 0.65625, the warmth test and `float_in_lava`) and `world/striders.rs` (`StriderEntity`, `spawn_strider`, `striders`, `strider_mut`, `tick_strider`). The tick: warmth (lava at the feet, under them or overlapping, or `#strider_warm_blocks`) sets `suffocating` unless it has no AI; fluid, fire (immune), wall and air checks; effects; ambient, hurt and death sounds (`entity.strider.*`, neutral volume, interval 120); travel (in water `travel_water`, in lava whose block is not at its feet `travel_lava`, otherwise ground travel with the lava floor); `entity.strider.step` or `step_lava` at volume 1 every 0.6 walked; fall damage except in lava; water and rain hurt 1 (`isSensitiveToWater`); age and love; then `floatStrider` (on the lava surface it is on the ground, otherwise its motion becomes `motion * 0.5 + 0.05` up). A saddled strider drops its saddle on death, besides its loot table (2 to 5 string).
  - `steering.rs`: `SteeringState::boost_factor` (`1 + 1.15 * sin(time / total * pi)` while boosting).
  - `movement.rs`: `Body.lava_floor` (ground travel stands on lava sources) and `Body.step_after` (the strider's `nextStep` of 0.6).
  - Vendored `player`: `collision::move_body_on_fluid`: with `lava_floor`, a lava source (level 0) with no lava above collides as a half slab (0 to 0.5) when the body's bottom is above `y + 0.5 - 1e-5` (`LiquidBlock.getCollisionShape` with `isAbove`). `move_body` is that without the floor, so nothing else changes. `World::fluid_is_lava`, and `#strider_warm_blocks` (lava) in the authored block tags.
  - `walk_path.rs`: the path start climbs lava rather than water for a mob that stands on fluid; `WalkProfile::strider` (water -1, lava 0, fire 0, no float) and `WalkTarget::Strider` (10 at lava, minus infinity elsewhere while in lava, else 0) for the AI of entry 3.
  - Wiring: `EntityKey::Strider`, `MobHit::Strider` and every per-kind list in `world.rs`, `combat.rs`, `pushing.rs`, `hazards.rs` and `witches.rs` (effects, healing, magic); the census counts striders as creatures.
  - `minecraft_terrain/src/server_mobs.rs`: `minecraft:strider` is simulated; `spawn_saved` reads `Age`, `InLove`, `Health`, `PersistenceRequired`, `NoAI` and `equipment.saddle`; `mob_tags` saves `Age`, `ForcedAge`, `InLove` and the saddle slot, keeping the other equipment slots as loaded. A strider with `Passengers` (a jockey) still waits dormant until entry 5.
  - `minecraft_survival/src/entities.rs`: MW2 hit key 23 for striders (`STRIDER_KEY`); the hit box comes with entry 4.
- **Tests:** `strider`: `a_strider_on_a_lava_source_stands_half_a_block_up`, `only_striders_stand_on_lava`, `out_of_lava_it_suffocates_and_slows_to_0_1155`, `water_and_rain_hurt_it`, `a_baby_is_0_45_by_0_85`, `the_boost_factor_peaks_at_2_15_halfway`, `striders_ignore_lava_and_fire`. `server_mobs`: `a_strider_round_trips_with_its_saddle_age_and_love`, `a_strider_with_a_rider_waits_dormant`.
- **Decisions:**
  - Strider movement uses ground travel even inside lava, as vanilla's `shouldTravelInFluid` does for a mob that can stand on the fluid at its block; only a strider whose block is not lava while it overlaps lava swims with `travel_lava`.
  - The suffocating speed modifier (-0.34 of the base) is applied before the potion effects' speed changes, a close match to vanilla's attribute order for the effects a strider can get.
  - Sounds use the neutral volume slider (`Entity.getSoundSource` is `NEUTRAL`; striders don't override it).
- **Test steps:** striders are drawn from entry 4 on; until then these steps show only sounds. Start in the Nether (`IW4L_MINECRAFT_DIMENSION=the_nether`), survival:
  1. Use a strider egg on the lava sea: the strider stands on the surface, half a block up (**1**).
  2. Use an egg on netherrack 5 blocks from the lava: it stands there, and turns pale and shivers (entry 4) (**2**).
  3. Pour a water bucket next to a strider on land so the water reaches it: it takes damage every half second.
  4. Kill a strider with the sword: 2 to 5 string drop.
  5. Save and exit with a strider on lava and one on land; restart: both are where they were.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:strider_spawn_egg*8,minecraft:water_bucket,minecraft:diamond_sword,minecraft:netherrack*64
```

### 3. Minecraft striders seek lava, follow warped fungus and breed with it

After e9e31c3e.

- **Changed:**
  - Vendored `entities`: new `strider_ai.rs`, the goal table of `Strider.registerGoals`: panic (1.65), breed (1.0), tempt (1.4, `#strider_tempt_items`), `StriderGoToLavaGoal` (`MoveToBlockGoal(1.0, 8, 2)`: only out of lava, a start delay of 100 to 200 goal ticks, the ring search over 8 blocks and 3 layers for lava with a pathfindable block above, a fresh path every 20 tries, and it keeps on until the strider is in lava or the lava is gone), follow parent (1.0), random stroll (1.0, interval 60, `DefaultRandomPos`, not water-avoiding), look at players (8), random look, look at striders (8). `world/striders.rs` runs them with navigation, move and look controls, plays `entity.strider.happy` (1 in 140 while tempted) and `entity.strider.retreat` (1 in 60 while panicking), keeps the ambient sound quiet in both, and breeds two striders into a baby.
  - New vendored `controls.rs`: `PlayerControls { id, yaw, pitch, main_hand, offhand, sneaking }`, the per-player held-item channel of track B (b1 adds the armour). `EntityWorld::set_player_controls` and `player_controls`. Striders are tempted through the world's item tags, so only `#strider_tempt_items` counts.
  - `stroll.rs`: a mob that stands on fluid takes a lava block as a stable destination (`StriderPathNavigation.isStableDestination`).
  - `strider.rs`: navigation, move control, `walk_profile`, `interact_food` (`Animal.mobInteract` with `#strider_food`) and `interact_saddle` (an adult, alive, unsaddled strider and a saddle in the hand).
  - `minecraft_terrain`: `mob_actions::interact` has a strider arm: warped fungus feeds (love, a baby's growth, the golden dandelion's age lock) with `entity.strider.eat` (volume 1, pitch `1 + (r1 - r2) * 0.2` from the strider's random), otherwise a saddle goes on with `entity.strider.saddle`. `TickInput.controls`, handed to the entity world before the mobs tick.
  - `minecraft_survival`: `PlayerView` carries `main_hand`, `offhand` and `sneaking`; `Entities::tick` sends one `PlayerControls` per living player. `render_anim` fills them (the main hand only while a Minecraft item is held, sneaking from crouch or prone).
  - Also drops a needless `mut` in entry 1's `a_saddle_used_on_a_pig_leaves_the_inventory_and_saddles_it` (a compiler warning).
- **Tests:** `strider_ai`: `a_strider_on_netherrack_walks_to_lava_within_eight_blocks`, `a_strider_in_lava_never_strolls_onto_land`, `warped_fungus_and_the_stick_tempt_it_at_1_4`. `mob_actions`: `two_fed_striders_make_a_baby`, `a_saddle_goes_on_an_adult_but_not_on_a_baby`, `warped_fungus_feeds_even_a_saddled_strider`. `controls`: `either_hand_counts`.
- **Decisions:**
  - `StriderGoToLavaGoal.canContinueToUse` doesn't call `MoveToBlockGoal`'s, so the strider never gives up after 1200 tries; checked with javap and kept that way (the brief said 1200).
  - Breeding gives no experience orbs, as pig breeding in this repo doesn't either.
  - The other species' tempt flags in `PlayerCandidate` stay false, as before (a known limitation).
- **Test steps (Nether, survival):**
  1. Use a strider egg on netherrack 5 blocks from the lava sea: within about 20 seconds it walks into the lava (**2**).
  2. Hold warped fungus near a few striders: they come to you (on land slowly), sometimes making a happy sound. Hold the warped fungus on a stick: the same. A carrot on a stick: nothing (**3**).
  3. Feed two adult striders warped fungus: each eats with its sound and shows hearts, and a baby strider appears between them (**3**).
  4. Feed the baby: it grows a little (green particles are not drawn yet).
  5. Use a saddle on an adult: the saddle is used up with the strider saddle sound (the saddle is drawn from entry 4). Try a saddle on the baby: nothing happens and you keep the saddle (**4**).
  6. Feed a saddled strider: it still eats.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:strider_spawn_egg*8,minecraft:warped_fungus*64,minecraft:warped_fungus_on_a_stick,minecraft:carrot_on_a_stick,minecraft:saddle,minecraft:saddle,minecraft:netherrack*64
```

### 4. Minecraft striders look like vanilla

After 0eead303.

- **Changed:**
  - New `minecraft_terrain/src/strider_render.rs`: `AdultStriderModel` (64 x 128: two 4 x 16 x 4 legs at (-4, 8, 0) and (4, 8, 0), the 16 x 14 x 16 body, six 12 x 0 x 16 bristle planes with rest rolls 0.8727, 1.1345 and 1.2217, the right ones mirrored) and `BabyStriderModel` (32 x 32: 2 x 4 x 2 legs at (-1.5, 20, 0) and (1.5, 20, 0), the 7 x 7 x 8 body at 17.25, three 7 x 3 x 0 bristles), cuboids by javap on `createBodyLayer`. `StriderModel.setupAnim` and both `customAnimations`: the legs swing and roll with the walk (speed capped at 0.25), the body rocks and bobs, the bristles sway with the walk and the age, and an unridden body takes the head's pitch and yaw. The texture is `strider`, `strider_cold`, `strider_baby` or `strider_cold_baby` by age and `suffocating`; while suffocating the body shakes by `cos(ageInTicks * 3.25) * pi * 0.4` degrees (`isShaking`). A saddled adult draws the model again with `entity/equipment/strider_saddle/saddle` (the saddle layer bakes the same adult layer).
  - `mesh.rs` loads the five textures; `client_mobs.rs` tracks striders and casts their shadows (0.5, a baby 0.25); `minecraft_survival/src/entities.rs` draws them and gives MW2 bullets their hit boxes (key 23).
- **Tests:** `strider_render`: `the_adult_has_three_cubes_and_six_bristle_planes`, `the_cold_texture_shows_while_suffocating`, `the_shake_is_cos_3_25_times_0_4_pi_degrees`, `a_saddled_adult_adds_the_saddle_layer`, `walking_swings_the_legs_against_each_other`. `entities`: `a_strider_hit_box_encodes_and_decodes`.
- **Decisions:** the spawner's displayed mob (track A) is not in this worktree, so the strider is not registered there; whoever merges second adds the one line.
- **Test steps (Nether, survival):**
  1. Use strider eggs on the lava sea: red striders with six swaying bristles stand on the lava; walking, their legs swing (**1**).
  2. Use an egg on netherrack away from the lava: it is pale purple and shivers until it reaches lava, where it turns red (**2**).
  3. Breed two (warped fungus): the baby is a small strider with three bristles, red on lava, purple on land.
  4. Saddle an adult: the saddle shows on its back (**4**).
  5. Shoot a strider with an MW2 gun: it takes damage and dies; kill one with the sword: string drops, and a saddled one also drops its saddle (**11**).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:strider_spawn_egg*8,minecraft:warped_fungus*64,minecraft:saddle,minecraft:saddle,minecraft:diamond_sword,minecraft:netherrack*64
```
