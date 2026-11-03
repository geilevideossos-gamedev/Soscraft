use super::*;
use crate::illager::{Illager, IllagerKind};
use crate::iron_golem::IronGolem;
use crate::walk_path::{Walker, beyond_home, find_walk_path};
use crate::world::PlayerAttack;
use minecraftoss_player::Block;
use minecraftoss_player::rng::LegacyRandom;

#[derive(Default)]
struct Field {
    blocks: Vec<(Pos, &'static str)>,
}

impl World for Field {
    fn block(&self, pos: Pos) -> Option<Block> {
        if let Some((_, id)) = self.blocks.iter().find(|(at, _)| *at == pos) {
            return Some(Block::new(*id));
        }
        (pos.1 < 64).then(|| Block::new("minecraft:stone"))
    }

    fn set_block(&mut self, _: Pos, _: Option<Block>) {}
}

fn walled(blocks: &[(Pos, &'static str)]) -> Field {
    Field {
        blocks: blocks.to_vec(),
    }
}

fn at(x: f64, z: f64) -> DVec3 {
    DVec3::new(x, 64.0, z)
}

fn player_at(id: u64, position: DVec3) -> PlayerCandidate {
    PlayerCandidate {
        id,
        position,
        eye_height: 1.62,
        main_hand_cow_food: false,
        offhand_cow_food: false,
        main_hand_pig_food: false,
        offhand_pig_food: false,
        main_hand_chicken_food: false,
        offhand_chicken_food: false,
        main_hand_carrot_on_a_stick: false,
        offhand_carrot_on_a_stick: false,
        main_hand_wolf_interest: false,
        offhand_wolf_interest: false,
        main_hand_horse_tempt: false,
        offhand_horse_tempt: false,
        alive: true,
        spectator: false,
        attackable: true,
    }
}

fn creative_at(id: u64, position: DVec3) -> PlayerCandidate {
    PlayerCandidate {
        attackable: false,
        ..player_at(id, position)
    }
}

fn spectator_at(id: u64, position: DVec3) -> PlayerCandidate {
    PlayerCandidate {
        attackable: false,
        spectator: true,
        ..player_at(id, position)
    }
}

fn towards(player: &PlayerCandidate, to: DVec3) -> PlayerView {
    let eye = player.position + DVec3::Y * f64::from(player.eye_height);
    let d = to - eye;
    let horizontal = (d.x * d.x + d.z * d.z).sqrt();
    PlayerView {
        head_yaw: (d.z.atan2(d.x).to_degrees() - 90.0) as f32,
        pitch: (-d.y.atan2(horizontal).to_degrees()) as f32,
        disguised: false,
    }
}

fn away(player: &PlayerCandidate, to: DVec3) -> PlayerView {
    let view = towards(player, to);
    PlayerView {
        head_yaw: view.head_yaw + 180.0,
        ..view
    }
}

fn turned(view: PlayerView, degrees: f32) -> PlayerView {
    PlayerView {
        head_yaw: view.head_yaw + degrees,
        ..view
    }
}

fn spawn(world: &mut EntityWorld, position: DVec3) -> u64 {
    world.spawn_family_kind(KIND, position).unwrap()
}

fn creaking_of(world: &EntityWorld, id: u64) -> &FamilyMob<Creaking> {
    world
        .creakings()
        .iter()
        .find(|mob| mob.core.id == id)
        .unwrap()
}

fn present(world: &EntityWorld, id: u64) -> bool {
    world.creakings().iter().any(|mob| mob.core.id == id)
}

fn middle(world: &EntityWorld, id: u64) -> DVec3 {
    creaking_of(world, id).core.body.position + DVec3::Y * 1.35
}

fn prime(world: &mut EntityWorld, id: u64) {
    let brain = &mut world.creaking_mut(id).unwrap().state.brain;
    brain.started = true;
    brain.sensors = [1, 1];
}

fn sounds(world: &mut EntityWorld) -> Vec<String> {
    world.take_sounds().into_iter().map(|s| s.event).collect()
}

fn heard(sounds: &[String], event: &str) -> bool {
    sounds.iter().any(|sound| sound == event)
}

fn sword(player_id: u64, position: DVec3, damage: f64) -> PlayerAttack {
    PlayerAttack {
        player_id,
        position,
        yaw: 180.0,
        attack_damage: damage,
        strength: 1.0,
        sprinting: false,
        can_critical: false,
        can_sweep: false,
    }
}

fn by_player(amount: f32, damage_type: &'static str, source: DamageSourceKind) -> Hurt {
    Hurt::new(amount, damage_type, source).by(Some(Target::Player(1)), Some("minecraft:player"))
}

#[test]
fn a_creaking_is_tall_with_one_health_and_its_kind() {
    let mut world = EntityWorld::default();
    world.set_mob_drops(true);
    let id = spawn(&mut world, at(0.5, 0.5));
    assert_eq!(world.entity_type(id), Some(KIND));
    assert_eq!(world.family_core(id).map(|(key, _)| key), Some(83));
    let mob = creaking_of(&world, id);
    let core = &mob.core;
    assert_eq!(
        (core.body.width, core.body.height, core.eye_height),
        (0.9, 2.7, 2.3)
    );
    assert_eq!((core.health, core.max_health), (1.0, 1.0));
    assert_eq!(core.body.step_height, 1.0625);
    assert_eq!(MOVEMENT_SPEED, f64::from(0.4_f32));
    assert_eq!((ATTACK_DAMAGE, FOLLOW_RANGE), (3.0, 32.0));
    let profile = &mob.state.profile;
    assert_eq!(
        (
            profile.follow_range,
            profile.max_up_step,
            profile.walk_target
        ),
        (32.0, 1.0625, WalkTarget::Zero)
    );
    assert!(profile.can_float && profile.home.is_none());
    assert_eq!(
        profile.malus(PathType::Fire),
        PathType::Fire.default_malus()
    );
    assert!(mob.state.can_move && !mob.state.active && !mob.state.heart_bound());
    let census = world.census();
    let entry = census.iter().find(|c| c.kind == KIND).unwrap();
    assert_eq!((entry.width, entry.height), (WIDTH, HEIGHT));
    assert!(
        world
            .mob_candidates()
            .iter()
            .any(|m| m.id == id && m.kind == KIND)
    );
    assert!(crate::monster_ai::is_enemy(KIND));
    let traits = world.family_traits(id).unwrap();
    assert_eq!(traits.category, "monster");
    assert!(traits.enemy && !traits.undead && !traits.fire_immune && traits.pushable);
    assert_eq!(traits.voice, ("creaking", 1.0, "hostile_volume"));
    assert_eq!((traits.knockback_resistance, traits.seats), (0.0, 0));
    assert!(world.player_attack(&sword(1, at(0.5, 2.5), 1.0), id).died);
    let deaths = world.take_deaths();
    assert_eq!(deaths.len(), 1);
    assert_eq!(
        (deaths[0].id, deaths[0].table, deaths[0].experience),
        (id, Some(KIND), 0)
    );
}

#[test]
fn an_active_creaking_freezes_while_any_player_looks_and_moves_when_none_does() {
    let mut field = Field::default();
    let mut world = EntityWorld::default();
    let id = spawn(&mut world, at(0.5, 0.5));
    prime(&mut world, id);
    let north = player_at(1, at(0.5, 8.5));
    let south = player_at(2, at(0.5, -7.5));
    let players = [north, south];
    let target = middle(&world, id);
    let look = |world: &mut EntityWorld, first: bool, second: bool| {
        let view = |player: &PlayerCandidate, on: bool| {
            if on {
                towards(player, target)
            } else {
                away(player, target)
            }
        };
        world.set_player_views(vec![(1, view(&north, first)), (2, view(&south, second))]);
    };
    look(&mut world, false, false);
    world.tick_with_players(&mut field, &players);
    sounds(&mut world);
    assert!(creaking_of(&world, id).state.can_move);
    look(&mut world, true, false);
    world.tick_with_players(&mut field, &players);
    let state = &creaking_of(&world, id).state;
    assert!(state.active && !state.can_move);
    assert_eq!(
        state.brain.memories.attack_target.get(),
        Some(&Target::Player(1))
    );
    let said = sounds(&mut world);
    assert!(
        heard(&said, ACTIVATE_SOUND) && heard(&said, FREEZE_SOUND),
        "{said:?}"
    );
    let still = creaking_of(&world, id).core.body.position;
    look(&mut world, false, true);
    world.tick_with_players(&mut field, &players);
    assert!(
        !creaking_of(&world, id).state.can_move,
        "the other player holds it"
    );
    let said = sounds(&mut world);
    assert!(!heard(&said, FREEZE_SOUND) && !heard(&said, UNFREEZE_SOUND));
    look(&mut world, true, true);
    for _ in 0..5 {
        world.tick_with_players(&mut field, &players);
    }
    let mob = creaking_of(&world, id);
    assert_eq!(mob.core.body.position, still, "frozen in place");
    look(&mut world, false, false);
    world.tick_with_players(&mut field, &players);
    let state = &creaking_of(&world, id).state;
    assert!(state.can_move && state.active);
    let said = sounds(&mut world);
    assert!(heard(&said, UNFREEZE_SOUND) && !heard(&said, DEACTIVATE_SOUND));
    let body = Body::new(at(0.5, 0.5), WIDTH, HEIGHT);
    assert_eq!(gaze_heights(&body, EYE_HEIGHT), [66.3, 64.5, 65.4]);
    let viewer = player_at(1, at(0.5, 8.5));
    let view = towards(&viewer, DVec3::new(0.5, 65.4, 0.5));
    let open = Field::default();
    assert!(looked_at(&open, &viewer, view, &body, EYE_HEIGHT));
    assert!(looked_at(
        &open,
        &viewer,
        turned(view, 59.0),
        &body,
        EYE_HEIGHT
    ));
    assert!(!looked_at(
        &open,
        &viewer,
        turned(view, 61.0),
        &body,
        EYE_HEIGHT
    ));
    assert!(!looked_at(
        &open,
        &viewer,
        turned(view, -61.0),
        &body,
        EYE_HEIGHT
    ));
    let stone = "minecraft:stone";
    let feet = walled(&[((0, 65, 1), stone), ((0, 66, 1), stone)]);
    assert!(
        looked_at(&feet, &viewer, view, &body, EYE_HEIGHT),
        "seen at its feet"
    );
    let eyes = walled(&[((0, 64, 1), stone), ((0, 65, 1), stone)]);
    assert!(
        looked_at(&eyes, &viewer, view, &body, EYE_HEIGHT),
        "seen at its eyes"
    );
    let hidden = walled(&[
        ((0, 64, 1), stone),
        ((0, 65, 1), stone),
        ((0, 66, 1), stone),
    ]);
    assert!(!looked_at(&hidden, &viewer, view, &body, EYE_HEIGHT));
}

fn woken(player: PlayerCandidate, peaceful: bool) -> (bool, bool) {
    let mut field = Field::default();
    let mut world = EntityWorld::default();
    if peaceful {
        world.set_difficulty(0);
    }
    let id = spawn(&mut world, at(0.5, 0.5));
    prime(&mut world, id);
    let target = middle(&world, id);
    world.set_player_views(vec![(player.id, towards(&player, target))]);
    for _ in 0..2 {
        world.tick_with_players(&mut field, &[player]);
    }
    let state = &creaking_of(&world, id).state;
    (state.active, state.can_move)
}

#[test]
fn a_creaking_wakes_when_a_survival_player_looks_within_twelve_blocks() {
    assert_eq!(woken(player_at(1, at(0.5, 12.4)), false), (true, false));
    assert_eq!(woken(player_at(1, at(0.5, 12.6)), false), (false, true));
    assert_eq!(woken(creative_at(1, at(0.5, 8.5)), false), (false, true));
    assert_eq!(woken(spectator_at(1, at(0.5, 8.5)), false), (false, true));
    assert_eq!(woken(player_at(1, at(0.5, 8.5)), true), (false, true));
    let mut field = Field::default();
    let mut world = EntityWorld::default();
    let id = spawn(&mut world, at(0.5, 0.5));
    prime(&mut world, id);
    let near = player_at(1, at(0.5, 8.5));
    let target = middle(&world, id);
    world.set_player_views(vec![(1, towards(&near, target))]);
    for _ in 0..2 {
        world.tick_with_players(&mut field, &[near]);
    }
    assert!(creaking_of(&world, id).state.active);
    sounds(&mut world);
    let far = player_at(1, at(0.5, 33.5));
    world.set_player_views(vec![(1, away(&far, target))]);
    prime(&mut world, id);
    world.tick_with_players(&mut field, &[far]);
    world.tick_with_players(&mut field, &[far]);
    let state = &creaking_of(&world, id).state;
    assert!(
        !state.active && state.can_move,
        "no attackable player within 32"
    );
    assert!(!state.brain.memories.attack_target.present());
    assert!(heard(&sounds(&mut world), DEACTIVATE_SOUND));
    world.set_player_views(vec![(1, towards(&near, target))]);
    prime(&mut world, id);
    for _ in 0..2 {
        world.tick_with_players(&mut field, &[near]);
    }
    assert!(creaking_of(&world, id).state.active);
    let gone_creative = creative_at(1, at(0.5, 8.5));
    world.tick_with_players(&mut field, &[gone_creative]);
    let state = &creaking_of(&world, id).state;
    assert!(
        !state.active && state.can_move,
        "a creative player is no prey"
    );
}

#[test]
fn glass_does_not_hide_a_creaking_but_stone_does() {
    let viewer = player_at(1, at(0.5, 8.5));
    let body = Body::new(at(0.5, 0.5), WIDTH, HEIGHT);
    let view = towards(&viewer, DVec3::new(0.5, 65.4, 0.5));
    let wall = |id: &'static str| {
        let blocks: Vec<(Pos, &'static str)> = (63..=68)
            .flat_map(|y| (-2..=2).map(move |x| ((x, y, 4), id)))
            .collect();
        walled(&blocks)
    };
    let clear = [
        "minecraft:glass",
        "minecraft:red_stained_glass",
        "minecraft:tinted_glass",
        "minecraft:glass_pane",
        "minecraft:lime_stained_glass_pane",
        "minecraft:iron_bars",
        "minecraft:waxed_copper_grate",
        "minecraft:oxidized_copper_bars",
        "minecraft:powder_snow",
    ];
    for id in clear {
        assert!(
            looked_at(&wall(id), &viewer, view, &body, EYE_HEIGHT),
            "{id}"
        );
    }
    for id in ["minecraft:stone", "minecraft:oak_leaves", "minecraft:mud"] {
        assert!(
            !looked_at(&wall(id), &viewer, view, &body, EYE_HEIGHT),
            "{id}"
        );
    }
    let glass = wall("minecraft:glass");
    let eye = viewer.position + DVec3::Y * 1.62;
    let centre = DVec3::new(0.5, 65.4, 0.5);
    assert!(
        !crate::sight::line_of_sight(&glass, eye, centre),
        "glass stops a mob's sight"
    );
    assert!(sees(&glass, eye, centre) && !visual_clip(&glass, eye, centre));
    let far = DVec3::new(0.5, 65.4, 130.0);
    assert!(
        !sees(&Field::default(), DVec3::new(0.5, 65.4, 0.5), far),
        "past 128 blocks"
    );
    assert_eq!(
        visual_boxes(&walled(&[((0, 64, 0), "minecraft:soul_sand")]), (0, 64, 0)),
        vec![[0.0, 0.0, 0.0, 1.0, 1.0, 1.0]]
    );
    let mut world = EntityWorld::default();
    let id = spawn(&mut world, at(0.5, 0.5));
    prime(&mut world, id);
    let target = middle(&world, id);
    world.set_player_views(vec![(1, towards(&viewer, target))]);
    let mut glass = glass;
    for _ in 0..2 {
        world.tick_with_players(&mut glass, &[viewer]);
    }
    assert!(
        creaking_of(&world, id).state.active,
        "it wakes behind glass"
    );
}

#[test]
fn a_frozen_creaking_is_not_pushed_or_knocked_back() {
    let mut world = EntityWorld::default();
    let id = spawn(&mut world, at(0.5, 0.5));
    assert!(world.bind_creaking(id, (0, 64, 3)));
    world.creaking_mut(id).unwrap().state.can_move = false;
    let traits = world.family_traits(id).unwrap();
    assert!(!traits.pushable);
    assert_eq!(traits.knockback_resistance, 1.0);
    assert!(world.families.push_body(KEY, id).is_none());
    let sprint = PlayerAttack {
        sprinting: true,
        ..sword(1, at(0.5, 2.5), 1.0)
    };
    let hit = world.player_attack(&sprint, id);
    assert!(hit.hurt && !hit.died && !hit.full_hit && hit.knockback);
    assert_eq!(creaking_of(&world, id).core.body.velocity, DVec3::ZERO);
    let mob = world.creaking_mut(id).unwrap();
    mob.state.can_move = true;
    mob.state.invulnerable = 0;
    assert!(world.families.push_body(KEY, id).is_some());
    let hit = world.player_attack(&sprint, id);
    assert!(hit.hurt && !hit.died);
    assert!(
        creaking_of(&world, id).core.body.velocity.length() > 0.1,
        "a moving one is knocked back"
    );
}

#[test]
fn a_creaking_hits_for_three_every_forty_ticks_with_its_attack_animation() {
    let mut field = Field::default();
    let mut world = EntityWorld::default();
    let id = spawn(&mut world, at(0.5, 0.5));
    let player = player_at(1, at(0.5, 1.7));
    let target = middle(&world, id);
    world.set_player_views(vec![(1, away(&player, target))]);
    prime(&mut world, id);
    world.tick_with_players(&mut field, &[player]);
    let mob = world.creaking_mut(id).unwrap();
    mob.state.active = true;
    mob.state
        .brain
        .memories
        .attack_target
        .set(Target::Player(1));
    mob.state.brain.update_activity(true);
    let mut hits = Vec::new();
    for tick in 0..100 {
        world.tick_with_players(&mut field, &[player]);
        let struck = world.take_player_hits();
        let events = world.take_entity_events();
        if struck.is_empty() {
            continue;
        }
        let mob = creaking_of(&world, id);
        assert_eq!(mob.state.attack_animation, ATTACK_ANIMATION);
        assert_eq!(mob.state.attack_tick, Some(mob.core.tick_count));
        assert!(events.contains(&(id, ATTACK_EVENT)));
        for hit in struck {
            assert_eq!((hit.player_id, hit.damage, hit.source), (1, 3.0, Some(id)));
            assert!(matches!(hit.kind, PlayerHitKind::Melee { .. }));
            hits.push(tick);
        }
    }
    assert!(hits.len() >= 2, "{hits:?}");
    assert!(
        hits.windows(2)
            .all(|pair| pair[1] - pair[0] == brain::ATTACK_COOLDOWN + 1),
        "{hits:?}"
    );
    let mob = creaking_of(&world, id);
    assert!(mob.core.swings >= 2);
    let mut frozen = EntityWorld::default();
    let id = spawn(&mut frozen, at(0.5, 0.5));
    let target = middle(&frozen, id);
    frozen.set_player_views(vec![(1, towards(&player, target))]);
    prime(&mut frozen, id);
    for _ in 0..60 {
        frozen.tick_with_players(&mut field, &[player]);
    }
    let state = &creaking_of(&frozen, id).state;
    assert!(state.active && !state.can_move);
    assert!(
        frozen.take_player_hits().is_empty(),
        "no hits while looked at"
    );
}

#[test]
fn an_unbound_creaking_dies_from_any_hit_and_gives_no_experience() {
    let mut world = EntityWorld::default();
    world.set_mob_drops(true);
    let id = spawn(&mut world, at(0.5, 0.5));
    let nothing = by_player(
        0.0,
        "minecraft:player_attack",
        DamageSourceKind::PlayerAttack,
    );
    let hit = world.hurt_family(id, &nothing).unwrap();
    assert!(hit.applied && !hit.died && hit.full);
    let said = sounds(&mut world);
    assert!(
        heard(&said, HOSTILE_HURT_SOUND) && !heard(&said, SWAY_SOUND),
        "{said:?}"
    );
    let arrow = by_player(2.0, "minecraft:arrow", DamageSourceKind::Projectile);
    assert!(world.hurt_family(id, &arrow).unwrap().died);
    assert!(heard(&sounds(&mut world), DEATH_SOUND));
    let deaths = world.take_deaths();
    assert_eq!((deaths.len(), deaths[0].experience), (1, 0));
    let mob_hit = spawn(&mut world, at(4.5, 0.5));
    let bite = Hurt::new(3.0, "minecraft:mob_attack", DamageSourceKind::MobAttack)
        .by(Some(Target::Mob(99)), Some("minecraft:zombie"));
    assert!(world.hurt_family(mob_hit, &bite).unwrap().died);
    let burnt = spawn(&mut world, at(8.5, 0.5));
    let lava = Hurt::new(4.0, "minecraft:lava", DamageSourceKind::Hazard);
    assert!(world.hurt_family(burnt, &lava).unwrap().died);
    let deaths = world.take_deaths();
    assert!(deaths.len() == 2 && deaths.iter().all(|death| death.experience == 0));
}

#[test]
fn a_heart_bound_creaking_shudders_instead_of_taking_damage() {
    let mut field = Field::default();
    let mut world = EntityWorld::default();
    let id = spawn(&mut world, at(0.5, 0.5));
    let home = (0, 64, 3);
    assert!(world.bind_creaking(id, home));
    let mob = creaking_of(&world, id);
    assert!(mob.state.heart_bound() && mob.state.linked.is_none());
    let profile = &mob.state.profile;
    assert_eq!(profile.home, Some(home));
    let maluses = [
        PathType::Damaging,
        PathType::PowderSnow,
        PathType::Lava,
        PathType::Fire,
        PathType::FireInNeighbor,
    ]
    .map(|path_type| profile.malus(path_type));
    assert_eq!(maluses, [8.0, 8.0, 8.0, 0.0, 0.0]);
    assert!(world.family_traits(id).unwrap().fire_immune);
    let hit = world.player_attack(&sword(1, at(0.5, 2.5), 10.0), id);
    assert!(hit.hurt && !hit.died && !hit.full_hit);
    let mob = creaking_of(&world, id);
    assert_eq!(
        (mob.core.health, mob.state.invulnerable),
        (1.0, INVULNERABILITY)
    );
    assert_eq!(mob.state.shudder_tick, Some(mob.core.tick_count));
    assert_eq!(mob.core.damage.hurts, 0, "no red flash");
    let said = sounds(&mut world);
    assert!(
        heard(&said, SWAY_SOUND) && !heard(&said, HOSTILE_HURT_SOUND),
        "{said:?}"
    );
    assert_eq!(world.take_creaking_heart_calls(), vec![(id, home)]);
    assert!(world.take_creaking_heart_calls().is_empty());
    assert!(!world.player_attack(&sword(1, at(0.5, 2.5), 10.0), id).hurt);
    for _ in 0..INVULNERABILITY {
        world.tick_with_players(&mut field, &[]);
    }
    let arrow = by_player(9.0, "minecraft:arrow", DamageSourceKind::Projectile);
    assert!(world.hurt_family(id, &arrow).unwrap().applied);
    assert_eq!(world.take_creaking_heart_calls(), vec![(id, home)]);
    for _ in 0..INVULNERABILITY {
        world.tick_with_players(&mut field, &[]);
    }
    sounds(&mut world);
    let bite = Hurt::new(3.0, "minecraft:mob_attack", DamageSourceKind::MobAttack)
        .by(Some(Target::Mob(99)), Some("minecraft:zombie"));
    assert!(world.hurt_family(id, &bite).unwrap().applied);
    assert!(
        world.take_creaking_heart_calls().is_empty(),
        "no player to blame"
    );
    assert!(heard(&sounds(&mut world), SWAY_SOUND));
    for _ in 0..INVULNERABILITY {
        world.tick_with_players(&mut field, &[]);
    }
    let fall = Hurt::new(20.0, "minecraft:fall", DamageSourceKind::Generic);
    let lava = Hurt::new(4.0, "minecraft:lava", DamageSourceKind::Hazard);
    let bolt = Hurt::new(5.0, "minecraft:lightning_bolt", DamageSourceKind::Hazard);
    let fireball = by_player(6.0, "minecraft:fireball", DamageSourceKind::Projectile);
    for refused in [fall, lava, bolt, fireball] {
        assert!(
            !world.hurt_family(id, &refused).unwrap().applied,
            "{refused:?}"
        );
    }
    let mob = creaking_of(&world, id);
    assert_eq!((mob.core.health, mob.state.invulnerable), (1.0, 0));
    let kill = Hurt::new(
        f32::MAX,
        "minecraft:generic_kill",
        DamageSourceKind::Generic,
    );
    assert!(world.hurt_family(id, &kill).unwrap().died);
    assert!(heard(&sounds(&mut world), DEATH_SOUND));
}

#[test]
fn an_unlinked_heart_bound_creaking_dies_but_a_new_one_waits_for_its_heart() {
    let mut field = Field::default();
    let mut world = EntityWorld::default();
    world.set_mob_drops(true);
    let home = (0, 64, 3);
    let id = spawn(&mut world, at(0.5, 0.5));
    let free = spawn(&mut world, at(6.5, 0.5));
    world.bind_creaking(id, home);
    for _ in 0..5 {
        world.tick_with_players(&mut field, &[]);
    }
    assert_eq!(creaking_of(&world, id).core.health, 1.0, "never marked yet");
    world.link_creakings(&[(id, home, 7)]);
    let state = &creaking_of(&world, id).state;
    assert_eq!((state.linked, state.emitter), (Some(true), 7));
    assert!(
        creaking_of(&world, free).state.linked.is_none(),
        "an unbound one is never linked"
    );
    world.tick_with_players(&mut field, &[]);
    assert_eq!(creaking_of(&world, id).core.health, 1.0);
    let links = world.creaking_links();
    assert_eq!(links.len(), 1);
    let link = links[0];
    assert_eq!(
        (link.id, link.home, link.persistent, link.tearing_down),
        (id, home, false, false)
    );
    assert_eq!(link.centre, link.position + DVec3::Y * 1.35);
    world.link_creakings(&[(id, (9, 64, 9), 7)]);
    assert_eq!(
        creaking_of(&world, id).state.linked,
        Some(false),
        "another heart"
    );
    sounds(&mut world);
    assert!(world.player_attack(&sword(1, at(0.5, 2.5), 10.0), id).hurt);
    assert!(!heard(&sounds(&mut world), SWAY_SOUND));
    assert!(world.take_creaking_heart_calls().is_empty());
    world.tick_with_players(&mut field, &[]);
    let mob = creaking_of(&world, id);
    assert!(mob.core.health == 0.0 && mob.core.damage.dead);
    assert!(world.take_deaths().is_empty(), "no death of its own");
    assert!(!heard(&sounds(&mut world), DEATH_SOUND));
    let fresh = spawn(&mut world, at(-6.5, 0.5));
    world.bind_creaking(fresh, home);
    for _ in 0..25 {
        world.tick_with_players(&mut field, &[]);
    }
    assert!(!present(&world, id), "gone after its death clock");
    assert_eq!(
        creaking_of(&world, fresh).core.health,
        1.0,
        "the new one waits"
    );
    assert_eq!(creaking_of(&world, free).core.health, 1.0);
    world.link_creakings(&[(fresh, home, 0)]);
    world.tick_with_players(&mut field, &[]);
    assert_eq!(creaking_of(&world, fresh).core.health, 1.0);
}

#[test]
fn a_torn_down_creaking_twitches_forty_five_ticks_and_leaves_without_a_second_death() {
    let mut field = Field::default();
    let mut world = EntityWorld::default();
    world.set_mob_drops(true);
    let home = (0, 64, 3);
    let id = spawn(&mut world, at(0.5, 0.5));
    world.bind_creaking(id, home);
    let killed = TearDown::Killed {
        by: Some(Target::Player(1)),
    };
    assert!(world.tear_down_creaking(id, killed));
    let mob = creaking_of(&world, id);
    assert!(mob.state.tearing_down && mob.core.health == 0.0 && mob.core.damage.dead);
    assert!(heard(&sounds(&mut world), TWITCH_SOUND));
    let deaths = world.take_deaths();
    assert_eq!(
        (deaths.len(), deaths[0].id, deaths[0].experience),
        (1, id, 0)
    );
    assert!(deaths[0].context.killed_by_player);
    assert_eq!(deaths[0].context.attacker, Some("minecraft:player"));
    for tick in 1..=TWITCH_DEATH {
        world.tick_with_players(&mut field, &[]);
        assert!(present(&world, id), "still twitching at {tick}");
        assert_eq!(creaking_of(&world, id).state.death_time, tick);
    }
    assert!(!heard(&sounds(&mut world), DEATH_SOUND));
    world.tick_with_players(&mut field, &[]);
    assert!(!present(&world, id));
    assert!(heard(&sounds(&mut world), DEATH_SOUND));
    assert!(world.take_deaths().is_empty(), "no second death");
    let now = spawn(&mut world, at(4.5, 0.5));
    world.bind_creaking(now, home);
    assert!(world.tear_down_creaking(now, TearDown::Now));
    assert!(!world.tear_down_creaking(now, TearDown::Now), "once");
    world.tick_with_players(&mut field, &[]);
    assert!(!present(&world, now));
    assert!(heard(&sounds(&mut world), DEATH_SOUND));
    assert!(world.take_deaths().is_empty());
}

#[test]
fn a_player_stuck_in_a_creaking_counts_past_four_heart_ticks() {
    let mut field = Field::default();
    let mut world = EntityWorld::default();
    let id = spawn(&mut world, at(0.5, 0.5));
    prime(&mut world, id);
    let near = creative_at(1, at(2.5, 0.5));
    world.tick_with_players(&mut field, &[near]);
    assert_eq!(
        creaking_of(&world, id).state.eyes,
        [near.position + DVec3::Y * 1.62]
    );
    let feet = creaking_of(&world, id).core.body.position;
    world.creaking_mut(id).unwrap().state.eyes = vec![feet + DVec3::Y * 1.62];
    let stuck: Vec<bool> = (0..6).map(|_| world.creaking_player_stuck(id)).collect();
    assert_eq!(stuck, [false, false, false, false, true, true]);
    world.creaking_mut(id).unwrap().state.eyes = vec![feet + DVec3::new(2.0, 1.62, 0.0)];
    assert!(!world.creaking_player_stuck(id));
    assert_eq!(creaking_of(&world, id).state.stuck, 0);
    world.creaking_mut(id).unwrap().state.eyes.clear();
    assert!(!world.creaking_player_stuck(id));
}

#[test]
fn a_heart_bound_creaking_never_paths_past_thirty_two_blocks_from_home() {
    assert!(!beyond_home((0, 64, 0), (32, 64, 0), (20, 64, 0)));
    assert!(beyond_home((0, 64, 0), (33, 64, 0), (20, 64, 0)));
    assert!(
        !beyond_home((0, 64, 0), (33, 64, 0), (40, 64, 0)),
        "no further than it stands"
    );
    let field = Field::default();
    let mut world = EntityWorld::default();
    let id = spawn(&mut world, at(20.5, 0.5));
    let walker = Walker {
        position: at(20.5, 0.5),
        on_ground: true,
        in_floatable_fluid: false,
    };
    let free = find_walk_path(&field, &walk_profile(), walker, at(45.5, 0.5), 0);
    assert!(free.reached, "{:?}", free.nodes.last());
    world.bind_creaking(id, (0, 64, 0));
    let profile = creaking_of(&world, id).state.profile.clone();
    let held = find_walk_path(&field, &profile, walker, at(45.5, 0.5), 0);
    assert!(!held.reached);
    assert!(
        held.nodes
            .iter()
            .all(|&(x, y, z)| x * x + (y - 64) * (y - 64) + z * z <= 1024),
        "{:?}",
        held.nodes
    );
    assert!(held.nodes.last().is_some_and(|&(x, _, _)| x >= 30));
}

#[test]
fn illagers_flee_a_creaking() {
    let mut field = Field::default();
    let mut world = EntityWorld::default();
    let id = spawn(&mut world, at(0.5, 0.5));
    world.creaking_mut(id).unwrap().core.no_ai = true;
    let illager = Illager::new(IllagerKind::Vindicator, at(3.5, 0.5));
    let vindicator = world.spawn_illager(illager, false);
    let distance = |world: &EntityWorld| {
        let at = world
            .illagers()
            .iter()
            .find(|e| e.id == vindicator)
            .unwrap()
            .illager
            .body
            .position;
        at.distance(creaking_of(world, id).core.body.position)
    };
    let start = distance(&world);
    for _ in 0..80 {
        world.tick_with_players(&mut field, &[]);
    }
    assert!(
        distance(&world) > start + 3.0,
        "{start} {}",
        distance(&world)
    );
}

#[test]
fn an_iron_golem_hunts_a_creaking() {
    let mut field = Field::default();
    let mut world = EntityWorld::default();
    let id = spawn(&mut world, at(0.5, 0.5));
    let golem = world.spawn_iron_golem(IronGolem::new(at(6.5, 0.5)), false);
    world.iron_golem_mut(golem).unwrap().random = LegacyRandom::new(5);
    for _ in 0..400 {
        world.tick_with_players(&mut field, &[]);
        if !present(&world, id) {
            break;
        }
    }
    assert!(!present(&world, id) || creaking_of(&world, id).core.health <= 0.0);
}
