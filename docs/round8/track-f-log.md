# Track F log: fixes from the interim test of release 1

Branch `track-f`, worktree `C:\Dev\mc-wt\track-f`, target dir `D:\iw4l-wt\track-f`. Brief: `track-f.md` (the owner's findings F1 to F4 from section A and kit A of release 1, `4d45e4fe`). Base: `aaf968e1`.

A commit can't name its own hash, so each entry is written with its commit and names the commit before it; the last entry's hash is in `git log` under its subject.

## F1. MW2 models get their light grid and reflections again

Commit: the one after `aaf968e1`.

- **Cause:** every MW2 pass binds its textures through one bind group of four binding arrays: 2D (layout count 8192), cube (256), 3D (32) and samplers (128). `ExactTextureTable::create_bind_group` bound only the views the table had collected so far (a few hundred 2D views, a handful of cubes, two 3D views: the placeholder and the model lighting atlas). On this PC (DX12, RTX 4050) the descriptors of a partially filled array after the first one are not found where the shader looks for them: the cube and 3D arrays read zeros. So every MW2 model lost its model lighting atlas (3D, the light grid ambient) and its reflection probe (cube, also the MW2 sky box). On MW2 maps the sun still lit the models, which hid it. In a Minecraft world the models take no primary light by design (`neutral_light_grid_sample`, picked primary 0), so the atlas is their only light: the gun, the hands and the third-person body came out exactly black (0, 0, 0) everywhere, by day in the Overworld too, not only in the Nether. The elytra, armour and Minecraft items are drawn by the Minecraft pass and were not affected.
- **How it was found** (a debug build in its own run folder on a hidden desktop, a copy of `r7final` and fresh worlds, nothing of the owner's install written): the pass hook's raw MW2 colour was already black before the Minecraft lightmap; the shader's registers showed the colour map and the fog fine and the 3D atlas lookup at zero; `textureLoad` on the bound atlas at the eye's tile returned zero although the tile upload and even a full-texture write to the same `GpuImage` (same view id as table index 1) had gone through; with every array filled to its layout count the gun was lit. The same black showed with the round 7 build (`e2313625`) and the `3c2c62b` release, with Cold-Blooded and other classes, default settings and a cold shader cache, so it was not a round 8 change.
- **Changed:** `render_gpu` `drawsurf/texture_table.rs`: `full_binding_array` fills each array of the table bind group up to its layout count with the array's first entry (the placeholder view or sampler), so every real entry keeps its index and every descriptor slot is written. The sun-caster group masks the sun shadow views first, then fills.
- **Side effect (MW2 maps):** models on MW2 maps now get their light grid ambient and reflections as in MW2, and the MW2 sky box (a cube) shows; before it was black.
- **Tests:** `render_gpu` `texture_table`: `a_binding_array_is_filled_to_its_layout_count_after_its_own_entries` (own entries keep their order, the rest is the first entry, an empty array stays empty) and `every_table_array_is_bound_with_the_layout_count` (for the 2D, cube, 3D and sampler entries of `texture_table_bind_entries`, the table capacity equals the layout count and the filled array has that length).
- **Decisions:** the fix is in the MW2 texture table, not in the Minecraft lighting hook: the hook and the Moody lightmap were right (the lightmap view showed the body lit by the Nether's ambient and the lava's block light). Filling costs one descriptor per slot when a table's bind group is rebuilt (once per table epoch), not per draw.
- **Owner's test steps** (survival, `iw4l-survival.exe map minecraft:overworld`, kit below, any class):
  1. By day in the Overworld, hold an MW2 gun: the gun and the gloves show their textures and shading (they were black). **Screenshot 1.**
  2. Press F5: the MW2 body is lit, with the armour and the elytra on it. F5 again for the front view. **Screenshot 2.**
  3. Stand under a roof at night, away from torches: the body and the gun are dark, as Minecraft mobs are there; place a torch next to you: they brighten.
  4. Build a Nether portal (obsidian, flint and steel) and go through. In third person the body is dimly lit like the terrain, warmer near lava, and never a black silhouette. **Screenshot 3.**
  5. Optional, an MW2 map (`iw4l-survival.exe map mp_rust`): the sky shows clouds instead of black, and guns show reflections on their metal.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:obsidian*14,minecraft:flint_and_steel,minecraft:torch*64,minecraft:elytra,minecraft:iron_chestplate
```

## F2a. The spyglass and the spears show their inventory sprite as icon

Commit: the one after `456bef90`.

- **Cause:** the inventory icon took its model from `item_model_reference`, which follows a `minecraft:select`'s `fallback` first. The 26.3 spyglass definition is a `minecraft:display_context` select whose `gui`, `ground`, `fixed` and `on_shelf` case is `item/spyglass` (the flat sprite) and whose fallback is `item/spyglass_in_hand` (the small 3D tube with `item/spyglass_model`). So the hotbar and inventory drew the in-hand texture strip (screenshot 2). The seven spears (`*_spear` with `*_spear_in_hand`) have the same definition and showed their in-hand sprite too. The bundles and the trident were not affected (same model in both cases, or the trident's special path).
- **Changed:** `minecraft_terrain` `interface.rs`: `item_gui_model_reference` picks the first `display_context` case whose `when` holds `gui`, else falls back to `item_model_reference`. `item_icons.rs` uses it for the icon. The held, dropped and framed models are unchanged (they keep `item_model_reference` and their own display contexts).
- **Tests:** `interface::tests::the_inventory_icon_takes_the_gui_case_of_a_display_context_select` (spyglass, bundle, a select with no `gui` case, a select on another property) and, with the 26.3 pack, `item_icons::tests::vanilla_spyglass_and_spear_icons_are_their_inventory_sprites` (the 32 px icon equals `item/spyglass` and `item/iron_spear` scaled with nearest).
- **Decisions:** the fix follows vanilla's `ItemModelResolver` for the GUI context instead of special-casing the spyglass, so every item with a GUI case (today the spyglass and the spears) is right.
- **Owner's test steps** (survival or creative, kit below):
  1. Open the inventory: the spyglass is the brass-and-glass sprite lying diagonally, like in vanilla, not a thin vertical strip. **Screenshot 1.**
  2. Put it on the hotbar: the same sprite there. Hold it: in hand it is still the small 3D spyglass.
  3. The iron spear in the inventory is the diagonal spear sprite. **Screenshot 2.**
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:spyglass,minecraft:iron_spear
```

## F2b. The spyglass zooms like vanilla

Commit: the one after `0d6f48be` (whose log entry said "six spears": there are seven, corrected here).

- **Before:** the spyglass had no use at all; right-click did nothing.
- **Vanilla 26.3** (javap): `SpyglassItem.use` plays `item.spyglass.use` and starts using at once, use duration 1200 ticks; `releaseUsing` and `finishUsingItem` play `item.spyglass.stop_using`; switching away or dying stops without a sound (`stopUsingItem`). `AbstractClientPlayer.getFieldOfViewModifier` returns 0.1 while scoping in first person; `Camera.tickFov` moves the modifier half way to its target each tick (clamped 0.1 to 1.5) and the FOV is `fov * modifier`. `Hud.extractSpyglassOverlay` draws `textures/misc/spyglass_scope.png` as a centred square of side `min(w, h) * scopeScale` with black around it; `scopeScale` moves `0.5 * deltaTicks` of the way to 1.125 every frame and is reset to 0.5 when not scoping. `MouseHandler` turns at 1/8 of the usual speed while scoping in first person. `FirstPersonHandsAndItemsRenderer.submitArmWithItem` draws no arms or items while scoping.
- **Changed:**
  - `minecraft_survival` `spyglass.rs` (new): `Scoping {slot, ticks}` on the player's `Hand` (per player, server side), `Scoping::start`, and `step`, which keeps scoping while the use key is held and the spyglass stays in that hand, and returns whether the stop sound plays (released or 1200 ticks: yes; switched away, dropped or dead: no). `offhand::has_use` counts the spyglass, so it works from the offhand too.
  - `render_anim` `minecraft_world.rs`: a spyglass use starts scoping with the use sound; scoping makes the hand busy (no punching, digging or other uses), slows walking like any item use, blocks MW2 fire and aim through `offhand_busy` and hides the gun, hides the first-person hands and items, and ends with the stop sound. `ui.scoping` tells the camera and the HUD; the scope texture is loaded with the other overlays.
  - `render_anim` `minecraft_spyglass.rs` (new) and `occupancy/view_kick.rs`: in first person the vertical FOV is multiplied by the smoothed modifier and look speed by 1/8 while scoping. Third person (F5) neither zooms nor shows the overlay, as in vanilla.
  - `hud` `minecraft_spyglass.rs` (new): the scope overlay below the hotbar and the frost outline, with vanilla's growing scale.
- **Tests:** `minecraft_survival` `spyglass::tests` (only a spyglass in the used hand starts; letting go stops with the sound, switching away, a changed slot or death stop quietly; the offhand counts; 1200 ticks finish like letting go), `offhand` `has_use` now asserts the spyglass; `render_anim` `minecraft_spyglass::tests` (the modifier halves its distance each tick and back, is frame-rate independent, and scales the vertical FOV by exactly 0.1); `hud` `minecraft_spyglass::tests` (the scale grows 0.5 to 1.125 by half a tick per frame, the 16:9 layout with black bars left and right, a small scope framed on four sides).
- **Decisions:**
  - The modifier is stepped continuously per frame (`0.5^(seconds * 20)`), which equals vanilla's per-tick halving with its partial-tick lerp closely enough and stays smooth at any frame rate.
  - The MW2 FOV setting is horizontal at 4:3; the zoom scales the matching vertical FOV by 0.1, like vanilla scales its vertical FOV.
  - The scope's growth uses vanilla's frame formula with the step clamped to 1, so a long frame can't overshoot.
  - Third person: the MW2 body has no Minecraft arm poses, so it does not raise the spyglass to its eye (vanilla's `SPYGLASS` arm pose). The use sound, the slowdown and the stop still happen.
  - Not done (vanilla client-only effects without a player-facing change here): the `minecraft:used` statistic and the spyglass advancements (no statistics or advancements yet).
- **Owner's test steps** (survival, kit below, first person):
  1. Select the spyglass in the hotbar and hold right-click: a quick zoom in to about ten times, the round spyglass overlay grows in with black bars left and right, and the spyglass use sound plays. The hands and the spyglass are not drawn, and walking is slow. **Screenshot 1.**
  2. Move the mouse while zoomed: the view turns much slower than normal.
  3. Let go: the view zooms back out over a few frames, the overlay is gone and the stop sound plays.
  4. Hold right-click again and press 1 to 9 to another slot while zoomed: zoom and overlay end at once, with no stop sound.
  5. Hold right-click and press F5: third person, no zoom and no overlay. F5 twice back to first person while still holding: zoom and overlay again. **Screenshot 2** (third person).
  6. Put the spyglass in the offhand (F with it selected), select an empty hotbar slot and hold right-click: it zooms from the offhand as well.
  7. Select a gun with the spyglass still in the offhand and use the offhand (hold F today; skip this step if F4 has changed the key): the gun is lowered, does not fire or aim while zoomed, and comes back when you let go.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:spyglass,minecraft:spyglass
```

## F3. Every spawn egg of a mob the game has hatches it

Commit: the one after `d17eaf15`.

- **Cause:** the cave spider was not simulated at all. An egg asks the server to summon its mob, and the server answers "not simulated" for a kind it has no entity for, so the egg does nothing and is kept. The zombie egg worked because zombies are simulated. Mineshaft spawners only showed a cave spider in the cage.
- **Survey:** a new test uses every one of the 88 `*_spawn_egg` items of the 26.3 pack on a block and checks what appears. Before this commit, 34 eggs hatched their mob. The others found:
  - **Cave spider:** did nothing (above).
  - **Ender dragon:** did nothing. The egg went to the mob world, which has no dragon, though `/summon minecraft:ender_dragon` worked.
  - **Horse and donkey:** they hatched, but the entity world named them cows. That name could reach kill messages and fireball blame.
  - **Jockeys:** a spider egg rolls a skeleton jockey one time in 100, and a baby zombie can roll a chicken jockey. The spider jockey left nothing (the egg was kept), and the chicken jockey hatched only the chicken.
- **Changed:**
  - `minecraftoss-entities` `spider.rs`: `Spider::cave` (`EntityTypes.CAVE_SPIDER`: 0.7 by 0.5, eyes at 0.45; `CaveSpider.createCaveSpider`: 12 health), with `max_health`, `type_id` and `poison_ticks` (`CaveSpider.doHurtTarget`: poison for 7 seconds on normal and 15 on hard, none on easy). `world.rs`, `villagers.rs`, `witches.rs`: the spider's own kind, health cap, loot table (`minecraft:cave_spider`) and census name. A cave spider's bite on a mob poisons it. `entity_type` names horses and donkeys by their kind. `monster_ai.rs`: iron golems hunt cave spiders (`Enemy`).
  - `minecraft_terrain`:
    - `server_mobs.rs` loads and saves `minecraft:cave_spider` and lists it as simulated, so `/summon` and spawners make it too.
    - `spawn_unmounted` hatches an egg's jockey as its two mobs side by side.
    - `server.rs` `hatch_egg` hands a dragon to the dragons, like `/summon`, and uses `spawn_unmounted`.
    - `spider_render.rs` draws a cave spider as the spider model scaled by 0.7 about the feet with `entity/spider/cave_spider` (`LayerDefinitions`: `MeshTransformer.scaling(0.7)`). `client_mobs.rs` gives it a 0.56 shadow (`CaveSpiderRenderer`).
    - `spawn_egg.rs` uses its 0.7 width for the spawn height.
  - `minecraft_survival`:
    - `death.rs`: `melee_effect` takes the difficulty and gives poison for a cave spider's bite.
    - `survival.rs` passes the difficulty.
    - `entities.rs`: cave spiders count as monsters near a bed under their own name.
  - `docs/MINECRAFT-SURVIVAL.md`: cave spiders in the simulated list, and the egg notes.
- **Tests:**
  - `minecraft_terrain`:
    - `server::tests::every_spawn_egg_of_a_mob_the_game_has_hatches_that_mob` (with the 26.3 pack): all 88 eggs are aimed and hatched. Exactly the 36 listed below hatch a mob of their own kind, and every other egg leaves no mob, no dormant entry and no dragon behind.
    - `server_mobs::tests::a_hatched_jockey_stands_its_rider_beside_its_mount`: spider and chicken jockeys come out as two mobs. A strider still carries its rider. A zombie horse's rider does not come without its horse. A cave spider saves as a cave spider with its health.
    - `spider_render::tests::a_cave_spider_is_the_spider_model_at_seven_tenths_with_its_own_skin`.
    - `spawn_egg::tests::an_iron_golem_is_as_wide_as_its_body` now also checks the cave spider's width.
  - `minecraftoss-entities` `world::tests::a_cave_spider_is_a_small_spider_with_its_own_kind_loot_and_poison`.
  - `minecraft_survival` `death::tests::a_cave_spider_bite_poisons_for_seven_seconds_on_normal_and_fifteen_on_hard`.
- **Eggs that hatch their mob (36):** bat, blaze, bogged, cave spider, chicken, cow, creeper, donkey, ender dragon, enderman, ghast, hoglin, horse, husk, iron golem, magma cube, mooshroom, parched, pig, piglin, piglin brute, sheep, shulker, skeleton, slime, spider, stray, strider, villager, witch, wither skeleton, wolf, zoglin, zombie, zombie villager, zombified piglin.
- **Eggs of mobs the game doesn't have yet (52):** each one does nothing and is kept, as before. They are allay, armadillo, axolotl, bee, breeze, camel, camel husk, cat, cod, copper golem, creaking, dolphin, drowned, elder guardian, endermite, evoker, fox, frog, glow squid, goat, guardian, happy ghast, llama, mule, nautilus, ocelot, panda, parrot, phantom, pillager, polar bear, pufferfish, rabbit, ravager, salmon, silverfish, skeleton horse, sniffer, snow golem, squid, sulfur cube, tadpole, trader llama, tropical fish, turtle, vex, vindicator, wandering trader, warden, wither, zombie horse and zombie nautilus. None is faked with another mob's model.
- **Decisions:**
  - **Cave spiders are real cave spiders,** built on the spider, because vanilla's `CaveSpider` is a `Spider` with these differences only:
    - its size, eye height and 12 health;
    - its `doHurtTarget` poison;
    - a `finalizeSpawn` that skips `Mob.finalizeSpawn`, so it gets no jockey, no spider effect and no follow-range bonus (the summon path already returns the plain fresh tag for it);
    - its renderer's scale, skin and shadow.
    - It keeps the spider's sounds, speed, attack and climbing, as in vanilla.
  - **Jockeys rolled by an egg** hatch both mobs side by side, the rider at the mount's spot. Only striders carry riders here, and a kept egg that does nothing would be worse. Naturally spawned spider and chicken jockeys still wait dormant as before. A zombie horse egg makes nothing, rider included, because the horse is not simulated.
  - **The dragon egg** goes the way of `/summon minecraft:ender_dragon` (its saved tag through the dragon list), so the dragon is the same as a summoned one.
- **Owner's test steps** (survival, normal difficulty, kit below):
  1. In a cave or at night, use a cave spider egg on the floor and a spider egg next to it. The cave spider is a small dark-blue spider about two thirds the spider's size, with red eyes. One egg is used each time. **Screenshot 1.**
  2. Let a cave spider bite you: the hearts turn green and you lose health for about 7 seconds, never below half a heart. Drink the milk to clear it. Switch to hard (`/difficulty hard`) and let it bite again: the poison lasts about 15 seconds. **Screenshot 2** (green hearts).
  3. Kill a cave spider with the sword: it drops string and sometimes a spider eye. Press F5: it is drawn small in third person too.
  4. Place the spawner in a dark spot with no torches and use a cave spider egg on it. The cage shows the small cave spider turning. Wait nearby: cave spiders come out of the spawner. **Screenshot 3.**
  5. Use each of the other eggs in the kit: a horse, a donkey, an iron golem, a villager, a wolf, a ghast, a shulker, a witch, a hoglin, a piglin, a strider, a mooshroom, a bat and a magma cube appear, each from its own egg.
  6. Use a zombie egg several times: a zombie comes out each time. Once in a while a baby zombie comes with a chicken next to it.
  7. Use the fox egg and the wither egg: nothing happens and they stay in the hotbar (those mobs are not in the game yet).
  8. Away from your base, high up in open air: use the ender dragon egg on a block. An ender dragon appears there, as with `/summon minecraft:ender_dragon`. **Screenshot 4.**
  9. Optional, creative (`/gamemode creative`): open the spawn eggs tab and try any egg from the list of 36 above. Each one hatches its mob.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:cave_spider_spawn_egg*16,minecraft:spider_spawn_egg*4,minecraft:zombie_spawn_egg*8,minecraft:horse_spawn_egg,minecraft:donkey_spawn_egg,minecraft:iron_golem_spawn_egg,minecraft:villager_spawn_egg,minecraft:wolf_spawn_egg,minecraft:ghast_spawn_egg,minecraft:shulker_spawn_egg,minecraft:witch_spawn_egg,minecraft:hoglin_spawn_egg,minecraft:piglin_spawn_egg,minecraft:strider_spawn_egg,minecraft:mooshroom_spawn_egg,minecraft:bat_spawn_egg,minecraft:magma_cube_spawn_egg,minecraft:ender_dragon_spawn_egg,minecraft:fox_spawn_egg,minecraft:wither_spawn_egg,minecraft:spawner,minecraft:milk_bucket,minecraft:iron_sword
```

## F4. G throws the lethal grenade and Shift+G the tactical

Commit: the one after `f5db780f`.

- **Before:** `G +frag` and `B +smoke` (round 7, track A). The owner's `settings.cfg` (v4) had exactly that, so B threw the tactical in code, but nothing in the game said so and the owner found no key for the flash. The bind system had no chords, so Shift+G could not be bound.
- **Changed** (all in `console`):
  - `binds.rs`: a new `BindButton::Shifted(KeyCode)`, a key pressed while either Shift is held.
    - `BindInputs::with_shifted_keys` takes the keys that have a Shift chord in either layer. For such a key, a press with Shift held goes to the chord and a press without Shift to the plain key. A key without a chord ignores Shift as before, so Shift+W still sprints forward.
    - Which binding a press fires is decided when the key goes down. The binding then stays held until the key is let go, so pressing or letting go of Shift while a grenade is cooking changes nothing.
    - `KeyBinds::move_tactical_to_shift_g` moves `+smoke` from B to Shift+G when the keyboard tactical is exactly B and Shift+G is free in both layers. `KeyBinds::shifted_keys` lists the chorded keys.
    - `captured_key` is the controls menu's key capture. A key pressed while Shift is held binds the chord. Shift on its own waits and binds Shift when it is let go, so Shift can still be bound alone.
  - `binds/key_names.rs`: `SHIFT+<key>` parses (any case) and displays as `SHIFT+G`. Each chord has its own host key number, 75 above its key (76 to 149, below the mouse at 180), so it can hold its own MW2 binding. `shift+shift`, a mouse button, the wheel or a pad button after `shift+` is refused.
  - `assets/default_controls.cfg`: `bind SHIFT+G +smoke` replaces `bind B +smoke`. `SHIFT +breath_sprint` and `F mc_swaphands` are unchanged, and the pad layouts too (RB lethal, LB tactical).
  - `user_settings.rs`: `settings.cfg` is written as v6. A v1 to v5 file gets `move_tactical_to_shift_g` once and is rewritten at start, after the older F/G move for v1 to v3. The controls menu uses `captured_key`.
  - `plugin/mod.rs`: the gameplay input reads the chords.
  - `docs/MINECRAFT-SURVIVAL.md`: the grenade keys and the v6 note.
- **Tests** (`console`):
  - `binds`:
    - `the_defaults_put_the_grenades_on_g_and_shift_g_and_leave_f_to_swap_hands` replaces the G-and-B test.
    - `the_tactical_moves_from_b_to_shift_g_only_when_untouched`: the move happens once. It does not happen with H, MOUSE4, an MW2 or Minecraft binding on Shift+G, or an unbound B.
    - `shift_chords_parse_display_list_and_load_back`.
    - `every_shift_chord_has_its_own_host_key_number_below_the_mouse`.
    - `shift_g_throws_the_tactical_and_g_alone_the_lethal`: both Shift keys, Shift pressed in the same frame, Shift+W, Shift pressed while G cooks, letting go of Shift before G, and plain G with Shift when no chord is bound.
    - `shift_g_keeps_its_throw_and_takes_no_minecraft_action`.
    - `the_controls_menu_takes_shift_and_a_key_as_one_chord`.
  - `user_settings`:
    - `the_tactical_on_b_moves_to_shift_g_once`: v4 and v5 move, and the owner's own v4 binding set loads back the same. MOUSE4, Shift+G taken and v6 are left alone.
    - `old_grenade_keys_move_to_g_and_shift_g`: the v1 to v3 F/G pair ends up on G and Shift+G, saved as v6.
    - Updated: `only_a_moved_grenade_pair_asks_for_the_file_to_be_rewritten` (a v5 B tactical asks, v6 does not), `older_settings_get_the_vanilla_minecraft_keys`, `current_settings_keep_their_minecraft_keys_unbound` (v6 too), `a_rebound_grenade_key_survives_a_restart`, `a_rebound_minecraft_key_survives_a_restart`, `older_settings_get_the_skate_key_on_a_free_j`, `a_rebound_skate_key_survives_a_restart` (v6 header), and the controls-menu test (`ui_bind_+smoke` is `SHIFT+G | L-BUMPER`).
- **Conflicts checked:**
  - No other default binding is on G or Shift+G in either layer. No game code reads G raw.
  - Shift is read raw only in places where the grenade keys don't throw:
    - the inventory's shift-click and chat text selection: a Minecraft screen or the chat is open, and gameplay input is captured;
    - the skate keyboard: the cmd buttons are zeroed while skating;
    - the debug fly camera.
  - Vanilla 26.3's `keyQuickActions` is G. It opens only a dialog the server sends, and there is none here (round 7 javap check).
- **MW2 behaviour:**
  - The keys are the same on MW2 maps. Shift stays `+breath_sprint`.
  - Shift+G while sprinting forward throws: `sprint_ending_buttons` (mask 0xcf35, which includes SMOKE) ends the sprint in `pmove`, and that runs before the weapon tick checks the offhand. Holding G keeps a new sprint from starting (0xcc35). Nothing MW2 needs breaks.
- **Decisions:**
  - **Only Shift chords**, the one modifier the owner asked for. Any key can take one, through the console (`bind SHIFT+H +smoke`) or the menu.
  - **B is freed** rather than kept as a second tactical key, so the layout is exactly the owner's. The move needs the untouched B tactical and a free Shift+G, like the round 7 move. Any other choice is kept.
  - **F only swaps hands:** with a Minecraft item or an empty hand, F swaps like vanilla. With a gun selected, holding F still uses the offhand item (eat, block, place), as since round 7. A gun can't go to the offhand, so F has nothing to swap there, and it is the only way to use the offhand item while holding a gun. F has no grenade role.
  - A `settings.cfg` saved by upstream's `iw4l.exe` would drop the `SHIFT+G` line it doesn't know. This build never writes for upstream, so this is noted only.
- **Not run in game.** The chord logic is covered by the unit tests above.
- **Owner's test steps** (survival, kit below, a class with Frag and Flash):
  1. Start once with your current `settings.cfg` and quit. `iw4l-artifacts\settings.cfg` starts with `// IW4L user settings v6`. It has `bind g +frag`, `bind SHIFT+G +smoke` and `bind f mc_swaphands`, and no `bind b +smoke`.
  2. Options > Controls > Actions: "Throw Frag/Use Equipment" shows `g | R-BUMPER` and "Throw Special Grenade" shows `SHIFT+G | L-BUMPER`. **Screenshot 1.**
  3. In the Minecraft world with a gun selected:
     - Press G: a frag flies.
     - Hold Shift and press G: a flash flies, and no frag.
     - Hold G to cook a frag, press Shift while still holding it, then let go: the frag is thrown, not a flash.
  4. Select cobblestone and press Shift+G: MW2's hand comes up with the flash and throws it, and the cobblestone comes back. **Screenshot 2** mid-throw. Select an empty slot: G throws a frag and Shift+G a flash.
  5. Sprint (Shift+W) and, still holding Shift, press G: the sprint ends and a flash flies. Let go of G and walk with Shift+W: you sprint as before.
  6. Press F with cobblestone selected: it goes to the offhand and nothing is thrown. Put the golden apples in the offhand, select the gun and hold F: you eat, as before.
  7. Options > Controls > Actions, click "Throw Special Grenade", hold Shift and press H: the row shows `SHIFT+H`. In game, Shift+H throws a flash, and Shift+G now throws a frag (G has no chord any more). Restart: the row still shows `SHIFT+H`. Click it again, hold Shift and press G to set it back.
  8. On an MW2 map (`iw4l-survival.exe map mp_rust`): G throws a frag and Shift+G a flash. **Screenshot 3.**
  9. Optional, controller: RB throws a frag and LB a flash, as before.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:cobblestone*64,minecraft:golden_apple*16
```
