# Track END log: the End

Branch `end`, worktree `C:\Dev\mc-wt\end`. Briefs: `e3-end.md` (commits E3-1 to E3-29), E2-4 of `e2-nether.md` (the Nether sky, moved here) and `e0-phase3-plan.md` (groups X1a, X1b, X2, ...). Base: 3aef2ebe (the dimension switch merged into `minecraft-survival`).

A commit can't name its own hash, so each entry names the previous commit's hash; the last entry's hash is in `git log` under its subject.

## Group X1a: End pieces that need no travel

### 1. Minecraft end city shulkers are no longer deleted far from players (E3-1)

Previous commit: 3aef2ebe

- **Defect:** mobs the entity world doesn't simulate yet wait dormant on the server thread (`minecraft_terrain/src/server.rs`, `ServerSim.dormant`). Every tick, a dormant mob of the monster category was deleted when it was more than 128 blocks from every player, or in peaceful. Shulkers are of the monster category, so the shulkers an end city places, and the ones saved in its chunks, were deleted as soon as the player was 128 blocks away, and the chunk was saved without them.
- **Changed:**
  - New `minecraft_terrain/src/dormant.rs`: `despawns(kind, persistent, tag, peaceful, nearest_squared)` is vanilla's `Mob.checkDespawn` for a dormant mob, with the two rules looked up by type: `allowed_in_peaceful` (`EntityType.isAllowedInPeaceful`) and `removed_when_far_away` (`Mob.removeWhenFarAway`).
  - `minecraft_terrain/src/server.rs` (`despawn_mobs`, the dormant part only): the dormant list is filtered through `dormant::despawns`. Mobs of other categories stay, as before.
- **Checked with javap (26.3):**
  - `Shulker extends AbstractGolem`, and `AbstractGolem.removeWhenFarAway` returns false. `Warden.removeWhenFarAway` returns false too. `EnderDragon.checkDespawn` does nothing, and `WitherBoss.checkDespawn` only removes it in peaceful.
  - `ZombieVillager.removeWhenFarAway` is false while it converts or once it has trade XP.
  - The monster-category types built without `notInPeaceful()` in `EntityTypes`: camel husk, ender dragon, piglin, shulker, sulfur cube, zombie horse and zombie nautilus. So shulkers also stay in peaceful.
  - `Mob.checkDespawn` removes a mob in peaceful before it looks at persistence, so a name-tagged dormant zombie still goes in peaceful (it stayed before).
- **Tests:** `dormant`: `shulkers_stay_far_from_players_and_in_peaceful`, `plain_monsters_go_beyond_128_blocks_and_in_peaceful`, `peaceful_keeps_the_vanilla_exceptions_but_far_away_still_removes_them`, `a_curing_or_traded_zombie_villager_stays`, `other_categories_are_left_alone`. `server`: `dormant_shulkers_stay_far_from_players_while_dormant_monsters_go` (a summoned shulker and pillager wait dormant; with the player 3000 blocks away the pillager goes and the shulker stays, also in peaceful).
- **Decisions:**
  - The idle despawn (one in 800 after 600 idle ticks) still never applies to dormant mobs, since they don't tick.
  - A raider in a raid (`Raider.removeWhenFarAway`) is not handled, as there are no raids yet.
- **Test steps:** shulkers aren't drawn until the outer End group (X6), so this can't be seen in game yet; the unit tests cover it. Optional, with vanilla 26.3 Java:
  1. In the `.env`, set `IW4L_MINECRAFT_WORLD=x1atest`. Start, and type `/summon minecraft:shulker ~ ~ ~2`.
  2. Type `/tp ~400 ~ ~`, wait 10 seconds, then `/tp ~-400 ~ ~`, and quit through the menu.
  3. Open `x1atest` in vanilla 26.3: the shulker sits where you summoned it.
- **Kit:** none needed.
