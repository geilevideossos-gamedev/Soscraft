use super::hazards::Exposed;
use super::*;
use crate::inside_blocks::Hazard;

pub const DAMAGE: f32 = 5.0;
const IGNITE_TICKS: i32 = 160;
const REACH: f64 = 3.0;
const HEIGHT: f64 = 6.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bolt {
    pub uuid: [i32; 4],
    pub at: DVec3,
}

impl Bolt {
    pub fn touches(&self, body: &Body) -> bool {
        let half = f64::from(body.width) / 2.0;
        let low = self.at - DVec3::splat(REACH);
        let high = self.at + DVec3::new(REACH, HEIGHT + REACH, REACH);
        let min = body.position - DVec3::new(half, 0.0, half);
        let max = body.position + DVec3::new(half, f64::from(body.height), half);
        min.cmplt(high).all() && max.cmpgt(low).all()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ThunderOutcome {
    Convert { id: u64, into: &'static str },
}

fn thunder_fire(body: &mut Body) {
    body.fire_ticks += 1;
    if body.fire_ticks == 0 {
        body.fire_ticks = IGNITE_TICKS;
    }
}

impl EntityWorld {
    pub fn thunder_hit(&mut self, bolt: &Bolt, world: &dyn World) -> Vec<ThunderOutcome> {
        let game_time = self.game_time;
        for key in self.order.clone() {
            let Some(mob) = self.struck_mut(key) else {
                continue;
            };
            if !mob.alive() || !bolt.touches(mob.body()) {
                continue;
            }
            if let EntityKey::Family(family, id) = key {
                if self.families.thunder_hit(family, id, bolt) == Some(true) {
                    continue;
                }
            }
            let Some(mob) = self.struck_mut(key) else {
                continue;
            };
            thunder_fire(mob.body());
            mob.hurt_hazard(Hazard::LightningBolt, DAMAGE, world, game_time);
        }
        Vec::new()
    }

    fn struck_mut(&mut self, key: EntityKey) -> Option<&mut dyn Exposed> {
        match key {
            EntityKey::Bat(id) => self
                .bats
                .iter_mut()
                .find(|e| e.id == id)
                .map(|e| e as &mut dyn Exposed),
            EntityKey::Vex(id) => self
                .vexes
                .iter_mut()
                .find(|e| e.id == id)
                .map(|e| e as &mut dyn Exposed),
            _ => self.exposed_mut(key),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Air;

    impl World for Air {
        fn block(&self, _: minecraftoss_player::Pos) -> Option<Block> {
            None
        }

        fn set_block(&mut self, _: minecraftoss_player::Pos, _: Option<Block>) {}
    }

    const SPOT: DVec3 = DVec3::new(0.5, 64.0, 0.5);

    fn bolt() -> Bolt {
        Bolt {
            uuid: [1, 2, 3, 4],
            at: SPOT,
        }
    }

    fn zombie(world: &EntityWorld, id: u64) -> &crate::zombie::Zombie {
        &world.zombies.iter().find(|e| e.id == id).unwrap().zombie
    }

    #[test]
    fn a_struck_zombie_takes_five_and_one_fire_tick() {
        let mut world = EntityWorld::default();
        let resting = world.spawn_zombie(crate::zombie::Zombie::new(SPOT + DVec3::X), true);
        let cooled = world.spawn_zombie(crate::zombie::Zombie::new(SPOT - DVec3::X), true);
        let far = world.spawn_zombie(crate::zombie::Zombie::new(SPOT + DVec3::X * 4.0), true);
        if let Some(e) = world.zombies.iter_mut().find(|e| e.id == cooled) {
            e.zombie.body.fire_ticks = -1;
        }
        assert!(world.thunder_hit(&bolt(), &Air).is_empty());
        let struck = 20.0 - damage_after_armor(5.0, 2.0, 0.0);
        assert_eq!(
            zombie(&world, resting).health,
            struck,
            "five through its armour"
        );
        assert_eq!(
            zombie(&world, resting).body.fire_ticks,
            1,
            "a mob at rest gets one fire tick"
        );
        assert_eq!(zombie(&world, cooled).health, struck);
        assert_eq!(
            zombie(&world, cooled).body.fire_ticks,
            160,
            "-1 becomes 0, then eight seconds"
        );
        assert_eq!(
            zombie(&world, far).health,
            20.0,
            "four blocks away is outside the box"
        );
        assert_eq!(zombie(&world, far).body.fire_ticks, 0);
    }

    #[test]
    fn armour_softens_lightning_and_blazes_are_hurt_too() {
        let mut world = EntityWorld::default();
        let zombie_id = world.spawn_zombie(crate::zombie::Zombie::new(SPOT), true);
        let skeleton = world.spawn_skeleton(crate::skeleton::Skeleton::new(SPOT + DVec3::Z), true);
        let blaze = world.spawn_blaze(crate::blaze::Blaze::new(SPOT - DVec3::Z), true);
        let bat = world.spawn_bat(crate::bat::Bat::new(SPOT + DVec3::Y * 8.0), true);
        let vex = world.spawn_vex(crate::vex::Vex::new(SPOT + DVec3::Y * 4.0), true);
        world.thunder_hit(&bolt(), &Air);
        let skeleton = world.skeletons.iter().find(|e| e.id == skeleton).unwrap();
        assert_eq!(skeleton.skeleton.health, 15.0, "no armour: all five");
        assert!(
            zombie(&world, zombie_id).health > skeleton.skeleton.health,
            "the zombie's armour softens it"
        );
        let blaze = world.blazes.iter().find(|e| e.id == blaze).unwrap();
        assert_eq!(
            blaze.blaze.health, 15.0,
            "fire immunity is no shield against lightning"
        );
        let bat = world.bats.iter().find(|e| e.id == bat).unwrap();
        assert_eq!(bat.bat.health, 1.0);
        let vex = world.vexes.iter().find(|e| e.id == vex).unwrap();
        assert_eq!(vex.vex.health, crate::vex::MAX_HEALTH - 5.0);
    }

    #[test]
    fn a_struck_cow_panics() {
        let mut world = EntityWorld::default();
        let id = world.spawn_cow(crate::cow::Cow::new(SPOT), false);
        world.thunder_hit(&bolt(), &Air);
        let cow = world.cows.iter().find(|e| e.id == id).unwrap();
        assert_eq!(cow.cow.health, 5.0);
        assert_eq!(cow.last_damage_source, Some(DamageSourceKind::Hazard));
    }

    #[test]
    fn a_registered_mob_answers_lightning_through_its_hook() {
        use crate::world::families::Families;
        use crate::world::families::pests::probe::{self, Probe};
        let mut world = EntityWorld::default();
        let plain = Families::spawn(&mut world, probe::KIND, SPOT).unwrap();
        let answering = Families::spawn(&mut world, probe::KIND, SPOT + DVec3::X).unwrap();
        world
            .family_mob_mut::<Probe>(answering)
            .unwrap()
            .state
            .thunder_answers = true;
        world.thunder_hit(&bolt(), &Air);
        let plain = world.family_mob_mut::<Probe>(plain).unwrap();
        assert_eq!(plain.state.thunder_bolts, vec![bolt().uuid]);
        assert_eq!(
            plain.core.health,
            probe::MAX_HEALTH - 5.0,
            "the base hit lands"
        );
        assert_eq!(plain.core.body.fire_ticks, 1);
        let answering = world.family_mob_mut::<Probe>(answering).unwrap();
        assert_eq!(answering.state.thunder_bolts, vec![bolt().uuid]);
        assert_eq!(
            answering.core.health,
            probe::MAX_HEALTH,
            "the family took the whole hit"
        );
        assert_eq!(answering.core.body.fire_ticks, 0);
    }
}
