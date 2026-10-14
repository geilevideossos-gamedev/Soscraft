use super::*;
use crate::illager::Raider;
use crate::monster_ai::Target;
use crate::raid::RaidView;

#[derive(Clone, Debug)]
pub struct RaiderView {
    pub id: u64,
    pub kind: &'static str,
    pub position: DVec3,
    pub velocity: DVec3,
    pub width: f32,
    pub height: f32,
    pub health: f32,
    pub raider: Raider,
    pub no_action_time: i32,
    pub tick_count: i32,
    pub hurt_by_player: Option<u64>,
    pub hunts_player_or_golem: bool,
    pub recruits: bool,
}

pub struct RaiderMut<'a> {
    pub raider: &'a mut Raider,
    pub no_action_time: &'a mut i32,
    pub recruit: &'a mut bool,
}

fn hurt_by_player(ai: &MonsterAi) -> Option<u64> {
    match ai.state.hurt_by {
        Some((Target::Player(player), _)) => Some(player),
        _ => None,
    }
}

impl EntityWorld {
    pub fn set_raids(&mut self, raids: Vec<RaidView>) {
        self.raids = raids;
    }

    pub fn raids(&self) -> &[RaidView] {
        &self.raids
    }

    pub(super) fn raid_view(&self, raid: Option<i32>) -> Option<RaidView> {
        let raid = raid?;
        self.raids.iter().find(|view| view.id == raid).copied()
    }

    fn hunts_player_or_golem(&self, ai: &MonsterAi) -> bool {
        match ai.state.target {
            Some(Target::Player(_)) => true,
            Some(Target::Mob(id)) => self.entity_type(id) == Some("minecraft:iron_golem"),
            _ => false,
        }
    }

    pub fn raiders(&self) -> Vec<RaiderView> {
        let mut out = Vec::new();
        for e in &self.illagers {
            let body = &e.illager.body;
            out.push(RaiderView {
                id: e.id,
                kind: e.illager.kind.type_id(),
                position: body.position,
                velocity: body.velocity,
                width: body.width,
                height: body.height,
                health: e.illager.health,
                raider: e.illager.raider.clone(),
                no_action_time: e.no_action_time,
                tick_count: e.tick_count,
                hurt_by_player: hurt_by_player(&e.ai),
                hunts_player_or_golem: self.hunts_player_or_golem(&e.ai),
                recruits: e.ai.state.illager.recruit,
            });
        }
        for e in &self.ravagers {
            let body = &e.ravager.body;
            out.push(RaiderView {
                id: e.id,
                kind: crate::ravager::TYPE_ID,
                position: body.position,
                velocity: body.velocity,
                width: body.width,
                height: body.height,
                health: e.ravager.health,
                raider: e.ravager.raider.clone(),
                no_action_time: e.no_action_time,
                tick_count: e.tick_count,
                hurt_by_player: hurt_by_player(&e.ai),
                hunts_player_or_golem: self.hunts_player_or_golem(&e.ai),
                recruits: e.ai.state.illager.recruit,
            });
        }
        for e in &self.witches {
            let body = &e.witch.body;
            out.push(RaiderView {
                id: e.id,
                kind: "minecraft:witch",
                position: body.position,
                velocity: body.velocity,
                width: body.width,
                height: body.height,
                health: e.witch.health,
                raider: e.witch.raider.clone(),
                no_action_time: e.no_action_time,
                tick_count: e.tick_count,
                hurt_by_player: hurt_by_player(&e.ai),
                hunts_player_or_golem: self.hunts_player_or_golem(&e.ai),
                recruits: e.ai.state.illager.recruit,
            });
        }
        out
    }

    pub fn raider_mut(&mut self, id: u64) -> Option<RaiderMut<'_>> {
        if let Some(e) = self.illagers.iter_mut().find(|e| e.id == id) {
            return Some(RaiderMut {
                raider: &mut e.illager.raider,
                no_action_time: &mut e.no_action_time,
                recruit: &mut e.ai.state.illager.recruit,
            });
        }
        if let Some(e) = self.ravagers.iter_mut().find(|e| e.id == id) {
            return Some(RaiderMut {
                raider: &mut e.ravager.raider,
                no_action_time: &mut e.no_action_time,
                recruit: &mut e.ai.state.illager.recruit,
            });
        }
        let e = self.witches.iter_mut().find(|e| e.id == id)?;
        Some(RaiderMut {
            raider: &mut e.witch.raider,
            no_action_time: &mut e.no_action_time,
            recruit: &mut e.ai.state.illager.recruit,
        })
    }
}

pub const GIFT_PICKUP_DELAY: i32 = 10;

#[derive(Clone, Debug, PartialEq)]
pub struct HeroGift {
    pub table: &'static str,
    pub position: DVec3,
    pub velocity: DVec3,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FireworkLaunch {
    pub villager: u64,
    pub position: DVec3,
    pub color: i32,
    pub flight: i32,
}

#[derive(Clone, Debug, Default)]
pub struct RaidVillagerEvents {
    pub bell_rings: Vec<minecraftoss_player::Pos>,
    pub hero_gifts: Vec<HeroGift>,
    pub fireworks: Vec<FireworkLaunch>,
}

fn bell_center(bell: minecraftoss_player::Pos) -> DVec3 {
    DVec3::new(
        f64::from(bell.0) + 0.5,
        f64::from(bell.1) + 0.5,
        f64::from(bell.2) + 0.5,
    )
}

impl EntityWorld {
    pub fn take_bell_rings(&mut self) -> Vec<minecraftoss_player::Pos> {
        std::mem::take(&mut self.raid_villager_events.bell_rings)
    }

    pub fn take_hero_gifts(&mut self) -> Vec<HeroGift> {
        std::mem::take(&mut self.raid_villager_events.hero_gifts)
    }

    pub fn take_fireworks(&mut self) -> Vec<FireworkLaunch> {
        std::mem::take(&mut self.raid_villager_events.fireworks)
    }

    pub fn bell_listeners(&self, bell: minecraftoss_player::Pos, radius: f64) -> Vec<u64> {
        let (min, max) = (
            DVec3::new(f64::from(bell.0), f64::from(bell.1), f64::from(bell.2)) - radius,
            DVec3::new(f64::from(bell.0), f64::from(bell.1), f64::from(bell.2)) + 1.0 + radius,
        );
        let meets = |body: &crate::movement::Body| {
            let half = f64::from(body.width) / 2.0;
            let p = body.position;
            p.x - half < max.x
                && p.x + half > min.x
                && p.y < max.y
                && p.y + f64::from(body.height) > min.y
                && p.z - half < max.z
                && p.z + half > min.z
        };
        let villagers = self
            .villagers
            .iter()
            .filter(|e| meets(&e.villager.body))
            .map(|e| e.id);
        let raiders = self
            .raiders()
            .into_iter()
            .filter(|r| {
                let half = f64::from(r.width) / 2.0;
                let p = r.position;
                p.x - half < max.x
                    && p.x + half > min.x
                    && p.y < max.y
                    && p.y + f64::from(r.height) > min.y
                    && p.z - half < max.z
                    && p.z + half > min.z
            })
            .map(|r| r.id);
        villagers.chain(raiders).collect()
    }

    pub fn hear_bell(
        &mut self,
        listeners: &[u64],
        bell: minecraftoss_player::Pos,
        radius: f64,
        game_time: i64,
    ) {
        let center = bell_center(bell);
        for entity in self
            .villagers
            .iter_mut()
            .filter(|e| listeners.contains(&e.id))
        {
            let near = entity.villager.body.position.distance_squared(center) < radius * radius;
            if entity.villager.health > 0.0 && near {
                if let Some(ai) = entity.ai.as_deref_mut() {
                    ai.brain.memories.heard_bell_time.set(game_time);
                }
            }
        }
    }

    pub fn raiders_near_bell(
        &self,
        listeners: &[u64],
        bell: minecraftoss_player::Pos,
        radius: f64,
    ) -> Vec<u64> {
        let center = bell_center(bell);
        self.raiders()
            .into_iter()
            .filter(|r| listeners.contains(&r.id) && r.health > 0.0)
            .filter(|r| r.position.distance_squared(center) < radius * radius)
            .map(|r| r.id)
            .collect()
    }

    pub fn glowing(&self, id: u64) -> bool {
        self.mob_effects(id)
            .is_some_and(|effects| effects.has(crate::effects::MobEffect::Glowing))
    }
}

pub fn celebrate_sound(kind: &str) -> Option<&'static str> {
    match kind {
        "minecraft:pillager" => Some("entity.pillager.celebrate"),
        "minecraft:vindicator" => Some("entity.vindicator.celebrate"),
        "minecraft:evoker" => Some("entity.evoker.celebrate"),
        "minecraft:ravager" => Some("entity.ravager.celebrate"),
        "minecraft:witch" => Some("entity.witch.celebrate"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_raider_kind_celebrates_with_its_own_sound() {
        for kind in [
            "minecraft:pillager",
            "minecraft:vindicator",
            "minecraft:evoker",
            "minecraft:ravager",
            "minecraft:witch",
        ] {
            let name = kind.trim_start_matches("minecraft:");
            assert_eq!(
                celebrate_sound(kind),
                Some(format!("entity.{name}.celebrate").as_str())
            );
        }
        assert_eq!(celebrate_sound("minecraft:zombie"), None);
    }

    #[test]
    fn pillagers_ravagers_and_witches_are_listed_with_their_raid() {
        let mut world = EntityWorld::default();
        let mut pillager = crate::illager::Illager::new(
            crate::illager::IllagerKind::Pillager,
            DVec3::new(0.5, 64.0, 0.5),
        );
        pillager.raider.raid_id = Some(3);
        pillager.raider.wave = 2;
        let pillager = world.spawn_illager(pillager, true);
        let ravager = world.spawn_ravager(
            crate::ravager::Ravager::new(DVec3::new(4.5, 64.0, 0.5)),
            true,
        );
        let witch = world.spawn_witch(crate::witch::Witch::new(DVec3::new(8.5, 64.0, 0.5)), true);
        let listed = world.raiders();
        let kinds: Vec<(u64, &str)> = listed.iter().map(|r| (r.id, r.kind)).collect();
        assert_eq!(
            kinds,
            [
                (pillager, "minecraft:pillager"),
                (ravager, crate::ravager::TYPE_ID),
                (witch, "minecraft:witch")
            ]
        );
        assert_eq!(
            (listed[0].raider.raid_id, listed[0].raider.wave),
            (Some(3), 2)
        );
        let raider = world.raider_mut(witch).expect("a witch is a raider");
        raider.raider.raid_id = Some(3);
        *raider.no_action_time = 7;
        assert_eq!(world.raiders()[2].raider.raid_id, Some(3));
        assert_eq!(world.raiders()[2].no_action_time, 7);
        assert!(world.raider_mut(999).is_none());
    }

    struct Field;

    impl World for Field {
        fn block(&self, pos: minecraftoss_player::Pos) -> Option<minecraftoss_player::Block> {
            (pos.1 < 64).then(|| minecraftoss_player::Block::new("minecraft:stone"))
        }
        fn set_block(
            &mut self,
            _: minecraftoss_player::Pos,
            _: Option<minecraftoss_player::Block>,
        ) {
        }
    }

    fn raid(center: minecraftoss_player::Pos, loss: bool) -> RaidView {
        RaidView {
            id: 1,
            center,
            active: true,
            over: loss,
            loss,
            groups_spawned: 1,
            victory: false,
            stopped: false,
            between_waves: false,
        }
    }

    fn raiding_pillager(world: &mut EntityWorld, at: DVec3) -> u64 {
        let mut pillager = crate::illager::Illager::pillager(at);
        pillager.raider.raid_id = Some(1);
        pillager.raider.can_join_raid = true;
        let id = world.spawn_illager(pillager, false);
        world.illager_mut(id).unwrap().set_random_seed(11);
        id
    }

    #[test]
    fn raiders_walk_from_outside_the_village_towards_the_raid_centre() {
        let mut world = EntityWorld::default();
        let bell = (80, 64, 0);
        assert!(world.pois.add(bell, crate::poi::PoiType::Meeting));
        assert!(world.pois.acquire(bell));
        world.set_raids(vec![raid(bell, false)]);
        let id = raiding_pillager(&mut world, DVec3::new(0.5, 64.0, 0.5));
        let mut recruited = false;
        for _ in 0..600 {
            world.tick(&mut Field);
            recruited |= world.raiders().iter().any(|view| view.recruits);
        }
        let at = world
            .illagers()
            .iter()
            .find(|e| e.id == id)
            .unwrap()
            .position();
        assert!(at.x > 20.0, "it heads for the centre: {at}");
        assert!(recruited, "on its way it calls other raiders");
        assert!(world.pois.is_village(bell), "the village is still there");
    }

    #[test]
    fn raiders_with_no_raid_stay_put() {
        let mut world = EntityWorld::default();
        let bell = (80, 64, 0);
        assert!(world.pois.add(bell, crate::poi::PoiType::Meeting));
        assert!(world.pois.acquire(bell));
        let id = raiding_pillager(&mut world, DVec3::new(0.5, 64.0, 0.5));
        world.illager_mut(id).unwrap().illager.raider.raid_id = None;
        world.set_raids(vec![raid(bell, false)]);
        for _ in 0..600 {
            world.tick(&mut Field);
        }
        let at = world
            .illagers()
            .iter()
            .find(|e| e.id == id)
            .unwrap()
            .position();
        assert!(at.x < 20.0, "only strolls: {at}");
    }

    #[test]
    fn raiders_celebrate_a_lost_raid_with_their_own_sound() {
        let mut world = EntityWorld::default();
        world.set_raids(vec![raid((0, 64, 0), true)]);
        let id = raiding_pillager(&mut world, DVec3::new(0.5, 64.0, 0.5));
        let mut sounds = Vec::new();
        let mut celebrating = false;
        for _ in 0..400 {
            world.tick(&mut Field);
            celebrating |= world.illagers()[0].illager.raider.celebrating;
            sounds.extend(world.take_sounds().into_iter().map(|sound| sound.event));
        }
        assert!(celebrating);
        assert!(
            sounds
                .iter()
                .any(|event| event.ends_with("entity.pillager.celebrate")),
            "{sounds:?}"
        );
        let _ = id;
    }

    fn raid_witch(world: &mut EntityWorld, at: DVec3) -> u64 {
        let mut witch = crate::witch::Witch::new(at);
        witch.raider.raid_id = Some(1);
        witch.raider.can_join_raid = true;
        let id = world.spawn_witch(witch, false);
        world.witch_mut(id).unwrap().random = minecraftoss_player::rng::LegacyRandom::new(5);
        id
    }

    fn thrown(world: &mut EntityWorld, ticks: usize) -> Vec<crate::potion::Potion> {
        let mut potions = Vec::new();
        for _ in 0..ticks {
            world.tick(&mut Field);
            for e in world.potions() {
                if !potions.iter().any(|(id, _)| *id == e.id) {
                    potions.push((e.id, e.potion.potion));
                }
            }
        }
        potions.into_iter().map(|(_, potion)| potion).collect()
    }

    #[test]
    fn a_raid_witch_heals_a_badly_hurt_raider_and_gives_regeneration_to_a_scratched_one() {
        for (health, wanted) in [
            (3.0, crate::potion::Potion::Healing),
            (20.0, crate::potion::Potion::Regeneration),
        ] {
            let mut world = EntityWorld::default();
            world.set_raids(vec![raid((0, 64, 0), false)]);
            raid_witch(&mut world, DVec3::new(0.5, 64.0, 0.5));
            let pillager = raiding_pillager(&mut world, DVec3::new(5.5, 64.0, 0.5));
            world.illager_mut(pillager).unwrap().illager.health = health;
            world.illager_mut(pillager).unwrap().no_ai = true;
            let potions = thrown(&mut world, 400);
            assert!(!potions.is_empty(), "it throws at the raider");
            assert_eq!(potions[0], wanted, "{potions:?}");
        }
    }

    #[test]
    fn a_witch_with_no_raid_leaves_raiders_alone() {
        let mut world = EntityWorld::default();
        let witch = raid_witch(&mut world, DVec3::new(0.5, 64.0, 0.5));
        world.witch_mut(witch).unwrap().witch.raider.raid_id = None;
        let pillager = raiding_pillager(&mut world, DVec3::new(5.5, 64.0, 0.5));
        world.illager_mut(pillager).unwrap().illager.health = 3.0;
        world.illager_mut(pillager).unwrap().no_ai = true;
        assert!(thrown(&mut world, 400).is_empty());
    }

    struct Town {
        bell: Option<minecraftoss_player::Pos>,
    }

    impl World for Town {
        fn block(&self, pos: minecraftoss_player::Pos) -> Option<minecraftoss_player::Block> {
            if Some(pos) == self.bell {
                return Some(minecraftoss_player::Block::new("minecraft:bell"));
            }
            (pos.1 < 64).then(|| minecraftoss_player::Block::new("minecraft:stone"))
        }
        fn set_block(
            &mut self,
            _: minecraftoss_player::Pos,
            _: Option<minecraftoss_player::Block>,
        ) {
        }
        fn can_see_sky(&self, pos: minecraftoss_player::Pos) -> bool {
            pos.1 >= 64
        }
    }

    const HOME: minecraftoss_player::Pos = (12, 64, 0);
    const TOWN_BELL: minecraftoss_player::Pos = (0, 64, 3);

    fn town() -> EntityWorld {
        let mut world = EntityWorld::default();
        world.set_day_time(1000);
        assert!(world.pois.add(HOME, crate::poi::PoiType::Home));
        world
    }

    fn townsfolk(
        world: &mut EntityWorld,
        at: DVec3,
        profession: crate::villager::Profession,
    ) -> u64 {
        let mut villager = crate::villager::Villager::new(at);
        villager.profession = profession;
        villager.xp = 1;
        let id = world.spawn_villager_active(villager, 0.0);
        world.villager_mut(id).unwrap().random = minecraftoss_player::rng::LegacyRandom::new(3);
        id
    }

    fn brain(world: &EntityWorld) -> &crate::villager_brain::Brain {
        &world.villagers()[0].ai.as_ref().unwrap().brain
    }

    fn town_raid(groups_spawned: i32, victory: bool, loss: bool) -> RaidView {
        RaidView {
            id: 2,
            center: TOWN_BELL,
            active: true,
            over: victory || loss,
            loss,
            groups_spawned,
            victory,
            stopped: false,
            between_waves: false,
        }
    }

    fn home_center() -> DVec3 {
        DVec3::new(
            f64::from(HOME.0) + 0.5,
            f64::from(HOME.1),
            f64::from(HOME.2) + 0.5,
        )
    }

    #[test]
    fn villagers_who_hear_a_bell_hide_at_home_until_it_has_been_quiet_for_15_seconds() {
        use crate::villager_brain::Activity;
        let mut world = town();
        let mut town = Town { bell: None };
        let villager = townsfolk(
            &mut world,
            DVec3::new(0.5, 64.0, 0.5),
            crate::villager::Profession::None,
        );
        for _ in 0..20 {
            world.tick(&mut town);
        }
        let heard = world.game_time();
        let listeners = world.bell_listeners(TOWN_BELL, 48.0);
        assert_eq!(listeners, [villager]);
        world.hear_bell(&listeners, TOWN_BELL, 32.0, heard);
        world.tick(&mut town);
        assert!(brain(&world).activities.is_active(Activity::Hide));
        let mut hidden_at = None;
        let mut closest = f64::MAX;
        while world.game_time() < heard + 299 {
            world.tick(&mut town);
            hidden_at = hidden_at.or(brain(&world).memories.hiding_place.get().copied());
            let at = world.villagers()[0].position();
            closest = closest.min(at.distance(home_center()));
        }
        assert_eq!(hidden_at, Some(HOME), "it hides at the home in reach");
        assert!(closest < 3.0, "it runs home: {closest}");
        assert!(brain(&world).activities.is_active(Activity::Hide));
        for _ in 0..3 {
            world.tick(&mut town);
        }
        let memories = &brain(&world).memories;
        assert!(!memories.heard_bell_time.present() && !memories.hiding_place.present());
        assert!(
            brain(&world).activities.is_active(Activity::Idle),
            "back to its day"
        );
    }

    #[test]
    fn before_a_raid_villagers_ring_their_bell_and_once_it_starts_they_hide() {
        use crate::villager_brain::Activity;
        let mut world = town();
        let mut town = Town {
            bell: Some(TOWN_BELL),
        };
        let villager = townsfolk(
            &mut world,
            DVec3::new(0.5, 64.0, 1.5),
            crate::villager::Profession::None,
        );
        let ai = world
            .villager_mut(villager)
            .unwrap()
            .ai
            .as_deref_mut()
            .unwrap();
        ai.brain.memories.meeting_point.set(TOWN_BELL);
        world.set_raids(vec![town_raid(0, false, false)]);
        let (mut pre_raid, mut rings) = (false, Vec::new());
        for _ in 0..600 {
            world.tick(&mut town);
            pre_raid |= brain(&world).activities.is_active(Activity::PreRaid);
            rings.extend(world.take_bell_rings());
        }
        assert!(pre_raid, "a raid with no wave yet is a pre-raid");
        assert_eq!(brain(&world).activities.default, Activity::PreRaid);
        assert!(!rings.is_empty(), "it rings the bell by the meeting point");
        assert!(rings.iter().all(|&at| at == TOWN_BELL));
        world.set_raids(vec![town_raid(1, false, false)]);
        let (mut hiding, mut scared) = (None, false);
        for _ in 0..600 {
            world.tick(&mut town);
            if brain(&world).activities.is_active(Activity::Raid) {
                hiding = hiding.or(brain(&world).memories.hiding_place.get().copied());
            }
            scared |= world.take_entity_events().contains(&(villager, 42));
        }
        assert_eq!(hiding, Some(HOME), "in the raid it hides at home");
        assert_eq!(brain(&world).activities.default, Activity::Raid);
        assert!(scared, "it sweats now and then");
    }

    #[test]
    fn villagers_celebrate_a_won_raid_with_fireworks_and_never_a_lost_one() {
        for (victory, loss) in [(true, false), (false, true), (false, false)] {
            let mut world = town();
            let mut town = Town { bell: None };
            townsfolk(
                &mut world,
                DVec3::new(0.5, 64.0, 0.5),
                crate::villager::Profession::None,
            );
            world.set_raids(vec![town_raid(5, victory, loss)]);
            let (mut cheers, mut fireworks) = (0, Vec::new());
            for _ in 0..1200 {
                world.tick(&mut town);
                cheers += world
                    .take_sounds()
                    .iter()
                    .filter(|sound| sound.event.ends_with("entity.villager.celebrate"))
                    .count();
                fireworks.extend(world.take_fireworks());
            }
            if victory {
                assert!(cheers > 0, "it cheers");
                assert!(!fireworks.is_empty(), "it launches fireworks");
                for launch in &fireworks {
                    assert!(crate::villager_brain::FIREWORK_COLORS.contains(&launch.color));
                    assert!((0..3).contains(&launch.flight));
                    assert!(launch.position.y > 64.0);
                }
            } else {
                assert_eq!((cheers, fireworks.len()), (0, 0), "loss {loss}");
            }
        }
    }

    fn hero_test_player(position: DVec3) -> crate::tempt::PlayerCandidate {
        crate::tempt::PlayerCandidate {
            id: 7,
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

    #[test]
    fn a_farmer_throws_a_hero_of_the_village_a_gift_and_nobody_else() {
        for hero in [true, false] {
            let mut world = town();
            let mut town = Town { bell: None };
            let villager = townsfolk(
                &mut world,
                DVec3::new(0.5, 64.0, 0.5),
                crate::villager::Profession::Farmer,
            );
            if hero {
                world.set_player_hero(7, Some(0));
            }
            let mut gifts = Vec::new();
            for _ in 0..1500 {
                let at = world.villagers()[0].position() + DVec3::new(3.0, 0.0, 0.0);
                world.tick_with_players(&mut town, &[hero_test_player(at)]);
                gifts.extend(world.take_hero_gifts());
            }
            if !hero {
                assert!(gifts.is_empty());
                continue;
            }
            assert!(!gifts.is_empty(), "the hero gets a gift");
            let gift = &gifts[0];
            assert_eq!(
                gift.table,
                "minecraft:gameplay/hero_of_the_village/farmer_gift"
            );
            assert!((gift.velocity.length() - 0.3).abs() < 1e-3);
            assert!(gifts.len() <= 2, "a gift a while at most: {}", gifts.len());
            assert_eq!(world.villagers()[0].id, villager);
        }
    }

    #[test]
    fn a_bell_finds_villagers_and_raiders_and_makes_the_raiders_glow() {
        let mut world = town();
        let villager = townsfolk(
            &mut world,
            DVec3::new(0.5, 64.0, 0.5),
            crate::villager::Profession::None,
        );
        let near = raiding_pillager(&mut world, DVec3::new(20.5, 64.0, 3.5));
        let far = raiding_pillager(&mut world, DVec3::new(45.5, 64.0, 3.5));
        let away = raiding_pillager(&mut world, DVec3::new(90.5, 64.0, 3.5));
        let listeners = world.bell_listeners(TOWN_BELL, 48.0);
        assert_eq!(listeners, [villager, near, far]);
        assert_eq!(world.raiders_near_bell(&listeners, TOWN_BELL, 32.0), [near]);
        assert_eq!(
            world.raiders_near_bell(&listeners, TOWN_BELL, 48.0),
            [near, far]
        );
        world.hear_bell(&listeners, TOWN_BELL, 32.0, 77);
        assert_eq!(
            brain(&world).memories.heard_bell_time.get().copied(),
            Some(77)
        );
        assert!(!world.glowing(near));
        world.add_mob_effect(
            near,
            crate::effects::EffectInstance::new(crate::effects::MobEffect::Glowing, 60, 0),
        );
        assert!(world.glowing(near) && !world.glowing(away));
    }

    #[test]
    fn a_raider_in_a_raid_never_despawns_far_from_players() {
        let mut world = EntityWorld::default();
        let raider = raiding_pillager(&mut world, DVec3::new(0.5, 64.0, 0.5));
        let mut lone = crate::illager::Illager::pillager(DVec3::new(2.5, 64.0, 0.5));
        lone.raider.can_join_raid = true;
        let lone = world.spawn_illager(lone, true);
        world.illager_mut(raider).unwrap().no_action_time = 900;
        let far = [DVec3::new(500.0, 64.0, 0.0)];
        world.check_despawn(&far, false, &|_| true);
        let ids: Vec<u64> = world.illagers().iter().map(|e| e.id).collect();
        assert_eq!(ids, [raider]);
        assert_ne!(lone, raider);
        assert_eq!(world.illagers()[0].no_action_time, 0);
        world.check_despawn(&far, true, &|_| true);
        assert!(world.illagers().is_empty(), "peaceful still takes it");
    }
}
