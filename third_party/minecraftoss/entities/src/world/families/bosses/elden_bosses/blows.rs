use super::EldenBoss;
use super::arena::eligible;
use crate::monster_ai::Target;
use crate::sight;
use crate::tempt::PlayerCandidate;
use crate::world::families::FamilyMob;
use crate::world::{EntityWorld, PlayerHitKind, Voice};
use elden_boss::data::{Cue, Harm};
use elden_boss::flight::share_of;
use elden_boss::shape::{self, Body};
use elden_boss::{BossDef, Dagger, Flight, Flown, Hit};
use glam::DVec3;
use minecraftoss_player::World;

pub const INDIRECT_MAGIC: &str = "minecraft:indirect_magic";
pub const INDIRECT_MAGIC_MESSAGE: &str = "indirectMagic";
pub const TICKS_PER_SECOND: f32 = 20.0;

pub fn kind(def: &BossDef, hit: &Hit, feet: DVec3, facing: f64) -> PlayerHitKind {
    let push = def.reactions.push(hit.reaction);
    let away = DVec3::new(feet.x - hit.from.x, 0.0, feet.z - hit.from.z)
        .try_normalize()
        .unwrap_or_else(|| shape::forward(facing));
    let throw = away * push.throw;
    match hit.harm {
        Harm::Physical => PlayerHitKind::Melee {
            attacker: hit.from,
            hunger_ticks: 0,
            lift: push.lift as f32,
            throw,
            disable_shield_seconds: f32::from(hit.shield_off) / TICKS_PER_SECOND,
        },
        Harm::Holy => PlayerHitKind::Typed {
            damage_type: INDIRECT_MAGIC,
            message: INDIRECT_MAGIC_MESSAGE,
            from: hit.from,
            push: throw + DVec3::Y * push.lift,
            bypasses_armor: true,
            bypasses_shield: true,
            disable_shield_seconds: 0.0,
            effect: None,
        },
    }
}

pub fn fly<W: World>(
    mob: &mut FamilyMob<EldenBoss>,
    level: &W,
    players: &[PlayerCandidate],
    thrown: &[Dagger],
) -> Vec<Hit> {
    let FamilyMob { core, state } = mob;
    let def = state.def;
    let bodies: Vec<(u64, Body)> = players
        .iter()
        .filter(|player| eligible(player))
        .map(|player| {
            let eye = f64::from(player.eye_height);
            (player.id, Body::player(player.position, eye))
        })
        .collect();
    let mut struck = Vec::new();
    let mut landed = Vec::new();
    state.daggers.retain_mut(|flight| {
        let (from, to) = flight.path();
        let wall = sight::clip(level, from, to).map(|(_, at)| share_of(from, to, at));
        match flight.fly(def, wall, &bodies) {
            Flown::Flying => true,
            Flown::Spent => false,
            Flown::Struck(hit) => {
                landed.push(hit.from + flight.velocity.normalize_or_zero());
                struck.push(hit);
                false
            }
            Flown::Stopped(at) => {
                landed.push(at);
                false
            }
        }
    });
    state.daggers.extend(thrown.iter().map(Flight::launch));
    for at in landed {
        for cue in def.cues.dagger_hit {
            if let Cue::Sound(sound) = cue {
                let voice = Voice::Event(sound.event, sound.volume, sound.pitch);
                core.voices.push((voice, at));
            }
        }
    }
    struck
}

pub fn deal(world: &mut EntityWorld, id: u64, players: &[PlayerCandidate], struck: &[Hit]) {
    let Some(mob) = world.family_mob_mut::<EldenBoss>(id) else {
        return;
    };
    let (def, facing) = (mob.state.def, f64::from(mob.core.yaw));
    for hit in struck {
        let Some(player) = players.iter().find(|player| player.id == hit.player) else {
            continue;
        };
        let kind = kind(def, hit, player.position, facing);
        let _ = world.family_strike(id, Target::Player(hit.player), hit.damage, kind);
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{margit, player_at};
    use super::*;
    use crate::world::PlayerHit;
    use elden_boss::State;
    use elden_boss::margit::MARGIT;
    use minecraftoss_player::{Block, Pos};
    use std::collections::HashSet;

    #[derive(Default)]
    struct Scene {
        stone: HashSet<Pos>,
    }

    impl World for Scene {
        fn block(&self, pos: Pos) -> Option<Block> {
            (pos.1 < 64 || self.stone.contains(&pos)).then(|| Block::new("minecraft:stone"))
        }

        fn set_block(&mut self, _: Pos, _: Option<Block>) {}
    }

    fn at(x: f64, y: f64, z: f64) -> DVec3 {
        DVec3::new(x, y, z)
    }

    fn awake(scene: &mut Scene, players: &[PlayerCandidate]) -> (EntityWorld, u64) {
        let mut world = EntityWorld::default();
        let home = at(0.5, 64.0, 0.5);
        let id = world.spawn_elden_boss(&MARGIT, home, 0.0, home, false);
        for _ in 0..100 {
            world.tick_with_players(scene, players);
            if margit(&world, id).state.brain.state() == State::Idle {
                let _ = world.take_player_hits();
                let _ = world.take_sounds();
                return (world, id);
            }
        }
        panic!("he never woke");
    }

    fn begin(world: &mut EntityWorld, id: u64, key: &str) {
        assert!(world.elden_boss_mut(id).unwrap().state.brain.begin(key));
    }

    fn hits_of(
        world: &mut EntityWorld,
        scene: &mut Scene,
        players: &[PlayerCandidate],
        ticks: usize,
    ) -> Vec<PlayerHit> {
        let mut hits = Vec::new();
        for _ in 0..ticks {
            world.tick_with_players(scene, players);
            hits.extend(world.take_player_hits());
        }
        hits
    }

    #[test]
    fn a_cane_hit_becomes_a_melee_player_hit_from_margit() {
        let mut scene = Scene::default();
        let players = [player_at(1, at(0.5, 64.0, 3.0))];
        let (mut world, id) = awake(&mut scene, &players);
        begin(&mut world, id, "cane_combo");
        let hits = hits_of(&mut world, &mut scene, &players, 20);
        assert_eq!(hits.len(), 1, "{hits:?}");
        let hit = hits[0];
        assert_eq!((hit.player_id, hit.damage, hit.source), (1, 4.0, Some(id)));
        let PlayerHitKind::Melee {
            attacker,
            hunger_ticks,
            lift,
            throw,
            disable_shield_seconds,
        } = hit.kind
        else {
            panic!("a physical hit is melee: {:?}", hit.kind);
        };
        let feet = margit(&world, id).core.body.position;
        assert!(attacker.distance(feet) < 0.1, "from his feet: {attacker}");
        assert_eq!((hunger_ticks, lift, throw), (0, 0.0, DVec3::ZERO));
        assert_eq!(disable_shield_seconds, 0.0, "a cane leaves the shield up");
    }

    fn hammer_hits() -> (Vec<PlayerHit>, u64) {
        let mut scene = Scene::default();
        let players = [player_at(1, at(0.5, 64.0, 3.0))];
        let (mut world, id) = awake(&mut scene, &players);
        begin(&mut world, id, "hammer_slam");
        let hits = hits_of(&mut world, &mut scene, &players, 76 + 8);
        (hits, id)
    }

    #[test]
    fn the_hammer_disables_a_shield_for_5_seconds() {
        let (hits, id) = hammer_hits();
        let slams: Vec<f32> = hits
            .iter()
            .filter_map(|hit| match hit.kind {
                PlayerHitKind::Melee {
                    disable_shield_seconds,
                    ..
                } => Some(disable_shield_seconds),
                _ => None,
            })
            .collect();
        assert_eq!(slams, [5.0], "{hits:?}");
        assert!(hits.iter().all(|hit| hit.source == Some(id)));
        let ring: Vec<&PlayerHit> = hits
            .iter()
            .filter(|hit| matches!(hit.kind, PlayerHitKind::Typed { .. }))
            .collect();
        assert_eq!(ring.len(), 1, "the shockwave is holy: {hits:?}");
        let PlayerHitKind::Typed {
            damage_type,
            message,
            bypasses_armor,
            bypasses_shield,
            disable_shield_seconds,
            ..
        } = ring[0].kind
        else {
            unreachable!();
        };
        assert_eq!((damage_type, message), (INDIRECT_MAGIC, "indirectMagic"));
        assert!(bypasses_armor && bypasses_shield);
        assert_eq!((ring[0].damage, disable_shield_seconds), (4.0, 0.0));
    }

    #[test]
    fn a_knockdown_lifts_and_throws_the_player() {
        let (hits, _) = hammer_hits();
        let (attacker, lift, throw, damage) = hits
            .iter()
            .find_map(|hit| match hit.kind {
                PlayerHitKind::Melee {
                    attacker,
                    lift,
                    throw,
                    ..
                } => Some((attacker, lift, throw, hit.damage)),
                _ => None,
            })
            .expect("the hammer lands");
        assert_eq!((damage, lift), (11.0, 0.5));
        let away = (at(0.5, 64.0, 3.0) - attacker) * DVec3::new(1.0, 0.0, 1.0);
        assert!(
            throw.distance(away.normalize() * 1.2) < 1e-9 && throw.z > 1.0,
            "1.2 away from him: {throw}"
        );
        let stagger = Hit {
            player: 1,
            damage: 7.0,
            harm: Harm::Holy,
            reaction: elden_boss::data::Reaction::Stagger,
            shield_off: 0,
            weapon: None,
            from: at(0.5, 64.0, 0.5),
            strike: 0,
        };
        let PlayerHitKind::Typed { push, .. } = kind(&MARGIT, &stagger, at(3.5, 64.0, 0.5), 0.0)
        else {
            panic!("holy");
        };
        assert!(push.distance(DVec3::new(0.8, 0.0, 0.0)) < 1e-9, "{push}");
        let under = kind(&MARGIT, &stagger, at(0.5, 64.0, 0.5), 90.0);
        let PlayerHitKind::Typed { push, .. } = under else {
            panic!("holy");
        };
        assert!(
            push.distance(DVec3::new(-0.8, 0.0, 0.0)) < 1e-9,
            "his facing: {push}"
        );
    }

    fn thrown_at(
        world: &mut EntityWorld,
        scene: &mut Scene,
        id: u64,
        players: &[PlayerCandidate],
    ) -> Flight {
        begin(world, id, "dagger_throw");
        for _ in 0..60 {
            world.tick_with_players(scene, players);
            if let Some(flight) = margit(world, id).state.daggers.first() {
                return *flight;
            }
        }
        panic!("no dagger thrown");
    }

    fn sounds_of(world: &mut EntityWorld, event: &str) -> Vec<DVec3> {
        let sounds = world.take_sounds().into_iter();
        sounds
            .filter(|sound| sound.event == event)
            .map(|sound| sound.position)
            .collect()
    }

    #[test]
    fn a_dagger_stops_at_a_wall() {
        let mut scene = Scene::default();
        for x in -3..=3 {
            for y in 64..68 {
                scene.stone.insert((x, y, 8));
            }
        }
        let players = [player_at(1, at(0.5, 64.0, 15.5))];
        let (mut world, id) = awake(&mut scene, &players);
        let flight = thrown_at(&mut world, &mut scene, id, &players);
        assert_eq!(flight.age, 0);
        assert!((flight.velocity.length() - 1.5).abs() < 1e-9);
        let mut hits = Vec::new();
        let mut struck = Vec::new();
        for _ in 0..10 {
            world.tick_with_players(&mut scene, &players);
            hits.extend(world.take_player_hits());
            struck.extend(sounds_of(&mut world, "item.trident.hit"));
        }
        assert!(margit(&world, id).state.daggers.is_empty(), "it stopped");
        assert!(hits.is_empty(), "{hits:?}");
        assert_eq!(struck.len(), 1, "{struck:?}");
        assert!(
            (struck[0].z - 8.0).abs() < 1e-6,
            "at the wall: {}",
            struck[0]
        );
    }

    #[test]
    fn a_dagger_hits_the_first_player_on_its_path() {
        let mut scene = Scene::default();
        let target = player_at(1, at(0.5, 64.0, 15.5));
        let (mut world, id) = awake(&mut scene, &[target]);
        thrown_at(&mut world, &mut scene, id, &[target]);
        let players = [target, player_at(2, at(0.5, 64.0, 8.5))];
        let mut hits = Vec::new();
        let mut struck = Vec::new();
        for _ in 0..12 {
            world.tick_with_players(&mut scene, &players);
            hits.extend(world.take_player_hits());
            struck.extend(sounds_of(&mut world, "item.trident.hit"));
            if margit(&world, id).state.daggers.is_empty() {
                break;
            }
        }
        assert_eq!(hits.len(), 1, "{hits:?}");
        let hit = hits[0];
        assert_eq!((hit.player_id, hit.damage, hit.source), (2, 4.0, Some(id)));
        assert!(matches!(
            hit.kind,
            PlayerHitKind::Typed {
                damage_type: INDIRECT_MAGIC,
                bypasses_shield: true,
                ..
            }
        ));
        assert_eq!(struck.len(), 1);
        assert!((struck[0].z - 8.2).abs() < 0.5, "{}", struck[0]);
    }

    #[test]
    fn a_dagger_is_gone_after_40_ticks() {
        let mut scene = Scene::default();
        let high = player_at(1, at(0.5, 75.0, 15.5));
        let (mut world, id) = awake(&mut scene, &[high]);
        let flight = thrown_at(&mut world, &mut scene, id, &[high]);
        assert!(flight.velocity.y > 0.0, "thrown up at him");
        let aside = [player_at(1, at(30.5, 64.0, 0.5))];
        let mut flying = 1;
        let mut hits = Vec::new();
        for _ in 0..60 {
            world.tick_with_players(&mut scene, &aside);
            hits.extend(world.take_player_hits());
            let daggers = &margit(&world, id).state.daggers;
            if daggers.iter().any(|dagger| dagger.start == flight.start) {
                flying += 1;
            }
        }
        assert_eq!(flying, i32::from(MARGIT.daggers.ticks));
        assert!(hits.is_empty(), "{hits:?}");
        assert!(sounds_of(&mut world, "item.trident.hit").is_empty());
    }
}
