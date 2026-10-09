use crate::potions::{PlayerPotion, PlayerPotionHit, PotionEffect, instant_amount};
use crate::thrown::{PLAYER_TARGET, PlayerBody};
use glam::DVec3;
use minecraftoss_entities::monster_ai::Target;
use minecraftoss_entities::potion::splash_scale;
use minecraftoss_entities::world::{EntityWorld, PlayerSplash};

pub const SPENT_TICKS: i32 = 20;

pub fn splashed(effect: &PotionEffect, scale: f64, duration_scale: f32) -> Option<PotionEffect> {
    let duration =
        effect.map_duration(|d| (scale * f64::from(d) * f64::from(duration_scale) + 0.5) as i32);
    let splashed = PotionEffect {
        duration,
        ..effect.clone()
    };
    (!splashed.ends_within(SPENT_TICKS)).then_some(splashed)
}

pub fn blame_player(world: &mut EntityWorld, id: u64, player: u64) {
    let by = Target::Player(player);
    if let Some(e) = world.zombie_mut(id) {
        let tick = e.tick_count;
        if let Some(ai) = e.ai.as_deref_mut() {
            ai.state.hurt_by = Some((by, tick));
        }
    } else if let Some(e) = world.skeleton_mut(id) {
        let tick = e.tick_count;
        if let Some(ai) = e.ai.as_deref_mut() {
            ai.state.hurt_by = Some((by, tick));
        }
    } else if let Some(e) = world.creeper_mut(id) {
        e.ai.state.hurt_by = Some((by, e.tick_count));
    } else if let Some(e) = world.spider_mut(id) {
        e.ai.state.hurt_by = Some((by, e.tick_count));
    } else if let Some(e) = world.slime_mut(id) {
        e.ai.state.hurt_by = Some((by, e.tick_count));
    } else if let Some(e) = world.witch_mut(id) {
        e.ai.state.hurt_by = Some((by, e.tick_count));
    } else if let Some(e) = world.iron_golem_mut(id) {
        e.ai.state.hurt_by = Some((by, e.tick_count));
    }
}

pub fn splash(
    world: &mut EntityWorld,
    at: DVec3,
    margin: f64,
    owner: u64,
    effects: &[PotionEffect],
    duration_scale: f32,
    players: &[PlayerBody],
) -> Vec<PlayerPotionHit> {
    let mut hits = Vec::new();
    if effects.is_empty() {
        return hits;
    }
    for target in crate::thrown::targets(world) {
        let Some(scale) = splash_scale(at, margin, target.min, target.max) else {
            continue;
        };
        for effect in effects {
            if effect.instantaneous() {
                let Some(mob) = effect.mob_effect() else {
                    continue;
                };
                let result = world.splash_instant(
                    target.id,
                    mob.effect,
                    mob.amplifier,
                    scale,
                    at,
                    Some(PLAYER_TARGET + owner),
                );
                if result.is_some_and(|r| r.applied) {
                    blame_player(world, target.id, owner);
                }
            } else if let Some(mob) =
                splashed(effect, scale, duration_scale).and_then(|e| e.mob_effect())
            {
                world.add_mob_effect(target.id, mob);
            }
        }
    }
    for player in players.iter().filter(|p| p.targetable()) {
        let (min, max) = player.bounds();
        let Some(scale) = splash_scale(at, margin, min, max) else {
            continue;
        };
        for effect in effects {
            let potion = if effect.instantaneous() {
                match instant_amount(effect, false, scale) {
                    Some(PlayerPotion::Harm { amount, .. }) => PlayerPotion::Harm {
                        amount,
                        by: Some(owner),
                    },
                    Some(other) => other,
                    None => continue,
                }
            } else {
                match splashed(effect, scale, duration_scale) {
                    Some(splashed) => PlayerPotion::Effect(splashed),
                    None => continue,
                }
            };
            hits.push(PlayerPotionHit {
                player: player.id,
                potion,
                at,
            });
        }
    }
    hits
}

pub fn of_witch(player: u64, splash: &PlayerSplash) -> PlayerPotionHit {
    use minecraftoss_entities::effects::EffectWork;
    let potion = match splash.instant {
        Some(EffectWork::Heal(amount)) => PlayerPotion::Heal(amount),
        Some(EffectWork::HurtMagic(amount)) => PlayerPotion::Harm { amount, by: None },
        _ => PlayerPotion::Effect(PotionEffect::new(
            splash.effect.id(),
            splash.duration,
            splash.amplifier,
        )),
    };
    PlayerPotionHit {
        player,
        potion,
        at: splash.at,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use minecraftoss_entities::effects::{EffectWork, MobEffect};

    #[test]
    fn a_lasting_splash_is_scaled_rounded_and_dropped_under_a_second() {
        let poison = PotionEffect::new("poison", 900, 0);
        assert_eq!(splashed(&poison, 0.5, 1.0).map(|e| e.duration), Some(450));
        assert_eq!(splashed(&poison, 0.625, 1.0).map(|e| e.duration), Some(563));
        assert_eq!(splashed(&poison, 0.5, 0.25).map(|e| e.duration), Some(113));
        assert_eq!(splashed(&poison, 0.02, 1.0), None);
        let forever = PotionEffect::new("poison", -1, 0);
        assert_eq!(splashed(&forever, 0.1, 1.0).map(|e| e.duration), Some(-1));
    }

    #[test]
    fn a_potion_two_blocks_off_gives_the_player_half_the_poison() {
        let mut world = EntityWorld::default();
        let players = [
            PlayerBody {
                id: 3,
                feet: DVec3::new(2.425, 64.0, 0.0),
                alive: true,
                spectator: false,
            },
            PlayerBody {
                id: 4,
                feet: DVec3::new(20.0, 64.0, 0.0),
                alive: true,
                spectator: false,
            },
        ];
        let effects = [PotionEffect::new("poison", 900, 0)];
        let hits = splash(
            &mut world,
            DVec3::new(0.0, 64.0, 0.0),
            0.0,
            3,
            &effects,
            1.0,
            &players,
        );
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].player, 3);
        let PlayerPotion::Effect(effect) = &hits[0].potion else {
            panic!("{hits:?}");
        };
        assert_eq!(effect.duration, (0.5 * 900.0 + 0.5) as i32);
    }

    #[test]
    fn harming_on_a_player_names_the_thrower() {
        let mut world = EntityWorld::default();
        let players = [PlayerBody {
            id: 1,
            feet: DVec3::ZERO,
            alive: true,
            spectator: false,
        }];
        let effects = [PotionEffect::new("instant_damage", 1, 1)];
        let hits = splash(
            &mut world,
            DVec3::new(0.0, 0.5, 0.0),
            0.0,
            9,
            &effects,
            1.0,
            &players,
        );
        assert_eq!(
            hits[0].potion,
            PlayerPotion::Harm {
                amount: 12.0,
                by: Some(9)
            }
        );
    }

    #[test]
    fn witch_splashes_reach_the_player_as_the_same_hits() {
        let at = DVec3::new(1.0, 2.0, 3.0);
        let harm = PlayerSplash {
            effect: MobEffect::InstantDamage,
            amplifier: 0,
            duration: 0,
            instant: Some(EffectWork::HurtMagic(6.0)),
            at,
        };
        assert_eq!(
            of_witch(0, &harm).potion,
            PlayerPotion::Harm {
                amount: 6.0,
                by: None
            }
        );
        let slow = PlayerSplash {
            effect: MobEffect::Slowness,
            amplifier: 0,
            duration: 1200,
            instant: None,
            at,
        };
        assert_eq!(
            of_witch(0, &slow).potion,
            PlayerPotion::Effect(PotionEffect::new("minecraft:slowness", 1200, 0))
        );
    }
}
