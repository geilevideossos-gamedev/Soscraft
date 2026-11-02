use super::ai::{self, PandaGoals};
use super::*;
use crate::controls::PlayerControls;
use crate::walk_path::WalkTarget;
use crate::world::families::land::animal_goals::Mate;
use crate::world::{DamageSourceKind, LeashHolder, PlayerAttack, PlayerHit};
use minecraftoss_player::{Block, Pos};

#[derive(Default)]
struct Grove {
    bamboo: Vec<Pos>,
    pen: bool,
}

impl World for Grove {
    fn block(&self, pos: Pos) -> Option<Block> {
        if self.bamboo.contains(&pos) {
            return Some(Block::new(ai::BAMBOO));
        }
        let wall = pos.0.abs() == PEN || pos.2.abs() == PEN;
        if self.pen && wall && (64..=65).contains(&pos.1) {
            return Some(Block::new("minecraft:stone"));
        }
        (pos.1 < 64).then(|| Block::new("minecraft:grass_block"))
    }

    fn set_block(&mut self, _: Pos, _: Option<Block>) {}
}

const PEN: i32 = 4;

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

fn attack(player_id: u64, position: DVec3, attack_damage: f64) -> PlayerAttack {
    PlayerAttack {
        player_id,
        position,
        yaw: 0.0,
        attack_damage,
        strength: 1.0,
        sprinting: false,
        can_critical: false,
        can_sweep: false,
    }
}

fn punch() -> Hurt {
    Hurt::new(
        1.0,
        "minecraft:player_attack",
        DamageSourceKind::PlayerAttack,
    )
    .by(Some(Target::Player(1)), Some("minecraft:player"))
}

fn panda_at(world: &mut EntityWorld, position: DVec3) -> u64 {
    world.spawn_family_kind(KIND, position).unwrap()
}

fn panda_with(world: &mut EntityWorld, position: DVec3, gene: Gene) -> u64 {
    let id = panda_at(world, position);
    panda_mut(world, id).state.set_genes(gene, gene);
    id
}

fn cub_at(world: &mut EntityWorld, position: DVec3) -> u64 {
    let mut mob = Panda::spawn(KIND, position).unwrap();
    mob.state.age.set(Age::BABY_START);
    set_baby(&mut mob, true);
    world.spawn_family(mob)
}

fn panda(world: &EntityWorld, id: u64) -> &FamilyMob<Panda> {
    world
        .family::<Panda>()
        .iter()
        .find(|mob| mob.core.id == id)
        .unwrap()
}

fn panda_mut(world: &mut EntityWorld, id: u64) -> &mut FamilyMob<Panda> {
    world.family_mob_mut::<Panda>(id).unwrap()
}

fn goals(world: &EntityWorld, id: u64) -> &PandaGoals {
    panda(world, id).state.goals().unwrap()
}

fn holding(item: &str, count: u8) -> Inventory {
    let mut inventory = Inventory::default();
    inventory.slots[0] = Some(ItemStack::new(item, count));
    inventory
}

fn count(inventory: &Inventory) -> Option<u8> {
    inventory.slots[0].as_ref().map(|stack| stack.count)
}

#[test]
fn a_panda_is_a_big_twenty_health_animal_with_its_kind() {
    let mut world = EntityWorld::default();
    world.set_mob_drops(true);
    let adult = panda_at(&mut world, at(0.5, 0.5));
    let cub = cub_at(&mut world, at(6.5, 0.5));
    assert_eq!(world.entity_type(adult), Some(KIND));
    assert_eq!(world.family_core(adult).map(|(key, _)| key), Some(52));
    let core = &panda(&world, adult).core;
    assert_eq!(
        (
            core.body.width,
            core.body.height,
            core.eye_height,
            core.baby
        ),
        (1.3, 1.25, 1.0625, false)
    );
    assert_eq!((core.health, core.max_health), (20.0, 20.0));
    assert_eq!(core.ambient_interval, 120);
    let state = &panda(&world, adult).state;
    assert_eq!(state.speed, 0.15000000596046448);
    assert_eq!(
        (state.main_gene, state.hidden_gene),
        (Gene::Normal, Gene::Normal)
    );
    assert!(
        state.can_pick_up_loot,
        "the constructor lets it pick up loot"
    );
    assert_eq!((state.held.as_ref(), state.drop_chance), (None, 0.085));
    assert_eq!(ATTACK_DAMAGE, 6.0);
    let census = world.census();
    assert_eq!(census.iter().filter(|c| c.kind == KIND).count(), 2);
    let candidates = world.mob_candidates();
    assert!(
        candidates
            .iter()
            .any(|m| m.id == adult && m.kind == KIND && !m.baby)
    );
    assert!(
        candidates
            .iter()
            .any(|m| m.id == cub && m.kind == KIND && m.baby)
    );
    assert!(!crate::monster_ai::is_enemy(KIND));
    let traits = world.family_traits(adult).unwrap();
    assert_eq!(traits.category, "creature");
    assert!(!traits.leashable && !traits.enemy && !traits.undead);
    assert!(!traits.fire_immune && !traits.breathes_water && !traits.panicking);
    assert_eq!(traits.voice, ("panda", 1.0, "neutral_volume"));
    assert_eq!(Panda::step_sound(KIND), Some(STEP_SOUND));
    let registered = MonsterKind::Registered(KIND);
    assert_eq!(registered.goal_names(), GOAL_NAMES);
    assert_eq!(registered.target_goal_names(), TARGET_NAMES);
    let ai = &panda(&world, adult).state.ai;
    assert_eq!(ai.state.follow_range, FOLLOW_RANGE);
    assert_eq!(ai.state.eye_height, EYE_HEIGHT);
    assert!(ai.state.walk.can_float, "its FloatGoal floats the path");
    assert_eq!(ai.state.walk.walk_target, WalkTarget::Animal);
    let player = player_at(1, at(3.5, 0.5));
    for id in [adult, cub] {
        assert!(
            world
                .player_attack(&attack(1, player.position, 100.0), id)
                .died
        );
    }
    let deaths = world.take_deaths();
    let adult = deaths.iter().find(|d| d.id == adult).unwrap();
    assert_eq!(adult.table, Some(KIND), "bamboo from entities/panda");
    assert!((1..=3).contains(&adult.experience), "{}", adult.experience);
    let cub = deaths.iter().find(|d| d.id == cub).unwrap();
    assert_eq!((cub.table, cub.experience), (None, 0), "cubs drop nothing");
}

#[test]
fn a_cub_is_half_size_with_its_own_eye_height() {
    let mut world = EntityWorld::default();
    let mut grove = Grove::default();
    let cub = cub_at(&mut world, at(0.5, 0.5));
    let mob = panda(&world, cub);
    assert_eq!(
        (
            mob.core.body.width,
            mob.core.body.height,
            mob.core.eye_height,
            mob.core.baby
        ),
        (0.65, 0.625, 0.28125, true)
    );
    let ai = &mob.state.ai.state;
    assert_eq!(
        (ai.walk.width, ai.walk.height, ai.eye_height),
        (0.65, 0.625, 0.28125)
    );
    world.tick(&mut grove);
    assert!(panda(&world, cub).state.age.ticks > Age::BABY_START);
    panda_mut(&mut world, cub).state.age.ticks = -1;
    world.tick(&mut grove);
    let grown = panda(&world, cub);
    assert!(!grown.core.baby && grown.state.age.ticks == 0, "it grew up");
    assert_eq!(
        (
            grown.core.body.width,
            grown.core.body.height,
            grown.core.eye_height
        ),
        (WIDTH, HEIGHT, EYE_HEIGHT)
    );
}

#[test]
fn random_genes_follow_the_sixteen_slot_table() {
    use Gene::*;
    let table = [
        Lazy, Worried, Playful, Weak, Aggressive, Weak, Weak, Weak, Weak, Brown, Brown, Normal,
        Normal, Normal, Normal, Normal,
    ];
    for (roll, gene) in table.into_iter().enumerate() {
        assert_eq!(Gene::from_roll(roll as u32), gene, "roll {roll}");
    }
    let mut counts = std::collections::HashMap::new();
    let mut random = LegacyRandom::new(1600);
    for _ in 0..16000 {
        *counts.entry(Gene::random(&mut random)).or_insert(0) += 1;
    }
    for (gene, slots) in [
        (Lazy, 1),
        (Worried, 1),
        (Playful, 1),
        (Aggressive, 1),
        (Weak, 5),
        (Brown, 2),
        (Normal, 5),
    ] {
        let expected = 16000 * slots / 16;
        let found = counts[&gene];
        assert!(
            (expected * 8 / 10..=expected * 12 / 10).contains(&found),
            "{gene:?}: {found} of about {expected}"
        );
    }
    for (id, gene) in Gene::ALL.into_iter().enumerate() {
        assert_eq!(gene.id(), id as i32);
        assert_eq!(Gene::from_name(gene.name()), gene);
    }
    assert_eq!(
        Gene::from_name("sleepy"),
        Normal,
        "an unknown name is normal"
    );
    let recessive: Vec<Gene> = Gene::ALL.into_iter().filter(|g| g.recessive()).collect();
    assert_eq!(recessive, vec![Brown, Weak]);
}

#[test]
fn a_recessive_gene_shows_only_when_both_match() {
    use Gene::*;
    for (main, hidden, shown) in [
        (Brown, Brown, Brown),
        (Brown, Normal, Normal),
        (Brown, Weak, Normal),
        (Weak, Weak, Weak),
        (Weak, Lazy, Normal),
        (Normal, Brown, Normal),
        (Lazy, Normal, Lazy),
        (Aggressive, Brown, Aggressive),
        (Playful, Worried, Playful),
    ] {
        assert_eq!(Gene::variant(main, hidden), shown, "{main:?} {hidden:?}");
    }
    let mut world = EntityWorld::default();
    let id = panda_at(&mut world, at(0.5, 0.5));
    panda_mut(&mut world, id).state.set_genes(Weak, Brown);
    assert_eq!(panda(&world, id).state.variant(), Normal);
    assert_eq!(goals(&world, id).variant, Normal);
    panda_mut(&mut world, id).state.set_genes(Worried, Brown);
    assert_eq!(
        goals(&world, id).variant,
        Worried,
        "the goals see the variant"
    );
}

#[test]
fn a_cub_takes_a_gene_from_each_parent_and_mutates_one_time_in_thirty_two() {
    use Gene::*;
    let first = (Lazy, Worried);
    let second = (Playful, Aggressive);
    let of_first = |gene: Gene| gene == Lazy || gene == Worried;
    let of_second = |gene: Gene| gene == Playful || gene == Aggressive;
    let (mut inherited, mut main_of_first) = (0, 0);
    let mut mains = std::collections::HashSet::new();
    let trials = 6400;
    let mut random = LegacyRandom::new(6400);
    let mut a = LegacyRandom::new(7);
    let mut b = LegacyRandom::new(13);
    for _ in 0..trials {
        let (main, hidden) = genes_from_parents(&mut random, first, &mut a, second, &mut b);
        if (of_first(main) && of_second(hidden)) || (of_second(main) && of_first(hidden)) {
            inherited += 1;
            mains.insert(main);
            if of_first(main) {
                main_of_first += 1;
            }
        }
    }
    let mutated = trials - inherited;
    assert!(
        (trials * 3 / 100..=trials * 9 / 100).contains(&mutated),
        "about one gene in 32 mutates: {mutated} of {trials}"
    );
    assert!(
        (inherited * 45 / 100..=inherited * 55 / 100).contains(&main_of_first),
        "either parent gives the main gene: {main_of_first} of {inherited}"
    );
    assert_eq!(mains.len(), 4, "either gene of a parent: {mains:?}");
    let mut random = LegacyRandom::new(3);
    let mut a = LegacyRandom::new(4);
    let mut b = LegacyRandom::new(5);
    let mut weak = 0;
    for _ in 0..320 {
        let genes = genes_from_parents(&mut random, (Weak, Weak), &mut a, (Weak, Weak), &mut b);
        if Gene::variant(genes.0, genes.1) == Weak {
            weak += 1;
        }
    }
    assert!(weak > 270, "two weak parents make weak cubs: {weak}");
}

#[test]
fn weak_pandas_have_ten_health_and_lazy_ones_walk_at_0_07() {
    use Gene::*;
    for (main, hidden, health, speed) in [
        (Weak, Weak, 10.0, SPEED),
        (Weak, Normal, 20.0, SPEED),
        (Normal, Weak, 20.0, SPEED),
        (Lazy, Normal, 20.0, LAZY_SPEED),
        (Normal, Lazy, 20.0, SPEED),
        (Aggressive, Aggressive, 20.0, SPEED),
    ] {
        let mut mob = Panda::spawn(KIND, at(0.5, 0.5)).unwrap();
        mob.state.set_genes(main, hidden);
        set_attributes(&mut mob);
        assert_eq!(
            (mob.core.max_health, mob.core.health, mob.state.speed),
            (health, health, speed),
            "{main:?} {hidden:?}"
        );
    }
    assert_eq!(LAZY_SPEED, 0.07000000029802322);
    let walked = |gene: Gene| {
        let mut world = EntityWorld::default();
        let mut grove = Grove::default();
        let id = panda_at(&mut world, at(0.5, 0.5));
        let mob = panda_mut(&mut world, id);
        mob.state.set_genes(gene, gene);
        set_attributes(mob);
        let player = player_at(1, at(9.5, 0.5));
        world.set_player_controls(&[PlayerControls {
            id: 1,
            main_hand: Some("minecraft:bamboo".to_owned()),
            ..PlayerControls::default()
        }]);
        for _ in 0..100 {
            world.tick_with_players(&mut grove, &[player]);
        }
        panda(&world, id).core.body.position.distance(at(0.5, 0.5))
    };
    let (lazy, normal) = (walked(Lazy), walked(Normal));
    assert!(
        lazy > 0.2 && lazy < normal * 0.6,
        "lazy {lazy}, normal {normal}"
    );
}

#[test]
fn bamboo_within_seven_blocks_across_and_two_up_counts() {
    for (offset, found) in [
        ((7, 0, 0), true),
        ((-7, 2, 7), true),
        ((7, 2, -7), true),
        ((0, 1, 0), true),
        ((8, 0, 0), false),
        ((0, 0, -8), false),
        ((0, 3, 0), false),
        ((0, -1, 0), false),
    ] {
        let grove = Grove {
            bamboo: vec![offset],
            pen: false,
        };
        assert_eq!(
            can_find_bamboo(&grove, DVec3::new(0.5, 0.0, 0.5)),
            found,
            "{offset:?}"
        );
    }
}

struct Breeding {
    world: EntityWorld,
    pandas: [u64; 2],
    cub: Option<u64>,
    cant_breed: Vec<i32>,
    max_unhappy: i32,
}

fn breeding(bamboo: Option<Pos>, gene: Gene) -> Breeding {
    let mut world = EntityWorld::default();
    world.set_mob_drops(true);
    let mut grove = Grove {
        bamboo: bamboo.into_iter().collect(),
        pen: true,
    };
    let first = panda_with(&mut world, at(0.5, 0.5), gene);
    let second = panda_with(&mut world, at(-1.5, 0.5), gene);
    let player = player_at(1, at(-0.5, 2.5));
    world.tick_with_players(&mut grove, &[player]);
    let mut bamboo = holding("minecraft:bamboo", 4);
    for id in [first, second] {
        let (result, events, _) = interact(&mut world, id, &mut bamboo, 0, false).unwrap();
        assert_eq!(result, InteractionResult::SuccessServer);
        assert_eq!(events, vec![AnimalEvent::Hearts]);
    }
    assert_eq!(world.take_entity_events(), vec![(first, 18), (second, 18)]);
    let mut out = Breeding {
        world,
        pandas: [first, second],
        cub: None,
        cant_breed: Vec::new(),
        max_unhappy: 0,
    };
    for tick in 0..640 {
        out.world.tick_with_players(&mut grove, &[player]);
        for sound in out.world.take_sounds() {
            if sound.event == CANT_BREED_SOUND {
                assert_eq!((sound.volume, sound.pitch), (1.0, 1.0));
                out.cant_breed.push(tick);
            }
        }
        for id in [first, second] {
            let unhappy = panda(&out.world, id).state.unhappy();
            out.max_unhappy = out.max_unhappy.max(unhappy);
        }
        out.cub = out
            .world
            .family::<Panda>()
            .iter()
            .map(|mob| mob.core.id)
            .find(|id| ![first, second].contains(id));
        if out.cub.is_some() {
            break;
        }
    }
    out
}

fn sulking(world: &Grove, tick_count: i32, unhappy: i32) -> (i32, Option<u64>, bool) {
    let mob = Panda::spawn(KIND, at(0.5, 0.5)).unwrap();
    let mut ai = (*mob.state.ai).clone();
    ai.retain_goals(&["PandaBreedGoal"]);
    ai.state.body.on_ground = true;
    ai.state.tick_count = tick_count;
    ai.state.players = vec![
        player_at(1, at(0.5, 9.5)),
        player_at(2, at(7.0, 0.5)),
        player_at(3, at(-5.5, 0.5)),
    ];
    let goals = ai.state.extra.get_mut::<PandaGoals>().unwrap();
    goals.unhappy = unhappy;
    goals.animal.in_love = true;
    goals.animal.mates = vec![Mate {
        id: 9,
        position: at(2.5, 0.5),
        width: WIDTH,
        height: HEIGHT,
        eye_height: EYE_HEIGHT,
        alive: true,
        in_love: true,
        panicking: false,
    }];
    ai.tick_goals(world, true);
    let breeds = ai.running_goals().contains(&"PandaBreedGoal");
    let unhappy = ai.state.extra.get::<PandaGoals>().unwrap().unhappy;
    (unhappy, ai.state.look_at_player.target_id, breeds)
}

#[test]
fn pandas_breed_only_with_bamboo_within_seven_blocks_and_sulk_otherwise() {
    let bare = Grove::default();
    let (unhappy, looked_at, breeds) = sulking(&bare, 10, 0);
    assert_eq!((unhappy, breeds), (ai::UNHAPPY_TICKS, false));
    assert_eq!(looked_at, Some(3), "the nearest player within eight");
    assert_eq!(sulking(&bare, 10, 5), (5, None, false), "already sulking");
    let grove = Grove {
        bamboo: vec![(7, 66, 0)],
        pen: false,
    };
    assert_eq!(
        sulking(&grove, 10, 0),
        (0, None, true),
        "bamboo near: it breeds"
    );

    let mut near = breeding(Some((5, 64, 0)), Gene::Weak);
    let cub = near.cub.expect("bamboo seven blocks off: a cub");
    assert!(near.cant_breed.is_empty() && near.max_unhappy == 0);
    let events = near.world.take_entity_events();
    assert!(
        events
            .iter()
            .any(|&(id, event)| near.pandas.contains(&id) && event == 18),
        "{events:?}"
    );
    for id in near.pandas {
        let parent = &panda(&near.world, id).state;
        assert!((5999..=6000).contains(&parent.age.ticks), "{id}");
        assert_eq!(parent.in_love, 0, "{id}");
    }
    let orbs = near.world.take_trade_experience();
    assert_eq!(orbs.len(), 1);
    assert!(orbs.iter().all(|(_, xp)| (1..=7).contains(xp)));
    let child = panda(&near.world, cub);
    assert!(child.core.baby && child.state.age.ticks < -23990);
    assert!(child.state.bred, "a bred cub saves its attributes");
    assert_eq!(
        (child.core.body.width, child.core.body.height),
        (BABY_WIDTH, BABY_HEIGHT)
    );
    let weak = child.state.variant() == Gene::Weak;
    assert_eq!(
        child.core.max_health,
        if weak { 10.0 } else { 20.0 },
        "{:?} {:?}",
        child.state.main_gene,
        child.state.hidden_gene
    );
    assert_eq!(child.core.health, child.core.max_health);

    let far = breeding(Some((12, 64, 0)), Gene::Normal);
    assert_eq!(far.cub, None, "bamboo eight or more blocks off: no cub");
    assert_eq!(
        far.max_unhappy,
        ai::UNHAPPY_TICKS - 1,
        "set to 32 and one off in the same tick"
    );
    assert_eq!(
        far.cant_breed.len(),
        4,
        "two sad sounds from each, once in a love: {:?}",
        far.cant_breed
    );
    let sounds = &far.cant_breed;
    for tick in sounds {
        let paired = sounds.contains(&(tick + 15)) || sounds.contains(&(tick - 15));
        assert!(paired, "at 29 and 14 ticks left: {sounds:?}");
    }
    for id in far.pandas {
        assert_eq!(panda(&far.world, id).state.in_love, 0, "the love ran out");
    }

    let none = breeding(None, Gene::Normal);
    assert_eq!(none.cub, None);
    assert_eq!(none.cant_breed.len(), 4);
}

fn fight(gene: Gene) -> (Vec<PlayerHit>, usize, EntityWorld, u64) {
    let mut world = EntityWorld::default();
    let mut grove = Grove::default();
    let id = panda_with(&mut world, at(0.5, 0.5), gene);
    let player = player_at(1, at(3.5, 0.5));
    for _ in 0..3 {
        world.tick_with_players(&mut grove, &[player]);
    }
    assert!(
        world
            .player_attack(&attack(1, player.position, 1.0), id)
            .hurt
    );
    let (mut hits, mut bites) = (Vec::new(), 0);
    for _ in 0..200 {
        world.tick_with_players(&mut grove, &[player]);
        hits.extend(world.take_player_hits());
        bites += world
            .take_sounds()
            .iter()
            .filter(|sound| sound.event == BITE_SOUND)
            .count();
    }
    (hits, bites, world, id)
}

#[test]
fn a_hit_panda_bites_back_once_unless_aggressive() {
    let (hits, bites, world, id) = fight(Gene::Normal);
    assert_eq!(hits.len(), 1, "one bite back");
    assert_eq!(bites, 1);
    let hit = &hits[0];
    assert_eq!((hit.player_id, hit.damage, hit.source), (1, 6.0, Some(id)));
    assert!(matches!(hit.kind, PlayerHitKind::Melee { .. }));
    assert_eq!(panda(&world, id).state.target(), None, "it walks off");
    assert!(!goals(&world, id).did_bite, "cleared with the target");

    let (hits, bites, world, id) = fight(Gene::Aggressive);
    assert!(hits.len() >= 4, "it keeps biting: {}", hits.len());
    assert_eq!(bites, hits.len());
    assert!(hits.iter().all(|hit| hit.damage == 6.0));
    assert_eq!(panda(&world, id).state.target(), Some(Target::Player(1)));
}

#[test]
fn only_aggressive_pandas_join_a_fight() {
    let mut world = EntityWorld::default();
    let mut grove = Grove::default();
    let hit = panda_with(&mut world, at(0.5, 0.5), Gene::Normal);
    let angry = panda_with(&mut world, at(6.5, 0.5), Gene::Aggressive);
    let calm = panda_with(&mut world, at(0.5, 6.5), Gene::Normal);
    let lazy = panda_with(&mut world, at(-6.5, 0.5), Gene::Lazy);
    let far = panda_with(&mut world, at(22.5, 0.5), Gene::Aggressive);
    let player = player_at(1, at(0.5, -5.5));
    for _ in 0..3 {
        world.tick_with_players(&mut grove, &[player]);
    }
    assert!(world.hurt_family(hit, &punch()).unwrap().applied);
    for _ in 0..4 {
        world.tick_with_players(&mut grove, &[player]);
    }
    let target = |id: u64| panda(&world, id).state.target();
    assert_eq!(target(hit), Some(Target::Player(1)), "it bites back");
    assert_eq!(
        target(angry),
        Some(Target::Player(1)),
        "the aggressive one joins"
    );
    assert_eq!(target(calm), None);
    assert_eq!(target(lazy), None);
    assert_eq!(target(far), None, "too far to be called");
}

#[test]
fn bamboo_calms_an_angry_panda() {
    let mut world = EntityWorld::default();
    let mut grove = Grove::default();
    let id = panda_with(&mut world, at(0.5, 0.5), Gene::Aggressive);
    let player = player_at(1, at(3.5, 0.5));
    for _ in 0..3 {
        world.tick_with_players(&mut grove, &[player]);
    }
    assert!(world.hurt_family(id, &punch()).unwrap().applied);
    let mut bitten = false;
    for _ in 0..60 {
        world.tick_with_players(&mut grove, &[player]);
        bitten |= !world.take_player_hits().is_empty();
    }
    assert!(bitten);
    assert_eq!(panda(&world, id).state.target(), Some(Target::Player(1)));
    let mut bamboo = holding("minecraft:bamboo", 2);
    let (result, _, _) = interact(&mut world, id, &mut bamboo, 0, false).unwrap();
    assert_ne!(result, InteractionResult::Pass);
    assert!(goals(&world, id).got_bamboo);
    for _ in 0..3 {
        world.tick_with_players(&mut grove, &[player]);
    }
    assert_eq!(panda(&world, id).state.target(), None, "bamboo calms it");
    assert!(!goals(&world, id).got_bamboo, "cleared with the target");
    let _ = world.take_player_hits();
    for _ in 0..100 {
        world.tick_with_players(&mut grove, &[player]);
        assert!(world.take_player_hits().is_empty(), "no more bites");
    }
}

fn panics(damage_type: &'static str, source: DamageSourceKind) -> bool {
    let mut world = EntityWorld::default();
    let mut grove = Grove::default();
    let id = panda_at(&mut world, at(0.5, 0.5));
    for _ in 0..3 {
        world.tick(&mut grove);
    }
    let hurt = Hurt::new(1.0, damage_type, source);
    assert!(world.hurt_family(id, &hurt).unwrap().applied);
    let mut panicked = false;
    for _ in 0..40 {
        world.tick(&mut grove);
        let mob = panda(&world, id);
        if mob.state.panicking() {
            panicked = true;
            assert!(world.family_traits(id).unwrap().panicking);
            let speed = mob.state.ai.state.navigation.speed_modifier;
            assert_eq!(speed, ai::PANIC_SPEED);
        }
    }
    panicked
}

#[test]
fn a_panda_panics_from_fire_and_cactus_but_not_from_a_punch() {
    use DamageSourceKind::{Generic, Hazard, MobAttack, PlayerAttack, Projectile};
    assert!(panics("minecraft:in_fire", Hazard));
    assert!(panics("minecraft:on_fire", Hazard));
    assert!(panics("minecraft:lava", Hazard));
    assert!(panics("minecraft:cactus", Hazard));
    assert!(panics("minecraft:hot_floor", Hazard));
    assert!(!panics("minecraft:player_attack", PlayerAttack));
    assert!(!panics("minecraft:mob_attack", MobAttack));
    assert!(!panics("minecraft:arrow", Projectile));
    assert!(!panics("minecraft:fall", Generic));
}

fn flees(gene: Gene, setup: impl FnOnce(&mut EntityWorld) -> Vec<PlayerCandidate>) -> bool {
    let mut world = EntityWorld::default();
    let mut grove = Grove::default();
    let id = panda_with(&mut world, at(0.5, 0.5), gene);
    let players = setup(&mut world);
    let mut fled = false;
    for _ in 0..40 {
        world.tick_with_players(&mut grove, &players);
        fled |= panda(&world, id)
            .state
            .running_goals()
            .contains(&"PandaAvoidGoal");
    }
    fled
}

#[test]
fn a_worried_panda_runs_from_players_within_eight_and_monsters_within_four() {
    let player = |x: f64| move |_: &mut EntityWorld| vec![player_at(1, at(x, 0.5))];
    assert!(flees(Gene::Worried, player(6.5)), "a player 6 blocks off");
    assert!(
        !flees(Gene::Worried, player(10.5)),
        "a player 10 blocks off"
    );
    assert!(!flees(Gene::Normal, player(6.5)), "only worried pandas run");
    let creative = |_: &mut EntityWorld| {
        vec![PlayerCandidate {
            attackable: false,
            ..player_at(1, at(5.5, 0.5))
        }]
    };
    assert!(!flees(Gene::Worried, creative), "a creative player");
    let zombie = |x: f64| {
        move |world: &mut EntityWorld| {
            world.spawn_zombie(crate::zombie::Zombie::new(at(x, 0.5)), true);
            Vec::<PlayerCandidate>::new()
        }
    };
    assert!(flees(Gene::Worried, zombie(3.5)), "a zombie 3 blocks off");
    assert!(!flees(Gene::Worried, zombie(6.5)), "a zombie 6 blocks off");
    assert!(!flees(Gene::Playful, zombie(3.5)));
    let slime = |world: &mut EntityWorld| {
        world.spawn_slime(crate::slime::Slime::new(at(3.5, 0.5), 1), true);
        Vec::<PlayerCandidate>::new()
    };
    assert!(!flees(Gene::Worried, slime), "a slime is no monster");
}

#[test]
fn bamboo_grows_a_cub_and_brings_an_adult_into_love() {
    let mut world = EntityWorld::default();
    let adult = panda_at(&mut world, at(0.5, 0.5));
    let cub = cub_at(&mut world, at(4.5, 0.5));
    let mut bamboo = holding("minecraft:bamboo", 5);
    let (result, events, _) = interact(&mut world, adult, &mut bamboo, 0, false).unwrap();
    assert_eq!(result, InteractionResult::SuccessServer);
    assert_eq!(events, vec![AnimalEvent::Hearts]);
    assert_eq!(panda(&world, adult).state.in_love, 600);
    assert_eq!(count(&bamboo), Some(4));
    assert_eq!(world.take_entity_events(), vec![(adult, 18)]);
    let (result, _, _) = interact(&mut world, adult, &mut bamboo, 0, false).unwrap();
    assert_eq!(result, InteractionResult::Pass, "in love it takes no more");
    assert_eq!(count(&bamboo), Some(4));

    let before = panda(&world, cub).state.age.ticks;
    let (result, events, _) = interact(&mut world, cub, &mut bamboo, 0, false).unwrap();
    assert_ne!(result, InteractionResult::Pass);
    assert!(events.is_empty() && world.take_entity_events().is_empty());
    assert_eq!(count(&bamboo), Some(3));
    let grown = panda(&world, cub).state.age.ticks - before;
    assert_eq!(grown, (-before / 20) / 10 * 20, "a tenth of the time left");

    let mut dandelion = holding("minecraft:golden_dandelion", 1);
    let (_, events, _) = interact(&mut world, cub, &mut dandelion, 0, false).unwrap();
    assert_eq!(events, vec![AnimalEvent::AgeLock]);
    assert!(panda(&world, cub).state.age.locked && panda(&world, cub).core.persistent);
    let (result, _, _) = interact(&mut world, cub, &mut bamboo, 0, false).unwrap();
    assert_eq!(result, InteractionResult::Pass, "a locked cub takes none");
    assert_eq!(count(&bamboo), Some(3));

    let parent = panda_at(&mut world, at(8.5, 0.5));
    panda_mut(&mut world, parent).state.age.set(PARENT_AGE);
    let (result, _, _) = interact(&mut world, parent, &mut bamboo, 0, false).unwrap();
    assert_eq!(result, InteractionResult::Pass);
    let fresh = panda_at(&mut world, at(12.5, 0.5));
    let mut wheat = holding("minecraft:wheat", 1);
    let (result, _, _) = interact(&mut world, fresh, &mut wheat, 0, false).unwrap();
    assert_eq!(result, InteractionResult::Pass, "only bamboo feeds a panda");
    let mut dandelion = holding("minecraft:golden_dandelion", 1);
    let (result, _, _) = interact(&mut world, fresh, &mut dandelion, 0, false).unwrap();
    assert_eq!(
        result,
        InteractionResult::Pass,
        "an adult cannot be age locked"
    );
    assert_eq!(count(&bamboo), Some(3));
}

#[test]
fn a_tempted_panda_follows_bamboo() {
    for (item, follows) in [("minecraft:bamboo", true), ("minecraft:wheat", false)] {
        let mut world = EntityWorld::default();
        let mut grove = Grove::default();
        let id = panda_at(&mut world, at(0.5, 0.5));
        let player = player_at(1, at(8.5, 0.5));
        world.set_player_controls(&[PlayerControls {
            id: 1,
            offhand: Some(item.to_owned()),
            ..PlayerControls::default()
        }]);
        let mut tempted = false;
        for _ in 0..400 {
            world.tick_with_players(&mut grove, &[player]);
            tempted |= panda(&world, id).state.tempted();
        }
        assert_eq!(tempted, follows, "{item}");
        if follows {
            let distance = panda(&world, id)
                .core
                .body
                .position
                .distance(player.position);
            assert!(distance < 4.5, "{item}: {distance}");
        }
    }
}

#[test]
fn a_panda_refuses_a_lead() {
    let mut world = EntityWorld::default();
    let id = panda_at(&mut world, at(0.5, 0.5));
    assert!(!world.can_be_leashed(id));
    assert!(!world.can_attach_leash(id, LeashHolder::Player(1)));
    assert!(!world.family_traits(id).unwrap().leashable);
}

#[test]
fn aggressive_and_worried_pandas_have_their_own_ambient_sounds() {
    for (gene, event) in [
        (Gene::Normal, "entity.panda.ambient"),
        (Gene::Lazy, "entity.panda.ambient"),
        (Gene::Aggressive, AGGRESSIVE_AMBIENT),
        (Gene::Worried, WORRIED_AMBIENT),
    ] {
        let mut world = EntityWorld::default();
        let mut grove = Grove::default();
        let id = panda_with(&mut world, at(0.5, 0.5), gene);
        let mut heard = Vec::new();
        for _ in 0..20 {
            panda_mut(&mut world, id).core.ambient_sound_time = 1000;
            world.tick(&mut grove);
            for sound in world.take_sounds() {
                if sound.event.ends_with("ambient") {
                    assert_eq!(sound.category, "neutral_volume");
                    assert!((0.8..=1.2).contains(&sound.pitch), "{}", sound.pitch);
                    heard.push(sound.event);
                }
            }
        }
        assert!(!heard.is_empty(), "{gene:?}");
        assert!(heard.iter().all(|e| e == event), "{gene:?}: {heard:?}");
    }
}
