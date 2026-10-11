//! The world's hazards, on every mob (pinned 26.3 `Entity.baseTick`'s
//! burning, `LivingEntity.baseTick`'s `isInWall`, and
//! `Entity.applyEffectsFromBlocks` after each move): a burning mob takes 1
//! damage a second out of lava; a mob whose eyes are in a suffocating block
//! takes 1; and after moving, the blocks it went through act on it (fire
//! and lava set it alight and hurt it, water and rain put it out with a
//! hiss, cactus, campfires, berry bushes and magma prick and burn,
//! cobwebs and bushes hold it). Fire resistance turns fire's damage away;
//! animals panic from fire, lava, magma and cactus.
use super::*;
use crate::effects::MobEffect;
use crate::inside_blocks::{block_effects, in_wall, BlockEffect, Hazard};

/// What the hazards need of a mob.
pub(crate) trait Exposed {
    fn body(&mut self) -> &mut Body;
    fn alive(&self) -> bool;
    /// `hurtServer` with the hazard's damage type: whether it took.
    fn hurt_hazard(&mut self, hazard: Hazard, amount: f32, world: &dyn World, game_time: i64) -> bool;
    fn random(&mut self) -> &mut LegacyRandom;
    fn voices(&mut self) -> &mut Vec<(Voice, DVec3)>;
    fn fire_immune(&self) -> bool {
        false
    }
}

pub fn fire_immune(kind: &str) -> bool {
    matches!(
        kind,
        "minecraft:area_effect_cloud"
            | "minecraft:blaze"
            | "minecraft:ender_dragon"
            | "minecraft:end_crystal"
            | "minecraft:ghast"
            | "minecraft:magma_cube"
            | "minecraft:shulker"
            | "minecraft:strider"
            | "minecraft:tnt"
            | "minecraft:vex"
            | "minecraft:warden"
            | "minecraft:wither"
            | "minecraft:wither_skeleton"
            | "minecraft:zoglin"
            | "minecraft:zombified_piglin"
    )
}

fn hurt<W: World>(mob: &mut impl Exposed, hazard: Hazard, amount: f32, world: &W, game_time: i64) -> bool {
    if hazard.is_fire() && mob.fire_immune() {
        return false;
    }
    mob.hurt_hazard(hazard, amount, world, game_time)
}

/// `Entity.baseTick`'s burning: a second's damage (`on_fire`) out of lava
/// on each twentieth tick, then a tick less; and in lava the fall distance
/// halves.
pub(crate) fn burn<W: World>(mob: &mut impl Exposed, world: &W, game_time: i64) {
    let immune = mob.fire_immune();
    let body = mob.body();
    let in_lava = FluidFrame::sample(world, body.position, body.width, body.height).in_lava();
    if body.fire_ticks > 0 && immune {
        body.fire_ticks = 0;
    } else if body.fire_ticks > 0 {
        if body.fire_ticks % 20 == 0 && !in_lava {
            mob.hurt_hazard(Hazard::OnFire, 1.0, world, game_time);
        }
        mob.body().fire_ticks -= 1;
    }
    // Lava breaks a fall's force.
    if in_lava {
        mob.body().fall_distance *= 0.5;
    }
}

/// `LivingEntity.baseTick`'s `isInWall`: suffocation, while alive.
pub(crate) fn suffocate<W: World>(mob: &mut impl Exposed, world: &W, eye_height: f32, game_time: i64) {
    if mob.alive() && in_wall(world, mob.body(), eye_height) {
        mob.hurt_hazard(Hazard::InWall, 1.0, world, game_time);
    }
}

/// `Entity.applyEffectsFromBlocks` after the mob moved (from
/// `old_position`, where the tick found it): the block effects in order
/// while it lives, rain putting fire out, the hiss of fire going out, and
/// the fire timer settling at 0 when not burning.
pub(crate) fn blocks_act<W: World>(mob: &mut impl Exposed, world: &W, old_position: DVec3, game_time: i64) {
    let body = mob.body();
    let moves = std::mem::take(&mut body.moves);
    let effects = block_effects(world, body, old_position, &moves);
    let before = body.fire_ticks;
    let immune = mob.fire_immune();
    let was_on_fire = before > 0;
    for effect in effects {
        if !mob.alive() {
            break;
        }
        match effect {
            BlockEffect::Hurt(hazard, amount) => {
                hurt(mob, hazard, amount, world, game_time);
            }
            BlockEffect::Stuck(multiplier) => {
                let body = mob.body();
                body.stuck = Some(multiplier);
                body.fall_distance = 0.0;
            }
            BlockEffect::ClearFreeze => {}
            BlockEffect::FireIgnite | BlockEffect::LavaIgnite | BlockEffect::LavaHurt if immune => {}
            BlockEffect::FireIgnite => {
                // `BaseFireBlock.fireIgnite`.
                let body = mob.body();
                if body.fire_ticks < 0 {
                    body.fire_ticks += 1;
                }
                if body.fire_ticks >= 0 {
                    body.fire_ticks = body.fire_ticks.max(160);
                }
            }
            BlockEffect::LavaIgnite => {
                let body = mob.body();
                body.fire_ticks = body.fire_ticks.max(300);
            }
            BlockEffect::Extinguish => {
                let body = mob.body();
                body.fire_ticks = body.fire_ticks.min(0);
            }
            BlockEffect::FireHurt(amount) => {
                hurt(mob, Hazard::InFire, amount, world, game_time);
            }
            BlockEffect::LavaHurt => {
                if mob.hurt_hazard(Hazard::Lava, 4.0, world, game_time) {
                    let pitch = 2.0 + mob.random().next_float() * 0.4;
                    let at = mob.body().position;
                    mob.voices().push((Voice::Event("entity.generic.burn", 0.4, pitch), at));
                }
            }
        }
    }
    let body = mob.body();
    let p = body.position;
    let feet = (p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32);
    let top = (feet.0, (p.y + f64::from(body.height)).floor() as i32, feet.2);
    if world.rain_at(feet) || world.rain_at(top) {
        body.fire_ticks = body.fire_ticks.min(0);
    }
    if was_on_fire && body.fire_ticks <= 0 {
        // `playEntityOnFireExtinguishedSound`.
        let random = mob.random();
        let (a, b) = (random.next_float(), random.next_float());
        let at = mob.body().position;
        mob.voices().push((Voice::Event("entity.generic.extinguish_fire", 0.7, 1.6 + (a - b) * 0.4), at));
    }
    let body = mob.body();
    if body.fire_ticks <= 0 && body.fire_ticks <= before {
        // `-getFireImmuneTicks()`: 0 for mobs.
        body.fire_ticks = 0;
    }
}

/// Fire resistance turns fire's damage away (`#is_fire`).
fn resists(effects: &crate::effects::MobEffects, hazard: Hazard) -> bool {
    hazard.is_fire() && effects.has(MobEffect::FireResistance)
}

/// The animals' damage kind for a hazard: panicking or not.
fn animal_source(hazard: Hazard) -> DamageSourceKind {
    if hazard.panics() {
        DamageSourceKind::Hazard
    } else {
        DamageSourceKind::Generic
    }
}

macro_rules! exposed {
    ($ty:ty, $mob:ident, |$e:ident, $h:ident, $a:ident, $w:ident, $t:ident| $hurt:expr) => {
        exposed!($ty, $mob, |$e, $h, $a, $w, $t| $hurt, |_s| false);
    };
    ($ty:ty, $mob:ident, |$e:ident, $h:ident, $a:ident, $w:ident, $t:ident| $hurt:expr, |$s:ident| $immune:expr) => {
        impl Exposed for $ty {
            fn body(&mut self) -> &mut Body {
                &mut self.$mob.body
            }
            fn alive(&self) -> bool {
                self.$mob.health > 0.0
            }
            #[allow(unused_variables)]
            fn hurt_hazard(&mut self, $h: Hazard, $a: f32, $w: &dyn World, $t: i64) -> bool {
                if resists(&self.effects, $h) {
                    return false;
                }
                let $e = self;
                $hurt
            }
            fn random(&mut self) -> &mut LegacyRandom {
                &mut self.random
            }
            fn voices(&mut self) -> &mut Vec<(Voice, DVec3)> {
                &mut self.voices
            }
            fn fire_immune(&self) -> bool {
                let $s = self;
                $immune
            }
        }
    };
}

exposed!(
    ZombieEntity,
    zombie,
    |e, h, a, w, t| {
        let amount = if h.bypasses_armor() { a } else { damage_after_armor(a, 2.0, 0.0) };
        e.hurt(amount).applied
    },
    |s| fire_immune(s.zombie.kind.type_id())
);
exposed!(SkeletonEntity, skeleton, |e, h, a, w, t| e.hurt(a).applied, |s| fire_immune(s.skeleton.kind.type_id()));
exposed!(CreeperEntity, creeper, |e, h, a, w, t| e.hurt(a).applied);
exposed!(SpiderEntity, spider, |e, h, a, w, t| e.hurt(a).applied);
exposed!(VillagerEntity, villager, |e, h, a, w, t| e.hurt_from(a, h.damage_type(), None, t).applied);
exposed!(BatEntity, bat, |e, h, a, w, t| e.hurt(a).applied);
exposed!(CowEntity, cow, |e, h, a, w, t| e.hurt(a, animal_source(h)).applied);
exposed!(PigEntity, pig, |e, h, a, w, t| e.hurt(a, animal_source(h)).applied);
exposed!(ChickenEntity, chicken, |e, h, a, w, t| e.hurt_with_source(a, animal_source(h)).applied);
exposed!(super::slimes::SlimeEntity, slime, |e, h, a, w, t| e.hurt(a).applied, |s| s.slime.magma);
exposed!(super::endermen::EndermanEntity, enderman, |e, h, a, w, t| e.hurt_by_environment(a, w).applied);
exposed!(super::golems::IronGolemEntity, golem, |e, h, a, w, t| e.hurt(a).applied);
exposed!(super::wolves::WolfEntity, wolf, |e, h, a, w, t| e.hurt_from(a, h.damage_type(), None, t).applied);
exposed!(super::ghasts::GhastEntity, ghast, |e, h, a, w, t| e.hurt(a).applied, |_s| true);
exposed!(super::blazes::BlazeEntity, blaze, |e, h, a, w, t| e.hurt(a).applied, |_s| true);
exposed!(super::striders::StriderEntity, strider, |e, h, a, w, t| e.hurt(a, animal_source(h)).applied, |_s| true);
exposed!(super::hoglins::HoglinEntity, hoglin, |e, h, a, w, t| e.hurt(a).applied, |s| s.hoglin.fire_immune());
exposed!(super::piglins::PiglinEntity, piglin, |e, h, a, w, t| if h.bypasses_armor() { e.hurt(a).applied } else { e.hurt_armored(a).applied });

impl Exposed for SheepEntity {
    fn body(&mut self) -> &mut Body {
        &mut self.body
    }
    fn alive(&self) -> bool {
        self.health > 0.0
    }
    fn hurt_hazard(&mut self, hazard: Hazard, amount: f32, _world: &dyn World, _game_time: i64) -> bool {
        if resists(&self.effects, hazard) {
            return false;
        }
        self.hurt(amount, animal_source(hazard)).applied
    }
    fn random(&mut self) -> &mut LegacyRandom {
        &mut self.random
    }
    fn voices(&mut self) -> &mut Vec<(Voice, DVec3)> {
        &mut self.voices
    }
}

impl Exposed for super::witches::WitchEntity {
    fn body(&mut self) -> &mut Body {
        &mut self.witch.body
    }
    fn alive(&self) -> bool {
        self.witch.health > 0.0
    }
    fn hurt_hazard(&mut self, hazard: Hazard, amount: f32, _world: &dyn World, game_time: i64) -> bool {
        if hazard.is_fire() && self.witch.effects.has(MobEffect::FireResistance) {
            return false;
        }
        let took = self.hurt(amount, false, false).applied;
        // `getLastDamageSource` (a fire type sends it for fire resistance).
        if took && hazard.is_fire() {
            self.witch.last_fire_damage = Some(game_time);
        }
        took
    }
    fn random(&mut self) -> &mut LegacyRandom {
        &mut self.random
    }
    fn voices(&mut self) -> &mut Vec<(Voice, DVec3)> {
        &mut self.voices
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

    struct Mob {
        body: Body,
        immune: bool,
        random: LegacyRandom,
        voices: Vec<(Voice, DVec3)>,
        hits: Vec<Hazard>,
    }

    impl Exposed for Mob {
        fn body(&mut self) -> &mut Body {
            &mut self.body
        }
        fn alive(&self) -> bool {
            true
        }
        fn hurt_hazard(&mut self, hazard: Hazard, _amount: f32, _world: &dyn World, _game_time: i64) -> bool {
            self.hits.push(hazard);
            true
        }
        fn random(&mut self) -> &mut LegacyRandom {
            &mut self.random
        }
        fn voices(&mut self) -> &mut Vec<(Voice, DVec3)> {
            &mut self.voices
        }
        fn fire_immune(&self) -> bool {
            self.immune
        }
    }

    fn mob(immune: bool) -> Mob {
        Mob { body: Body::new(DVec3::new(0.5, 64.0, 0.5), 0.6, 1.95), immune, random: LegacyRandom::new(0), voices: Vec::new(), hits: Vec::new() }
    }

    #[test]
    fn the_nether_mobs_and_the_bosses_are_immune_to_fire() {
        for kind in ["blaze", "ghast", "magma_cube", "strider", "wither_skeleton", "zoglin", "zombified_piglin", "ender_dragon", "wither", "shulker", "warden", "vex"] {
            assert!(fire_immune(&format!("minecraft:{kind}")), "{kind}");
        }
        for kind in ["zombie", "skeleton", "piglin", "piglin_brute", "hoglin", "enderman", "slime", "husk"] {
            assert!(!fire_immune(&format!("minecraft:{kind}")), "{kind}");
        }
    }

    #[test]
    fn a_fire_immune_mob_puts_its_fire_out_and_shrugs_off_fire_but_not_cactus() {
        let mut burning = mob(false);
        burning.body.fire_ticks = 40;
        burn(&mut burning, &Air, 0);
        assert_eq!((burning.body.fire_ticks, burning.hits.clone()), (39, vec![Hazard::OnFire]));
        let mut immune = mob(true);
        immune.body.fire_ticks = 40;
        burn(&mut immune, &Air, 0);
        assert_eq!((immune.body.fire_ticks, immune.hits.len()), (0, 0));
        for hazard in [Hazard::InFire, Hazard::Lava, Hazard::HotFloor, Hazard::Campfire, Hazard::OnFire] {
            assert!(!hurt(&mut immune, hazard, 4.0, &Air, 0), "{hazard:?}");
            assert!(hurt(&mut burning, hazard, 4.0, &Air, 0), "{hazard:?}");
        }
        assert!(hurt(&mut immune, Hazard::Cactus, 1.0, &Air, 0));
        assert_eq!(immune.hits, vec![Hazard::Cactus]);
    }
}
