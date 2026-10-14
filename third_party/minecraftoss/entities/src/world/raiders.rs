use super::*;
use crate::illager::Raider;
use crate::monster_ai::Target;
use crate::raid::RaidView;

#[derive(Clone, Debug)]
pub struct RaiderView {
    pub id: u64,
    pub kind: &'static str,
    pub position: DVec3,
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
