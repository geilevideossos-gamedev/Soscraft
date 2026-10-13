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
