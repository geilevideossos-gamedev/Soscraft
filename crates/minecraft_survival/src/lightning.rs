use crate::death::DamageCause;
use glam::DVec3;
use minecraft_terrain::lightning::{BoltView, PlayerStrike};
use minecraft_terrain::mesh::{Atlas, ChunkMesh};
use minecraft_terrain::mob_tracking::{tracking_range, tracks};
use minecraft_terrain::protection::Protection;
use minecraftoss_entities::inside_blocks::Hazard;
use minecraftoss_player::rng::LegacyRandom;
use std::collections::HashSet;

pub const CAUSE: DamageCause = DamageCause::Hazard(Hazard::LightningBolt);
pub const THUNDER: &str = "minecraft:entity.lightning_bolt.thunder";
pub const IMPACT: &str = "minecraft:entity.lightning_bolt.impact";
const IGNITE_TICKS: i32 = 160;
const THUNDER_VOLUME: f32 = 10000.0;
const IMPACT_VOLUME: f32 = 2.0;

#[derive(Default)]
pub struct Bolts {
    views: Vec<BoltView>,
    heard: HashSet<i32>,
    listener: Option<DVec3>,
}

impl Bolts {
    pub fn receive(&mut self, views: Vec<BoltView>) {
        self.views = views;
    }

    fn known(&self) -> impl Iterator<Item = &BoltView> {
        let range = tracking_range(minecraft_terrain::lightning::BOLT);
        self.views.iter().filter(move |bolt| {
            self.listener
                .is_some_and(|at| tracks(range, range, at.to_array(), bolt.position))
        })
    }

    pub fn tick(
        &mut self,
        listener: DVec3,
        random: &mut LegacyRandom,
        sounds: &mut Vec<(String, DVec3, f32, f32)>,
    ) {
        self.listener = Some(listener);
        let known: Vec<(i32, DVec3)> = self.known().map(|bolt| (bolt.id, bolt.position)).collect();
        self.heard
            .retain(|id| known.iter().any(|(bolt, _)| bolt == id));
        for (id, at) in known {
            if !self.heard.insert(id) {
                continue;
            }
            let pitch = 0.8 + random.next_float() * 0.2;
            sounds.push((THUNDER.to_owned(), at, THUNDER_VOLUME, pitch));
            let pitch = 0.5 + random.next_float() * 0.2;
            sounds.push((IMPACT.to_owned(), at, IMPACT_VOLUME, pitch));
        }
    }

    pub fn flashing(&self) -> bool {
        self.known().any(|bolt| bolt.life >= 0)
    }

    pub fn append(&self, mesh: &mut ChunkMesh, atlas: &Atlas, camera: DVec3) {
        minecraft_terrain::lightning_render::append_bolts(mesh, &self.views, atlas, camera);
    }
}

pub fn hit(struck: &PlayerStrike) -> crate::entities::Hit {
    (
        minecraft_terrain::lightning::DAMAGE,
        Some(struck.at),
        None,
        CAUSE,
        Default::default(),
        None,
    )
}

pub fn burn(fire_ticks: &mut i32, protection: &Protection, cause: DamageCause) {
    if cause != CAUSE {
        return;
    }
    *fire_ticks += 1;
    if *fire_ticks == 0 {
        *fire_ticks = protection.burning_ticks(IGNITE_TICKS).max(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::survival::Survival;
    use minecraftoss_player::survival::{Armor, SurvivalStatus};

    fn resting(fire_ticks: i32, health: f32) -> Survival {
        let mut survival = Survival::default();
        let mut status = SurvivalStatus::default();
        status.health = health;
        survival.restore(status, fire_ticks, 0, 0);
        survival
    }

    fn strike(survival: &mut Survival) -> bool {
        let (damage, from, toward, cause, push, effect) = hit(&PlayerStrike {
            player: 0,
            at: [0.5, 64.0, 0.5],
        });
        let blocked = crate::offhand::blocked_damage(0.0, toward, damage);
        assert_eq!(blocked, 0.0, "the shield never blocks lightning");
        assert_eq!(push.landing(true, false, 0.0), None, "no knockback");
        survival.struck(damage, blocked, Armor::default(), from, cause, effect)
    }

    #[test]
    fn lightning_ignores_the_shield_and_knockback_and_lights_no_resting_player() {
        let mut resting = resting(-20, 20.0);
        assert!(strike(&mut resting));
        assert_eq!(resting.status.health, 15.0);
        assert_eq!(
            resting.fire_ticks(),
            -19,
            "the hit itself lights no resting player"
        );
        let mut cooling = self::resting(-1, 20.0);
        strike(&mut cooling);
        assert_eq!(
            cooling.fire_ticks(),
            160,
            "-1 becomes 0, then eight seconds"
        );
        let mut burning = self::resting(100, 20.0);
        strike(&mut burning);
        assert_eq!(burning.fire_ticks(), 101);
        let mut other = self::resting(-1, 20.0);
        other.struck(3.0, 0.0, Armor::default(), None, DamageCause::Generic, None);
        assert_eq!(other.fire_ticks(), -1, "only lightning bumps the fire");
    }

    #[test]
    fn a_lightning_death_reads_struck_by_lightning() {
        let mut survival = resting(-20, 2.0);
        strike(&mut survival);
        assert_eq!(survival.status.health, 0.0);
        let language = [("death.attack.lightningBolt", "%1$s was struck by lightning")]
            .into_iter()
            .map(|(key, text)| (key.to_owned(), text.to_owned()))
            .collect();
        let line = survival.death_line(&language, |_| true, |_| None);
        assert_eq!(line.message, "%1$s was struck by lightning");
        assert_eq!(CAUSE.damage_type(), "minecraft:lightning_bolt");
    }

    #[test]
    fn a_new_bolt_thunders_once_and_flashes_while_lit() {
        let view = |id: i32, life: i32, x: f64| BoltView {
            id,
            position: DVec3::new(x, 64.0, 0.5),
            seed: 9,
            life,
        };
        let listener = DVec3::new(0.5, 64.0, 0.5);
        let mut bolts = Bolts::default();
        let mut random = LegacyRandom::new(5);
        let mut sounds = Vec::new();
        bolts.receive(vec![view(1, 1, 8.5), view(2, 1, 300.5)]);
        assert!(!bolts.flashing(), "nothing is known before the first tick");
        bolts.tick(listener, &mut random, &mut sounds);
        let heard: Vec<(&str, DVec3, f32)> = sounds
            .iter()
            .map(|(event, at, volume, _)| (event.as_str(), *at, *volume))
            .collect();
        let at = DVec3::new(8.5, 64.0, 0.5);
        assert_eq!(
            heard,
            vec![(THUNDER, at, 10000.0), (IMPACT, at, 2.0)],
            "the bolt 300 blocks off is past its sixteen chunks"
        );
        assert!((0.8..=1.0).contains(&sounds[0].3) && (0.5..=0.7).contains(&sounds[1].3));
        assert!(bolts.flashing());
        bolts.receive(vec![view(1, 0, 8.5)]);
        bolts.tick(listener, &mut random, &mut sounds);
        assert_eq!(sounds.len(), 2, "once per bolt");
        assert!(bolts.flashing());
        bolts.receive(vec![view(1, -1, 8.5)]);
        bolts.tick(listener, &mut random, &mut sounds);
        assert!(!bolts.flashing(), "drawn, but dark between flashes");
        bolts.receive(vec![view(1, 1, 8.5)]);
        assert!(bolts.flashing(), "a re-flash lights the sky again");
        bolts.receive(vec![view(3, 1, 8.5)]);
        bolts.tick(listener, &mut random, &mut sounds);
        assert_eq!(sounds.len(), 4, "a new bolt thunders");
        bolts.receive(Vec::new());
        bolts.tick(listener, &mut random, &mut sounds);
        assert!(!bolts.flashing());
    }

    #[test]
    fn summon_lists_the_lightning_bolt() {
        let packs = minecraft_terrain::pack::PackStack::open(Vec::new()).unwrap();
        let entities = crate::commands::registries(&packs).entities;
        assert!(
            entities
                .iter()
                .any(|kind| kind == minecraft_terrain::lightning::BOLT)
        );
        assert!(
            entities.windows(2).all(|pair| pair[0] < pair[1]),
            "sorted, each once"
        );
    }
}
