use super::ai::{self, PandaGoals};
use super::*;
use crate::controls::PlayerControls;
use crate::fluid::FluidFrame;
use crate::walk_path::WalkTarget;
use crate::world::families::land::animal_goals::Mate;
use crate::world::{DamageSourceKind, LeashHolder, PlayerAttack, PlayerHit};
use minecraftoss_player::{Block, Pos};

#[derive(Default)]
struct Grove {
    bamboo: Vec<Pos>,
    pen: bool,
    ledge: bool,
    items: Vec<WorldItem>,
    next_item: i32,
    dropped: Vec<(i32, DVec3)>,
}

impl Grove {
    fn drop_item(&mut self, position: DVec3, item: &str, count: i32, pickup_delay: i32) -> i32 {
        self.next_item += 1;
        self.items.push(WorldItem {
            id: self.next_item,
            position,
            item: item.to_owned(),
            count,
            components: None,
            pickup_delay,
        });
        self.next_item
    }

    fn age_items(&mut self) {
        for item in &mut self.items {
            item.pickup_delay = (item.pickup_delay - 1).max(0);
        }
    }

    fn has(&self, id: i32) -> bool {
        self.items.iter().any(|item| item.id == id)
    }
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
        let floor = if self.ledge && pos.2 >= 1 { 60 } else { 64 };
        (pos.1 < floor).then(|| Block::new("minecraft:grass_block"))
    }

    fn set_block(&mut self, _: Pos, _: Option<Block>) {}

    fn items_in(&self, min: DVec3, max: DVec3) -> Vec<WorldItem> {
        let half = ai::ITEM_SIZE / 2.0;
        let touches = |p: DVec3| {
            p.x + half > min.x
                && p.x - half < max.x
                && p.y + ai::ITEM_SIZE > min.y
                && p.y < max.y
                && p.z + half > min.z
                && p.z - half < max.z
        };
        let near = self.items.iter().filter(|item| touches(item.position));
        near.cloned().collect()
    }

    fn item(&self, id: i32) -> Option<WorldItem> {
        self.items.iter().find(|item| item.id == id).cloned()
    }

    fn take_item(&mut self, id: i32, count: i32) {
        if let Some(item) = self.items.iter_mut().find(|item| item.id == id) {
            item.count -= count;
        }
        self.items.retain(|item| item.count > 0);
    }

    fn spawn_item(&mut self, position: DVec3, stack: &ItemStack, velocity: DVec3, delay: i32) {
        let id = self.drop_item(position, &stack.id, i32::from(stack.count), delay);
        self.dropped.push((id, velocity));
    }
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
            ..Grove::default()
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
        ..Grove::default()
    };
    let first = panda_with(&mut world, at(0.5, 0.5), gene);
    let second = panda_with(&mut world, at(-1.5, 0.5), gene);
    let player = player_at(1, at(-0.5, 2.5));
    world.tick_with_players(&mut grove, &[player]);
    let mut bamboo = holding("minecraft:bamboo", 4);
    for id in [first, second] {
        let (result, events, _, _) = interact(&mut world, id, &mut bamboo, 0, false).unwrap();
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
        ..Grove::default()
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
    let (result, _, _, _) = interact(&mut world, id, &mut bamboo, 0, false).unwrap();
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
    let (result, events, _, _) = interact(&mut world, adult, &mut bamboo, 0, false).unwrap();
    assert_eq!(result, InteractionResult::SuccessServer);
    assert_eq!(events, vec![AnimalEvent::Hearts]);
    assert_eq!(panda(&world, adult).state.in_love, 600);
    assert_eq!(count(&bamboo), Some(4));
    assert_eq!(world.take_entity_events(), vec![(adult, 18)]);
    let (result, events, _, _) = interact(&mut world, adult, &mut bamboo, 0, false).unwrap();
    assert_eq!(result, InteractionResult::SuccessServer);
    assert!(events.is_empty(), "in love it sits down to eat one instead");
    assert!(goals(&world, adult).sitting && goals(&world, adult).eating());
    assert_eq!(count(&bamboo), Some(3));

    let before = panda(&world, cub).state.age.ticks;
    let (result, events, _, _) = interact(&mut world, cub, &mut bamboo, 0, false).unwrap();
    assert_ne!(result, InteractionResult::Pass);
    assert!(events.is_empty() && world.take_entity_events().is_empty());
    assert_eq!(count(&bamboo), Some(2));
    let grown = panda(&world, cub).state.age.ticks - before;
    assert_eq!(grown, (-before / 20) / 10 * 20, "a tenth of the time left");

    let mut dandelion = holding("minecraft:golden_dandelion", 1);
    let (_, events, _, _) = interact(&mut world, cub, &mut dandelion, 0, false).unwrap();
    assert_eq!(events, vec![AnimalEvent::AgeLock]);
    assert!(panda(&world, cub).state.age.locked && panda(&world, cub).core.persistent);
    let (result, _, _, _) = interact(&mut world, cub, &mut bamboo, 0, false).unwrap();
    assert_eq!(result, InteractionResult::Pass, "a locked cub takes none");
    assert_eq!(count(&bamboo), Some(2));

    let parent = panda_at(&mut world, at(8.5, 0.5));
    panda_mut(&mut world, parent).state.age.set(PARENT_AGE);
    let (result, _, _, _) = interact(&mut world, parent, &mut bamboo, 0, false).unwrap();
    assert_eq!(result, InteractionResult::SuccessServer, "a parent eats it");
    assert!(world.take_entity_events().is_empty());
    let fresh = panda_at(&mut world, at(12.5, 0.5));
    let mut wheat = holding("minecraft:wheat", 1);
    let (result, _, _, _) = interact(&mut world, fresh, &mut wheat, 0, false).unwrap();
    assert_eq!(result, InteractionResult::Pass, "only bamboo feeds a panda");
    let mut dandelion = holding("minecraft:golden_dandelion", 1);
    let (result, _, _, _) = interact(&mut world, fresh, &mut dandelion, 0, false).unwrap();
    assert_eq!(
        result,
        InteractionResult::Pass,
        "an adult cannot be age locked"
    );
    assert_eq!(count(&bamboo), Some(1));
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
fn the_sit_back_and_roll_amounts_ease_in_and_out_like_vanilla() {
    let mut world = EntityWorld::default();
    let mut grove = Grove::default();
    let id = panda_at(&mut world, at(0.5, 0.5));
    let close = |a: f32, b: f32| (a - b).abs() < 1e-5;
    let state = &panda(&world, id).state;
    for amount in [state.sit_amount, state.back_amount, state.roll_amount] {
        assert_eq!(amount, Amount::default());
    }
    panda_mut(&mut world, id).state.goals_mut().sitting = true;
    let mut sat = Vec::new();
    for _ in 0..8 {
        let mob = panda_mut(&mut world, id);
        mob.core.look.pitch = 30.0;
        mob.state.ai.state.look_control.pitch = 30.0;
        world.tick(&mut grove);
        let mob = panda(&world, id);
        assert_eq!(
            (mob.core.look.pitch, mob.state.ai.state.look_control.pitch),
            (0.0, 0.0),
            "a sitting panda looks straight ahead"
        );
        sat.push(mob.state.sit_amount);
    }
    let up = [0.15, 0.3, 0.45, 0.6, 0.75, 0.9, 1.0, 1.0];
    for (tick, (amount, expected)) in sat.iter().zip(up).enumerate() {
        assert!(close(amount.now, expected), "tick {tick}: {amount:?}");
        let old = if tick == 0 { 0.0 } else { up[tick - 1] };
        assert!(close(amount.old, old), "tick {tick}: {amount:?}");
    }
    assert!(close(sat[2].at(0.5), 0.375), "lerped by the partial tick");
    assert!(close(sat[2].at(0.0), 0.3) && close(sat[2].at(1.0), 0.45));
    let back = panda(&world, id).state.back_amount;
    assert_eq!(back, Amount::default(), "only the sit amount moved");

    let goals = panda_mut(&mut world, id).state.goals_mut();
    goals.sitting = false;
    goals.on_back = true;
    goals.rolling = true;
    let mut fell = Vec::new();
    for _ in 0..7 {
        world.tick(&mut grove);
        let state = &panda(&world, id).state;
        fell.push((state.sit_amount.now, state.back_amount.now, state.roll_amount.now));
    }
    let down = [0.81, 0.62, 0.43, 0.24, 0.05, 0.0, 0.0];
    for (tick, &(sit, back, roll)) in fell.iter().enumerate() {
        assert!(close(sit, down[tick]), "tick {tick}: {sit}");
        assert!(close(back, up[tick]) && close(roll, up[tick]), "tick {tick}");
    }

    let goals = panda_mut(&mut world, id).state.goals_mut();
    goals.on_back = false;
    goals.rolling = false;
    goals.eat_counter = 3;
    assert!(goals.eating() && !goals.can_perform_action());
    goals.eat_counter = 0;
    assert!(!goals.eating() && goals.can_perform_action());
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

const CAKE: &str = "minecraft:cake";

fn fed_parent(world: &mut EntityWorld, position: DVec3) -> u64 {
    let id = panda_at(world, position);
    panda_mut(world, id).state.age.set(PARENT_AGE);
    let mut bamboo = holding(ai::BAMBOO, 2);
    let (result, events, _, dropped) = interact(world, id, &mut bamboo, 0, false).unwrap();
    assert_eq!(result, InteractionResult::SuccessServer);
    assert!(events.is_empty() && dropped.is_none());
    id
}

fn held(world: &EntityWorld, id: u64) -> Option<(String, u8)> {
    let held = panda(world, id).state.held.as_ref();
    held.map(|stack| (stack.id.clone(), stack.count))
}

fn tempting(world: &mut EntityWorld, at: DVec3) -> PlayerCandidate {
    world.set_player_controls(&[PlayerControls {
        id: 1,
        main_hand: Some(ai::BAMBOO.to_owned()),
        ..PlayerControls::default()
    }]);
    player_at(1, at)
}

#[test]
fn a_panda_picks_up_a_whole_stack_of_bamboo_or_a_cake_only_with_mob_griefing_and_an_empty_paw() {
    let mut world = EntityWorld::default();
    let mut grove = Grove::default();
    let eater = panda_at(&mut world, at(0.5, 0.5));
    let baker = panda_at(&mut world, at(8.5, 0.5));
    let full = panda_at(&mut world, at(16.5, 0.5));
    let shy = panda_at(&mut world, at(24.5, 0.5));
    for id in [eater, baker, full, shy] {
        panda_mut(&mut world, id).core.no_ai = true;
    }
    let stick = ItemStack::new("minecraft:stick", 1);
    panda_mut(&mut world, full).state.held = Some(stick);
    panda_mut(&mut world, shy).state.can_pick_up_loot = false;
    let stalks = grove.drop_item(at(2.0, 0.5), ai::BAMBOO, 5, 0);
    let wheat = grove.drop_item(at(0.5, 1.5), "minecraft:wheat", 3, 0);
    let far = grove.drop_item(at(0.5, 3.0), ai::BAMBOO, 1, 0);
    let late = grove.drop_item(at(8.5, 1.5), ai::BAMBOO, 2, 20);
    let cake = grove.drop_item(at(9.5, 0.5), CAKE, 1, 0);
    let spare = grove.drop_item(at(16.5, 1.5), ai::BAMBOO, 1, 0);
    let ignored = grove.drop_item(at(24.5, 1.5), ai::BAMBOO, 1, 0);
    world.set_mob_griefing(false);
    for _ in 0..3 {
        world.tick(&mut grove);
        grove.age_items();
    }
    assert_eq!(grove.items.len(), 7, "nothing without mob_griefing");
    assert_eq!(held(&world, eater), None);
    world.set_mob_griefing(true);
    world.tick(&mut grove);
    assert_eq!(
        held(&world, eater),
        Some((ai::BAMBOO.to_owned(), 5)),
        "the whole stack"
    );
    assert_eq!(panda(&world, eater).state.drop_chance, GUARANTEED_DROP);
    assert!(!grove.has(stalks));
    assert!(
        grove.has(wheat) && grove.has(far),
        "not wheat, nor bamboo out of reach"
    );
    assert_eq!(held(&world, baker), Some((CAKE.to_owned(), 1)));
    assert!(
        !grove.has(cake) && grove.has(late),
        "bamboo it may not pick up yet stays"
    );
    assert_eq!(held(&world, full), Some(("minecraft:stick".to_owned(), 1)));
    assert!(grove.has(spare), "a full paw takes nothing");
    assert_eq!(held(&world, shy), None);
    assert!(grove.has(ignored), "nor a panda that may not pick up loot");
}

#[test]
fn a_sitting_panda_eats_after_eighty_ticks_and_finishes_after_a_hundred() {
    let mut seeds = LegacyRandom::new(2604);
    let mut pauses = 0;
    for _ in 0..24 {
        let mut world = EntityWorld::default();
        let mut grove = Grove::default();
        let id = fed_parent(&mut world, at(0.5, 0.5));
        panda_mut(&mut world, id).core.random = LegacyRandom::new(seeds.next_long());
        let start = panda(&world, id).core.body.position;
        let mut last = goals(&world, id).eat_counter;
        assert_eq!(last, 1, "fed, it starts eating at once");
        let mut finished = false;
        for _ in 0..4000 {
            world.tick(&mut grove);
            let now = goals(&world, id).eat_counter;
            for sound in world.take_sounds() {
                if sound.event != EAT_SOUND {
                    continue;
                }
                assert!(
                    last > 0 && last % 5 == 0,
                    "a munch every fifth tick: {last}"
                );
                assert!(sound.volume == 0.5 || sound.volume == 1.0);
                assert!((0.8..=1.2).contains(&sound.pitch), "{}", sound.pitch);
                assert_eq!(sound.category, "neutral_volume");
            }
            let events = world.take_game_events();
            let eaten = events.iter().filter(|e| e.event == EAT_EVENT).count();
            let mob = panda(&world, id);
            let sitting = mob.state.goals().unwrap().sitting;
            if sitting {
                let moved = mob.core.body.position - start;
                assert!(
                    moved.x.abs() < 1e-9 && moved.z.abs() < 1e-9,
                    "it sits still"
                );
            }
            if last > 0 && now == 0 {
                assert!(last > EAT_PAUSE, "it eats past eighty: {last}");
                if mob.state.held.is_some() {
                    assert!(sitting && eaten == 0, "a pause");
                    pauses += 1;
                } else {
                    assert!(last > EAT_DONE, "it finishes past a hundred: {last}");
                    assert!(!sitting, "and stands up");
                    assert_eq!(eaten, 1, "with the eat game event");
                    finished = true;
                    break;
                }
            } else {
                assert_eq!(eaten, 0);
            }
            last = now;
        }
        assert!(finished);
    }
    assert!(pauses > 0, "now and then it pauses and starts again");
}

#[test]
fn feeding_a_panda_in_love_seats_it_with_one_bamboo_and_returns_its_old_item() {
    let mut world = EntityWorld::default();
    let mut grove = Grove::default();
    let id = panda_at(&mut world, at(0.5, 0.5));
    let mut bamboo = holding(ai::BAMBOO, 5);
    let (_, events, _, dropped) = interact(&mut world, id, &mut bamboo, 0, false).unwrap();
    assert_eq!(events, vec![AnimalEvent::Hearts]);
    assert!(dropped.is_none());
    assert_eq!(world.take_entity_events(), vec![(id, 18)]);
    let mob = panda_mut(&mut world, id);
    mob.state.held = Some(ItemStack::new(CAKE, 1));
    mob.state.drop_chance = GUARANTEED_DROP;
    mob.state.ai.forward = 0.4;
    let (result, events, position, dropped) =
        interact(&mut world, id, &mut bamboo, 0, false).unwrap();
    assert_eq!(result, InteractionResult::SuccessServer);
    assert!(events.is_empty() && world.take_entity_events().is_empty());
    assert_eq!(position, at(0.5, 0.5));
    let dropped = dropped.map(|stack| (stack.id, stack.count));
    assert_eq!(dropped, Some((CAKE.to_owned(), 1)), "its old item");
    assert_eq!(count(&bamboo), Some(3));
    assert_eq!(held(&world, id), Some((ai::BAMBOO.to_owned(), 1)));
    let mob = panda(&world, id);
    assert_eq!(
        mob.state.drop_chance, GUARANTEED_DROP,
        "the slot stays guaranteed"
    );
    assert!(mob.state.in_love > 0);
    let sat = mob.state.goals().unwrap();
    assert!(sat.sitting && sat.eat_counter == 1);
    assert_eq!(mob.state.ai.forward, 0.0);
    assert!(mob.state.ai.state.navigation.is_done());
    let (result, _, _, _) = interact(&mut world, id, &mut bamboo, 0, false).unwrap();
    assert_eq!(result, InteractionResult::Pass, "not while it sits");
    assert_eq!(count(&bamboo), Some(3));

    let player = tempting(&mut world, at(3.5, 0.5));
    for _ in 0..40 {
        world.tick_with_players(&mut grove, &[player]);
        let position = panda(&world, id).core.body.position;
        assert_eq!((position.x, position.z), (0.5, 0.5), "tempted, it sits on");
    }
    assert!(goals(&world, id).eating());

    let creative = panda_at(&mut world, at(8.5, 0.5));
    panda_mut(&mut world, creative).state.in_love = 300;
    panda_mut(&mut world, creative).state.held = Some(ItemStack::new(CAKE, 1));
    let (result, _, _, dropped) = interact(&mut world, creative, &mut bamboo, 0, true).unwrap();
    assert_eq!(result, InteractionResult::SuccessServer);
    assert!(
        dropped.is_none(),
        "a creative player's bamboo replaces the cake"
    );
    assert_eq!(count(&bamboo), Some(3));
    assert_eq!(held(&world, creative), Some((ai::BAMBOO.to_owned(), 1)));

    let wet = panda_at(&mut world, at(16.5, 0.5));
    panda_mut(&mut world, wet).state.in_love = 300;
    let fluid = &mut panda_mut(&mut world, wet).state.ai.state.fluid;
    *fluid = FluidFrame {
        water_height: 0.4,
        lava_height: 0.0,
    };
    let (result, _, _, _) = interact(&mut world, wet, &mut bamboo, 0, false).unwrap();
    assert_eq!(result, InteractionResult::Pass, "not in water");
    assert!(!goals(&world, wet).sitting && held(&world, wet).is_none());
}

fn sitter(lazy: bool, seed: u64) -> MonsterAi {
    let mob = Panda::spawn(KIND, at(0.5, 0.5)).unwrap();
    let mut ai = (*mob.state.ai).clone();
    ai.retain_goals(&["PandaSitGoal"]);
    ai.state.body.on_ground = true;
    ai.state.random = LegacyRandom::new(seed);
    if lazy {
        sit_goals(&mut ai).variant = Gene::Lazy;
    }
    ai
}

fn sit_goals(ai: &mut MonsterAi) -> &mut PandaGoals {
    ai.state.extra.get_mut::<PandaGoals>().unwrap()
}

#[test]
fn a_panda_that_stops_sitting_drops_its_item_and_waits() {
    let grove = Grove::default();
    let food = WorldItem {
        id: 1,
        position: at(3.5, 0.5),
        item: ai::BAMBOO.to_owned(),
        count: 1,
        components: None,
        pickup_delay: 0,
    };
    let wet = FluidFrame {
        water_height: 0.4,
        lava_height: 0.0,
    };
    let mut seeds = LegacyRandom::new(1717);
    let mut waits = [Vec::new(), Vec::new()];
    for (index, lazy) in [false, true].into_iter().enumerate() {
        for _ in 0..40 {
            let mut ai = sitter(lazy, seeds.next_long());
            ai.state.tick_count = 100;
            sit_goals(&mut ai).holding = true;
            ai.tick_goals(&grove, true);
            assert_eq!(ai.running_goals(), ["PandaSitGoal"]);
            let goals = sit_goals(&mut ai);
            assert!(goals.sitting && std::mem::take(&mut goals.sat_down));
            ai.state.fluid = wet;
            ai.state.tick_count = 102;
            ai.tick_goals(&grove, true);
            assert!(ai.running_goals().is_empty(), "in water it gets up");
            let goals = sit_goals(&mut ai);
            assert!(!goals.sitting && !goals.holding);
            assert_eq!(goals.dropped, Some(at(0.5, 0.5)), "and lets go of it");
            goals.food = vec![food.clone()];
            ai.state.fluid = FluidFrame::default();
            let mut tick = 102;
            while ai.running_goals().is_empty() {
                tick += 2;
                ai.state.tick_count = tick;
                ai.tick_goals(&grove, true);
            }
            waits[index].push(tick - 102);
            assert!(
                !ai.state.navigation.is_done(),
                "it walks to the bamboo it saw"
            );
            assert!(!sit_goals(&mut ai).sitting);
        }
    }
    for (index, most) in [(0, 3180), (1, 1180)] {
        let ok = waits[index].iter().all(|wait| (200..=most).contains(wait));
        assert!(ok, "{:?}", waits[index]);
    }
    assert!(
        waits[0].iter().any(|&wait| wait > 1180),
        "only lazy pandas always sit again within a minute"
    );

    let mut ai = sitter(false, 5);
    ai.state.tick_count = 100;
    sit_goals(&mut ai).food = vec![food.clone()];
    ai.tick_goals(&grove, true);
    assert_eq!(ai.running_goals(), ["PandaSitGoal"]);
    assert!(!sit_goals(&mut ai).sitting && !ai.state.navigation.is_done());
    ai.state.fluid = wet;
    ai.state.tick_count = 102;
    ai.tick_goals(&grove, true);
    assert_eq!(
        sit_goals(&mut ai).dropped,
        None,
        "nothing held, nothing dropped"
    );
    ai.state.fluid = FluidFrame::default();
    ai.state.tick_count = 104;
    ai.tick_goals(&grove, true);
    assert_eq!(ai.running_goals(), ["PandaSitGoal"], "and no wait");

    let mut world = EntityWorld::default();
    let mut grove = Grove::default();
    let id = panda_at(&mut world, at(0.5, 0.5));
    panda_mut(&mut world, id).state.held = Some(ItemStack::new(CAKE, 1));
    for _ in 0..4 {
        world.tick(&mut grove);
    }
    assert!(goals(&world, id).sitting, "it sits down with its cake");
    let feet = panda(&world, id).core.body.position;
    let player = tempting(&mut world, at(4.5, 0.5));
    for _ in 0..4 {
        world.tick_with_players(&mut grove, &[player]);
    }
    assert_eq!(
        held(&world, id),
        None,
        "bamboo tempts it up, and it lets go"
    );
    assert!(!goals(&world, id).sitting);
    let cake = &grove.items[0];
    assert_eq!(
        (cake.item.as_str(), cake.count, cake.pickup_delay),
        (CAKE, 1, PICKUP_DELAY)
    );
    assert_eq!(cake.position, feet, "at its feet");
    let (_, push) = grove.dropped[0];
    assert!(
        push.y == 0.2 && push.x.abs() <= 0.1 && push.z.abs() <= 0.1,
        "{push}"
    );
}

#[test]
fn a_held_item_drops_on_death_always_when_picked_up() {
    let mut seeds = LegacyRandom::new(8085);
    let mut dies = |held: Option<(&str, u8, f32)>, baby: bool, by_player: bool, drops: bool| {
        let mut world = EntityWorld::default();
        world.set_mob_drops(drops);
        let id = if baby {
            cub_at(&mut world, at(0.5, 0.5))
        } else {
            panda_at(&mut world, at(0.5, 0.5))
        };
        let mob = panda_mut(&mut world, id);
        mob.core.random = LegacyRandom::new(seeds.next_long());
        if let Some((item, count, chance)) = held {
            mob.state.held = Some(ItemStack::new(item, count));
            mob.state.drop_chance = chance;
        }
        if by_player {
            let hit = world.player_attack(&attack(1, at(3.5, 0.5), 100.0), id);
            assert!(hit.died);
        } else {
            let magic = Hurt::new(100.0, "minecraft:magic", DamageSourceKind::Magic);
            assert!(world.hurt_family(id, &magic).unwrap().died);
        }
        let deaths = world.take_deaths();
        let death = deaths.into_iter().find(|death| death.id == id).unwrap();
        let stacks = death.equipment.into_iter();
        stacks
            .map(|stack| (stack.id, stack.count))
            .collect::<Vec<_>>()
    };
    let picked = Some((ai::BAMBOO, 5, GUARANTEED_DROP));
    for by_player in [false, true] {
        let drops = dies(picked, false, by_player, true);
        assert_eq!(drops, [(ai::BAMBOO.to_owned(), 5)], "{by_player}");
        assert!(
            dies(picked, true, by_player, true).is_empty(),
            "a cub drops nothing"
        );
        assert!(
            dies(picked, false, by_player, false).is_empty(),
            "nor without mob_drops"
        );
    }
    let fed = Some((ai::BAMBOO, 1, DROP_CHANCE));
    let mut shaken = 0;
    for _ in 0..800 {
        assert!(
            dies(fed, false, false, true).is_empty(),
            "only a player's kill"
        );
        shaken += dies(fed, false, true, true).len();
    }
    assert!((40..=100).contains(&shaken), "{shaken} of 800");
    assert!(dies(None, false, true, true).is_empty());
}

#[test]
fn a_worried_panda_does_not_settle_on_picked_up_bamboo() {
    for (gene, settles) in [(Gene::Worried, false), (Gene::Normal, true)] {
        let mut world = EntityWorld::default();
        let mut grove = Grove::default();
        let id = panda_with(&mut world, at(0.5, 0.5), gene);
        panda_mut(&mut world, id).core.random = LegacyRandom::new(31);
        grove.drop_item(at(0.5, 0.5), ai::BAMBOO, 3, 0);
        let (mut sat, mut ate) = (0, false);
        for _ in 0..600 {
            world.tick(&mut grove);
            grove.age_items();
            let goals = goals(&world, id);
            sat += usize::from(goals.sitting);
            ate |= goals.eating();
        }
        assert_eq!(ate, settles, "{gene:?}");
        if !settles {
            assert_eq!(sat, 0, "it never stays sat");
            let lying: i32 = grove.items.iter().map(|item| item.count).sum();
            let kept = held(&world, id).map_or(0, |(_, count)| i32::from(count));
            assert_eq!(lying + kept, 3, "and eats none of it");
        }
    }
}

fn goal_only(name: &str, gene: Gene, baby: bool, seed: u64) -> MonsterAi {
    let mob = Panda::spawn(KIND, at(0.5, 0.5)).unwrap();
    let mut ai = (*mob.state.ai).clone();
    ai.retain_goals(&[name]);
    ai.state.body.on_ground = true;
    ai.state.random = LegacyRandom::new(seed);
    let goals = sit_goals(&mut ai);
    goals.variant = gene;
    goals.baby = baby;
    ai
}

fn goal_starts(
    ai: &mut MonsterAi,
    grove: &Grove,
    updates: i32,
    flag: fn(&mut PandaGoals) -> &mut bool,
) -> Vec<i32> {
    let mut starts = Vec::new();
    for update in 1..=updates {
        ai.state.tick_count = update * 2;
        ai.tick_goals(grove, true);
        if std::mem::take(flag(sit_goals(ai))) {
            starts.push(update);
        }
    }
    starts
}

fn sneezing(goals: &mut PandaGoals) -> &mut bool {
    &mut goals.sneezing
}

fn on_back(goals: &mut PandaGoals) -> &mut bool {
    &mut goals.on_back
}

fn rolling(goals: &mut PandaGoals) -> &mut bool {
    &mut goals.rolling
}

#[test]
fn a_weak_cub_sneezes_and_adults_within_ten_jump() {
    let grove = Grove::default();
    let sneezes = |gene: Gene, baby: bool, able: bool| {
        let mut ai = goal_only("PandaSneezeGoal", gene, baby, 77);
        sit_goals(&mut ai).sitting = !able;
        goal_starts(&mut ai, &grove, 20000, sneezing).len()
    };
    let weak = sneezes(Gene::Weak, true, true);
    assert!(
        (55..=125).contains(&weak),
        "one in 250, then one in 3000: {weak}"
    );
    let plain = sneezes(Gene::Normal, true, true);
    assert!((1..=18).contains(&plain), "one in 3000: {plain}");
    assert_eq!(sneezes(Gene::Weak, false, true), 0, "adults never sneeze");
    assert_eq!(sneezes(Gene::Weak, true, false), 0, "nor a cub that sits");

    for mob_drops in [true, false] {
        let mut world = EntityWorld::default();
        world.set_mob_drops(mob_drops);
        let mut grove = Grove::default();
        let near = panda_at(&mut world, at(5.5, 0.5));
        let edge = panda_at(&mut world, at(10.0, 0.5));
        let far = panda_at(&mut world, at(12.5, 0.5));
        let high = panda_at(&mut world, DVec3::new(0.5, 75.0, 6.5));
        let sitting = panda_at(&mut world, at(0.5, 4.5));
        let wet = panda_at(&mut world, at(-4.5, 0.5));
        let small = cub_at(&mut world, at(0.5, -4.5));
        let cub = cub_at(&mut world, at(0.5, 0.5));
        for id in [near, edge, far, high, sitting, wet, small, cub] {
            let mob = panda_mut(&mut world, id);
            mob.core.no_ai = true;
            mob.core.body.on_ground = true;
        }
        panda_mut(&mut world, sitting).state.goals_mut().sitting = true;
        panda_mut(&mut world, wet).state.ai.state.fluid = FluidFrame {
            water_height: 0.4,
            lava_height: 0.0,
        };
        let mob = panda_mut(&mut world, cub);
        mob.state.set_genes(Gene::Weak, Gene::Weak);
        mob.state.goals_mut().sneezing = true;
        let mut counters = Vec::new();
        let mut heard = Vec::new();
        for tick in 1..=21 {
            world.tick(&mut grove);
            let goals = goals(&world, cub);
            counters.push((goals.sneezing, goals.sneeze_counter));
            for sound in world.take_sounds() {
                if sound.event.contains("sneeze") {
                    assert_eq!(sound.category, "neutral_volume");
                    assert_eq!((sound.volume, sound.pitch), (1.0, 1.0));
                    heard.push((tick, sound.event));
                }
            }
            if tick < 21 {
                let still = panda(&world, near).core.body.velocity.y == 0.0;
                assert!(still && world.take_family_gifts().is_empty(), "{tick}");
            }
        }
        let mut expected: Vec<_> = (1..=20).map(|counter| (true, counter)).collect();
        expected.push((false, 0));
        assert_eq!(counters, expected);
        assert_eq!(
            heard,
            [
                (1, PRE_SNEEZE_SOUND.to_owned()),
                (21, SNEEZE_SOUND.to_owned())
            ]
        );
        for id in [near, edge] {
            let body = &panda(&world, id).core.body;
            assert_eq!(
                body.velocity.y,
                f64::from(0.42_f32),
                "an adult within ten jumps"
            );
            assert!(body.needs_sync);
        }
        for id in [far, high, sitting, wet, small, cub] {
            let body = &panda(&world, id).core.body;
            assert_eq!(body.velocity.y, 0.0, "{id}");
        }
        let gifts = world.take_family_gifts();
        if mob_drops {
            let at = panda(&world, cub).core.body.position;
            assert_eq!(
                gifts,
                [crate::world::families::FamilyGift {
                    table: SNEEZE_GIFT,
                    this_type: KIND,
                    at,
                }]
            );
        } else {
            assert!(gifts.is_empty(), "no gift without mob_drops");
        }
    }
}

#[test]
fn a_lazy_panda_lies_on_its_back_until_clicked() {
    let grove = Grove::default();
    let lies = |gene: Gene, able: bool| {
        let mut ai = goal_only("PandaLieOnBackGoal", gene, false, 31);
        sit_goals(&mut ai).sitting = !able;
        goal_starts(&mut ai, &grove, 4000, on_back).len()
    };
    assert!(
        lies(Gene::Lazy, true) > 0,
        "a lazy panda lies down now and then"
    );
    for gene in [Gene::Normal, Gene::Worried, Gene::Playful, Gene::Weak] {
        assert_eq!(lies(gene, true), 0, "{gene:?}");
    }
    assert_eq!(lies(Gene::Lazy, false), 0, "not while it sits");

    let wet = FluidFrame {
        water_height: 0.4,
        lava_height: 0.0,
    };
    let mut ai = goal_only("PandaLieOnBackGoal", Gene::Lazy, false, 32);
    let mut update = 0;
    while !sit_goals(&mut ai).on_back {
        update += 1;
        ai.state.tick_count = update * 2;
        ai.tick_goals(&grove, true);
        assert!(update < 4000);
    }
    ai.state.fluid = wet;
    update += 1;
    let got_up = update * 2;
    ai.state.tick_count = got_up;
    ai.tick_goals(&grove, true);
    assert!(!sit_goals(&mut ai).on_back, "in water it gets up");
    ai.state.fluid = FluidFrame::default();
    while !sit_goals(&mut ai).on_back {
        update += 1;
        ai.state.tick_count = update * 2;
        ai.tick_goals(&grove, true);
    }
    assert!(update * 2 > got_up + 200, "it waits ten seconds first");

    let mut world = EntityWorld::default();
    let mut grove = Grove::default();
    let id = panda_with(&mut world, at(0.5, 0.5), Gene::Lazy);
    let mut ticks = 0;
    while !goals(&world, id).on_back {
        world.tick(&mut grove);
        ticks += 1;
        assert!(ticks < 20000, "a lazy panda lies down");
    }
    let yaw = panda(&world, id).core.yaw;
    for _ in 0..40 {
        world.tick(&mut grove);
        let mob = panda(&world, id);
        assert!(mob.state.goals().unwrap().on_back);
        assert_eq!(mob.core.yaw, yaw, "it stays put on its back");
    }
    let state = &panda(&world, id).state;
    assert_eq!(state.back_amount.now, 1.0);
    assert!(!state.goals().unwrap().can_perform_action());
    let mut empty = Inventory::default();
    let (result, events, _, dropped) = interact(&mut world, id, &mut empty, 0, false).unwrap();
    assert_eq!(result, InteractionResult::SuccessServer);
    assert!(events.is_empty() && dropped.is_none());
    let goals = goals(&world, id);
    assert!(
        !goals.on_back && goals.can_perform_action(),
        "a click gets it up"
    );
    world.tick(&mut grove);
    assert!(panda(&world, id).state.back_amount.now < 1.0);
}

#[test]
fn a_playful_panda_rolls_with_three_hops_in_thirty_two_ticks() {
    let grove = Grove::default();
    let rolls = |gene: Gene, baby: bool, on_ground: bool| {
        let mut ai = goal_only("PandaRollGoal", gene, baby, 5);
        ai.state.body.on_ground = on_ground;
        goal_starts(&mut ai, &grove, 6000, rolling).len()
    };
    let playful = rolls(Gene::Playful, false, true);
    assert!(
        (170..=280).contains(&playful),
        "one in 30, then one in 250: {playful}"
    );
    let cub = rolls(Gene::Normal, true, true);
    assert!((10..=40).contains(&cub), "one in 250: {cub}");
    assert!(
        rolls(Gene::Playful, true, true) > 150,
        "a playful cub rolls as often"
    );
    assert_eq!(
        rolls(Gene::Normal, false, true),
        0,
        "only playful pandas and cubs"
    );
    assert_eq!(rolls(Gene::Playful, false, false), 0, "not in the air");
    assert_eq!(ai::roll_ahead(0.0), (0, 1));
    assert_eq!(ai::roll_ahead(90.0), (-1, 0));
    assert_eq!(ai::roll_ahead(45.0), (-1, 1));
    assert_eq!(ai::roll_ahead(20.0), (0, 1));

    let mut world = EntityWorld::default();
    let mut grove = Grove::default();
    let id = panda_with(&mut world, at(0.5, 0.5), Gene::Playful);
    panda_mut(&mut world, id).state.ai.retain_goals(&[]);
    for _ in 0..3 {
        world.tick(&mut grove);
    }
    let start = panda(&world, id).core.body.position;
    let yaw = panda(&world, id).core.yaw;
    panda_mut(&mut world, id).state.goals_mut().rolling = true;
    let mut rolled = Vec::new();
    for _ in 0..34 {
        world.tick(&mut grove);
        let mob = panda(&world, id);
        let goals = mob.state.goals().unwrap();
        assert_eq!(mob.core.yaw, yaw, "it rolls straight on");
        let body = &mob.core.body;
        rolled.push((
            goals.rolling,
            goals.roll_counter,
            body.velocity,
            body.on_ground,
        ));
    }
    let counters: Vec<_> = rolled
        .iter()
        .map(|&(on, counter, ..)| (on, counter))
        .collect();
    let mut expected: Vec<_> = (1..=32).map(|counter| (true, counter)).collect();
    expected.extend([(false, 33), (false, 0)]);
    assert_eq!(counters, expected);
    let delta = panda(&world, id).state.roll_delta;
    let radians = f64::from(yaw * 0.017453292);
    let ahead = DVec3::new(-radians.sin(), 0.0, radians.cos());
    assert!((delta.length() - 0.2).abs() < 0.03, "{delta}");
    assert!(delta.dot(ahead) > 0.17, "{delta}");
    let mut hops = Vec::new();
    for &(_, counter, velocity, on_ground) in &rolled[..32] {
        if velocity.y == ROLL_LIFT {
            hops.push(counter);
        }
        if counter == 1 {
            assert_eq!(velocity, delta + DVec3::new(0.0, ROLL_LIFT, 0.0));
        } else if ROLL_HOPS.contains(&counter) {
            assert_eq!((velocity.x, velocity.z), (0.0, 0.0), "{counter}");
            assert!(!on_ground || velocity.y == ROLL_LIFT, "{counter}");
        } else {
            assert_eq!((velocity.x, velocity.z), (delta.x, delta.z), "{counter}");
        }
    }
    assert_eq!(hops, [1, 15, 23], "a hop at the start and on landing");
    let moved = panda(&world, id).core.body.position - start;
    assert!(moved.dot(ahead) > 3.0, "{moved}");
    let roll = panda(&world, id).state.roll_amount;
    assert!(
        roll.now < 1.0 && roll.now < roll.old,
        "it eases out of the roll"
    );
}

#[test]
fn a_playful_panda_rolls_off_a_ledge() {
    let ledge = Grove {
        ledge: true,
        ..Grove::default()
    };
    let rolls = |gene: Gene, baby: bool| {
        let mut ai = goal_only("PandaRollGoal", gene, baby, 9);
        goal_starts(&mut ai, &ledge, 50, rolling)
    };
    let every: Vec<i32> = (1..=50).collect();
    assert_eq!(rolls(Gene::Playful, false), every, "over the edge at once");
    assert_eq!(rolls(Gene::Normal, true), every, "a cub too");
    assert!(
        rolls(Gene::Normal, false).is_empty(),
        "a plain adult never rolls"
    );

    let mut world = EntityWorld::default();
    let mut grove = Grove {
        ledge: true,
        ..Grove::default()
    };
    let id = panda_with(&mut world, at(0.5, 0.5), Gene::Playful);
    let mut fell = None;
    for tick in 1..=60 {
        world.tick(&mut grove);
        if panda(&world, id).core.body.position.y < 61.0 {
            fell = Some(tick);
            break;
        }
    }
    assert!(fell.is_some(), "it tumbles off the edge");
    assert!(panda(&world, id).core.body.position.z > 1.0);
}
