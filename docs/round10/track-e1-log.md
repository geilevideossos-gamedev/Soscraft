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
