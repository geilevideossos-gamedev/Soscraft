# Track E1 log: screens, visuals and sounds

Branch `track-e1`, worktree `C:\Dev\mc-wt\track-e1`, target dir `D:\iw4l-wt\track-e1`. Brief: `track-e.md` (overview and the E1 sections). Base: `ed23b000`.

A commit can't name its own hash, so each entry is written with its commit and names the commit before it; the last entry's hash is in `git log` under its subject.

## Protocol

- "Minecraft taming shows hearts, and a failed try or a bucking horse shows smoke" (after `280f5fe9`): `EntitySnapshot` has a new field `mob_puffs`. The snapshot rides in the state frame's `entities` blob and in `ToClient::Bursts`, so host and friend must run the same build. No message or codec tag changed.

## E-kit. Kits and /give take banner patterns, loaded crossbows and firework stars

Commit: "Minecraft item commands and kits take banner patterns, shield colours, loaded crossbows and firework stars", the one after `ed23b000`.

- **Cause** (as in the brief, re-checked at `ed23b000`): `/give` and `/kit` (`crates/minecraft_commands/src/args.rs`, `components`) let through only book content, the ominous amplifier, custom data and enchantments, so anything else was `arguments.item.component.unknown`. The `.env` kit (`crates/minecraft_terrain/src/crafting.rs`, `kit_components`) kept the same few and dropped the rest without a word.
- **Vanilla 26.3** (javap on the 26.3 jar):
  - `BannerPatternLayers`: a list of `Layer {pattern, color}`. `pattern` is a `BannerPattern` holder: a registry id, or inline `{asset_id, translation_key}`. `color` is a `DyeColor` name.
  - `base_color`: a `DyeColor` name.
  - `ChargedProjectiles`: up to 1024 `ItemStackTemplate`s, each `{id, count (1-99, default 1), components?}` or a bare item id. `minecraft:air` is refused ("Item must be non-empty").
  - `Fireworks`: `flight_duration` (unsigned byte, default 0) and up to 256 `explosions`.
  - `FireworkExplosion`: `shape` (`small_ball`, `large_ball`, `star`, `creeper`, `burst`), `colors` and `fade_colors` (int lists), `has_trail` and `has_twinkle` (booleans, default false).
  - Every default is left out when vanilla saves a component (`optionalFieldOf` with a default).
- **Changed:**
  - New `third_party/minecraftoss/world/src/item_components.rs` (`minecraftoss_world::item_components`): `content(component, snbt)` reads one SNBT value with the vendored `snbt` reader, checks it against the codecs above and returns the component's JSON in the shape vanilla saves. `COMPONENTS` lists the five names. It reuses `book.rs`'s `tag_json`, `list`, `string`, `number` and `describe`, now `pub(crate)`.
  - `args.rs`: the five components are accepted in `[...]`. `snbt_value` reads the value up to the next `,` or `]` outside brackets and quotes, so lists, compounds and quoted or bare strings all work. A value that `content` refuses gives `arguments.item.component.malformed` (component and reason) with the cursor at the value, like book content.
  - `crafting.rs` `kit_components`: the same five through `item_components::content`. A refused value is left out, as the kit already did for books and ominous amplifiers.
  - Fireworks code reads `flight_duration` as before (`firework.rs`), so a kit or `/give` rocket now flies as long as its component says. Nothing draws banner patterns, shield colours or the loaded crossbow yet: groups `item-states`, `banner-items` and `player-looks` do that.
- **Tests:**
  - `minecraftoss-world` `item_components::tests`: `banner_layers_take_ids_with_or_without_a_namespace_and_inline_patterns`, `a_base_colour_is_a_dye_name_quoted_or_not`, `charged_projectiles_are_item_templates_with_a_count_of_one_left_out`, `fireworks_keep_their_flight_and_stars_in_the_saved_shape`, `a_firework_star_takes_its_shape_colours_and_flags`, `malformed_lists_and_layers_are_refused`.
  - `minecraft_commands` `args::tests`: `banners_and_shields_take_patterns_and_a_base_colour`, `a_crossbow_takes_its_charged_projectiles`, `rockets_and_stars_take_their_firework_components`, `bad_dyes_shapes_and_lists_are_malformed_components` (a bad dye, a bad shape, a compound for a list, an unterminated list, a count of 0 and a broken int array, each malformed at the value; a repeated component).
  - `minecraft_terrain` `crafting::tests`: `kit_banners_shields_crossbows_and_fireworks_carry_their_components` (the banner, shield, crossbow, rocket and star entries of both E kits give the right JSON; a bad dye, a bad shape or a compound for a list leaves the component out; `give_kit` puts them on the stacks), `a_kit_rocket_keeps_its_star_through_a_save_and_load` (`firework_rocket[fireworks={flight_duration:1,explosions:[{shape:large_ball,colors:[I;11743532],has_trail:1b}]}]*64` gives one stack of 64 with that component; it goes through `stack_tag`, gzip NBT and `stack_of` unchanged and still flies 1).
- **Decisions:**
  - The parser lives in the vendored world crate next to `book`, which already serves both `/give` and the kit. `minecraft_commands` does not depend on `minecraft_terrain`, so it can't call `item_nbt::tag_json`. Adding that dependency would have changed `Cargo.lock`.
  - Components are stored in the shape vanilla saves them: ids get the `minecraft:` namespace, and defaults are left out (a projectile count of 1, a flight of 0, empty explosion and colour lists, false flags).
  - A true `has_trail` or `has_twinkle` is stored as `1`. A saved vanilla byte loads as `1` in our JSON, so a stack is the same before and after a save. Code that reads the flags (E2's fireworks) should treat any non-zero number, or `true`, as set.
  - Banner pattern ids are not checked against the `banner_pattern` registry, which is data-driven and read at runtime by the drawing code. An unknown id is kept and draws nothing. Vanilla refuses an unknown id.
  - A `flight_duration` outside 0-255 is refused. Vanilla's unsigned byte silently wraps a number read from SNBT.
  - A charged projectile's own `components` are kept as given and not checked.
  - Our SNBT reader accepts a trailing comma in a list (`[{id:"arrow"},]`), so that is not malformed.
- **Owner's test steps** (survival, the host, the kit below in the `.env`, a new world):
  1. **The kit.** Open the inventory. It has 4 white banners, a shield, a crossbow, 16 rockets, 16 more rockets and an elytra. The patterns, the shield's colour and the loaded arrow are not drawn yet (later E1 groups). **Screenshot 1.**
  2. **Flight 3 rockets.** Put on the elytra, climb about 30 blocks, jump and glide. Use a rocket from the second stack (plain, flight 1): it pushes for about 1 second. Use one from the first stack (flight 3): it pushes for about 2 seconds, clearly longer.
  3. **`/give` takes the five components.** Type each line. Each gives the item with no red error:
     - `/give @s minecraft:white_banner[banner_patterns=[{pattern:"creeper",color:"black"}]]`
     - `/give @s minecraft:shield[base_color=blue]`
     - `/give @s minecraft:crossbow[charged_projectiles=[{id:"minecraft:arrow"}]]`
     - `/give @s minecraft:firework_rocket[fireworks={flight_duration:2,explosions:[{shape:star,colors:[I;14602026]}]}] 8`
     - `/give @s minecraft:firework_star[firework_explosion={shape:creeper,colors:[I;4312372],has_twinkle:1b}]`
  4. **Bad values are refused.** Each line gives a red "Malformed ... component" error naming the component and the reason, and gives nothing. **Screenshot 2** (the chat with the three errors):
     - `/give @s minecraft:shield[base_color=purpl]` ("Unknown element name:purpl")
     - `/give @s minecraft:firework_star[firework_explosion={shape:square}]` ("Unknown element name:square")
     - `/give @s minecraft:white_banner[banner_patterns={pattern:"creeper",color:"black"}]` ("Not a list")
  5. **`/kit`.** Empty a few backpack slots and type `/kit`: the same items come again.
  6. **Save and load.** Save and quit, load the world. Glide again and use a rocket from the first stack: it still pushes for about 2 seconds.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:white_banner[banner_patterns=[{pattern:"minecraft:creeper",color:"black"},{pattern:"minecraft:border",color:"red"}]]*4,minecraft:shield[base_color=red,banner_patterns=[{pattern:"minecraft:cross",color:"yellow"}]],minecraft:crossbow[charged_projectiles=[{id:"minecraft:arrow"}]],minecraft:firework_rocket[fireworks={flight_duration:3}]*16,minecraft:firework_rocket*16,minecraft:elytra
```

## E-particles. Taming hearts, bucking smoke, trust hearts, raid sweat and the llama spit puff

### Taming hearts and smoke

Commit: "Minecraft taming shows hearts, and a failed try or a bucking horse shows smoke", the one after `280f5fe9`.

- **Cause** (as in the brief, re-checked): the vendored mob world records vanilla's `broadcastEntityEvent` calls (7 tamed, 6 failed try or buck, 41 and 40 for the ocelot's trust, 18 in love, the villagers' 12, 13, 14 and 42), but `ServerSim::tick_mobs_with` threw them away with `let _ = mobs.take_entity_events();`, and the client had no heart particle.
- **Vanilla 26.3** (javap): `TamableAnimal`, `AbstractHorse` and `Animal.handleEntityEvent` draw 7 particles (`HEART` for 7 and 18, `SMOKE` for 6), each with motion `nextGaussian() * 0.02` three times, at `getRandomX(1)`, `getRandomY() + 0.5`, `getRandomZ(1)`. `Ocelot.handleEntityEvent` draws hearts for 41, smoke for 40, and hands anything else to `Animal` (so 18 is hearts). `HeartParticle`: the passed motion is ignored (`super(..., 0, 0, 0)`), velocity * 0.01, y + 0.1, friction 0.86, quad size * 1.5, lifetime 16, no physics, opaque, and `getQuadSize` grows over the first 1/32 of its life.
- **Changed:**
  - New `crates/minecraft_terrain/src/mob_particles.rs`: `PuffKind` (`Heart`, `Smoke`, `AngryVillager`, `HappyVillager`, `Splash`), `MobPuff` (kind, count, feet, width, height, lift, tracking range) and `puff_for(entity_type, event)`, the vanilla table. This commit has the animal rows: 7 and 18 hearts, 6 smoke, the ocelot's 41 hearts, 40 smoke and 18 hearts, each 7 particles half a block up. Villagers give nothing yet, and every other event gives nothing (the ravager's 39 and 69 stay with the clock-driven effects). `mob_puffs(world, events)` places each puff on the mob's body (`EntityWorld::body_of`) with its `mob_tracking::tracking_range`, and drops events of mobs that are gone or have no row.
  - `server.rs`: the dropped events line (and its comment) is now `mob_puffs` collecting the puffs into `ServerSim.mob_puffs`, which `finish_batch` moves into the new `EntitySnapshot.mob_puffs`.
  - `mob_sender.rs`: `near` keeps a puff for a friend only in a chunk they have and within the mob's tracking range; `take_bursts`, `add_bursts` and `has_bursts` treat puffs as one-shot bursts, so they leave the droppable state frame, are kept until sent and go on the reliable `ToClient::Bursts`.
  - `ambient_particles.rs`: `Kind::Heart` (vanilla's `HeartParticle`, ticked with `coast`, sprite `heart`, added to the atlas list).
  - `entity_view.rs`: `mob_puffs()` draws each puff with the view's own random in vanilla's order, called with one line at the top of `bursts()`. Host and friend see the same kind, count and place, not the same specks.
- **Tests:**
  - `minecraft_terrain` `mob_particles::tests`: `taming_shows_seven_hearts_and_a_failed_try_seven_smokes` (horse, llama, wolf, cat), `an_ocelot_shows_hearts_when_it_trusts_and_smoke_when_it_does_not`, `a_villager_in_love_and_a_ravager_s_roar_draw_nothing_here`, `an_event_of_a_mob_that_is_gone_is_dropped`, `a_puff_sits_on_the_mob_s_body`.
  - `minecraft_terrain` `server::tests::a_tamed_wolf_s_hearts_reach_the_snapshot`: a wolf is given bones until it is tamed; each failure's snapshot holds one smoke puff of 7 on the wolf, the success's one heart puff, and the next snapshot none.
  - `minecraft_terrain` `ambient_particles::tests::a_heart_rises_slowly_through_blocks_for_sixteen_ticks`.
  - `minecraft_survival`: `mob_sender::tests::mob_puffs_go_on_the_reliable_stream_and_stay_with_their_range`, `entity_view::tests::a_heart_puff_draws_seven_hearts_on_the_mob`, `remote::tests::a_friend_sees_the_hosts_taming_hearts`.
- **Decisions:**
  - An ocelot's 18 gives hearts too: the brief's table had only 41 and 40, but vanilla's `Ocelot.handleEntityEvent` passes 18 on to `Animal`.
  - Each viewer draws the specks with its own random (`EntityView.random`), so host and friend see the same puff in slightly different specks, as with the wither smoke.
  - `HAPPY_VILLAGER` puffs are already drawn by this commit's view code, but nothing sends one until the villager rows of the next commit.
- **Owner's test steps** (survival, Normal, by day on plains, a new world, the kit below; hearts are small red hearts that rise slowly for under a second, smoke is a few grey puffs that drift up):
  1. **A horse.** Use a horse egg and right-click the horse with an empty hand: you ride it while it runs about. When it rears and throws you off, 7 grey smoke puffs rise around it. **Screenshot 1.** Feed it wheat a few times and try again until it stays calm: 7 red hearts rise around it, about half a block above its body. **Screenshot 2.**
  2. **A llama.** Use a llama egg and tame it the same way: smoke when it throws you, hearts when it is tamed.
  3. **A wolf.** Use a wolf egg and right-click it with a bone. Each failure shows smoke. The success shows hearts, and the wolf sits with a red collar. **Screenshot 3.**
  4. **A cat.** Use a cat egg. Hold cod and stand still until it creeps close, then right-click it with cod. A failure shows smoke and a success shows hearts. **Screenshot 4.** Tame a second cat the same way, then feed both cod again: hearts on each.
  5. **An ocelot.** Use an ocelot egg, hold cod and stand still. When it is within 3 blocks, right-click it with cod. A failure shows smoke. The success shows hearts, and it stops running from you. **Screenshot 5.**
  6. **Love hearts.** Feed the tamed horse from step 1 a golden carrot: hearts.
  7. **Two windows** (host H, friend F, `scripts\mp-local.ps1` or over the relay; both with the kit):
     1. F tames a wolf with bones while H stands 10 blocks away. Smoke and then hearts show in **both** windows. **Screenshot 6**, both windows.
     2. H rides an untamed horse until it bucks, with F watching from 10 blocks: F sees the smoke. H tames it: F sees the hearts.
     3. F walks 200 blocks away, and H tames a cat: F sees nothing. F comes back, and H feeds the tame cat cod: F sees the hearts.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:horse_spawn_egg*2,minecraft:llama_spawn_egg*2,minecraft:wolf_spawn_egg*2,minecraft:cat_spawn_egg*4,minecraft:ocelot_spawn_egg*2,minecraft:wheat*64,minecraft:golden_carrot*16,minecraft:bone*32,minecraft:cod*64,minecraft:iron_sword,minecraft:cooked_beef*32
```

### Villager hearts, anger, happiness and raid sweat

Commit: "Minecraft villagers show hearts, anger, happiness and raid sweat", the one after `8618a93d`.

- **Cause:** the villagers' entity events reached the server's puff list after the last commit, but `puff_for` had no villager rows and the client had no angry-villager or splash particle.
- **Vanilla 26.3** (javap): `Villager.handleEntityEvent` draws 12 `HEART`, 13 `ANGRY_VILLAGER`, 14 `HAPPY_VILLAGER` and 42 `SPLASH` through `AbstractVillager.addParticlesAroundSelf`: 5 particles, motion `nextGaussian() * 0.02` three times, at `getRandomX(1)`, `getRandomY() + 1.0`, `getRandomZ(1)`. Anything else goes to `AbstractVillager`, so a villager's 18 draws nothing. `HeartParticle$AngryVillagerProvider` is the heart particle half a block higher with the `angry` sprite and colour (1, 1, 1). `SplashParticle` is a `WaterDropParticle` with gravity 0.04, whose velocity becomes `(xa, 0.1, za)` when `ya` is 0 and `xa` or `za` is not.
- **Changed:**
  - `mob_particles.rs`: the villager rows (12 heart, 13 angry, 14 happy, 42 splash, each 5 particles a block up); a villager's other events, 18 included, give nothing.
  - `ambient_particles.rs`: `Kind::AngryVillager` (the heart particle at `at + (0, 0.5, 0)`, sprite `angry`, added to the atlas list, ticked with `coast`) and `Kind::Splash` (the rain drop's draws with gravity 0.04 and the sideways override, ticked with `drip`, sprites `splash_0..3`).
  - `entity_view.rs`: `mob_puffs()` maps the two new kinds.
- **Tests:**
  - `minecraft_terrain` `mob_particles::tests::villager_events_give_five_particles_a_block_up` (12, 13, 14, 42; a wolf's 12 and a wandering trader's 14 give nothing; a spawned villager's 42 becomes one splash puff of 5 a block up with range 10, its 18 nothing).
  - `minecraft_terrain` `ambient_particles::tests`: `an_angry_cloud_starts_half_a_block_higher_and_is_white`, `a_splash_drop_falls_at_0_04_and_pops_on_the_ground`.
  - `minecraft_survival` `entity_view::tests::a_villager_s_puffs_draw_five_of_their_kind_a_block_up`.
- **Decisions:** none beyond the brief. The villagers' events come from code that already ran: 13 when a player hurts a living villager, 14 after a trade, 12 to both parents and the baby after breeding, 42 one tick in 100 for a villager with AI inside an active raid.
- **Owner's test steps** (survival, Normal, by day, a new world, the kit below):
  1. **Anger.** Use a villager egg. Punch it once with an empty hand: a few dark grey clouds puff over its head. **Screenshot 7.**
  2. **Happiness.** Place the composter beside a second villager. When it has become a farmer (a straw hat), make one trade: green sparkles over its head. **Screenshot 8.** A 26.3 farmer offers 2 of its 5 novice trades at random (20 wheat, 26 potatoes, 22 carrots or 15 beetroot for an emerald, or an emerald for 6 bread), so it always wants at least one crop; the kit has all four.
  3. **Raid sweat.** `/locate structure minecraft:village_plains` and `/tp` there. Walk 60 blocks out, drink an ominous bottle and walk back in. Wait for the raid. While the villagers hide in the houses, each one now and then (about every 5 seconds) shows a few light-blue water drops around its head, which fall and vanish. **Screenshot 9**, through a door or window.
  4. **Optional, villager hearts.** Put down the three beds near two villagers and throw each one 6 bread. After a while hearts rise over both, and a baby villager appears with hearts of its own.
  5. **Two windows** (host H, friend F): during a raid with both in the village, F also sees the villagers' water drops; H punches a villager next to F, and F sees the grey clouds.
- **Kit** (the whole `particles-events` group, taming steps of the last commit included; 22 backpack slots):

```
IW4L_MINECRAFT_KIT=minecraft:horse_spawn_egg*2,minecraft:llama_spawn_egg*2,minecraft:wolf_spawn_egg*2,minecraft:cat_spawn_egg*4,minecraft:ocelot_spawn_egg*2,minecraft:villager_spawn_egg*4,minecraft:wheat*64,minecraft:potato*64,minecraft:carrot*64,minecraft:beetroot*64,minecraft:golden_carrot*16,minecraft:bone*32,minecraft:cod*64,minecraft:bread*32,minecraft:white_bed,minecraft:white_bed,minecraft:white_bed,minecraft:composter,minecraft:emerald*64,minecraft:ominous_bottle*2,minecraft:iron_sword,minecraft:cooked_beef*32
```

## E-trading. Trading screen: number keys, F, Q, drags and double-click

### Number keys and F on the payment and result slots

Commit: "Minecraft trading screen takes number keys and the offhand key on its payment and result slots", the one after `1443f0e4`.

- **Cause** (as in the brief, re-checked at `1443f0e4`): the HUD already sends `McClick::Swap` for a number key or F over any slot, and the friend's client sends the same `MenuAction::Swap`, but `merchant_op` (`menus.rs`) had no arm for a swap on `Container(0|1|2)` or `Output`, and `merchant_local` lets through only the player's own slots, so the click was dropped. `MerchantOp` and `MerchantMenu` had no swap either.
- **Vanilla 26.3** (javap, `AbstractContainerMenu.doClick`, SWAP branch): only for buttons 0-8 and 40. Both empty: nothing. Hotbar empty: if `mayPickup`, the hotbar gets the slot's stack, then `onSwapCraft`, `setByPlayer(EMPTY)` and `onTake`. Slot empty: if `mayPlace`, the slot takes `split(getMaxStackSize(stack))` when the hotbar stack is bigger than that (min(99, the item's max)), else the whole stack. Both full: needs `mayPickup && mayPlace`; an overfull hotbar stack puts `split(max)` in, `onTake` runs and the old slot stack goes to `inventory.add`, or is dropped with `player.drop(stack, true)`; otherwise the two swap and `onTake` runs. `MerchantResultSlot.mayPlace` is false, so the result only goes into an empty hotbar or offhand slot, and `mayPickup` needs the active offer to be paid for either way round. A payment slot's `onTake` is `setChanged` (the offers re-read); the result's pays for the trade.
- **Changed:**
  - `third_party/minecraftoss/entities/src/merchant.rs`: `MerchantMenu::swap_payment(slot, hotbar, selected, inventory, merchant)` follows the SWAP branch on a payment slot, with `set_payment_by_player` for `setByPlayer` and `update_sell_item` for `onTake`. It returns what `Inventory::add_item` could not take back. `swap_result(hotbar, inventory, merchant)` puts the result in an empty hotbar or offhand slot when `may_take_result` allows it, empties the slot, re-reads the offers and pays with `take_result`, as `click_result` does.
  - `world/merchants.rs`: `EntityWorld::merchant_swap` (payment 0 or 1, result 2) next to `merchant_click_payment`.
  - `crates/minecraft_terrain/src/server.rs`: `MerchantOp::Swap { slot, hotbar }` next to `Payment`; `Server::merchant` runs it. A stack that does not fit drops at the feet, like the overflow of a close (the next commit throws it from the eye instead).
  - `crates/minecraft_survival/src/menus.rs`: `merchant_op` maps `Swap` on `Container(0..=2)` or `Output` to `MerchantOp::Swap` (`Output` is slot 2). `keeps_gun` still refuses a gun from the hotbar into `Container(_)`.
- **Tests:**
  - `minecraftoss-entities` `merchant::tests`: `a_number_key_swaps_a_payment_with_a_hotbar_slot_or_the_offhand` (to an empty hotbar slot and back, to the offhand and into the second payment, a key other than 1-9 or F does nothing, a carrot stack swaps with the wheat, no trade), `an_overfull_hotbar_stack_puts_a_slot_s_worth_in_and_the_old_payment_goes_to_the_inventory` (16 of 20 eggs with a stack size of 16 go in, the wheat goes back to the inventory, or comes back to be dropped when the inventory is full), `a_number_key_on_the_result_trades_once_into_an_empty_hotbar_slot_only` (one trade into hotbar 1, refilled; a filled slot takes nothing; the offhand takes the second; then the result is gone).
  - `minecraft_terrain` `server::tests::number_keys_on_the_trading_screen_move_payments_and_trade_the_result_into_the_hotbar` (a real farmer: the wheat out to hotbar 2 and back, the result into hotbar 3 with one orb, a second press does nothing).
  - `minecraft_survival` `menus::tests::a_villager_s_trading_screen_turns_clicks_into_trades`: a number key on payment 2 and F on the output send `MerchantOp::Swap`.
- **Decisions:**
  - `swap_payment` takes the selected hotbar slot, because vanilla's `Inventory.add` puts the old payment on the selected stack first (`add_item(stack, selected)`).
  - Until the next commit the rare leftover of an overfull hotbar stack drops at the player's feet; the next commit throws it from the eye like vanilla's `player.drop(stack, true)`.
- **Owner's test steps** (survival, by day on open grass at least 60 blocks from a village, a new world, the kit below; the kit fills only the backpack, so the hotbar and offhand start empty):
  1. **A farmer.** Place the composter, fence a 5 x 5 pen around it and use a villager egg inside. Wait until it wears a straw hat. Right-click it and pick its crop offer (wheat, potatoes, carrots or beetroot: a 26.3 farmer picks 2 of its 5 novice trades at random, so it always wants at least one crop). The crop moves into the first payment slot and an emerald shows as the result. "The crop" below means that one.
  2. **Number key on a payment.** Hover the first payment slot and press 1: the crop goes to hotbar slot 1 and the emerald disappears. **Screenshot 1.** Press 1 again: the crop goes back and the emerald returns.
  3. **F on a payment.** Hover the payment and press F: the crop goes to the offhand. Press F again to bring it back.
  4. **Number key on the result.** Hover the result and press 2: one trade is made, the emerald lands in hotbar slot 2, the payment shrinks by the price and an experience orb drops. Press 2 again over the result: nothing happens, because a filled hotbar slot doesn't take the result. **Screenshot 2.**
  5. **Swapping two stacks.** Close the screen, put a stack of another crop in hotbar slot 3, open the farmer again and pick the crop offer. Hover the payment and press 3: the two stacks change places, and the result shows only if the farmer also buys the other crop.
  6. **Two windows** (host H, friend F, `scripts\mp-local.ps1` or over the relay, both with the kit): H closes the screen and F does steps 2-4 on the same farmer. They work the same, and H sees F's orb drop.
- **Kit** (18 backpack slots; each `*192` gives three stacks):

```
IW4L_MINECRAFT_KIT=minecraft:wheat*192,minecraft:potato*192,minecraft:carrot*192,minecraft:beetroot*192,minecraft:emerald*64,minecraft:villager_spawn_egg*4,minecraft:composter,minecraft:oak_fence*32,minecraft:torch*32,minecraft:bread*16
```

### Q and Ctrl+Q on the payment and result slots

Commit: "Minecraft trading screen throws from its payment and result slots with Q", the one after `7ec4eebd`.

- **Cause:** the HUD sends `McClick::Throw` for Q and Ctrl+Q over a slot that holds something, but `merchant_op` had no arm for a throw on the payments or the result, so it was dropped; `MerchantOp` had no throw and `MerchantUpdate` could not hand stacks back to throw.
- **Vanilla 26.3** (javap, `AbstractContainerMenu.doClick`, THROW branch; `Slot.safeTake` and `tryRemove`; `MerchantContainer.removeItem`): only with an empty cursor. The amount is 1 for Q and the slot's whole count for Ctrl+Q. Each `safeTake(amount, MAX)` is dropped with `player.drop(stack, true)`; with Ctrl it repeats while the slot still holds the same item (`isSameItem`). `MerchantContainer.removeItem(2, n)` always takes the whole result, then `setByPlayer(EMPTY)` re-reads the offers and the result's `onTake` pays. So Q on the result makes one trade and throws it, and Ctrl+Q keeps trading and throwing until the payments run short or the offer is out of stock. On a payment, Q throws one and Ctrl+Q the stack (the slot is empty after it, so the loop stops).
- **Changed:**
  - `merchant.rs`: `throw_payment(slot, whole, inventory, merchant)` (`try_remove`, then `onTake`'s re-read) and `throw_result(whole, inventory, merchant)` (the Ctrl loop), both refusing a carried cursor. The take of a whole paid result (`take_whole_result`: `may_take_result`, empty the slot, re-read, `take_result`) is shared with `swap_result`.
  - `world/merchants.rs`: `EntityWorld::merchant_throw` next to `merchant_swap`.
  - `server.rs`: `MerchantOp::Throw { slot, whole }` next to `Swap`; `MerchantUpdate.thrown`, set by `Server::merchant` for a throw and for the leftover of a number-key swap (which no longer drops at the feet), empty for a close request.
  - `third_party/minecraftoss/player/src/inventory.rs`: `Inventory::queue_drop`, next to `take_pending_drops`.
  - `crates/minecraft_survival/src/entities.rs`: `apply_merchant_answer` queues `update.thrown` as pending drops. Every path that takes pending drops throws them from the player's eye: `pump_merchant` (the host in `minecraft_world.rs`, a friend in `minecraft_players.rs`), the host's `apply_input`, `menus::apply` and `close`; a death drains them with the inventory.
  - `menus.rs`: `merchant_op` maps `Throw` on `Container(0..=2)` or `Output` to `MerchantOp::Throw`.
- **Tests:**
  - `minecraftoss-entities` `merchant::tests`: `q_on_the_result_makes_one_trade_and_ctrl_q_trades_until_the_offer_runs_out` (Q: one trade, refilled; Ctrl+Q on a 2-use offer: two trades, then out of stock and nothing more; Ctrl+Q with 50 wheat: two trades until the wheat runs short; nothing with a carried stack), `q_drops_one_from_a_payment_and_ctrl_q_the_stack`.
  - `minecraft_terrain` `server::tests::q_on_the_trading_screen_hands_the_thrown_stacks_back_and_drops_the_trades_orbs` (a real farmer: Q gives one emerald in `thrown` and one orb, Ctrl+Q two more trades and orbs, Q and Ctrl+Q on the payment give 1 and 3 wheat, and the server spawns no item).
  - `minecraft_survival`: `entities::tests::trading_screens_reach_only_their_player_and_set_the_slots_before_anything_else` (an answer's `thrown` becomes a pending drop), `menus::tests::stacks_thrown_from_the_trading_screen_are_tossed_by_the_host_and_by_a_friend` (Ctrl+Q on the output sends `MerchantOp::Throw` for the host and for a friend, and the answer's stack comes out of `pump_merchant`, which is `MerchantShown.thrown`, once), and Q on a payment and Ctrl+Q on the output in `a_villager_s_trading_screen_turns_clicks_into_trades`.
- **Decisions:**
  - Thrown stacks come back to the player through the inventory's pending drops (`Inventory::queue_drop`), so the host, a friend and a forced close share one path, and the toss is the player's own Q toss from the eye. The server never tosses.
  - The leftover of a number-key swap with an overfull hotbar stack is thrown the same way (vanilla `player.drop(stack, true)`), replacing the drop at the feet of the last commit.
  - No drop statistic is counted for a stack thrown from the trading screen, as for the other screens' Q.
- **Owner's test steps** (the same world and farmer as above, the same kit; pick the crop offer again first):
  1. **Q on the result.** Hover the result and press Q: one trade is made and the emerald is thrown in front of you, with an experience orb. **Screenshot 3.**
  2. **Ctrl+Q on the result.** Press Ctrl+Q over the result: it keeps trading and throwing emeralds until the crop in the payment runs short or the offer is sold out (a red cross). Orbs drop for each trade.
  3. **Q on a payment.** Pick the crop offer again. Hover the payment and press Q: one crop is thrown. Press Ctrl+Q: the rest of that stack is thrown and the result disappears. **Screenshot 4.**
  4. **Carrying a stack.** Pick up a stack from the inventory, hover the result and press Q: nothing happens. Put the stack back.
  5. **Two windows:** F does steps 1-3 on the same farmer after H closes the screen. The emeralds and crops fly from F's eye in F's direction, and H sees them land in front of F. **Screenshot 5**, both windows.

### Drags over the payment slots

Commit: "Minecraft trading screen shares a dragged stack over its payment slots", the one after `89e5769b`.

- **Cause:** the HUD already lets a drag cover the two payment slots (`droppable` lists them for the trading screen) and sends `McClick::Spread`, but `merchant_op` had no arm for it and `merchant_local` refuses a spread that touches a payment, so the drag did nothing.
- **Vanilla 26.3** (javap, `AbstractContainerMenu.doClick`, QUICK_CRAFT): the payment slots take drags (plain `Slot`s: `mayPlace` true, `canDragTo` not overridden); the result does not (`MerchantResultSlot.mayPlace` false). Each slot ends with `min(stack max, slot max, existing + place count)`, the place count being `floor(count / slots)` for a left drag and 1 for a right drag, and the cursor keeps the rest. Each `setByPlayer` on a payment re-reads the offers. Vanilla keeps the dragged slots in a `HashSet`, so their order is not kept either.
- **Changed:**
  - `merchant.rs`: `MerchantMenu::spread(players, payments, right, inventory, merchant)` hands the inventory slots (storage only, below 36) and the two payments to `Inventory::distribute_limited`, the way `Containers::spread` does, with a payment capacity of min(99, the stack's max). Each payment that changed is set with `set_payment_by_player`, which re-reads the offers.
  - `world/merchants.rs`: `EntityWorld::merchant_spread`.
  - `server.rs`: `MerchantOp::Spread { players: u64, payments: u8, right }` next to `Throw`, with bit masks so `MerchantOp` stays `Copy`; `Server::merchant` unpacks them.
  - `menus.rs`: `merchant_op` maps a `Spread` that touches `Container(0)` or `Container(1)` to it. A spread over inventory slots only stays local as before; `keeps_gun` still refuses a carried gun.
- **Tests:**
  - `minecraftoss-entities` `merchant::tests::a_drag_shares_the_carried_stack_over_the_payments_and_the_inventory` (64 wheat left-dragged over the first payment and two inventory slots: 21 each, 1 kept, and the result re-read; a right drag over both payments and the two slots adds 1 each, keeps 6, and the result goes because a trade of one cost wants the second payment empty; 64 over both payments and two slots: 16 each; a payment with carrots and the offhand take none).
  - `minecraft_terrain` `server::tests::a_drag_on_the_trading_screen_spreads_over_the_marked_payments_and_inventory_slots` (the bit masks reach the right slots of a real farmer's screen).
  - `minecraft_survival` `menus::tests::a_villager_s_trading_screen_turns_clicks_into_trades`: a right drag over both payments, an inventory slot and the offhand sends the masks `(1 << 12) | (1 << 40)` and `0b11`.
- **Decisions:**
  - `MerchantOp::Spread` carries bit masks so `MerchantOp` stays `Copy` (the inventory has 45 slots, which fit in a u64). The drag's order is lost, as in vanilla's `HashSet`; the inventory slots are filled before the payments, as for chests and benches.
- **Owner's test steps** (the same world and farmer; close the screen and open it again with empty payment slots, without picking an offer):
  1. **Left drag.** Pick up a full stack (64) of the crop from the inventory. Hold the left button and drag across the first payment slot and one empty inventory slot, then let go: 32 land in each, the cursor is empty, and the emerald shows as the result. **Screenshot 6.**
  2. **Right drag.** Pick up another full stack of the crop and right-drag over both payment slots and the same inventory slot: one more lands in each (33, 1 and 33), and the emerald disappears, because a trade that costs one item wants the second payment slot empty (as in vanilla). **Screenshot 7.**
  3. **Four slots.** Close the screen (the payments go back to the inventory) and open it again. Pick up a full stack and left-drag over both payment slots and two empty inventory slots: 16 land in each of the four and the cursor is empty.
  4. **Two windows:** F does steps 1-3 on the same farmer after H closes the screen; it works the same.

### No gathering on a double click

Commit: "Minecraft trading screen never gathers on a double click, like vanilla", the one after `4d9d80f1`.

- **Cause:** the HUD turned a double click on the trading screen into `McClick::Gather` (or, with Shift, a quick move of every matching stack) like on any other screen. On a payment slot the host dropped it; on an inventory slot `merchant_local` let it through to `grid_action`, which gathered the inventory's matching stacks onto the cursor.
- **Vanilla 26.3** (javap): `MerchantMenu.canTakeItemForPickAll` returns false (`iconst_0; ireturn`). `AbstractContainerScreen.mouseReleased` checks it before both the PICKUP_ALL click and the Shift double-click quick-move loop, so on the trading screen neither happens on any slot, and the second release falls through to an ordinary click: the stack the first click picked up is put back.
- **Changed:**
  - `crates/hud/src/minecraft_clicks.rs`: the `Menu` trait has `picks_all()` (true by default). `Clicks::release` checks it next to `takes_all`, so on a menu that never picks all, the double click's second release is a plain click.
  - `crates/hud/src/minecraft_inventory.rs`: `MenuView::picks_all` is false on `McScreen::Merchant`.
  - `crates/minecraft_survival/src/menus.rs`: `merchant_action` does not run a `Gather` locally, so a stray one from an older friend's client does nothing either.
- **Tests:**
  - `hud` `minecraft_clicks::tests::a_double_click_on_a_menu_that_never_picks_all_puts_the_stack_back` (the second release sends a plain `Slot` click and no `Gather`; a Shift double click sends no extra quick moves).
  - `minecraft_survival` `menus::tests::a_villager_s_trading_screen_turns_clicks_into_trades`: a `Gather` on an inventory slot sends nothing and leaves the cursor and the other wheat stack alone.
- **Decisions:**
  - A double click on the trading screen gathers nothing on any slot, because vanilla's `MerchantMenu.canTakeItemForPickAll` is false. The owner's "double-click does nothing" was vanilla; what we change is the gathering we used to do on inventory slots.
  - The refusal is in `merchant_action`, not in `merchant_local` as the brief planned: `mount_action` also uses `merchant_local` for the horse screen's own inventory slots, and a horse screen does gather in vanilla (`HorseInventoryMenu` keeps the default `canTakeItemForPickAll`).
- **Owner's test steps** (the same world and farmer):
  1. **Trading screen.** Open the farmer. Put two small stacks of the same crop in the inventory (split one with a right click). Click one of them to pick it up, then click the same slot again quickly: the stack is put back, and the other stack is **not** gathered onto the cursor. **Screenshot 8.** Shift-double-click a crop stack: only that stack moves between the inventory and the hotbar.
  2. **Normal inventory.** Close the screen and press E. Do the same double click: here the other stack is gathered onto the cursor (unchanged).
  3. **Two windows:** F does step 1 on the same farmer after H closes the screen; nothing is gathered there either.

**Kit for the whole `trading-keys` group** (the same as for the first commit; 18 backpack slots):

```
IW4L_MINECRAFT_KIT=minecraft:wheat*192,minecraft:potato*192,minecraft:carrot*192,minecraft:beetroot*192,minecraft:emerald*64,minecraft:villager_spawn_egg*4,minecraft:composter,minecraft:oak_fence*32,minecraft:torch*32,minecraft:bread*16
```
