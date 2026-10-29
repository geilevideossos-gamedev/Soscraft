use crate::effects::MobEffects;
use crate::health::DamageResult;
use crate::look::LookControl;
use crate::tempt::PlayerCandidate;
use crate::world::families::{Despawn, Family, FamilyMob, Hurt, MobCore};
use crate::world::{EntityWorld, Voice};
use elden_boss::data::{Cue, Sound};
use elden_boss::{BossDef, Brain, Seen, Sense};
use glam::DVec3;
use minecraftoss_player::World;

pub const KEY: u8 = 96;
pub const EYE_SHARE: f64 = 0.85;
pub const LOOK_TURN: f32 = 30.0;
pub const LOOK_PITCH: f32 = 40.0;

const REFUSED: DamageResult = DamageResult {
    applied: false,
    dealt: 0.0,
    died: false,
    full: false,
};

#[derive(Clone)]
pub struct EldenBoss {
    pub def: &'static BossDef,
    pub brain: Brain,
    pub anchor: DVec3,
    pub rest_yaw: f32,
    pub walk: Option<DVec3>,
}

pub fn boss(def: &'static BossDef, position: DVec3) -> FamilyMob<EldenBoss> {
    let eye = (def.height * EYE_SHARE) as f32;
    let width = def.width as f32;
    let height = def.height as f32;
    let mut core = MobCore::new(def.id, position, width, height, eye, def.health);
    core.persistent = true;
    core.body.step_height = def.step_height as f32;
    let seed = position.x.to_bits()
        ^ position.y.to_bits().rotate_left(21)
        ^ position.z.to_bits().rotate_left(42);
    FamilyMob {
        core,
        state: EldenBoss {
            def,
            brain: Brain::new(def, seed),
            anchor: position,
            rest_yaw: 0.0,
            walk: None,
        },
    }
}

pub fn tick_of(core: &MobCore) -> u64 {
    u64::try_from(core.tick_count).unwrap_or(0)
}

fn steady(core: &mut MobCore) {
    core.effects = MobEffects::default();
    core.body.velocity.x = 0.0;
    core.body.velocity.z = 0.0;
    core.body.fall_distance = 0.0;
}

fn sense_of(players: &[PlayerCandidate]) -> Vec<Seen> {
    players
        .iter()
        .filter(|player| player.alive && !player.spectator && player.attackable)
        .map(|player| Seen {
            id: player.id,
            feet: player.position,
            eye: f64::from(player.eye_height),
            ground: player.position.y,
            reachable: true,
            room: None,
        })
        .collect()
}

fn think(mob: &mut FamilyMob<EldenBoss>, players: &[PlayerCandidate]) {
    let FamilyMob { core, state } = mob;
    let seen = sense_of(players);
    let position = core.body.position;
    let sense = Sense {
        tick: tick_of(core),
        position,
        yaw: f64::from(core.yaw),
        on_ground: core.body.on_ground,
        anchor: state.anchor,
        players: &seen,
    };
    let act = state.brain.step(&sense);
    core.yaw = act.yaw as f32;
    if let Some(look) = act.look {
        core.look
            .set_look_at_with_limits(look, LOOK_TURN, LOOK_PITCH);
    }
    core.look.tick(position, core.eye_height, core.yaw, true);
    state.walk = act.walk;
    for cue in &act.cues {
        if let Cue::Sound(sound) = cue {
            let voice = Voice::Event(sound.event, sound.volume, sound.pitch);
            core.voices.push((voice, position));
        }
    }
}

fn spoken(sound: Sound, pitch: f32) -> Voice {
    Voice::Event(sound.event, sound.volume, sound.pitch * pitch)
}

fn speak(mob: &mut FamilyMob<EldenBoss>) {
    let voices = mob.state.def.voices;
    let awake = mob.state.brain.awake();
    let said = std::mem::take(&mut mob.core.voices);
    mob.core.voices = said
        .into_iter()
        .filter_map(|(voice, at)| {
            let voice = match voice {
                Voice::Ambient(_) if !awake => return None,
                Voice::Ambient(pitch) => spoken(voices.ambient, pitch),
                Voice::Hurt(pitch) => spoken(voices.hurt, pitch),
                Voice::Death(pitch) => spoken(voices.death, pitch),
                other => other,
            };
            Some((voice, at))
        })
        .collect();
}

fn settle(mob: &mut FamilyMob<EldenBoss>) {
    mob.core.max_health = mob.state.brain.max_health();
    mob.core.health = mob.state.brain.health();
}

impl Family for EldenBoss {
    const KEY: u8 = KEY;
    const KINDS: &'static [&'static str] = &elden_boss::IDS;

    fn spawn(kind: &'static str, position: DVec3) -> Option<FamilyMob<Self>> {
        elden_boss::def(kind).map(|def| boss(def, position))
    }

    fn tick<W: World>(
        world: &mut EntityWorld,
        id: u64,
        level: &mut W,
        players: &[PlayerCandidate],
        ticks: &dyn Fn(DVec3) -> bool,
    ) {
        if let Some(mob) = world.family_mob_mut::<Self>(id) {
            steady(&mut mob.core);
        }
        let Some(base) = world.family_base_tick::<Self, W>(id, level, players) else {
            return;
        };
        if let Some(mob) = world.family_mob_mut::<Self>(id) {
            if !base.no_ai {
                think(mob, players);
                if base.alive {
                    let yaw = mob.core.yaw;
                    mob.core.body.trim_small_velocity();
                    let _ = mob.core.body.travel_air(&*level, DVec3::ZERO, 0.0, yaw);
                    mob.core.body.fall_distance = 0.0;
                }
            }
            speak(mob);
            settle(mob);
        }
        world.family_after_move::<Self, W>(id, level, base.game_time, ticks);
    }

    fn hurt(_mob: &mut FamilyMob<Self>, _hurt: &Hurt) -> DamageResult {
        REFUSED
    }

    fn experience(_mob: &mut FamilyMob<Self>) -> i32 {
        0
    }

    fn despawn(_mob: &FamilyMob<Self>) -> Despawn {
        Despawn {
            kept: true,
            ..Despawn::of_category("monster")
        }
    }

    fn targetable(_kind: &str) -> bool {
        false
    }

    fn knockback_resistance(_mob: &FamilyMob<Self>) -> f64 {
        1.0
    }
}

impl EntityWorld {
    pub fn spawn_elden_boss(
        &mut self,
        def: &'static BossDef,
        position: DVec3,
        yaw: f32,
        anchor: DVec3,
        no_ai: bool,
    ) -> u64 {
        let mut mob = boss(def, position);
        mob.core.yaw = yaw;
        mob.core.look = LookControl::new(yaw);
        mob.core.no_ai = no_ai;
        mob.state.anchor = anchor;
        mob.state.rest_yaw = yaw;
        self.spawn_family(mob)
    }

    pub fn elden_bosses(&self) -> &[FamilyMob<EldenBoss>] {
        self.family::<EldenBoss>()
    }

    pub fn elden_boss_mut(&mut self, id: u64) -> Option<&mut FamilyMob<EldenBoss>> {
        self.family_mob_mut::<EldenBoss>(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::{EffectInstance, MobEffect};
    use crate::world::families::targetable;
    use elden_boss::State;
    use elden_boss::margit::MARGIT;
    use minecraftoss_player::{Block, Pos};

    struct Field;

    impl World for Field {
        fn block(&self, pos: Pos) -> Option<Block> {
            (pos.1 < 64).then(|| Block::new("minecraft:stone"))
        }

        fn set_block(&mut self, _: Pos, _: Option<Block>) {}
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

    fn margit_at(world: &mut EntityWorld, x: f64, z: f64) -> u64 {
        let at = DVec3::new(x, 64.0, z);
        world.spawn_elden_boss(&MARGIT, at, 0.0, at, false)
    }

    fn margit(world: &EntityWorld, id: u64) -> &FamilyMob<EldenBoss> {
        let bosses = world.elden_bosses();
        bosses.iter().find(|mob| mob.core.id == id).unwrap()
    }

    #[test]
    fn margit_is_a_persistent_boss_of_his_size_and_kind() {
        let mut world = EntityWorld::default();
        let id = margit_at(&mut world, 0.5, 0.5);
        assert_eq!(world.entity_type(id), Some("iw4l:margit"));
        assert_eq!(world.family_core(id).map(|(key, _)| key), Some(KEY));
        let core = &margit(&world, id).core;
        let body = &core.body;
        assert_eq!((body.width, body.height, body.step_height), (1.4, 3.0, 1.0));
        assert!((core.eye_height - 2.55).abs() < 1e-6);
        assert_eq!((core.health, core.max_health), (800.0, 800.0));
        assert!(core.persistent);
        let traits = world.family_traits(id).unwrap();
        assert_eq!(traits.category, "monster");
        assert_eq!(traits.knockback_resistance, 1.0);
        assert!(!traits.enemy && !traits.undead && !traits.fire_immune);
        assert!(!traits.leashable && traits.seats == 0);
        let census = world.census();
        assert!(
            census
                .iter()
                .any(|entry| entry.kind == "iw4l:margit" && entry.persistent)
        );
        assert!(!targetable("iw4l:margit"));
        assert!(!crate::monster_ai::is_enemy("iw4l:margit"));
        assert!(
            world.mob_candidates().iter().all(|mob| mob.id != id),
            "no mob targets him"
        );
    }

    #[test]
    fn peaceful_removes_margit() {
        let mut world = EntityWorld::default();
        let id = margit_at(&mut world, 0.5, 0.5);
        let near = [DVec3::new(0.5, 64.0, 4.5)];
        assert_eq!(world.check_despawn(&near, false, &|_| true), 0);
        assert_eq!(world.check_despawn(&near, true, &|_| true), 1);
        assert!(world.mob_ids().all(|mob| mob != id));
    }

    #[test]
    fn margit_never_despawns_far_from_players() {
        let mut world = EntityWorld::default();
        let id = margit_at(&mut world, 0.5, 0.5);
        let far = [DVec3::new(500.5, 64.0, 0.5)];
        for persistent in [true, false] {
            let core = &mut world.elden_boss_mut(id).unwrap().core;
            core.persistent = persistent;
            core.no_action_time = 10_000;
            for _ in 0..2_000 {
                assert_eq!(world.check_despawn(&far, false, &|_| true), 0);
            }
        }
        assert!(world.mob_ids().any(|mob| mob == id));
    }

    #[test]
    fn margit_sleeps_until_a_player_comes_within_24_blocks_and_then_watches_him() {
        let mut world = EntityWorld::default();
        let id = margit_at(&mut world, 0.5, 0.5);
        let far = [player_at(1, DVec3::new(30.5, 64.0, 0.5))];
        for _ in 0..20 {
            world.tick_with_players(&mut Field, &far);
        }
        assert_eq!(margit(&world, id).state.brain.state(), State::Asleep);
        assert!(world.take_sounds().is_empty(), "he sleeps quietly");
        let near = [player_at(1, DVec3::new(10.5, 64.0, 0.5))];
        world.tick_with_players(&mut Field, &near);
        assert!(margit(&world, id).state.brain.awake());
        for _ in 0..60 {
            world.tick_with_players(&mut Field, &near);
        }
        let mob = margit(&world, id);
        assert_eq!(mob.state.brain.target(), Some(1));
        let eyes = DVec3::new(10.5, 64.0 + f64::from(1.62_f32), 0.5);
        assert!(
            mob.core.look.wanted.distance(eyes) < 1e-9,
            "{}",
            mob.core.look.wanted
        );
        assert!(mob.core.body.on_ground);
        assert_eq!(mob.core.body.position.y, 64.0);
    }

    #[test]
    fn margit_speaks_in_named_events_and_ignores_potions() {
        let mut world = EntityWorld::default();
        let id = margit_at(&mut world, 0.5, 0.5);
        let poison = EffectInstance::new(MobEffect::Poison, 100, 0);
        let _ = world.add_mob_effect(id, poison);
        let players = [player_at(1, DVec3::new(6.5, 64.0, 0.5))];
        let mut sounds = Vec::new();
        for _ in 0..400 {
            world.tick_with_players(&mut Field, &players);
            sounds.extend(world.take_sounds());
        }
        assert!(!margit(&world, id).core.effects.has(MobEffect::Poison));
        assert_eq!(margit(&world, id).core.health, 800.0);
        let events: Vec<&str> = sounds.iter().map(|s| s.event.as_str()).collect();
        assert!(
            events.contains(&"entity.illusioner.mirror_move"),
            "{events:?}"
        );
        assert!(events.contains(&"entity.evoker.ambient"), "{events:?}");
        assert!(events.iter().all(|event| !event.contains("margit")));
        assert!(sounds.iter().all(|s| s.category == "hostile_volume"));
    }
}
