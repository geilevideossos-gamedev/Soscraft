# Round 9, track B log: items and the screen

Brief: `docs/round9/track-b.md`. Branch `track-b`, worktree `C:\Dev\mc-wt\track-b`, target dir `D:\iw4l-wt\track-b`, base 24dfdf71.

## Commits

### B10.1 A Minecraft hit shows a faint half-second red flash instead of MW2's lasting low-health blood

Base: 24dfdf71.

What changed (`crates/hud/src/blood.rs`):
- `BloodOverlayLatch` holds a `HurtFlash { last_event, hit_at }`. Its pure `observe(damage_event, now_ms)` takes the first damage count it sees as the baseline (no flash on joining or respawning), starts a flash whenever the replicated `ps.damage_event` changes (compared with `!=`, so a wrapped counter still counts) and returns `mc_hurt_flash(now - hit_at)`.
- `mc_hurt_flash(elapsed_ms)` is `MC_HURT_FLASH_PEAK * sin(t^4 * PI)` with `t = 1 - elapsed / 500`, and 0 outside 0..500 ms. `MC_HURT_FLASH_MS = 500`, `MC_HURT_FLASH_PEAK = 0.3`. The curve is vanilla's hurt-tilt curve (`GameRenderer.bobHurt`), so the flash peaks about 80 ms after the hit.
- `update_blood_overlay` reads `Option<Res<frame::MinecraftUi>>`. On a Minecraft map (`ui.active || ui.credits.is_some()`) the overlay alpha is the flash; health no longer feeds it. On MW2 maps `blood_overlay_lerp` runs exactly as before.
- The flash is reset on `LifeStarted`, when there is no player state, outside the game and when Show Blood is off. The `should_draw_blood_overlay` gate is unchanged, so Show Blood off hides the flash.

Tests added (hud): `the_first_damage_count_seen_does_not_flash`, `a_hit_flashes_faintly_and_fades_within_half_a_second`, `the_flash_peak_stays_far_below_the_mw2_low_health_overlay`, `a_wrapped_damage_counter_still_flashes`, `low_health_alone_shows_no_flash_on_minecraft`, `a_new_life_forgets_the_last_hit`. The two Show Blood tests are unchanged and green.

Decisions:
- **Not vanilla:** vanilla 26.3 has no full-screen red on a hit or at low health (`Hud.extractVignette` reads only the world border). The faint flash keeps MW2-style feedback at the owner's request. The peak is one constant, `MC_HURT_FLASH_PEAK`, for tuning.
- While the overlay is hidden for another reason (HUD hidden, a menu that hides the HUD, dead or spectating pm types) with Show Blood on, the flash still follows the damage count. A hit taken while hidden then shows at most the rest of its half second when the HUD comes back, instead of a full flash that starts late.
- An old `hit_at` is dropped once its half second is over, so a frame clock that restarts (a new map) cannot replay an old flash.
- Client-side only, nothing on the wire: the host and each friend read their own replicated `ps.damage_event` and their own `MinecraftUi.active`, so each window flashes only for its own hits, and Show Blood is per PC.

### B10.2 No MW2 pain vision on Minecraft maps

Base: f7cfddd4 (B10.1).

What changed (`crates/render_frontend/src/assemble/drawsurf/film_vision_view.rs`):
- `update_film_vision_view` reads `Option<Res<frame::MinecraftUi>>`.
- The pure `pain_wanted(health, scripted, minecraft)` is true only for a scripted pain vision at half health or less, off Minecraft maps. On a Minecraft map `pain_strength` stays 0, so the pain vision never blends in. MW2 maps keep the old rule, including the fade back down while healing.

Tests added (render_frontend): `no_mw2_pain_vision_on_minecraft_maps` (`(Some(0.1), true, true)` false, `(Some(0.1), true, false)` true, `(Some(0.9), true, false)` false, plus no health and no script both false).

### B9.1 Minecraft potion and tipped arrow icons show their vanilla potion colour

Base: 05578b70 (B10.2).

What changed:
- `minecraft_terrain/src/potions.rs`: `stack_color(components)` is `Some(PotionContents::from_components(..).color())` when the stack has `minecraft:potion_contents` (string or object form), else `None`. The colour maths is unchanged.
- New `minecraft_terrain/src/item_tint.rs`: an icon key is the item id plus `~rrggbb` when the stack has potion contents (`key`, `split`, `of_stack`, `KEY_MARK = '~'`). `split` takes the last `~`, so a dial key (`#`) stays whole for `dials::split` afterwards; a bad or empty suffix gives the id with no colour.
- `interface.rs` `item_model_tints(packs, model, potion)`: the `minecraft:potion` tint uses the stack colour, or its `default` (0x385DC6) without one. Dye, firework, map colour, team and custom model data still use their default. `mesh.rs` passes `None` at both callers for now (the world commit is next).
- `item_icons.rs` `item_icon_at`: splits the tint, then the dial, and tints layer 0 with the colour, so the potion overlay and the tipped arrow head take it.
- `frame::McStack.icon`: the icon atlas key. `hud/src/minecraft_gui.rs` looks icons up by `stack.icon`; names stay keyed by `stack.id`.
- `render_anim/src/minecraft_inventory.rs`: `convert` sets `icon = item_tint::of_stack(s)` and collects icon keys. The atlas loop names items by the base id and renders each icon from `item_tint::key(dials.key(base), potion)` (the small `icon_source` helper); the animated refresh uses `of_stack` too. `minecraft_creative.rs` `icon_ids` returns `stack.icon`. The gun handling is untouched.
- The friend's inventory goes through the same `publish`, so friends get the colours too.

Tests added:
- minecraft_terrain: `a_stack_colour_needs_potion_contents` (None without the component, F82423 for healing in both forms, a custom colour, water's default, strong_turtle_master 0x8D85E6); `item_tint::a_key_splits_back_into_its_id_and_colour`, `a_dial_key_is_left_for_the_dial_split`, `a_bad_suffix_has_no_colour`, `a_poison_splash_potion_keys_its_colour` (`minecraft:splash_potion~87a363`); `interface::a_potion_tint_takes_the_stack_colour_or_its_default` and two dye/firework asserts that ignore a colour; `item_icons::potion_and_tipped_arrow_icons_take_the_stack_colour_on_layer_zero` (on the real 26.3 pack: tinted differs from plain, and every bare layer-0 pixel equals the sprite times F82423/255, for the potion overlay and the tipped arrow head).
- render_anim: `two_potions_share_an_id_and_a_name_but_not_an_icon`.

Decisions:
- Every stack with potion contents gets its own atlas cell, even when its colour is the default (water, awkward). That is vanilla's tint result and costs one cell per potion kind; `ICON_ATLAS_LIMIT` has room.
- Dyed leather keeps its default tint (out of scope in the brief; the same key can carry it later).

## Owner's test steps (B9.1)

1. **Inventory (E).** Each bottle has its own liquid colour:
   - Healing is red, Poison olive green and Swiftness cyan;
   - Fire Resistance is orange, Night Vision lime and Strong Turtle Master lavender blue;
   - Invisibility is almost white, Leaping pale yellow and Weakness dark grey;
   - Water and Awkward keep the old blue.

   The splash and lingering bottles and the tipped arrows' heads are coloured the same way. Hovering still shows each potion's name.
2. **Hotbar and cursor.** Potions in the hotbar and one carried on the cursor keep their colours.
3. **Creative.** In `/gamemode creative`, the potions in Food & Drinks and the tipped arrows in Combat each show their own colour.

Held, dropped and framed potions still show the old blue until the next commit (B9.2).

```
IW4L_MINECRAFT_KIT=minecraft:potion@healing,minecraft:potion@poison,minecraft:potion@swiftness,minecraft:potion@fire_resistance,minecraft:potion@night_vision,minecraft:potion@strong_turtle_master,minecraft:potion@invisibility,minecraft:potion@water,minecraft:potion@awkward,minecraft:potion@leaping,minecraft:potion@weakness,minecraft:splash_potion@harming*2,minecraft:splash_potion@regeneration*2,minecraft:splash_potion@poison,minecraft:lingering_potion@poison*2,minecraft:lingering_potion@slow_falling,minecraft:tipped_arrow@poison*16,minecraft:tipped_arrow@strength*16,minecraft:tipped_arrow@leaping*16,minecraft:item_frame*4,minecraft:cow_spawn_egg*2
```

## Owner's test steps (B10)

1. Options > Advanced Video: check that Show Blood is on.
2. **One hit.** Let a zombie hit you once: a faint red flash, gone in about half a second, with the view kick and the blinking hearts.
3. **Low hearts.** Go down to 1-2 hearts with the zombie and a cactus. Between hits the screen is completely clear (the screenshot 11 case).
4. **Standing at 1 heart.** Stand still at 1 heart for 10 seconds: the screen stays clear, with no red and no pain vision, and only the hearts jitter.
5. **Healing.** Eat golden apples: nothing changes on screen.
6. **Show Blood off.** Turn Show Blood off and step onto a magma block: no flash. Turn it back on.
7. **The totem.** Hold the totem and survive a lethal fall: at most one short flash.
8. **Respawning.** Die and respawn: no flash is replayed.
9. **A friend.**
   - When the friend is hit, only the friend's window flashes.
   - The friend's screen is clear at 1 heart.
   - The friend's own Show Blood switch works only on their PC.
10. **MW2.** A plain MW2 map still has MW2's low-health blood and pain vision.

```
IW4L_MINECRAFT_KIT=minecraft:zombie_spawn_egg*8,minecraft:cactus*4,minecraft:magma_block*4,minecraft:golden_apple*16,minecraft:cooked_beef*32,minecraft:totem_of_undying,minecraft:cobblestone*64,minecraft:ladder*32,minecraft:diamond_sword,minecraft:shield
```

## State doc changes

`docs/MINECRAFT-SURVIVAL.md` line 431 (MW2 perks and scripts that heal or resize health): keep the first sentence and replace the last one ("Since MW2's maximum health stays at its spawn value, MW2's blood overlay and pain vision stay on while the hearts are low.") with:
"MW2's low-health blood overlay and pain vision no longer show on Minecraft maps, for the host or for friends; MW2 maps keep both. Instead a hit gives a faint red flash of about half a second (MW2-style feedback, not vanilla: vanilla has no screen red), and Show Blood (Options > Advanced Video) turns it off."
