# Track NETHER log: the Nether

Branch `nether`, worktree `C:\Dev\mc-wt\nether`. Briefs: `e2-nether.md` (commits E2-1 to E2-30) and `e0-phase3-plan.md` (groups N1-N5). Base: 3aef2eb (the dimension switch merged).

A commit can't name its own hash, so each entry is written with its commit and its hash is filled in by the next commit; the last entry's hash is in `git log` under its subject.

## Group N1: a Nether level that runs

E2-1, E2-2, E2-3, E2-5 and E2-12. E2-4 (no sky, sun, stars or clouds) moved to the End track's X1b.

### 1. Minecraft levels know their dimension's rules (E2-1)

Commit: see `git log` (the next entry names it).

- **Changed:**
  - New `minecraft_survival/src/dimension.rs`: `DimensionRules` read from the 26.3 data pack for a `Dimension`: `coordinate_scale`, `min_y`, `height`, `logical_height`, `has_skylight`, `has_ceiling`, `nether_cardinal_light` (`cardinal_light` is `nether`), the `gameplay/bed_rule` as a `BedRule` (`can_sleep`, `can_set_spawn` as `BedRuleTest` always, when dark or never, `destroy_on_use`, `destroy_on_leave`, the error message's translation key), `respawn_anchor_works`, `water_evaporates`, `fast_lava` and `nether_portal_spawns_piglin`.
  - `minecraft_terrain/src/environment.rs`: `dimension_value(system, name)` is vanilla's `EnvironmentAttributeReader.getDimensionValue`: the dimension type's value over the default, without any biome layer.
  - `minecraft_survival/src/level.rs`: `Terrain.rules`, loaded by `Terrain::open` on the loader thread. A dimension whose rules can't be read fails to open like one whose environment can't.
- **Tests:** `dimension`: `a_dimension_type_gives_its_scale_heights_lighting_and_gameplay_rules` (synthetic Nether-like type), `a_dimension_without_a_bed_rule_sleeps_when_dark_and_sets_the_spawn` (the default `BedRule.CAN_SLEEP_WHEN_DARK`, and the three `BedRule$Rule` tests), `the_three_vanilla_dimensions_have_their_own_rules` (the real 26.3 data under `MINECRAFTOSS_ROOT`: overworld 1:1, 384 high from -64, beds sleep when dark, portal piglins; Nether 8:1, logical height 128, cardinal light, beds and straw beds explode, anchors work, water evaporates, fast lava; End 1:1, 256, beds explode). `environment`: `a_dimension_value_ignores_the_biomes`.
- **Decisions:**
  - The plan names `Level.rules`; the rules sit on `Level.terrain.rules` instead, because `Terrain::open` is where a level can still fail to open and because the portal search in N2 runs in `prepare`, which only gets the `Terrain`.
  - The gameplay rules are read as dimension values, as vanilla reads them (`getDimensionValue` for `fast_lava`, checked with javap on `LavaFluid`). No 26.3 biome overrides any of them.
- **Test steps:** nothing changes in game; the rules are used by the next entries and by N2 and N5.
- **Kit:** none.
