# Elden Ring bosses in Minecraft worlds

Round 10, track F. This is the design for roadmap step 3 ("One Elden Ring boss as a proof of concept"). Track F builds the proof of concept from it; its brief is `docs/round10/track-f.md`.

ELDEN RING, Margit and the names of his moves belong to FromSoftware and Bandai Namco. This is a private fan fork. Nothing from the game is in this repository, and nothing ever will be. The proof of concept uses our own model, our own numbers and vanilla Minecraft sounds read at runtime. Real game data, later, is read from the owner's own copy of the game and never committed, the same rule as for the Mojang files.

**Status, 2026-10-28:** design written, proof of concept not built yet. ELDEN RING is not installed on this PC (no Steam install on C: or D:), so nothing could be read from the game. Every number for Margit below is our own estimate, and the data says so (`DataSource::Estimate`). Section 14.2 says what real data would need.

Paths are as of 6e0e0573 (the round 10 track-d merge). Vanilla facts were checked with javap on the 26.3 `client.jar`.

---

## 1. The decisions on one page

| Question | Decision |
|---|---|
| First boss | **Margit, the Fell Omen** (section 3) |
| Where the boss lives | A mob of the host's level, in the shared mob simulation (`EntityWorld`), plugged in through track D0's mob registry. His fight logic lives in a new pure crate, `crates/elden_boss`, with no engine types and no game files (sections 2.1, 4) |
| How a boss is described | As data: a table of moves with wind-up, active and recovery windows, tracking stops, random holds, root motion, hit shapes, thrown daggers, conjured weapons, sound and particle cues, key poses and follow-ups. A later boss is mostly a new table plus a model |
| Model and animation now | Our own posed-cube model, a skin painted by code, key poses timed by the move table and played on the shared keyframe system |
| Real game data later | A baker of our own reads the owner's unpacked game files and writes a file next to the exe. That file replaces the hand-written table. It is never committed (section 14.2) |
| MW2 guns | Bullets and explosives hit his box and go through his own resistances. Every bullet counts: player hits skip the vanilla hurt cooldown (section 7) |
| Minecraft weapons | Swords, axes, bows, crossbows, tridents and TNT work with vanilla damage, crits and enchantments |
| Stagger | A stance bar. When it breaks he kneels for 4 seconds and takes more damage, and the first Minecraft melee hit from the front is a critical |
| Phases | Two. The second starts at half health: he conjures a golden hammer and gains new moves |
| Boss bar | The vanilla top bar, red, named "Margit, the Fell Omen", for every player within 48 blocks while he fights. The boss music plays |
| Multiplayer | The host runs him. Friends see every move through `MobShow`. His health grows with the players present when the fight starts (x1.6, x2.2, x2.8) |
| Summon or find | Survival: put a lantern on two stacked gold blocks. Tests: `/summon iw4l:margit`. Later: a gate arena found in the world |
| Saving | In his chunk, as a vanilla `minecraft:marker` that carries `data.iw4l`, which vanilla keeps. Loading a world resets an unfinished fight |
| Death and reward | Gold, an enchanted golden apple and experience that grow with the number of fighters, and "GREAT ENEMY FELLED" on every fighter's screen |

---

## 2. How a boss fight fits our game

### 2.1 The Minecraft world
A boss is a mob of the level he stands in, like the wither. The host's level thread ticks him with the other mobs (`crates/minecraft_terrain/src/server.rs`, `EntityWorld::tick`). He walks on blocks with vanilla collisions and pathfinding. He is pushed, filed in chunk sections, saved with his chunk, and shown to every player who tracks his chunk.

**Why not a list of our own, like the dragon?** The dragon (`Dragons` in `crates/minecraft_terrain/src/ender_dragon.rs`) flies, ignores most blocks, and keeps its own list in the level server. A walking boss needs what the entity world already has: movement and collisions, navigation, pushing, hits from arrows, tridents and explosions, death and loot, and the friend mirror. A list of our own would have to copy each of them and route every hit to it, as the dragon's `end_hit` does. The wither showed in round 9 that a boss with its own AI fits the entity world. Track D0 makes a new mob family cost one line per registry, so the entity world is now the cheap way.

**Why a separate crate for the fight logic?** The entity world is the vendored MinecraftOSS port. The boss logic (choosing moves, timing them, phases, stance, arena rules) has nothing of Minecraft in it, and it is where real game data will plug in. `crates/elden_boss` holds it as plain Rust that depends only on `glam`. It is tested headless in seconds, like the reference's `sim` module. The family code in the entity world calls it every tick.

### 2.2 MW2 guns
- **Bullets.** MW2's trace finds his box (`EntityView::boxes`, the same boxes aim assist reads) and sends a mob shot with the shooter's id. MW2's own hit-location multiplier applies first, so a hit on the top of the box is a headshot.
- **His resistances.** The shot reaches him by a typed route (`Bullet` or `Blast`) instead of the generic mob attack, so his own resistance and stance rules apply (section 7).
- **One box.** The proof of concept gives him one box of 1.4 x 3.0 blocks. Several boxes on his bones (head, body, weapon) come with real hurtbox data later.
- **The MW2 knife** does not hurt any Minecraft mob today: its trace only reaches blocks and players (`crates/sim/src/combat.rs`). So it does not hurt him either. Making the knife hit mobs is its own later change (section 17).
- **Killstreaks** hit him as blasts.
- **No hit-stop.** The reference freezes both fighters for an instant when a blow lands. We never do that: it would freeze a host-run boss for every player.

### 2.3 Minecraft weapons
- Left clicks with swords and axes reach him through the normal mob path (`Entities::punch`, then the entity world's `player_attack_with`), with vanilla attack charge, crits, sweeps and enchantments.
- Arrows, tridents and thrown items hit him through the entity world's projectile code, through the generic arms D0 and track E2 add this round.
- TNT, creepers, end crystals, beds and respawn anchors hurt him through the level's explosions.
- Shields block his physical hits. His golden (holy) hits go through shields and armour (section 7.4).

### 2.4 Multiplayer
- The host runs the boss, his AI and every hit, like every other mob. Friends run no AI.
- A friend gets him from his saved tag (`MobAdd`) and gets his state every tick in `MobShow`: which move, how far into it, his phase, and the daggers in flight. So a friend sees the same pose at the same moment.
- Friends' bullets and swings reach the host with the friend's id. Damage, threat, kills and experience go to the right player.
- His health scales with the players in the arena when the fight starts (section 9).
- The boss bar reaches each player in range through the existing `ToClient::BossBar` messages. Friends keep any bar id, so the bar needs no new message.

### 2.5 Saving
The boss is saved with his chunk, in the vanilla 26.3 entity format, as a `minecraft:marker` carrying his state in `data.iw4l` (section 12). Vanilla keeps a marker and its `data`, so a world opened in vanilla 26.3 and then in our game again loses nothing. Loading a world resets an unfinished fight, as quitting does in Elden Ring.

### 2.6 What MW2 movement changes
Elden Ring is built on rolls with invincibility frames. MW2 movement on blocks has no roll. You run at about 5.3 blocks a second (MW2's 190 units a second at 36 units a block, `crates/sim/src/voxel.rs` `BLOCK`), sprint, jump, crouch and go prone (`docs/MINECRAFT-SURVIVAL.md`, "Movement on blocks"). So the boss is tuned for running, not rolling:
- every strike has a clear wind-up with a tell: his eyes flare gold, or a golden weapon appears;
- he stops turning toward his target 8 ticks before a melee strike lands (the reference's tracking stop), so running away from him in that moment makes him miss;
- shockwave rings can be jumped;
- the delayed swings Margit is known for punish panic, not slow reactions;
- thrown daggers fly straight at where you were when they left his hand.

No dodge roll is planned. If one comes later, it is a player feature for all Minecraft worlds, not part of a boss.

---

## 3. Which boss first

Scores are 1 to 5; higher is better for a first boss. Health values and move counts are what players remember, so treat them as approximate.

| Boss | Known | Buildable without game files | Size in blocks | Phases / moves | Telegraphs | Summon or find | Against guns | Multiplayer |
|---|---|---|---|---|---|---|---|---|
| Soldier of Godrick | 1 (tutorial) | 5 (a plain soldier) | human | 1 / about 4 | 3 | fog gate | 1: melee only, dies in seconds | 2 |
| **Margit, the Fell Omen** | **5** (the first great wall) | **4** (hooded old man, cane, glowing gold weapons) | **about 3 tall, 1.4 wide** | **2 at half health / 8 moves plus follow-ups** | **5** (famous delays, gold appears before each holy weapon) | **5** (fog gate, Margit's Shackle) | **5** (daggers punish campers, a leap closes gaps, a shockwave punishes groups) | **4** (switches targets) |
| Tree Sentinel | 4 | 3 (a mounted rider; our horse rig helps) | about 4 tall mounted | 1 / about 8 | 4 | 4 (patrols open ground) | 4 (galloping charge) | 3 |
| Godrick the Grafted | 5 | 1 (grafted arms, a dragon-head arm in phase 2) | about 5 tall | 2 / 12 or more | 3 | 4 | 4 | 4 |
| Leonine Misbegotten | 2 | 3 | about 3 | 1 / about 8 | 3 | 3 | 3 | 3 |
| Crucible Knight | 4 | 5 (a knight with sword and shield) | human | 2 / about 9 | 4 | 4 (evergaol) | 3 (the shield blocks bullets from the front, so friends flank) | 4 |
| Bell Bearing Hunter | 2 | 4 | human | 1 / about 7 | 4 | 5 (a night-only shack) | 3 | 3 |

**Margit first, because:**
- He is one of the best-known bosses of the game, and the first real wall for every player.
- He has two clear phases and a moveset small enough to write by hand.
- He is humanoid and close to iron-golem size, so our pathfinding, collisions and posed-cube rig work for him as they are.
- His attacks answer every way MW2 players will try to cheese a fight: daggers for campers, a leap for kiting, a shockwave for groups, and delayed swings that are fair without a roll.
- The golden weapons he conjures are the telegraph, and we can draw them as glowing cubes with no game assets.

Who he is, briefly: a hunched old man in a dark hooded cloak who walks with a cane and conjures golden holy weapons: daggers, a sword, and from about half health a hammer. Players meet him at the gate of Stormveil Castle behind a fog wall. An item, Margit's Shackle, pins him down for a moment, twice per fight.

**Runner-up:** the Crucible Knight. He is the simplest model, has two phases, and can reuse Margit's rig. **Pipeline test:** the Soldier of Godrick could be a short first test of the boss pipeline, but it is not needed; Margit's headless fight test (section 4.4) covers the same ground.

---

## 4. The boss as data

### 4.1 The crate
`crates/elden_boss` (new, edition 2024, `glam = "=0.29.3"`, the version the vendored crates use, so `DVec3` is the same type on both sides). No other dependencies, no file reading, no engine types.

| Module | Holds |
|---|---|
| `data` | The types: `BossDef`, `PhaseDef`, `MoveDef` and the parts of a move. `def(id)` and the list of boss ids |
| `margit` | Margit's table (section 5), `source: DataSource::Estimate` |
| `brain` | The state machine: one `step(&Sense) -> Act` per server tick |
| `shape` | Hit geometry: arcs, circles, rings, a segment against a box |
| `hurt` | Damage into the boss: resistances, stance, kneeling, criticals, threat |
| `fight` | The arena: waking, scaling, leash, reset, being stuck |
| tests | Unit tests per module, and a headless whole fight |

The rest of the game reads one thing: a `&'static BossDef`. The hand-written tables are `static`. A baked table (section 14.2) is loaded once at start and leaked into a `'static` table, so the game keeps one type.

### 4.2 A boss (`BossDef`)

| Field | Meaning | Margit |
|---|---|---|
| `id`, `name` | Entity id and display name | `iw4l:margit`, "Margit, the Fell Omen" |
| `source` | `Estimate` (hand-written) or `Baked` (from game data) | `Estimate` |
| `width`, `height` | His box, in blocks | 1.4 x 3.0 |
| `health` | Health with one player | 800 |
| `scaling` | Health factor by players at the wake | 1.0, 1.6, 2.2, 2.8, then +0.6 each |
| `stance`, `stance_refill`, `kneel` | Stance points, quiet ticks before it refills, kneel ticks | 80, 120, 80 |
| `resist` | Damage and stance factors per source (section 7.1) | section 7.1 |
| `speed`, `step_height`, `turn` | Walk speed attribute, step height, turn per tick while tracking | 0.3, 1.0, 30 degrees |
| `arena` | Wake, leash, reset and bar radii, reset and stuck ticks | 24, 24, 40, 48 blocks; 100, 100 ticks |
| `phases` | Each phase: where it starts, cooldown factor, idle gap, the shift move | section 6 |
| `moves` | The move table | section 5 |
| `daggers` | Speed, flight ticks, damage | 1.5 blocks a tick, 40 ticks, 4 holy |
| `experience`, `drops` | Base experience, per-fighter experience, items | 100 (+50 a fighter); section 13 |
| `voices`, `cues` | Ambient, hurt and death sounds; shared cue names | section 14.1 |

### 4.3 A move (`MoveDef`)
All times are server ticks from the start of the move (20 a second).

| Part | Meaning |
|---|---|
| `key` | A stable name (`"cane_combo"`). Saves, the network, poses and tests use the key or its index, never a line number |
| `phases` | The phases that may pick it |
| `pick` | Distance band, angle band, weight and cooldown for choosing it |
| `length` | Total ticks |
| `track` | Windows in which he turns toward his target. Each ends at least 8 ticks before the strike it aims |
| `hold` | One random pause: at a tick, the move's clock stops for a random number of ticks. This is how Margit's delayed swings work |
| `motion` | Root motion: a step forward, a hop back, or a leap to a landing point fixed when tracking ends |
| `strikes` | Hit windows: from, to, shape, damage, physical or holy, reaction, shield disable |
| `throws` | Daggers: the tick, the phases, a chance |
| `conjure` | Which golden weapon shows (dagger, sword, hammer) and from when to when |
| `cues` | Sounds (played by the host) and particle bursts (drawn by each viewer) at ticks |
| `poses` | Key poses at ticks. The renderer turns them into a keyframe clip |
| `branches` | Follow-ups: at a tick, with a chance and a condition, switch to another move, or end early with a short recovery |

**Shapes**, in his own frame (forward is where he faces when the window starts):
- **Arc:** reach, half-angle, low and high height. A player is hit when the near side of their box is within reach, inside the angle, and overlaps the height band. Margit's arcs reach 1 block below his feet (2.5 above unless the table says otherwise), so a player in a 2-deep hole beside him is still hit.
- **Circle:** a point in front of him and a radius. Used for leap landings.
- **Ring:** a point and a radius that grows over the window. A player is hit when the ring's edge passes under their feet and their feet are less than 1 block above the ground. Jumping clears it.

Each strike lands at most once per player (the reference's "landed once" idea, kept per player).

### 4.4 The brain
One `Brain` per boss, stepped once per server tick with a `Sense` and returning an `Act`. It has its own seeded random, so a fight replays the same from the same seed.

| State | What happens | Leaves to |
|---|---|---|
| `Asleep` | Stands at his anchor and takes no damage, so nobody can snipe him from afar before the fight | `Wake` when a player comes within the wake radius of his anchor |
| `Wake` | 30 ticks: rises from golden mist. The fight starts, health is scaled | `Idle` |
| `Idle` | Walks toward his target or circles it, facing it, for a gap of random length | `Move` |
| `Move` | Plays one move: tracking, holds, root motion, strikes, throws, cues, branches | `Idle`, another `Move` (a branch), `Kneel`, `Shift` |
| `Kneel` | Stance broke: 80 ticks on one knee, no strikes | `Idle` |
| `Shift` | The phase change: 30 ticks, the hammer forms, stance cannot break | `Idle` in the new phase |
| `Fade` | 20 ticks: vanishes in golden mist and appears somewhere else | `Idle` |
| `Return` | After a reset or a load: walks back to his anchor at full health and takes no damage; fades home if no path is found within 100 ticks | `Asleep`, or `Wake` if a player comes within the wake radius |
| `Dead` | 20 ticks: collapses into golden mist | removed |

`Sense` holds: the tick, his position, facing and health, whether he stands on the ground, and the eligible players (id, feet, eye height, the height of the ground under their feet for the ring test, whether a path reaches them). The game fills the ground height from the level's blocks: `PlayerCandidate` has no on-ground flag. `Act` holds: where to walk, where to face, the root motion this tick, the hits that landed (player, damage, physical or holy, reaction, shield disable, where it came from), the daggers to launch, the cues to play, a fade target, and the clip shown (state, move, the move's own clock, phase).

**Choosing a move:** the moves of his phase that are off cooldown, whose distance band holds his target and whose angle band holds the angle to it. One is rolled by weight. If none fits, he walks closer, or turns. Follow-ups ignore cooldowns.

**Headless whole fight:** a test plays a full fight with two scripted players on flat ground, with no window and no game: one shoots from range and strafes, one fights with a sword and backs off at every wind-up. It checks that every move and both phases appear, that he kneels and takes a critical, that he dies, and that the same seed dies on the same tick. This is the reference's "the tour shows everything" idea applied to a boss.

### 4.5 Ticks, not frames
The reference steps its fighters at 60 Hz with animation data at 30 frames a second. Our server ticks at 20 Hz. The hand-written tables are in ticks, and every tick of a window is checked, so no window is skipped. Data baked from the game later (30 frames a second) is converted by widening each window outward: from `floor(frame / 1.5)` to `ceil(frame / 1.5)`.

---

## 5. Margit's moveset

All numbers are estimates. Times are ticks (20 a second), distances are blocks, damage is Minecraft health on Normal (a player has 20). "Physical" hits are blocked by shields and reduced by armour; "holy" hits are not (section 7.4).

### 5.1 The moves

| # | Key | Phases | Picked at | Timeline | Strikes | Damage | Reaction |
|---|---|---|---|---|---|---|---|
| 1 | `cane_combo` | 1, 2 | 0-4, within 60 degrees | swings at 12-14 and 22-24; the cane rises and holds 0-10 ticks at 34; third swing at 44-46; ends at 70 | arc 3.5, 100 degrees (third 120), height -1 to 2.5 | 4, 4, 6 physical | flinch, flinch, stagger |
| 2 | `cane_slam` | 1, 2 | 0-4, within 45 | cane overhead, holds 0-12 ticks at 20; slam at 28-30; ends at 54 | arc 3.5, 60 degrees | 7 physical | stagger |
| 3 | `dagger_throw` | 1, 2 | 5-30, within 90 | a gold dagger forms at 2; holds 10-30 ticks at 10; throws at 22; ends at 36 | one dagger | 4 holy | flinch |
| 4 | `backstep_daggers` | 1, 2 | 0-5, within 90 | hops 4 back at 2-12; daggers at 22 and 30, and at 38 (always in phase 2, one in three in phase 1); ends at 60 | daggers | 4 holy each | flinch |
| 5 | `leap_slam` | 1, 2 | 8-16, within 30 | crouches until 14, where the landing point is fixed (6-12 away, inside the leash); airborne 14-30 with a 3-block apex; lands at 30-32; ends at 60 | circle 1.5, 1 in front of the landing | 8 physical | knockdown |
| 6 | `sword_sweep` | 1, 2 | 0-5, within 60 | a gold sword forms at 0-8; sweeps at 22-25; ends at 48 | arc 4.5, 180 degrees, height -1 to 2 | 7 holy | stagger |
| 7 | `hammer_slam` | 2 | 0-4, within 45 | a gold hammer forms at 0-10; raised, holds 0-8 ticks at 30; slams at 40-42; shockwave 40-46; ends at 76 (his longest opening) | arc 3.5, 70 degrees; ring from 2 in front, 0.5 to 3.5 | 11 physical; ring 4 holy | knockdown, shield off 5 s; ring flinch |
| 8 | `hammer_leap` | 2 | 8-16, within 30 | hammer forms at 0; crouches until 16 (landing fixed); airborne 16-34 with a 4-block apex; lands at 34-36; shockwave 34-40; ends at 70 | circle 1.5, 1.5 in front; ring 0.5 to 4 | 12 physical; ring 5 holy | knockdown, shield off 5 s; ring flinch |

| Key | Weight | Cooldown (ticks) | Tracking windows |
|---|---|---|---|
| `cane_combo` | 30 | 20 | 0-4, 12-14, 26-36 (he tracks through the hold) |
| `cane_slam` | 20 | 40 | 0-20 (and through the hold) |
| `dagger_throw` | 25 | 60 | 0-18 (and through the hold) |
| `backstep_daggers` | 15 | 100 | 12-26 |
| `leap_slam` | 30 | 120 | 0-14 |
| `sword_sweep` | 15 | 40 | 0-14 |
| `hammer_slam` | 25 | 80 | 0-32 (and through the hold) |
| `hammer_leap` | 20 | 160 | 0-16 |

### 5.2 Follow-ups (branches)

| From | At tick | Chance and condition | To |
|---|---|---|---|
| `cane_combo` | 16 | 25% | ends with a 14-tick recovery (one swing) |
| `cane_combo` | 26 | 30% | ends with a 16-tick recovery (two swings) |
| `cane_combo` | 26 | 40%, phase 2 | `hammer_slam` (cane twice, then the hammer) |
| `cane_combo` | 52 | 30% | `sword_sweep` |
| `sword_sweep` | 30 | target farther than 6 | `dagger_throw` (a dagger at a retreating target) |
| `leap_slam` | 40 | 30%, phase 2 | `sword_sweep` |

Branches are tried in table order; the first that passes wins.

### 5.3 Clips that are not picked

| Clip | Ticks | What it shows |
|---|---|---|
| `idle` | loops | breathing, the cane planted, a slow head turn toward his target |
| `walk` | loops | a heavy limping walk, driven by the distance moved |
| `wake` | 30 | rises from golden mist |
| `shift` | 30 | raises his left hand; the hammer forms with a gold flash at 20 |
| `kneel` | 80 | down on one knee, leaning on the cane |
| `fade` | 20 | dissolves in golden mist at 10, appears at the new spot |
| `death` | 20 | sinks to his knees and dissolves into golden mist |

### 5.4 Tells and daggers
- His eyes flare gold 8 ticks before every strike of a delayed swing, and a golden weapon is visible for at least 8 ticks before it strikes.
- Daggers fly at 1.5 blocks a tick for at most 40 ticks, with no gravity, aimed at the target's chest when they leave his hand. A dagger stops at the first solid block and hits the first player along its path.
- **Not modelled:** a kick or shove, faster phase-2 cane variants, and his exact dagger counts. These need the game's data.

---

## 6. Phases

| | Phase 1 | Phase 2 |
|---|---|---|
| Starts | at the wake | the first time health falls to half or below |
| Moves | 1-6 | 1-8, plus the cane-into-hammer follow-up |
| Cooldowns | as listed | x0.8 |
| Idle gap between moves | 10-20 ticks | 6-12 ticks |
| Backstep daggers | 2, a third one in three times | always 3 |
| Look | cane and conjured weapons | a golden shimmer drips from him |

**The shift:** it starts when the current strike window ends (the rest of that move is cancelled), or at once when he is between moves. It lasts 30 ticks: he raises his left hand, a golden hammer forms, and a gold flash goes off at tick 20 with `entity.elder_guardian.curse` and `block.beacon.activate`. During the shift he takes damage but his stance cannot break; stance damage still counts and can break right after. The bar keeps its colour. In Elden Ring the change is soft, with no cutscene, and so is ours.

---

## 7. Damage, stance and staggers

### 7.1 Damage into him

| Source | Health he loses | Stance he loses | Hurt cooldown |
|---|---|---|---|
| MW2 bullet | MW2 points x 0.2 (the mob scale) x 0.5 (his resistance) | MW2 points / 60 | none |
| MW2 blast (grenades, launchers, C4, killstreaks) | MW2 points x 0.8 (the mob scale) x 0.25 | MW2 points / 15 | none (one hit per blast) |
| Minecraft melee | vanilla (charge, crits, sweeps, enchantments) | damage x 1.5 | none |
| Arrow, trident, thrown item | vanilla | damage x 1.0 | none |
| Explosion (TNT, creeper, crystal, bed, anchor) | vanilla x 0.5 | damage x 1.0 | none |
| Fire, lava, cactus, drowning, suffocation | vanilla | none | vanilla (10 ticks) |
| Falling | none: he takes no fall damage | | |
| Potion effects | none apply to him | | |
| While kneeling | everything x 1.5 | | |
| A critical (first melee hit from the front while kneeling) | x 3, and he gets up | | |

Examples (one player, 800 health): an M4A1 body hit at close range (30 MW2 points) takes 3 health and 0.5 stance; a head hit (x1.4) takes 4.2. A Barrett chest hit (105) takes 10.5 and 1.75 stance. A frag at its centre (150) takes 30 and 10 stance. A netherite sword with Sharpness V (11) takes 11 and 16.5 stance, or 16.5 and about 25 stance with a crit.

### 7.2 No hurt cooldown for player hits
Every vanilla mob has a hurt cooldown (`health.rs` `hurt_absorbed`): after a full hit, for 10 ticks a new hit only counts by how much it is bigger. The cooldown belongs to the victim, so it is shared by all attackers. Against a boss that would mean:
- an M4A1 could do at most one full hit every half second, about 12 health a second, however many players shoot;
- with four players the fight would last as long as with one, and health scaling would only make it longer.

So **player-made hits skip the cooldown on Margit**: bullets, blasts, melee, arrows, tridents, thrown items and explosions. Each source is already limited by its own rate: a gun by its fire rate and magazine, a sword by its attack charge, a bow by its draw. **Environmental damage keeps the vanilla cooldown** (fire, lava, cactus, drowning, suffocation), or standing in fire would take 20 health a second.

### 7.3 Stance, kneeling and criticals
- Stance is 80. Each hit takes stance (section 7.1). After 120 ticks (6 s) with no stance damage it refills fully.
- At 0 he kneels for 80 ticks (4 s): the current move is cancelled, he strikes nothing, and every source does x1.5.
- The first Minecraft melee hit from the front while he kneels (within 60 degrees of his facing and 4 blocks of him) is a critical: x3, crit particles and `entity.player.attack.crit`, and he rises at once with a 20-tick recovery. This stands in for Elden Ring's riposte.
- Stance refills when he rises. It does not scale with players.
- He never flinches from normal hits, as bosses do in Elden Ring: only a broken stance stops a move.
- He is never knocked back (knockback resistance 1).

### 7.4 Damage out (his hits on players)
- **Physical** hits are vanilla mob attacks (`PlayerHitKind::Melee`, damage type `mob_attack`): armour, Protection and a raised shield count. Hammer hits disable a shield for 5 s, like an axe.
- **Holy** hits are magic from a mob (`PlayerHitKind::Magic`, damage type `indirect_magic`): they ignore armour and shields, because vanilla 26.3 lists `indirect_magic` in `#bypasses_armor` and so in `#bypasses_shield` (checked in the 26.3 data). This is our mapping of Elden Ring's holy damage, not game data.
- **Difficulty** scales them like every mob hit with a source (`difficulty_scaled`): Easy halves them and adds 1, Hard x1.5. On Peaceful he does not exist.
- **Reactions:** a flinch is the vanilla knockback; a stagger throws the player 0.8 back; a knockdown lifts 0.5 and throws 1.2 back, like an iron golem's toss. There is no movement lock: that would be new MW2 movement code.
- **Death messages:** "Sos was slain by Margit, the Fell Omen" for physical hits, "Sos was killed by Margit, the Fell Omen using magic" for holy ones.

### 7.5 How long a fight takes (estimates)
Assumptions: about 70% of shots hit, about half the time is spent dodging, magazines are reloaded.

| Players and weapons | His health | Time |
|---|---|---|
| One, M4A1 | 800 | about 1.5-2 minutes, about 400 rounds |
| One, netherite sword Sharpness V | 800 | about 2-2.5 minutes |
| One, Barrett | 800 | about 2.5 minutes |
| One, with rockets and frags as well | 800 | about 1 minute |
| Four, M4A1s | 2240 | about 1-1.5 minutes |

Every factor lives in one place (`crates/elden_boss/src/margit.rs`), so the owner's feedback becomes a one-line change.

---

## 8. The arena
His **anchor** is where he appeared. The arena is a circle around it; there are no walls in the proof of concept.

| Rule | Value | What happens |
|---|---|---|
| Wake | 24 blocks | An eligible player within 24 blocks of the anchor wakes him. The fight starts: health is scaled and the bar shows |
| Leash | 24 blocks | He never walks or leaps to a point farther than 24 from his anchor. A target outside gets daggers |
| Reset | 40 blocks, 100 ticks | No eligible player within 40 of the anchor for 5 s: full health, phase 1, full stance, cooldowns and threat cleared, the bar goes, he walks back (or fades home if no path is found within 100 ticks) and sleeps |
| Stuck | 100 ticks | His target is inside the leash, but no path reaches it and none of his hits landed on it for 5 s (a hole deeper than his swings reach, a nook walled in so he cannot get next to it): he fades and appears beside it, within 3 blocks across and 2 blocks up or down of its feet. If there is no room there (a player on a pillar), he keeps throwing daggers instead. Our own anti-cheese rule, not Elden Ring's |
| Fog ring | 24 blocks | While he fights, golden particles mark the leash circle near each viewer. They stand in for Elden Ring's fog wall and block nothing |

**Eligible** means alive, not a spectator, attackable (not creative) and in the same level. If every fighter dies, nobody is within 40 blocks for 5 s and the fight resets. A dead player respawns at their bed, anchor, spawn point or tactical insertion flare, which works well as a site of grace near an arena.

---

## 9. Multiplayer: scaling and sync

### 9.1 Health scaling
The fighters are the eligible players within 40 blocks of the anchor at the wake.

| Fighters | 1 | 2 | 3 | 4 | more |
|---|---|---|---|---|---|
| Health factor | 1.0 | 1.6 | 2.2 | 2.8 | +0.6 each |
| Margit's health | 800 | 1280 | 1760 | 2240 | |

- The factor is fixed at the wake; a player who joins later does not change it.
- Health keeps its share: a Margit summoned at 52% stays at 52% after scaling.
- Stance and his damage do not scale.
- These steps are our choice. Elden Ring scales boss health in co-op too, but the real factors are in the game's params.

### 9.2 Who he attacks
After each move, and at the wake, he picks the eligible player with the most **threat**: the damage that player did to him in the last 200 ticks (10 s). With no threat, he picks the nearest. He never picks a player farther than 40 blocks from his anchor. If his target dies or leaves, he picks again at once.

### 9.3 What friends receive
| What | How | New? |
|---|---|---|
| The boss itself | `MobAdd` with his saved tag (the marker carrier, with his anchor) | no |
| Position and facing | the per-tick `ServerMob` | no |
| Pose, phase, daggers | `MobShow.elden: Option<EldenShow>` (clip, the move's own clock, held, phase, awake, up to 3 daggers as start, velocity and age) | yes, a wire change |
| Sounds | the mob voice path (`Voice::Event`), like every mob sound | no |
| Particles | drawn by each viewer from the clip and the cue table, so nothing is sent | no |
| The bar | `ToClient::BossBar` | no |
| "GREAT ENEMY FELLED" | a system line on the action bar (`Clientbound::System { overlay: true }`) | no |

`MobShow` changes, so host and friend need the same build. The release step bumps `minecraft_net::PROTOCOL` once for everything this round changes on the wire.

### 9.4 Credit
Every hit carries the player's id, friends included. Threat, the kill and the experience follow it, and the death loot sees `killed_by_player`.

---

## 10. The boss bar and music
- **Who sees it:** every player in his level within 48 blocks of him while he is awake. It goes when he resets or dies.
- **Look:** the vanilla bar at the top, red, progress overlay, the name "Margit, the Fell Omen" as plain text (there is no language key). Progress is health over his scaled maximum.
- **Ids:** dragon 1, raids `0x100` and up, withers `1 << 40` plus the entity id; Elden Ring bosses `1 << 41` plus the entity id.
- **Host:** a fourth bar list next to the dragon, raid and wither bars (`frame/src/minecraft_ui.rs`, drawn by `hud/src/minecraft_boss.rs`). **Friends:** they keep every bar id already (`minecraft_survival/src/remote.rs`).
- **Music:** the bar asks for music (`play_music`). Vanilla plays its boss music only in the End, where only the dragon's bar asks for it. We play `music.dragon` in any dimension while a bar asks for music, and stop it outside the End when no bar asks any more. This changes nothing for vanilla bosses. An original track can replace it later.
- **Later:** an Elden Ring style bar at the bottom of the screen with a yellow trail that shows recent damage, drawn from the same message.

---

## 11. Summoning and finding him

### 11.1 `/summon` (tests and creative)
`/summon iw4l:margit [pos] [nbt]`. He appears asleep with his anchor at that spot and wakes when a player comes within 24 blocks. A `Health` in the NBT is used (tests start him near half health this way). Loading a saved world never reads it.

### 11.2 The altar (survival)
**Put a lantern on top of two stacked gold blocks.**
- When a player places the lantern (standing, not hanging), the three blocks vanish in golden mist and Margit rises where the pillar stood, facing the nearest player, who is the placer in practice: `Command::PlayerEdit` (`crates/minecraft_terrain/src/server.rs`) carries no player id, and adding one would change a shared command for every track. His anchor is that spot.
- It works in every dimension, and not on Peaceful, like the wither.
- It costs 18 gold ingots and a lantern (iron nuggets and a torch): reachable without the Nether, unlikely to happen by accident, and gold is his colour.
- The check runs where the wither's skull check runs (`ServerSim::player_edit_block`), in our own module.

### 11.3 Later: finding him
- **A gate arena** generated in the world: a walled stone courtyard of about 24 x 32 with a gate and a fog wall, a nod to Stormveil's gate, with Margit asleep at its centre. It needs a fork structure set beside the vanilla ones, and it appears only in new chunks.
- **Margit's Shackle:** an item that pins him down for 3 s when used within 16 blocks, twice per fight, as in Elden Ring.
- **A spawn egg** in the creative inventory.

---

## 12. Saving

### 12.1 The carrier
He is saved in his chunk's entity file like any mob, but as a vanilla marker:

```
{id: "minecraft:marker", Pos: [...], Rotation: [...], UUID: [...],
 data: {iw4l: {boss: "iw4l:margit", anchor: [x, y, z], yaw: f}}}
```

What vanilla 26.3 does (javap):
- An entity with an unknown id is dropped on load with the warning "Skipping Entity with id ..." (`EntityType.create`), and is lost when vanilla saves the chunk again. So an `iw4l:margit` id would be lost by one visit in vanilla.
- Every entity saves its custom data as `data` (`Entity.customData`), and a marker keeps it. So a marker carrier survives a visit in vanilla, invisible and doing nothing, and our game turns it back into Margit.
- This follows the fork's rule for items: a vanilla carrier plus `minecraft:custom_data` with an `iw4l` id (`item_nbt.rs`).

Our loader claims a marker only when `data.iw4l.boss` names a known boss. Every other marker stays dormant and is saved as it came.

### 12.2 Loading resets the fight
A loaded Margit stands where he was saved, at full health, in phase 1, with full stance, and walks back to his anchor to sleep there (the `Return` state). Health, phase and threat are not saved. This is Elden Ring's rule (quitting resets a boss) and it makes a half-finished fight impossible to save-scum. An autosave does not reset anything: only loading him from disk does. A chunk that unloads and loads again (every player went far away) loads him the same way, which the reset rule of section 8 would have done anyway.

### 12.3 What else is saved
- Nothing per player for the proof of concept.
- Later, first kills per world can go in `dimensions/minecraft/<dimension>/data/iw4l/bosses.dat` through `Save::write_saved_in` with the id `iw4l:bosses` (the storage splits the id into folder and file), beside `data/minecraft/ender_dragon_fight.dat`. Vanilla never reads other namespaces.

---

## 13. Death, loot and rewards
- **Death:** 20 ticks; he sinks to his knees and dissolves into golden mist (`entity.evoker.death` low, `entity.illusioner.mirror_move`).
- **Drops,** where he dies: 1 enchanted golden apple, 2 gold blocks (the altar's gold back) and 3 gold ingots per fighter.
- **Experience:** 100, plus 50 per fighter after the first, as orbs.
- **Every fighter** (the fighters at the wake, plus anyone who hurt him) sees "GREAT ENEMY FELLED" on the action bar and hears `ui.toast.challenge_complete`. The action bar line is white: our overlay line has no colour (`frame::minecraft_ui::McOverlayMessage` holds plain text, drawn white in `hud/src/minecraft_hotbar.rs`). A gold line is left for later.
- **A player he kills** dies like any player killed by a mob: drops, death message (section 7.4), respawn.

The loot is our decision, not Elden Ring data.

---

## 14. Assets

### 14.1 Stand-ins now
- **Model:** posed cubes (`crates/minecraft_terrain/src/posed_cube.rs`), our own:
  - a hunched old man in a long, ragged dark cloak with a big hood, a pale face with gold eyes, a long grey beard, two small horns through the hood, bare left arm, a wooden cane with a crook;
  - about 3 blocks tall standing hunched, drawn at a humanoid layout scaled up by the root matrix, as the wither's model is;
  - the golden dagger, sword and hammer are extra parts, shown only while conjured, painted gold in his own skin and drawn at full light (`Skin` sky and block light 15, `glow_render::FULL_BRIGHT`). `posed_cube::Skin` has no tint, so gold comes from the skin's pixels. Not `glow_render::append_glowing`: that is the Glowing effect's white hull, seen through walls.
- **Skin:** a 64 x 64 skin painted by code when the atlas is built (`minecraft:iw4l/margit`, beside the code-made `iw4l/white`): a small palette with seeded noise for cloth, fur, beard, skin and wood. No PNG is committed.
- **Animation:** about 20 key poses (ready, cane back, swing through, cane overhead, throw wind-up, release, crouch, airborne, landing, sword sweep, hammer raised, hammer down, kneel, and so on) on the shared keyframe system (`crates/minecraft_terrain/src/keyframes.rs`, `Definition`, `Channel`, `Keyframe`). Each move's clip comes from its `poses` list, so the drawn swing always matches the hit window.
- **Particles** (existing kinds, nothing new in `ambient_particles.rs`): golden mist is `Totem` plus gold `SpellKind::Effect` spell particles (`spell_particles.rs`, which take a colour, as the wither's charge does in `entity_view.rs` `wither_effects`); conjuring sparks are `EndRod`; slams throw `Poof` in a ring and one `Explosion`; criticals use `Crit`; the fog ring is gold `SpellKind::Effect`. `ambient_particles::Kind::EndRod` has no tint: it fades from white to cream.
- **Sounds:** vanilla 26.3 events (all checked in `SoundEvents`), read at runtime from the downloaded assets:

| Moment | Sound |
|---|---|
| Ambient, hurt, death | `entity.evoker.ambient`, `entity.illusioner.hurt`, `entity.evoker.death`, pitched down |
| Steps | `entity.iron_golem.step`, low |
| Cane swing | `entity.player.attack.sweep`, low |
| A golden weapon forms | `block.amethyst_block.chime` and `block.beacon.activate`, high |
| Dagger thrown, dagger hits | `item.trident.throw`, `item.trident.hit` |
| Leap | `entity.ravager.roar` |
| Slam, hammer slam | `item.mace.smash_ground`, `item.mace.smash_ground_heavy` with `entity.generic.explode` quiet and low |
| Stance broken | `entity.ravager.stunned` |
| Phase change | `entity.elder_guardian.curse` and `block.beacon.activate` |
| Golden mist | `entity.illusioner.mirror_move` |
| Felled | `ui.toast.challenge_complete` |

- **Music:** vanilla `music.dragon`, already downloaded for the End.

### 14.2 Real game data later
Nothing can be extracted on this PC today. When the owner installs the game, this is the path.

**What the owner needs:** nothing yet. The baker below is not written, so unpacked files do nothing for the game until it exists. When it does:
1. **ELDEN RING** on Steam, installed. The tools expect `C:\Program Files (x86)\Steam\steamapps\common\ELDEN RING\Game`. The game's own `oo2core_6_win64.dll` in that folder decompresses its files; it is the owner's copy and is never redistributed.
2. **UXM Selective Unpack**, used to unpack only, never to patch. A selective unpack of one boss is at most a few hundred MB; a full unpack is tens of GB, too much for D:.
3. **WitchyBND**, to unpack `regulation.bin` into `regulation-bin/` (the `*.param` files).
4. Optional helpers to look up ids and field meanings: Paramdex (param layouts), Smithbox (params and ids), DSAnimStudio (animation events).

**Which files, for Margit:**

| Need | Game file | Notes |
|---|---|---|
| Move timings | `chr/c<id>.anibnd.dcx` (TAE events) | attack windows, cancels, sound frames, super armour |
| Animations and skeleton | the same file (Havok HKX) | baked to joint positions per tick |
| Model, hurtbox points | `chr/c<id>.chrbnd.dcx` (FLVER, dummy polys) | where strikes and hurtboxes sit on the bones |
| Textures | `chr/c<id>_h.texbnd.dcx` | for a textured model, much later |
| Health, stance, defences | `NpcParam` | |
| Which attack is which | `BehaviorParam` (the NPC one), `AtkParam_Npc` | damage, hit radii, attack types |
| Daggers and shockwaves | `Bullet` | |
| Effects | `SpEffectParam` | |
| Move choice | `script/aiscript/*.luabnd.dcx` (compiled Lua), `NpcThinkParam` | read and described, never ported |
| Sounds | the Wwise banks under `sd/` | converted to Ogg |

Margit's character id could not be checked without the game; look it up by name (Smithbox, Paramdex) first. Margit and Morgott share a body, so one set of files may serve both.

**Folder layout:** everything in one folder as `chr/...`, `regulation-bin/...`, `script/...` and `sd/...`. `ER_FILES` points at it and `ER_GAME_DIR` at the `Game` folder: the reference's variable names, read at runtime like `MINECRAFTOSS_ROOT`.

**The baker:** our own tool (a later crate, for example `crates/elden_data`), written fresh. The reference has no licence, so we may not copy its Python tools; we know the formats from them and from public format notes. It reads the files above and writes `elden/<boss>.bin` next to the exe:
- moves in the shape of `MoveDef`, windows converted from 30 frames a second to ticks (section 4.5);
- health, stance, resistances and damage from the params, mapped onto our scales;
- strike shapes from attack hit radii and dummy-poly positions;
- joint tracks per clip for the renderer;
- sounds as Ogg.

**The seam:**
- At start, if `IW4L_ELDEN_DATA` names a folder with a baked file for a boss, and the file passes the same `check()` the hand-written tables pass, the game uses it (`source: Baked`). Otherwise it uses the hand-written table.
- Move keys stay the same, so saves, the network and the tests do not change.
- The renderer gets a second path for the same clip: baked joint tracks instead of hand-made key poses.
- The tests never need the game.
- Baked files and unpacked game files are listed in `.gitignore` and never committed.

---

## 15. What the reference teaches (ideas only)
The reference (`C:\Dev\references\ELDEN-RING-Combat-Rewrite`, Rust on Bevy 0.19) recreates the Elden Ring player character from the game's own files: timings, root motion, cancels, i-frames, weapon hit lines, guard, poise and hit-stop. Its only enemy is a sparring dummy that cycles four swings of two kinds (an overhead slam and a low sweep). It has no licence file, so we read it and never copy it. What we take, as ideas:

1. **The same data shape, hand-written or extracted.** An action is a table of windows, cancels, turn locks, root motion and hit lines. We write Margit by hand in that shape; a baker fills the same shape later. Unlike the reference, a missing baked file never stops the build: the hand-written table is the fallback.
2. **The simulation says which clip and which frame; the view only draws it.** Our host steps the boss; `MobShow` carries the clip and the clock; every viewer poses him locally.
3. **The dummy's loop is a boss's skeleton:** idle and track, wind up (tracking stops at a set moment, so late dodges work), strike in an active window, recover, and break into a stagger on poise damage. We extend it with a move table, distance and angle rules, combos, holds, phases and an arena.
4. **Hits land when the weapon reaches the target,** checked every tick of the window, each at most once per target. We keep that per player.
5. **Poise and stagger** become our stance bar, with our own numbers for guns.
6. **Telegraphs** matter even more without a roll: the reference colours its dummy's wind-ups; we show golden weapons and flaring eyes.
7. **A headless self-play test** of a whole fight, like the reference's tour of everything.
8. **The extraction pipeline** (DCX with the game's own Oodle DLL, BND4, TAE, PARAM, HKX spline animations, Wwise banks) shows which files a boss needs and that a selective unpack is small.

What the reference does not have, and we must build: boss AI, enemy movesets on their own skeletons, several hurtboxes, phases, a fight structure, projectiles, several players and saving.

---

## 16. Out of scope for the proof of concept
- Real game data, textures from the game, the baker.
- An Elden Ring style bottom bar, the gate arena, Margit's Shackle, a spawn egg, lock-on.
- A dodge roll, stamina, hit-stop, status buildup (bleed, frost).
- The MW2 knife hitting mobs.
- Several hurtboxes per boss; weak points beyond the MW2 headshot.
- Punishing heals (he could throw a dagger at a player who is eating; it needs the player's use state in the mob's view).
- Iron golems and other mobs fighting him; he ignores mobs and they ignore him.

---

## 17. Later bosses and systems

**Bosses, in order:**
1. **Margit, the Fell Omen** (this track).
2. **Crucible Knight:** the same humanoid rig. His shield blocks bullets from the front, so friends flank him; in phase 2 a tail and wings appear.
3. **Tree Sentinel:** a mounted knight on our horse rig, patrolling open plains; a galloping charge.
4. **Bell Bearing Hunter:** appears at night near a structure; a spectral sword.
5. **Morgott, the Omen King:** Margit's true form, the same body with more moves; mostly a new table.
6. **Godrick the Grafted:** large, two phases with a dragon-head arm; needs several hurtboxes and real data.

**Systems that come with them:**
- the baker and real data (section 14.2), then textured models;
- the Elden Ring style bottom bar with a damage trail;
- a fork structure set for arenas with fog walls, found with `/locate`;
- the MW2 knife against mobs (all mobs, not only bosses);
- Margit's Shackle and other boss items;
- first kills per world and per player;
- an original boss music track.

---

## 18. Risks and open questions
- **D0 timing.** The entity side of the proof of concept waits for D0's registry. The pure crate does not, so the track starts at once with it (`docs/round10/track-f.md`, segments).
- **Gun balance.** All factors are guesses until the owner plays. They live in one file.
- **A big body in tight places.** A 1.4 x 3.0 body needs 2-wide, 3-high paths. Caves and villages will block him; the stuck rule fades him to his target, and the arena is meant for open ground.
- **Many wire changes this round.** Several tracks change `MobShow` and other messages. The release step bumps the protocol once; host and friends always need the same build.
- **The friend's pose.** A friend sees him a few ticks late, like every mob. Strikes are decided on the host, so a friend can be hit by a swing that looked a little early on their screen; the tracking stop and wind-ups are long enough that this should not matter.
- **Vanilla round trip.** Checked with javap for markers and unknown ids, not yet tried in a running vanilla 26.3. Track F's owner steps include the check.
