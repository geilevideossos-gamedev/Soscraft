use crate::effects::MobEffects;
use crate::health::{DamageResult, DamageState};
use crate::look::LookControl;
use crate::monster_ai::Target;
use crate::navigation::GroundNavigation;
use crate::tempt::PlayerCandidate;
use crate::world::families::{Despawn, Family, FamilyMob, Hurt, MobCore};
use crate::world::{DamageSourceKind, EntityWorld, Voice, voice_pitch};
use elden_boss::data::{Cue, Sound};
use elden_boss::{Act, BossDef, Brain, Dagger, Hit, Sense, Source};
use glam::DVec3;
use minecraftoss_player::World;

pub mod arena;

pub const KEY: u8 = 96;
pub const EYE_SHARE: f64 = 0.85;
pub const LOOK_TURN: f32 = 30.0;
pub const LOOK_PITCH: f32 = 40.0;
pub const HURT_VOICE_GAP: i32 = 10;
pub const FALL: &str = "minecraft:fall";

const UNSEEN: DVec3 = DVec3::splat(f64::INFINITY);

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
    pub seen: Vec<(u64, DVec3)>,
    pub voiced: i32,
    pub navigation: GroundNavigation,
    pub walk_block: Option<(i32, i32, i32)>,
    pub walked: i32,
    pub ways: Vec<arena::Way>,
    pub planned: i32,
    pub stride: DVec3,
    pub dropped: bool,
    pub hits: Vec<Hit>,
    pub throws: Vec<Dagger>,
}

impl EldenBoss {
    pub fn seen_at(&self, player: u64) -> Option<DVec3> {
        let seen = self.seen.iter().find(|(id, _)| *id == player);
        seen.map(|(_, at)| *at)
    }
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
            seen: Vec::new(),
            voiced: -HURT_VOICE_GAP,
            navigation: GroundNavigation::default(),
            walk_block: None,
            walked: -arena::PLAN_GAP,
            ways: Vec::new(),
            planned: -arena::PLAN_GAP,
            stride: DVec3::ZERO,
            dropped: false,
            hits: Vec::new(),
            throws: Vec::new(),
        },
    }
}

pub fn tick_of(core: &MobCore) -> u64 {
    u64::try_from(core.tick_count).unwrap_or(0)
}

fn steady(mob: &mut FamilyMob<EldenBoss>) {
    let FamilyMob { core, state } = mob;
    core.effects = MobEffects::default();
    if core.health > 0.0 {
        core.body.velocity = state.stride;
    }
    core.body.fall_distance = 0.0;
}

fn think<W: World>(mob: &mut FamilyMob<EldenBoss>, level: &W, players: &[PlayerCandidate]) -> Act {
    arena::survey(mob, level, players);
    let FamilyMob { core, state } = mob;
    state.seen = players.iter().map(|p| (p.id, p.position)).collect();
    let seen = arena::seen(level, players, &state.ways);
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
    state.hits.clone_from(&act.hits);
    state.throws.clone_from(&act.throws);
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
    act
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

fn source_of(mob: &FamilyMob<EldenBoss>, hurt: &Hurt) -> Option<Source> {
    let damage = hurt.amount;
    match hurt.source {
        DamageSourceKind::PlayerAttack => {
            let from = match hurt.attacker {
                Some(Target::Player(player)) => mob.state.seen_at(player),
                _ => None,
            };
            let from = from.unwrap_or(UNSEEN);
            Some(Source::Melee { damage, from })
        }
        DamageSourceKind::MobAttack => Some(Source::Melee {
            damage,
            from: UNSEEN,
        }),
        DamageSourceKind::Projectile => Some(Source::Projectile { damage }),
        DamageSourceKind::Explosion => Some(Source::Explosion { damage }),
        DamageSourceKind::Magic => None,
        DamageSourceKind::Generic | DamageSourceKind::Hazard => {
            (hurt.damage_type != FALL).then_some(Source::Environment { damage })
        }
    }
}

fn gate(damage: &mut DamageState, amount: f32) -> Option<(f32, bool)> {
    let mut probe = *damage;
    let mut health = f32::MAX;
    let passed = probe.hurt_absorbed(&mut health, f32::MAX, amount, |dealt| dealt);
    if !passed.applied {
        return None;
    }
    damage.cooldown_ticks = probe.cooldown_ticks;
    damage.last_hurt = probe.last_hurt;
    Some((passed.dealt, passed.full))
}

fn record(core: &mut MobCore, loss: f32, full: bool) -> DamageResult {
    let kept = core.damage;
    core.damage.cooldown_ticks = 0;
    let max = core.max_health;
    let result = core
        .damage
        .hurt_absorbed(&mut core.health, max, loss, |dealt| dealt);
    core.damage.cooldown_ticks = kept.cooldown_ticks;
    core.damage.last_hurt = kept.last_hurt;
    if !full {
        core.damage.hurt_ticks = kept.hurt_ticks;
        core.damage.hurts = kept.hurts;
    }
    DamageResult { full, ..result }
}

pub fn take(mob: &mut FamilyMob<EldenBoss>, source: Source, player: Option<u64>) -> DamageResult {
    let FamilyMob { core, state } = mob;
    if core.damage.dead || core.health <= 0.0 {
        return REFUSED;
    }
    let mut source = source;
    let mut full = true;
    if let Source::Environment { damage } = &mut source {
        let Some((passed, whole)) = gate(&mut core.damage, *damage) else {
            return REFUSED;
        };
        (*damage, full) = (passed, whole);
    }
    let tick = tick_of(core);
    let taken = state.brain.take(&elden_boss::Hurt {
        source,
        player,
        tick,
    });
    if taken.health <= 0.0 && !taken.died {
        return REFUSED;
    }
    core.max_health = state.brain.max_health();
    let loss = if taken.died {
        core.health
    } else {
        taken.health
    };
    let result = record(core, loss, full);
    core.health = state.brain.health();
    let position = core.body.position;
    let burning = core.body.fire_ticks > 0;
    core.damage.place_death(result, position, burning);
    core.no_action_time = 0;
    if full && !result.died && core.tick_count >= state.voiced + HURT_VOICE_GAP {
        state.voiced = core.tick_count;
        core.ambient_sound_time = -core.ambient_interval;
        let pitch = voice_pitch(&mut core.random, false);
        core.voices
            .push((spoken(state.def.voices.hurt, pitch), position));
    }
    result
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
            steady(mob);
        }
        let Some(base) = world.family_base_tick::<Self, W>(id, level, players) else {
            return;
        };
        if let Some(mob) = world.family_mob_mut::<Self>(id) {
            if !base.no_ai {
                let act = think(mob, &*level, players);
                if base.alive {
                    mob.core.body.trim_small_velocity();
                    arena::carry_out(mob, &*level, &act);
                }
            }
            speak(mob);
            settle(mob);
        }
        world.family_after_move::<Self, W>(id, level, base.game_time, ticks);
    }

    fn hurt(mob: &mut FamilyMob<Self>, hurt: &Hurt) -> DamageResult {
        let player = match hurt.attacker {
            Some(Target::Player(player)) => Some(player),
            _ => None,
        };
        match source_of(mob, hurt) {
            Some(source) => take(mob, source, player),
            None => REFUSED,
        }
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

    fn display_name(kind: &str) -> Option<&'static str> {
        elden_boss::def(kind).map(|def| def.name)
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
    use crate::creeper::CreeperExplosion;
    use crate::effects::{EffectInstance, MobEffect};
    use crate::projectile::ArrowImpact;
    use crate::world::families::{display_name, targetable};
    use crate::world::{PLAYER_TARGET, PlayerAttack};
    use elden_boss::State;
    use elden_boss::margit::MARGIT;
    use elden_boss::shape::wrap;
    use minecraftoss_player::{Block, Pos};

    struct Field;

    impl World for Field {
        fn block(&self, pos: Pos) -> Option<Block> {
            (pos.1 < 64).then(|| Block::new("minecraft:stone"))
        }

        fn set_block(&mut self, _: Pos, _: Option<Block>) {}
    }

    pub(super) fn player_at(id: u64, position: DVec3) -> PlayerCandidate {
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

    pub(super) fn margit(world: &EntityWorld, id: u64) -> &FamilyMob<EldenBoss> {
        let bosses = world.elden_bosses();
        bosses.iter().find(|mob| mob.core.id == id).unwrap()
    }

    fn health(world: &EntityWorld, id: u64) -> f32 {
        margit(world, id).state.brain.health()
    }

    fn sword(player_id: u64, position: DVec3) -> PlayerAttack {
        PlayerAttack {
            player_id,
            position,
            yaw: 0.0,
            attack_damage: 11.0,
            strength: 1.0,
            sprinting: false,
            can_critical: false,
            can_sweep: false,
        }
    }

    fn woken(world: &mut EntityWorld, id: u64, players: &[PlayerCandidate]) {
        for _ in 0..5 {
            world.tick_with_players(&mut Field, players);
        }
        assert!(margit(world, id).state.brain.awake());
    }

    struct Burning;

    impl World for Burning {
        fn block(&self, pos: Pos) -> Option<Block> {
            if pos == (0, 64, 0) {
                return Some(Block::new("minecraft:fire"));
            }
            (pos.1 < 64).then(|| Block::new("minecraft:stone"))
        }

        fn set_block(&mut self, _: Pos, _: Option<Block>) {}
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
        assert_eq!(display_name("iw4l:margit"), Some("Margit, the Fell Omen"));
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

    #[test]
    fn an_asleep_margit_takes_no_hits() {
        let mut world = EntityWorld::default();
        let id = margit_at(&mut world, 0.5, 0.5);
        world.tick_with_players(&mut Field, &[]);
        let hit = world.player_attack(&sword(1, DVec3::new(0.5, 64.0, 2.5)), id);
        assert!(!hit.hurt);
        assert_eq!(health(&world, id), 800.0);
        assert_eq!(margit(&world, id).core.damage.hurts, 0, "no red flash");
    }

    #[test]
    fn an_arrow_and_tnt_hurt_margit() {
        let mut world = EntityWorld::default();
        let id = margit_at(&mut world, 0.5, 0.5);
        let players = [player_at(1, DVec3::new(0.5, 64.0, 8.5))];
        woken(&mut world, id, &players);
        let before = health(&world, id);
        let impact = ArrowImpact {
            target_id: id,
            damage: 6.0,
            velocity: DVec3::new(0.0, 0.0, -2.0),
        };
        let hit = world.apply_arrow_hit(impact, PLAYER_TARGET + 1).unwrap();
        assert!(hit.applied && hit.full);
        let mob = margit(&world, id);
        assert_eq!(
            (health(&world, id), mob.core.health),
            (before - 6.0, before - 6.0)
        );
        assert_eq!(mob.state.brain.hurt_by(), &[1]);
        let velocity = mob.core.body.velocity;
        assert_eq!((velocity.x, velocity.z), (0.0, 0.0), "no knockback");
        let (before, stance) = (health(&world, id), mob.state.brain.stance());
        let at = mob.core.body.position;
        let blast = CreeperExplosion {
            position: DVec3::new(0.5, 64.0, 3.5),
            radius: 4.0,
            powered: false,
        };
        world.blast_by_player(&Field, blast, &players, 1);
        let lost = before - health(&world, id);
        let broken = stance - margit(&world, id).state.brain.stance();
        assert!(lost > 5.0, "{lost}");
        assert!(
            (lost * 2.0 - broken).abs() < 1e-3,
            "half of {broken}: {lost}"
        );
        world.tick_with_players(&mut Field, &players);
        let now = margit(&world, id).core.body.position;
        assert_eq!((now.x, now.z), (at.x, at.z), "the blast does not move him");
    }

    #[test]
    fn a_blast_at_his_feet_does_not_lift_him() {
        let mut world = EntityWorld::default();
        let id = margit_at(&mut world, 0.5, 0.5);
        let far = [player_at(1, DVec3::new(30.5, 64.0, 0.5))];
        world.tick_with_players(&mut Field, &far);
        let at = margit(&world, id).core.body.position;
        let blast = CreeperExplosion {
            position: DVec3::new(0.5, 64.0, 1.5),
            radius: 4.0,
            powered: false,
        };
        world.blast_by_player(&Field, blast, &far, 1);
        let pushed = margit(&world, id).core.body.velocity.y;
        assert!(pushed > 0.1, "the blast pushes him up: {pushed}");
        for _ in 0..20 {
            world.tick_with_players(&mut Field, &far);
            let now = margit(&world, id).core.body.position;
            assert_eq!(now, at, "he stays on the ground");
        }
    }

    #[test]
    fn standing_in_fire_hurts_him_once_per_10_ticks() {
        let mut world = EntityWorld::default();
        let id = margit_at(&mut world, 0.5, 0.5);
        let players = [player_at(1, DVec3::new(0.5, 64.0, 8.5))];
        let mut last = health(&world, id);
        let mut drops = Vec::new();
        for _ in 0..40 {
            world.tick_with_players(&mut Burning, &players);
            let now = health(&world, id);
            if now < last {
                drops.push(last - now);
            }
            last = now;
        }
        assert_eq!(drops, [1.0; 4]);
        assert!(margit(&world, id).core.body.fire_ticks > 0);
        assert!(margit(&world, id).core.damage.cooldown_ticks > 10);
        let hit = world.player_attack(&sword(1, DVec3::new(0.5, 64.0, 2.5)), id);
        assert!(hit.hurt && hit.full_hit);
        assert_eq!(
            health(&world, id),
            last - 11.0,
            "the sword ignores the fire's cooldown"
        );
        assert_eq!(margit(&world, id).state.brain.stance(), 80.0 - 16.5);
    }

    #[test]
    fn a_sword_hit_from_the_front_on_a_kneeling_margit_is_a_critical() {
        let mut world = EntityWorld::default();
        let id = margit_at(&mut world, 0.5, 0.5);
        let front = DVec3::new(0.5, 64.0, 3.0);
        let behind = DVec3::new(0.5, 64.0, -3.5);
        let players = [player_at(1, front), player_at(2, behind)];
        let mut ready = false;
        for _ in 0..600 {
            world.tick_with_players(&mut Field, &players);
            let mob = margit(&world, id);
            let facing = wrap(f64::from(mob.core.yaw)).abs() < 10.0;
            if mob.state.brain.state() == State::Idle && facing {
                ready = true;
                break;
            }
        }
        assert!(ready, "he faces the player in front between moves");
        for _ in 0..5 {
            assert!(world.player_attack(&sword(1, front), id).hurt);
        }
        assert_eq!(margit(&world, id).state.brain.state(), State::Kneel);
        world.tick_with_players(&mut Field, &players);
        let mut sounds: Vec<String> = world.take_sounds().into_iter().map(|s| s.event).collect();
        let before = health(&world, id);
        assert!(world.player_attack(&sword(2, behind), id).hurt);
        let from_behind = before - health(&world, id);
        assert!((from_behind - 16.5).abs() < 1e-3, "{from_behind}");
        let before = health(&world, id);
        assert!(world.player_attack(&sword(1, front), id).hurt);
        let critical = before - health(&world, id);
        assert!((critical - 33.0).abs() < 1e-3, "{critical}");
        world.tick_with_players(&mut Field, &players);
        sounds.extend(world.take_sounds().into_iter().map(|s| s.event));
        assert!(
            sounds.contains(&"entity.ravager.stunned".to_owned()),
            "{sounds:?}"
        );
        assert!(
            sounds.contains(&"entity.player.attack.crit".to_owned()),
            "{sounds:?}"
        );
    }

    #[test]
    fn a_killing_blow_credits_the_player_and_joins_the_death_list() {
        let mut world = EntityWorld::default();
        let id = margit_at(&mut world, 0.5, 0.5);
        let front = DVec3::new(0.5, 64.0, 3.0);
        let players = [player_at(1, front)];
        woken(&mut world, id, &players);
        world
            .elden_boss_mut(id)
            .unwrap()
            .state
            .brain
            .set_health(5.0);
        let hit = world.player_attack(&sword(1, front), id);
        assert!(hit.hurt && hit.died);
        let mob = margit(&world, id);
        assert_eq!(mob.core.health, 0.0);
        assert!(mob.core.damage.dead);
        assert_eq!(mob.state.brain.state(), State::Dead);
        let deaths = world.take_deaths();
        assert_eq!(deaths.len(), 1);
        let death = &deaths[0];
        assert_eq!((death.id, death.table), (id, Some("iw4l:margit")));
        assert!(death.context.killed_by_player);
        assert_eq!(death.experience, 0, "no reward yet");
        assert!(death.equipment.is_empty());
        let mut sounds = Vec::new();
        for _ in 0..25 {
            world.tick_with_players(&mut Field, &players);
            sounds.extend(world.take_sounds().into_iter().map(|s| s.event));
        }
        assert!(
            sounds.contains(&"entity.evoker.death".to_owned()),
            "{sounds:?}"
        );
        assert!(world.mob_ids().all(|mob| mob != id), "gone after his death");
    }

    #[test]
    fn falls_and_potions_do_not_hurt_margit() {
        let mut world = EntityWorld::default();
        let anchor = DVec3::new(0.5, 64.0, 0.5);
        let high = DVec3::new(0.5, 84.0, 0.5);
        let id = world.spawn_elden_boss(&MARGIT, high, 0.0, anchor, false);
        let players = [player_at(1, DVec3::new(0.5, 64.0, 8.5))];
        for _ in 0..60 {
            world.tick_with_players(&mut Field, &players);
        }
        let mob = margit(&world, id);
        assert!(mob.core.body.on_ground && mob.core.body.position.y == 64.0);
        assert!(mob.state.brain.awake());
        assert_eq!(health(&world, id), 800.0, "no fall damage");
        let fall = Hurt::new(5.0, FALL, DamageSourceKind::Generic);
        assert!(!world.hurt_family(id, &fall).unwrap().applied);
        let magic = Hurt::new(5.0, "minecraft:magic", DamageSourceKind::Magic);
        assert!(!world.hurt_family(id, &magic).unwrap().applied);
        world.splash_instant(id, MobEffect::InstantDamage, 1, 1.0, anchor, None);
        assert_eq!(health(&world, id), 800.0, "harming does nothing");
        let lava = Hurt::new(4.0, "minecraft:lava", DamageSourceKind::Hazard);
        assert!(world.hurt_family(id, &lava).unwrap().applied);
        assert_eq!(health(&world, id), 796.0, "lava does");
    }
}
