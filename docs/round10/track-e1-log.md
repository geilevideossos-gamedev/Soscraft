# Track E1 log: screens, visuals and sounds

Branch `track-e1`, worktree `C:\Dev\mc-wt\track-e1`, target dir `D:\iw4l-wt\track-e1`. Brief: `track-e.md` (overview and the E1 sections). Base: `ed23b000`.

A commit can't name its own hash, so each entry is written with its commit and names the commit before it; the last entry's hash is in `git log` under its subject.

## Protocol

- "Minecraft taming shows hearts, and a failed try or a bucking horse shows smoke" (after `280f5fe9`): `EntitySnapshot` has a new field `mob_puffs`. The snapshot rides in the state frame's `entities` blob and in `ToClient::Bursts`, so host and friend must run the same build. No message or codec tag changed.
- "Friends see their mount's hearts" (after `ac0b573a`): `minecraft_net::PlayerView` (sent as `ToClient::Player`) has a new last field `vehicle_health: Option<[f32; 2]>`. An old friend build can't read the new view, so the release step must bump `PROTOCOL`. No message index or codec tag changed.

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

## E-effects. Effects listed beside the inventory

### The list's frames and the effect names

Commit: "Minecraft inventory loads the effect list frames and effect names", the one after `12214823`.

- **Cause** (as in the brief, re-checked at `12214823`): the GUI atlas (`crates/render_anim/src/minecraft_gui.rs`) loaded no sprite from `container/inventory/`, so the list's frames were missing, and `TEXT_PREFIXES` dropped every `effect.` and `enchantment.level.` text, so the HUD had no effect names, no "∞" and no level numerals.
- **Vanilla 26.3** (the 26.3 pack): `textures/gui/sprites/container/inventory/` holds only `effect_background` and `effect_background_ambient`, both 32 x 32, nine-slice with a border of 4 (their `.png.mcmeta`). `EffectsInInventory` draws them, the names are `effect.minecraft.<id>` plus `enchantment.level.<n>`, and an endless effect shows `effect.duration.infinite` ("∞", which the default font's `include/default.json` has).
- **Changed:** `SPRITE_FOLDERS` has `container/inventory/` (18 folders) next to the creative folder, and `TEXT_PREFIXES` keeps `effect.` and `enchantment.level.` (16 prefixes). Nothing draws them yet; the next commit does.
- **Tests:** `render_anim` `minecraft_gui::tests::every_mob_effect_icon_and_the_effect_frames_load` also checks that both frames load 32 x 32 as nine-slices with a border of 4, that the effect name, "∞" and "II" are kept while an enchantment name is not, and that the font has "∞".
- **Decisions:** none.
- **Owner's test steps:** none on their own: this commit only loads textures and texts. The steps come with the next commit.

### The list beside the inventory

Commit: "Minecraft inventory lists the active effects beside it", the one after `c274aa21`.

- **Cause** (as in the brief, re-checked at `c274aa21`): `draw_hud` (`crates/hud/src/minecraft_hotbar.rs`) always drew the top-right effect icons, and the screen pass in `crates/hud/src/minecraft_inventory.rs` never drew an effect list.
- **Vanilla 26.3** (javap: `EffectsInInventory`, `InventoryScreen`, `CreativeModeInventoryScreen`, `Hud.extractEffects`, `MobEffectUtil.formatDuration`, `StringUtil.formatTickDuration`, `ComponentRenderUtils.clipText`, `GuiGraphicsExtractor`):
  - Only the inventory and the creative screen own the list. `canSeeEffects` is `width - (leftPos + imageWidth + 2) >= 32`, and the HUD skips its icons exactly when the open screen's `showsActiveEffects()` is that.
  - Boxes start at `x0 = leftPos + imageWidth + 2`, `y = topPos`, in natural order (`Ordering.natural()`), `33` apart, or `132 / (n - 1)` apart with more than 5. Width: `min(maxWidth, max(32 + name + 7, 32 + time + 7))`, `maxWidth` being `room - 7` from a room of 120 up and 32 below.
  - Each box: the 32-high nine-slice frame (ambient or not), then, when `w - 39 > 0`, the name at `(x0 + 32, y + 7)` in white and the time 9 lower in `0xFF808080`, both with a shadow (the five-argument `text` passes `true`), then the 18 x 18 icon at `(x0 + 7, y + 7)` with no blinking.
  - The name is cut to `room - width("...")` and gets "..." when it is wider than the text room. A cut name, or a box with no text room, shows a tooltip of the name and time when the mouse is in `[x0, x0 + w] x [y, y + step]`, edges included. The first tooltip set in a frame wins (`setTooltipForNextFrameInternal`).
  - Name: `effect.<namespace>.<id>`, plus a space and `enchantment.level.<amplifier + 1>` for amplifiers 1 to 9. Time: "∞" for an endless effect, else `floor(ticks / tickrate)` seconds as `mm:ss`, or `hh:mm:ss` from one hour.
  - `InventoryScreen` draws the list before handing over to `AbstractRecipeBookScreen`, so the list also shows when a narrow window's recipe book covers the inventory.
- **Changed:**
  - `crates/hud/src/minecraft_effects.rs`: `list_spot(ui, gui)` gives `[leftPos, imageWidth, topPos]` for an open inventory or creative screen, from the same layout and `minecraft_recipe_book::origin` as the screen pass, so the recipe book's shift is in it. `hides_icons(ui, gui)` is true when that spot leaves 32 or more. `listed(gui, effects, spot, width)` builds the boxes (`Listed`: icon, frame, place, width, full name, time, the drawn label, the hover rectangle) by the rules above, `draw_listed` draws them and `listed_tooltip` gives the hovered box's name and time.
  - `minecraft_hotbar.rs` `draw_hud`: the top-right icons are skipped when `hides_icons`.
  - `minecraft_inventory.rs`: the screen pass draws the list right after the dark gradient, and the effect tooltip comes last in the tooltip chain, with the title gap.
- **Tests** (`hud` `minecraft_effects::tests`):
  - `the_list_sits_right_of_the_inventory_in_natural_order_33_apart`: at width 480 the inventory's spot is `[152, 176, 43]`; four effects in natural order at x 330, y 43, 76, 109, 142, with widths 69, 129, 111 and 143; "Hero of the Village" cut to "Hero of the Vi..." with a hover box; an ambient effect gets the ambient frame; no effects give nothing.
  - `more_than_five_effects_pack_into_132_pixels`: 7 effects 22 apart, 6 effects 26 apart, 5 effects 33 apart.
  - `a_narrow_room_gives_icon_boxes_with_a_tooltip_and_too_little_gives_nothing`: with the recipe book open at 480 the room is 73, so the boxes are 32 wide with no text and a tooltip inside `[407, 439]`; the shared edge row goes to the upper box; a room of 31 (width 396) gives nothing and 32 (width 398) gives icon boxes.
  - `names_carry_levels_two_to_ten_and_times_count_twenty_ticks_a_second`: "Regeneration II", "Bad Omen V", no level for amplifiers 0 and 10, the key for a missing level text; "00:05" (100 and 119 ticks), "08:00", "59:59", "01:40:00", "∞".
  - `a_long_name_is_cut_to_fit_with_three_dots`.
  - `the_hud_icons_hide_exactly_when_the_list_has_room`: for the inventory with and without the book (480, 396, 398, and the narrow 378 where the book covers it), the creative inventory and search tabs, a chest, a crafting table and a closed screen, `hides_icons` is true exactly when `listed` gives a list; the creative spot at 480 is `[142, 195, 58]`.
  - `a_listed_effect_draws_its_frame_text_and_icon_in_vanilla_order`: the frame first (330, 43 to 399, 75), the white name and its shadow, the grey time 9 lower, the icon last at (337, 50); a box with no text room draws only the frame and the icon.
- **Decisions:**
  - The list counts 20 ticks a second, as the brief decided. Vanilla divides by the `/tick rate`, which the HUD does not know; under `/tick rate 10` vanilla would show twice the time.
  - The list is drawn after the dark gradient, not before it as the brief said. Vanilla draws the gradient and the screen's background in a lower layer and the list in the screen's own pass, so it is not darkened.
  - The effect tooltip is the last one in our tooltip chain. Vanilla's list sets its tooltip first, but no other tooltip of these screens can come from the area right of the screen, so nothing changes. Between two hover boxes the upper box wins, as in vanilla.
  - The list also shows on a narrow window with the recipe book covering the inventory, as in vanilla.
  - The tooltip's two lines are white and keep the usual 2-pixel gap after the first line, like every vanilla tooltip of more than one line.
  - The HUD icons hide whenever the open screen has room for the list, even with no effects, as vanilla's `showsActiveEffects` does; with no effects there is nothing to hide anyway.
- **Owner's test steps** (survival, a new world, by day on open grass at least 60 blocks from any village, the kit below, default GUI scale, recipe book closed unless a step says otherwise):
  1. **The list.** Drink a night vision potion and eat a golden apple. Run `/effect give @s minecraft:hero_of_the_village 600 0`. The icons show at the top right. Open the inventory (E). To the right of the inventory is a column of grey boxes, each with an icon, a white name and a grey time. From the top: "Regeneration II" (a few seconds, while it lasts), "Absorption 01:5x", "Night Vision 07:5x", "Hero of the Village 09:5x". The icons at the top right are gone. **Screenshot 1.**
  2. **The recipe book.** Click the recipe book button. The inventory moves right and the boxes shrink to icons only. Hover one: a tooltip shows its name and time. **Screenshot 2.** Close the book: the full boxes come back.
  3. **Many effects.** Close the inventory. Run `/effect give @s minecraft:speed 60`, `/effect give @s minecraft:haste 60 1`, `/effect give @s minecraft:strength 60` and `/effect give @s minecraft:luck infinite`. Open the inventory. With 7 or more effects the boxes overlap, packed into the same height. Luck shows "∞" at the bottom and Haste shows "Haste II". **Screenshot 3.**
  4. **Other screens.** Place the chest and open it: the icons stay at the top right and there is no list (vanilla). Run `/gamemode creative` and press E: the list sits beside the creative screen, on every tab. **Screenshot 4.** Run `/gamemode survival`.
  5. **Clearing.** Drink a milk bucket and open the inventory: no boxes. Close it: no icons either.
  6. **Two windows** (host H, friend F, `scripts\mp-local.ps1` or over the relay, both with the kit): F drinks a night vision potion and opens the inventory: the list shows beside it and F's top-right icons hide. **Screenshot 5**, F's window. H's own screen is unchanged.
- **Kit** (6 backpack slots):

```
IW4L_MINECRAFT_KIT=minecraft:golden_apple*8,minecraft:potion@long_night_vision,minecraft:potion@long_night_vision,minecraft:chest,minecraft:milk_bucket,minecraft:milk_bucket
```

## E-omen. Omen sounds from /effect give, and the ominous bottle tooltip

### Omens given by command play their sound

Commit: "Minecraft omens given by command play their sound", the one after `796a9e13`.

- **Cause** (as in the brief, re-checked at `796a9e13`): `effect_give` (`crates/minecraft_survival/src/commands.rs`) called `status.add_effect` and played nothing. Only the ominous bottle's drink had an omen sound, from its own fresh-omen check in `Survival::drink_ominous_bottle`.
- **Vanilla 26.3** (javap): `MobEffects` registers `bad_omen`, `trial_omen` and `raid_omen` with `withSoundOnAdded(SoundEvents.APPLY_EFFECT_*)`, which are `event.mob_effect.bad_omen`, `.trial_omen` and `.raid_omen`. `LivingEntity.addEffect` calls `onEffectAdded` only when the effect was not there before; an upgrade goes through `onEffectUpdated`, which plays nothing. `MobEffect.onEffectAdded` plays the sound at the entity with volume 1 and pitch 1 for everyone near (`playSound(null, ...)`).
- **Changed:**
  - `crates/minecraft_survival/src/survival.rs`: `Survival::give_effect(kind, ticks, amplifier)` adds the effect and, when it was absent before and present after, queues the kind's added sound (`added_sound`: the three omens). It returns whether the effect changed, as `/effect give` counted before. `take_effect_sounds()` hands the queue over. `drink_ominous_bottle` now gives Bad Omen through `give_effect` and moves the sound it queued into its `heard` list, so the fresh-omen rule is in one place and the drink is still heard once.
  - `commands.rs` `effect_give`: uses `give_effect`.
  - `crates/minecraft_survival/src/actions.rs` `act()`: drains the player's queued effect sounds into `effects` as sounds at the player (no position, volume 1, pitch 1), the drink's route, so they are heard wherever a drink's sounds are: by the host for the host's own, and for a friend's at the friend's eye.
  - The raid omen of a Bad Omen turning in a village (`turn_bad_omen`, `render_anim/src/minecraft_players.rs`) is unchanged.
- **Tests:**
  - `minecraft_survival` `commands::tests::a_fresh_omen_given_by_command_queues_its_sound_once_for_its_player`: Bad Omen, the same Bad Omen again, Bad Omen at a higher level, Speed, Trial Omen and Raid Omen queue exactly the three omen sounds in order, and the queue is empty once taken; `/effect give Alex` queues the sound on the friend, not the host; a cleared omen sounds again.
  - `minecraft_survival` `actions::tests::an_omen_given_since_the_last_frame_is_heard_once_from_the_player`: `act()` gives one trial omen sound at the player with volume 1 and pitch 1, and none the frame after.
  - `minecraft_survival` `survival::tests::an_ominous_bottle_is_drunk_away_and_gives_bad_omen_at_its_amplifier` still passes and now also checks that the drink leaves no queued sound.
- **Decisions:**
  - `give_effect` returns whether the effect changed (vanilla's `addEffect` result), not whether it was fresh; the sound is the only thing tied to a fresh effect. Track D's Bad-to-Trial Omen turn gets the trial omen sound by calling it.
  - The sound is played at the player's eye like the drink's sounds, not at the feet as vanilla does; for a sound this close to the listener the difference is not audible.
- **Owner's test steps** (survival, a new world, by day on open grass at least 60 blocks from any village, no kit needed):
  1. **Bad Omen.** Run `/effect clear @s`, then `/effect give @s minecraft:bad_omen 60`: an eerie omen sound plays and the Bad Omen icon shows. Run the same command again: no sound, because the omen is already there. Run `/effect give @s minecraft:bad_omen 60 2`: the omen gets stronger, still no sound.
  2. **Trial and Raid Omen.** Run `/effect give @s minecraft:trial_omen 60`: the trial omen sound. Run `/effect give @s minecraft:raid_omen 5`: the raid omen sound. Run `/effect clear @s`.
  3. **Speed.** Run `/effect give @s minecraft:speed 60`: no sound.
  4. **The bottle.** Drink an ominous bottle: the omen sound plays once (not twice), then the bottle's dispose sound. Drink a milk bucket.
  5. **Two windows** (host H, friend F, standing next to each other away from any village): H runs `/effect give <F's name> minecraft:bad_omen 30`. F hears the omen sound and H hears it too. H runs `/effect clear <F's name>`.
- **Kit** (only for step 4):

```
IW4L_MINECRAFT_KIT=minecraft:ominous_bottle*2,minecraft:milk_bucket
```

### The ominous bottle's Bad Omen line

Commit: "Minecraft ominous bottles show their Bad Omen line", the one after `f3bd9636`.

- **Cause** (as in the brief, re-checked at `f3bd9636`): `McStack` (`crates/frame/src/minecraft_ui.rs`) had no effect lines, `convert` in `crates/render_anim/src/minecraft_inventory.rs` never read `minecraft:ominous_bottle_amplifier`, and `tooltip_lines` in `crates/hud/src/minecraft_inventory.rs` had nothing to draw under the name.
- **Vanilla 26.3** (javap): `Items.OMINOUS_BOTTLE` has the default component `OminousBottleAmplifier(0)`. `OminousBottleAmplifier.addToTooltip` builds `MobEffectInstance(BAD_OMEN, 120000, amplifier, false, false, true)` and hands it to `PotionContents.addPotionTooltip(list, consumer, 1.0, tickRate)`. `getPotionDescription` is the effect's name, wrapped in `potion.withAmplifier` ("%s %s") with `potion.potency.<amplifier>` when the amplifier is above 0. The time is added with `potion.withDuration` ("%s (%s)") unless `endsWithin(20)` (an effect of 20 ticks or less, not endless), formatted by `MobEffectUtil.formatDuration` as `mm:ss`, or `hh:mm:ss` from an hour. The colour is the category's `getTooltipFormatting`: Bad Omen is NEUTRAL, which is BLUE (0x5555FF); HARMFUL would be RED. Bad Omen has no attribute modifiers, so there is no "When Applied:" block.
- **Changed:**
  - `frame/src/minecraft_ui.rs`: `McStack.effects: Vec<(String, bool)>` (the line, and whether it is beneficial or neutral), and `McStack` derives `Default`. The test literals in `frame` and `hud` end with `..Default::default()` instead of naming the new field, so later fields (banner lines, dyed armour) don't touch them again. The two literals that already copy another stack (`minecraft_screens.rs` and `minecraft_merchant.rs`) needed nothing.
  - `render_anim/src/minecraft_inventory.rs`: `convert` fills `effects` from `effect_lines`, which gives an ominous bottle its Bad Omen line at `ominous_amplifier` (0 for a bottle with no components) and 120000 ticks. `effect_line(language, effect, amplifier, ticks)` builds a line by the rules above, with the numeral as a fallback for a missing potency text, so potions can use it later.
  - `hud/src/minecraft_inventory.rs` `tooltip_lines`: the effect lines come right after the name, blue for beneficial or neutral and red for harmful. `minecraft_gui.rs` has `BLUE` (0xFF5555FF) next to `RED`.
- **Tests:**
  - `render_anim` `minecraft_inventory::tests::an_ominous_bottle_shows_its_bad_omen_with_its_level_and_time`: a plain bottle and amplifier 0 give `("Bad Omen (01:40:00)", true)`, amplifier 4 gives `("Bad Omen V (01:40:00)", true)`, amplifier 1 with no potency text gives "Bad Omen II (01:40:00)", a potion gives no line, 300 ticks give "(00:15)", 20 ticks give no time, and a missing name shows its key.
  - `hud` `minecraft_inventory::tests::an_ominous_bottle_tooltip_shows_its_omen_in_blue_under_the_name`: the lines are the white name, then the omen line in blue; a harmful line is red.
- **Decisions:**
  - The time counts 20 ticks a second, as the effect list does (E-effects decision): vanilla divides by the `/tick rate`.
  - The bottle's name stays white. Vanilla's ominous bottle is UNCOMMON, so its name is yellow there, but our tooltips draw no rarity colours yet for any item; that is a separate change.
- **Owner's test steps** (survival, a new world, the kit below):
  1. **The plain bottle.** Open the inventory and hover the stack of two ominous bottles: "Ominous Bottle", and under it a blue "Bad Omen (01:40:00)". **Screenshot 1.**
  2. **The strong bottle.** Hover the single bottle: "Bad Omen V (01:40:00)" in blue. **Screenshot 2.**
  3. **Creative.** Run `/gamemode creative`, open the inventory, search "ominous" and hover the bottle: "Bad Omen (01:40:00)". Run `/gamemode survival`.
  4. **Two windows** (host H, friend F, both with the kit): F hovers both bottles in F's inventory and sees the same two lines.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:ominous_bottle*2,minecraft:ominous_bottle[ominous_bottle_amplifier=4]
```

## E-horses. Horse falls, the gallop, vehicle hearts and dyed leather horse armour

### Horses fall like vanilla horses

Commit: "Minecraft horses fall like vanilla horses", the one after `7c253993`.

- **Cause** (as in the brief, re-checked at `7c253993`): the ridden horse (`equines.rs` `ride_tick`), the grazing or rearing horse and the horse on its own AI (`world.rs`, the two `travel_air` landings of the cow tick) all called the generic `living::fall_damage`: safe distance 3, multiplier 1 and the generic `small_fall`/`big_fall` sound. `horse::SAFE_FALL_DISTANCE` (6) and `FALL_DAMAGE_MULTIPLIER` (0.5) were never used, so a horse was hurt by a 4-block drop and by a strong horse's full jump on flat ground.
- **Vanilla 26.3** (javap): `AbstractHorse.causeFallDamage(d, mult, src)` plays `entity.horse.land` (a baby of any horse kind: `entity.baby_horse.land`) at volume 0.4 and pitch 1 when `d > 1`, then `calculateFallDamage`; at 0 or less it stops there, otherwise `hurt`, `propagateFallToPassengers(d, mult, src)` and `playBlockFallSound`. No generic fall sound. `LivingEntity.calculateFallDamage` is `floor((d + 1e-6 - SAFE_FALL_DISTANCE) * mult * FALL_DAMAGE_MULTIPLIER)`. `Llama.causeFallDamage` has no land sound and hurts and propagates only at `d >= 6`. `d` is the distance after the block's reduction (a bed halves it) and `mult` the block's multiplier (hay 0.2, slime 0).
- **Changed:**
  - `third_party/minecraftoss/entities/src/world/living.rs`: `landed_block(body, world)` gives the landed block's `(fall_reduction, fall_multiplier)` and `block_fall_sound(body, world)` the block's fall sound at half volume and three quarters pitch. `fall_damage` uses both, unchanged in what it does.
  - `world/equines.rs`: `horse_fall(horse, body, world, fallen, baby, voices)` follows `AbstractHorse.causeFallDamage` (no land sound for llamas) and records `pending_fall = Some((distance, multiplier))` when the horse is hurt, for its riders (next commit). `cow_fall(Option<&mut HorseState>, ...)` uses it for horses and the plain `fall_damage` for cows. `ride_tick` calls `horse_fall`.
  - `world.rs`: the two cow-tick landings call `equines::cow_fall` (one line each).
  - `horse.rs`: `HorseState.pending_fall: Option<(f64, f32)>`, `None` by default and never saved.
- **Tests** (`minecraftoss-entities` `world::equines::tests`, on a test ground with a chosen block at y 63 and a stone fall sound):
  - `a_ridden_horse_falling_ten_blocks_takes_two_and_hands_its_rider_the_fall`: 8, 10 and 20 blocks onto stone take 1, 2 and 7 and leave `pending_fall` at that distance with multiplier 1; 20 blocks onto a bed count as 10 and take 2.
  - `a_horse_landing_from_seven_blocks_takes_nothing_and_hands_nothing_on`: 4 and 7 blocks take nothing and leave no pending fall, but play `entity.horse.land` once at 0.4 and no block fall sound.
  - `a_horse_landing_on_hay_spares_itself_and_its_rider`: 10 blocks onto hay or slime take nothing; 20 onto hay take 1 with multiplier 0.2.
  - `a_full_jump_on_flat_ground_never_hurts_a_strong_horse`: three full jumps at jump strength 1.0 (peaks above 5 blocks) leave it at full health.
  - `a_horse_landing_plays_the_horse_land_sound_but_no_generic_fall`: `entity.horse.land` (0.4, 1) and `block.stone.fall` (0.5, 0.75), no `*_fall` event; an unridden foal plays `entity.baby_horse.land` and takes 2 from 10 blocks; a cow still takes 7 with `entity.generic.big_fall`.
  - `a_llama_landing_makes_no_land_sound`: no `.land` event, the block fall sound, and 2 damage from 10 blocks.
  - All 410 `minecraftoss-entities` tests pass.
- **Decisions:**
  - The brief's one helper is two (`landed_block` and `block_fall_sound`), so the block's fall sound is looked up only when a horse is hurt; `fall_damage` uses the same two instead of its own copy.
  - `horse_fall` returns at once for a fall of 0, as vanilla calls `fallOn` only when `fallDistance > 0` (our bodies report a landing every tick on the ground).
  - Llamas skip vanilla's extra `d >= 6` check: damage above 0 already needs `d >= 8` (multipliers are at most 1), so it can never matter.
  - A dying horse still takes the generic fall sound through `dying_travel` (its hurt fails anyway); only the three living landings changed, as the brief planned.
  - The rider takes no share yet: the next commit hands `pending_fall` to the riders.
- **Owner's test steps** (survival, a new world, by day on flat ground, the kit below; the rider's own damage comes with the next commit, and the mount's hearts with group `vehicle-hearts`, so here the horse's hurt shows as a red flash and its hurt sound):
  1. **A ten-block drop.** Run `/fill ~3 ~ ~-3 ~9 ~9 ~3 stone` (a stone block 10 high), then `/tp @s ~6 ~10 ~` and `/summon minecraft:horse ~ ~ ~2 {Tame:1b}`. Saddle the horse and ride straight off the edge. On landing you hear a heavy hoof thud and the stone's fall sound, and the horse flashes red with its hurt sound. You hear no "oof" of the generic fall. **Screenshot 1**, just after landing.
  2. **A seven-block drop.** At a fresh spot run `/fill ~3 ~ ~-3 ~9 ~6 ~3 stone` (7 high), `/tp @s ~6 ~7 ~` and summon and saddle a new tame horse the same way. Ride off the edge: the thud plays, but the horse does not flash red.
  3. **Hay.** At the foot of the 10-high block, where you landed in step 1, run `/fill ~-1 ~-1 ~-1 ~1 ~-1 ~1 hay_block`. Go back up as in step 1 and ride off at the same place onto the hay: the thud plays, no red flash.
  4. **Jumps.** On flat grass, hold Space until the jump bar is full and let go, ten times: the horse never flashes red (before this commit a strong horse hurt itself).
  5. **Dropped from the air.** On flat grass, run `/summon minecraft:horse ~3 ~10 ~`: it falls 10 blocks, thuds and flashes red. Run `/summon minecraft:llama ~-3 ~10 ~`: it falls 10 blocks and flashes red, with no hoof thud. Run `/summon minecraft:cow ~ ~10 ~3`: the cow still lands with the generic big "oof" fall sound.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:saddle,minecraft:saddle,minecraft:saddle,minecraft:hay_block*16,minecraft:cooked_beef*16
```

### Riders share their horse's fall

Commit: "Minecraft riders share their horse's fall", the one after `704004c8`.

- **Cause** (as in the brief, re-checked at `704004c8`): a seated player's own fall is switched off (`minecraft_riding.rs` clears `steps.air_peak` every tick, as vanilla's `rideTick` resets the rider's fall distance), and nothing handed the horse's landing on to its riders, so a rider never took fall damage.
- **Vanilla 26.3** (javap): `AbstractHorse.causeFallDamage` calls `propagateFallToPassengers(d, mult, src)` only after the horse itself was hurt; `Entity.propagateFallToPassengers` calls `causeFallDamage(d, mult, src)` on every passenger with the **horse's** distance and block multiplier. `Player.causeFallDamage` takes nothing while `mayfly` (creative and spectator), then `LivingEntity.causeFallDamage`: `floor((d + 1e-6 - 3) * mult)`, the fall sound (`big_fall` above 4) and the hurt. The rider's own fall distance is 0, so a death is `death.attack.fall` ("hit the ground too hard").
- **Changed:**
  - `third_party/minecraftoss/entities/src/world/riding.rs`: `EntityWorld::take_rider_falls() -> Vec<(player, distance, multiplier)>`, next to `take_player_dismounts`: it drains every horse's `pending_fall` and gives one entry per player passenger. Mob riders are skipped.
  - `crates/minecraft_terrain/src/server.rs`: `Output.rider_falls` beside `dismounts`, filled in `run_tick` right after the vehicle losses, so a frozen tick's ridden horses count too.
  - `crates/minecraft_survival/src/entities.rs`: `route` hands each fall to its player's `PlayerOutputs.rider_falls`.
  - `crates/minecraft_survival/src/survival.rs`: `Survival::rider_fall(distance, multiplier) -> i32` with `fall_damage(distance, multiplier)`. `fall` and `rider_fall` share `hurt_by_fall`: the fall damage gamerule, invulnerability (creative and spectator), damage-type immunity and Feather Falling, then `DamageCause::Fall` recorded with the given fall distance (0 for a rider).
  - `crates/render_anim/src/minecraft_players.rs` `struck()`: each rider fall of a living player is applied and, when it hurts, plays `fall_sound(damage)` and `entity.player.hurt`, as a landing on foot does. `struck()` runs for the host and every friend, on the host.
- **Tests:**
  - `minecraftoss-entities` `world::equines::tests`: `a_ridden_horse_falling_ten_blocks_takes_two_and_hands_its_rider_the_fall` now takes the fall with `take_rider_falls` (`(0, 10.0, 1.0)` for 10 blocks, `(0, 10.0, 1.0)` for 20 onto a bed, once only, and the pending fall is gone after); a horse with no rider hands nothing on. The 4- and 7-block, hay, slime and full-jump tests check that no rider fall comes out; 20 blocks onto hay hand the rider multiplier 0.2; the llama hands its fall on too. All 410 tests pass.
  - `minecraft_survival` `survival::tests::a_rider_takes_the_horse_s_fall_and_hits_the_ground_too_hard`: 10 blocks take 7; 20 onto hay take 3; a fatal one reads "hit the ground too hard"; 3 blocks, 7.9 onto hay and slime take 0; creative and the gamerule off take 0; Feather Falling IV softens 7 as on foot.
  - `minecraft_survival` `entities::tests::rider_falls_reach_the_host_and_the_friend`: each fall reaches only its own player's outputs.
  - `minecraft_survival` (766), `minecraft_terrain` (1409) and `render_anim` (213) pass.
- **Decisions:**
  - The test the brief named `a_rider_takes_the_horse_s_fall_from_three_blocks_up_and_hits_the_ground_too_hard` is `a_rider_takes_the_horse_s_fall_and_hits_the_ground_too_hard`: nothing in it is about three blocks up.
  - The rider hears no block fall sound: vanilla's `playBlockFallSound` looks 0.2 below the rider's own feet, which on a horse is the air above the horse's feet.
  - A wind charge's impulse does not shorten a rider's share (vanilla would clip it at the rider's impulse height); a seated rider almost never has one.
  - No `FALL_ONE_CM` statistic, as for falls on foot (the fork keeps no statistics).
  - Mob riders take no share yet (the brief's "Left out": no generic "hurt mob by id" before D0's registry).
- **Owner's test steps** (survival, a new world, by day on flat ground, the kit below; full health and no golden apple's extra hearts before each drop; the mount's own hearts on the HUD come with group `vehicle-hearts`):
  1. **Ten blocks.** Run `/fill ~3 ~ ~-3 ~9 ~9 ~3 stone` (a stone block 10 high), then `/tp @s ~6 ~10 ~` and `/summon minecraft:horse ~ ~ ~2 {Tame:1b}`. Saddle it, ride it and ride straight off the edge. On landing you hear the horse's heavy thud and your own big fall sound and hurt, and you lose 3.5 hearts (7 damage). The horse flashes red. **Screenshot 1**, the hearts just after landing.
  2. **Seven blocks.** At a fresh spot run `/fill ~3 ~ ~-3 ~9 ~6 ~3 stone` (7 high), `/tp @s ~6 ~7 ~`, summon and saddle a new tame horse and ride off the edge: the thud plays, but you lose no hearts. **Screenshot 2.**
  3. **Hay.** Go back to the foot of the 10-high block where you landed in step 1 and run `/fill ~-1 ~-1 ~-1 ~1 ~-1 ~1 hay_block`. Go back up as in step 1 and ride off at the same place onto the hay: no hearts lost.
  4. **Jumps.** On flat grass, jump with a full jump bar ten times: you never lose a heart.
  5. **Creative.** Run `/gamemode creative`, go back up the 10-high block on a horse and ride off: no damage. Run `/gamemode survival`.
  6. **Two windows** (host H, friend F, `scripts\mp-local.ps1` or over the relay, both with the kit): F builds a 10-high block as in step 1, summons a tame horse, saddles it and rides off while H watches from the ground. F loses 3.5 hearts and hears the fall; H hears F's horse thud. **Screenshot 3**, both windows.
- **Kit** (the whole `horse-fall` group; 5 backpack slots):

```
IW4L_MINECRAFT_KIT=minecraft:saddle,minecraft:saddle,minecraft:saddle,minecraft:hay_block*16,minecraft:cooked_beef*16
```

### Riders see their mount's hearts

Commit: "Minecraft riders see their mount's hearts in place of the food bar", the one after `530d931f`.

- **Cause** (as in the brief, re-checked at `530d931f`): the HUD (`crates/hud/src/minecraft_hotbar.rs`) drew hearts, drumsticks and bubbles from `ui.vitals` only. Nothing carried the mount's health: `frame::MinecraftUi` had `jump_bar` but no vehicle field, and neither did `minecraft_survival::player::Player`.
- **Vanilla 26.3** (javap, `net.minecraft.client.gui.Hud`):
  - `getPlayerVehicleWithHealth`: the camera player's vehicle when it is a `LivingEntity`. `Entity.showVehicleHealth` is `this instanceof LivingEntity`, and no vehicle class overrides it (checked: `AbstractHorse`, `Strider`, `Pig`, `Camel`, `HappyGhast`, the nautiluses).
  - `getVehicleMaxHearts`: `(int)(getMaxHealth() + 0.5f) / 2`, at most 30. `getVisibleVehicleHeartRows(n)`: `ceil(n / 10.0)`.
  - `extractVehicleHealth`: `health = ceil(getHealth())`. Rows of up to 10 hearts start at `guiHeight - 39` and go up by 10; heart `i` of a row sits at `x = guiWidth / 2 + 91 - i * 8 - 9`. Each heart draws `hud/heart/vehicle_container`, then `vehicle_full` when `2i + 1 + rowOffset < health` and `vehicle_half` when it equals `health`; `rowOffset` grows by 20 a row. It is called after, and outside, the `canHurtPlayer` check, so a creative rider sees it.
  - `extractPlayerHealth`: when `getVehicleMaxHearts` is not 0 the food bar is not drawn. The air row starts at `guiHeight - 49`, 10 more up when the food bar is drawn, and `getAirBubbleYLine` moves it by `-(rows - 1) * 10`; with no vehicle that is `+10`. So the bubbles stay at `guiHeight - 49` on foot and with one row of mount hearts, and sit just above the top row with two or three.
- **Changed:**
  - `third_party/minecraftoss/entities/src/world/riding.rs`: `EntityWorld::vehicle_health(player) -> Option<(health, max)>`, next to `ride_jump_scale`: the cow body's `health` and `max_health` for the horse family (horses, donkeys, mules, llamas), the strider's `health` and `strider::MAX_HEALTH` for striders, `None` on foot.
  - `crates/minecraft_survival/src/player.rs`: `Player.vehicle_health`, next to `jump_bar`.
  - `crates/render_anim/src/minecraft_players.rs` `struck()`: sets it right after the jump bar, from the player's own level, for the host and every friend.
  - `crates/frame/src/minecraft_ui.rs`: `MinecraftUi.vehicle_health`, next to `jump_bar`; the host's HUD fills it in `render_anim/src/minecraft_world.rs` (none while dead, like the jump bar).
  - `crates/hud/src/minecraft_vitals.rs`: `vehicle_max_hearts`, `vehicle_rows` and `vehicle_hearts(health, max) -> Vec<(column, row, sprite)>` by the rules above.
  - `minecraft_hotbar.rs` `draw_hud`: the mount hearts are drawn after the vitals block, outside its survival-only filter; the drumsticks are skipped while the mount has hearts, and the bubbles move up by `(rows - 1) * 10`.
- **Tests:**
  - `minecraftoss-entities` `world::riding::tests::vehicle_health_names_horse_and_strider_health_and_none_on_foot`: on foot none; a horse of 22 of 26 gives `(22, 26)` and nothing for another player; a llama `(15, 15)`; a strider at 13.5 gives `(13.5, 20)`; a mob rider on a strider gives the player nothing.
  - `hud` `minecraft_vitals::tests`: `a_thirty_health_horse_shows_fifteen_vehicle_hearts_in_two_rows` (10 and 5 hearts, a hurt one at 23 has 11 full and a half at the second heart of the top row; 22 health makes 11 hearts in two rows, 21 still 10 in one, 20 for a strider, 100 capped at 30, under a heart none), `a_fifteen_health_horse_shows_seven` (7 full hearts and no half; 15.4 still 7, 15.5 makes 8), `a_half_heart_shows_vehicle_half` (12.5 and 13 end on a half at the seventh heart, 14 does not, 0 shows ten empty containers).
  - `hud` `minecraft_hotbar::tests::vehicle_hearts_hide_the_drumsticks_and_lift_the_bubbles_a_row_per_extra_row`: on foot 10 drumsticks at y 213 and the bubbles at 203; on a strider (20) no drumsticks, ten full mount hearts at 213 starting at x 322, the bubbles still at 203; on a horse of 25 of 30 two rows (213 and 203), 12 full and one half, the bubbles at 193; in creative the 15 hearts and no food or bubbles; a mount with less than one heart leaves the food bar.
  - `render_anim` `minecraft_gui::tests::the_real_gui_textures_and_fonts_load_into_one_atlas` also checks that the three vehicle heart sprites load.
  - `minecraftoss-entities` (411), `hud` (199), `minecraft_survival` (766), `render_anim` (213) and `frame` pass; `check -p launcher` passes.
- **Decisions:**
  - Players ride only the horse family and striders in the fork, so `vehicle_health` names those two. Any other living vehicle a player could ride later should be added there; vanilla shows the hearts for every living vehicle.
  - The mount hearts are drawn in creative and adventure too (vanilla draws them outside `canHurtPlayer`), but not in spectator, where our hotbar is not drawn at all and nobody rides.
  - A friend's mount hearts are worked out on the host from the friend's own level in `struck()` (next commit sends them).
- **Owner's test steps** (survival, a new world, by day on flat grass, the kit below; mount hearts are the row of hearts on the right, where the food bar was):
  1. **Thirty health, two rows.** Run `/summon minecraft:horse ~ ~ ~2 {Tame:1b,attributes:[{id:"minecraft:max_health",base:30.0d}]}`. Right-click it with a saddle, then ride it. The food bar is gone and 15 full mount hearts show on the right: a row of 10 on the hotbar's level and a row of 5 above it, both lined up at the right end. Your own hearts stay on the left. **Screenshot 1.**
  2. **A hurt horse.** Crouch to dismount and punch the horse once with an empty hand (1 damage), then ride it again: the leftmost heart of the top row is now a half heart. Dismount, right-click the horse with a golden carrot (it heals 4) and ride it again: all 15 hearts are full again. Dismount: the food bar comes back. **Screenshot 2**, while riding the hurt horse.
  3. **Twenty-one and twenty-two.** Run `/summon minecraft:horse ~ ~ ~2 {Tame:1b,attributes:[{id:"minecraft:max_health",base:21.0d}]}`, saddle and ride it: one full row of 10 hearts. Then do the same with `base:22.0d`: 10 hearts and one more above the right end.
  4. **A horse from the egg.** Use a horse egg, right-click it with an empty hand until it stops bucking, saddle and ride it: between 7 and 15 hearts, depending on the horse.
  5. **Creative.** While riding, run `/gamemode creative`: your own hearts go, the mount hearts stay. Run `/gamemode survival`.
  6. **A strider.** Use a strider egg, saddle the strider and ride it holding the warped fungus on a stick: 10 mount hearts. **Screenshot 3.**
  7. **Optional, the bubbles.** Dig a pool 3 deep next to a saddled tame horse, dive until most bubbles are gone, then climb out and ride the 30-health horse at once: for the few seconds while the bubbles refill, they show just above the top row of mount hearts.
- **Kit** (9 backpack slots; saddles do not stack):

```
IW4L_MINECRAFT_KIT=minecraft:horse_spawn_egg*2,minecraft:strider_spawn_egg*2,minecraft:saddle,minecraft:saddle,minecraft:saddle,minecraft:saddle,minecraft:warped_fungus_on_a_stick,minecraft:golden_carrot*16,minecraft:cooked_beef*16
```

### Friends see their mount's hearts

Commit: "Friends see their mount's hearts", the one after `ac0b573a`.

- **Cause:** the last commit worked out every player's mount health on the host (`struck()` runs for the host and each friend), but the friend's HUD is built from `minecraft_net::PlayerView`, which had no field for it, so a friend riding a horse still saw the food bar.
- **Changed:**
  - `crates/minecraft_net/src/messages.rs`: `PlayerView.vehicle_health: Option<[f32; 2]>` (health, max), next to `jump_bar`, `None` by default.
  - `crates/minecraft_survival/src/views.rs` `player_view`: fills it from `Player.vehicle_health`. A change makes a new `sent_key`, so a hurt or healed mount is sent at once.
  - `crates/render_anim/src/minecraft_remote.rs` `show_player`: the friend's `ui.vehicle_health` comes from the view, next to the jump bar, and is cleared while the friend is dead (the hearts are drawn outside the vitals block, so a stale value would otherwise stay on the death screen).
- **Tests:**
  - `minecraft_net` `codec::tests::a_player_view_carries_the_mount_s_health`: `Some([22.0, 26.0])` comes back through `ToClient::Player`, and a default view comes back with `None`. The shared `player_view()` fixture now carries `Some([22.0, 26.0])`, so `every_message_to_the_client_round_trips` covers it too.
  - `minecraft_survival` `views::tests::a_rider_s_view_carries_the_mount_s_health_and_a_change_makes_a_new_key`.
  - `minecraft_net`, `minecraft_survival` (767) and `render_anim` (213) pass; `check -p launcher` passes.
- **Decisions:**
  - No `PROTOCOL` bump in this commit (see `## Protocol`): the release step bumps it once.
  - The pair is `[f32; 2]` on the wire, as the brief planned, and `(f32, f32)` everywhere else.
- **Owner's test steps** (host H and friend F, `scripts\mp-local.ps1` or over the relay, both on the new build and with the kit below, survival, by day on flat grass):
  1. **The friend rides.** F runs `/summon minecraft:horse ~ ~ ~2 {Tame:1b,attributes:[{id:"minecraft:max_health",base:30.0d}]}` (or H runs it next to F), saddles it and rides it. F's food bar is replaced by 15 mount hearts in two rows on the right. **Screenshot 4**, F's window.
  2. **A hurt mount.** H punches F's horse once with an empty hand: in F's window the leftmost heart of the top row turns into a half heart within a moment. F dismounts: F's food bar comes back.
  3. **A strider.** F uses a strider egg, saddles the strider and rides it: 10 mount hearts in F's window.
  4. **Both at once.** H rides a horse from the kit's egg (tame it first) while F rides the 30-health horse: each window shows its own mount's hearts, not the other's. **Screenshot 5**, both windows.
  5. **An old friend build** is refused at `Hello` once the release has bumped `PROTOCOL`.
- **Kit** (the whole `vehicle-hearts` group, the same as for the last commit; 9 backpack slots):

```
IW4L_MINECRAFT_KIT=minecraft:horse_spawn_egg*2,minecraft:strider_spawn_egg*2,minecraft:saddle,minecraft:saddle,minecraft:saddle,minecraft:saddle,minecraft:warped_fungus_on_a_stick,minecraft:golden_carrot*16,minecraft:cooked_beef*16
```

### Ridden horses gallop and breathe

Commit: "Minecraft ridden horses gallop and breathe", the one after `0a6ad1ec`.

- **Cause** (as in the brief, re-checked at `0a6ad1ec`): `equines.rs` `horse_movement_sounds` only ever picked `entity.horse.step` (a foal: `entity.baby_horse.step`), or `entity.horse.step_wood` on a wooden block, and `HorseState` had no gallop counter, so a ridden horse sounded like a walking one.
- **Vanilla 26.3** (javap):
  - `AbstractHorse.playStepSound`: while it carries anyone (`isVehicle`) and `canGallop`, `gallopSoundCounter++`. Above 5 and divisible by 3 it calls `playGallopSound`; at 5 or less it plays `entity.horse.step_wood` at the block's volume × 0.15 and pitch, even on grass; otherwise nothing. Carrying no one, wooden sound types play `step_wood` and others `step` (a foal: `entity.baby_horse.step`).
  - `AbstractHorse.playGallopSound`: `entity.horse.gallop` at volume × 0.15 and the block's pitch. `Horse.playGallopSound` adds `random.nextInt(10) == 0` → `entity.horse.breathe` (a foal: `entity.baby_horse.breathe`) at volume × 0.6.
  - `canGallop` is true in `AbstractHorse` and set false in `AbstractChestedHorse`'s constructor, so donkeys, mules and llamas never gallop. `Donkey` and `Mule` do not override `playStepSound`.
  - `AbstractHorse.tickRidden`: `gallopSoundCounter = 0` whenever the ridden input's `z <= 0` (no forward key, backing up, or rearing).
- **Changed:**
  - `third_party/minecraftoss/entities/src/horse.rs`: `HorseKind::can_gallop()` (horses only), next to `wears_armor`; `HorseState.gallop_counter`, next to the other counters, 0 at first and never saved (vanilla does not save it either).
  - `world/emissions.rs`: `MovementSounds.horse_step` is now a `HorseStep`: `Walk { step, wood }` (the old pair), `Wood`, `Gallop { breathe }` and `Silent`. `play_movement` plays `Wood` as `entity.horse.step_wood` and `Gallop` as `entity.horse.gallop`, both at the block's step volume (already × 0.15) and pitch; a gallop with a breath draws `random.next_int(10)` and on 0 plays it at that volume ÷ 0.15 × 0.6.
  - `world/equines.rs`: `horse_step_sounds(horse, body, baby, carrying)` beside `horse_movement_sounds`. On a step emission of a carrying horse that can gallop it counts and picks `Wood`, `Gallop` or `Silent`; otherwise it gives the walking sounds unchanged. `ride_tick` resets the counter when the ridden input's `z <= 0` and calls it with `carrying` true.
  - `world.rs`: the two cow-tick sound lines (the grazing or rearing horse and the horse on its own AI) take `entity.horse.as_mut()` and call `horse_step_sounds` with `ride.carrying`.
- **Tests** (`minecraftoss-entities` `world::equines::tests`, on a flat test ground with a chosen step sound at volume and pitch 1):
  - `a_ridden_horse_clops_five_times_then_gallops_every_third_step`: holding W for 200 ticks over grass and over wood, the counter goes up by one each step; steps 1-5 are `entity.horse.step_wood` only, steps 6, 9, 12, ... are `entity.horse.gallop` (at most one `entity.horse.breathe` after it), and the steps between are silent.
  - `stopping_resets_the_gallop`: after a gallop, letting go of W leaves the counter at 0; pressing W again starts with five wooden clops; backing up (S) makes every step a wooden clop at counter 1 and never a gallop.
  - `a_ridden_donkey_and_an_unridden_horse_only_step`: only horses can gallop; a ridden donkey steps `entity.horse.step` on grass and `entity.horse.step_wood` on wood and never counts; an unridden horse (and a foal, `entity.baby_horse.step`) never counts and steps as before, wooden on cherry wood.
  - `a_gallop_sometimes_breathes`: a gallop names `entity.horse.breathe` (a foal `entity.baby_horse.breathe`); over 200 gallops with a seeded random, the breath comes exactly when `next_int(10)` gives 0, at 0.48 for a block of volume 0.8 and at the block's pitch, and the gallop at 0.12.
  - All 415 `minecraftoss-entities` tests pass; `check -p launcher` passes.
- **Decisions:**
  - The counting is a new `horse_step_sounds` beside `horse_movement_sounds` instead of a new signature for it: `CowEntity::movement_sounds` (`world.rs`, the fluid splash, which takes `&self`) keeps calling the old one, so that line is untouched.
  - Whether the horse carries anyone is passed in (`ride.carrying` in the cow tick, `true` in `ride_tick`) rather than read from `HorseState.ridden`, which is set at the end of each tick and so is one tick late on mounting and dismounting; vanilla's `isVehicle` is the current state.
  - `Gallop { breathe: Option<..> }`: only `Horse.playGallopSound` breathes, so the skeleton and zombie horses of track M can gallop with `None` (and get `can_gallop` true).
  - The counter counts every step emission, also one where the world knows no step sound (vanilla only skips a liquid block, which never makes a step here); such a step plays nothing.
  - A wild horse bucking its rider counts too (it is a vehicle), and its counter is only reset by a rider's steering, which needs a saddle, as in vanilla.
- **Owner's test steps** (survival, a new world, by day on flat grass, the kit below; these are sounds, so no screenshots: note what you hear for each step):
  1. **The gallop.** Run `/summon minecraft:horse ~ ~ ~2 {Tame:1b}`, right-click it with a saddle and ride it. Hold W across the grass. The first five hoofbeats are hollow wooden clops, then you hear a gallop every third step with quiet steps between, and now and then a snort.
  2. **Stopping.** Let go of W until the horse stands still, then hold W again: the five wooden clops come back before the gallop starts again.
  3. **Backing up.** Hold S: the horse backs up slowly with a wooden clop on every step and never gallops.
  4. **Planks.** Run `/fill ~ ~-1 ~ ~2 ~-1 ~30 oak_planks` while riding (a plank strip in front of you, facing +Z; or lay the kit's planks by hand) and ride along it holding W: the same pattern, five clops, then a gallop every third step.
  5. **On its own.** Dismount (crouch) and step back a few blocks. When the horse wanders by itself you hear its ordinary hoof steps on grass and wooden clops on the planks, never a gallop.
  6. **A donkey.** Run `/summon minecraft:donkey ~ ~ ~2 {Tame:1b}`, saddle it and ride it holding W: ordinary hoof steps only (wooden clops on the planks), no gallop and no snort.
  7. **Two windows** (host H, friend F, `scripts\mp-local.ps1` or over the relay, both on this build): F summons, saddles and rides a tame horse past H, holding W. H hears F's horse clop five times and then gallop.
- **Kit** (3 backpack slots; saddles do not stack):

```
IW4L_MINECRAFT_KIT=minecraft:saddle,minecraft:saddle,minecraft:oak_planks*64
```

### Dyed items show their colour in icons, hands and on the ground

Commit: "Minecraft dyed items show their colour in icons, hands and on the ground", the one after `ece5ea8f`.

- **Cause** (as in the brief, re-checked at `ece5ea8f`): the drawn-item key (`item_tint`, `id~rrggbb`) carried only a potion colour, and `interface.rs` `item_model_tints` always gave a `minecraft:dye` tint its default, so every dyed leather item was brown (#A06540) in inventories, in the first-person hand, in third person and on the ground.
- **Vanilla 26.3** (data): `items/leather_horse_armor.json` and the four leather armour pieces tint layer 0 with `{type: minecraft:dye, default: -6265536}` and leave layer 1 (the overlay) plain. The dye tint reads `minecraft:dyed_color` (an int, or `{rgb}`), which `body_gear::dyed_color` already reads in both forms.
- **Changed:**
  - `crates/minecraft_terrain/src/item_tint.rs`: new `of_stack_at(base, stack)`, where `base` is the id or a dial key; it keys the potion colour or, when there is none, `body_gear::dyed_color`. `of_stack(stack)` is `of_stack_at(&stack.id, stack)`.
  - `interface.rs` `item_model_tints`: `minecraft:dye` takes the keyed colour or else its default, like `minecraft:potion` (moved out of the default-only arm with firework, map colour, team and custom model data).
  - `crates/render_anim/src/minecraft_hand.rs`: the first-person key is `of_stack_at(dials.key(&stack.id), stack)`, so a held compass, recovery compass or clock keeps its dial.
  - `body_render.rs` `held_items`: a held item in third person (the host's and the friends' bodies, `anim/minecraft_gear.rs`) keys `item.potion.or(item.dye)`. `GearItem.dye` is already on the wire, so friends see it with no wire change.
  - Dropped items (`mesh.rs` `of_stack`) and inventory icons (`minecraft_inventory.rs` `convert` and the hotbar list, both `of_stack`) need no change: they key through `of_stack`, and `icon_source` splits the colour off the icon key and keys it again around the dial key, so the dye passes through it.
- **Tests:**
  - `minecraft_terrain` `item_tint::tests::a_dyed_stack_keys_its_dye_and_a_dial_base_keeps_its_model`: red leather horse armour keys `~b02e26`, the `{rgb}` form splits back to 0x3C44AA, an undyed one keys the plain id, a compass keyed on a dial base comes back through `item_tint::split` and then `dials::split` to `minecraft:item/compass_16`, and a potion with a stray `dyed_color` keeps its potion colour.
  - `interface::tests::dyed_and_firework_tints_use_their_default_colour_through_selects`: the assertion that pinned the old limit (a keyed colour on the leather horse armour's dye tint gave #A06540) now expects the keyed colour; added the leather helmet select with 0xB02E26 and the dyed wolf armour, which still resolves to its undyed model (no tints).
  - `item_icons::tests::a_red_leather_horse_armour_icon_is_red_on_layer_zero_and_plain_on_layer_one` (real pack, skipped without it): every bare layer-0 pixel of the red icon is the sprite times #B02E26, every opaque overlay pixel is the overlay sprite and equals the plain icon there. (The brief listed this test under `horse-dye`; it belongs here because this commit makes the icon red.)
  - `body_render::tests::a_held_dyed_leather_piece_keys_its_dye`.
  - `render_anim` `minecraft_inventory::tests::a_dyed_leather_icon_keeps_its_colour_and_shares_its_name`: the red and the plain icon differ, both name `minecraft:leather_horse_armor`, and `icon_source` keeps `~b02e26`.
  - `minecraft_terrain` passes (1412; `sections::tests::background_full_update_matches_the_synchronous_one` failed once in the full run, a timing test that waits at most 1000 × 1 ms for a background thread while other tracks were building, and passed when run again alone), `render_anim` passes (214), and `check -p launcher` passes.
- **Decisions:**
  - The key holds one colour, so a stack with both components keys the potion colour (the brief's order). No vanilla item has both; a leather piece from a release-2 kit (`leather_*@red`, which got a stray `potion_contents` because of the missing tag fixed in the next commit) now shows the default potion blue on its dye layer instead of brown. Take a new one from the kit after the next commit.
  - `icon_source` keeps its body: it gets the icon key, not the stack, and already re-keys whatever colour the key holds. Only the hand builds its key from the stack, so only the hand calls `of_stack_at` with a dial base.
  - Third-person held items take the dye too (`body_render.rs`), as vanilla's item renderer does for every display context; the brief named only the first-person hand.
  - Left as before: item frames (`FrameView` carries only the potion colour; E-banners' `player-looks` gives frames the item look) and dyed wolf armour (its item model is a `has_component` condition, and the walker always takes `on_false`, the undyed model).
- **Owner's test steps:** this commit alone has no way to get a dyed item in game except a leatherworker's level 4 trade (the kit's `@red` needs the next commit), so its steps are with the next commit.

### Kits dye leather with the 26.3 dye tag

Commit: "Minecraft kits dye leather with the 26.3 dye tag", the one after `8c9db2c1`.

- **Cause:** `crafting.rs` `kit_potion_stack` gave an `@<dye>` entry `minecraft:dyed_color` only for items in `#minecraft:dyeable`. That tag is gone in 26.3 (not in the datapack's `tags/item/` and not in `client.jar`), so in a real kit `leather_horse_armor@red`, and every leather armour piece with a dye, got `potion_contents` `{potion: "minecraft:red"}` instead of a dye. The test passed only because its stand-in jar defined a `dyeable` tag. Round 7's `@dye` (track B log) was written against the older tag.
- **Vanilla 26.3** (data): `#minecraft:cauldron_can_remove_dye` lists the four leather armour pieces, `leather_horse_armor` and `wolf_armor`, which are exactly the items that take `minecraft:dyed_color`.
- **Changed:** `crafting.rs` `kit_potion_stack` checks `#minecraft:cauldron_can_remove_dye`; the stand-in tag in the test jar (`tags()`) has the new name and also lists `leather_horse_armor`. This covers the `.env` kit and `/kit` (`minecraft_survival` `commands.rs`, which calls the same function).
- **Tests:**
  - `minecraft_terrain` `crafting::tests::kit_dyes_leather_horse_armour_with_the_26_3_tag`: with the stand-in jar, `leather_horse_armor@red` gets `minecraft:dyed_color` 0xB02E26 and nothing else, and a plain one gets no components. With the real `client.jar` (skipped without it): `#minecraft:dyeable` holds nothing, the four leather pieces, the leather horse armour and the wolf armour with `@red` all get `dyed_color` 0xB02E26 and no `potion_contents`, and `potion@red` still gets `potion_contents`.
  - `kit_dyes_dyeable_items_with_a_dye_name` passes unchanged on the renamed stand-in tag.
  - All 1413 `minecraft_terrain` tests pass (the real-data branch ran); `check -p launcher` passes.
- **Decisions:** none beyond the brief.
- **Owner's test steps** (the whole `dye-icons` group; survival or creative, a new world, by day on flat grass, the kit below):
  1. **Icons.** Open the inventory (the kit fills the backpack). The red, blue and lime leather horse armour icons are red, blue and lime with their untinted buckles and trim on top, the plain one is brown, and the red leather chestplate and blue boots are red and blue. Hover each: all four horse armours are named "Leather Horse Armor". **Screenshot D1.**
  2. **First-person hand.** Move the red horse armour to the hotbar and hold it: it is red in your hand. Hold the blue boots: blue. **Screenshot D2.**
  3. **Third person.** Press F5 while holding the red horse armour: it is red in your character's hand. Press F5 twice more to go back.
  4. **On the ground.** Press Q to drop the lime horse armour and the red chestplate: both lie on the ground in their colour. Pick them up again: still lime and red. **Screenshot D3.**
  5. **A compass still turns.** Hold the compass from the kit and turn around: the needle still swings in your hand and in the hotbar, as before this commit. Hold the clock: it still shows the sun or moon.
  6. **Friend (two windows, `scripts\mp-local.ps1` or over the relay, both on this build).** The friend gets the same kit (or `/kit @a` from the host). In the friend's window the icons, the held armour and the dropped armour are coloured; when the friend holds the red horse armour, the host sees it red in the friend's hand. **Screenshot D4**, both windows.
  - The armour on a horse is still brown; that is the next group (`horse-dye`).
- **Kit** (9 backpack slots):

```
IW4L_MINECRAFT_KIT=minecraft:leather_horse_armor@red,minecraft:leather_horse_armor@blue,minecraft:leather_horse_armor@lime,minecraft:leather_horse_armor,minecraft:leather_chestplate@red,minecraft:leather_boots@blue,minecraft:compass,minecraft:clock,minecraft:cooked_beef*16
```

### Horses keep the dye of their leather armour

Commit: "Minecraft horses keep the dye of their leather armour", the one after `674afcc0`.

- **Cause** (as in the brief, re-checked at `674afcc0`): a horse kept only the id of what it wore (`HorseState.body_item`), so the dye was lost on every path: equipping by right-click (`equines.rs` `horse_interact`), the mount screen (`MountInventory::of` built the body stack with no components, and `body()` and `Command::MountInventory` passed only the id to `set_mount_inventory`), the death drop (`horse_death_equipment`), loading (`server_mobs.rs` read only `equipment.body.id`) and saving (`body_equipment` kept the old tag whenever the id matched, so a re-dyed armour under the same id came back with its old colour).
- **Vanilla 26.3** (javap): `Equippable.equipOnTarget` puts `stack.split(1)` in the body slot, so the worn armour or carpet keeps all its components, and the slot saves and drops that stack. `LivingEntity.onEquipItem` plays the equip sound when the new stack is not `isSameItemSameComponents` with the old one.
- **Changed:**
  - `third_party/minecraftoss/entities/src/horse.rs`: `HorseState.body_components: Option<serde_json::Value>`, next to `body_item` (which stays the id, so every `body_item.as_deref()` site is unchanged); `None` at first.
  - `world/equines.rs`: right-click equipping copies the held stack's components, for horse armour and llama carpets alike; `horse_death_equipment` drops the worn stack with its components; `set_mount_inventory` takes the body as `Option<ItemStack>` and counts a change of id or components as a new body (with the armour or carpet sound when one is put on).
  - `crates/minecraft_terrain/src/mount_inventory.rs`: `MountInventory::of` builds the body cell with the worn components, and the new `body_stack()` beside `body()` gives the whole body stack.
  - `server.rs`: `Command::MountInventory.body` is an `Option<ItemStack>`, filled from `body_stack()` (`minecraft_survival` `entities.rs` `set_mount_inventory` already passes the whole screen, so it is unchanged).
  - `server_mobs.rs`: a horse loads `equipment.body.components` (`tag_json`), and `body_equipment` writes them (`json_tag`) and keeps the loaded body tag only when both the id and the components are unchanged.
  - Friends: the horse's look is its save tag (`mob_tracking`), so a friend's copy gets the components through the same load with no other change, and a new dye is a new look.
- **Tests:**
  - `minecraftoss-entities` `world::equines::tests::equipping_and_dropping_dyed_leather_keeps_the_colour`: a red leather horse armour equipped by right-click keeps 0xB02E26 on the horse and in the dropped stack; through `set_mount_inventory` a lime one replaces it with the armour sound, the same lime stack again makes no sound, an undyed one clears the colour, an empty slot clears both; a llama's carpet with a custom name keeps it on the llama and in its drop.
  - `minecraft_terrain` `mount_inventory::tests::the_body_slot_keeps_its_dye_both_ways`: the screen shows the worn red armour with its colour, picking it up puts the red stack on the cursor, a shift-clicked lime one fills the slot with its colour, and `set_mount_inventory` with `body_stack()` puts the lime on the horse, which the next screen shows again.
  - `server_mobs::tests::a_horse_keeps_dyed_leather_armour_through_save_and_load`: a saved horse in red leather loads with 0xB02E26, saves back its untouched body tag, saves 0x80C71F after a re-dye under the same id, loads that again, saves no components once undyed, and an iron horse armour loads with none.
  - `mob_tracking::tests::a_friend_sees_a_horse_s_dyed_armour_and_its_new_dye`: the friend's copy gets the red, and a re-dye on the host sends one new look with the lime.
  - The donkey test's `set_mount_inventory` call passes an `ItemStack`.
  - All 439 `minecraftoss-entities` tests and all 1511 `minecraft_terrain` tests pass (3 ignored, as before); `check -p launcher` passes.
- **Decisions:**
  - Llama carpets keep their components too (the brief had the carpet path clear them). Vanilla's `equipOnTarget` keeps the whole stack for every body item, so a named carpet comes back named; the llama renderer reads only the id, so nothing else changes.
  - The loaded body tag is kept only when its components, read back through `tag_json`, equal the horse's. That is exactly what the load stored, so an untouched horse saves its exact NBT (bytes stay bytes), and any change writes the new stack.
  - Components on a `body` slot with no id are not loaded (the horse wears nothing then).
- **Protocol:** no message or codec change. `Command::MountInventory` is the host's in-process server command, and a friend's horse still comes as its save tag (now with the body's `components`, which an older build ignores).
- **Owner's test steps:** the armour on the horse itself stays brown until the next commit, so the steps for the whole group are with it.

### Dyed leather horse armour shows its colour on the horse

Commit: "Minecraft dyed leather horse armour shows its colour on the horse", the one after `6c83db8b`.

- **Cause** (as in the brief, re-checked at `6c83db8b`): `horse_render.rs` gave leather horse armour the fixed tint #A06540 and drew one layer only, so a dyed armour was brown on the horse and the untinted `leather_overlay` (buckles, straps and trim) was missing on every leather armour.
- **Vanilla 26.3** (data, `assets/minecraft/equipment/leather.json` in `client.jar`): the `horse_body` layers are `minecraft:leather` with `dyeable.color_when_undyed = -6265536` (#A06540), then `minecraft:leather_overlay` with no tint. Both textures are in `textures/entity/equipment/horse_body/`. The other horse armour assets have no `dyeable` layer.
- **Changed:**
  - `crates/minecraft_terrain/src/body_gear.rs`: `dye_in(components)` reads `minecraft:dyed_color` (an int or `{rgb}`) from a component map; `dyed_color(stack)` calls it.
  - `horse_render.rs`: `armor_layers(horse)` gives the worn armour's texture, its tint and, for leather, the overlay texture. Leather's tint is the dye (`dye_in` on `HorseState.body_components`, through `armor_render::dye_tint`) or else #A06540; every other armour stays untinted and has no overlay. `append_horses` draws the armour layer as before and the overlay over it, untinted, on the same cubes in the translucent pass, as the markings are drawn over the coat, so the two layers never fight for the same depth. `all_textures` adds `horse_body/leather_overlay`, which puts it in the entity atlas.
  - Friends draw their copy of the horse with the same code, and it has the components since the previous commit.
- **Tests:** `minecraft_terrain` `horse_render::tests::dyed_leather_tints_the_armour_and_draws_the_plain_overlay`: `armor_layers` gives nothing for a bare horse, #A06540 and the overlay for plain leather, the red for 0xB02E26, the blue for `{rgb: 0x3C44AA}`, and white with no overlay for iron armour with a stray dye; `all_textures` holds the overlay. Drawn from a saved horse in red leather on stand-in textures: after the coat, every armour vertex samples the leather texture with the red tint (times the face shade), and the translucent pass holds exactly as many overlay vertices, untinted. The horse_render, body_gear, item_tint, item_icons, llama_render, mob_tracking and mount_inventory tests pass (64); `check -p launcher` passes.
- **Decisions:**
  - Only leather takes a dye on the horse: a `dyed_color` on iron, gold, diamond, netherite or copper horse armour is ignored, as vanilla's only `dyeable` horse layer is leather's.
  - The overlay goes in the translucent pass (the brief's plan), not in the opaque mesh after the dyed layer.
- **Left out** (the brief's E-horses list): dyeing leather in the crafting grid (`minecraft:crafting_dye`, recipe `leather_horse_armor_dyed`) is not supported for any leather item; dyed armour comes from the kit and the leatherworker's level 4 trade. Wolves can't wear wolf armour yet, so a dyed wolf armour shows its colour only as an item.
- **Owner's test steps** (the whole `horse-dye` group; survival, a new world, by day on flat grass, the kit below):
  1. **Red on a horse.** Run `/summon minecraft:horse ~ ~ ~2 {Tame:1b}`. Move the red leather horse armour from the backpack to the hotbar, hold it and right-click the horse: you hear the armour sound, and the horse wears red armour with the plain brown buckles, straps and trim on top. Walk around it: the red and the trim do not flicker. **Screenshot A1.**
  2. **Plain leather and iron.** Summon two more tame horses beside it. Put the plain leather horse armour on one: brown with the same trim. Put the iron horse armour on the other: iron, as before. **Screenshot A2**, all three horses.
  3. **The screen.** Crouch and right-click the red horse. The armour slot shows the red icon. Take it out: it is red on the cursor and in your inventory. Shift-click the lime armour in your inventory: it goes into the armour slot with the armour sound. Close the screen: the horse wears lime with the trim. **Screenshot A3.**
  4. **Save and load.** Save and quit to the title, then load the world. The horse still wears lime, and crouch-right-click shows the lime icon in its armour slot.
  5. **The drop.** Kill the lime horse with the iron sword (`/kill` takes only players in the fork). The armour lies on the ground lime, and it is lime when you pick it up (hover: "Leather Horse Armor"). **Screenshot A4.**
  6. **Riding.** Saddle the horse in plain leather and ride it: the armour, its trim, the saddle and the reins move with the horse.
  7. **Friend (two windows, `scripts\mp-local.ps1` or over the relay, both on this build).** The friend gets the same kit (or `/kit @a` from the host). The host puts the blue armour on a horse and the friend puts the red one on another: each window shows the blue horse blue and the red horse red, both with the trim. Then the host opens the blue horse's screen and swaps the blue for the lime: in the friend's window the horse turns lime within a second. **Screenshot A5**, both windows.
- **Kit** (8 backpack slots):

```
IW4L_MINECRAFT_KIT=minecraft:leather_horse_armor@red,minecraft:leather_horse_armor@blue,minecraft:leather_horse_armor@lime,minecraft:leather_horse_armor,minecraft:iron_horse_armor,minecraft:saddle,minecraft:iron_sword,minecraft:cooked_beef*16
```

## E-banners. Banner patterns and crossbow states

### Item models pick crossbow and bow states

Commit: "Minecraft item models pick crossbow and bow states like vanilla", the one after `502b206b`.

- **Cause** (as in the brief, re-checked at `502b206b`): the item model walker (`special_models.rs` `collect_leaves`) read only `minecraft:using_item` from a bool and `minecraft:display_context`; every other `select` and `range_dispatch` took its fallback. So `items/crossbow.json` always gave `item/crossbow` (no arrow, no rocket, no pull) and `items/bow.json` always `item/bow`. A drawn item's key (`item_tint`) held only the id, a dial model and a colour, so nothing about the stack's charge could reach the model.
- **Vanilla 26.3** (javap and the 26.3 `items/` data):
  - `Charge.get`: no `charged_projectiles` or an empty list is `none`; a list holding a `firework_rocket` is `rocket`; any other list is `arrow`. The case names are `none`, `arrow` and `rocket`.
  - `CrossbowPull.get`: 0 when the crossbow is charged (or has no living holder), else `UseDuration.useDuration` (whole ticks used) over `CrossbowItem.getChargeDuration` as floats.
  - `UseDuration.get`: 0 unless the holder is using this very stack, then the whole ticks used (or the ticks left with `remaining: true`).
  - `crossbow.json`: select `charge_type` (`arrow` → `crossbow_arrow`, `rocket` → `crossbow_firework`), fallback condition `using_item`, on_true range_dispatch `crossbow/pull` (fallback `crossbow_pulling_0`, 0.58 `pulling_1`, 1.0 `pulling_2`). `bow.json`: condition `using_item`, on_true range_dispatch `use_duration` scale 0.05 (fallback `bow_pulling_0`, 0.65 `pulling_1`, 0.9 `pulling_2`). All six crossbow models and the three pulling bows take their `display` from `item/crossbow` and `item/bow`.
- **Changed:**
  - New `crates/minecraft_terrain/src/item_look.rs`:
    - `ItemLook { charge, use_ticks, base, layers }` and `ItemLook::of(components, use_ticks)`: the charge as `Charge::of` reads it, the use ticks of the using hand, `minecraft:base_color` and up to 16 `minecraft:banner_patterns` layers (a pattern id or inline `asset_id`, and a dye).
    - The key part: `suffix()` gives `|charge=arrow`, `|use=15`, `|base=red` and `|layer=<dye>,<asset>` parts (empty for a plain stack); `parse` reads them back; `keyed(base, look)` appends it; `split(key)` cuts it off again.
    - `ItemState { using, charge, use_ticks, charge_duration }` (25 ticks) and `ItemLook::leaf(model, context)`: the one plain model leaf the look's state picks, or nothing when the look holds no state.
  - `item_tint::of_stack_at` puts the look between its base (the id, a lodestone key or a dial key) and the `~rrggbb` colour, so `of_stack`, the first-person hand and the inventory icons all carry it, and `item_tint::split` and `dials::split` still give the id.
  - `special_models.rs`: `collect_leaves` takes an `ItemState`. `select` `minecraft:charge_type` takes the case named by the charge; `range_dispatch` `minecraft:crossbow/pull` and `minecraft:use_duration` (times `scale`) take the last entry whose threshold is at most the value, as `dials.rs` does; the condition `minecraft:using_item` reads the state's flag. The chosen branch comes first, then the fallback, then the other entries, as before. `item_leaves_with(model, state, context)` is new; `item_leaves` and `item_leaves_in` keep their bool for the current callers.
  - `mesh.rs` `ItemVisuals::model` (dropped items, items in the hand and on mobs) and `item_icons.rs` `item_icon_at` split the look off after the colour and resolve the leaf it picks with `model::resolve_item_leaf`; with no state they resolve as before.
  - `render_anim` `minecraft_inventory.rs` `icon_source` splits the look off before the dial lookup and puts it back, so names stay keyed by the plain id.
  - `dials.rs` is not touched.
- **Tests:**
  - `minecraft_terrain` `item_look::tests`: `a_look_reads_the_charge_the_patterns_and_the_base_colour` (empty, arrow, bare-string, rocket and un-namespaced rocket lists; id and inline patterns, a bad dye, a bad id and a missing pattern skipped; 20 layers keep the first 16; the suffix and `parse` round trip; junk parts ignored), `a_look_sits_after_the_dial_and_before_the_colour_and_splits_back` (a loaded crossbow, a potion with patterns and a colour, dyed leather with a base colour, and a dial-keyed compass all split back to the same id, dial model, look and colour), `a_look_picks_a_plain_leaf_only_when_it_holds_a_state`.
  - `special_models::tests::crossbows_and_bows_pick_their_charge_and_pull_like_vanilla`: the real crossbow and bow trees. No state → `crossbow`; using for 0, 14, 15, 24, 25 and 40 ticks → `pulling_0`, `pulling_0`, `pulling_1`, `pulling_1`, `pulling_2`, `pulling_2`; an arrow → `crossbow_arrow` even while used; a rocket → `crossbow_firework`; a 15-tick charge at 15 ticks → `pulling_2`; the bow at 0, 12, 13, 17 and 18 ticks → `pulling_0`, `pulling_0`, `pulling_1`, `pulling_1`, `pulling_2`; a `remaining` use duration takes its fallback.
  - `item_icons::tests::loaded_crossbow_icons_show_their_arrow_or_rocket` (real pack, skipped without it): the plain, arrow and rocket crossbow icons are the `crossbow_standby`, `crossbow_arrow` and `crossbow_firework` sprites, and a 25-tick look is `crossbow_pulling_2`.
  - `render_anim` `minecraft_inventory::tests::a_loaded_crossbow_icon_keeps_its_look_and_shares_its_name`.
  - All 1517 `minecraft_terrain` tests (3 ignored, as before) and all 218 `render_anim` tests pass; `check -p launcher` passes.
- **Decisions:**
  - The look sits after the dial or lodestone part and before the colour, behind its own mark `|`, which no id, model, lodestone key or colour holds. Code that wants the model splits the colour, then the look, then the dial; `dials::split` on a key with a look still gives the right id.
  - The look already holds the base colour and the pattern layers, so a patterned banner or shield gets its own key and icon now; it draws as before until group `banner-items`. A pattern's asset is its id (true of all 43 vanilla patterns), or the inline `asset_id`. The first 16 layers count, as `BannerRenderer` draws indices 0 to 15; a malformed layer among them is dropped.
  - Every stack with these components gets a look, whatever the item, since vanilla's properties read any stack; only models that ask for them change.
  - A look replaces the default model only when its state picks exactly one plain model leaf; a stack with no charge and no use ticks resolves exactly as before. A `use_duration` with `remaining: true` is not read (no 26.3 item has one) and takes its fallback.
  - The charge duration is the 25 ticks the AI uses; Quick Charge does not shorten the pull (the brief's decision).
  - Players have no crossbow or bow use in the fork, so a player's crossbow shows standby, arrow or rocket and never the pull, and a player's bow never pulls.
- **Protocol:** no change. Item keys are built on each client from the stacks it has.
- **Owner's test steps** (survival, a new world, by day, the kit below):
  1. **Icons.** Open the inventory. The first crossbow's icon has an arrow lying in it, the second a firework rocket, the third is the plain unloaded crossbow. Hover each: all three are named "Crossbow". **Screenshot C1.**
  2. **Hotbar and hand.** Move the three crossbows to the hotbar: the hotbar icons are the same. Hold each in first person: the arrow, the rocket and the plain string show in your hand. **Screenshot C2.**
  3. **On the ground.** Press Q with each crossbow: on the ground the first shows its arrow, the second its rocket, the third is plain. Pick them up again: the icons are unchanged.
  4. **Not yet:** in third person (F5) and in an item frame the loaded crossbows still look plain (group `player-looks`), and pillagers and piglins still hold a plain crossbow (the next commit).
  5. **Friend (two windows, both on this build).** The friend gets the same kit (or `/kit @a` from the host): the friend's own icons, hand and dropped crossbows show the arrow and the rocket in the friend's window.
- **Kit** (5 backpack slots):

```
IW4L_MINECRAFT_KIT=minecraft:crossbow[charged_projectiles=[{id:"minecraft:arrow"}]],minecraft:crossbow[charged_projectiles=[{id:"minecraft:firework_rocket"}]],minecraft:crossbow,minecraft:item_frame*2,minecraft:cooked_beef*16
```

### Pillagers and piglins show their crossbow's pull and loaded arrow

Commit: "Minecraft pillagers and piglins show their crossbow's pull and loaded arrow", the one after `ed3792f7`.

- **Cause** (as in the brief, re-checked at `ed3792f7`): `illager_render.rs` (`append_illagers`, the head item and both hands) and `piglin_render.rs` (`append_piglins`, both hands) keyed each held item by `stack.id` alone, so a pillager's or piglin's crossbow was always drawn as the plain `item/crossbow`, although the stack held its `charged_projectiles` (set by `Illager::set_crossbow_charged` and `Piglin::set_crossbow_charged`) and the entity its use ticks (`using_item`).
- **Vanilla 26.3:** `IsUsingItem` and `UseDuration` count only for the stack in the using hand (`getUseItem() == stack`). Pillagers and piglins start using the hand from `ProjectileUtil.getWeaponHoldingHand(mob, CROSSBOW)`: the main hand when it holds a crossbow, else the off hand.
- **Changed:**
  - `item_look::held_key(stack, use_ticks)`: the id with the stack's look and the use ticks (whole ticks, as `using_item` holds them).
  - `illager_render.rs`: `hand_key(entity, slot)` keys the stack in an equipment slot; the use ticks go only on `Illager::crossbow_hand()` and only while `using_item` is `Some`. `append_illagers` walks the two hand slots (right and left by `left_handed`, as before) through it, and keys the head item (the captain's banner) with `held_key` and no use ticks.
  - `piglin_render.rs`: `hand_key(entity, main)` does the same for a piglin, whose using hand is the main hand when it holds a crossbow (`Piglin::holds_crossbow`), else the off hand. `append_piglins` walks the main and off hand through it.
  - Friends: the friend's copy of the mob already gets the crossbow's components through the look tag (a load or a shot changes the saved equipment, so a new look is sent) and `using_item` through `MobShow`, so it keys the same with no other change.
  - `docs/MINECRAFT-SURVIVAL.md`: the piglin fighting paragraph says the string pulls and the arrow shows; the "drawn uncharged" limit is gone.
- **Tests:**
  - `minecraft_terrain` `illager_render::tests::a_pillager_s_crossbow_pulls_in_its_using_hand_and_shows_its_arrow_when_loaded`: drawn through `append_illagers`, a pillager's crossbow keys plain, then `|use=0`, `|use=15` and `|use=25` while charging, `|charge=arrow` when loaded; left-handed with a rocket in the off hand, only the crossbow (now in the left hand) pulls; with the crossbow moved to the off hand, the off hand pulls; a patterned banner on the head keys its look.
  - `piglin_render::tests::a_piglin_s_crossbow_pulls_in_its_using_hand_and_shows_its_arrow_when_loaded`: the same for a piglin with a gold ingot in the off hand, left-handed when loaded, and an off-hand crossbow pulling when the main hand holds a sword.
  - `mob_tracking::tests::a_friend_keys_a_pillager_s_and_a_piglin_s_crossbow_like_the_host`: through the trackers' add, look and move events, a friend's pillager and piglin key their crossbows like the host's: plain, pulling at 15 and 20 ticks, loaded with an arrow, and plain again after the shot.
  - All 1520 `minecraft_terrain` tests pass (3 ignored, as before); `check -p launcher` passes.
- **Decisions:**
  - Whole use ticks, as vanilla's `UseDuration`: a crossbow loads in 25 ticks, so each mob kind adds at most 26 pulling keys to the item model cache.
  - The captain's banner is keyed with its look already; its patterns show from group `banner-items` on.
  - Vindicators, evokers, skeletons and other mobs go through the same keys (illagers) or keep their plain ids (the others); a skeleton's bow pull belongs to the mob tracks (the brief's decision).
- **Protocol:** no change.
- **Owner's test steps** (survival, Normal, by day, flat open ground, the kit below):
  1. **A pillager loads.** Use a pillager spawn egg about 12 blocks away and stand still with the shield raised. When it loads, the string of its crossbow pulls back in three steps (about a second in all). **Screenshot C3** while it pulls.
  2. **The arrow.** When loaded, an arrow lies in its crossbow while it aims, for 1 to 2 seconds; after the shot the crossbow is empty again and the next load starts. **Screenshot C4** with the arrow.
  3. **A left-handed one.** Use more pillager eggs: about one in twenty is left-handed and holds the crossbow in its left hand; its crossbow pulls and shows the arrow the same way.
  4. **A piglin.** Run `/summon minecraft:piglin ~ ~ ~10 {IsImmuneToZombification:1b,equipment:{mainhand:{id:"minecraft:crossbow",count:1}}}` (if it holds no crossbow, summon another). Without gold armour it attacks: it steps back, its crossbow string pulls back in three steps, and the arrow shows until it shoots. **Screenshot C5.**
  5. **Friend (two windows, both on this build).** The friend stands next to the host: in the friend's window the pillager's and the piglin's crossbows pull and show the arrow at the same moments. **Screenshot C6**, both windows.
- **Kit** (4 backpack slots):

```
IW4L_MINECRAFT_KIT=minecraft:pillager_spawn_egg*16,minecraft:shield,minecraft:iron_sword,minecraft:cooked_beef*16
```

### Banners and shields show their patterns in hand, on heads, on the ground and as icons

Commit: "Minecraft banners and shields show their patterns in hand, on heads, on the ground and as icons", the one after `98cff7bf`.

- **Cause** (as in the brief, re-checked at `98cff7bf`):
  - `special_models.rs` `item_model(special)` built `minecraft:banner` and `minecraft:shield` without the stack. `push_banner` drew one flag tinted with the base colour, and `push_shield` always used `entity/shield/shield_base_nopattern`.
  - `special_tints` gave only a banner's base colour. `Sheet.tinted` was a bool that always meant tint index 0, so a model could hold only one tint.
  - The atlas held `entity/banner/banner_base`, `entity/banner/base` and `entity/shield/shield_base_nopattern`, but none of the 43 pattern sheets.
  - The item key already carried the look's base colour and layers (group `item-states`), but nothing read them.
- **Vanilla 26.3** (javap on the 26.3 jar, sheets checked in the 26.3 pack):
  - `BannerRenderer.submitPatterns(.., model, state, banner, baseColor, patterns)` draws `Sheets.BANNER_PATTERN_BASE` (`entity/banner/base`) or, for a shield, `SHIELD_PATTERN_BASE` (`entity/shield/base`), tinted with `DyeColor.getTextureDiffuseColor` of the base colour.
  - It then draws layers 0 to 15 (`i < 16 && i < size`), each with `Sheets.getBannerSprite` or `getShieldSprite` (`entity/banner/<asset path>` or `entity/shield/<asset path>`) and tinted with its own colour. Every layer goes through the same model, ordered after the base (`order(i + 2)`).
  - `ShieldSpecialRenderer.submit`: a shield is patterned when `banner_patterns` is not empty or `base_color` is set.
    - A patterned shield is drawn with `SHIELD_BASE` (`entity/shield/shield_base`), and then `submitPatterns` runs with the base colour, or `WHITE` when none is set.
    - Otherwise the shield is drawn with `SHIELD_BASE_NO_PATTERN`.
    - In 26.3 the patterns go over the whole `ShieldModel`. In the 26.3 pack, `entity/shield/base` and every shield pattern sheet are clear over the handle's part of the sheet, so drawing them on the plate alone looks the same.
  - A banner item's base colour is the item model's `color` (`items/white_banner.json`). Its `base_color` component is not drawn.
  - The pattern sheets mix alphas. `banner/creeper` and `banner/border` use 191 and 255, and `banner/gradient` runs from 255 at the top of the flag to 0 at the bottom.
- **Changed:**
  - `special_models.rs`:
    - `Sheet.tint: Option<usize>` (`tinted(index)`) replaces the bool. `push_cube` gives the faces that tint index.
    - `push_banner(.., layers)` adds one flag cube per layer after the flag, with the same cube, pose and UVs. Each layer uses `<namespace>:entity/banner/<path>` of its asset at tint index `i + 1`, for at most 16 layers. An asset that is not an id is skipped and keeps its tint slot. A placed banner passes no layers yet, so it is drawn as before.
    - `push_shield(look)` draws the plate and handle with `shield_base`, then a plate cube of `entity/shield/base` at tint 0 and one plate cube per layer, when the look has a base colour or patterns. Otherwise it keeps `shield_base_nopattern`.
    - `item_model(special, look)`.
    - `special_tints(model, look)`: a banner gives `[base from the model's colour, layer colours...]`, and a patterned shield `[base colour or white, layer colours...]`. A plain shield gives none, as before.
    - `pattern_textures(packs)` lists every sheet in the pack's `textures/entity/banner/` and `textures/entity/shield/`.
  - `mesh.rs`:
    - The world atlas adds `pattern_textures`, which is 89 sheets with the 26.3 pack.
    - `ItemVisuals::model` parses the look once. It builds the model with `model::resolve_item_model_with(.., look)` and the tints with `interface::item_look_tints(.., look)`. This covers dropped items, the first-person hand, mobs' held and head items (the raid captain's banner) and every other posed item.
  - `model.rs`: `resolve_item_model_with(pack, item, context, look)` is new, and `resolve_item_model_in` calls it with no look. `item_geometry` passes the look to the special models.
  - `interface.rs`: `item_look_tints(.., look)` is new, and `item_model_tints` calls it with no look.
  - `item_icons.rs`: `item_icon_at` builds a special item's geometry and tints with the look.
  - `item_icon.rs`:
    - `model_icon` blends a texel with partial alpha over what the earlier faces drew. It reads a copy taken before each face, so the two triangles of a face never blend twice.
    - Before this, such a texel replaced the pixel and left a see-through hole. A gradient layer would have made the flag's base colour vanish from the icon.
    - Block icons keep their old raster.
  - `docs/MINECRAFT-SURVIVAL.md`: the limit "There are no banner or shield patterns" now says where patterns show, and where they don't yet.
- **Tests** (`minecraft_terrain`):
  - `special_models::tests::a_patterned_banner_item_lays_one_tinted_flag_per_pattern_over_its_base`:
    - A white banner with creeper (black), a bad id and border (red) gets pole, bar, flag (`entity/banner/base`, tint 0), creeper (tint 1) and border (tint 2).
    - Each layer has the flag's corners and UVs, and the tints are white, black and red.
    - A wall banner has no pole.
    - With 20 layers the model keeps 16, ending at `p15` with tint 16, and the tints are 17.
    - A layer that isn't an id is skipped, while `test:sun` keeps tint slot 2.
    - A placed banner is unchanged.
  - `special_models::tests::the_ominous_banner_lays_its_eight_patterns_on_white`: the stack from `ominous_banner()` keys a look that splits back. It lists the eight vanilla layers in order, draws 3 + 8 elements with `entity/banner/<pattern>` sheets, and tints white, then cyan, through black.
  - `special_models::tests::a_shield_takes_the_patterned_base_only_with_a_base_colour_or_patterns`:
    - A plain shield uses `shield_base_nopattern` with no tints.
    - A red base colour gives `shield_base` and a tinted `entity/shield/base` plate with the plate's corners and UVs (tints `[red]`).
    - A yellow cross alone gives a white base under the cross (`[white, yellow]`).
    - A base colour with two patterns gives five elements with tints red, yellow and black.
  - `special_models::tests::every_vanilla_pattern_is_in_the_atlas_and_patterned_icons_differ_from_plain_ones` (with the real 26.3 pack and datapack, skipped without them):
    - Each of the 43 `banner_pattern` registry entries has its banner and shield sheet in the world atlas, and so do `banner_base`, `shield_base` and `shield_base_nopattern`.
    - The ominous banner's icon differs from the plain white banner's, and a red shield with a yellow gradient differs from the plain shield. Both keep the plain icon's outline alpha for alpha.
  - `item_icons::tests::banner_icons_blend_their_pattern_layers_and_skip_unknown_patterns` (a small test pack): a half-clear red stripe changes the icon but keeps every pixel's alpha, so it blends and leaves no holes. Each changed pixel is redder. A pattern with no sheet, before or after the stripe, leaves the icon unchanged.
  - `item_icons::tests::a_dropped_banner_draws_its_pattern_layers_in_their_colours`: a dropped plain banner has 18 faces and a striped one 24. The flag's vertices carry white and the stripe's red.
  - All 1526 `minecraft_terrain` tests pass (3 ignored, as before), and `check -p launcher` passes.
- **Decisions:**
  - In the world, in the hand and on mobs, the layers are cut out at alpha 0.1, like every entity sheet here (`render_gpu .../minecraft_world.rs`, the `opaque` fragment for vertex alpha below 0.75). Vanilla blends them (`RenderTypes.bannerPattern`), so here:
    - A gradient's fade becomes a hard edge near the bottom of the flag. The white gradient on the kit's blue banner covers about nine tenths of the flag.
    - The creeper face and the border, which use alphas 191 and 255, are drawn in one solid colour.
  - Icons blend like vanilla, because a cut-out icon needs the same hard edge and a replaced pixel left holes. Coplanar layers resolve to the later one, through `GreaterEqual` in the world and the blend in icons.
  - The shield's layers are drawn on the plate only, because every 26.3 shield sheet is clear over the handle.
  - A pattern whose sheet is not in the pack draws nothing and keeps its tint slot. Vanilla would refuse such a component. The atlas lists only the `minecraft` folders, so patterns from other namespaces are skipped the same way.
  - Plain banners and shields are built exactly as before, with the same elements and tints, and placed banners are not changed (group `placed-banners`).
- **Protocol:** no change. Keys and models are built on each client from the stacks it has.
- **Not yet:**
  - Placed banners still show their base colour only (group `placed-banners`).
  - A player's banner or shield in third person, as a friend sees it, or in an item frame shows no pattern (group `player-looks`).
  - Placed banners still drop nothing when broken (group `banner-roundtrip`).
  - The shield recipe is E2's (`fw-craft`).
- **Owner's test steps** (survival, a new world, by day, the kit below):
  1. **Icons.** Open the inventory.
     - The white banner shows a black creeper face inside a red border.
     - The blue banner shows a white fade from the top, a lime globe and a yellow bottom stripe.
     - The black banner is plain black.
     - The first shield is red with a yellow cross, the second plain black, and the third the plain wooden shield.
     - **Screenshot B1.**
  2. **Hand.** Hold the white banner, then the blue banner, in first person: their patterns show on the flag. On the blue banner the fade has a hard edge near the bottom of the flag, and the flag above it is white (the cut-out decision). **Screenshot B2.**
  3. **Raised shield.** Put the red shield in the offhand (F) and hold right-click to raise it: the yellow cross shows on the red plate. Swap in the black shield: it is plain black. **Screenshot B3.**
  4. **On the ground.** Press Q with each banner and shield. On the ground they show the same patterns and colours as in the inventory. Pick them up again.
  5. **The captain.**
     - On open ground, use pillager eggs until one carries a banner on its head (a captain, about one in seventeen).
     - Its banner shows the ominous pattern: a cyan diamond with grey and black stripes on white, in a black border. **Screenshot B4.**
     - Kill it with the sword. The dropped "Ominous Banner" shows the pattern on the ground, as an icon and in your hand. **Screenshot B5.**
  6. **Not yet.**
     - Placing a patterned banner shows only its base colour.
     - Breaking it drops nothing.
     - In third person (F5), in an item frame and in a friend's window, a player's banner or shield shows no pattern.
     - Later E1 groups fix these.
  7. **Friend (two windows, both on this build).** The friend gets the same kit (or `/kit @a` from the host). The friend's own icons, hand and dropped banners show the patterns in the friend's window, and the captain's banner shows the ominous pattern in both windows. **Screenshot B6**, both windows.
- **Kit** (9 backpack slots):

```
IW4L_MINECRAFT_KIT=minecraft:white_banner[banner_patterns=[{pattern:"minecraft:creeper",color:"black"},{pattern:"minecraft:border",color:"red"}]]*4,minecraft:blue_banner[banner_patterns=[{pattern:"minecraft:gradient",color:"white"},{pattern:"minecraft:globe",color:"lime"},{pattern:"minecraft:stripe_bottom",color:"yellow"}]]*4,minecraft:black_banner*4,minecraft:shield[base_color=red,banner_patterns=[{pattern:"minecraft:cross",color:"yellow"}]],minecraft:shield[base_color=black],minecraft:shield,minecraft:pillager_spawn_egg*16,minecraft:diamond_sword,minecraft:cooked_beef*16
```

### Banners keep their patterns and name when placed

Commit: "Minecraft banners keep their patterns and name when placed", the one after `ace496b4`.

- **Cause** (as in the brief, re-checked at `ace496b4`): placing a banner item wrote only the block state (`actions.rs`, `body.write_blocks`). The server level made the default banner block entity, so `banner_patterns`, `custom_name` and every other component of the stack were lost, and the saved banner had none. Only shulker boxes took their components (`containers.place_shulker`).
- **Vanilla 26.3** (javap on `BlockEntity` and `BannerBlockEntity`):
  - `BlockItem.place` calls `BlockEntity.applyComponentsFromItemStack(stack)`, which runs `applyComponents(prototype, patch)`.
  - `BannerBlockEntity.applyImplicitComponents` reads `BANNER_PATTERNS` (default empty) and `CUSTOM_NAME` (may be null). `saveAdditional` writes `patterns` only when they are not empty, and `CustomName` only when there is a name.
  - `BlockEntity.applyComponents` then keeps the rest of the stack's patch in `components`: `patch.forget(implicit).split().added()`. The implicit set holds the components the block entity read, plus `block_entity_data` and `block_state`, and removals (`!minecraft:...`) are not kept. `saveWithoutMetadata` always writes `components`.
- **Changed:**
  - New vendored `third_party/minecraftoss/world/src/level/components.rs` (declared next to `lectern` in `level/mod.rs`):
    - `implicit(kind)` lists the components a block entity kind reads itself. For `minecraft:banner` these are `banner_patterns` → `patterns` and `custom_name` → `CustomName`; other kinds have none.
    - `applied(entity, components)` gives the block entity tag after vanilla's `applyComponents`. An implicit component replaces its field (an empty pattern list or a missing name clears it), and every other component except `block_entity_data`, `block_state` and removals goes into `components`, which replaces the old one.
    - `Level::apply_block_entity_components(pos, components)` makes the block entity when it is missing (`ensure_block_entity`), writes the new tag with `set_block_entity_tag` and returns false where the block has no block entity. It is generic, so track D can use it for placed spawners and vaults.
    - `is_banner(id)` and `kind(tag)` are small helpers for the callers.
  - `crates/minecraft_terrain/src/server.rs`: `ServerSim::block_entity_components(pos, components)` turns the stack's JSON components into NBT (`item_nbt::components_tag`) and calls the level. `Command::BlockEntityComponents` sits next to `Command::Lectern`, with `ServerHandle::block_entity_components` next to `lectern`.
  - `crates/minecraft_survival/src/actions.rs`: after `write_blocks`, beside `place_shulker`, a placed banner whose held stack has components sends them through the new `Entities::block_entity_components`. The command follows the placing edit on the same channel, so the level already holds the banner.
  - The server reports the changed block entity as before (`collect_block_entity_changes`), so it reaches the stream and the chunk save.
  - `crates/minecraft_survival/src/level.rs` `save_server_block_entities`: a banner block entity the server changed also goes into `friend_changes`, so `session::block_updates` sends friends its new tag.
    - Before this, friends got a server block entity only with a block change (the banner's default tag, a tick before the patterns arrive) or as a spawner or vault (`spawner_tags`).
  - `crates/minecraft_survival/Cargo.toml`: `minecraftoss-world` moves from the dev-dependencies to the dependencies (`Cargo.lock` already lists it).
  - `docs/MINECRAFT-SURVIVAL.md`: placed banners keep `patterns`, `CustomName` and `components`, and the save limits name banners next to shulker boxes.
- **Tests:**
  - `minecraftoss-world` `level::components::tests`:
    - `a_banner_takes_its_patterns_and_name_and_stores_the_other_components`: the ominous components give `patterns` and a `components` of `item_name`, `rarity` and `tooltip_display`. A custom name gives `CustomName`, and `block_state`, `block_entity_data` and a removal are not stored.
    - `placing_again_replaces_the_patterns_the_name_and_the_components`: the old patterns, name and components go, and an empty pattern list writes no `patterns`.
    - `other_block_entities_store_every_component`: a spawner keeps a custom name in `components`. A tag that is not a compound gives nothing. `is_banner` takes standing and wall banners but not shields or other namespaces.
  - `minecraft_terrain` `server::tests` (real data, skipped without it):
    - `a_placed_banner_takes_the_patterns_name_and_components_of_its_item`: a placed white banner given the components of `ominous_banner()` reports a block entity with the eight layers in order (rhombus cyan to border black), `components` with `item_name` (`block.minecraft.ominous_banner`), `rarity` (`uncommon`) and `tooltip_display`, and no `CustomName`, and the level keeps the same tag. A blue wall banner given a name and one layer gets `CustomName`, one layer and empty `components`. Stone takes nothing, and empty components change nothing.
    - `a_banner_s_components_reach_the_server_thread_and_come_back_as_its_block_entity`: through `ServerHandle`, the command comes back in `Output.block_entities` with the eight layers.
  - `minecraft_survival` `actions::tests::a_placed_banner_keeps_its_patterns_for_the_save_and_for_friends`: a player places the ominous banner with `act`. The server's block entity reaches the stream and `friend_changes` (only the banner's position), and `session::block_updates` sends friends the tag with eight layers and the three stored components.
- **Decisions:**
  - The level function lives in its own vendored file `level/components.rs`, declared next to `lectern`, not inside `lectern.rs`. It is generic and has nothing to do with lecterns.
  - Like vanilla, the block entity keeps every other component of the stack, not only the three of the ominous banner, and a later placement replaces all of them.
  - Only banners send their components for now (the brief's scope). Shulker boxes keep their own path through `Containers`, and other blocks still drop a placed item's name (the save limit in `MINECRAFT-SURVIVAL.md`).
  - Friends learn of a banner's new block entity through `friend_changes` in `level.rs`. The line in `entities.rs` that fills `spawner_tags` stays as it is, because track W may add its sculk tags there.
- **Protocol:** no message or codec change. Friends already receive `ToClient::BlockEntity`, and they now get it once more for a banner whose block entity the server changed.
- **Owner's test steps:** a placed banner still shows only its base colour (group `placed-banners`), so nothing changes on screen yet. The steps for both commits of the group are with the next commit.

### Banners drop with their patterns and name

Commit: "Minecraft banners drop with their patterns and name", the one after `3bd5848d`.

- **Cause** (as in the brief, re-checked at `3bd5848d`):
  - `third_party/minecraftoss/player/src/loot.rs` `apply_modifiers` returned `None` for a `copy_components` from `block_entity` whose `include` names anything but `custom_name`. The `?` in `entry` then voided the whole roll. So a banner broken by hand or by an MW2 bullet or blast dropped nothing. The same was true of an empty, unnamed shulker box (a filled or named one dropped only through its `kept` stack), a player head and an unbroken decorated pot.
  - `Entities::drop_blocks` gave `kept` components only to a drop whose id equals the block's, which a `*_wall_banner` never matches, and `harvest` built `kept` for shulker boxes only.
  - The server level's own drops (TNT and other server explosions, a banner whose wall or floor goes, wither breaks: `drop_resources` and `on_explosion_hit`) roll `core/src/loot.rs`, whose `copy_components` passes without copying, so they dropped a plain banner.
- **Vanilla 26.3** (the 26.3 data and javap):
  - The 16 banner loot tables (`blocks/white_banner` and the others) hold one item under `survives_explosion`, with a `copy_components` from `block_entity` that includes `custom_name`, `item_name`, `tooltip_display`, `banner_patterns` and `rarity`. A wall banner drops like its standing banner.
  - The `block_entity` source copies `BlockEntity.collectComponents()`: the stored `components` plus `collectImplicitComponents`, which for a banner sets `banner_patterns` and `custom_name`.
  - `Block.dropResources` and `ServerExplosion` hand the block entity to the loot context, so explosions and a lost support drop the same components. `ServerExplosion.addOrAppendStack` merges only stacks with the same components.
- **Changed:**
  - `third_party/minecraftoss/player/src/loot.rs`: a `copy_components` from `block_entity` no longer voids the roll. It copies nothing itself, and the caller fills in the components. Other sources still void it. The stale comment about custom names went with the old check.
  - `third_party/minecraftoss/world/src/level/components.rs`:
    - `collected(entity)` is `collectComponents`: the stored `components` plus the kind's implicit fields.
    - `dropped_components(entity)` gives what the banner loot table copies (`BANNER_LOOT`, the five ids above), or nothing for other kinds or when there is nothing to copy.
    - `standing(id)` turns `*_wall_banner` into `*_banner`.
    - `Level::keep_block_entity_components(pos, state, drops)` puts those components on the drops that are the block's own item.
  - `level/drops.rs` `drop_resources` and `level/explosion.rs` `on_explosion_hit` call it after the roll, before the drops are popped or merged, while the block entity is still there.
  - `third_party/minecraftoss/world/src/chunk_map.rs`: `ChunkMap::stored_block_entity(pos)` reads the block entity as last stored, from the waiting block entity edits or the chunk copy, without writing the waiting block edits. `TerrainStream::stored_block_entity` calls it.
  - `crates/minecraft_survival/src/actions.rs` `harvest`: `kept` also holds, for each broken block, the `dropped_components` of its stored block entity, as JSON (`item_nbt::tag_json`). The break is already a waiting edit at that point, and the block entity is still readable.
  - `crates/minecraft_survival/src/entities.rs` `drop_blocks`: kept components go to the drop that is the block's standing item, and the extra boxed stack is made for shulker boxes only.
  - `docs/MINECRAFT-SURVIVAL.md`: a mining line on banner drops and on the other tables that now drop the plain item.
- **Tests:**
  - `minecraftoss-player` `loot::tests::components_copied_from_the_block_entity_leave_the_item_to_the_caller`: a wall banner rolls the standing banner's table with a `block_entity` copy and drops one plain banner, and another source still gives no answer. The jar test (skipped without the harness jar) now expects a yellow shulker box and a red banner from a red wall banner.
  - `minecraftoss-world` `level::components::tests::a_broken_banner_gives_back_what_its_loot_table_copies`:
    - A placed ominous banner gives back exactly its four components.
    - A named banner with patterns, lore and a rarity gives back the name, the patterns and the rarity, but not the lore.
    - A plain banner and a spawner give nothing, and `collected` reads a spawner's stored components.
    - `standing` maps wall banners only.
  - `minecraft_terrain` `server::tests::a_banner_blown_up_or_torn_from_its_wall_drops_with_its_components` (real data): a decaying explosion of power 1 on the placed ominous banner drops one white banner with the ominous components. Breaking the stone behind the named blue wall banner drops one blue banner with its name and pattern, and no wall banner item.
  - `minecraft_terrain` `terrain::tests::a_broken_block_s_block_entity_stays_readable_until_the_break_is_written` (real data):
    - A banner's block entity is read while its edit waits, and again after it is written.
    - After the break is recorded, it is still read until the chunk is written, and then it is gone.
    - A chunk that is not loaded gives nothing.
  - `minecraft_survival` `actions::tests::a_broken_banner_drops_with_its_patterns_and_name` (real data and the 26.3 jar). Each banner is placed with `act` and broken through `harvest`:
    - A standing ominous banner broken by hand drops one white banner with the ominous components.
    - A named blue banner placed on a wall drops one blue banner with its components.
    - An MW2 blast of power 1 keeps the ominous components.
    - A plain red banner drops a plain red banner. In release 2 these all dropped nothing.
    - A creative player's break drops nothing.
  - With both commits, all tests of the touched crates pass:
    - 1530 in `minecraft_terrain` (3 ignored, as before);
    - 787 in `minecraft_survival`, plus `vanilla_world`;
    - 199 in `minecraftoss-player`, with the `scenarios` stand-in files of `full-suite.ps1` made for the run and removed after;
    - 118 in `minecraftoss-world`.

    `check -p launcher` passes.
- **Decisions:**
  - **Server drops are handled in the server level, not at the two `render_anim` blast sites** (the brief's plan used `keep_dropped_contents` there). The level still holds the block entity when it rolls the loot, as vanilla does. That way:
    - a patterned banner never merges with a plain one in a blast;
    - a blast that keeps no drop (one in its power) adds none, whereas `keep_dropped_contents` spawns a banner when it finds none;
    - a plain banner lying nearby can't take the components;
    - a banner that falls with its wall or floor, and wither breaks, keep their patterns too.

    `render_anim` is not touched.
  - **`harvest` reads the stored block entity, not `loaded_chunk`** (the brief pointed at the read in `session.rs`). `loaded_chunk` writes the waiting edits first. The break is already one of them when `harvest` runs, so the banner's block entity would be gone. A banner broken within a tick or two of being placed, before the server's block entity reaches the stream, drops plain.
  - The copied list is the 26.3 banner loot table's `include` list, written into `BANNER_LOOT` (all 16 tables are alike). Only banners get components this way. Shulker boxes keep their own `Containers` path.
  - Every `copy_components` from `block_entity` now lets the roll through, as the brief says. So these blocks drop the plain item where they used to drop nothing:
    - an empty, unnamed shulker box, as vanilla does;
    - a player head, without the skin profile (heads use the default skin here, and a placed head doesn't keep one);
    - an unbroken decorated pot, without its sherds (a cracked pot still rolls nothing, since the `dynamic` entry isn't ported).

    Bee nests and beehives still roll nothing, because of their `copy_state`.
- **Protocol:** no change.
- **Owner's test steps** (both commits of the group `banner-roundtrip`; survival, a new world, by day on flat grass, the kit below). Placed banners still show their base colour only, until group `placed-banners`; the drops show the patterns.
  1. **By hand.**
     - Build a wall of planks two high.
     - Place the patterned white banner on the ground, the patterned blue banner on the wall and a black banner on the ground.
     - Break each with the axe. The white one drops a white banner whose icon shows the black creeper face in a red border. The blue one drops a blue banner (not a wall banner) with the white fade, the lime globe and the yellow stripe. The black one drops a plain black banner. In release 2 none of them dropped anything.
     - **Screenshot D1**, the three drops on the ground.
  2. **Save and load.** Place the patterned white banner, save and quit to the title, load the world and break it: it drops with its pattern.
  3. **Its wall goes.** Hang the patterned blue banner on the plank wall and break the plank behind it. The banner falls and drops with its pattern.
  4. **TNT.**
     - Place three patterned white banners a few blocks apart around a TNT block and light it with flint and steel.
     - A blast keeps each drop one time in four, so expect about one banner. Every banner that drops keeps its pattern.
     - **Screenshot D2.**
  5. **MW2.** Place a patterned white banner and shoot it until it breaks: it drops with its pattern. Throw a frag grenade at another one: when it drops (about one time in four), it has its pattern.
  6. **The ominous banner.**
     - Use pillager eggs on open ground until one carries a banner (a captain), and kill it.
     - Pick up the "Ominous Banner", place it and break it. It drops as an "Ominous Banner" again (hover it in the inventory), with the pattern in its icon, and it stacks with no other banner.
     - **Screenshot D3**, the inventory with the ominous banner hovered.
  7. **Creative.** Run `/gamemode creative`, place a patterned banner and break it: nothing drops. Run `/gamemode survival`.
  8. **The shulker box.** Place the empty shulker box and break it: it drops (in release 2 an empty box dropped nothing). Put a stone in it, break it again: it drops with the stone inside.
  9. **Friend (two windows, both on this build).**
     - The friend gets the same kit (or `/kit @a` from the host).
     - The friend places the patterned white banner and the host breaks it: it drops with its pattern.
     - The host places the patterned blue banner on a wall and the friend breaks it: it drops with its pattern.
     - **Screenshot D4**, both windows.
- **Kit** (11 backpack slots):

```
IW4L_MINECRAFT_KIT=minecraft:white_banner[banner_patterns=[{pattern:"minecraft:creeper",color:"black"},{pattern:"minecraft:border",color:"red"}]]*8,minecraft:blue_banner[banner_patterns=[{pattern:"minecraft:gradient",color:"white"},{pattern:"minecraft:globe",color:"lime"},{pattern:"minecraft:stripe_bottom",color:"yellow"}]]*4,minecraft:black_banner*2,minecraft:pillager_spawn_egg*16,minecraft:oak_planks*32,minecraft:diamond_axe,minecraft:diamond_sword,minecraft:tnt*2,minecraft:flint_and_steel,minecraft:shulker_box,minecraft:stone*4
```

## Review fixes

### The starting kit keeps the quotes inside its components

Commit: "Minecraft starting kits keep the quotes inside their components", the one after `3bd690c4`.

- **Defect** (found in the track review): the kit a player gets on joining a world (the host in `render_anim/src/minecraft_world.rs`, a friend in `minecraft_host.rs`) read `IW4L_MINECRAFT_KIT` from the process environment, which `asset_transport::load_dotenv` fills with `dotenvy`. `dotenvy` reads an unquoted value like a shell does: a `"` in the middle opens and closes a quote and is dropped. So the E-kit line's `{pattern:"minecraft:creeper",color:"black"}` arrived as `{pattern:minecraft:creeper,color:black}`, which is not SNBT (a bare string can't hold `:`), and the banner patterns, the shield's patterns and the crossbow's arrow were left out without a word. Only `/kit`, which reads the `.env` itself (`minecraft_commands::kit::read_env_file`), kept them. The same was true of the book kits of round 8 (`pages:["One, two"]` became two pages, `One` and `two`).
- **Changed:**
  - `crates/minecraft_commands/src/kit.rs`: `kit_line(env_file)` reads the kit line from the `.env` the way `/kit` does and falls back to the process environment when the file has none.
  - `render_anim` `minecraft_world.rs` (the host's join) and `minecraft_host.rs` (a friend's join): one line each, `kit_line(minecraft_survival::commands::env_file())` in place of `std::env::var`, so the starting kit and `/kit` read the same line.
- **Tests:** `minecraft_commands` `kit::tests::the_kit_line_keeps_the_quotes_inside_its_components`: a `.env` with the E-kit banner and crossbow entries gives the line back with its quotes, and `kit_entries` reads the banner's patterns and the crossbow's arrow from it.
- **Decisions:** when the `.env` names the kit, the file wins over the process environment, as for `/kit`.
- **Owner's test steps** (survival, a new world each time, the kit below in the `.env`):
  1. **A quoted book.** Start the world with the kit below and open the written book "Kit" from the backpack: it has one page reading "One, two". Before this fix it had two pages, "One" and "two". **Screenshot R1.**
  2. **The same through `/kit`.** Empty a backpack slot and run `/kit`: the new book also has the one page "One, two".
  3. **Banners and the crossbow.** With the E-kit line of this log in the `.env` and a new world, the starting kit's banners, shield and crossbow now carry their patterns and the arrow like the ones `/kit` gives. This only shows once the later E1 groups draw banner patterns and the loaded crossbow; until then step 1 is the check.
- **Kit:**

```
IW4L_MINECRAFT_KIT=written_book[written_book_content={title:"Kit",author:"Owner",pages:["One, two"]}]
```

## Final verification

### Clippy on the touched crates

Commit: "Track E1 code passes clippy", the one after `70df989a`.

- **Clippy** on `frame`, `hud`, `minecraft_commands`, `minecraft_net`, `minecraft_survival`, `minecraft_terrain`, `render_anim`, `minecraftoss-entities`, `minecraftoss-player` and `minecraftoss-world` (`--all-targets`, `-A clippy::approx_constant`, the `third_party/scenarios` stand-in files of `full-suite.ps1` made for the run and removed after) found two warnings from this track; every other warning in these crates is older:
  - `minecraft_net` `codec.rs`, the test `a_player_view_carries_the_mount_s_health`: `Box::new(PlayerView::default())` (`box_default`) is now `Box::default()`.
  - `hud` `minecraft_recipe_book.rs` `Tip`: `McStack.effects` (E-omen) made `Tip::Item` 217 bytes larger than `Tip::Text`, past `large_enum_variant`'s 200. `Tip::Item` now boxes its stack; `minecraft_inventory.rs` reads it unchanged through the box.
- **Left alone:** `hud` `minecraft_screens.rs` `SlotDraw` and `minecraft_survival` `menus.rs` `Open` (`large_enum_variant`) were over the limit before this track.
- **Tests:** none new; `hud` and `minecraft_net` pass.
- **Owner's test steps** (survival, a new world, the combined kit below):
  1. **Recipe book tooltips.** Open the inventory and click the recipe book button. Hover a recipe button: its item's tooltip shows as before, with "Right Click for More" under it when the button holds more than one recipe. **Screenshot V1.**
  2. **A ghost recipe.** If the filter button shows "Showing Craftable", click it so all recipes show. Click a recipe with a red background (one you can't pay for): grey ghost items fill the crafting grid. Hover a ghost item: its tooltip shows.

## Combined kit

One line for a quick look at the whole track in one world (27 backpack slots). Each section above has its full kit. The banner, shield and crossbow entries of E-kit are left out to stay within 27 slots; the E-banners kits carry the patterned banners and shields and the loaded crossbows, and E-kit's `/give` steps still check them.

```
IW4L_MINECRAFT_KIT=minecraft:written_book[written_book_content={title:"Kit",author:"Owner",pages:["One, two"]}],minecraft:firework_rocket[fireworks={flight_duration:3}]*16,minecraft:firework_rocket*16,minecraft:elytra,minecraft:horse_spawn_egg*4,minecraft:wolf_spawn_egg*2,minecraft:cat_spawn_egg*2,minecraft:villager_spawn_egg*4,minecraft:strider_spawn_egg*2,minecraft:wheat*64,minecraft:potato*64,minecraft:carrot*64,minecraft:beetroot*64,minecraft:bone*32,minecraft:cod*64,minecraft:golden_carrot*16,minecraft:composter,minecraft:ominous_bottle*2,minecraft:ominous_bottle[ominous_bottle_amplifier=4],minecraft:potion@long_night_vision,minecraft:golden_apple*8,minecraft:milk_bucket,minecraft:saddle,minecraft:saddle,minecraft:hay_block*16,minecraft:leather_horse_armor@red,minecraft:leather_chestplate@red
```
